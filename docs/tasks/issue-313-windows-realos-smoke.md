---
id: "TASK-034UjNKCoOCnJfLYQhT8Fx"
type: "task"
status: "in-progress"
title: "Issue #313 — Windows 実 OS SendInput キーボード smoke 実装タスク"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-313-windows-realos-smoke.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-313-windows-realos-smoke.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-313-windows-realos-smoke.md"
    - "docs/designs/issue-313-windows-realos-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-313-windows-realos-smoke.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-313-windows-realos-smoke.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-313-windows-realos-smoke.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — Windows 実 OS SendInput キーボード smoke 実装タスク

計画 `PLAN-034UjMMA7vTOdlkqHFOrca` のスライスを実施順のタスクに分けたもの。

## タスク

- [x] T1 `os_input.rs` + lib.rs の `pub mod`/re-export（SendInput 注入）。
- [x] T2 `real_os_smoke.rs` example + `Cargo.toml` `[[example]]`。
- [x] T3 `mise.toml` manual lane（`example-ui-gpui-real-os-smoke`）。
- [x] T4 build & Windows 実機 smoke 実行 → verdict JSON 採取。
- [x] T5 `evidence/rf313-os-capability-matrix/` report/results 作成。
- [x] T6 `test-ui-gpui`/`check`/`lint`/`fmt-check`/manual lane 検証。
- [x] T7 `origin/main` 取り込み後の再検証。
- [x] T8 review-protocol レビュー（blocker なし）。
- [x] T9 PR 作成（`Refs #313`、Issue は Open 継続）。

## Upstream

- `docs/plans/issue-313-windows-realos-smoke.md`
- `docs/designs/issue-313-windows-realos-smoke.md`
- `docs/specs/issue-313-windows-realos-smoke.md`

## Dependencies

- GPUI Host・#479 キーボード規約・#451 Tab 巡回・#243 実窓証拠・
  #314 `capture_window`（全て main 済み）。

## Affected Files

- `crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs`（新規）
- `crates/ruviform-ui-gpui/src/lib.rs`（`pub mod` + re-export のみ）
- `crates/ruviform-ui-gpui/examples/real_os_smoke.rs`（新規）
- `crates/ruviform-ui-gpui/Cargo.toml`（`[[example]]` のみ）
- `mise.toml`（manual lane 1 件）
- `evidence/rf313-os-capability-matrix/`（新規）
- `docs/{specs,designs,plans,tasks,test-specs,impl}` の本 Issue 文書

## Done When

- [ ] Windows 実機で `mise run example-ui-gpui-real-os-smoke` が
  verdict JSON（環境 metadata 込み）を出力し exit code が verdict
  を反映する。
- [ ] evidence に Windows 行の実測 Pass/Fail・#243/#314 リンク・
  SendInput=OS automation injection の明記・NotRun/Unsupported の
  理由と owner を含む report.md + results.jsonl がある。
- [ ] `mise run test-ui-gpui`/`check`/`lint`/`fmt-check` が Pass。

## Verification

- `mise run check-ui-gpui` `lint-ui-gpui` `test-ui-gpui` `fmt-check`
  `check` `lint`、および `mise run example-ui-gpui-real-os-smoke`
  （Windows 対話 session、manual lane）。
