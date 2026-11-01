---
id: "TASK-034Qfs4U6jKNPhfoasgTFI"
type: "task"
status: "done"
title: "T-3 acceptance tests + ownership PoC + fault injection"
created: "2026-11-01"
updated: "2026-11-01"
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
    - "docs/plans/implement-issue-195-m2-typed-graph-audio-runtime.md"
  implemented-by:
    - "docs/impl/ir/issue-195-m2-typed-graph-compiler-runtime.md"
  depends-on:
    - "docs/plans/implement-issue-195-m2-typed-graph-audio-runtime.md"
    - "docs/tasks/m2-t2-typed-rt-runtime-activation.md"
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

# T-3 acceptance tests + ownership PoC + fault injection

## Work

plan Step 4-5: `tests/typed_m2.rs`（ex01-ex12/ex20・frames 64/128/256/可変・zero/tail・`alloc_counter` RT 監査・uninit/alias fault）と `tests/ownership_poc.rs`（同一 MockDSP で Plan-owned vs Stable Slot 比較・G2-05 fault 注入）を実装し、ADR を PoC 結果で `accepted` 更新する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-195-m2-typed-graph-audio-runtime.md`
- `docs/test-specs/issue-195-m2-typed-graph-compiler-runtime-tests.md`
- `docs/adr/m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md`

## Dependencies

- `docs/tasks/m2-t2-typed-rt-runtime-activation.md`

## Affected Files

- `crates/ruviform-audio/tests/typed_m2.rs`・`tests/ownership_poc.rs`
- `docs/adr/m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md`（accepted 更新）

## Done When

- [ ] 受入 test 全件緑（ex01-ex12/ex20 + rt_ 監査 + frames matrix）
- [ ] 同一 MockDSP の両案比較測定が実施され ADR に結果が記録される
- [ ] G2-05 fault 注入（rebuild/conflict/retire-full/collector-stopped/shutdown）で last-good 維持・RT destructor なしが観測される

## Verification

- [ ] `mise exec -- cargo test -p ruviform-audio --test typed_m2 --test ownership_poc` exit 0
- [ ] `mise run test-rt` exit 0
