---
id: "TASK-034QidcTgfNesKcwIF1aZ7"
type: "task"
status: "done"
title: "T-4 PoC acceptance tests, rt alloc, perf evidence, impl record, merge+verify"
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
    - "docs/plans/implement-issue-185-generic-probe-poc.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-185-generic-probe-poc.md"
    - "docs/tasks/p185-t3-probe-contract-broker-collector.md"
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
    - "docs/test-specs/issue-185-generic-probe-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# T-4 PoC acceptance tests, rt alloc, perf evidence, impl record, merge+verify

## Work

plan Step 5-6: `tests/common` に `ProbePoint` 宣言 port fixture を追加し、`tests/typed_probe.rs` で test-spec ex01-ex15・`rt_probe_*` で ex16 を実装。ON/OFF 負荷差・alloc 0・実時間性能を計測し `docs/impl/ir/issue-185-generic-probe.md` に記録（未検証範囲も明示）。`git fetch origin && git merge origin/main` → lockfile 再生成 → 全検証を実行。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-185-generic-probe-poc.md`
- `docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md`
- `docs/specs/issue-185-generic-probe-rt-capture.md`

## Dependencies

- `docs/tasks/p185-t3-probe-contract-broker-collector.md`

## Affected Files

- `crates/ruviform-audio/tests/common/mod.rs`、`crates/ruviform-audio/tests/typed_probe.rs`、`rt_probe_*` テスト、`docs/impl/ir/issue-185-generic-probe.md`、`Cargo.lock`

## Done When

- [ ] test-spec ex01-ex16 が全て緑
- [ ] `rt_probe_*` が `assert_no_alloc` 窓 0
- [ ] probe 有無で boundary 出力 bit 一致・rebind 後に正しい slot を観測
- [ ] impl record に実行コマンド・結果・未検証範囲が記録

## Verification

- [ ] `cargo test -p ruviform-audio --test typed_probe` exit 0
- [ ] `mise run test-rt` exit 0
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`verify-ci` exit 0
