---
id: "TASK-630clapEditorVerbs05"
type: "task"
status: "done"
title: "Issue #630 CLAP editor verb 実バイナリ coverage slice — task"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-630-clap-editor-verbs.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-630-clap-editor-verbs.md"
  implements:
    - "docs/plans/issue-630-clap-editor-verbs.md"
  implemented-by:
    - "docs/impl/ir/issue-630-clap-editor-verbs.md"
  depends-on:
    - "docs/plans/issue-630-clap-editor-verbs.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 CLAP editor verb 実バイナリ coverage slice — task

- [x] discovery 決裁: `test_editor` を resize-capable 化
      （ADR 記録）
- [x] `src/editor.rs` resize-capable 化（`can_resize`/`set_size`/
      `win32::resize`/`GuiState.size`/doc・unit test 追従）
- [x] `tests/host_path.rs` 非 ignore 層 2 件 + Windows ignored
      実機層 1 件追加
- [x] `README.md` size verbs 記述追従
- [x] canonical 検証（fmt-check/check/test/build 全 exit 0、
      lint は scope 外既存破損で Fail 記録、adapter 非回帰
      check/lint/test exit 0）
- [x] ignored 実機層実走（`cargo test --test host_path --
      --ignored` exit 0 — 実測値を検証 md に記録）
- [x] `test-plugin-abi-validator` 再実走 + `evidence/rf630/
      abi-validator/` 簿記更新（verdict Passed）
- [x] 検証 md + doc chain + impl ir/exp 完成
- [x] review-protocol・commit・push・PR
