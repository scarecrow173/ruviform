---
id: "DSGN-n3Ws6xBr4Ym8kTf2hQv9Jc"
type: "design"
status: "approved"
title: "Issue #631 AC4 evidence-fields 完成 — 設計"
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
    - "docs/specs/issue-631-evidence-fields.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 AC4 evidence-fields 完成 — 設計

## 概要

Spec `docs/specs/issue-631-evidence-fields.md` の field を、
既存 API・既存依存（`sha2`）のみで `crates/ruviform-plugin-dock-
test` に実装する。変更点は 4 系統:

1. session-scoped per-window identity digest を全 real case の
   `environment` に刻む
2. 全 pixel capture に `capture_sha256`・PNG artifact を
   `{path, sha256}` に拡張し `real-capture` の opt-in
   artifact 対応を追加
3. `observed.semantic_tree` を mount 済み `ViewSpec` の全
   component key に拡張（real lane 2 case + headless
   `command-receipt`）
4. README verdict 表・bin doc・`mise.toml` description の
   stale 修復

## Window identity digest

### 収集点

`run_all_with_pixel_artifacts`（`real_dock.rs`）で window A–D
mount 直後・`session_scale_factor` 計測と同じ位置で、各
window について以下を集める:

- `gui.window_geometry(window_id)` → `OsWindowGeometry`
  （origin・physical px・scale factor）
- `WindowId` の `Display`（`w1` 等の安定文字列）
- mount 時に lane が指定した title（`WindowSpec.title` —
  A は default "ruviform"、B/C/D は各 mount の title）

`window_geometry` の読み取りは case 実行とは別に session
開始時に 1 回だけ行う。読み取りに失敗した window は entry
を残しつつ `sha256: null` とする（identity を捏造しない）。

### canonical 文字列と digest

`dpi_body`（`:6135-6150`）の `window_geometry_sha256` と
同方式 — deterministic な canonical 文字列を組み立て、
`Sha256::digest` でハッシュ化する。canonical 文字列は:

```
ruviform-plugin-dock-test window-identity v1
window_id: {id}
title: {title}
geometry: {geometry_json 相当の決定的表現}
```

geometry の表現は `geometry_json`（case row 用の既存
serialization）と同じフィールド順・同じ値を使い、float
の書式が run 間で安定するよう既存の整形を再利用する。

### 保持と stamp

- `RealDockContext` に `window_identities: Vec<WindowIdentity>`
  （または `serde_json::Value` の map）を追加する。
- `finalize_case` の merged extras（`session_scale_factor`
  と同パターン、`:473-485`）で全 real case の
  `environment.window_identities` に刻む。
- `blocked_reports`/`blocked_probe` は session を持たない
  ため刻まない（field 自体が出ない = 正直な非記録）。
- `real-window-dpi` の `window_geometry_sha256` は当該
  case の再観測 digest であり変更しない。

## Capture / artifact digest

### capture_sha256

実際に capture した RGBA 画素バイトの SHA-256 を、対象の
全 pixel capture step row に `capture_sha256` として記録
する。対象と画素ソース:

- `real-capture`（`observed` の `capture_frame` 結果 —
  `RgbaImage::pixels()` の `&[u8]` RGBA8）
- `real-dcomp-pixel` の `screen_pixels`/`printwindow_pixels`/
  `windowdc_pixels`（`ospix::PixelCapture.rgba`）
- `real-dcomp-foreground` の `fg_observe_*` screen object
- `real-dcomp-present` の `pres_capture`/`pres_screen`

`backend`/`width`/`height` は既存 field で記録済みのものは
そのまま、未記録の row には併記する。

### artifact {path, sha256}

`save_pixel_artifact` を `(width, height, rgba)` 引数に
一般化し、`ospix::PixelCapture` だけでなく `RgbaImage`
（`capture_frame` の戻り値）も同じ writer に通せるように
する。戻り値を `path` のみから `{path, sha256}` に拡張し
（書き込んだ PNG bytes の digest — `artifact::sha256_hex`
または `Sha256::digest` を使用）、`record_pixel_artifact`
経由で全 caller の row に `artifact: {path, sha256}` object
を刻む。書込失敗時は従来どおり `artifact_error` field で
verdict を変えない。

`--pixel-artifacts` は opt-in のまま — canonical
`test-ui-plugin-dock-real` は artifacts dir を渡さない。
`real-capture` も `ctx.pixel_artifacts` を見て同じ
`save_pixel_artifact` helper で PNG を書く。

## Semantic tree 全量化

### 共有 helper

`scenario.rs` に `SemanticGuiTest` trait generic の helper
を追加する（real lane・headless lane が共有 — lane 間で
spec 構造と observe 手順を揃えるため、cfg 境界の重複は
既知のまま共有 helper 1 箇所に集約する）:

```rust
fn semantic_tree_json<G: SemanticGuiTest>(
    gui: &mut G,
    view: &ViewSpec,
) -> serde_json::Map<String, serde_json::Value>
```

walk は `ViewHost::validate_spec`（`view.rs:1185`）と同じ:

- root `ComponentSpec` から再帰
- `ComponentSpec::Panel { children, .. }` → 各 child を再帰
- `ComponentSpec::Tabs { tabs, .. }` → 各 tab の alias key
  と `tab.child` の canonical key の両方を列挙し、child を
  再帰
- 各 component の `key()` を `resolve_target`+`observe` し
  `component_snapshot_json` で記録
- resolve/observe 失敗は `null`（`:1204-1214` の先例）

### 適用箇所

- real lane `real-window-mount` — 現行の `plugin.dock`+
  slot 2 key 手動列挙を helper に置き換える（結果は同じ
  2 key だが spec からの導出になる）。
- real lane `real-command-receipt` — 現行 `fader-ch1` のみを
  helper で全量化（`sv_fader_dock_fixture` は
  `panel`/`mixer-tabs`/`Ch 1`/`fader-ch1`/`Ch 2`/`fader-ch2`
  の 6 key）。
- headless `command_receipt_body` — 同じ helper を使い
  `semantic_tree` を全量化（fake driver の
  `resolve_target`/`observe` は alias/canonical 両 key で
  snapshot を返すことを確認済み）。

verdict ロジックとは分離した best-effort evidence — 観測
失敗で case の verdict を変えない。

## README / doc 修復

- `README.md` verdict 表: `real-window-dpi`（live geometry
  観測判定）・`real-window-close`（armed Veto/Defer→Approve/
  既定 Approve の実観測）を現行実装に合わせ、
  `real-os-input` と `real-window-dpi` の間に
  `real-command-receipt` 行を catalog 順で追加 → 22 行。
- `bin/plugin-dock-real-scenario.rs` doc の `--pixel-artifacts`
  対象 case 列挙に `real-capture` を追加。
- `mise.toml` `test-ui-plugin-dock-real` description の
  "honest Unsupported for DPI/IME/close" を実態に合わせる
  （DPI/close は実 verdict 化済み・IME のみ Unsupported）。
  canonical 引数・依存・lock は不変。

## テスト方針

- `real_dock.rs` `#[cfg(all(test, feature = "real-dock"))]`:
  - window identity helper の canonical 文字列→digest が
    deterministic であること（同入力→同 digest・異入力→
    異 digest）。
  - semantic tree helper が `sv_fader_dock_fixture` で
    alias/canonical 両 key を含む全 6 key を列挙し、
    観測失敗 key が `null` になること。
  - `save_pixel_artifact`/`record_pixel_artifact` が
    `{path, sha256}` を返す/刻むこと（tmpdir に PNG を書き
    bytes digest と一致を検証）。
- `tests/plugin_dock.rs`（headless feature）:
  - headless `command-receipt` report の
    `observed.semantic_tree` が全 6 key を持ち各値が
    snapshot object（または失敗時 `null`）であること。
- `tests/console.rs`:
  - 件数定数（31）は不変 — semantic tree 全量化・新 field
    追加で行数・case 数は変わらないことを確認する既存
    断言が green を維持。

## Evidence 再生成

- `mise run test-ui-plugin-dock-real-headless` — `--blocked-
  probe` で `plugin-dock-real-headless.jsonl` を再生成
  （headless で決定的・必須）。新 field は session 不在の
  ため刻まれず、全行 `NotRun`+`environment_blocked` を
  exit 0 で維持することを確認。
- `mise run test-ui-plugin-dock-real` — interactive Windows
  desktop があれば実走して `plugin-dock-real-scenario.jsonl`
  を新 field 込みで再生成。不可なら NotRun として正直に
  記録し、既存 JSONL を environment_blocked で上書きしない。
- `evidence/rf631/verification.md` に本 slice の検証記録を
  追記。

## リスクと対策

- `window_geometry` が session 開始時に失敗する window —
  `sha256: null` で正直に記録（identity 捏造禁止）。
- `capture_frame`/`ospix` の capture が空・失敗 — 既存の
  fail-closed verdict を変えず、成功 capture のみ digest
  を刻む。
- semantic tree walk が予期しない `ComponentSpec` variant
  を持つ spec — `validate_spec` と同じ網羅 match で
  将来 variant 追加時にコンパイルエラーで気付ける。
- interactive desktop 不在 — real-scenario JSONL は未再生成
  として verification.md/PR に明記（偽 Pass 禁止）。
