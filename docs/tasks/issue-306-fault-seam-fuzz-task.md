---
id: "TASK-034WXSB8zGVc9NKQicjOEp"
type: "task"
status: "done"
title: "Issue #306 G07 fault seam 判定・契約試験 + G08 corpus replay"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-306-fault-seam-fuzz-task.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-306-fault-seam-fuzz-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-306-fault-seam-fuzz-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-306-fault-seam-fuzz.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 G07 fault seam 判定・契約試験 + G08 corpus replay

## Work

- G07: `ruviform-io-file` に opt-in `fault-injection` feature +
  `#[doc(hidden)]` `mock` module facade を追加し、`ruviform-io-e2e`
  `tests/fault_seam.rs` で `Indeterminate`・publish 後 DirSync
  失敗・`TempState::Unknown`・phase 別 `NotPublished` を公開 API
  のみで決定的に検証する。実 OS で誘発不能な class は
  mock-oracle + OS capability 根拠で `Unsupported` を維持する。
- G08: `tests/corpus_replay.rs` に固定 seed・bounded・決定的な
  corpus-replay 試験を `#[ignore]` で追加し、`mise.toml` に
  opt-in task `test-io-e2e-corpus` を追加。Windows で 1 回実行し
  証拠を残す。
- 証拠・記述・doc チェーン・index 再生成を完了させる。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-306-fault-seam-fuzz-plan.md`
- `docs/specs/issue-306-fault-seam-fuzz.md`
- `docs/designs/issue-306-fault-seam-fuzz-design.md`（approved）
- `docs/test-specs/issue-306-fault-seam-fuzz.md`

## Dependencies

- 前提実装: `ruviform-io-file` の `PlatformFileOps`/`MockFileOps`/
  `publish_pipeline`（既存）。`with_ops`/`StageOwner`/`RecoveryInfo`
  は crate-private — facade は crate 内に限定。
- io-e2e dev-dep feature 指定は test target のみ影響。

## Affected Files

- `crates/ruviform-io-file/Cargo.toml`
- `crates/ruviform-io-file/src/lib.rs`
- `crates/ruviform-io-file/src/mock.rs`
- `crates/ruviform-io-file/src/staged.rs`（cfg 拡張のみ）
- `crates/ruviform-io-e2e/Cargo.toml`
- `crates/ruviform-io-e2e/src/lib.rs`
- `crates/ruviform-io-e2e/README.md`
- `crates/ruviform-io-e2e/tests/fault_seam.rs`（新規）
- `crates/ruviform-io-e2e/tests/corpus_replay.rs`（新規）
- `mise.toml`（task 追加のみ）
- `evidence/rf306/fault-seam-fuzz-report.md`（新規）

## Done When

- [x] G07 seam 判定が技術的根拠付きで確定し、契約試験が
      deterministic に pass（Indeterminate / DirSync 後失敗 /
      TempState::Unknown / phase 別 NotPublished）
- [x] seam が公開 API・既定 feature build・RT 面を変えないことを
      feature 無し check/lint/test で確認
- [x] G08 opt-in task が `mise.toml` に追加され、Windows で 1 回
      実行した記録が evidence に残る
- [x] fmt/check/lint/test の該当 lane が pass
- [x] docs index が doc-maintenance で再生成されている

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check-io-file` `lint-io-file` `test-io-file`
- [x] `mise run check-io-e2e` `lint-io-e2e` `test-io-e2e`
- [x] `mise run test-io-e2e-corpus`（Windows 1 回）
- [ ] 波及時: io-structured / io-compression の check/lint/test
