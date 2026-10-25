---
id: "IMPL-034LINnXPOUOZEuD5lHPdy"
type: "impl"
status: "completed"
title: "Issue #99 working-space kernels — Task 2 implementation"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-proc/src/working_space.rs"
      - type: "file"
        path: "docs/impl/ir/issue-99-working-space-kernels-task-2-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/transfer.rs"
      - type: "file"
        path: "docs/tasks/working-space-kernels.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-99-color-conversion-kernels.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/working-space-kernels.md"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #99 working-space kernels — Task 2 implementation

## Summary

Issue #99 Task 2 完了 — actions 4–6（`ToWorking`・`FromWorking` カーネル +
gate）を全範囲実施。`working_space.rs`（新規・1597 行）に working
space 入口カーネル `ToWorking`（`Rgba8`/`RgbaF16`/`RgbaF32` → `RgbaF32`
の transfer decode + gamut 融合パス）と出口カーネル `FromWorking`
（ctor で dst primaries/transfer を焼付ける `RgbaF32` →
`Rgba8`/`RgbaF32`）を実装し、`lib.rs` で公開した。Task 1 の
`#![allow(dead_code)]` は removal note どおり除去 — 唯一の未消費 item
`L_DS` のみ `#[expect(dead_code)]` で残置（消費者は Task 3 の
`ToneMapSdr`）。4 gate 全 green 実 log・独立レビュー 2 件（P0/P1 なし）
の指摘を全て修正済み。

## Context

approved plan `docs/plans/implement-issue-99-color-conversion-kernels.md`
の Task 2（`docs/tasks/working-space-kernels.md`・actions 4–6）。
upstream: Task 1 IR
（`docs/impl/ir/issue-99-rgba-f32-transfer-task-1-implementation.md` —
`transfer.rs` 内部基盤と `PixelFormat::RgbaF32`）・approved
spec/design/ADR（color-conversion-kernels 系・broad-color-management）・
TSpec `color-working-space-kernel-tests.md`（proposed）。後続 task
（tone-map-sdr-kernel・icc-transform-feature・
rgba-f32-composite-affine-arms）は本 task のカーネルと内部基盤を消費する
— `ToneMapSdr`/`IccTransform`/`RgbaF32` arm は Task 3/4/5・未着手。

## Implementation

### Changes

- `crates/ruviform-video-proc/src/working_space.rs`（新規・1597 行・
  公開 mod — `lib.rs` で `pub use`）:
  - pixel readers — `rgba8_signal`（`v/255` 正規化）・
    `rgba_f16_signal`（既存 `pub(crate)` `f16_bits_to_f64` 再利用で
    `binary16` を `f64` へ厳密 decode）・`rgba_f32_signal`（raw
    little-endian `binary32` → `f64` 拡大 — 全 `f32` は `f64` で厳密・
    `NaN`/`±∞` 伝播）。
  - fused per-pixel transforms — `to_working_px`（RGB decode →
    `TO_BT2020` matrix・alpha は正規化 signal をパススルー）・
    `from_working_px`（`FROM_BT2020` matrix → encode・alpha パススルー）・
    `f32_px_bytes`（f64 → binary32 LE serialize — 参照定義の丸めそのもの）・
    `quantize8`（`round(clamp(v,0,1)·255)` RHU・`NaN` → 0 fold を明示）。
  - `ToWorking` — unit ctor `new()` + `Default` impl・
    `process(src, dst)`。src ∈ {`Rgba8`, `RgbaF16`, `RgbaF32`}・dst
    `RgbaF32`。全検証を書込み前に完了（format・dimensions・color 解釈・
    共有 `dst` 排他所有 — 拒否呼出しは `dst` 不変・部分書込みなし）。
    `src.color()` を呼出し毎に読み `transfer_slot`/`primaries_slot` で
    既定解決（`Unspecified` → `Bt709`・未知 `#[non_exhaustive]` variant
    は `InvalidValue("color transfer")`/`InvalidValue("color
    primaries")` 拒否）。`matrix`/`range` は RGB 入力に非適用で非検証。
    `RgbaF32`→`RgbaF32` identity 入力でも画素パスを通す（no-op early
    return なし — working-space `ColorInfo` 記録が本カーネルの責務）。
    post-loop で `dst.set_color({Bt2020, Linear, Identity, Full, hdr:
    src.hdr})` + `set_pts`/`set_duration` 伝播（color conversion は
    retime しない）。
  - `FromWorking` — `new(dst_primaries, dst_transfer) -> Result<Self>`
    （`RgbToYuv` 先例 — `Unspecified` → `Bt709` 既定解決・未知 variant
    `InvalidValue` 拒否・結合 `FROM_BT2020` 行列 + `TRANSFER_ENCODE` fn
    pointer + `RESOLVED_*` tag を ctor で焼付け — 非 RT by convention・
    実 alloc なし）。`process` は src `RgbaF32`・dst ∈ {`Rgba8`,
    `RgbaF32`} — `Rgba8` 経路は `quantize8` で RHU 量子化（負・1 超は
    量子化段で飽和）・`RgbaF32` 経路は encode 規則どおり書出し（
    `Pq`/`Hlg` clamp 済み・`Bt709`/`Linear` 非 clamp）。identity ctor
    （`Bt2020`/`Linear`）は `RgbaF32` → `Rgba8` の純粋量子化を表現 —
    shuffle 専用カーネルは持たない（plan どおり）。
  - doc — module/type/method に working space 定義・`ColorInfo` 解釈規則・
    ≤ 4 ulp（binary32）等価性契約（`Rgba8` 出力は量子化境界 ±1 code を
    許容）・非有限伝播（`0·∞` → `NaN` mixing 含む）・検証/メタデータ規約・
    API 単位の `**RT contract:**` ラベル（`new`/`Default` = 非 RT by
    convention・`process` = RT-suitable alloc-free・非ブロッキング・
    有界）・HDR 未通過直結は consumer 誤用の明記。公開 API 例は doctest
    4 件（`ToWorking`/`ToWorking::new`/`FromWorking`/`FromWorking::new`）。
  - inline `#[cfg(test)] mod tests` — 30 tests（unit 26 +
    `rt_process_*` 4）: Bt709/Pq/Hlg decode golden・`RgbaF16` signal
    読取・`P3D65` gamut 写像・`Unspecified` 既定・全既知 color variant
    受理・matrix/range 非検証・identity 経路（画素コピー + 記録）・
    非有限伝播・メタデータ記録（`dst` pts 事前 scrub で伝播を観測可能化）・
    拒否群（`dst` 不変断言付き）・padding 不変（dst 非書込み + src
    padding 非読取り）・RT 契約（repeatable/deterministic・src 非破壊）。
- `crates/ruviform-video-proc/src/lib.rs`: `ToWorking`/`FromWorking` の
  kernel 一覧 bullet・`mod working_space;`・
  `pub use working_space::{FromWorking, ToWorking};` を追加。
- `crates/ruviform-video-proc/src/transfer.rs`: Task 1 の removal note
  どおり module 先頭の `#![allow(dead_code)]` を削除。唯一の未消費
  item `L_DS`（消費者は Task 3 の `ToneMapSdr`）に
  `#[expect(dead_code)]` + 理由コメントを付与 — `expect` は使用者が
  できた時点で attribute 自体が警告されるため、自然な除去トリガになる。

### Decisions During Implementation

- per-row dispatch（deviation・deliberate）— src/dst format の match を
  row ループ内に保持。format は loop-invariant だが、分岐のホイスティング
  は per-format に重複する row ループを生むため KISS 側を選択（参照実装・
  分岐予測が実コストを吸収する想定 — 最適化 backend は等価性契約の内側で
  後続自由）。
- `dst_format` capture（deviation）— `FromWorking::process` は
  `dst.format()` を `dst.plane_mut(0)` の可変借用の前に capture する
  （借用中は `dst.format()` を呼べないため、行内 match は capture 値で
  判定 — コメントで明記）。
- `quantize8` の `NaN` → 0 fold を明示実装 — `as u8` cast 自体も 0 に
  収束するが、挙動を仕様として固定（review で golden 化 —
  `from_working_nan_channels_quantize_to_zero`）。
- `L_DS` は `#[expect(dead_code)]` — `allow` ではなく `expect` を選択
  （Task 3 で `ToneMapSdr` が消費した時点で clippy が未充足を警告 →
  自然な除去トリガ。Task 1 の module-level `allow` とは意図的に非対称）。
- in-loop match は防御的 arm 構成 — 検証済み format の明示 arm + `_` →
  `Err`（composite.rs の `_` arm graceful 拒否規約と同型）。到達不能だが
  将来の `#[non_exhaustive]` variant 追加でも panic-free を保つ。

### Independent reviews（修正 CONFIRMED — P0/P1 なし）

実装後に独立レビュー 2 件を実施 — P0/P1 finding はなく、指摘・改善点を
全て修正した。

- metadata テストの pts-propagation が tautology 化（`dst` が最初から
  `src` の pts を保持し伝播を観測できない）→ 両 metadata テストで
  `dst.set_pts(MediaTime::ZERO)` の事前 scrub を追加し、伝播を実際に検証
  （`to_working_records_working_space_metadata`・
  `from_working_records_metadata`）。
- in-loop match の `_` arm 到達不能性を防御的に強化 — 明示 `RgbaF32`
  arm + `_` → `Err` の形へ（composite.rs 規約と同型）。
- doc wording 修正 — 「bit-exact」の限定子・`# Errors` doc・乱れた
  コメント 2 箇所を修正。
- doctest の unused bindings を `_kernel` へ修正。
- 3 テスト追加 — `to_working_decodes_bt709_tagged_rgba_f16`（F16 +
  Bt709 golden）・`from_working_nan_channels_quantize_to_zero`
  （NaN → `quantize8` fold の固定）・
  `to_working_padded_src_stride_reads_rows_only`（padded src の行
  のみ読取り）。
- 拒否テストに `dst` 不変断言を追加 — ピクセル・`ColorInfo`・pts が
  拒否呼出しで変更されないことを機械検証。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ。

### Rejected

なし。

## Validation

- A6 gate（実 log・全 exit 0）:
  - `mise run check` — exit 0（`cargo check --workspace --all-targets`）
  - `mise run lint` — exit 0（`cargo clippy --workspace --all-targets
    -- -D warnings` clean — `#![allow(dead_code)]` 除去後も
    `#[expect(dead_code)]` は充足済みで warning なし）
  - `mise run test` — exit 0（`cargo test --workspace`・
    ruviform-video-proc lib tests **130/130 pass** — うち 30 が
    `working_space.rs` 新規（unit 26 + `rt_process_*` 4）・doctests
    **19/19 pass** — working_space 由来 4 件を含む）
  - `mise run test-rt` — exit 0（`rt_` 名前フィルタで
    `working_space::tests::rt_process_*` 4 件が実選択・pass —
    video-proc lib 内の `rt_` 一致は計 19 件）
  - `mise run fmt-check` — exit 0（`cargo fmt --all -- --check`
    clean）
- 構造確認（実コマンド）:
  - `rg -n "pub use working_space" crates/ruviform-video-proc/src/lib.rs`
    — `pub use working_space::{FromWorking, ToWorking};`（L126）
  - `rg -n "RT contract" crates/ruviform-video-proc/src/working_space.rs`
    — API 単位の `**RT contract:**` ラベル 4 箇所（`ToWorking::new`/
    `process`・`FromWorking::new`/`process`）
  - `rg -n "dead_code" crates/ruviform-video-proc/src/transfer.rs` —
    module-level `allow` は除去済み・`#[expect(dead_code)]` が `L_DS`
    のみに残存
- レビュー指摘修正後に gate 再実行で再 green を確認済み。

## Non-Goals

- `ToneMapSdr`・`IccTransform`/`IccIntent`・composite/affine の
  `RgbaF32` arm・feature `icc` — Task 3–5 の scope。
- `FromWorking` の `RgbaF16` dst・`RgbaF32 ↔ RgbaF16` shuffle 専用
  カーネル — plan の non-goals どおり（identity ctor が純粋量子化を兼ねる）。
- `assert_no_alloc` による alloc 機械検出 — design で後続 Issue へ defer
  済み（`rt_process_*` は repeatable/deterministic・src 非破壊の観測
  検証 — #97 と同一レーン）。
- `Processor` trait 実装・`RtContext` 引数・フレームプール・
  engine/OFX への配線 — plan non-goals どおり。
- `src`/`dst` の `ColorInfo` 一致検査・working-space 記述の強制 —
  design が明示的に非検証と決定（consumer 責務）。

## Risks

- per-row dispatch の分岐は参照実装としての選択 — 最適化 backend 導入
  時は hoisted 構造への置換が等価性契約（≤ 4 ulp）の内側で可能。
- `L_DS` の `#[expect(dead_code)]` は Task 3 で消費されない場合 lint
  が未充足を報告する — 構造負債ではなく意図的な着火点（消費時に自然に
  除去を要求される設計）。
- `ToWorking` は `src.color()` を信用入力として解釈する — 未タグ素材は
  `Bt709` 既定で変換される（既定規則自体は spec/design の確定事項・
  consumer が正しいタグを供給する前提）。

## Follow-ups

- Task 3 `docs/tasks/tone-map-sdr-kernel.md`（actions 7–8）— `L_DS`・
  `pq_eotf`/`pq_eotf_inv`・`HLG_NOMINAL_PEAK` を消費し
  `#[expect(dead_code)]` を除去する側。
- Task 5 `docs/tasks/rgba-f32-composite-affine-arms.md` —
  `CompositeOver`/`Crossfade`/`AffineTransform` の `RgbaF32` arm が
  本 task の working-space 保持形を受理する側。
- 本 PR merge 後に Task 3 着手（plan の単一クリティカルパス順序）。
