---
id: "TASK-QQTcvWDmsdkWt6I3mJdYEa"
type: "task"
status: "done"
title: "Issue #196 T-4: verification + capability evidence + impl records"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/196"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-196-typed-graph-plugin-adapter.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-196-t-3-bridge-crate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-196-typed-graph-plugin-adapter.md"
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-196-typed-graph-plugin-adapter-tests.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T-4: verification + capability evidence + impl records

## Deliverable

- `crates/ruviform-audio-plugin-bridge/tests/`: multi-bus E2E・
  loop=100/min=128・座標一致・fault・capability matrix・`rt_` 無割当。
- M4-01〜08 実測証拠（test 出力 + impl records）。
- `docs/impl/ir/issue-196-t-{1,2,3,4}-*.md`。
- L0/L1 別検証記録（L1 = bus 列挙 Unavailable・規格 lane 回帰のみ）。

## Acceptance

- `cargo test -p ruviform-audio-plugin-bridge` 緑。
- `cargo test -p ruviform-plugin-host --locked`・`cargo test --workspace
  --locked` 緑。
- `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 緑。
- `mise run test-l1-real-plugin`/`lint-l1-real-plugin` 緑（回帰）。

## Notes

- 未対応 Case は名前と理由を残す（L1 bus 列挙・CLAP in-block・全状態
  preflight・TT-001〜024 の非 adapter 層ケース）。
