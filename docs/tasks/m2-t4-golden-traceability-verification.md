---
id: "TASK-034Qfs62FTCiTj3vNuOwAz"
type: "task"
status: "done"
title: "T-4 golden legacy, EX/TA traceability, merge+lockfile+verification, impl record"
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
    - "docs/tasks/m2-t3-tests-ownership-poc-faults.md"
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

# T-4 golden legacy, EX/TA traceability, merge+lockfile+verification, impl record

## Work

plan Step 6-7: `tests/golden_legacy.rs`（旧 API golden fixture・G2-07）、EX-001〜EX-020/TA-001〜TA-012 の traceability を impl record `docs/impl/ir/issue-195-m2-typed-graph-audio-runtime.md` に記録、`git fetch origin && git merge origin/main` → `cargo generate-lockfile` → 全検証実行・証拠記録。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-195-m2-typed-graph-audio-runtime.md`
- `docs/test-specs/issue-195-m2-typed-graph-compiler-runtime-tests.md`

## Dependencies

- `docs/tasks/m2-t3-tests-ownership-poc-faults.md`

## Affected Files

- `crates/ruviform-audio/tests/golden_legacy.rs`
- `docs/impl/ir/issue-195-m2-typed-graph-audio-runtime.md`
- `Cargo.lock`

## Done When

- [ ] golden fixture が旧 API で byte 同一 PCM を出力・既存 suite 不変
- [ ] EX/TA 全 ID が test→impl→evidence に対応付け・N/A/未検証が明示される
- [ ] merge 後に全検証コマンドが実行され exit code が impl record に記録される

## Verification

- [ ] `mise exec -- cargo test -p ruviform-audio --test golden_legacy` exit 0
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` exit 0
- [ ] `mise exec -- cargo test -p ruviform-audio --locked` / `cargo test --workspace --locked` exit 0
