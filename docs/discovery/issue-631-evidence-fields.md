---
id: "DISC-x7Kp2mQwR9vTn4bYc6dF"
type: "discovery"
status: "draft"
title: "Issue #631 AC4 evidence-fields 完成 slice"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 AC4 evidence-fields 完成 slice

## Summary

Issue #631 AC4（Screenshot/Pixel diff/semantic tree/Command+Receipt/
OS and GPU/Case Seed/Window & Plugin hash、Repo/SHA・Ruviform
resolved SHA の保存）の残 field を、manifest 変更不要の部分集合で
完成させる slice の探索記録。対象は `crates/ruviform-plugin-dock-test`
に閉じ、catalog 件数（headless 9 + real 22）・`Cargo.toml`/
`Cargo.lock`・production crate は不変。結論: window identity
digest・capture/artifact digest・semantic tree 全量・README 修復は
全て既存 API・既存依存（`sha2`）の範囲で実装可能（confirmed —
コード精読済み）。GPU 情報のみ `windows-sys` feature 追加または
GPUI renderer-info API が必要で本 slice 対象外。

## Exploration Goal

AC4 evidence field の残りを fail-closed・帰属可能な形で JSONL
evidence に刻む:

- **window hash 全行**: session 開始時に window A–D 各々の
  `window_geometry` 初回値 + window id + mount title から
  SHA-256 を計算し、全 real case の `environment` に刻む。
- **capture/artifact digest**: 全 pixel capture step row に
  `capture_sha256`（+`backend`/`width`/`height`）を記録し、
  opt-in PNG artifact を `{path, sha256}` 形に拡張。`--pixel-artifacts`
  を `real-capture` にも拡張する。
- **semantic tree 全量**: `observed.semantic_tree` を記録する
  case（`real-window-mount`・`real-command-receipt`・headless
  `command-receipt`）で mount 済み `ViewSpec` の `ComponentSpec`
  children を walk し全 component key を observe する。
- **README 修復**: stale verdict 行（`real-window-dpi`/
  `real-window-close`）修復 + `real-command-receipt` 行追加。

## Key Issues and Assumptions

- window identity は「case が使う window」ではなく session-scoped
  の per-window digest を全 case の `environment` に刻む設計が
  `session_scale_factor` 先例（`finalize_case` merged extras）と
  一致し最も単純 — confirmed: `finalize_case` は case 非依存の
  session fact を stamp する既存 wrapper。
- `window_geometry` は title を返さない（`OsWindowGeometry` は
  origin/physical px/scale factor のみ）— title は mount 時の
  `WindowSpec`（lane が自ら指定した値）を canonical 文字列に
  使う。confirmed: `run_all_with_pixel_artifacts` が B/C/D の
  `WindowSpec` を構築し、A は `WindowSpec::default()`（title
  "ruviform"）。
- `blocked_reports`/`blocked_probe` 経路は session を持たない —
  session-scoped field を刻まず `null`/非記録のままが正直。
  confirmed: `blocked_reports` は `finalize_real`（`finalize_case`
  ではない）を呼ぶ。
- `real-window-dpi` の `window_geometry_sha256` は当該 case の
  再観測 geometry digest であり、本 slice の session 開始時
  window identity とは別 field — 変更しない。confirmed:
  `dpi_body` は case 内で 2 回 `window_geometry` を読み直す。
- capture digest は実際に capture した RGBA 画素バイトの
  SHA-256 — `ospix::PixelCapture.rgba`（dcomp 系）と
  `RgbaImage::pixels()`（`real-capture` の `capture_frame`）の
  両方に適用可能。confirmed: `RgbaImage::pixels()` は `&[u8]`
  RGBA8 を返す。
- `save_pixel_artifact` は `ospix::PixelCapture` を取り PNG を
  書く唯一の境界 — `{path, sha256}` はここの戻り値を拡張して
  全 caller（`record_pixel_artifact` 経由の全 step row）に
  波及させる。confirmed: `save_pixel_artifact` の caller は
  `record_pixel_artifact` のみ。
- `real-capture` の `--pixel-artifacts` 対応は `RgbaImage` を
  `save_pixel_artifact` に渡せるよう引数を `(width, height,
  rgba)` に一般化すれば既存 helper を再利用できる — 新規
  writer は不要。
- semantic tree 全量化は `ViewHost::validate_spec` の flat 列挙
  （`view.rs:1185`）と同じ walk — Panel `children` 再帰 +
  Tabs は tab alias key と canonical child key の両方を列挙。
  `ViewHost::mounted` は alias key でも mount されるため
  `observe` は alias/canonical 両 key で snapshot を返す。
  confirmed: `mount` は `validate_spec` の flat 全件を
  `mounted.insert` する。
- `sv_fader_dock_fixture().spec` は Panel>Tabs>2×Fader で
  mounted key は `panel`/`mixer-tabs`/`Ch 1`/`fader-ch1`/
  `Ch 2`/`fader-ch2` の 6 件 — 現行 `semantic_tree` は
  `fader-ch1` のみで大幅に不完全。confirmed: `lab_story.rs`。
- `plugin_dock_view_spec()` は Panel>NativeSlot で mounted key
  は `plugin.dock`/`plugin.editor` の 2 件 — 現行 mount case の
  2 key 記録は偶然全量と一致するが、spec からの導出に変えると
  fixture 変更時に自動追従する。confirmed: `plugin_dock.rs`。
- observe 失敗は `null` 記録（`:1204-1214` 先例）— verdict
  ロジックとは分離した best-effort evidence。新規 driver API
  不要（`resolve_target`+`observe` は `SemanticGuiTest` trait
  の既存 method）。
- canonical `--pixel-artifacts` 配線（`test-ui-plugin-dock-real`
  に artifacts dir を常時追加）は**採用しない** — `*.png` は
  `.gitattributes` で `binary`（LFS 非対象）のため canonical
  lane が毎回 PNG をコミットすると Git 履歴に純バイナリ churn
  が積み上がる。`capture_sha256` があれば画素を保存せず run
  間 diff が可能。opt-in artifacts + canonical digest を採用。
- GPU 情報は `windows-sys` の `Win32_Graphics_Dxgi` feature 追加
  （manifest 変更 = ask-first）か GPUI renderer-info API（現存
  しない）が必要 — 本 slice 対象外、missing_issue 候補として
  IR/PR に記録。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| window identity を case ごとの使用 window のみ stamp | environment が小さい | close case は B/C 2 window・全 case 網羅に case→window 対応表が必要で plumbing が増える | no |
| session 全 window の identity map を全 case に stamp（`session_scale_factor` 先例） | 単純・全行網羅・case→window 対応不要・監査時に session の window census が見える | environment が window 数分増える（4 件のみ） | yes |
| `environment.windows` に生 geometry のみ記録し hash なし | 実装最小 | AC4 は hash 保存を要求 — digest が本体 | no |
| capture digest を PNG artifact の hash に一本化 | field が減る | canonical run は PNG を書かない決定 — 画素バイト自体の digest が必須 | no |
| semantic tree を spec 全文 serialize | 実装最小 | mounted/snapshot 状態（display/committed/phase）を含まず AC4 の semantic 観測証拠にならない | no |
| canonical lane に `--pixel-artifacts` を配線 | PNG が毎回残る | `*.png` binary churn・opt-in 方針と矛盾 | no |

## Tentative Conclusions and Hypotheses

- confirmed: window identity は session 開始時に window A–D
  各々について `window_geometry`（origin・physical px・scale
  factor）+ `WindowId` + mount title を canonical 文字列化し
  `Sha256::digest`（`dpi_body:6135-6150` 方式踏襲）で 1 回だけ
  計算、`RealDockContext` に保持 → `finalize_case` の merged
  extras で全 real case の `environment.window_identities` に
  刻む。geometry 読取失敗の window は `sha256: null` の正直な
  記録（捏造しない）。
- confirmed: capture digest は `Sha256::digest(rgba bytes)` を
  全 pixel capture step row の `capture_sha256` に記録 —
  `real-capture`（`capture_frame` の `RgbaImage::pixels()`）、
  `real-dcomp-pixel` の `screen_pixels`/`printwindow_pixels`/
  `windowdc_pixels`、`real-dcomp-foreground` の `fg_observe_*`
  screen object、`real-dcomp-present` の `pres_capture`/
  `pres_screen` row。
- confirmed: `save_pixel_artifact` を `(width, height, rgba)`
  引数に一般化し `{path, sha256}`（書いた PNG bytes の digest）
  を返す — `record_pixel_artifact` が `artifact: {path, sha256}`
  object を row に刻む。`real-capture` は `ctx.pixel_artifacts`
  を見て同 helper で PNG を書く。
- confirmed: semantic tree は `scenario.rs` に
  `SemanticGuiTest` generic の helper として実装し、
  `ViewHost::validate_spec` と同じ walk（Panel children 再帰 +
  Tabs alias/canonical 両 key）で全 key を `resolve_target`+
  `observe` → `component_snapshot_json`、失敗は `null`。
  `real-window-mount`・`real-command-receipt`（real lane）と
  headless `command-receipt` が共有する。
- confirmed: README verdict 表は `real-window-dpi`（live
  geometry 観測判定）・`real-window-close`（armed
  Veto/Defer→Approve/既定 Approve の実観測）を現行実装に
  合わせ、`real-os-input` と `real-window-dpi` の間に
  `real-command-receipt` 行を catalog 順で追加する。
- confirmed: `mise.toml` `test-ui-plugin-dock-real` の
  description が "honest Unsupported for DPI/IME/close" のまま
  stale — 実 verdict 化済みの DPI/close を反映する文言修正は
  lease 内の正直な修復として行う（canonical 引数は不変）。

## Open Questions

- interactive Windows desktop での real lane 実走可否 — 本
  worker 環境が interactive session を持つかは実走で確認し、
  不可なら `plugin-dock-real-scenario.jsonl` 再生成を NotRun
  として正直に記録する（headless 側は必須実行）。

## Promotion Candidates

- [x] spec-doc needed: `docs/specs/issue-631-evidence-fields.md`
  — field 形状・fail-closed 規範・opt-in artifacts 決定を確定する。
- [x] adr-doc needed: `docs/adr/issue-631-evidence-fields.md`
  — opt-in artifacts + canonical digest の採用と semantic tree
  helper の lane 共有配置を記録する。

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — auto-close しない）。
- `docs/discovery/issue-631-real-window-catalog-flip.md` —
  `session_scale_factor`/`finalize_case`/`case_seed` 先例。
- `docs/discovery/issue-631-command-receipt.md`・
  `docs/adr/issue-631-headless-command-receipt.md` —
  command-receipt の real/headless 対応構造。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs` — 変更対象
  本体（context/finalize/capture helpers/case bodies/tests）。
- `crates/ruviform-plugin-dock-test/src/scenario.rs` — headless
  `command_receipt_body` + 共有 helper 配置先。
- `crates/ruviform-ui/src/view.rs` — `ComponentSpec`/`ViewSpec`/
  `validate_spec` walk 先例。
- `crates/ruviform-ui-test/src/lab_story.rs` —
  `sv_fader_dock_fixture` spec 構造。
- `crates/ruviform-ui-test/src/scenario/plugin_dock.rs` —
  `plugin_dock_view_spec` 構造。
- `POLICIES.md` — GUI-L2・Pass/Fail/NotRun/Unsupported・
  fail-closed 規範。

## Evidence

### 2026-11-18 — 実装と検証

- 実装結果・検証 lane・evidence 再生成状況は
  `docs/impl/ir/issue-631-evidence-fields.md`・
  `evidence/rf631/verification.md` を参照。<|end_message|><|message_model|>exec<|content_invoke_tool_json|>{
