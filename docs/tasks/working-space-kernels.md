---
id: "TASK-034LFHddlrfvXu5Jlmwdr0"
type: "task"
status: "todo"
title: "working space カーネル実装（ToWorking + FromWorking）"
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
  implements:
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
  implemented-by: []
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

# working space カーネル実装（ToWorking + FromWorking）

## Work

approved plan `implement-issue-99-color-conversion-kernels.md` の **actions 4–6 を 1 task の ordered substeps として実行**。`working_space.rs` 新設 — working space 入口カーネル `ToWorking`（decode+gamut 融合パス）と出口カーネル `FromWorking`（ctor で dst primaries/transfer 焼付け）を `transfer.rs` 内部基盤の上に実装する。

### A4 — `ToWorking`

- `crates/ruviform-video-proc/src/working_space.rs` 新設 — `ToWorking`（`new()` 無引数 + `Default` impl・`process(src, dst)`）
- src ∈ {`Rgba8`（`v/255`）, `RgbaF16`（`f16_bits_to_f64` 経由 — 既存 `pub(crate)` コーデックを再利用）, `RgbaF32`（raw LE 読取）} → dst `RgbaF32`。`Yuv*p`・未知 variant は `InvalidValue("src format")`・寸法不一致 `InvalidValue("dimensions")`・共有 `dst` は `InvalidValue("dst plane")` — 全検証書込み前
- `src.color()` を呼出し毎に読み `primaries_slot`/`transfer_slot` で既定解決（`Unspecified` → `Bt709`・未知 variant → `InvalidValue("color primaries")`/`"color transfer"`）。`matrix`/`range` は検証しない（RGB 入力に非適用）
- 画素毎: 4 チャネル decode → RGB のみ 3×3 行列（`src` primaries → BT.2020）→ alpha パススルー。`RgbaF32`→`RgbaF32` identity 入力でも画素コピーを通す（no-op early return なし）
- `dst.color` = `{primaries: Bt2020, transfer: Linear, matrix: Identity, range: Full, hdr: src.hdr}`・`pts`/`duration` 伝播
- doc: `**RT contract:**` RT-suitable（alloc-free・非ブロッキング・有界）・参照実装 + ≤ 4 ulp（binary32）等価性契約・`Unspecified` 解釈・メタデータ規約・非有限伝播を明記

### A5 — `FromWorking`

- 同モジュールに `FromWorking` — `new(dst_primaries: ColorPrimaries, dst_transfer: TransferCharacteristics) -> Result<Self>`（`RgbToYuv` 先例: `Unspecified` → `Bt709` 既定解決・未知 variant `InvalidValue` 拒否・結合行列 + encode パラメータを ctor で焼付け — 非 RT by convention・実 alloc なし）
- src `RgbaF32` → dst ∈ {`Rgba8`, `RgbaF32`}・同一寸法・排他所有。画素毎: 3×3 行列（BT.2020 → dst primaries）→ encode → alpha パススルー。`Rgba8` は `round(clamp(v,0,1)·255)`（RHU）量子化・`RgbaF32` は encode 規則どおり書出し（`Pq`/`Hlg` clamp 済み・`Bt709`/`Linear` 非 clamp）
- `dst.color` = `{ctor 解決値, ctor 解決値, Identity, Full, hdr: src.hdr}`・`pts`/`duration` 伝播。ctor `Bt2020`/`Linear` の identity 経路 = `RgbaF32` 純粋量子化（`Rgba8` 化）を表現 — shuffle 専用カーネルは持たない
- doc: `**RT contract:**`・≤ 4 ulp（`Rgba8` 出力は量子化境界 ±1 code）・HDR 未通過直結は consumer 誤用の旨を明記
- `lib.rs`: `mod working_space` + `pub use working_space::{ToWorking, FromWorking}` + カーネル一覧へ 2 カーネル追加

### A6 — gate

- test-spec `color-working-space-kernel-tests.md` の `ToWorking`/`FromWorking` ケース（golden/ulp-bound・`Unspecified` 既定・拒否群・メタデータ・identity 経路・`rt_process_*`）を inline `#[cfg(test)] mod tests` に実装・全 pass
- `mise run test`・`mise run lint`・`mise run test-rt` exit 0 — `rt_process_*` が名前フィルタで実際に選択されること

## Completion Gate

3 entry 全 green 実 log + lead review の後にのみ task 完了・buildable scope commit。

## Verification

- [ ] `mise run test` — exit 0（working_space.rs の unit tests 全 pass）
- [ ] `mise run lint` — exit 0
- [ ] `mise run test-rt` — exit 0 かつ本 task の `rt_process_*` が一致・pass
- [ ] `rg -n "pub use working_space" crates/ruviform-video-proc/src/lib.rs` — re-export 存在
- [ ] `rg -n "RT contract" crates/ruviform-video-proc/src/working_space.rs` — API 単位の RT ラベル存在
