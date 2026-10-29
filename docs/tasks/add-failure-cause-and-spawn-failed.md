---
id: "TASK-034OpsfP4ZxOuDqHZhEILm"
type: "task"
status: "done"
title: "T2. FailureCause + HostError::SpawnFailed 型追加（boundary.rs・codec 互換テスト）"
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
        path: "crates/ruviform-plugin-core/src/boundary.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
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
    - "docs/test-specs/issue-121-l1-failure-cause-notification-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# T2. FailureCause + HostError::SpawnFailed 型追加（boundary.rs・codec 互換テスト）

## Work

`crates/ruviform-plugin-core/src/boundary.rs` に `FailureCause`（`pub`・`Copy`・`#[non_exhaustive]`・7 variant — `Spawn`/`Handshake`/`Timeout`/`Protocol`/`Transport`/`ExitObserved`/`Internal`）を追加し、enum doc に設計の意味ベース分類表を転記する（`HostError` と同じ配置 — `HostNotification::InstanceFailed.cause` と `HostError::SpawnFailed.cause` の両方から参照されるため plugin-core 所在が依存方向として正しい）。`HostError::SpawnFailed { cause: FailureCause, diagnostics_id: u64 }` variant を追加（`Copy` payload）。in-crate の exhaustive match を修正。`wire.rs` に codec 互換テストを追加: `encode_host_error(SpawnFailed)` → `Err(WireError::UnsupportedVariant)`（`:1947` の `_` catch-all 経由・`EndpointUnavailable` と同じ local-only 扱い）+ `WIRE_VERSION` 不変。`FailureCause`/`HostNotification`/`ChildExit` は wire 型ではないため serialize 対象外 — codec テストは `HostError` 系の網羅で担保する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-121-l1-diagnostics.md`
- `docs/designs/issue-121-l1-diagnostics.md`（「原因の分類と収束」節の分類表）
- `docs/adr/issue-121-l1-diagnostics-policy.md`

## Dependencies

- なし（先行独立タスク）

## Affected Files

- `crates/ruviform-plugin-core/src/boundary.rs`
- `crates/ruviform-plugin-core/src/wire.rs`

## Done When

- [ ] `FailureCause` 7 variant が enum doc の分類表つきで追加されている
- [ ] `HostError::SpawnFailed{cause, diagnostics_id}` が追加されている
- [ ] codec テストが `SpawnFailed` → `UnsupportedVariant`・`WIRE_VERSION` 不変を pin

## Verification

- [ ] `mise run check` exits 0
- [ ] `mise run lint` exits 0
- [ ] `mise exec -- cargo test -p ruviform-plugin-core` exits 0
- [ ] `mise exec -- cargo doc -p ruviform-plugin-core` 警告なし
