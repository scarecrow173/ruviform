---
id: "TASK-mV2zXwAeR5yNuIoL7pSt4G"
type: "task"
status: "done"
title: "Issue #414 — projection→controlled display 正規経路の実装"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/414"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-414-projection-controlled-display.md"
      - type: "file"
        path: "docs/designs/issue-414-projection-controlled-display.md"
      - type: "file"
        path: "docs/plans/issue-414-projection-controlled-display.md"
      - type: "file"
        path: "docs/test-specs/issue-414-projection-controlled-display.md"
      - type: "file"
        path: "docs/impl/ir/issue-414-projection-controlled-display.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/component_view.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-414-projection-controlled-display.md"
  implemented-by:
    - "docs/impl/ir/issue-414-projection-controlled-display.md"
  depends-on:
    - "docs/plans/issue-414-projection-controlled-display.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-414-projection-controlled-display.md"
    - "docs/designs/issue-414-projection-controlled-display.md"
    - "docs/test-specs/issue-414-projection-controlled-display.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-414-projection-controlled-display.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/410"
  defers: []
  deferred-by: []
---

# Issue #414 — projection→controlled display 実装タスク

- [x] `behavior.rs`：`projection_value_input` decode +
      `InteractionControl::apply_external` + `pending_external`
      stage/flush + unit tests
- [x] `view.rs`：`apply_projection` の display 配線 +
      `register_control` の last_observed 復元 + unit tests
- [x] `ruviform-ui-gpui/render.rs`：inbox→display の
      renderer-read-path test 2 件
- [x] `ruviform-ui-test/component_view.rs`：`ui_dvr_09_*` 3 件
- [x] `driver-lab/cases.json`：`UI-DVR-09` case 追加
- [x] doc chain（spec/design/plan/task/test-spec/impl）
- [x] `origin/main` merge + mise 検証一式
- [x] commit / push / PR
