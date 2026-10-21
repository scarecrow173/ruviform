---
id: "TASK-034JcD2Xk1x6RZjHn779an"
type: "task"
status: "done"
title: "RGB→YUV 変換と Rgba8↔RgbaF16 カーネルの実装"
created: "2026-10-20"
updated: "2026-10-21"
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
    - "docs/plans/implement-issue-26-video-foundation.md"
  implemented-by:
    - "docs/impl/ir/implement-rgb-to-yuv-and-f16-kernels.md"
  depends-on:
    - "docs/plans/implement-issue-26-video-foundation.md"
    - "docs/tasks/implement-yuv-to-rgb-kernel.md"
  blocks:
    - "docs/tasks/sync-docs-issue-26.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/video-foundation-api.md"
  references: []
  defers: []
  deferred-by: []
---

# RGB→YUV 変換と Rgba8↔RgbaF16 カーネルの実装

## Work

`ruviform-video-proc` に残りのスカラー変換カーネルを追加する（spec AC7 後半・AC8・AC9、plan Phase D-1/D-2、検証 V-4）。T3a が提供した `coef.rs` の係数ロジックと `YuvToRgb` の構造を踏襲する。

- `crates/ruviform-video-proc/src/coef.rs`: `RgbCoef`（Q16 係数 9 本 + `y`/`cb_q16`/`cr_q16` 画素演算）と `rgb_to_yuv_table()` を追加 — `matrix_slot`/`range_slot` を再利用し、適用係数を出力 `ColorInfo` へ書き戻すための逆写像 `RESOLVED_MATRIX`/`RESOLVED_RANGE` を追加
- `crates/ruviform-video-proc/src/rgb_to_yuv.rs`: `RgbToYuv` 準備済みカーネル — `new(matrix, range) -> Result<Self>`（係数は構築引数で指定 — plan Q2 既定。`Unspecified`→Bt601/Limited、`Identity`/未知 variant → 構築時 `Err`）+ `process(&mut self, src, dst)`（`**RT contract:**` RT-suitable — alloc-free・non-blocking・有界）。検証: src は `Rgba8`、dst は `Yuv420p`/`Yuv422p`/`Yuv444p`（dst format がサブサンプリングを選択）、寸法一致、`dst.plane_mut(..)==None`（共有中）→ `Err`。全検証は書込前に完了
- chroma downsample: 420 は 2×2 セル・422 は水平ペア・444 は画素単位の box 平均。**Q16 ドメインのまま平均してから `clip8`**（量子化前平均 — 先に u8 化してから平均するのとは結果が異なり得る経路）。奇数寸法は存在画素のみ。siting 前提（水平 co-sited・垂直 center）は T3a と対の規約として doc 明記
- 出力 `ColorInfo` 更新規則（AC8）: matrix/range → 実際に適用した係数に対応する値（`RESOLVED_MATRIX`/`RESOLVED_RANGE`）、primaries/transfer/hdr は非破壊伝播。`pts`/`duration` は src から伝播（Q4）。等価性契約は整数演算のみのため**ビット完全一致**
- `crates/ruviform-video-proc/src/rgba_f16.rs`: `Rgba8ToRgbaF16` / `RgbaF16ToRgba8` 準備済みカーネル — `v/255` ↔ `round(clamp(v)·255)` の数値変換のみ（Q3 既定）。`ColorInfo`・`pts`/`duration` は不変伝播
- **binary16 コーデックはソフトウェア実装**（plan D-2 の「primitive `f16`」仮定が成立しないための代替）: `f16` は `x86_64-pc-windows-msvc` を含む MSVC 系ターゲットで Rust 1.99.0 時点でも unstable（rust-lang/rust#116909）であり、stable MSRV クレートでは使用不可。`RgbaF16` のバイトレイアウト（LE binary16×4）自体が契約なので、`f64` 経由の IEEE 754 round-to-nearest-even エンコード（`f16::from_f64` とビット一致）と exact デコードを crate 内部に実装 — `unsafe`・外部依存なし。等価性契約: forward は 1 ulp（binary16 ドメイン）、inverse はビット完全一致
- `src/lib.rs`: 再エクスポート + crate doc「Kernels」節に両カーネルを追記
- テスト: golden vector（3 matrix × 2 range × 既知画素・係数導出をコメントで示す）・chroma downsample（420 奇数寸法・422 水平平均・444 画素単位・Q16 平均の識別テスト）・エッジケース（共有 dst 拒否・構築時 `Err`・stride≠row_bytes）・f16 端数値（NaN・±∞・負・>1・±0）・全 u8 コードの往復・`rt_` 契約テスト・doctest

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-26-video-foundation.md](../plans/implement-issue-26-video-foundation.md) — 本タスクの Phase D（D-1/D-2）と検証 V-4 の正本。Open Question Q2（係数は構築引数）・Q3（f16 値規則）・Q4（dst 時刻伝播）、Risk Register 4/5/8/9
- [../specs/implement-video-foundation-api.md](../specs/implement-video-foundation-api.md) — AC7 後半（`Rgba8 → Yuv*p`・`Rgba8 ↔ RgbaF16`）・AC8（`Unspecified` 既定規則・出力 `ColorInfo` 更新・HDR 非破壊搬送）・AC9（エッジケース）
- [../adr/video-proc-kernel-api-and-color-hdr-scope.md](../adr/video-proc-kernel-api-and-color-hdr-scope.md) — D1-b（二相形状）・D2-a（実装範囲）・D3-a（HDR 範囲）・D6（`RtContext` 非依存）・D7-a（media-core `Error` 再利用）
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — RT 契約・DSP/SIMD 決定（スカラー参照実装 + 等価性契約）
- [../test-specs/video-foundation-api.md](../test-specs/video-foundation-api.md) — 契約保証の正本（`verified-by`）
- [implement-yuv-to-rgb-kernel.md](implement-yuv-to-rgb-kernel.md)（T3a）— `coef.rs` 共有ロジックとカーネル構造の前提（`depends-on`）

## Dependencies

- T3a の完了（`coef.rs` の `matrix_slot`/`range_slot`/`clip8`/`MATRIX_WEIGHTS`/`RANGE_SCALE` と `YuvToRgb` の参照実装が `done` 済み）
- `ruviform-video-core` の `VideoFrame`/`Plane`/`PlaneMut`/`PixelFormat`/`ColorInfo` API と `ruviform-media-core` の `Error`/`Result`
- 本タスクは [sync-docs-issue-26.md](sync-docs-issue-26.md)（T4）をブロックする

## Affected Files

- `crates/ruviform-video-proc/src/coef.rs` — `RgbCoef` + `rgb_to_yuv_table()` + `RESOLVED_MATRIX`/`RESOLVED_RANGE` 追加
- `crates/ruviform-video-proc/src/rgb_to_yuv.rs`（新規）— `RgbToYuv` カーネル + テスト
- `crates/ruviform-video-proc/src/rgba_f16.rs`（新規）— `Rgba8ToRgbaF16`/`RgbaF16ToRgba8` + ソフトウェア binary16 コーデック + テスト
- `crates/ruviform-video-proc/src/lib.rs` — crate doc「Kernels」節追記 + 再エクスポート
- `docs/impl/ir/implement-rgb-to-yuv-and-f16-kernels.md`（記入）・`docs/impl/ir/README.md`（索引行追加）
- 本タスクドキュメント（本文記入・status 遷移）・`docs/tasks/README.md`（status 列同期）

## Done When

- [x] AC7 後半: `RgbToYuv` が `Rgba8 → Yuv420p/Yuv422p/Yuv444p`（dst format で選択）を Bt601/Bt709/Bt2020 × Limited/Full で動作し、420=2×2・422=水平 2・444=画素単位の chroma downsample（Q16 ドメイン平均 → `clip8`）を含む。スカラー実装・ビット完全一致の等価性契約が doc 明記され、golden vector で検証されている
- [x] AC7 後半: `Rgba8ToRgbaF16`/`RgbaF16ToRgba8` が `v/255` ↔ `round(clamp(v)·255)`（round-half-up）で動作し、LE binary16 バイト契約・ulp-bounded forward / ビット完全一致 inverse の等価性契約が doc 明記されている
- [x] AC6 踏襲: 両カーネルが非 RT `new()`/`new(matrix, range)` → `process(&mut self, src, dst)` の二相形状を持ち、`RtContext` 引数・`Processor` impl を持たない。`**RT contract:**` ラベルが全公開 API に付いている
- [x] AC8: `RgbToYuv` の係数は構築引数由来（Q2 既定 — `Unspecified`→Bt601/Limited、`Identity`/未知 variant → 構築時 `Err`）、出力 `ColorInfo` は適用 matrix/range を記録し primaries/transfer/hdr 伝播。f16 カーネルは `ColorInfo` 不変伝播。`pts`/`duration` は src から伝播（Q4）。transfer 適用・トーンマップなし
- [x] AC9: siting 前提・奇数寸法の端数規則（存在画素のみ）の doc 明記、stride≠row_bytes で padding 非書込、`#[non_exhaustive]` 未知 variant の `_` arm graceful 処理（`unreachable!()` なし）
- [x] `dst.plane_mut(..)==None`（共有中）→ `Err`、全検証が書込前に完了（失敗時部分書き込みなし）
- [x] `rt_` テストが `rt_` プレフィックス命名で存在し `rt_` フィルタで意図した集合のみ選択される（非 RT テスト名に `rt_` 部分文字列なし）
- [x] impl record `docs/impl/ir/implement-rgb-to-yuv-and-f16-kernels.md` が completed で存在し、本タスクの `implemented-by`・status・両 README が同期済み

## Verification

- [x] `mise exec -- cargo test -p ruviform-video-proc` — 41 unit + 9 doctest 全合格（golden vector・chroma downsample・f16 端数値・往復を含む）
- [x] `mise exec -- cargo test -p ruviform-video-proc rt_ -- --list` — rgb_to_yuv 2 件 + rgba_f16 4 件 + yuv_to_rgb 2 件 = 8 件のみ選択（非 RT テスト名に `rt_` 部分文字列なし）
- [x] `mise exec -- cargo clippy -p ruviform-video-proc --all-targets -- -D warnings` — 0 warnings
- [x] `mise run fmt-check` — 差分なし（`mise run fmt` 実施済み）
- [x] `mise run check` — ワークスペース型チェック exit 0
- [x] `mise run test` — ワークスペース全体 PASS
- [x] `mise run lint` / `mise run test-rt` — 全 PASS（rt_ レーンは意図した集合のみ）
- [x] `mise exec -- cargo metadata --format-version 1 --no-deps` — video-proc deps は `ruviform-media-core` + `ruviform-video-core` のみ、`publish: []`
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` — 0 findings
- [x] `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` — 0 findings
