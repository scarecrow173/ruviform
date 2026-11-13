---
id: "TASK-034WYaQlaDnroaK9zoe9D3"
type: "task"
status: "in-progress"
title: "Issue #389 Parent Lease 移行タスク"
created: "2026-11-13"
updated: "2026-11-13"
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
  implements:
    - "docs/plans/issue-389-parent-lease-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-389-parent-lease-plan.md"
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
    - "docs/test-specs/issue-389-parent-lease.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 Parent Lease 移行タスク

正本: `docs/plans/issue-389-parent-lease-plan.md`。
設計: `docs/designs/issue-389-parent-lease-design.md`（D1–D6）。
検証仕様: `docs/test-specs/issue-389-parent-lease.md`（PL-01..06）。

## Work

`GuiParentToken` を公開 API から除去し、generation 付き不透明
`ParentLease { generation: u32, payload: u64 }` へ移行する。
plugin-core（boundary/plugin/wire/lib）→ plugin-host
（command/host/composite/editor_session/lib + tests）→
adapter/poc（vst3/clap/ui-gpui docs/poc-native-dock）の順で追従。
`ParentSlotLease`/`PluginEditorSessionId`/session 状態機械は不変。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-389-parent-lease-plan.md`
- `docs/designs/issue-389-parent-lease-design.md`
- `docs/specs/issue-389-parent-lease.md`
- `docs/test-specs/issue-389-parent-lease.md`

## Dependencies

- PR #503/#530/#544 で統合済みの Plugin Editor Session 状態機械
  （再構築しない）。
- PR #427 の `ParentSlotLease`/`LeaseRedeemer` 移行表。

## Affected Files

- `crates/ruviform-plugin-core/`、`crates/ruviform-plugin-host/`、
  `crates/ruviform-plugin-vst3/`、`crates/ruviform-plugin-clap/`、
  `crates/ruviform-poc-native-dock/`、
  `crates/ruviform-ui-gpui/src/gpui_driver/surface.rs`（doc のみ）、
  `evidence/rf389-session-hardening/`、本 Issue の文書チェーン。

## チェックリスト

- [x] T-1 plugin-core: `ParentLease` 新設・`GuiParentToken` 除去・
  plugin/wire/lib/README 追従・wire テスト更新。
- [x] T-2 plugin-host: command/host/composite/editor_session/lib +
  tests 追従。
- [x] T-3 adapter/poc: vst3 `parent.payload()`、clap、ui-gpui doc、
  poc redeemer×3・README/main doc。
- [x] T-4 contract test 更新（PL-01..06）+ evidence/impl 記録。
- [ ] T-5 `mise run fmt-check`/`check`/`lint`/`test`（変更クレート +
  下流 plugin-export/ofx/audio-plugin-bridge）+ review-protocol + PR。

## Done When

- [ ] `grep -r GuiParentToken crates/` が 0 件。
- [ ] `ParentLease` が generation+payload を保持し wire roundtrip
      可能。
- [ ] session-facing API は `ParentSlotLease` のまま不変。
- [ ] L1 GUI は Unsupported のまま。wire GUI verb の実装を主張しない。

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check/lint/test ruviform-plugin-core`
- [ ] `mise run check/lint/test ruviform-plugin-host`
- [ ] `mise run check/lint/test ruviform-plugin-vst3` /
      `ruviform-plugin-clap`（`--features host`）
- [ ] `mise run check/lint ruviform-poc-native-dock` /
      `ruviform-ui-gpui`
- [ ] `mise run check ruviform-plugin-export ruviform-plugin-ofx
      ruviform-audio-plugin-bridge`（下流非回帰）
