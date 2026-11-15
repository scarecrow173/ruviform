---
id: "IMPL-034YfD7Ww0GzDhsIAOEu4H"
type: "impl"
status: "in-progress"
title: "issue-532-dcomp-pixel"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/ospix.rs"
      - type: "file"
        path: "docs/designs/issue-532-dcomp-pixel.md"
      - type: "file"
        path: "docs/plans/issue-532-dcomp-pixel.md"
      - type: "file"
        path: "docs/specs/issue-532-dcomp-pixel.md"
      - type: "file"
        path: "docs/tasks/issue-532-dcomp-pixel.md"
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-pixel.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/bin/plugin-dock-real-scenario.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-532-dcomp-pixel.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-532-dcomp-pixel.md"
    - "docs/plans/issue-532-dcomp-pixel.md"
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "docs/test-specs/issue-532-dcomp-pixel.md"
    - "docs/impl/exp/issue-532-dcomp-pixel.jsonl"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-532-dcomp-pixel.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-532-dcomp-pixel.jsonl"
    rejected: []
---

# issue-532-dcomp-pixel

## Summary

`rf631.plugin-dock-real` に 5 件目の real case `real-dcomp-pixel` を
追加し、docked 状態の `test_editor` 実 child HWND 矩形に対して画面
合成画素（screen-DC `BitBlt`）を主 oracle、`PrintWindow` と
`GetDC`+`BitBlt` の window-DC を診断 note として採取する。
併せて全 real report の `environment` に
`gpui_disable_direct_composition`（生文字列・未設定は `null`）を
記録し、binary に opt-in `--pixel-artifacts <dir>` を追加した。
実機（interactive Windows session）で baseline / env-on の双方を
実走し、`Fail`（画面に `EDITOR_COLOR` 緑が出ない）を正直に固定した。

## Context

`scarecrow173/ruviform#532` の再現固定 slice。`docs/specs` →
`docs/designs` → `docs/plans` → `docs/tasks` →
`docs/test-specs`（いずれも `issue-532-dcomp-pixel`）のチェーンに
従う。方式 A'（`GPUI_DISABLE_DIRECT_COMPOSITION`、受理値
`"1"`/`"true"`）の実機観測までが範囲で、方式採否・本 fix は
後続 slice。

## Implementation

### Changes

- `src/ospix.rs`（新規・`real-dock` feature）: query/capture-only
  の Win32 画素観測。`window_screen_rect` / `presented`
  （`GA_ROOT`+`IsIconic`+`IsWindowVisible`）/ `capture_screen_rect`
  （screen DC `BitBlt`→32bpp top-down DIB→BGRA→RGBA8、
  backend `win32-bitblt-screen`）/ `capture_print_window`
  （`PW_CLIENTONLY|PW_RENDERFULLCONTENT`、backend
  `win32-printwindow`）/ `capture_window_dc`（`GetDC`+`BitBlt`、
  backend `win32-windowdc`）。非 Windows は stub で honest error。
  `oschild` と同じ「window を作らない・動かさない・message を
  送らない」境界、Win32 型は `u64` handle に narrow。
- `src/real_dock.rs`: `attempt_real_attach` を観測 callback 注入
  可能に一般化（既存 `real-editor-attach` の振る舞いは不変）、
  close/detach/session 収束 tail を `conclude_editor_session`
  に抽出、editor 系 preamble を `prepare_editor_host` に集約
  （`Box<ScenarioOutcome>` で `result_large_err` 回避）。
  `observe_editor_child_lifecycle`（既存契約:
  child 存在・parent 一致・visible/alive・close 成功・HWND 破棄・
  `Closed+concluded`・events `[Attached{Docked}, Closed]`）と
  `observe_editor_pixel_lifecycle`（lifecycle 記録 + 画素観測）
  を分離。`real_dcomp_pixel_case` が両者を合成して verdict 決定。
  `green_stats`/`green_fraction`（閾値 `g>=200, r<=80, b<=80,
  fraction>=0.5`）。`save_pixel_artifact` は opt-in PNG 出力で
  失敗は `artifact_error` note のみ（verdict 不変）。
  `dcomp_env_extra`/`finalize_real` で全 real report の
  `environment["gpui_disable_direct_composition"]` に生値を記録。
- `src/real_catalog.rs`: `REAL_CATALOG` に `real-dcomp-pixel`
  行追加（required: `visual-capture`, `native-plugin-dock`）。
- `src/bin/plugin-dock-real-scenario.rs`: `--pixel-artifacts
  <dir>` 引数追加（未指定時は PNG を書かない）、
  `run_all_with_pixel_artifacts` 経由で `RealDockContext` へ伝播。
- `src/lib.rs` / `README.md`: 4→5 case の記述更新、
  `--pixel-artifacts` と env 記録の説明。
- `Cargo.toml`: `real-dock` feature の `windows-sys` optional dep
  に `Win32_Graphics_Gdi` feature 追加（既存 edge の拡張・新規 dep
  なし）。
- `tests/console.rs`: `--list --include-real` が 13 行
  （headless 8 + real 5）になる期待値に更新。

### Decisions During Implementation

- 画素 oracle は **screen-DC `BitBlt`（`win32-bitblt-screen`）を
  主**、`PrintWindow`/`GetDC`+`BitBlt` は **診断 note**。
  `PrintWindow` は child 自身の raster で画面合成を示さないため
  verdict gate にしない（rf388 evidence の先例どおり）。
- `GetDC`+`BitBlt`（`win32-windowdc`）を実装中盤で追加した
  診断軸: 初回実機観測で `PrintWindow` が `DefWindowProcW` class
  に対し green を返さず「child が描いたが合成で隠れた」のか
  「child surface 自体に fill が無い」のか判別不能だったため、
  `WM_PRINT` を介さない window-DC 直読みで child の backing
  surface を観測する第 2 診断を追加（結果: env-on で
  `green_fraction=1.0`、baseline で `0.0` — 判別に寄与）。
- env var は process 起動時 env 前置（`GPUI_DISABLE_DIRECT_COMPOSITION=1
  mise run test-ui-plugin-dock-real` 相当の shell env 伝播）で与え、
  binary 内 `set_var` は行わない — 観測対象を変えないため。
  `mise.toml` は lease 外のため lane 自体は不変、env-on run は
  canonical JSONL を上書きしない別 `--out` の補助実行として記録。
- `u64`→`f64` は `cast_precision_loss` 回避のため `u32` 飽和経由の
  `green_fraction` に集約。`ScenarioOutcome` は `Box` で
  `result_large_err` 回避。`too_many_lines` は child 毎処理を
  `observe_one_child_pixels`/`record_*_pixels` に分割して解消
  （`#[allow]` 不使用）。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-532-dcomp-pixel.jsonl` — baseline/env-on の
  実機観測と `windowdc` 診断軸追加の判断を記録。

### Rejected

- なし。

## Validation

実施済み（詳細は `evidence/rf631/verification.md` と本 PR 本文）:

- `mise run fmt-check` / `mise run fmt` — pass
- `mise run check|lint|test ruviform-plugin-dock-test`（default）— pass
- `mise run check|lint|test ruviform-plugin-dock-test --features
  real-dock` — pass（`real_catalog_matches_entries` 等 green）
- `mise run test-ui-plugin-dock` / `test-ui-plugin-dock-console` /
  `test-ui-scenario-contract` / `check-ui-driver-boundary` — pass
- `mise run test-ui-plugin-dock-real`（baseline・interactive
  session）— exit 1、`real-dcomp-pixel` verdict `fail`
  （`green_fraction=0.0` via `win32-bitblt-screen`）
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` env-on 実走 — exit 1、同様に
  `fail`（screen 0.0、診断 `windowdc` は 1.0）。exit 127 は未観測
- `mise exec -- prek run --all-files` — pass

## Risks

- env-on で `windowdc=1.0`（child surface は緑）にもかかわらず
  screen が 0.0 のまま — `GPUI_DISABLE_DIRECT_COMPOSITION=1` が
  GPUI 0.2.2 で受理される値であることは source 確認済みだが、
  非 DComp 経路（`CreateSwapChainForHwnd`+`FLIP_SEQUENTIAL`）でも
  画素が画面に出ない可能性を示唆。方式 A' が画素問題を解消するかは
  未確定 — 本 slice は観測のみで採否は後続 slice。
- 単一ホスト・単一 GPU（RTX 4070 SUPER）・単一 DPI 環境での観測。
  他環境での再現性は未検証。
- `PrintWindow` は `DefWindowProcW` class の child に対し情報量が
  限定的（実測 0.0）— 診断 note であり verdict には不使用。

## Follow-ups

- 方式採否（A/A'/B/C）の決裁 — 本 slice の観測（env-on でも screen
  緑が出ない）を入力に後続 slice で実施。
- `mise.toml` L153-170 の lane コメント「editor-pixel question
  stays deferred」は本 slice で stale 化 — lease 外のため未修正、
  Coordinator 統合時に対応。
- docs index（`docs/*/README.md`）は Coordinator が統合時に再生成。
- `Cargo.lock` は本 slice で差分なし（`windows-sys` feature 拡張は
  既存 edge 内）。
