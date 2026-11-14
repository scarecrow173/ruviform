---
id: "IMPL-034XGt17uREc5neDsnIhoO"
type: "impl"
status: "completed"
title: "Issue #389 dock-test session-lifecycle 実装記録"
created: "2026-11-14"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-389-session-lifecycle.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-389-session-lifecycle.md"
      - type: "doc"
        path: "docs/designs/issue-389-session-lifecycle.md"
      - type: "doc"
        path: "docs/plans/issue-389-session-lifecycle.md"
      - type: "doc"
        path: "docs/tasks/issue-389-session-lifecycle.md"
      - type: "doc"
        path: "docs/test-specs/issue-389-session-lifecycle.md"
      - type: "doc"
        path: "docs/impl/ir/issue-389-session-lifecycle.md"
      - type: "doc"
        path: "docs/impl/exp/issue-389-session-lifecycle.md"
      - type: "file"
        path: "docs/impl/exp/issue-389-session-lifecycle-exp.jsonl"
      - type: "dir"
        path: "evidence/rf389-dock"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/plugin_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "file"
        path: "docs/designs/issue-389-plugin-editor-session.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-389-session-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-389-session-lifecycle.md"
    - "docs/plans/issue-389-session-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-389-session-lifecycle.md"
    rejected: []
---

# Issue #389 dock-test session-lifecycle 実装記録

## Summary

`ruviform-plugin-dock-test` の `rf631.plugin-dock` scenario に session-lifecycle 系 3 case（`session-lifecycle`・`host-terminate`・`open-or-degrade`）を追加し、実 `test_gain` VST3 バイナリ経路で `PluginEditorSessions` の begin/busy/refused-open/close-converge/end/re-begin・`notify_terminated` exactly-once・degrade 非発行を観測可能にした。あわせて `docs/designs/issue-389-plugin-editor-session.md` の stale verification-status（R5 matrix・D8・`GuiParentToken`/`WIRE_VERSION`）を現行コード事実へ reconcile した。

## Context

`docs/tasks/issue-389-session-lifecycle.md`（← plan/spec/design/test-spec）に従う Issue `scarecrow173/ruviform#389` tracker の dock-test slice。#631 で作った harness の fail-closed・正直 verdict 規律を維持し、session API（plugin-host、読み取りのみ）の実バイナリ消費証拠を追加する。Issue #389 は OPEN のまま（L2/L3・`test_editor`・consumer gates は後続 slice）。

## Implementation

### Changes

- `crates/ruviform-plugin-dock-test/src/scenario.rs`: `SessionCaseState`・`editor_ops`/`rejecting_editor_ops`・`CountingEditorOps`・`teardown_editor_case`・`session_lifecycle_steps`/`host_terminate_steps`/`open_or_degrade_steps` を追加し `run_all` を 5→8 case に。`attempt_editor_attach` の redeemer closure を共有 helper へ集約（挙動不変）。module doc case 表を追従。
- `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs`: case 数断言 5→8、fail-closed NotRun 一覧と fixture dependent 一覧に新 case を追加、新 case の `pass`+`real-abi-test-plugin` 断言を追加。
- `crates/ruviform-plugin-dock-test/README.md`: verdict 表に新 case 行を追加。
- `docs/designs/issue-389-plugin-editor-session.md`: R5 matrix の VST3/CLAP L0 行・D8 移行表・D5/R2/R4/R6 の `GuiParentToken`/`WIRE_VERSION` 記述を現行事実へ reconcile（改訂履歴を残す形式）。
- `evidence/rf389-dock/`: lane 実走 JSONL・verification report を記録。

### Decisions During Implementation

- `CountingEditorOps`（`EditorSessionOps` decorator）で `open`/`close` dispatch 回数を観測 — refused open 後の `close` が adapter `CloseGui` を呼ばないことを state 収束だけでなく直接証拠化した。
- `host-terminate` は born-`Closed` session では `Terminated` event が出ないため、refused open で `Failed{Unsupported}` の live session を使う honest 経路を採用。
- `open-or-degrade` は test_gain が Docked/Floating 両方を拒否するため `Err(Ops(Unsupported))`+event 0 件の double-refusal honesty を観測対象とした（`OpenOutcome::Degraded` 成功経路は fixture 未整備で対象外）。
- teardown は `notify_terminated`→`end_session` 全件→slot release→`Destroy` の best-effort 単一経路に集約し、assertion 失敗時も必ず通る構成にした。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-389-session-lifecycle.md` —
  実装過程の観察記録（`too_many_lines` 対応、illegal lifecycle
  transition 修正、`Terminated` event 条件、degrade honesty）。
  イベントログ:
  `docs/impl/exp/issue-389-session-lifecycle-exp.jsonl`。

### Rejected

- なし。

## Validation

- `mise run fmt-check`: pass
- `mise run check|lint|test ruviform-plugin-dock-test`: pass
  （integration 2 test — missing-artifact fail-closed・fixture
  verdict 8 case）
- `mise run build ruviform-plugin-test-vst3`: pass（test_gain
  cdylib 生成）
- `mise run test-ui-plugin-dock`: pass — 8 case 実走
  （`view-mount=pass`(fake), `artifact=pass`, `host-lifecycle=pass`,
  `editor-attach=unsupported`, `session-lifecycle=pass`,
  `host-terminate=pass`, `open-or-degrade=pass`,
  `dock-pixels=not_run`。新規 3 case は `real-abi-test-plugin`）
- `mise run test ruviform-plugin-host --test editor_session
  --test editor_session_driver`: pass（session registry/状態機械
  契約の非回帰）
- `mise run check-ui-driver-boundary`: pass
- `mise exec -- prek run --all-files`: pass
- 詳細は `evidence/rf389-dock/verification.md`。

## Risks

- session 3 case の `Pass` は session registry 契約
  （`SessionBusy`・refused-open converge・`Terminated{HostLost}`
  exactly-once・degrade 非発火）の実走証拠であり、実 editor
  attach・Native Dock・画素の証拠ではない（test_gain `createView`
  null → open は honest `Unsupported`）。
- `OpenOutcome::Degraded` の成功経路は editor surface を持つ
  fixture がなく未観測 — fixture 拡張時に `open-or-degrade` case
  が正直に `Degraded` を記録する構成。

## Follow-ups

- `test_editor` plugin class（`createView` が実 surface を返す実
  物 — `scarecrow173/ruviform#630` 残存 scope）による実 Dock
  PASS・実画素検証（`dock-pixels` は `NotRun` 維持 —
  `scarecrow173/ruviform#532`）。
- 実 Window/実 OS GUI L2/L3（`scarecrow173/ruviform#388`・
  `scarecrow173/ruviform#631` 後続 slice）と `open_or_degrade`
  の実 editor 向け degrade 経路の実 attach 検証。
- Issue #389 は OPEN のまま（tracker — L2/L3・consumer gates は
  後続 slice）。
