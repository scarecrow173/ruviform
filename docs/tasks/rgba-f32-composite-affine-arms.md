---
id: "TASK-034LFHlxNkMOGDilPsdP0f"
type: "task"
status: "done"
title: "既存カーネルの RgbaF32 arm 拡張（CompositeOver/Crossfade/AffineTransform）"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/affine.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "docs/tasks/rgba-f32-composite-affine-arms.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
  implemented-by:
    - "docs/impl/ir/issue-99-rgba-f32-arms-task-5-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
    - "docs/tasks/rgba-f32-format-and-transfer-internals.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/color-working-space-kernel-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# 既存カーネルの RgbaF32 arm 拡張（CompositeOver/Crossfade/AffineTransform）

## Work

approved plan `implement-issue-99-color-conversion-kernels.md` の **actions 12–14 を 1 task の ordered substeps として実行**。`composite.rs`/`affine.rs` に `RgbaF32` arm を追加し、#97 の linear-light deferred を閉じる。受理集合拡張のみ — 既存 `Rgba8`/`RgbaF16` 経路・Q8 量子化・メタデータ規約は不変。

### A12 — `composite.rs` — `CompositeOver`/`Crossfade` `RgbaF32` arm

- `CompositeOver`: `RgbaF32 → RgbaF32` arm — `RgbaF16` パスと同式を f32 で評価（binary16 丸めなし・Q8 量子化を継承 — `q8` helper の既存規約どおり）。straight alpha・premultiply 前提なし・transparent black・メタデータ規約（dst = over 側の color/pts/duration 伝播）は不変
- `Crossfade`: `RgbaF32 → RgbaF32` arm — 同式 f32 評価（`a×(1−t) + b×t` 混合・binary16 丸めなし）
- 等価性契約: 参照実装（f64 評価を binary32 へ丸め）に ≤ 2 ulp — doc の等価性契約節を `RgbaF16`（≤ 2 ulp binary16）と並記で更新
- `RgbaF32` src を受けない既存 arm（`Rgba8`/`RgbaF16`）の挙動不変 — `_` arm は graceful 拒否のまま

### A13 — `affine.rs` — `AffineTransform` `RgbaF32` arm

- `RgbaF32 → RgbaF32` arm — 写像（affine 行列）/複写のみ・色空間変換なし。`Rgba8`/`RgbaF16` 経路と同じ geometric transform を raw binary32 値に適用 — bit-exact 維持（写像は値変更なし・搬移のみ）
- 既存 arm の挙動不変・`_` arm graceful 拒否維持

### A14 — gate + doc 整合

- test-spec `color-working-space-kernel-tests.md` の `RgbaF32` arm ケース（受理・等価性 bound・拒否群・metadata・`rt_process_*`）を各モジュール inline tests に実装・全 pass
- `crates/ruviform-video-proc/README.md`（カーネル一覧・受理 format 表・feature `icc` 記述）・`crates/README.md` 索引行・`lib.rs` 一覧の実装状況一致確認
- `mise run test`・`mise run lint`・`mise run test-rt` exit 0

## Completion Gate

3 entry 全 green 実 log + lead review の後にのみ task 完了・buildable scope commit。

## Verification

- [x] `mise run test` — exit 0（video-proc lib tests 171/171 — composite.rs の `RgbaF32` 18 件・affine.rs の 6 件を含む全 pass・実 log）
- [x] `mise run lint` — exit 0（clippy `--workspace --all-targets -D warnings` clean）
- [x] `mise run test-rt` — exit 0 かつ本 task の `rt_process_*_f32_*` 6 件が一致・pass（video-proc で 27 件一致・workspace 107 件実選択）
- [x] `rg -n "RgbaF32" crates/ruviform-video-proc/src/composite.rs crates/ruviform-video-proc/src/affine.rs` — 両モジュールに arm が存在（受理 match arm + dispatch + doc + tests で 49 件一致）
- [x] `rg -n "RgbaF32" crates/ruviform-video-proc/README.md crates/README.md` — kernel 一覧/索引行が実装状況と一致（9 件一致 — Issue #99 kernel section + composite/affine format 表 + video-proc 索引行）
