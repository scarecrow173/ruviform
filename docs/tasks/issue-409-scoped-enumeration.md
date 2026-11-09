---
id: "TASK-034TnjAUsNWElluY7lj4cl"
type: "task"
status: "done"
title: "Issue #409 scoped re-root + bounded enumeration"
created: "2026-11-07"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/409"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-409-scoped-enumeration.md"
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
    - "docs/plans/issue-409-scoped-enumeration.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #409 scoped re-root + bounded enumeration

## Work

`open_scoped_root`（保持 handle を新 floor）と `read_dir_each`
（bounded/停止可能列挙）を vfs 境界から rooted 公開 API まで
実装する。vfs `list_each` を全 backend に導入し `list` を
collect 化、mock を tree-model + lazy 列挙へ改修し、単体・統合
test を追加する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-409-scoped-enumeration.md`
- `docs/designs/issue-409-scoped-enumeration.md`
- `docs/plans/issue-409-scoped-enumeration.md`
- `docs/test-specs/issue-409-scoped-enumeration.md`

## Dependencies

- なし（同一 crate 内変更、外部依存追加なし）

## Affected Files

- `crates/ruviform-io-file/src/{vfs.rs,rooted.rs}`
- `crates/ruviform-io-file/src/vfs/{unix.rs,windows.rs,unsupported.rs,mock.rs}`
- `crates/ruviform-io-file/src/rooted/tests.rs`
- `crates/ruviform-io-file/tests/rooted_read.rs`
- `crates/ruviform-io-file/README.md`

## Done When

- [x] `open_scoped_root` が解決 handle のみを新 floor とし、
  anchor 不能時 `Unsupported`、identity 照合なし。
- [x] `read_dir_each` が全量 Vec 化せず `Stop` 後に OS 列挙 /
  probe / callback を続行しない。`read_dir` は collect wrapper。
- [x] test-spec の Examples が mock + real FS test で固定され、
  既存 rooted/archive test が非破壊で通過。
- [x] 実測した OS の検証結果を記録し、未実測 OS は NotRun
  （記録は commit/PR/worker_done 報告にて）。

## Verification

- [x] `mise run fmt-check` / `check` / `lint` / `test` が pass
  （targeted lane は pass 済；workspace 全体の check/lint/test は
  origin/main merge 後に最終実測 — 2026-11-09 `eaab7230`→`967b6fd8`
  取り込み後に全て exit 0 を再確認）。
- [x] targeted io-file + archive 回帰が pass
  （`check-io-file`/`lint-io-file`/`test-io-file` exit 0）。
- [x] `mise run fmt-check` pass・`mise run check-linux`
  （x86_64-unknown-linux-gnu check+clippy）pass。
- [x] 独立 review（Reviewer A cooperative / B adversarial）実施・
  blocking findings 修正確認済（unix readdir errno・Windows
  per-call FILE_OBJECT・sb.Information bounds・anchor 失敗
  `Unsupported` 化・mock node-children 化）。
