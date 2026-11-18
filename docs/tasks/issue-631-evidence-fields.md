---
id: "TASK-k9Jm2vXc8Qp4wRt6yHb3Ns"
type: "task"
status: "in-progress"
title: "Issue #631 AC4 evidence-fields 完成"
created: "2026-11-18"
updated: "2026-11-18"
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
  implements:
    - "docs/plans/issue-631-evidence-fields.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-631-evidence-fields.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 AC4 evidence-fields 完成

## Work

Issue #631 AC4 の残 evidence field を manifest 変更不要の
部分集合で完成させる slice。変更は `crates/ruviform-
plugin-dock-test` に閉じ、catalog 件数（headless 9 + real
22）・`Cargo.toml`/`Cargo.lock`・production crate は不変。

1. **window identity digest**: `run_all_with_pixel_artifacts`
   で window A–D mount 直後に `window_geometry` 初回値 +
   `WindowId` + mount title を canonical 文字列化して
   `Sha256::digest`（`dpi_body:6135-6150` 方式踏襲）を
   1 回だけ計算し `RealDockContext` に保持 →
   `finalize_case` の merged extras（`session_scale_factor`
   と同パターン）で全 real case の
   `environment.window_identities` に刻む。
   `blocked_reports`/`blocked_probe` は session 不在のため
   刻まない。geometry 読取失敗 window は `sha256: null`。
   `real-window-dpi` の `window_geometry_sha256` は変更しない。
2. **capture/artifact digest**: 全 pixel capture step row に
   `capture_sha256`（実 RGBA 画素バイトの SHA-256）を記録 —
   `real-capture`（`capture_frame` の `RgbaImage::pixels()`）、
   `real-dcomp-pixel` の `screen_pixels`/`printwindow_pixels`/
   `windowdc_pixels`、`real-dcomp-foreground` の
   `fg_observe_*`、`real-dcomp-present` の
   `pres_capture`/`pres_screen`。`save_pixel_artifact` を
   `(width, height, rgba)` 引数に一般化し `{path, sha256}`
   を返す → `record_pixel_artifact` が `artifact: {path,
   sha256}` object を row に刻む。`--pixel-artifacts` を
   `real-capture` にも拡張（opt-in 維持）。
3. **semantic tree 全量化**: `scenario.rs` に
   `SemanticGuiTest` generic の helper を追加し、
   `ViewHost::validate_spec`（`view.rs:1185`）と同じ walk
   （Panel children 再帰 + Tabs alias/canonical 両 key）で
   mount 済み `ViewSpec` の全 component key を列挙、
   `resolve_target`+`observe`+`component_snapshot_json` で
   記録（失敗は `null`）。適用: real lane
   `real-window-mount`・`real-command-receipt`、headless
   `command_receipt_body`。
4. **doc 修復**: `README.md` verdict 表の
   `real-window-dpi`/`real-window-close` stale 行修復 +
   `real-command-receipt` 行追加（22 行）。bin doc の
   `--pixel-artifacts` 対象に `real-capture` 追記。
   `mise.toml` `test-ui-plugin-dock-real` description の
   stale 文言修正（canonical 引数不変）。
5. **evidence 再生成**: `plugin-dock-real-headless.jsonl`
   （`--blocked-probe`、必須）+ `plugin-dock-real-scenario.
   jsonl`（interactive desktop あれば）を再生成し、
   `verification.md` に slice 記録追記。
6. **doc chain** 完成（discovery/spec/design/plan/task/
   test-spec/adr/ir/exp）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-631-evidence-fields.md`
- `docs/specs/issue-631-evidence-fields.md`
- `docs/designs/issue-631-evidence-fields.md`
- `docs/discovery/issue-631-evidence-fields.md`
- `docs/test-specs/issue-631-evidence-fields.md`

## Dependencies

- `RealOsGuiTest::window_geometry`/`observe`/`resolve_target`/
  `capture_frame`（実装済み・変更なし）。
- `ospix::PixelCapture`/`capture_screen_rect`/
  `capture_print_window`/`capture_window_dc`（実装済み）。
- `ViewHost::validate_spec` の flat 列挙先例（`view.rs:1185`）。
- `plugin_dock_view_spec()`・`sv_fader_dock_fixture().spec`
  （public data・walk 可能）。
- `sha2` 既存依存・`artifact::sha256_hex` helper。
- `component_snapshot_json`（real lane `real_dock.rs:1056`・
  headless `scenario.rs:992` — cfg 境界の重複は既知・ADR 記録済み）。

## Affected Files

- `crates/ruviform-plugin-dock-test/src/real_dock.rs`
- `crates/ruviform-plugin-dock-test/src/scenario.rs`
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-real-scenario.rs`
- `crates/ruviform-plugin-dock-test/README.md`
- `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs`
- `mise.toml`（description のみ・引数不変）
- `evidence/rf631/plugin-dock-real-headless.jsonl`（再生成）
- `evidence/rf631/plugin-dock-real-scenario.jsonl`（interactive あれば再生成）
- `evidence/rf631/verification.md`（slice 記録追記）
- `docs/{discovery,specs,designs,plans,tasks,test-specs,adr,impl/ir,impl/exp}/issue-631-evidence-fields.*`

## Done When

- [ ] 全 real case の `environment.window_identities` に
  session 開始時の per-window identity（id・title・geometry・
  sha256、失敗時 `null`）が記録される。
- [ ] 全 pixel capture step row に `capture_sha256`・`backend`・
  `width`・`height` が記録され、opt-in artifact は
  `{path, sha256}` object になる。
- [ ] `real-capture` が `--pixel-artifacts` で PNG を書け、
  canonical lane は artifacts を書かない。
- [ ] `real-window-mount`・`real-command-receipt`・headless
  `command-receipt` の `observed.semantic_tree` が mount 済み
  spec の全 component key（alias 含む）を持ち、観測失敗 key
  は `null`。
- [ ] README verdict 表が `REAL_CATALOG`/実挙動と一致し、
  `real-command-receipt` 行を含む 22 行になる。
- [ ] `plugin-dock-real-headless.jsonl` が全行 `NotRun`+
  `environment_blocked` を exit 0 で維持して再生成される。
- [ ] interactive Windows desktop があれば
  `plugin-dock-real-scenario.jsonl` が新 field 込みで再生成
  される（不可なら NotRun として正直に記録）。
- [ ] 非回帰 lane が green、`verification.md` に記録が追記される。

## Verification

- [ ] `mise run fmt-check` — pass
- [ ] `mise run check/lint/test ruviform-plugin-dock-test`
  （default/`real-dock`/`lab-gui`）— pass
- [ ] `mise run test-ui-plugin-dock` / `-console` / `-lab` —
  pass
- [ ] `mise run test-ui-plugin-dock-real-headless` — pass（exit 0）
- [ ] `mise run test-ui-plugin-dock-real` — interactive desktop
  あれば実走（`real-dcomp-pixel` 既知 Fail による exit 1 は
  設計どおり）、不可なら NotRun 記録
- [ ] `mise run test-ui-scenario-contract` — pass
- [ ] `mise run check-ui-driver-boundary` — pass
- [ ] `mise exec -- prek run --all-files` — pass

## Notes

- canonical `--pixel-artifacts` 化は採用しない（PNG binary
  churn — ADR `docs/adr/issue-631-evidence-fields.md` に記録）。
- GPU 情報は `windows-sys` feature 追加または GPUI
  renderer-info API が必要で本 slice 対象外 — missing_issue
  候補として IR/PR に記録。
- lease 外 stale コメント（`bin/plugin-dock-console.rs:9`・
  `src/lib.rs:53-54`）は残件化。
- docs index（`docs/**/README.md`）は Coordinator が統合時に
  再生成 — Worker は編集しない。
