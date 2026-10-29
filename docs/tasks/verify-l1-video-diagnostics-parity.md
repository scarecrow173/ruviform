---
id: "TASK-034OpsG2QXJ66gdCKuXWSJ"
type: "task"
status: "done"
title: "T11. video parity（stderr・cause・counter 非混入・fixture 同条件）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/118"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/video.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_video_e2e.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
    - "docs/tasks/pipe-and-drain-l1-stderr.md"
    - "docs/tasks/wire-failure-cause-classification.md"
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
    - "docs/test-specs/issue-121-l1-parity-nonregression-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# T11. video parity（stderr・cause・counter 非混入・fixture 同条件）

## Work

`l1_video_e2e` で video dedicated 子の diagnostics parity を検証する — `VideoL1Instance::spawn`（`l1/video.rs` ~:150-165）は `spawn_scanned`・notify 機構を共有するため主な配線は `l1/mod.rs` 側で既に通っているはず（`src/video.rs` facade は spawn/IO を持たず変更不要）。

- video 子が piped stderr + drain + retention を同じ契約で持つ（`--stderr-bytes` で `l1-test-video-worker` 出力を捕捉 — fixture flag は T6 で両 worker 同名追加済み）
- render timeout → `Timeout`・protocol 違反 → `Protocol`・crash → `ExitObserved` の通知 payload が audio と同一语义
- `create_video_instance` 失敗の pre-mint 経路が `SpawnFailed{cause, diagnostics_id}` で返る（`InstanceLimit` パススルー無退行）
- audio counter が video route に混入しない（video に counter query 面を作らない）

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-121-l1-diagnostics.md`
- `docs/test-specs/issue-121-l1-parity-nonregression-tests.md`

## Dependencies

- `docs/tasks/pipe-and-drain-l1-stderr.md`（stderr drain + video fixture flag）
- `docs/tasks/wire-failure-cause-classification.md`（cause 配線）
- `docs/tasks/add-pre-mint-diagnostics.md`（`create_video_instance` の `SpawnFailed{cause, diagnostics_id}` 経路は T8 の成果 — 依存なしでは検証不能）

## Affected Files

- `crates/ruviform-plugin-host/src/l1/video.rs`（必要な配線のみ — ほぼ `l1/mod.rs` 共有機構で通る）
- `crates/ruviform-plugin-host/tests/l1_video_e2e.rs`

## Done When

- [x] video 子の stderr 配管・drain・deadline・flags が audio と同一契約で動く
- [x] render timeout/protocol 違反の通知 `cause` が pin されている
- [x] video route に audio counter が混入していない

## Verification

- [x] `mise exec -- cargo test --test l1_video_e2e` exits 0
- [x] `mise run lint-l1-real-plugin` exits 0
