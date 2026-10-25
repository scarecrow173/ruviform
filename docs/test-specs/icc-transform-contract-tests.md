---
id: "TSPEC-034LFCoZvNuuSaGpSTrbp0"
type: "test-spec"
status: "approved"
title: "IccTransform ICC 経路 契約テスト仕様"
created: "2026-10-25"
updated: "2026-10-25"
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
    - "docs/designs/color-conversion-kernels.md"
  derived-by: []
  verifies:
    - "docs/specs/color-conversion-kernels.md"
    - "docs/designs/color-conversion-kernels.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# IccTransform ICC 経路 契約テスト仕様

> status: approved — Issue #99 の実装（Task 4 + Task 6 full gate・feature `icc` lane 全 green）で契約が実結果により確認済み。run 可能コマンド（`--features icc` 選択列を含む）は task 側に定義する（本書は契約と oracle の正本）。

## Purpose

spec `docs/specs/color-conversion-kernels.md`・approved design `docs/designs/color-conversion-kernels.md` に基づき、Cargo feature `icc` 配下の `IccTransform`/`IccIntent`（moxcms 0.9 optional 依存・private implementation 閉じ込め）の受入契約を固定する — profile parse/executor 構築の非 RT ctor 分離・alloc-free `process`・moxcms pin 版への bit-exact 等価性・`CmsError` → `InvalidValue` 写像・feature 無効時の外部依存ゼロ。ICC 経路の実装が置換・再設計された場合かつ同等の代替保証がレビュー済みの場合に限り本 spec を retire する。working space カーネル群（`ToWorking`/`FromWorking`/`ToneMapSdr`・`RgbaF32` 拡張）は `color-working-space-kernel-tests.md` が担当する。

## Feature

`ruviform-video-proc` の `IccTransform` カーネル（`new(src_profile: &[u8], dst_profile: &[u8], intent: IccIntent) -> Result<Self>` + `process(src, dst)` — src/dst ∈ {`Rgba8`, `RgbaF32`} 任意組合せ）と `IccIntent`（`#[non_exhaustive]` enum `{Perceptual, RelativeColorimetric}`）。moxcms `ColorProfile::new_from_slice` + `create_transform_f32(Layout::Rgba, …)` 経路の feature `icc` 限定公開経路。

## Rules

- **feature 分離**: `IccTransform`/`IccIntent` は feature `icc` 有効時のみ公開。ベース経路（feature 無効）の依存は `ruviform-video-core`・`ruviform-media-core` のみで外部依存ゼロを維持し、`cargo test -p ruviform-video-proc`（feature 無し）が icc 配下コード・テストを含まず通る。moxcms の型・`CmsError`・feature 名は公開 API に露出しない（引数・戻り値・エラー文字列を通じても）
- **`IccIntent`**: `{Perceptual, RelativeColorimetric}` の `#[non_exhaustive]` enum — 内部で moxcms `RenderingIntent` へ写像。未知 variant（将来追加）は graceful に扱う
- **ctor（非 RT）**: `new` は両 profile を `ColorProfile::new_from_slice` で parse — 失敗は `InvalidValue("icc profile")`。`create_transform_f32` の失敗は `InvalidValue("icc transform")`。`TransformOptions` は `rendering_intent` = intent 写像値・`allow_use_cicp_transfer = true`・`prefer_fixed_point = false`（f32 executor の参照精度を固定小数に落とさない）・`allow_extended_range_rgb_xyz = true`・interpolation/barycentric は既定（`Linear`/`Low`）。profile parse + executor 構築の alloc は非 RT ctor 分離として許容
- **`process`（alloc-free）**: src/dst ∈ {`Rgba8`, `RgbaF32`} 任意組合せ・同一寸法・排他所有。画素毎に `[f32;4]` スタック配列へ正規化（`Rgba8` は `v/255`・`RgbaF32` は raw LE 読取 — moxcms は `[0,1]` 正規化入力を要求）→ executor `transform(&[src_px], &mut [dst_px])` を **1 画素単位の 1 要素スライス**で呼ぶ（行バッファ確保を避け alloc-free を構造保証 — バッチ経路は最適化 backend 論点）→ 出力エンコード（`RgbaF32` raw 書出し・`Rgba8` は `round(clamp(v,0,1)·255)`）。executor `transform` のエラーは `InvalidValue("icc transform")`
- **alpha**: moxcms `Layout::Rgba` の alpha パススルー経路を使い、alpha は変換しない（実装時に alpha 非変換を検証し doc に規約明記）
- **`ColorInfo` 更新**: `dst.color = {primaries: Unspecified, transfer: Unspecified, matrix: Identity, range: Full, hdr: None}` — ICC profile が記述する色空間は CICP enum へ写像できないため unspecified が正直な記録（consumer が既知なら `set_color` で上書き）。`pts`/`duration` は `src` から伝播
- **検証/エラー規約**: 全検証は書込み前 — 共有 `dst` は `InvalidValue("dst plane")`・format 不適合は `InvalidValue("src format")`/`"dst format"`・寸法不一致は `InvalidValue("dimensions")`。拒否呼出しは `dst` を一切変更しない（panic-free・部分書込みなし）
- **等価性契約**: pin した moxcms バージョン + feature 集合（`default-features = false` + `lut` + `extended_range`・SIMD 系無効）の出力が参照 — executor は同一 build 構成内で決定的（スカラー経路のみだが、moxcms `mlaf` が aarch64 NEON baseline・`target_feature = "fma"` x86/x86_64 で fused FMA を選択するため、bit 出力は target-feature/architecture 間で異なり得る）。正規化/量子化 shim は厳密なため golden vector は同一 build 構成上で executor 出力に対し **bit-exact** で検証する — literal golden vector は x86_64 非 FMA baseline（CI `icc-verify` lane）に pin し `cfg` でゲートする。moxcms バージョン/feature 変更時は golden を再導出する。クロスエンジン等価性（lcms2/qcms との一致）は契約外
- **extended range**: `RgbaF32` dst は moxcms extended-mode 出力（負・1 超を保持 — HDR/広色域 profile 変換の情報損失を防ぐ）、`Rgba8` dst は量子化 clamp
- **RT 契約**: `process` は `**RT contract:**` ラベル付き alloc-free・非ブロッキング・有界 — moxcms executor が事前構築パイプライン上の純粋写像である設計に依拠（実装レビューで確認）。`rt_process_*` 命名テスト（repeatable/deterministic・ソース非破壊 — alloc 機械検出は別 Issue）を持ち、feature `icc` 有効時の `mise run test-rt` 相当経路で選択可能。ctor は非 RT
- **検証経路の別レーン**: `cargo test --workspace`（default feature）は icc 配下を検証しない — `cargo test -p ruviform-video-proc --features icc` を実行する mise タスク（plan が `test-video-proc-icc`/`lint-video-proc-icc` を定義）と CI ステップが存在し、feature 有効列が実際に選択・実行される

## Examples

- **sRGB→sRGB identity 相当**: 同一 sRGB ICC profile（fixture バイト列）の `IccTransform` が `Rgba8`→`Rgba8` で executor 参照出力と bit-exact — golden vector を pin 版 moxcms（x86_64 非 FMA baseline）で導出・同梱し、literal golden テストは `target_arch`/`target_feature` の `cfg` でゲート
- **`Rgba8`→`RgbaF32` / `RgbaF32`→`Rgba8` / `RgbaF32`→`RgbaF32`**: 端点組合せの golden — `RgbaF32` 出力は extended range で [0,1] 外を保持、`Rgba8` 出力は `round(clamp·255)`（± 境界は厳密）
- **ctor 拒否**: 不正/切詰 profile バイト列 → `InvalidValue("icc profile")`（`dst` ・フレーム不関与）。transform 構築不能な profile 組合せ → `InvalidValue("icc transform")`
- **alpha 非変換**: alpha = 0.5 系の入力画素で出力 alpha が入力と一致（moxcms alpha パススルー規約の実証）
- **`ColorInfo` 記録**: dst が `{Unspecified, Unspecified, Identity, Full, hdr: None}`・`pts`/`duration` 伝播 — consumer 上書きは `set_color` で可能
- **共通拒否**: 非対象 format・寸法不一致・共有 `dst` backing → `InvalidValue` で `dst` 部分書込みなし。`stride > row_bytes` パディングは不変
- **feature 無効**: `--features icc` なしのクレートに `IccTransform`/`IccIntent` が露出しない（`use` 文のコンパイル失敗で構造検証 — feature-gated 公開面の契約）
- **`rt_process_*`（icc feature 有効時）**: 反復呼出しは決定的・panic せず `src` byte-identical

## Guarantees

- ctor の失敗は `Result<Self>` で返り、profile バイト列は呼出し側所有の `&[u8]` — executor 構築に失敗したオブジェクトは公開されない
- `process` の画素写像は pin 版 moxcms executor 出力に bit-exact — 正規化/量子化 shim は厳密であり、executor 以上の誤差を導入しない
- 全 `process` は検証を書込み前に完了し、拒否呼出しは `dst` を一切変更しない（panic-free・部分書込みなし — ADR D7）
- feature `icc` 無効のベース経路は外部依存ゼロ — `ruviform-video-core`・`ruviform-media-core` のみ（spec AC4）
- moxcms の型・エラー・feature 名は公開 API に露出しない — private implementation detail（rtrb 先例の閉じ込め）

## Non-goals

- ICC profile の供給経路（素材メタデータからの ICC 取得・profile 管理 UI — media-io/アプリ層責務）
- `ColorInfo` → CICP profile 便利コンストラクタ（`new_from_cicp` 経由 — design deferred）
- intent 拡張集合（Saturation/AbsoluteColorimetric）・黒点補償・`in_place`/`options`/`any_to_any`/SIMD 系 moxcms feature・CMYK/多チャネル layout
- CICP-described 変換の ICC 経路統一（`ToWorking`/`FromWorking` と `IccTransform` は独立経路 — 等価性主張なし）
- クロスエンジン等価性（lcms2/qcms 等との出力一致）・moxcms 以外の CMS バックエンド
- `assert_no_alloc` による alloc 機械検出（#97/#99 共通の後続 Issue 管轄）
- moxcms バージョン/feature 更新時の golden 再導出自動化（手順は design deferred — pin + `Cargo.lock` で現状固定）

## Risk

- moxcms API の誤用（`Layout` 不一致・per-pixel 呼出しでなく行バッファ確保する実装）→ alloc-free 契約と alpha 規約の退行 — per-pixel スライス呼出し構造と alpha 非変換テストで固定
- `prefer_fixed_point = true`/`extended_range` 無効化の紛れ込み → 参照精度・[0,1] 外保持の喪失 — TransformOptions 値を contract で固定
- `CmsError` や moxcms 型の公開 API 露出 → 依存閉じ込め規約の破綻 — 公開面 grep/コンパイル形状で固定
- feature `icc` 経路が default 検証対象外のまま regress → `cargo test --workspace` が icc コードをカバーしない構造的事実を mise タスク/CI の別レーンで固定（未実施なら検証列は base のみ意味を持つ — design が義務として記録）
- golden vector が別バージョン/feature 集合で導出される → 等価性契約の基盤が暗黙変化 — pin 版識別と再導出規則を固定
- 非 RT ctor と `process` の分離崩れ（`process` 内での lazy 構築・parse）→ RT 契約違反 — ctor 焼付け構造と `rt_process_*` で固定
