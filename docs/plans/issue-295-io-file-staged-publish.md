---
id: "PLAN-034RoUNsT1SND8BxtMWhXC"
type: "plan"
status: "approved"
title: "Issue #295 ruviform-io-file 実装計画 (IO-IMP-01)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/295"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-295-io-file-staged-publish.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #295 ruviform-io-file 実装計画 (IO-IMP-01) Implementation Plan

## Goal

新クレート `ruviform-io-file` に IO-01 v0.4 の staged publish 公開契約
（型状態・PublishOptions・Receipt/Failure・RecoveryHandle・Readback）と
private `PlatformFileOps` + Fault Inject Mock を実装し、Mock 駆動の
契約検証を完遂する（実 OS backend は IO-IMP-02 へ委譲）。

## Scope

単一実装ストリーム・1 plan・2 task。変更は `crates/ruviform-io-file/`
新設 + 共有台帳（root `Cargo.toml`・`Cargo.lock`・`crates/README.md`・
`AGENTS.md` クレート数・本 doc チェーン）に限定。実 OS 実装・旧
`StagedOutput` 撤去・Consumer 移行は対象外。

## Upstream Documents

- `docs/specs/issue-295-io-file-staged-publish-contract.md`（approved）
- Issue #295 本文（受入正本）・Issue #283 IO-01 v0.4・Issue #282 統合設計

## Design Inputs

- `docs/designs/issue-295-io-file-staged-publish.md`（approved —
  モジュール構成・`PlatformFileOps` 契約・パイプライン分類表・所有権構造）

## File Map

- `crates/ruviform-io-file/Cargo.toml` — workspace メタ + `[lints] workspace`
- `crates/ruviform-io-file/README.md` — 責務・制約・既知制限
- `crates/ruviform-io-file/src/lib.rs` — crate doc・re-export・静的検証
- `crates/ruviform-io-file/src/error.rs` — `FileError`/`FileErrorKind`
- `crates/ruviform-io-file/src/options.rs` — `PublishOptions`・3 enum
- `crates/ruviform-io-file/src/outcome.rs` — `OperationId`・`PublishEffect`・
  `PublishPhase`・`DurabilityEvidence`・`TempState`・`PublishFailureKind`・
  `PublishFailure`
- `crates/ruviform-io-file/src/receipt.rs` — `PublishReceipt`・`RecoveryInfo`・
  `RecoveryObservation`・`RecoveryHandle`・`CleanupReceipt`・`CleanupFailure`
- `crates/ruviform-io-file/src/staged.rs` — `StagedFile`・`SealedFile`・
  `ReadbackFile`・`StageOwner`・publish パイプライン
- `crates/ruviform-io-file/src/platform.rs` — private `PlatformFileOps`・
  `ApplyOutcome`・`TempDisposition`・`PublishAttempt`・`CleanupAttempt`・
  `ReadSeek`・既定 factory（Unsupported）
- `crates/ruviform-io-file/src/mock.rs` — `#[cfg(test)]` `MockFileOps`・
  注入スクリプト・in-memory FS
- 共有台帳: `Cargo.toml`（members）・`Cargo.lock`・`crates/README.md`・`AGENTS.md`

## Implementation Sequence

- [ ] 1. クレート scaffold（Cargo.toml/README/lib.rs 骨格）+ workspace members 登録
- [ ] 2. `error.rs`/`options.rs`/`outcome.rs`/`receipt.rs` の公開型を実装
- [ ] 3. `platform.rs`（trait + 分類語彙 + Unsupported factory）
- [ ] 4. `staged.rs`（StageOwner・型状態・パイプライン・Readback・Drop/abort）
- [ ] 5. `mock.rs` + unit test 行列 + compile-fail doctest
- [ ] 6. 共有台帳更新（crates/README.md・AGENTS.md・Cargo.lock 再生成）
- [ ] 7. `git fetch origin`→`git merge origin/main`→最終検証→コミット

## Dependencies and Risks

- 依存クレートなし（std のみ）。`Cargo.lock` は `cargo` 経由で再生成し追跡継続。
- リスク: `StageOwner` の Drop 型からの所有権移譲（`ops: Option` + `take()` で
  解決済み・design 確定）、`create` の Unsupported 公開を消費者が誤認する
  リスク（rustdoc + `FileErrorKind::Unsupported` + README で明示）。
- 並行 worktree では他 worker が root `Cargo.toml`/`crates/README.md` を
  触る可能性 → 最終検証前に `origin/main` を取り込み台帳を再整合する。

## Task Breakdown

- T1（`docs/tasks/issue-295-t1-crate-and-contract.md`）: scaffold + 公開型 +
  platform/mock + 型状態・パイプライン + unit test 行列。
- T2（`docs/tasks/issue-295-t2-ledger-verify.md`）: 共有台帳更新・main 取込・
  最終検証（fmt/check/lint/test）・IR 記録。

## Verification

- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test`
      （新クレート含め workspace 全体）
- [ ] test-spec の行列: `PublishEffect` × `TempDisposition` × 失敗段階 ×
      durability 要求の分類が unit test で緑
- [ ] compile-fail doctest（`SealedFile` への `Write`/`Seek`）・
      `Write+Seek+Send` 静的検証
- [ ] `create` → `FileErrorKind::Unsupported`・InvalidInput の unit test

## Open Questions / Gaps

- `ReadbackFile` の seek-range/flush 契約細部は実 OS 実装（#296）の
  handle 設計で追加検証が必要 — 現行は「初期位置 0・Temp 同一対象」のみ pin。

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 worker が実装）
- Independent work streams: none（T1→T2 直列）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 焦点: `create` が Unsupported を返す公開方針、Mock が `#[cfg(test)]` の
  範囲、パイプライン分類表と test-spec の一致。
