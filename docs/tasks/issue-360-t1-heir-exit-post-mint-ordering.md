---
id: "TASK-034SmDZDujSnKDIPh6c8CJ"
type: "task"
status: "done"
title: "Issue #360 t1 — heir 系 4 サイトの exit を post-mint へ message 順序で固定"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/360"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-360-heir-exit-post-mint-ordering.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-360-heir-exit-post-mint-ordering.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-360-heir-exit-post-mint-ordering.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-360-heir-exit-post-mint-ordering.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-360-heir-exit-post-mint-ordering.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #360 t1 — heir 系 4 サイトの exit を post-mint へ message 順序で固定

## Work

`--leave-heir-after-msgs=2` の 4 サイトで、flag を `3` に変更し mint 復帰後に `Initialize`（msg3）を 1 回送信して leave を発火させる。3rd command の結果は `{Ok(_), Err(HostError::Ipc)}` oracle。誤った前提のコメントを修正し、`l1-test-worker.rs` の flag 契約に mint window の注意を追記する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-360-heir-exit-post-mint-ordering.md`
- `docs/specs/issue-360-l1-proxy-e2e-heir-flake.md`
- `docs/designs/issue-360-heir-exit-post-mint-ordering.md`
- `docs/adr/issue-360-mint-child-exit-test-scenario.md`

## Dependencies

- なし（単独 task）。

## Affected Files

- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（`destroy_detaches_reader_when_heir_holds_stdout`、`leave_heir_yields_exit_observed_and_bounded_drain`、`heir_stderr_inside_grace_lands_in_record`、`heir_stderr_after_drain_stop_is_never_read`）
- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`（flag doc のみ、code 変更なし）

## Done When

- [x] 4 サイトの `--leave-heir-after-msgs` が `3` で、各テストが mint 後に `Initialize` を 1 回送信する。
- [x] 3rd command 結果が `Ok(_) | Err(HostError::Ipc)` のみ許容される。
- [x] `l1-test-worker.rs` の `--leave-heir-after-msgs` doc が handshake 内回数での mint window exit を警告する。

## Verification

- [x] `mise exec -- cargo test -p ruviform-plugin-host --test l1_proxy_e2e heir`（6 回連続 green、SpawnFailed 非観測。Windows 11 x64 / Rust 1.99.0）。
- [x] `mise run fmt-check`（exit 0）/ `mise run lint`（exit 0）/ `mise run test`（exit 0、147 suite 全 green）。
