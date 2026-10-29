---
id: "TASK-034OpsGOwquB9NPINeMKyz"
type: "task"
status: "done"
title: "T12. docs/ledger・最終検証（#120/#101 trigger・index・mise 全量・audit）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/120"
    - "https://github.com/scarecrow173/ruviform/issues/101"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/designs/plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/designs/foundation-observability-minimal-api.md"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
    - "docs/tasks/add-l1-diagnostics-unit-tests.md"
    - "docs/tasks/add-l1-diagnostics-e2e.md"
    - "docs/tasks/verify-l1-video-diagnostics-parity.md"
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

# T12. docs/ledger・最終検証（#120/#101 trigger・index・mise 全量・audit）

## Work

最終的な文書同期と全量検証:

- **trigger ledger（spec R-F3・AC7）**: `docs/designs/plugin-l1-out-of-process.md` 等の trigger 記録に **#120 再トリガ**（per-child 3 スレッド目 — driver+reader+stderr drain のスレッド構成変化）と **#101「observability 最初の実需要」**（`ruviform-observability` が L1 diagnostics に初消費）を記録する。
- **index/README**: `crates/README.md` クレート数カウント・各 index 行の同期。
- **依存理由記録**: `libc`/`windows-sys`（platform-gated — `poll`/`PeekNamedPipe` は std 不在、design 承認済み案A の必要 dep）の追加理由を PR/README に記録。
- **全量検証**: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` + `mise run lint-l1-real-plugin` + `mise run test-l1-real-plugin` 全パス、`doc_status.js audit` blocking 0、`git diff --check` クリーン。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-121-l1-diagnostics.md`
- `docs/specs/issue-121-diagnostics.md`（R-F3・AC7）

## Dependencies

- `docs/tasks/add-l1-diagnostics-unit-tests.md`
- `docs/tasks/add-l1-diagnostics-e2e.md`
- `docs/tasks/verify-l1-video-diagnostics-parity.md`

## Affected Files

- `docs/designs/plugin-l1-out-of-process.md`・`docs/designs/foundation-observability-minimal-api.md`（trigger ledger）
- `crates/README.md`・各 docs index

## Done When

- [x] #120/#101 の trigger ledger に本 issue の変化（3 スレッド目・observability 初消費者）が記録されている
- [x] 全量検証コマンドが実行され結果が記録されている（未実行の検証を実行済みと報告しない）
- [x] `doc_status.js audit` が blocking 0・findings なし

## Verification

- [x] `mise run fmt-check` && `mise run check` && `mise run lint` && `mise run test` && `mise run test-rt` exits 0
- [x] `mise run lint-l1-real-plugin` && `mise run test-l1-real-plugin` exits 0
- [x] `node .agents/skills/doc-status/scripts/doc_status.js audit --type all --json` が `blocking: 0`
- [x] `git diff --check` exits 0
