# ruviform-math

Ruviform メディア基盤の数値演算・ベクトル・SIMD カーネル・CPU 機能検出・数値ユーティリティを担うクレート。
メディア固有概念・I/O・スレッドは持たない。

## 責務

- `dsp` — SIMD ディスパッチ DSP カーネルの本番層（Issue #53）。呼び出し側は `SimdToken` を
  prepare／ブロック境界で一度構築し、全カーネル呼び出しに渡す。RT パス上で dispatch・
  機能検出は走らない。小バッファは測定済み閾値でスカラー参照へルーティングする
  （結果は不変・性能のみの差）。精度契約: 入力は有限 `f32` で、強いキャンセレーション領域は
  固定 ulp 上限の対象外。
- `simd_poc` — Issue #19 DSP/SIMD PoC の実験ブリッジ（`#[doc(hidden)]`）。公開 API
  契約ではない。`dsp` の各 SIMD パスが照合されるスカラー参照実装を置く。
  PoC 採否 ADR により昇格・削除され得る。
- それ以外の公開 API は未確定であり、後続の実装 Issue で確定する。

## 層・許可依存

- 層: Primitives（最下位、設計ベースライン クレート表 A.2）
- 許可依存: なし（std のみ）。`fearless_simd` 等の safe-SIMD バックエンドは実装詳細であり、
  その型を公開 API に露出しない。

## RT 契約

- 全公開関数は RT-suitable を目標に設計する: アロケーションなし・有界
  （本クレートは RT 契約の対象）。契約違反入力は `debug_assert` で捕捉し、
  release ビルドは well-defined な範囲のみを処理する。
- スカラー参照実装は必須。SIMD パスはカーネルごとに参照実装との等価性を検証する。
- `simd_poc` は実験モジュールであり、本番 RT 契約の適用外（入力生成ヘルパーは
  `Vec` を確保し、テスト／ベンチ専用）。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 A.2、「DSP／SIMD 設計」）
- Issue #19 DSP/SIMD PoC: docs/designs/dsp-simd-poc.md, docs/adr/dsp-simd-poc.md
- 採否 ADR・精度契約: docs/adr/dsp-simd-backend-adoption.md, docs/adr/simd-ulp.md
- Issue #53 本番ディスパッチ層: docs/designs/simd-dispatch-layer.md
- MSRV 引上げ（fearless_simd 本番昇格前提）: docs/adr/workspace-msrv-1-85-1-89-fearless-simd.md
