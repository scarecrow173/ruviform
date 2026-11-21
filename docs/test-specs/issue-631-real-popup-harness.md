---
id: "TSPEC-01M7Y96P5AREWKR3Y63G1N66DX"
type: "test-spec"
status: "approved"
title: "Issue #631 real-editor-popup テスト仕様 — title event log・top-level WS_POPUP HWND・消失・teardown の各 oracle と fail-closed 写像"
created: "2026-11-21"
updated: "2026-11-21"
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
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-631-real-popup-harness.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-editor-popup テスト仕様 — title event log・top-level WS_POPUP HWND・消失・teardown の各 oracle と fail-closed 写像

## Purpose

`real-editor-popup` が証明しなければならないことを固定する。
popup の存在を fixture の論理状態や `send_key` の戻り値から
推論せず、**OS 観測できる title event log と実 HWND の
inventory/style/rect/生死**だけで判定する。

## Feature

`session.send_key` による `VK_P`（`'p'`/`0x50`）down+up を 2
回、production `PluginEditorSession` → `InProcessEditorOps` →
`HostCommand::SendGuiKey` → `Vst3Plugin` → `IPlugView` chain
経由で送り、fixture の `popup_toggle` が作る実
`WS_POPUP` top-level HWND の生成・消失・teardown を観測する。

## Test scenarios

### 構造・投影（オフライン検証 — `cargo test`）

| # | 判定 | 場所 |
| --- | --- | --- |
| T1 | `real_entries` が 29 行で `real-editor-popup` を `real-editor-focus` と `real-session-crash` の間に含む | `real_dock.rs` 配列長 + `real_catalog_matches_entries` pairwise 照合 |
| T2 | `REAL_CATALOG` 行が `case`/`scenario`/`required`（`NativePluginDock`）で `real_entries` と一致 | `real_catalog_matches_entries` |
| T3 | `blocked_reports`/`blocked_probe` が 29 `NotRun` 行を生成し session-scoped 欄を持たない（headless fail-closed 形状） | `blocked_reports_fail_closed`・`blocked_probe_rows_carry_no_session_scoped_fields` |
| T4 | `--list --include-real` が 38 行（9 headless + 29 real）を catalog 順で出力し popup 行を含む | `list_include_real_appends_real_lane_rows` |

### real-window 実行（`test-ui-plugin-dock-real` — interactive Windows）

| # | シナリオ | 判定 oracle |
| --- | --- | --- |
| R1 | open toggle 受理 | `editor_popup_open` step が `pass`（`Ops(Unsupported)` なら `unsupported` verdict → case `Unsupported`） |
| R2 | open title log | 全 child title が `…\|kd:0070:0050:0000,po:1,ku:0070:0050:0000` で終了 |
| R3 | open HWND | `own_windows` に fixture popup ちょうど 1 個（class `RuviformTestEditor` + title prefix `rvf-test-editor-popup-` + owner == `GA_ROOT(child)`）、alive、`WS_POPUP` set、`WS_CHILD` clear、rect (0,0) 200x100 |
| R4 | close toggle 受理（open `Accepted` 時のみ） | `editor_popup_close` step `pass` |
| R5 | close title log | 全 child title が `…,pc:1,…` を含む全 suffix で終了 |
| R6 | close HWND | 同条件 popup が inventory 0 個 かつ記録 HWND `IsWindow==0` |
| R7 | session 維持 | toggle 後 `Attached` |
| R8 | close 収束 | `session.close` → `IsWindow==0` → `Closed+concluded` → events == `[Attached{Docked}, Closed]` |
| R9 | teardown | close 後に owner 非依存の fixture popup 0 個 |

### verdict 写像（fail-closed）

| # | 状況 | 期待 verdict |
| --- | --- | --- |
| V1 | fixture 欠落 / container lease 不成立 / 非 interactive 環境 | `NotRun` + `environment_blocked` |
| V2 | open toggle `Ops(Unsupported)` 拒否 | `Unsupported` |
| V3 | toggle 受理済みで R2/R3/R5/R6/R8/R9 のいずれか不成立 | `Fail`（最初の失敗 step が detail。`NotRun` に降格しない） |
| V4 | close toggle の受理後拒否/失敗 | `Fail`（contract surprise — `Refused` 化しない） |
| V5 | R1–R9 すべて成立 | `Pass` |
| V6 | 非 Windows stub | `Unsupported`（`supported:false` — 誠実な境界） |

### 否定的契約（してはいけないこと）

- `popup_open` 論理フラグや `send_key` 戻り値だけで Pass を
  推論しない — HWND inventory が唯一の実 popup oracle。
- `EnumChildWindows(container)` で popup を探さない —
  `WS_POPUP` top-level は子孫列挙に現れない。
- owner 未解決時に owner 不問で popup を拾わない —
  `owner: None` は 0 件一致 = 記録済み `Fail`。
- teardown で popup 残存を見逃さない — (0,0) 200x100 の残存は
  後続 case の観測遮蔽になる。
- Issue #532 既知 `Fail`（`real-dcomp-bandescape`/
  `real-dcomp-childinv`）を popup slice の失敗として記録
  しない、また隠さない。

## Evidence

- `evidence/rf631/plugin-dock-real-scenario.jsonl` — interactive
  run の全 step/verdict/extras（`"popup"` object 含む）。
- `evidence/rf631/plugin-dock-real-headless.jsonl` — headless
  blocked-probe 29 `NotRun` 行 +
  `observed.environment_blocked`、`environment_kind` に
  `real-window` を名乗らない。
- `evidence/rf631/verification.md` — 実行コマンド・OS・exit
  code・verdict 分布と、local 検証 / 統合受入の分離。

## Verification mapping

`mise run test`（T1–T4）、`mise run test-ui-plugin-dock` /
`test-ui-plugin-dock-console`（投影・console）、
`mise run test-ui-plugin-dock-real-headless`（V1 blocked-probe）、
`mise run test-ui-plugin-dock-real`（R1–R9・V1–V5）。local 合格は
merge 後 CI/integration acceptance の代替ではない。
