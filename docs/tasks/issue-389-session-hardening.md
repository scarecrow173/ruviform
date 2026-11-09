---
id: "TASK-034UnqS8L3S2tF4h6u8LJd"
type: "task"
status: "in-progress"
title: "Issue #389 Plugin Editor Session hardening タスク（timeout/degrade/GraphicsLost/DComp 記録）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-389-session-hardening.md"
    - "docs/test-specs/issue-389-session-hardening.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-session-hardening.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-389-session-hardening.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-session-hardening.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 Plugin Editor Session hardening タスク

正本: `docs/plans/implement-issue-389-session-hardening.md`。
設計: `docs/designs/issue-389-session-hardening.md`（D1–D5）。
検証仕様: `docs/test-specs/issue-389-session-hardening.md`（SH-01..18）。

## チェックリスト

- [x] T-1 `editor_session.rs` — D1/D2: `SessionClock`・
  `MonotonicSessionClock`・`SessionDeadlinePolicy`・
  `DEFAULT_REQUEST_TIMEOUT`・deadline arm（`next_request`）・
  `tick`/`request_deadline`/`set_deadline_policy`/`tick_all`。
- [x] T-2 `editor_session.rs` — D3: `OpenOutcome`・`Degraded` event・
  `degrade_pending`・`open_or_degrade`・`begin_degrade`・
  `complete_attach` の Degraded 分岐。
- [x] T-3 `editor_session.rs` — D4: `SessionFailCause::GraphicsLost`・
  `EditorOpError::GraphicsLost` + `fail_cause`/`Display` 写像。
- [x] T-4 `lib.rs` export 追加 + `README.md` 追記。
- [x] T-5 `tests/editor_session.rs` — FakeOps mode 別 result・
  FakeClock・SH-01..18 実装。
- [x] T-6 design D5 capability matrix・
  `evidence/rf389-session-hardening/report.md`・impl record。
- [ ] T-7 `mise run fmt-check`/`check`/`lint`/`test` green +
  review-protocol + PR。

## 完了条件

- test-spec SH-01..18 が green、既存 contract test 非回帰。
- capability matrix が DComp `Unsupported`（実測）・pixel
  compositing `NotRun` を正直に記録。
- API は additive のみ — `ruviform-poc-native-dock`・
  `ruviform-plugin-core`・`ruviform-ui-gpui`・workspace manifest
  に差分なし。
