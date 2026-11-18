# Issue #631 evidence-fields — 実装実験記録（2026-11-18）

`docs/tasks/issue-631-evidence-fields.md` の実装で
試行・観測した事項の記録。実装結果の正本は
[`docs/impl/ir/issue-631-evidence-fields.md`](../ir/issue-631-evidence-fields.md)、
イベントログは
[`issue-631-evidence-fields-exp.jsonl`](issue-631-evidence-fields-exp.jsonl)
を参照。

## 試行 1: window identity digest の canonical 文字列設計

- **目的**: session 開始時の window A–D identity を run 間で
  比較可能な digest として全 real case に刻む。
- **観測**: `real-window-dpi` の `window_geometry_sha256`
  （`dpi_body`）は case 内の再観測 geometry digest であり、
  session 開始時 identity とは目的が異なる — 同一 canonical
  方式（window id + 幾何 + `Sha256::digest`）を踏襲しつつ、
  mount title を canonical 入力に追加して window 間の区別を
  保証した。`OsWindowGeometry` は title を持たないため、
  title は mount 時の `WindowSpec` から取得する必要があった
  （`WindowSpec::default().title` = window A の "ruviform"、
  B/C/D は明示 title）。
- **結論**: `window_identity_entry(window, title,
  Option<OsWindowGeometry>)` helper を導入し、geometry 失敗時は
  `{geometry: null, sha256: null}` の正直な記録に留める設計を
  採用。`RealDockContext.window_identities: Value` に census
  を保持し `finalize_case` で刻印 — `session_scale_factor`
  の merged extras パターンを踏襲。

## 試行 2: `save_pixel_artifact` の一般化範囲

- **目的**: `ospix::PixelCapture` 専用だった artifact writer を
  `real-capture` の `RgbaImage` にも使える形にする。
- **観測**: `capture_frame` は `RgbaImage`（`pixels() ->
  &[u8]`）を返し、`ospix` 系 capture は `PixelCapture`
  （`rgba: Vec<u8>`）を返す — 共通するのは `(width, height,
  rgba bytes)` のみ。`RgbaImage::new` は `Vec<u8>` を要求する
  ため `&[u8]` 入力は `to_vec()` が必要。
- **結論**: `save_pixel_artifact(dir, name, width, height,
  rgba: &[u8]) -> Result<PixelArtifact, String>` に一般化し、
  `PixelArtifact { path, sha256 }`（書き込んだ PNG bytes の
  digest）を返す設計を採用。`record_pixel_artifact` の呼出側は
  `capture.width, capture.height, &capture.rgba` を渡す形に
  統一 — 5 箇所の call site を機械的に更新。

## 試行 3: semantic tree walk の alias/canonical 重複 observe

- **目的**: `observed.semantic_tree` を mount 済み spec の全
  component key に拡張する。
- **観測**: `ViewHost::validate_spec`（`view.rs:1185`）の flat
  列挙は Tabs で `(&tab.key, tab.child)` alias と
  `(child.key(), child)` canonical の両方を mounted に含める
  — semantic tree も同じ key 集合を持つべき。alias key での
  `resolve_target`+`observe` は Fake/Real 両 driver で child
  node の snapshot を返すことを unit test で確認（6 key 全て
  `contains_key`、`fader-ch1` は object）。
- **結論**: `scenario.rs` に `SemanticGuiTest` generic の
  `semantic_tree_json` を実装 — `component_snapshot_json`
  は lane ごとに既存のまま（cfg 境界の重複は既知・ADR 記録済み）。
  実 lane・headless lane の両 `command_receipt_body` と
  `real_window_mount_case` に適用。

## 試行 4: interactive desktop の有無判定

- **目的**: `test-ui-plugin-dock-real` の実走可否を確認する。
- **観測**: `test-ui-plugin-dock-lab` が `READY_TIMEOUT`(15s)
  を超えても終了しなかった — `WindowLifecycle::Ready` が
  到達したことを意味し、interactive Windows desktop が利用
  可能と判明。lab window を `CloseMainWindow()` で正常終了
  させた後、`test-ui-plugin-dock-real` を実走 — 4 window が
  mount され全 22 case が実 verdict を記録（18 pass・
  1 fail=`real-dcomp-pixel`・3 unsupported）。
- **結論**: real-scenario JSONL は実走 evidence として再生成
  済み — NotRun 記録は不要だった。

## 試行 5: lint/format の追従

- **観測**: `cargo fmt --check` で 4 件の整形差分（closure
  の 1 行化・`RgbaImage::new` の let 束縛・test の
  `save_pixel_artifact` 呼出・`semantic_tree_json` の
  `map_or` 改行）。`cargo fmt -p` で解消。
- **結論**: `mise run fmt-check` green。clippy `-D warnings`
  は初回から clean（新規 `unsafe`/`allow` なし）。
