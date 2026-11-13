---
id: "PLAN-034WXSiTHXNyUkkb6VasRN"
type: "plan"
status: "completed"
title: "Issue #306 G07/G08 fault seam・corpus replay 実装計画"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-306-fault-seam-fuzz-plan.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-306-fault-seam-fuzz.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-306-fault-seam-fuzz-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-fault-seam-fuzz.md"
    - "docs/designs/issue-306-fault-seam-fuzz-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-306-fault-seam-fuzz.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 G07/G08 fault seam・corpus replay 実装計画 Implementation Plan

## Goal

Issue #306 残件のうち headless に実装可能な G07（test-only fault
seam 判定・公開 API 経由の契約試験）と G08（bounded・再現可能な
corpus-replay opt-in 入口）を、公開 API・既定 build を変えずに
実装し、実行証拠を `evidence/rf306/` に残す。

## Scope

単一 implementation stream。G07（io-file seam + io-e2e 契約試験）
と G08（io-e2e replay 試験 + mise opt-in task）は同じ slice の
受入条件であり、seam 実装と契約試験は順序依存するが分割はしない。

## Upstream Documents

- `docs/specs/issue-306-fault-seam-fuzz.md`
- `docs/designs/issue-306-fault-seam-fuzz-design.md`（approved）
- `docs/test-specs/issue-306-fault-seam-fuzz.md`
- `docs/specs/issue-306-io-e2e-fault-fuzz.md`（前 slice 正本）

## Design Inputs

- `docs/designs/issue-306-fault-seam-fuzz-design.md`
- `docs/designs/issue-295-io-file-staged-publish.md`
- `docs/designs/issue-296-io-file-os-backend.md`

## File Map

- `crates/ruviform-io-file/Cargo.toml` - `fault-injection` feature 追加
- `crates/ruviform-io-file/src/lib.rs` - `mock` module の cfg 拡張
- `crates/ruviform-io-file/src/mock.rs` - facade（`FaultScript`/
  `PublishScript`/`PublishReport`/`TempReport`/`staged_file`）追加、
  `MockFs` の pub 化
- `crates/ruviform-io-file/src/staged.rs` - `with_ops` の cfg 拡張
  （test-only・最小変更）
- `crates/ruviform-io-e2e/Cargo.toml` - dev-dep に
  `ruviform-io-file` の `fault-injection` feature 指定
- `crates/ruviform-io-e2e/tests/fault_seam.rs` - G07 契約試験
- `crates/ruviform-io-e2e/tests/corpus_replay.rs` - G08 replay 試験
- `crates/ruviform-io-e2e/src/lib.rs` / `README.md` - 記述更新
- `mise.toml` - `test-io-e2e-corpus` opt-in task 追加
- `evidence/rf306/fault-seam-fuzz-report.md` - 判定・結果・実行記録
- 本 doc チェーン（spec/design/plan/task/test-spec/ir/exp）

## Implementation Sequence

- [x] io-file: `fault-injection` feature + `mock` module cfg 拡張 +
      facade 実装 + `with_ops` cfg 拡張
- [x] io-file: feature 無し `check`/`clippy`/`test` で既定 build
      不変を確認
- [x] io-e2e: dev-dep feature 指定 + `tests/fault_seam.rs` 契約試験
      実装（Indeterminate / DirSync 後失敗 / TempState::Unknown /
      phase 別 NotPublished）
- [x] io-e2e: `tests/corpus_replay.rs`（splitmix64 PRNG・seed・
      budget・deadline・format 別除外）実装 + `#[ignore]` 付与
- [x] `mise.toml` に `test-io-e2e-corpus` opt-in task 追加
- [x] io-e2e README/lib.rs 記述更新
- [x] Windows で corpus lane 1 回実行 + G07/G08 検証 lane 実行
- [x] `evidence/rf306/fault-seam-fuzz-report.md` 記録
- [x] docs index 再生成（doc-maintenance）
- [x] review-protocol → commit → push → PR

## Dependencies and Risks

- `StagedFile::with_ops`・`StageOwner`・`RecoveryInfo` は
  `pub(crate)`/private — facade は io-file crate 内に置き、公開
  signature に private 型を出さない（`private_interfaces` 回避）。
- `MockScript`/`MockFileOps` は既存 `#[cfg(test)]` — cfg を
  `any(test, feature = "fault-injection")` へ拡張する最小変更のみ。
  `MockScript` の pub(crate) 型を facade の pub signature に
  出さない。
- io-e2e dev-dep の feature 指定は io-file の feature を有効化する
  が、io-e2e 自体の公開面・既定 build には影響しない
  （dev-dependencies は test target のみ）。
- corpus replay の panic は `catch_unwind` で分類し fail せず
  記録、budget 超過は fail-loud。

## Task Breakdown

- `docs/tasks/issue-306-fault-seam-fuzz-task.md`（本 plan の唯一の
  task。G07/G08 は同一 slice・順序依存のため分割しない）

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check-io-file` / `lint-io-file` / `test-io-file`
      （feature 無し = 既定 build 不変）
- [x] `mise run check-io-e2e` / `lint-io-e2e` / `test-io-e2e`
- [x] `mise run test-io-e2e-corpus`（opt-in・Windows 1 回実行）
- [x] `cargo check -p ruviform-io-file --features fault-injection`
      / `--no-default-features` 系の補助確認（必要時）

## Open Questions / Gaps

- corpus seed bytes の具体的な case 数・byte budget は実装時に
  test file 内 `const` で確定し evidence に記録する
  （設計の bound 方針は確定済み）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker dispatch 済み・
  単一 stream）
- Independent work streams: none
- User approval required before dispatch: yes（review-protocol で
  代替 — coordinator 契約に基づく）
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute
  task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- reviewer focus: seam が公開 API/既定 build/RT 面を侵さないこと、
  facade が private 型を公開 signature に出さないこと、G08 が
  bounded/opt-in/決定的であること、NotRun/Unsupported を Pass に
  繰り上げていないこと。
