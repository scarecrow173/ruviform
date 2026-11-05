---
id: "TASK-klizn5ZWBc8d4fBskVs3or"
type: "task"
status: "done"
title: "T2 TAR Writer・宣言 size cap・finish 型分離・PAX x 出力 (Issue #303)"
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
    - "docs/plans/issue-303-io-archive-tar.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-303-io-archive-tar.md"
    - "docs/tasks/issue-303-t1-tar-reader.md"
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
    - "docs/test-specs/issue-303-io-archive-tar.md"
  references: []
  defers: []
  deferred-by: []
---

# T2 TAR Writer・宣言 size cap・finish 型分離・PAX x 出力 (Issue #303)

## Work

`src/tar/write.rs`（`TarArchiveWriter<W: Write>`・`start_entry`・
`add_directory`・`TarEntryWriter`・`TarEntryOptions`）を実装する。
`TarEntryWriter` は宣言 size で write を cap し、実書込 < 宣言での
`finish` や未 finish Drop は archive を poison する（`InvalidState`
）。header は USTAR（name ≤100・prefix 分割）を基本とし、長名・
octal 上限超過 size は PAX `x` Entry を先行出力する。`finish` は
2 zero block を書いて inner を返す。`tests/tar_writer.rs` で
cap/poison/長名 PAX/round-trip を検証する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-303-io-archive-tar.md`
- `docs/designs/issue-303-io-archive-tar.md`

## Acceptance

- write cap・宣言不足 finish Err + poison・未 finish Drop poison・
  `TarArchiveWriter::finish` の zero block 出力をテストで pin。
- 長名の USTAR prefix 分割・PAX `x` 先行出力・octal 超過 size の
  PAX `size=` 出力が自前 Reader で round-trip できる。
