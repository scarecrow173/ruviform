---
id: "TSPEC-Sn3LD1xCxNBxTN3FHLJUFa"
type: "test-spec"
status: "approved"
title: "ruviform-dsp テスト仕様"
created: "2026-11-03"
updated: "2026-11-03"
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
    - "docs/designs/ruviform-dsp.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by: []
  verifies:
    - "docs/designs/ruviform-dsp.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform-dsp テスト仕様

## ケース

- `window_shape_symmetry`: Hann/Blackman 係数が対称・端点 ~0・中心 ~1
- `biquad_dc_and_nyquist_response`: lowpass 設計で DC 通過 ~1・Nyquist 減衰
- `fir_streaming_matches_one_shot`: 分割 process の出力が一括 `fir` と一致（履歴キャリ）
- `fft_known_spectrum_and_roundtrip`: インパルス/正弦の既知スペクトル・inverse で復元
- `resampler_half_rate_doubles_length`: step=0.5 で ~2 倍出力・位相連続
- `rt_process_no_alloc`: assert_no_alloc 窓で biquad/fir/fft/resampler の process を 0 alloc 実測
