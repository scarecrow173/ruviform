---
id: "PLAN-034JZDNRML000000000004"
type: "plan"
status: "approved"
title: "Issue #54 実装計画 — denormal flush ノード"
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

  derived-by:
    - "docs/tasks/denormal-flush-node.md"
    - "docs/tasks/denormal-closeout.md"

  verifies: []
  verified-by:
    - "docs/test-specs/denormal-flush-tests.md"

  references: []
  defers: []
  deferred-by: []
---

# Issue #54 実装計画

## 手順

1. `crates/ruviform-audio/src/denormal.rs` に `DenormalFlush` ノード実装 + lib.rs export
2. 単体テスト: denormal→±0・normal 不変・NaN/Inf 透過・upstream pull 呼出し
3. RT テスト `rt_denormal_flush_no_alloc`（assert_no_alloc 窓）
4. 文書クローズ + 検証列

## Verification

- [x] denormal が ±0 に正規化され normal 値が不変
- [x] `rt_` テスト 0 alloc
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス
- [x] doc 監査で本チェーン起因 findings 0
