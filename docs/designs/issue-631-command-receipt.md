---
id: "DESIGN-Hc82nSyZlQ9tIe5oRu3yMd"
type: "design"
status: "approved"
title: "Issue #631 real-command-receipt case 設計"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-command-receipt.md"
    - "docs/discovery/issue-631-command-receipt.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-command-receipt case 設計

## Context

`rf631.plugin-dock-real` lane は `RealOsGuiTest::run` の 1 session
内で 22 case を逐次実行する。session は mount 順に window id を
振り（`windows()` = mount order）、window A は sentinel（GPUI
platform loop は最終 window close で終了）、B/C は
`real-window-close` の破壊 target。`real-command-receipt` は case
順で close より先に走る（`real_entries` の catalog 順）。

Command→Receipt 契約（driver-neutral、変更なし）:

- gesture/semantic input → view が `MintedIntent`（`command_id`・
  `expect_receipt`・decode 済み `CommandIntent`）を mint →
  `drain_intents` で採取。
- `post_event(window, Event)` で `Event::Result`（intent id 相関）
  と `Event::Notify`（projection: `control_id`/`revision`/`value`）
  を view へ配送。
- `ViewHost::apply_receipt` が `command_id` で binding を routing
  し `CommandPhase` を遷移（`Applied`/`Rejected`/`Unsupported`）。
- `ViewHost::apply_projection` が authoritative state を revision
  guard 付きで適用し `Observed` へ settle。
- `CommandReceipt::from_result` が配送 event から receipt を
  decode（`command_id`/`status`/`revision`/`detail`）。
- `ScenarioOutcome.receipts` → `ScenarioReport.receipts` →
  `jsonl.rs::case_line` の `receipts` field（catalog-flip slice
  で配線済み）。

先例: `ruviform-ui-test/tests/scenario_contract.rs` の
`post_event_routes_receipt_and_projection_to_binding` が同じ
intent→receipt→projection の流れを fake driver で検証済み。
fixture 先例: `lab_story::sv_fader_dock_fixture`（fader control
`fader-ch1` を持つ command-bound spec）。

## Scope

### In Scope

- `real_catalog.rs`: `real-command-receipt` row 追加。
- `real_dock.rs`: window D mount・`RealDockContext` field・
  `command_receipt_body`/case・close 終端 assert 一般化・
  module doc 表更新。
- `tests/console.rs`: 件数 assertion 更新（real 22・console 30）。
- evidence JSONL 2 本の再生成 + `verification.md` 追記。

### Out of Scope

- `ruviform-ui`/`ruviform-ui-gpui`/`ruviform-ui-test` の変更、
  HEADLESS_MATRIX 側行、pixel/IME/verb/consumer/AC4 残 field、
  新規依存。

## Components and Boundaries

### Session 構成（`run_all_with_pixel_artifacts`）

- window A（view 631・dock spec）— sentinel。全 case の土台。
- window B（view 632・dock spec・(1450,80)）— focus/close target。
- window C（view 633・dock spec・(2900,80)）— close default
  target。
- window D（view 634・`sv_fader_dock_fixture`・(4350,80)・新規）
  — `real-command-receipt` の command-minting fixture。dock spec
  は command binding を持たないため専用 window とし、既存
  window の spec・view identity を一切変更しない。B/C/D は A の
  右へ offset し window が重ならない（OS click が A に着地する
  前提を維持）。
- `RealDockContext` に `window_d: Option<WindowId>`・
  `view_d: ViewInstanceId` を追加。

### `real-command-receipt`（`command_receipt_body`）

`run_scenario` + `steps_outcome` の既存構造で、観測は全て
`SemanticGuiTest` 公開面（`resolve_target`/`dispatch_input`/
`drain_intents`/`post_event`/`settle`/`observe`/
`component_snapshot_json`）のみ:

1. `resolve_target` — window D / view 634 / `fader-ch1` を解決。
2. gesture 1（applied 経路）: fader へ Begin→Preview→Commit を
   `dispatch_input` で駆動 → `drain_intents` で採取 →
   `expect_receipt` な commit intent を `command_id` で特定
   （display text/位置ではなく minted 値で相関）。begin/preview
   の非 receipt intent は evidence に intent 数のみ記録。
3. `post_event(window_d, Event::result_ok(intent.id, {"status":
   "applied","revision":1}))` → `settle` → `observe` で
   `CommandPhase::Applied` と committed/display 値を記録。
4. `post_event(window_d, Event::notify({"control_id":"fader-ch1",
   "revision":2,"value":{"normalized":0.7}}))` → `settle` →
   `observe` で `CommandPhase::Observed`・`observed.revision=2`
   を記録（Applied→Observed の別 level 証明）。
5. gesture 2（rejected 経路）: 別 commit intent を mint →
   `post_event` で `{"status":"rejected","detail":"consumer
   refused the commit"}`（`Event::result_err`）→ `settle` →
   `observe` で `CommandPhase::Rejected` と、直前の authoritative
   projection（`observed.revision=2`・`value.normalized=0.7`）が
   binding に保持されることを記録。
6. 各 `post_event` で使った `Event::Result` から
   `CommandReceipt::from_result` で decode した receipt を
   `receipts` vec へ収集し、`ScenarioOutcome.receipts` へ
   populate（失敗時も収集済み分を保持）。
7. observed extras: window/view/component id・`semantic_tree`
   （`component_snapshot_json` の fader snapshot）を記録。

判定: 全 step pass → `Pass`。receipt-expecting intent 不在・
phase 不整合・observe 失敗は honest `Fail`（`steps_outcome` の
step verdict が証拠）。timeout/driver error は `ScenarioError`
経由の既存写像。

### `real-window-close` 終端 assert の一般化

現行は `windows() == [window_a]`（sentinel のみ残存）を要求するが、
window D 追加で正しい終端は `[A, D]`。case 冒頭に採取した
`windows`（mount order）から close target B/C を除いた集合を
survivor として保持し、終端で `windows() == survivors` を assert。
survivor 全員の `window_events` に `CloseRequested`/`Destroyed`
が無いことを検査（従来は A のみ — D も同列に検証する方が
invariant として正しい）。step 名は `survivors_only`/
`survivors_isolated` へ改名し、outcome detail を「every
non-target window observed no close lifecycle and survived」へ
更新。B/C の close sequence assert・terminal retirement probe は
不変。

### catalog/件数の協調更新

- `REAL_CATALOG` に `real-command-receipt`（driver=`RealOs`・
  required=`SemanticAction`・case 順は `real-os-input` の次
  = close より前）を追加。
- `real_entries` に対応 `ScenarioEntry`（case/scenario/required
  一致）を追加 — `real_catalog_matches_entries` が検査。
- `tests/console.rs`: `--list` が headless 8 + real 22 = 30 行を
  期待する assertion を更新（従来 29 = 8 + 21）。
- `real_dock.rs` の blocked-probe 系 assertion（typed entry
  配列 21→22）と module doc「twenty-two cases」表を更新。

## Data and Control Flow

1. `RealOsGuiTest::run` が 4 mount で session を開始 →
   `real_entries` を `run_entries` で逐次実行。
2. command-receipt case（case 10）: `run_scenario` が preflight
   （`SemanticAction` claim 済み）→ body が gesture→intent→
   `post_event`→settle→observe を駆動 → `ScenarioOutcome`
   （steps・observed extras・receipts）を返す。
3. `finalize_case`/`finalize_real` が report を組立
   （`environment_kind=real-window`・session scale factor・
   `case_seed`・`receipts`）。
4. `render_jsonl` が run header + case line（`receipts` 非空）を
   `plugin-dock-real-scenario.jsonl` へ出力。
5. blocked-probe/`run_all` の session 失敗経路は
   `blocked_reports`（`NotRun`+`environment_blocked`・
   `deterministic-backend`）を維持 — 新 case も同じ形状。

## Non-Functional Constraints

- RT 境界: non-RT のみ（既存 lane と同じ）。
- honesty: receipt は実 event の decode のみ。`Unsupported`/
  `NotRun`/`Fail` を偽装しない。`environment_kind=real-window`
  は実走 session 内のみ。
- driver boundary: 公開面は既存 driver-neutral 型のみ
  （`check-ui-driver-boundary` green 維持）。GPUI/Win32 型を
  JSONL・公開 API に出さない。
- `#[non_exhaustive]` policy: 新規公開 enum なし。
- `workspace.lints` authoritative、`clippy -- -D warnings`。
- 既存 fixture（`sv_fader_dock_fixture`）は変更しない — 再利用
  のみ。

## Risks and Trade-offs

- window D 追加で session の window が 4 つに — mount/paint
  コスト増のみ。`real-multiwindow-focus` は `>= 2` 判定、close
  case は一般化後の survivor assert で影響なし（case 順で
  command-receipt は close より先）。
- fader gesture の OS-level timing（pointer 座標・settle 時間）
  で intent mint が遅延/失敗する可能性 — `settle_until` の
  bounded 待機と honest Fail で対応。実走で観測済み。
- `Applied`→`Observed` の projection は `control_id`/`revision`
  の stale guard に依存 — revision を実値（gesture commit の
  revision+1）で送る設計で実走確認済み。
- rejected 経路は `CommandPhase::Rejected` への settle と直前
  authoritative projection の保持を観測する — reject が
  観測済み state を黙って巻き戻す/進めることを許さない。
  実走では phase=`rejected`・`observed.revision=2` の projection
  が binding に保持されることを確認（`display`/`committed` は
  component snapshot の実値をそのまま記録 — 観測値の合成は
  しない）。
- `close_body` の survivor 一般化は assert を弱めない —
  「mount 時の全 window のうち close target 以外が生存」へ
  等価以上に強い invariant。

## Deferred Design Concerns

- HEADLESS_MATRIX 側 command-receipt — ruviform#725 lease 解除後。
- `command_id` を case id へ写像する横断 correlation（同一
  case 内の intent↔receipt は本 slice で実証済み。run 横断の
  seed 相関は `case_seed` が既に担う）。
- consumer（Synvelis/AudioLens）への driver-neutral 投影 —
  synvelis#114/#176・audio-lens#127 gate。
- AC4 の GPU 情報・semantic tree 全量・HWND/plugin hash —
  API/依存制約で別 slice。

## Verification Notes

- [x] `mise run fmt-check` / `check`・`lint`・`test`
  （default/`real-dock`/`lab-gui`）green。
- [x] `mise run test-ui-plugin-dock-real-headless` — 全行
  `NotRun`+`environment_blocked`、exit 0、22 case 行。
- [x] `mise run test-ui-plugin-dock-real` — interactive Windows
  で実走、`real-command-receipt` pass + receipts 2 件、
  `real-window-close` pass（survivor 一般化後）、
  `real-dcomp-pixel` の既知 Fail のみ残存（exit 1 は設計どおり）。
- [x] `mise run test-ui-plugin-dock-console`・
  `test-ui-plugin-dock`・`test-ui-scenario-contract`・
  `check-ui-driver-boundary` green。
- [x] `mise exec -- prek run --all-files` green。
- [x] `evidence/rf631/verification.md` へ slice 記録追記。
