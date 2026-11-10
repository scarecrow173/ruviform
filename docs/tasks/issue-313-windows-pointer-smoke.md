---
id: "TASK-034UkPTR1W1N5M0K3P05"
type: "task"
status: "in-progress"
title: "Issue #313 — Windows 実 OS SendInput ポインタ smoke 実装タスク"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-313-windows-pointer-smoke.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-313-windows-pointer-smoke.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-313-windows-pointer-smoke.md"
    - "docs/designs/issue-313-windows-pointer-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-313-windows-pointer-smoke.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-313-windows-pointer-smoke.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-313-windows-pointer-smoke.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — Windows 実 OS SendInput ポインタ smoke 実装タスク

計画 `PLAN-034UkPTR1W1N5M0K3P04` のスライスを実施順のタスクに
分けたもの。

## タスク

- [x] T1 `os_input.rs` ポインタ API（`cursor_position`/`move_to`/
  `mouse_down`/`mouse_up`、virtual-desktop 正規化、部分注入を
  `Host` 化）+ 非 Windows `Unsupported` stub。
- [x] T2 `capture.rs` に `OsWindowGeometry` + `window_geometry` +
  Windows `client_geometry`（非 Windows `Unsupported`）。
- [x] T3 `render.rs`/`lib.rs` に paint 記録 bounds store の
  driver 共有化 + `component_bounds` query（`ViewChannels`/
  `ComponentBounds` で lint 適合）。
- [x] T4 `real_os_pointer_smoke.rs` example + `Cargo.toml`
  `[[example]]`（照準・drag/click 注入・read-back・カーソル
  復元・verdict JSON）。
- [x] T5 `mise.toml` manual lane
  （`example-ui-gpui-real-os-pointer-smoke`）。
- [x] T6 build & Windows 実機 smoke 実行 → verdict JSON 採取
  （verdict=pass、exit 0）。
- [x] T7 `evidence/rf313-os-capability-matrix/` report/results
  更新（pointer 行を実測 Pass へ、macOS/Linux は NotRun 継続）。
- [x] T8 `test-ui-gpui`/`check-ui-gpui`/`lint-ui-gpui`/
  `fmt-check` 検証。
- [x] T9 docs チェーン + index 再生成、PR 作成（`Refs #313`、
  Issue は Open 継続）。

## Upstream

- `docs/plans/issue-313-windows-pointer-smoke.md`
- `docs/designs/issue-313-windows-pointer-smoke.md`
- `docs/specs/issue-313-windows-pointer-smoke.md`

## Dependencies

- キーボードスライスの `os_input`/`real_os_smoke` 骨格、
  #243 実窓証拠、#314 `capture_window`/`client_screen_rect`
  （全て main 済み）。

## Affected Files

- `crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs`
- `crates/ruviform-ui-gpui/src/gpui_driver/capture.rs`
- `crates/ruviform-ui-gpui/src/render.rs`
- `crates/ruviform-ui-gpui/src/lib.rs`
- `crates/ruviform-ui-gpui/examples/real_os_pointer_smoke.rs`（新規）
- `crates/ruviform-ui-gpui/Cargo.toml`
- `mise.toml`
- `evidence/rf313-os-capability-matrix/`（verdict 追加 + 更新）
- `docs/{specs,designs,plans,tasks,test-specs,impl}` の本
  スライス文書

## Done When

- [x] Windows 実機で `mise run example-ui-gpui-real-os-pointer-smoke`
  が verdict JSON（環境 metadata・geometry・bounds・read-back
  込み）を出力し exit code が verdict を反映する。
- [x] evidence の pointer 行が実測 Pass/Fail/Unsupported/NotRun
  と根拠を持ち、skipped/未実装を Pass に偽装していない。
- [x] `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`/`fmt-check`
  が Pass。

## Verification

- `mise run check-ui-gpui` `lint-ui-gpui` `test-ui-gpui`
  `fmt-check`、および `mise run example-ui-gpui-real-os-pointer-smoke`
  （Windows 対話 session、manual lane）。
