---
id: "TASK-034LFH4wXKj32ElWzaI2lb"
type: "task"
status: "done"
title: "RgbaF32 pixel format + transfer/primaries 内部基盤"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-proc/src/transfer.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-video-core/src/pixel.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
  implemented-by:
    - "docs/impl/ir/issue-99-rgba-f32-transfer-task-1-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
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

# RgbaF32 pixel format + transfer/primaries 内部基盤

## Work

approved plan `implement-issue-99-color-conversion-kernels.md` の **actions 1–3 を 1 task の ordered substeps として実行**。`PixelFormat::RgbaF32`（video-core 公開 enum 拡張）と全カーネル共有の `transfer.rs` 内部基盤（`pub(crate)` — 公開 API 面を広げない）を設ける。後続 task（working-space-kernels・tone-map-sdr-kernel・icc-transform-feature・rgba-f32-composite-affine-arms）の共通前提。

### A1 — `PixelFormat::RgbaF32`（video-core）

- `crates/ruviform-video-core/src/pixel.rs` に `RgbaF32` variant 追加 — packed RGBA・4× IEEE-754 binary32 little-endian・1 plane・16 bytes/pixel（design「決定サマリ」どおり）
- `plane_count() == 1`・`is_planar() == false`・`bytes_per_sample(0) == 16`（範囲外 plane は 0）・`chroma_subsampling() == None` — 4 メソッド arm と `bytes_per_sample` doc（「one whole pixel on a packed plane」列挙へ `RgbaF32` = 16 bytes を追記）・variant doc（working-space 保持形である旨）を更新
- 既存 3 テスト（`plane_layout_per_format`・`bytes_per_sample_per_plane`・`subsampling_per_format`）へ `RgbaF32` 行を追加 — 表駆動で回帰検出
- `#[non_exhaustive]` のまま non-breaking — `RgbaF32` を知らない既存カーネル/consumer は `_` arm で graceful 拒否を維持（既存挙動不変 — 手を付けない）

### A2 — `transfer.rs` 新設（video-proc・`pub(crate)`）

- `crates/ruviform-video-proc/src/transfer.rs` を `pub(crate)` mod として新設（`lib.rs` に `mod transfer;` のみ — re-export なし）:
  - transfer decode/encode 関数群 — `Linear`（identity）・`Bt709`/`Smpte170m`（H.273 OETF 逆/順関数・toe 分岐 `v < 0.081`・負入力定義済み）・`Pq`（ST 2084 EOTF/逆 — `m1`/`m2`/`c1`/`c2`/`c3` 定数・decode は信号域 `[0,1]` 外を clamp、encode は `[0,1]` へ clamp）・`Hlg`（BT.2100 `S = v²/3` / `(exp((v−c)/a)+b)/12`・`S^1.2` × nominal 1000 nits → `×10` — `a`/`b`/`c` 定数・clamp 規則）。design「`RgbaF32` と working-space 定義」の式を逐語実装（定数は design 記載値 — 独自改変なし）
  - `ColorPrimaries` 各 variant → BT.2020 結合 3×3 行列の f64 導出定数テーブル（RGB→XYZ D65 正規化 — chromatic adaptation なし・`src→BT.2020` と `BT.2020→dst` の両方向）
  - `primaries_slot`/`transfer_slot` 解決関数 — `Unspecified` → `Bt709` 既定・未知 `#[non_exhaustive]` variant は `_` arm で `InvalidValue("color primaries")`/`InvalidValue("color transfer")` 拒否
  - working-space 定数 — `L_DS = 100` nits・HLG nominal peak 1000 nits・絶対輝度スケール `SDR_SCALE`（1.0 = 100 cd/m²）
- inline `#[cfg(test)] mod tests` — decode/encode 往復・boundary（`v = 0.081` toe・`v = 0.5` HLG・信号域外 clamp）・既定規則解決・行列定数 sanity（identity・往復逆行列）を test-spec `color-working-space-kernel-tests.md` の契約に沿って配置

### A3 — narrow gate

- `mise run check`・`mise run lint`・`mise run test` exit 0 — 既存カーネル無退行・`RgbaF32` テスト行が green

## Completion Gate

3 entry 全 green 実 log + lead review の後にのみ task 完了・buildable scope commit（1 substep/1 commit ではない）。

## Verification

- [x] `mise run check` — exit 0（`cargo check --workspace --all-targets`・実 log）
- [x] `mise run lint` — exit 0（clippy `--all-targets -D warnings` clean）
- [x] `mise run test` — exit 0（pixel.rs の `RgbaF32` 行・transfer.rs の unit tests が実行・pass — video-proc lib tests 100/100・実 log）
- [x] `rg -n "RgbaF32" crates/ruviform-video-core/src/pixel.rs` — variant + 4 メソッド arm + テスト行に一致（10 箇所）
- [x] `rg -n "mod transfer" crates/ruviform-video-proc/src/lib.rs` — `pub(crate)` 登録・公開 re-export なし
