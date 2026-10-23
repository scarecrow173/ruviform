---
id: "TASK-034KSVlq4M72vjrxvDRQbP"
type: "task"
status: "done"
title: "コンポジットカーネル実装（CompositeOver + Crossfade + 共有 f16 コーデック格上げ）"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-proc/src/composite.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/src/rgba_f16.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-97-composite-kernels.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-97-composite-kernels.md"
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

# コンポジットカーネル実装（CompositeOver + Crossfade + 共有 f16 コーデック格上げ）

## Work

`composite.rs` 新設 — `CompositeOver`（`process(src, dst, opacity)` の
source-over in-place 合成）と `Crossfade`（`process(from, to, dst, t)`
の全 4 チャネル加重混合）を実装する（plan 手順 1–3・design の
`CompositeOver`/`Crossfade` 規約節）。先行整備として `rgba_f16.rs` の
`f64_to_f16_bits`/`f16_bits_to_f64` を `pub(crate)` へ格上げし
`RgbaF16` パスで再利用する（複製しない）。`AffineTransform` は後続
task（affine-kernel-and-closeout）で実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-issue-97-composite-kernels.md
- docs/designs/composite-transition-kernels.md
- docs/specs/composite-transition-kernels.md
- docs/test-specs/composite-transition-kernels-tests.md

## Dependencies

- `ruviform-video-core`（`VideoFrame`/`PixelFormat`/`SharedPlane` —
  `plane`/`plane_mut`/`row`/`row_mut`/`format`）・`ruviform-media-core`
  （`Error`/`Result` — ADR D7）。新規依存は追加しない
- `rgba_f16.rs` の f16 コーデック関数（本 task で `pub(crate)` 化）

## Affected Files

- crates/ruviform-video-proc/src/rgba_f16.rs（`f64_to_f16_bits`/
  `f16_bits_to_f64` を `pub(crate)` へ格上げ）
- crates/ruviform-video-proc/src/composite.rs（`CompositeOver` +
  `Crossfade` + inline `#[cfg(test)] mod tests`）
- crates/ruviform-video-proc/src/lib.rs（`mod composite` +
  `pub use composite::{CompositeOver, Crossfade}`）

## Done When

- [x] `rgba_f16.rs` の f16 コーデックが `pub(crate)` で共有される
  （公開 API 面は不変・コーデック複製なし）
- [x] `CompositeOver`/`Crossfade` が design の形状で実装される
  （`new()` 無引数 + `Default` impl・`process` 可変引数・`**RT
  contract:**` ラベル・参照実装 + 等価性契約・メタデータ規約の doc）
- [x] Q8 規約が design どおり: `w`/`tq` は呼出し毎に 1 回だけ量子化し
  両ピクセルパスが共有。`Rgba8` は `b` を画素毎 1 回計算して
  `oa`/`oc` で再利用する自己無撞着展開（`oa == 0` → `(0,0,0,0)`）。
  `RgbaF16` は f32 評価 + RNE binary16 丸め（同ガード + `oa < 2⁻¹⁴`
  denormal 域 → `(0,0,0,0)`）
- [x] `w == 0` は全検証通過後・画素書込み前の early return —
  `dst` 不変が保証され、共有 `dst` 拒否は `opacity = 0` でも先行する
- [x] test-spec の `CompositeOver`/`Crossfade` ケースをパス
  （golden vector・`oc` 境界 trap vector・`oa == 0`・early return・
  検証拒否・`from == to` 許容・`rt_process_*`）

## Verification

- [x] `mise run check`/`lint`/`test` パス
- [x] `mise run test-rt` で本 task の `rt_process_*` が選択・パス

## Completion Notes

- `rgba_f16.rs`: `f64_to_f16_bits`/`f16_bits_to_f64` を `pub(crate)` へ
  格上げ（「Shared with the composite kernels (`composite.rs`)」の注記
  付き）— 公開 API 面は不変、コーデック複製なし
- `composite.rs` 新設: 共有ヘルパ `q8_weight`（`round(v·255)` の
  呼出し毎 1 回量子化）/`q8_scale`/`f16_channel`/`f16_bytes` と
  画素関数 `composite_over_u8`（`sa`・backdrop 項 `b` を画素毎 1 回
  計算して `oa`/`oc` が再利用する自己無撞着 Q8 展開・`oa == 0` →
  `(0,0,0,0)`）/`composite_over_f16`（同一式 f32 評価 + RNE binary16
  丸め・`oa < 2⁻¹⁴` denormal 域を含むガード → `(0,0,0,0)`・非有限
  チャネルは f32 セマンティクス伝播）/`crossfade_u8`/`crossfade_f16`
  を実装。`CompositeOver::process(src, dst, opacity)`・
  `Crossfade::process(from, to, dst, t)` は両者とも `new()` 無引数 +
  `Default` impl の unit 形状、`**RT contract:**` ラベル付き
- 検証順序: パラメータ（`opacity`/`t` の `is_finite` + `[0,1]`）→
  format（`src`/`to`/`dst` ∈ {`Rgba8`,`RgbaF16`}・`InvalidValue`
  ラベル `"src format"`/`"to format"`/`"dst format"`）→ 寸法一致
  （`"dimensions"`）→ 共有 `dst` 拒否（`"dst plane"`）を全て書込み前に
  実施。`CompositeOver` の `w == 0` early return は `dst.plane_mut`
  取得成功後・画素書込み前に配置 — 共有 `dst` は `opacity = 0` でも
  先行拒否されることを専用テストで固定
- メタデータ: `CompositeOver` は `dst` の `ColorInfo`/`pts`/`duration`
  を不変（in-place キャンバス）、`Crossfade` は `from` の値を `dst`
  へ伝播（`set_color`/`set_pts`/`set_duration`）
- テスト: composite.rs `#[cfg(test)] mod tests` に 26 件（`CompositeOver`
  golden・`oc` 境界 trap vector 2 ケース・`oa == 0` 両フォーマット・
  `opacity = 0` byte-identical early return・検証拒否群・`RgbaF16`
  golden/HDR 保持/denormal/非有限伝播・in-place メタデータ保持・
  `Crossfade` 端点再現/量子化重み golden/同一フレーム許容/検証拒否/
  `from` メタデータ伝播・パディング非破壊・`rt_process_*` 4 件）+
  doctest 4 件 — test-spec 全ケースパス
- `lib.rs`: `mod composite` + `pub use composite::{CompositeOver,
  Crossfade}` を追加（カーネル一覧・「Kernel API shape」段落の更新は
  affine-kernel-and-closeout task 側で実施）
- 検証: `mise run check`/`lint`/`test`/`test-rt` 全パス（後続 task の
  最終検証列で再確認済み）。依存は `ruviform-video-core`・
  `ruviform-media-core` のみ（Cargo.toml 確認済み）
