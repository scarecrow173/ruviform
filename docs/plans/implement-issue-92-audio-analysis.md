---
id: "PLAN-034Kq8IiJyWJrYeKbeaNK9"
type: "plan"
status: "approved"
title: "Issue #92 実施計画 — ruviform-audio-analysis 新設"
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
  derived-by:
    - "docs/tasks/audio-analysis-crate.md"
    - "docs/tasks/audio-analysis-closeout.md"
  derives-from:
    - "docs/designs/audio-analysis.md"

  verifies: []
  verified-by:
    - "docs/test-specs/audio-analysis-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #92 実施計画

## 手順

1. `crates/ruviform-audio-analysis` をワークスペースに追加（audio-core・dsp・media-core 依存）
2. `level.rs`: `LevelMeter`/`ChannelLevel`（ch 数 prepare・peak/rms 蓄積・`levels()`）
3. `spectrum.rs`: `SpectrumAnalyzer`（`Fft`+`Window` prepare・`process` は切詰め/ゼロ詰め・`magnitudes()`）
4. `peaks.rs`: `WaveformPeaks`/`PeakPair`（非 RT デシメーション `Vec` 返却）
5. 単体 + `rt_` 検証・検証列

## 設計から実装へ具体化する判断

- `process` 入力長の規則: 短い → ゼロ詰め、長い → 先頭切詰め（ブロック境界の決定的動作）
- `SpectrumAnalyzer::magnitudes()` は n/2+1 bin の振幅（|z|、正規化なし — スケールは描画側責務）
- `LevelMeter` は加算ではなく「直近ブロック値」を保持（バリスティックは非ゴール）

## Verification

- [x] `mise run` 検証列全パス・doc 監査チェーン起因 findings 0
- [x] test-spec 全ケース（検証拒否・計算値・rt_ 0 alloc）
