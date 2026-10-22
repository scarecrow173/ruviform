---
id: "DESIGN-034Kq7iQj2mt7VuR8YP2vq"
type: "design"
status: "approved"
title: "ruviform-audio-analysis 設計（レベル/スペクトル/波形ピーク）"
created: "2026-10-22"
updated: "2026-10-22"
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
    - "docs/specs/audio-analysis.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/plans/implement-issue-92-audio-analysis.md"
    - "docs/test-specs/audio-analysis-tests.md"
  verifies: []
  verified-by:
    - "docs/test-specs/audio-analysis-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-audio-analysis 設計（レベル/スペクトル/波形ピーク）

## Context

ベースライン責務表の `ruviform-audio-analysis`（レベル/ピーク/スペクトル
解析・UI 描画データ生成）を新設する。計算カーネルは `ruviform-dsp` に
委譲し、本クレートは「解析器の状態・スナップショット」責務のみ持つ。
RT 経路（`process`）と非 RT 経路（prepare・描画データ生成）を
既存の `prepare`/`process` 規約で分離する。

## Scope

### In Scope

- `LevelMeter`: ch 数で prepare、`process(&PlanarBufferView)` で
  ch 毎 `ChannelLevel {peak, rms}` を内部バッファに蓄積、`levels()` 参照
- `SpectrumAnalyzer`: `new(fft_size, Window)` で prepare（窓係数・
  re/im/振幅バッファ確保）。`process(&[f32])` → 内部 `magnitudes()`
  （`n/2+1` bin の振幅）。mono f32
- `WaveformPeaks`: 非 RT。`peaks(&[f32], bins)` → `Vec<PeakPair{min,max}>`
  デシメーション。部分 bin は ceil 分割
- エラーは `ruviform_media_core::Error`（`InvalidValue`/`InvalidInput` 系）で正規化

### Out of Scope

- dB 変換・加重・バリスティック・マルチチャネルスペクトル・位相
- キャッシュ・RT→非 RT チャネル自体（利用側の規約）

## Components and Boundaries

- `level` モジュール: 計測状態（ch×2 f32 の事前確保 Vec）+ 参照 API。
  `process` は既存バッファへの書き込みのみ
- `spectrum` モジュール: `dsp::Fft` + `Window` 係数を保持。
  `process` は入力を窓掛け→`Fft::process`→|z| を確保済み出力へ
- `peaks` モジュール: 純粋な計算関数 + `PeakPair` 値型。
  返却 `Vec` の確保は非 RT で許容（API 契約に明記）
- 依存: `ruviform-audio-core`（`PlanarBufferView`/`Sample`）、
  `ruviform-dsp`（`Fft`/`Window`）、`ruviform-media-core`（`Error`/`Result`）

## Data and Control Flow

1. prepare: `new(channels)`/`new(fft_size, window)` → 検証 + 内部 Vec 確保
2. `process(view)`: ch ループで peak=max|x|・rms=sqrt(Σx²/n) を `levels[i]` へ
3. `process(samples)`: 短い入力は末尾ゼロ詰め、長い入力は先頭 n に切詰め
   （窓掛け後 FFT → 振幅）
4. `peaks(samples, bins)`: 各 bin の min/max を走査して `Vec` 返却

## Non-Functional Constraints

- `process` 系は allocation-free・blocking なし・O(n)
- `SpectrumAnalyzer` の `process` は確保済み re/im を毎回再利用
  （入力長の検証のみ、長すぎは `InvalidValue("length")` ではなく切詰め
  — ブロック境界で呼ばれるため決定的動作を優先）
- `WaveformPeaks::peaks` は非 RT 専用（`Vec` 確保）を API doc で明記

## Risks and Trade-offs

- 切詰め/ゼロ詰めの選択は「ブロック境界入力」仮定を固定する
  → 任意長検証が要るなら将来 `process_exact` 追加で拡張可能
- `LevelMeter::levels()` が内部借用を返す設計 → 同時可変なしの単純契約。
  スナップショット交換は利用側の rt 基盤で行う

## Deferred Design Concerns

- ピーク階層キャッシュ | 需要未確定 | 波形 UI 実装時 | 再設計コスト
- バリスティック/dB 出力 | 表示仕様未確定 | メーター UI 実装時 | API 差分

## Verification Notes

- [x] 依存が audio-core・dsp・media-core のみで層内順序を満たす
- [x] 全 `process` が確保済み状態のみで動作（RT 契約）
