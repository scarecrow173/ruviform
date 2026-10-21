---
id: "TASK-034JQH8VJI6x2EXHMsOJU9"
type: "task"
status: "done"
title: "公開カーネル API + ルーティング + panic-free 化 + 精度契約 doc"
created: "2026-10-21"
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
    - "docs/plans/implement-issue-53-simd-dispatch-layer.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-53-simd-dispatch-layer.md"
    - "docs/tasks/simd-dispatch-token-and-kernels.md"
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
    - "docs/test-specs/simd-dispatch-layer-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# 公開カーネル API + ルーティング + panic-free 化 + 精度契約 doc

## Work

`dsp` の公開カーネル関数 8 本を実装: トークン第 1 引数 + サイズ閾値でスカラールーティング（add<128 / scale<64 / clamp_unit<256 / その他常時 token 経路 — design §ルーティングの確定値）。契約違反を `debug_assert` + release 非 panic（共通プレフィックス・channels クランプ）に統一。`BiquadCoeffs`/`SineCoeffs`/`SineState` を `dsp` から `pub use` 昇格。各関数 doc に精度契約（bit 完全一致 / ulp 256 / ulp 8 + 有限入力前提 + NaN/Inf 契約外）を明記。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-issue-53-simd-dispatch-layer.md
- docs/specs/simd-dispatch-layer.md
- docs/designs/simd-dispatch-layer.md
- docs/adr/simd-ulp.md
- docs/tasks/simd-dispatch-token-and-kernels.md

## Dependencies

- 前タスクの完了（直列依存）

## Affected Files

- crates/ruviform-math/src/dsp.rs
- crates/ruviform-math/src/simd_poc.rs（`pub use` 昇格の整合のみ）

## Done When

- [x] 8 関数すべてが公開・精度契約 doc 付き
- [x] 閾値 const が design 確定値と一致し根拠コメント付き
- [x] release で契約違反入力が panic しない（共通範囲のみ処理）

## Verification

- [x] `mise run check` / `lint` / `fmt-check` パス
- [x] 閾値境界 ±1 の動作確認テストがグリーン
