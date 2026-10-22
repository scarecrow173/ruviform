---
id: "SPEC-034Kq7ipiNwhFkBrHtwlII"
type: "spec"
status: "approved"
title: "ruviform-audio-analysis 実装 spec"
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
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/designs/audio-analysis.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform-audio-analysis 実装 spec

## What Should Be Built

`ruviform-audio-analysis` クレートの新設 — Domain foundations 層の解析クレート。
レベル/ピーク測定・FFT スペクトル解析・UI 描画用波形ピークデータ生成の
3系統を、`prepare`（非 RT・事前確保）/ `process`（RT 適合・無確保）分離の
規約で提供する。計算カーネルは `ruviform-dsp`（`Fft`・`Window`）を利用し、
変換 DSP の重複実装は持たない。

## Why Now

Issue #92。ベースライン Phase 2 対象クレートの未起票残り。
`ruviform-dsp`（#75）とプラグイン基盤（#74,#90,#91）が揃い、
解析レイヤの依存先が確定した。

## Users and Value

- メーターブリッジ・波形表示・スペクトラム表示を持つ上位層
  （engine / UI / plugin host 側の非 RT 描画経路）
- RT スレッドで計算し非 RT 側で消費する規約（SPSC/スナップショット交換）に
  沿った、RT パスで汚染しない解析器

## Scope

### In Scope

- `LevelMeter`（ch 数で prepare・process は無確保、ch 毎 peak/RMS のスナップショット）
- `SpectrumAnalyzer`（FFT サイズ + `Window` で prepare、mono f32 ブロックの
  振幅スペクトルを内部バッファへ）
- `WaveformPeaks`（非 RT: min/max デシメーションで描画データ生成）
- `media_core::Error` 規約のエラー正規化・`#[non_exhaustive]` 公開 enum なし方針

### Out of Scope

- マルチチャネルスペクトル・位相スペクトル・dB/加重フィルタ（K-weighted 等）
- 描画データのキャッシュ機構・RT→非 RT 受け渡しチャネル自体（rt クレートの規約を利用側で使う）
- 減衰バリスティック（attack/release）やオートスケール

## Requirements

- `process` 系は事前確保済み内部バッファのみで動作（allocation-free・blocking なし）
- `prepare` 系はサイズ/ch 数の検証を行い `Result` を返す
- 依存は `ruviform-audio-core`・`ruviform-dsp`（+ Primitives の `ruviform-media-core` エラー型）のみ。
  `ruviform-media-io`・`ruviform-video-proc` 等同順位 foundations への依存禁止
- `rt_` 命名テストで `assert_no_alloc` 検証

## Acceptance Criteria

- [ ] 3 API が test-spec の全ケースをパス
- [ ] `rt_` テストで process 経路 0 alloc 実測
- [ ] `mise run` 検証列全パス・doc 監査チェーン起因 findings 0

## Deferred Design Concerns

- キャッシュ機構（大容量 PCM のピーク階層キャッシュ） | 需要未確定 | 波形 UI 実装時 | 再設計コスト
- K-weighted/True Peak | 補間 DSP の設計要 | 測定要件確定時 | 後付け API 差分

## Sources and Relations

- docs/designs/ruviform-architecture-design-baseline.md（責務表 L241・層内順序 L214・Phase 2 L570）
- Issue #92・#75（dsp カーネル）・#22/PR #64（RT 検証基盤）
