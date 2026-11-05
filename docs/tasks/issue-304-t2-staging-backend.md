---
id: "TASK-KfnrekDg0F7pVPV37olgXQ"
type: "task"
status: "done"
title: "T2 Staging backend・Session/Tree・identity cleanup (Issue #304)"
created: "2026-11-05"
updated: "2026-11-05"
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
    - "docs/plans/issue-304-io-archive-extract.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-304-io-archive-extract.md"
    - "docs/tasks/issue-304-t1-path-policy.md"
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
    - "docs/test-specs/issue-304-io-archive-extract.md"
  references: []
  defers: []
  deferred-by: []
---

# T2 Staging backend・Session/Tree・identity cleanup (Issue #304)

## Work

unix（dirfd/`openat`/`O_NOFOLLOW`/`O_CREAT|O_EXCL`）と windows
（`NtCreateFile`/`NtOpenFile`/`NtDeleteFile` + `RootDirectory`/
`OBJ_DONT_REPARSE`/`FILE_OPEN_REPARSE_POINT`）の `StagingDir`/
`StagingFile` backend、`ExtractionSession`/`ExtractedTree`/
`ExtractOptions`/`ExtractionReceipt`/`ExtractionFailure`/
`ExtractionRecoveryHandle`/`CleanupReceipt`/`ExtractionCleanupFailure`
を実装する。cleanup は records の node identity 照合付き降下削除
（差し替えは消さず retained 報告）。`tests/extract_staging.rs` で
実 FS 試験（排他 create・identity・Drop/cleanup・symlink/junction/
foreign dir 差し替え・recovery inspect/cleanup）を pin する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-304-io-archive-extract.md`
- `docs/designs/issue-304-io-archive-extract.md`

## Acceptance

- 隔離 Staging Root が排他的に作成され、`extract_zip`/`extract_tar`/
  `abort` の状態機械・`ExtractedTree` の path/receipt/handoff/Drop が
  契約どおり動くことを実 FS 試験で pin する。
- cleanup が root/中間 dir の identity 照合で差し替えを検出し、
  tree 外・foreign node を削除しないことを symlink/junction/foreign
  dir の差し替え試験で pin する（Windows 分はローカル実測）。
- `ExtractionFailure` が `Option<ExtractionRecoveryHandle>` を
  同梱し、`inspect`/`cleanup` が動くことを mock/実 FS で pin する。
