---
id: "TSPEC-034JZDNRML000000000005"
type: "test-spec"
status: "approved"
title: "denormal flush テスト仕様"
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
    - "docs/designs/denormal-flush-node.md"

  derived-by: []
  verifies:
    - "docs/designs/denormal-flush-node.md"

  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# denormal flush テスト仕様

## ケース

- `denormal_flush_zeroes_denormals`: denormal 入力 → ±0（符号保持）
- `denormal_flush_preserves_normal`: normal 値・ゼロ・NaN・Inf は不変
- `denormal_flush_in_graph_order`: グラフ内でノード順序の間に挿入でき、直前ノード出力のみを flush
- `rt_denormal_flush_no_alloc`: assert_no_alloc 窓内で pull 実行 0 alloc
