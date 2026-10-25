---
id: "IMPL-034LFHxKq9vNmTzRwYpJdG"
type: "impl"
status: "completed"
title: "Issue #99 RgbaF32 format + transfer internals — Task 1 implementation"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-proc/src/transfer.rs"
      - type: "file"
        path: "docs/impl/ir/issue-99-rgba-f32-transfer-task-1-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-video-core/src/pixel.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
      - type: "file"
        path: "docs/tasks/rgba-f32-format-and-transfer-internals.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-99-color-conversion-kernels.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/rgba-f32-format-and-transfer-internals.md"
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

# Issue #99 RgbaF32 format + transfer internals — Task 1 implementation

## Summary

Issue #99 Task 1 完了 — actions 1–3（`PixelFormat::RgbaF32`・
`transfer.rs` 内部基盤・narrow gate）を全範囲実施。
`PixelFormat::RgbaF32`（packed RGBA・4× IEEE-754 binary32 LE・
1 plane・16 B/px・working-space 保持形）を video-core 公開 enum に
追加し、全色変換カーネル共有の `transfer.rs`（`pub(crate)`・
transfer decode/encode・primaries→BT.2020 行列・slot 解決・
working-space 定数・12 unit tests）を video-proc に新設。
3 gate 全 green 実 log、lead review 指摘（F1–F5）を全て修正済み。

## Context

approved plan `docs/plans/implement-issue-99-color-conversion-kernels.md`
の Task 1（`docs/tasks/rgba-f32-format-and-transfer-internals.md`・
actions 1–3）。upstream: approved spec/design/ADR
（color-conversion-kernels 系・broad-color-management）・TSpec
`color-working-space-kernel-tests.md`（proposed）。後続 task
（working-space-kernels・tone-map-sdr-kernel・icc-transform-feature・
rgba-f32-composite-affine-arms）の共通前提となる最初の task —
`ToWorking`/`FromWorking`/`ToneMapSdr` は Task 2/3・未着手。

## Implementation

### Changes

- `crates/ruviform-video-core/src/pixel.rs`:
  - `PixelFormat::RgbaF32` variant — packed RGBA・IEEE 754
    single-precision (`binary32`) little-endian per channel・
    1 plane・16 bytes/pixel。variant doc は design の working-space
    定義を転記（`{primaries: Bt2020, transfer: Linear,
    matrix: Identity, range: Full}` タグで `1.0` = 100 cd/m² の
    linear-light 絶対輝度を保持・`[0,1]` 非制限）。
  - 4 メソッド arm — `plane_count() == 1`・`is_planar() == false`
    （`matches!` を明示 match に展開し packed arm に追加）・
    `bytes_per_sample(0) == 16`・`chroma_subsampling() == None`。
    `bytes_per_sample` doc の「one whole pixel on a packed plane」
    列挙に `RgbaF32` = 16 bytes を追記。
  - 3 表駆動テストへ `RgbaF32` 行追加 — `plane_layout_per_format`
    （1 plane・non-planar）・`bytes_per_sample_per_plane`（`0` → 16・
    `1` → 0）・`subsampling_per_format`（`None`）。
  - `#[non_exhaustive]` のまま non-breaking — `RgbaF32` を知らない
    既存カーネル/consumer は `_` arm graceful 拒否を維持（既存挙動
    不変）。
- `crates/ruviform-video-proc/src/transfer.rs`（新規・702 行・
  `pub(crate)` mod — `lib.rs` に `mod transfer;` のみ・re-export なし）:
  - working-space 定数 — `SDR_SCALE = 100.0`（1.0 = 100 cd/m²）・
    `HLG_NOMINAL_PEAK = 1000.0`（BT.2100 nominal `L_W`）・
    `L_DS = 100.0`（BT.2390 target・Task 3 の `ToneMapSdr` で使用）。
  - transfer 関数群 — `Linear`（identity・非 clamp）・
    `decode_bt709`/`encode_bt709`（H.273 OETF 逆/順関数・toe 分岐
    `v < 0.081` / `l < 0.018`・負入力定義済み・encode 非 clamp）・
    `decode_pq`/`encode_pq`（ST 2084 — 公開 helper `pq_eotf`/
    `pq_eotf_inv` + `PQ_*` 定数・decode は信号域 `[0,1]` clamp・
    encode は `l ≤ 0` → 0・`l ≥ SDR_SCALE` → 1 の saturate）・
    `decode_hlg`/`encode_hlg`（BT.2100 逆/順 OETF `S = v²/3` /
    `(exp((v−c)/a)+b)/12` + reference OOTF `S^1.2` at nominal peak →
    動作域 `×10`・`HLG_A`/`HLG_B`/`HLG_C`/`HLG_GAMMA` 定数・clamp）。
  - slot 解決 — `TRANSFER_SLOTS`/`PRIMARIES_SLOTS` = 5・
    `transfer_slot`/`primaries_slot`（`Unspecified` → `Bt709` slot 0・
    未知 `#[non_exhaustive]` variant は `_` arm で
    `InvalidValue("color transfer")`/`InvalidValue("color primaries")`
    拒否）・`RESOLVED_TRANSFER`/`RESOLVED_PRIMARIES`（適用値を
    `Unspecified` ではなく resolved `Bt709` として記録 — `coef.rs`
    の `RESOLVED_MATRIX` と同じ目的）。
  - dispatch 表 — `TRANSFER_DECODE`/`TRANSFER_ENCODE`（
    `fn(f64) -> f64` 配列・`Smpte170m` は `Bt709` 曲線を共有）。
  - primaries 行列 — `Mat3` + `apply`（RGB のみ・alpha passthrough）・
    `TO_BT2020`/`FROM_BT2020`（`src→XYZ` と `dst→XYZ` の D65 正規化
    行列を事前結合した f64 定数 — 全 variant が D65 white を共有
    するため chromatic adaptation 不要・run-time inversion なし・
    `Bt2020` slot は厳密 identity）。
  - module doc — working-space 規約（`RgbaF32` + fixed `ColorInfo`
    タグ・絶対輝度 scale・`[0,1]` 非制限）・decode/encode の向き・
    clamp 規則・NaN 伝播・f64 参照式と ≤4 ulp 等価契約を記述。
    契約正本は `docs/designs/color-conversion-kernels.md` と
    TSpec を doc 末尾で明示。
  - inline `#[cfg(test)] mod tests` — 12 unit tests（slot 既定規則・
    `RESOLVED_*` 記録値・Linear identity bit-exact・Bt709 toe/power
    端点・Pq/Hlg 信号域 clamp・往復・行列 identity/往復逆/white
    保存・wide-gamut 負成分）。
- `crates/ruviform-video-proc/src/lib.rs`: `mod transfer;` のみ追加。

### Decisions During Implementation

- dispatch は slot + `fn` pointer 表構成（`transfer_slot` を prepare
  時 1 回解決 → `TRANSFER_DECODE[slot]` で画素ループから分岐を排除）
  — `coef.rs` の `RESOLVED_MATRIX` と同じ既定規則・報告規約を踏襲。
  plan は API 形状を未指定だったため、後続 kernel（Task 2/3/5）が
  prepare-time 解決できる slot 設計を選択（deviation として記録）。
- `pq_eotf`/`pq_eotf_inv` を `pub(crate)` で露出（deviation）—
  Task 3 の `ToneMapSdr` は BT.2390 EETF を luminance（nits）域で
  適用するため、`SDR_SCALE` 正規化込みの private
  `decode_pq`/`encode_pq` とは別に nits 域の EOTF/逆 EOTF が必要。
- `#![allow(dead_code)]` を module 先頭に付与（deviation）—
  consuming kernels は Task 2/3 で着地するまで全 item の唯一の
  使用者は unit tests のみ。removal note コメント付きで Task 2 の
  `ToWorking`/`FromWorking` 着地時に 1 行削除する前提。
- 全 math は f64 — kernels の参照定義「f64 評価 → binary32 丸め」・
  最適化 backend は ≤4 ulp 等価契約に拘束（design どおり・module
  doc に明記）。
- primaries 行列は事前結合 f64 定数 — 全 `ColorPrimaries` variant
  が D65 white を共有するため chromatic adaptation は構成しない
  （design「決定サマリ」どおり）。
- `is_planar()` は `matches!` から明示 match へ展開 — `RgbaF32`
  arm の追加に伴い exhaustive 記述に統一（既存戻り値不変）。

### Lead review（修正 CONFIRMED — F1–F5）

初回実装後のレビューで 5 finding を検出・全て修正した。

- F1: `encode_pq(+∞)` が `∞/∞ = NaN` を返す経路 → `l >= SDR_SCALE`
  で `1.0` を返す saturate guard を追加 + `INFINITY → 1.0` assert を
  `pq_encode_clamps_to_signal_domain` に追加（`NaN` 自身は両比較を
  通らず formula path で伝播する旨コメント）。
- F2: `FROM_BT2020` doc の結合式が `TO_BT2020` と同じ向きに誤記 →
  `(dst→XYZ)⁻¹ · BT.2020→XYZ`（`TO` の厳密逆行列）へ訂正。
- F3: `pq_eotf` の NaN 伝播が非自明（分子の `.max(0.0)` は
  `f64::max` の non-NaN operand 規則で NaN を呑むが分母の
  `c2 − c3·NaN` は伝播）→ mul_add コメントで挙動を明文化。
- F4: `encode_hlg` の exp 分岐（`a·ln(12·S − b) + c`）に golden が
  なく clamp 境界のみカバー → `encode_hlg(1.0)` の非 clamp
  exp-branch golden（≈0.62962）を追加。
- F5: 往復テストの緩い bound（5e-4）の根拠が未記述 → H.273
  toe/power 対が 0.081/0.018 knot 周辺で ~2.5·10⁻⁴ 不連続である旨
  コメント追記。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ。

### Rejected

なし。

## Validation

- A3 narrow gate（実 log・全 exit 0）:
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets`・既存カーネル無退行）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean・`#![allow(dead_code)]` は
    module 内の未使用 item のみを許容）
  - `mise run test` — exit 0（`cargo test --workspace`・
    ruviform-video-proc lib tests **100/100 pass** — うち 12 が
    `transfer.rs` 新規・pixel.rs `RgbaF32` 行 3 件も green）
  - `cargo fmt --check`（対象 2 クレート）— clean
- 構造確認（実コマンド）:
  - `rg -n "RgbaF32" crates/ruviform-video-core/src/pixel.rs` —
    variant・4 メソッド arm・`bytes_per_sample` doc・3 テスト行に
    一致（10 箇所）
  - `rg -n "mod transfer" crates/ruviform-video-proc/src/lib.rs` —
    `mod transfer;`（`pub(crate)`）・`pub use` なし
- レビュー指摘 F1–F5 修正後に gate 再実行で再 green を確認。

## Non-Goals

- `ToWorking`/`FromWorking`/`ToneMapSdr`/`IccTransform` 各カーネル
  本体・composite/affine の `RgbaF32` arm・feature `icc` — Task 2–5
  の scope（本 task は共有内部基盤のみ）。
- `transfer.rs` の公開 — `pub(crate)` のまま・公開 API 面は
  `PixelFormat::RgbaF32` のみ（plan どおり）。
- 既存 code-value カーネル（`YuvToRgb`/`RgbToYuv`/`Rgba8ToRgbaF16`/
  `RgbaF16ToRgba8`/`CompositeOver`/`Crossfade`/`AffineTransform`）の
  `RgbaF32` 受理 — `_` arm graceful 拒否を維持・既存挙動不変。
- プラットフォーム別 `powf` 差の厳密 golden — ulp-bound/tolerance
  比較で吸収（test-spec の契約どおり）。

## Risks

- `transfer.rs` は consuming kernels 未着地のため機械的な未使用検出
  は `#![allow(dead_code)]` で抑止中 — Task 2 で removal note どおり
  削除しないと lint 経路の意味をなさない構造負債が残る。
- slot 構成（`fn` pointer 表 + `RESOLVED_*`）は plan 未指定部分の
  実装判断 — Task 2/3/5 がこの表に依存するため、後続変更は表順と
  `primaries_slot`/`transfer_slot` の同期が必要（定数 doc に明記済み）。
- `pq_eotf`/`pq_eotf_inv` の `pub(crate)` 露出は `ToneMapSdr` 前提
  — Task 3 がこの API を消費しない場合は private 化の検討余地。

## Follow-ups

- Task 2 `docs/tasks/working-space-kernels.md`（`ToWorking`/
  `FromWorking`・actions 4–6）— `transfer.rs` を消費し
  `#![allow(dead_code)]` を除去する側。
- Task 3 `docs/tasks/tone-map-sdr-kernel.md` — `pq_eotf`/
  `pq_eotf_inv`・`L_DS`・`HLG_NOMINAL_PEAK` を消費。
- 本 PR merge 後に Task 2 着手（plan の単一クリティカルパス順序）。
