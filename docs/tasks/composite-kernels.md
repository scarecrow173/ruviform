---
id: "TASK-034KSVlq4M72vjrxvDRQbP"
type: "task"
status: "todo"
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

- [ ] `rgba_f16.rs` の f16 コーデックが `pub(crate)` で共有される
  （公開 API 面は不変・コーデック複製なし）
- [ ] `CompositeOver`/`Crossfade` が design の形状で実装される
  （`new()` 無引数 + `Default` impl・`process` 可変引数・`**RT
  contract:**` ラベル・参照実装 + 等価性契約・メタデータ規約の doc）
- [ ] Q8 規約が design どおり: `w`/`tq` は呼出し毎に 1 回だけ量子化し
  両ピクセルパスが共有。`Rgba8` は `b` を画素毎 1 回計算して
  `oa`/`oc` で再利用する自己無撞着展開（`oa == 0` → `(0,0,0,0)`）。
  `RgbaF16` は f32 評価 + RNE binary16 丸め（同ガード + `oa < 2⁻¹⁴`
  denormal 域 → `(0,0,0,0)`）
- [ ] `w == 0` は全検証通過後・画素書込み前の early return —
  `dst` 不変が保証され、共有 `dst` 拒否は `opacity = 0` でも先行する
- [ ] test-spec の `CompositeOver`/`Crossfade` ケースをパス
  （golden vector・`oc` 境界 trap vector・`oa == 0`・early return・
  検証拒否・`from == to` 許容・`rt_process_*`）

## Verification

- [ ] `mise run check`/`lint`/`test` パス
- [ ] `mise run test-rt` で本 task の `rt_process_*` が選択・パス
