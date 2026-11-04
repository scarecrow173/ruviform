---
id: "TASK-AacpjXZ02dbHUiacENEOSZ"
type: "task"
status: "done"
title: "T1 SinkDegraded Clone derive 追加（Issue #336）"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/336"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-observability/src/logging/build.rs"
      - type: "file"
        path: "crates/ruviform-observability/tests/logging.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-336-sink-degraded-clone.md"
  implemented-by:
    - "docs/impl/ir/implement-issue-336-sink-degraded-clone.md"
  depends-on:
    - "docs/plans/issue-336-sink-degraded-clone.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-336-sink-degraded-clone.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  task:
    lane: "implementation"
---

# T1 SinkDegraded Clone derive 追加（Issue #336）

## Goal

`crates/ruviform-observability/src/logging/build.rs` の `SinkDegraded` に
`Clone` を derive し、`tests/logging.rs` に clone 複製がフィールド値を
保持する最小アサーションを追加する。

## Acceptance

- `SinkDegraded` が `#[derive(Debug, Clone)]` になる（他の trait は追加しない）。
- `result.degraded.clone()` の複製が `sink_index` / `kind` / `reason` を
  保持することをテストで示す。
- `LoggingBuildResult` / `LoggingLayersResult` は非 Clone のまま。
- `mise run fmt` / `check` / `lint` / `test` が green。

## 関連文書

- spec: [docs/specs/issue-336-sink-degraded-clone.md](../specs/issue-336-sink-degraded-clone.md)
- ADR: [docs/adr/issue-336-sink-degraded-clone-derive.md](../adr/issue-336-sink-degraded-clone-derive.md)
- design: [docs/designs/issue-336-sink-degraded-clone.md](../designs/issue-336-sink-degraded-clone.md)
- 実装計画: [docs/plans/issue-336-sink-degraded-clone.md](../plans/issue-336-sink-degraded-clone.md)
