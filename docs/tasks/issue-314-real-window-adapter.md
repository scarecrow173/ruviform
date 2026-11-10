---
id: "TASK-314rwDs5nB9vGtH4jK6mZ"
type: "task"
status: "completed"
title: "Issue #314 slice 4 residual — real-window adapter tasks"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "docs/plans/issue-314-real-window-adapter.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-314-real-window-adapter.md"
---

# tasks

- [x] T1 `real-scenario` feature + `UiJob`/`Inner.ui_jobs`/`set_ui_jobs`/`drain_ui_jobs` + `view_of`/`minted_from_ipc` 共有化
- [x] T2 `gpui_driver/real_window.rs`: `RealOsGuiTest` run/settle/call、SemanticGuiTest/VisualGuiTest impl、honest capabilities
- [x] T3 `tests/real_window.rs`: canonical story walk + idle/teardown pixel capture + golden_check promote→pass + evidence env gate
- [x] T4 `stories.json` `drivers.real-os` → pass（実績 note + cargo-test mechanism）
- [x] T5 `evidence/issue-314-real-window/` 生成（実 desktop で実行）
- [x] T6 文書チェーン + IR（#262/#313 証拠 link 含む）
- [ ] T7 mise 検証 lane・merge main・review-protocol・PR（本 task の残工程）
