---
id: "PLAN-FHZdvjxcjsxFNodfVh9zOt"
type: "plan"
status: "approved"
title: "Issue #75 実施計画 — ruviform-dsp 新設"
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
  derived-by:
    - "docs/tasks/ruviform-dsp-crate.md"
    - "docs/tasks/ruviform-dsp-closeout.md"
  verifies: []
  verified-by:
    - "docs/test-specs/ruviform-dsp-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #75 実施計画

## 手順

1. `crates/ruviform-dsp` 新設 — `window.rs`/`biquad.rs`/`fir.rs`/`fft.rs`/`resample.rs` + workspace メンバー追加
2. 単体テスト: test-spec 全ケース（窓対称性・biquad 既知応答・FIR 分割/連続一致・FFT 往復・リサンプラ位相）
3. RT テスト `rt_process_no_alloc`（assert_no_alloc 窓）
4. 文書クローズ + 検証列

## Verification

- [x] 5 カーネル prepare/process 分離・`process` 0 alloc 実測
- [x] `mise run` 検証列全パス・doc 監査チェーン起因 findings 0
