---
id: "SPEC-Nb71vRwYkP8sHd4mQt2xLc"
type: "spec"
status: "approved"
title: "Issue #631 real-command-receipt case"
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
    - "docs/discovery/issue-631-command-receipt.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-command-receipt case

## What Should Be Built

`rf631.plugin-dock-real` catalog に `real-command-receipt` case を
1 行追加する。実 Window 上の fader fixture（`sv_fader_dock_fixture`
— command-bound control を持つ既存 fixture、変更なし）へ
Begin→Preview→Commit の実 gesture を駆動し、mint された
receipt-expecting intent の `command_id` に対して driver-neutral
`post_event` 経由で実 `Event::Result`（applied/rejected）と
`Event::Notify` projection を投入する。`CommandReceipt::from_result`
で decode した receipt を `ScenarioOutcome.receipts` へ populate
し、JSONL の `receipts` field に非空で記録する（AC1 残 link +
AC4 Command+Receipt evidence）。

session は window A（sentinel）+ B/C（close target）に加え、
window D（`ViewInstanceId` 634・fader fixture）を第 4 mount として
追加する。`real-window-close` の終端 assert は「非対象 window が
全て生存」へ一般化する（window D が sentinel A と共に残るのが
正しい終端）。

## Why Now

Issue #631 AC1 の必須リンク「OS Input Injection→Command→
Applied/Rejected Receipt→同一 Case ID 関連付け」は全 22 real row
で未カバー — 既存 case は `receipts` が全て空だった。driver-neutral
契約（`dispatch_input`/`drain_intents`/`post_event`/`observe`/
`settle`）と receipt decode・report 配線は全て実装済みのため、
本 slice は catalog row + case 実装 + evidence 再生成に閉じる。

## Users and Value

- GUI Test Driver / Scenario Runner の consumer — Command→Receipt
  相関を case 単位で Pass/Fail/NotRun 判定し、receipt を JSONL
  evidence として取得できる。
- Issue #631 の受入判定者 — AC1 の Command→Receipt link と AC4 の
  Command+Receipt populate が実観測 evidence 付きで判定される。
- 後続 slice（HEADLESS_MATRIX 側 command-receipt・consumer 投影・
  AC4 残 field）の実装者 — 本 slice の verdict contract と
  receipts 形式を土台にできる。

## Scope

### In Scope

- `real_catalog.rs`: `real-command-receipt` row を `REAL_CATALOG`
  に追加（`required` = `SemanticAction`）。
- `real_dock.rs`: `real_entries` に対応 entry を追加、session に
  window D（fader fixture）を第 4 mount、`RealDockContext` に
  `window_d`/`view_d` 追加、`command_receipt_body` 実装、
  `real-window-close` の終端 assert 一般化、module doc 表更新、
  console/catalog 件数の整合（real 22 行・console 30 行）。
- `tests/console.rs`: `--list` 行数 29→30・real-dock 系 assertion
  の 21→22 更新。
- `evidence/rf631/plugin-dock-real-scenario.jsonl` /
  `plugin-dock-real-headless.jsonl` の再生成と
  `evidence/rf631/verification.md` への slice 記録追記。
- doc chain（discovery/spec/design/plan/task/test-spec/adr/impl）。

### Out of Scope

- HEADLESS_MATRIX 側 command-receipt 行（`tests/plugin_dock.rs`・
  `Cargo.toml` は ruviform#725 lease 占有中 — 後続 slice）。
- `crates/ruviform-ui`/`ruviform-ui-gpui`/`ruviform-ui-test` の
  変更（既存 fixture・driver 契約をそのまま利用）。
- `real-dcomp-pixel` の Fail 修正（#532 gate — 既知 DComp 欠陥を
  正直に保持）。
- `real-window-ime`・editor verbs・crash/hang の実装（#389/#630
  領域 — `Unsupported` 維持）。
- DPI 変更駆動・consumer 投影（synvelis#114/#176・
  audio-lens#127 gate）。
- AC4 の残 field（GPU 情報・完全な semantic tree・window
  HWND/plugin hash digest）。
- 新規依存追加・`Cargo.lock`/manifest 変更。

## Requirements

- case は実 Window（window D の fader fixture）上で
  Begin→Preview→Commit gesture を駆動し、`drain_intents` で
  receipt-expecting commit intent を採取する。採取 intent の
  `command_id`/`expect_receipt`/decode 済み `CommandIntent` を
  evidence に記録する。
- receipt-expecting intent が mint されない場合は honest
  `Fail`/`NotRun` — receipt を捏造しない。`command_id` は表示
  文字列や位置ではなく minted intent の値で相関する。
- `post_event` で投入する `Event::Result` は
  `CommandReceipt::from_result` で decode し、view へ配送した
  event と同一のものから receipt を収集する。
- applied 経路: `status:"applied"`+revision の result で
  `CommandPhase::Applied` を settle/observe し、続く
  `Event::Notify` projection（`control_id`/`revision`/`value`）
  で `CommandPhase::Observed` を settle/observe する —
  `Applied` と `Observed` は別 evidence level として記録する。
- rejected 経路: 別 gesture の commit に `status:"rejected"` の
  result を投入し `CommandPhase::Rejected` を観測する。
- `ScenarioOutcome.receipts` に decode 済み `CommandReceipt` を
  populate し、失敗時もそれまでに収集した receipt を保持する。
- `observed` に step 単位の観測（gesture intent 数・command_id・
  phase 遷移・committed/observed 値・semantic tree）を記録する。
- `REAL_CATALOG` と `real_entries` の `case`/`scenario`/`required`
  は協調して更新する（feature-gated 整合 test が検査）。
- headless/blocked では全行 `NotRun`+`observed.environment_blocked`
  の fail-closed 形状を維持し、`EnvironmentKind::RealWindow` は
  実走 session 内の report のみが名乗る。
- `real-window-close` の終端 assert は「mount 時 window 集合から
  close target を除く全 window が生存」へ一般化し、window D を
  sentinel A と同列の非対象 survivor として検証する。
- GPUI/Win32 型を公開 API・JSONL に出さない（driver boundary
  維持）。

## Acceptance Criteria

- [ ] `real-command-receipt` が実 Windows desktop で実走し、
  Begin→Preview→Commit が receipt-expecting intent を mint、
  `command_id` 相関の applied/rejected `Event::Result` で
  `Applied`→`Observed`/`Rejected` の `CommandPhase` 遷移を実観測
  して `Pass`/`Fail` verdict を返す。
- [ ] 同 case の `receipts` が JSONL に非空で出る（applied +
  rejected の decode 済み `CommandReceipt`、command_id/status/
  revision/detail を含む）。
- [ ] `REAL_CATALOG`/`real_entries` の required capabilities が
  一致し、`real_catalog_matches_entries`・
  `blocked_reports_fail_closed` が green。
- [ ] `test-ui-plugin-dock-real-headless`（`--blocked-probe`）が
  全行 `NotRun`+`environment_blocked` を exit 0 で維持し、
  `plugin-dock-real-headless.jsonl` が再生成される（22 case 行）。
- [ ] `real-window-close` が window D 追加後も正しい終端
  （非対象 window 全生存）で `Pass`/`Fail` を返す。
- [ ] 非回帰 lane（`fmt-check`/`check`/`lint`/`test`
  （default+`real-dock`+`lab-gui` features）/
  `test-ui-plugin-dock-console`/`test-ui-plugin-dock`/
  `test-ui-scenario-contract`/`check-ui-driver-boundary`/
  `prek run --all-files`）が green。
- [ ] `evidence/rf631/verification.md` に本 slice の検証記録が
  追記される。

## Deferred Design Concerns

- HEADLESS_MATRIX 側 command-receipt 行 — ruviform#725 lease
  （`tests/plugin_dock.rs`/`Cargo.toml`）解除後の後続 slice。
- `real-dcomp-pixel` verdict=fail — #532 gate（DComp occlusion
  既知欠陥）。本 slice で修正も Pass 化もしない。
- editor verbs・IME・DPI 変更・crash/hang — #389/#630 領域。
- consumer 投影 — synvelis#114/#176・audio-lens#127 gate。
- AC4 の残 field（GPU 情報・semantic tree 全量・HWND/plugin
  hash）— 公開 API/依存の制約で別 slice。

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — auto-close しない）。
- `docs/discovery/issue-631-command-receipt.md` — 探索記録。
- `docs/specs/issue-631-real-window-catalog-flip.md` — 前 slice の
  spec（receipts/case_seed 配線の由来）。
- `docs/impl/ir/issue-631-real-window-catalog-flip.md` — 前 slice
  の実装記録。
- `POLICIES.md`（検証レベル・Pass/Fail/NotRun/Unsupported 規範・
  fail-closed）。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`・
  `real_catalog.rs`・`tests/console.rs` — 変更対象。
- `crates/ruviform-ui-test/src/scenario/plugin_dock.rs`・
  `lab_story.rs` — `sv_fader_dock_fixture`（変更なし・再利用）。
- `crates/ruviform-ui-test/tests/scenario_contract.rs` —
  receipt/projection 先例。
- `evidence/rf631/verification.md` — slice 記録の追記先。
