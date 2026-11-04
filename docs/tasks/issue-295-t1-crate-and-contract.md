---
id: "TASK-034RoWzXiLur2zHG5iHl7u"
type: "task"
status: "done"
title: "T1 ruviform-io-file クレート新設・公開契約・Mock Publish (Issue #295)"
created: "2026-11-04"
updated: "2026-11-04"
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
    - "docs/plans/issue-295-io-file-staged-publish.md"
  implemented-by:
    - "docs/impl/ir/issue-295-ruviform-io-file-staged-publish-contract.md"
  depends-on:
    - "docs/plans/issue-295-io-file-staged-publish.md"
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
    - "docs/test-specs/issue-295-io-file-staged-publish.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 ruviform-io-file クレート新設・公開契約・Mock Publish (Issue #295)

## Work

`crates/ruviform-io-file` を新設し、IO-01 v0.4 の公開契約
（`StagedFile`/`SealedFile`/`ReadbackFile`/`PublishOptions` 3 軸・
`PublishReceipt`/`PublishFailure`・`RecoveryInfo`/`RecoveryHandle`・
`CleanupReceipt`/`CleanupFailure`・`FileError`/`FileErrorKind`）、
private `PlatformFileOps` trait + 分類語彙、`#[cfg(test)]` Fault Inject
Mock、publish パイプラインの決定的分類ロジック、unit test 行列、
compile-fail doctest を実装する。workspace `members` に登録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-295-io-file-staged-publish.md`
- `docs/designs/issue-295-io-file-staged-publish.md`
- `docs/specs/issue-295-io-file-staged-publish-contract.md`

## Dependencies

- なし（crate 依存ゼロ。Issue #283 v0.4 契約が実装基準）

## Affected Files

- `crates/ruviform-io-file/**`（新設）
- `Cargo.toml`（members）・`Cargo.lock`

## Done When

- [x] 公開 API がコンパイル可能で rustdoc 典型使用例がある
- [x] Mock 行列の unit test が緑（effect × temp × 失敗段階 × durability）
- [x] compile-fail doctest・`Write+Seek+Send` 静的検証・`create` の
      Unsupported/InvalidInput 検証がある

## Verification

- [x] `mise run check` / `mise run lint` / `mise run test` が新クレート込みで緑
      （全 PASS — IR の Validation 表参照）
