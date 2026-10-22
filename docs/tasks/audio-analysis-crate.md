---
id: "TASK-034Kq8ZHrS7hOfvYmTEUQI"
type: "task"
status: "done"
title: "audio-analysis クレート骨格 + LevelMeter/SpectrumAnalyzer/WaveformPeaks"
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
  implements:
    - "docs/plans/implement-issue-92-audio-analysis.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-92-audio-analysis.md"
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

# audio-analysis クレート骨格 + LevelMeter/SpectrumAnalyzer/WaveformPeaks

## Work

`ruviform-audio-analysis` クレート新設 + 3 API 実装（design の In Scope）。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-issue-92-audio-analysis.md
- docs/designs/audio-analysis.md
- docs/specs/audio-analysis.md

## Dependencies

- #75 ruviform-dsp（Fft/Window）・audio-core（PlanarBufferView）・media-core（Error）

## Affected Files

- Cargo.toml（members 追加）
- crates/ruviform-audio-analysis/{Cargo.toml,src/lib.rs,src/{level,spectrum,peaks}.rs}

## Done When

- [x] 3 API が test-spec 全ケースをパス
- [x] `rt_` テストで process 0 alloc

## Verification

- [x] `mise run check`/`lint`/`test`/`test-rt` パス
