---
id: "PLAN-034KSVb6BAzhMH2cZgxnSy"
type: "plan"
status: "approved"
title: "Issue #97 実施計画 — video-proc コンポジット/遷移カーネル追加"
created: "2026-10-23"
updated: "2026-10-23"
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
    - "docs/designs/composite-transition-kernels.md"
  derived-by:
    - "docs/tasks/composite-kernels.md"
    - "docs/tasks/affine-kernel-and-closeout.md"
  verifies: []
  verified-by:
    - "docs/test-specs/composite-transition-kernels-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #97 実施計画 — video-proc コンポジット/遷移カーネル追加

## Goal

`ruviform-video-proc` にコンポジット/遷移カーネル初期セット —
`CompositeOver`（source-over アルファ合成・`dst` in-place）・
`Crossfade`（2 ソース加重混合遷移）・`AffineTransform`（整数矩形写像 +
最近傍サンプリング）とパラメータ型 `Rect`/`RectMap` を実装する。

## Scope

単一実装ストリーム — 既存 `ruviform-video-proc` クレートへの
3 カーネル + 2 パラメータ型の追加。spec/design の Out of Scope
（GPU 経路・OFX 写像・追加 blend モード・linear-light 合成・フルアフィン/
回転・せん断・bilinear 以上のフィルタ・premultiplied 変種・planar YUV・
wipe 等の追加遷移・`ruviform-edit`/engine 配線・`assert_no_alloc` 導入）は
含めない。

## Upstream Documents

- docs/specs/composite-transition-kernels.md
- docs/adr/video-proc-kernel-api-and-color-hdr-scope.md（D1-b 二相 API・
  D2 合成カーネル予約・D7 エラーモデル）
- docs/adr/second-target-editing-model.md（D2-a コンポジット =
  video-proc カーネル）

## Design Inputs

- docs/designs/composite-transition-kernels.md（approved — カーネル集合・
  Q8 自己無撞着整数演算・f32/ulp-bound 等価性契約・検証/エラー規約・
  メタデータ規約・trap vector が確定済み）

## File Map

- `crates/ruviform-video-proc/src/rgba_f16.rs` — `f64_to_f16_bits`/
  `f16_bits_to_f64` を `pub(crate)` へ格上げ（design「実装単位」の共有
  コーデック方針 — 同等コーデック複製ではなく格上げを採用）
- `crates/ruviform-video-proc/src/composite.rs` — `CompositeOver` +
  `Crossfade`（呼出し毎 Q8 重み量子化・straight-alpha 演算を共有）+
  inline `#[cfg(test)] mod tests`
- `crates/ruviform-video-proc/src/affine.rs` — `Rect`/`RectMap` +
  `AffineTransform`（`i128`/`u128` 写像演算）+ inline tests
- `crates/ruviform-video-proc/src/lib.rs` — `mod`/`pub use` 追加・
  「Kernel API shape」段落へコンポジット族の `process` 可変引数規約を
  追記・カーネル一覧更新・「Scaling and compositing kernels stay
  reserved for later issues」記述の差替え
- `crates/ruviform-video-proc/README.md` — カーネル一覧と保留記述を
  実装状況へ更新
- `crates/README.md` — `ruviform-video-proc` 索引行の説明更新

## Implementation Sequence

- [ ] 1. 共有内部整備 — `rgba_f16.rs` の `f64_to_f16_bits`/
  `f16_bits_to_f64` を `pub(crate)` へ格上げし、`composite.rs` の骨格
  （呼出し毎 Q8 量子化ヘルパ・フォーマット/寸法/共有 backing の
  書込み前検証）を配置する
- [ ] 2. `CompositeOver` — `new()`（無引数・`Default` impl）+
  `process(src, dst, opacity)`。`opacity` の `is_finite()` + `[0, 1]`
  検証（違反は `InvalidValue("opacity")`）→ `w = round(opacity·255)`
  量子化 → フレーム検証（format 一致 ∈ {`Rgba8`, `RgbaF16`}・寸法
  一致・共有 `dst` 拒否）→ `w == 0` なら書込み前 early return。
  `Rgba8` は design の Q8 展開（`sa`/`b`/`oa`/`oc` 全て整数
  round-half-up・`b` は画素毎 1 回計算で `oa`/`oc` が再利用・
  `oa == 0` → `(0,0,0,0)`）、`RgbaF16` は同一式を f32 ドメイン評価 +
  RNE binary16 丸め（`oa < 2⁻¹⁴` ガード → `(0,0,0,0)`）。`dst` の
  `ColorInfo`/`pts`/`duration` は不変
- [ ] 3. `Crossfade` — `new()` + `process(from, to, dst, t)`。`t` の
  `is_finite()` + `[0, 1]` 検証 → `tq = round(t·255)` 量子化 →
  3 フレームの format/寸法一致 + 共有 `dst` 検証 → 全 4 チャネル
  lerp（`Rgba8` 整数 round-half-up / `RgbaF16` f32 + RNE 丸め）。
  `dst` メタデータへ `from` の `ColorInfo`/`pts`/`duration` を伝播
- [ ] 4. `AffineTransform` + `Rect`/`RectMap` — `process(src, dst, map)`。
  `map.src` の非空・`src` フレーム内包含を `i64` 拡幅で検証
  （違反は `InvalidValue("src rect")`）、`map.dst` 空/キャンバス非交差は
  エラーとせず全画素 transparent black。写像演算（`2·offset·map.src.w`
  等）は `i128`/`u128`、セル中心最近傍、矩形外は transparent black 充填。
  `dst` メタデータへ `src` の値を伝播
- [ ] 5. `lib.rs` 更新 — `mod composite`/`mod affine` + `pub use`、
  「Kernel API shape」段落へ `process(src, dst, opacity)` /
  `process(from, to, dst, t)` / `process(src, dst, map)` のコンポジット族
  規約を追記、カーネル一覧へ 3 カーネル追加・予約記述の差替え
- [ ] 6. テスト — test-spec 全ケース（golden vector・端点・trap
  vector・検証拒否・`rt_process_*` RT 契約テスト）を各モジュールの
  inline `#[cfg(test)] mod tests` へ実装

## Dependencies and Risks

- `rgba_f16.rs` の `f64_to_f16_bits`/`f16_bits_to_f64` は現状 private —
  格上げは `pub(crate)` に留め、公開 API 面を広げない（design の実装
  単位節の選択肢のうち「格上げ」を採用 — コーデック複製は DRY 違反）
- 依存は `ruviform-video-core`・`ruviform-media-core` のみ —
  `ruviform-rt`・同順位・engine 層への依存追加は禁止（spec 要件）。
  `assert_no_alloc` は本設計で導入しない確定事項であり、`rt_process_*`
  の alloc-free はレビュー担保（機械検出は次の RT 保証強化 Issue）
- `CompositeOver` の in-place `dst` と `process` 可変引数形状は二相 API
  形状からの設計上の逸脱 — design Risks and Trade-offs で確定済みの
  規約であり、実装側で再検討しない
- 新規公開型は struct のみ（`Rect`/`RectMap`/3 カーネル）— 公開 enum
  追加なし。`PixelFormat` 未知 variant は `_` arm で `InvalidValue` へ
  落とす graceful 処理を維持する

## Task Breakdown

- docs/tasks/composite-kernels.md — 手順 1–3（共有内部整備 +
  `CompositeOver` + `Crossfade`）
- docs/tasks/affine-kernel-and-closeout.md — 手順 4–6（`AffineTransform`
  + `lib.rs` 更新 + テスト）+ 文書クローズ（README/索引・検証列・
  doc 監査・チェーン frontmatter 更新。composite-kernels に依存）

## Verification

- [ ] test-spec 全ケースパス（`mise run test`）
- [ ] `mise run test-rt` で `rt_process_*` テストが選択・パス
- [ ] `mise run fmt-check`/`check`/`lint` パス
- [ ] `crates/ruviform-video-proc` の依存が `ruviform-video-core`・
  `ruviform-media-core` のみ（同順位・`ruviform-rt`・engine 依存なし —
  Cargo.toml 確認）
- [ ] doc 監査チェーン起因 findings 0（`doc_status.js audit`）

## Open Questions / Gaps

- なし — design の Deferred Design Concerns（blend 拡張・linear-light・
  フルアフィン・premult 変種・追加遷移・planar YUV・`assert_no_alloc`
  導入）は本計画の範囲外として確定済み

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（task 2 件・依存直列のため分割不要）
- Independent work streams: none（affine-kernel-and-closeout は
  composite-kernels に依存）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `CompositeOver` の Q8 自己無撞着展開（`b` の画素毎 1 回計算と
  `oa`/`oc` での再利用・`oc ≤ 255` の構造的保証）と `w == 0` early
  return の順序（検証通過後・書込み前 — 共有 `dst` 拒否は opacity=0
  でも先行する）がレビュー焦点
- `AffineTransform` の `i128`/`u128` 写像演算と `i64` 拡幅の境界検査
  （`map.dst` キャンバス非交差をエラーとしない定義済み結果）も焦点
