---
id: "TASK-034UiUpK8eTff8QiCEtGrP"
type: "task"
status: "done"
title: "Issue #480 — tab_focus op 駆動解放の実装と恒久テスト"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-480-tab-focus-release.md"
    - "https://github.com/scarecrow173/ruviform/issues/480"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-480-tab-focus-release.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-480-tab-focus-release.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-480-tab-focus-release.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-480-tab-focus-release.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-480-tab-focus-release.md"
  verified-by:
    - "docs/test-specs/issue-480-tab-focus-release.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #480 — tab_focus op 駆動解放の実装と恒久テスト

## Work

`RvfView::flush_events`（`crates/ruviform-ui-gpui/src/render.rs`）に
`ViewEvent::Op(ProjectionOp::Removed { key } | ProjectionOp::Unmounted { key })`
を処理する match arm を追加し、`self.tab_focus.remove(&key)` で strip の
FocusHandle を解放する。`ProjectionOp` を `ruviform_ui::view` の import
に追加し、`tab_focus` フィールドの doc comment を実態に整合させる。
gpui-test に 3 つの恒久テスト（Removed・kind-swap・Unmounted、
sibling 保持、`Updated` 保持）を追加する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-480-tab-focus-release.md`
- `docs/specs/issue-480-tab-focus-release.md`
- `docs/designs/issue-480-tab-focus-release.md`

## Dependencies

- なし（`ProjectionOp`/`ViewEvent` は既存契約、変更不要）。

## Affected Files

- `crates/ruviform-ui-gpui/src/render.rs` — `flush_events`、import、
  `tab_focus` doc comment、`mod tests`。
- `docs/specs|designs|plans|test-specs|tasks/issue-480-tab-focus-release.md`
  と各 `README.md` index。

## Done When

- [x] test-spec 全ケースの恒久テストが RED→GREEN で検証される
      （RED: 3 fail / GREEN: 3 pass、補助証拠 `cargo test --features
      gpui-test -- focus_handle`）。
- [x] `selection: Some → None` の `Updated` が handle を保持することが
      design・test-spec・テストで固定されている。
- [x] `control_focus`（#479 FU-c）への展開可能性が design の
      Deferred Design Concerns に記録されている。
- [x] 全 lane green（`mise run test-ui-gpui`・`lint-ui-gpui`・
      `check-ui-driver-boundary`・`check`・`lint`・`fmt-check`。
      `origin/main` 取込後に実施、全て exit 0。`mise run test`
      workspace 全体も green）。

## Verification

- [x] `cargo test -p ruviform-ui-gpui --features gpui-test -- focus_handle`
      — RED→GREEN 確認（補助証拠; canonical = `mise run test-ui-gpui`）。
- [x] `mise run test-ui-gpui` / `lint-ui-gpui` /
      `check-ui-driver-boundary` / `check` / `lint` / `fmt-check`
      （`origin/main` 取込後、全て exit 0）。
- [x] review-protocol レビューで blocker なし。
