---
id: "IMPL-x7Qw2eRt5Yb8nKj3Hs6Pf"
type: "impl"
status: "completed"
title: "Issue #631 AC4 evidence-fields 完成（window hash・capture/artifact digest・semantic tree 全量）"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-631-evidence-fields.md"
      - type: "file"
        path: "docs/specs/issue-631-evidence-fields.md"
      - type: "file"
        path: "docs/designs/issue-631-evidence-fields.md"
      - type: "file"
        path: "docs/plans/issue-631-evidence-fields.md"
      - type: "file"
        path: "docs/tasks/issue-631-evidence-fields.md"
      - type: "file"
        path: "docs/test-specs/issue-631-evidence-fields.md"
      - type: "file"
        path: "docs/adr/issue-631-evidence-fields.md"
      - type: "file"
        path: "docs/impl/ir/issue-631-evidence-fields.md"
      - type: "file"
        path: "docs/impl/exp/issue-631-evidence-fields.md"
      - type: "file"
        path: "docs/impl/exp/issue-631-evidence-fields-exp.jsonl"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/bin/plugin-dock-real-scenario.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/plugin_dock.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "evidence/rf631/plugin-dock-real-scenario.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-real-headless.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-scenario.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console.jsonl"
      - type: "file"
        path: "evidence/rf631/verification.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-631-evidence-fields.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../discovery/issue-631-evidence-fields.md"
    - "../../specs/issue-631-evidence-fields.md"
    - "../../designs/issue-631-evidence-fields.md"
    - "../../plans/issue-631-evidence-fields.md"
    - "../../test-specs/issue-631-evidence-fields.md"
    - "../../adr/issue-631-evidence-fields.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-631-evidence-fields.md"
    rejected: []
---

# Issue #631 AC4 evidence-fields 完成

## Summary

Issue #631 AC4 の残 evidence field を manifest 変更不要の
部分集合で完成。`crates/ruviform-plugin-dock-test` に閉じ、
catalog 件数（headless 9 + real 22）・`Cargo.toml`/`Cargo.lock`・
production crate は不変。interactive Windows desktop で real
lane を実走し、`plugin-dock-real-scenario.jsonl` を新 field
込みで再生成した（`real-dcomp-pixel` の既知 `Fail` は
`ruviform#532` の正直な regression signal として据え置き、
exit 1 は設計どおり）。

## Context

Follows the Issue #631 doc chain: discovery → spec → design →
plan → task → test-spec → ADR。先行 slice（catalog flip・
real-command-receipt・headless-command-receipt）の着地済み
API のみを使用 — 新規 driver API・依存・feature 追加なし。

## Implementation

### window identity digest（`environment.window_identities`）

- `RealDockContext` に `window_identities: Value` を追加。
- `run_all_with_pixel_artifacts` の session 開始部で window
  A–D 各々について `window_identity_entry` を 1 回だけ呼び、
  `window_id`・mount title・session 開始時 `window_geometry`
  （origin・physical px・scale_factor）から canonical 文字列
  `"ruviform-plugin-dock-test window-identity v1|{window}|
  {title}|{origin_x},{origin_y},{width},{height},{scale_factor}"`
  を構成して `Sha256::digest`（`dpi_body` の
  `window_geometry_sha256` 方式踏襲 — あちらは case 内再観測
  digest、本 field は session 開始時 identity で別 field）。
- `finalize_case` の merged extras で `window_identities` を
  全 real case の `environment` に刻印（`session_scale_factor`
  と同パターン）。`blocked_reports`/`blocked_probe` は
  `finalize_real` 直行で session が存在しないため刻まない
  （fail-closed 維持）。
- geometry 読取失敗 window は `{geometry: null, sha256: null}`
  の正直な記録 — identity を捏造しない。

### capture/artifact digest

- `rgba_sha256(rgba: &[u8]) -> String` helper を追加 —
  `capture_sha256` の canonical digest（実 RGBA8 画素バイト）。
- `save_pixel_artifact` を `(dir, name, width, height, rgba)`
  に一般化し `PixelArtifact { path, sha256 }` を返す
  （sha256 は書き込んだ PNG bytes の digest）。
- `record_pixel_artifact` は `artifact: {path, sha256}` object
  を row に刻む（従来の path-only 文字列から shape 変更 —
  AC4 要求の additive 変更として verification.md に明記）。
- `capture_sha256` を全 capture row に追加:
  `real-capture` observed、`real-dcomp-pixel` の
  `screen_pixels`/`printwindow_pixels`/`windowdc_pixels`、
  `real-dcomp-foreground` の `fg_observe_*` screen object、
  `real-dcomp-present` の `pres_capture`/`pres_screen`。
- `--pixel-artifacts` を `real-capture` に拡張（opt-in —
  canonical lane は artifacts dir を渡さず PNG を書かない）。
  bin doc の対象 case 列挙を追従。

### semantic tree 全量化

- `scenario.rs` に `pub(crate) fn semantic_tree_json<G:
  SemanticGuiTest>(gui, window, view, spec) -> Map<String,
  Value>` を追加。`ViewHost::validate_spec`（`view.rs:1185`）
  と同じ walk: Panel children 再帰、Tabs は各 tab の alias
  key + child canonical key の両方を `resolve_target`+
  `observe`+`component_snapshot_json` で記録。観測失敗 key は
  `null`（best-effort evidence、verdict 入力ではない）。
- 適用箇所: real lane `real_window_mount_case`（従来の手書き
  2 key tree を置換）・real `command_receipt_body`（fader
  key のみ → 全 6 key）、headless `command_receipt_body`
  （同様）。
- `component_snapshot_json` の lane 間複製は維持（既知・
  ADR `issue-631-headless-command-receipt` 記録済み — 共有化は
  別 slice の判断事項）。

### doc 修復

- `README.md` verdict 表: `real-window-dpi`/`real-window-close`
  の stale `Unsupported` 行を catalog-flip 後の実 verdict
  記述に更新、`real-command-receipt` 行を catalog 順に追加
  （表は 22 行 = `REAL_CATALOG` と一致）。
- `plugin-dock-real-scenario.rs` bin doc: `--pixel-artifacts`
  対象に `real-capture` と `{path, sha256}`/`capture_sha256`
  を追記。
- `mise.toml` `test-ui-plugin-dock-real` description:
  "honest Unsupported for DPI/IME/close" の stale 文言を
  実挙動（live-geometry DPI verdict・armed-lifecycle close
  verdict・IME のみ Unsupported）に修正。canonical 引数は
  不変。

## Evidence 再生成

- `plugin-dock-real-scenario.jsonl`: interactive Windows
  desktop で `test-ui-plugin-dock-real` を実走（exit 1 =
  `real-dcomp-pixel` 既知 Fail による設計どおり）。全 22 case
  の `environment.window_identities`・capture row の
  `capture_sha256`・semantic_tree 全量を確認。
- `plugin-dock-real-headless.jsonl`: `--blocked-probe` で
  再生成（exit 0、全 22 row `not_run`+`environment_blocked`、
  session-scoped field 非記録を維持）。
- `plugin-dock-scenario.jsonl`/`plugin-dock-console.jsonl`:
  `test-ui-plugin-dock`/`test-ui-plugin-dock-console` lane が
  再生成（headless `command-receipt` の semantic_tree 全量を
  含む）。

## Verification

`evidence/rf631/verification.md` の slice 追記を参照 —
fmt-check・check/lint/test（default/real-dock/lab-gui）・
test-ui-plugin-dock(-console/-lab/-real-headless)・
test-ui-scenario-contract・check-ui-driver-boundary・prek
全て実走・pass。`test-ui-plugin-dock-real` は exit 1
（`real-dcomp-pixel` 既知 Fail）で evidence 再生成済み。

## Follow-ups / 残件

- **GPU 情報（AC4 残 field）**: `windows-sys` の
  `Win32_Graphics_Dxgi` feature 追加（manifest 変更 =
  ask-first）または GPUI renderer-info API（現存しない）が
  必要 — missing_issue 候補。
- **lease 外 stale コメント**: `bin/plugin-dock-console.rs:9`
  の 'twenty-one catalog rows'、`src/lib.rs:53-54` の
  DPI/IME/close `Unsupported` 記述（IME 以外は flip 済みで
  stale）— 本 slice の lease 外のため残件化。
- **`component_snapshot_json` lane 間複製**: scenario.rs と
  real_dock.rs に同型 helper が残る — 共有化は別 slice。
- **docs index**: `docs/**/README.md` は Coordinator が統合時に
  `doc-maintenance` で再生成。
