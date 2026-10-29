---
id: "TASK-034OpsFegc9YAYfop3MYbp"
type: "task"
status: "done"
title: "T10. L1 diagnostics E2E（counter・stderr・cause・bundled・supervisor・pre-mint）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
    - "docs/tasks/wire-l1-audio-counters.md"
    - "docs/tasks/add-pre-mint-diagnostics.md"
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
    - "docs/test-specs/issue-121-l1-audio-counters-tests.md"
    - "docs/test-specs/issue-121-l1-stderr-drain-tests.md"
    - "docs/test-specs/issue-121-l1-diagnostics-retention-tests.md"
    - "docs/test-specs/issue-121-l1-failure-cause-notification-tests.md"
    - "docs/test-specs/issue-121-l1-parity-nonregression-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# T10. L1 diagnostics E2E（counter・stderr・cause・bundled・supervisor・pre-mint）

## Work

`l1-test-worker` + `PluginHost` 公開面で E2E oracle を書く（`l1_proxy_e2e`・必要に応じて `l1_supervisor_e2e`）:

- **counter 観測**: underrun 発生 → `dropped_submits`/`passthrough_submits`/`results_failed_observed` の query 観測（in-place + detached endpoint 双方）
- **stderr**: `--stderr-bytes` で `Hello` 前出力を含む捕捉・pipe capacity 超過で drain 継続 + tail 保持 + `dropped_bytes` 精確・`--heir-stderr-*` で deadline/unread flags・heir grandchild で bounded teardown（OS 別期待値 — POSIX deadline・Windows は flag 観測）
- **cause payload**: EOF→`Transport`・timeout→`Timeout`・違反→`Protocol` の通知 `cause` が観測できる。**`ExitObserved` の deterministic oracle は `--leave-heir-after-msgs` fixture**（grandchild が stdout を保持 → EOF が来ず `try_wait` が唯一の観測者）。plain crash は EOF と exit が競合するため `Transport`/`ExitObserved` の lock 直列化順 — どちらかを断言しない。`ChildExit{code,signal}` 语义保持・exactly-once 無退行
- **bundled fan-out**: 同居全 instance が同一 `cause` の通知を受け、診断レコードは child スコープで 1 件
- **supervisor**: watched `InstanceFailed{cause}` の `{handle, ..}` 消費・`Restarted`/`GaveUp` 语义不変・Unsupervised pass-through
- **pre-mint**: 各 `phase`（pre-spawn 検証/OS spawn/drain 初期化/mint・handshake）で `Err(SpawnFailed{cause, diagnostics_id})`・rollback で partial child 残存なし・`Err(InstanceLimit)`/`Err(Ipc)` パススルー無退行・`found/evicted/not-found` query

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-121-l1-diagnostics.md`
- 各 `docs/test-specs/issue-121-l1-*.md`（Rules/Examples が oracle の正本）

## Dependencies

- `docs/tasks/wire-l1-audio-counters.md`（counter）
- `docs/tasks/add-pre-mint-diagnostics.md`（retention・fixture・cause）

## Affected Files

- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`・`tests/l1_supervisor_e2e.rs`

## Done When

- [x] counter・stderr・cause・bundled・supervisor・pre-mint の各 oracle が新規 E2E で pin されている
- [x] 通知 exactly-once・`ChildExit` status・`Err(InstanceLimit)`/`Err(Ipc)` パススルーが無退行
- [x] heir bounded teardown が OS 別期待値で pin されている

## Verification

- [x] `mise run test` exits 0
- [x] `mise exec -- cargo test --test l1_supervisor_e2e` exits 0
