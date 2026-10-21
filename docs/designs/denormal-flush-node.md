---
id: "DESIGN-034JZDNRML000000000003"
type: "design"
status: "approved"
title: "DenormalFlush ノード設計"
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
    - "docs/specs/denormal-mitigation.md"
    - "docs/adr/denormal-mitigation-policy.md"

  derived-by:
    - "docs/plans/implement-issue-54-denormal-policy.md"
    - "docs/test-specs/denormal-flush-tests.md"

  verifies: []
  verified-by:
    - "docs/test-specs/denormal-flush-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# DenormalFlush ノード設計

## Overview

`ruviform-audio` の `AudioNode` 実装として denormal flush を提供する。`pull` 時に上流へ pull した後、出力バッファの各サンプルを走査し、denormal（`|x| < f32::MIN_POSITIVE`）を ±0 へ正規化する。

## 詳細

- 型: `DenormalFlush`（unit 型・状態なし）— そのものが `AudioNode`。順序リストモデルに合わせ、`graph.push(flush_node)` で任意位置に挿入（上流の出力をその場で flush）
- `pull` 実装: `out` の全チャンネルを走査し denormal を ±0 へ正規化
- flush 判定: `x.is_finite() && x != 0.0 && x.abs() < f32::MIN_POSITIVE` → `0.0f32.copysign(x)` 相当（実質 `x * 0.0` ではなく明示的代入）
- RT 契約: `pull` は RT-suitable（alloc なし・分岐のみ・ブロッキングなし）
- 既定挙動不変: `Graph`/`Mixer`/`Session` には自動挿入しない。利用者が `graph.push(Box::new(DenormalFlush::new(node)))` で明示挿入

## Verification

- [x] `AudioNode` trait の実装として既存 pull グラフに整合
- [x] `unsafe` 0・alloc 0・ブロッキングなし（RT-suitable）
