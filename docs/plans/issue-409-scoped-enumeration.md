---
id: "PLAN-034TnojDZIYRnnvSTlTg34"
type: "plan"
status: "draft"
title: "Issue #409 scoped re-root + bounded enumeration 実装計画"
created: "2026-11-07"
updated: "2026-11-07"
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
    - "docs/specs/issue-409-scoped-enumeration.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-409-scoped-enumeration.md"
    - "docs/designs/issue-409-scoped-enumeration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-409-scoped-enumeration.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #409 scoped re-root + bounded enumeration 実装計画 Implementation Plan

## Goal

保持 capability を新 floor とする re-root と、停止可能な bounded
directory 列挙を `ruviform-io-file` に実装する。

## Scope

一つの実装ストリーム（同一 crate 内の vfs 境界変更に root 公開
API が依存するため分割しない）。`ruviform-io-file` と直接関連
test/docs/mise のみ。

## Upstream Documents

- `docs/specs/issue-409-scoped-enumeration.md`（正本）

## Design Inputs

- `docs/designs/issue-409-scoped-enumeration.md`（approved）

## File Map

- `crates/ruviform-io-file/src/vfs.rs` - `VfsDir::list_each` +
  `list` default collect、`VisitFlow`/`ListingOutcome`
- `crates/ruviform-io-file/src/vfs/unix.rs` - `list_each`（独立
  DIR*・Drop guard・per-entry probe）
- `crates/ruviform-io-file/src/vfs/windows.rs` - `list_each`
  （NtQueryDirectoryFile batch 内 stop）
- `crates/ruviform-io-file/src/vfs/unsupported.rs` - `list_each`
  = `Unsupported`
- `crates/ruviform-io-file/src/vfs/mock.rs` - tree-model 化・lazy
  列挙・counter/fault 注入
- `crates/ruviform-io-file/src/rooted.rs` - `open_scoped_root`・
  `read_dir_each`・`EntryFlow`/`EnumerationOutcome`・`read_dir`
  collect 化・新 scope floor 構築（anchor 注入・chain=[dir]；
  `resolve.rs` は不変）
- `crates/ruviform-io-file/src/rooted/tests.rs` - mock 単体 test
- `crates/ruviform-io-file/tests/rooted_read.rs` - real FS
  統合 test（rename/replacement・bounded・panic・独立 state）
- `crates/ruviform-io-file/README.md` - 公開 API 追記
- `mise.toml` - 必要時のみ最小 task

## Implementation Sequence

- [x] vfs `list_each`/`VisitFlow`/`ListingOutcome` を定義し
  `list` を default collect 化
- [x] unix backend `list_each`（fdopendir・Drop guard・lazy）
- [x] windows backend `list_each`（batch 内 stop・次 call 抑止）
- [x] unsupported backend `list_each` = `Unsupported`
- [x] mock tree-model 化 + lazy enum + counter/fault
- [x] `RootedDir::open_scoped_root`（anchor 必須・handle 権威）
- [x] `RootedDir::read_dir_each` + `read_dir` collect 化
- [x] RED test 追加（mock + real FS）→ 失敗確認 → GREEN
- [x] README/docs 更新・mise 最小 task 追加（不足時のみ）
- [ ] mise fmt-check/check/lint/test + targeted lane 実測
- [ ] origin/main merge → lock/index 再生成 → 再検証
- [ ] 独立 review 修正確認 → commit/push/PR → worker_done

## Dependencies and Risks

- `canonicalize_handle` を持たない backend で re-root は
  `Unsupported`（縮小契約は採用しない、spec deferred）。
- mock tree-model 化は既存 `put`/`remove` 使用形態を保持する
  必要（既存 test 非破壊）。
- unix のみ unsafe（fdopendir/closedir）は既存同様の最小範囲に
  留める（unsafe 新規拡散禁止に従い Drop guard 内のみ）。

## Task Breakdown

- `docs/tasks/issue-409-scoped-enumeration.md`（単一 task、
  vfs→rooted→tests→docs の順序を Done When で固定）

## Verification

- [ ] `mise run fmt-check`（0 差分）
- [ ] `mise run check`（workspace 全対象）
- [ ] `mise run lint`（clippy -D warnings）
- [ ] `mise run test`（workspace、既存 rooted/archive 回帰含む）
- [ ] targeted: io-file の rooted tests・archive tests
- [ ] `check-linux`（unix backend compile/lint、未実測 OS は
  NotRun 明記）

## Open Questions / Gaps

- `read_dir_each` visitor error 伝播は現行 capture-slot 方式
  （generic error 型は deferred）。Issue 例示 API に合わせる。
- Windows batch size は既存 64KB を維持（Issue 許容の OS 固定
  batch、明示済）。

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（同一 crate 内で密結合の
  ため subagent 分割は行わない）
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- レビュー焦点: anchor 必須の `Unsupported` 分類が Issue 指針
  （path rebind 禁止・黙示縮小禁止・Weak 一律拒否禁止）と整合
  するか、mock tree-model 化が既存 test の前提（parent-first
  put）を壊さないか、windows batch 内 stop が「後続 OS call
  不発行」を満たすか。
