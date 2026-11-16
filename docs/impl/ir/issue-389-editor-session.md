---
id: "IMPL-034YvCERJ00wfoaoAYbNf2"
type: "impl"
status: "draft"
title: "Issue #389 real-session-terminate 実装記録"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "dir"
        path: "evidence/rf389"
      - type: "file"
        path: "docs/impl/exp/issue-389-editor-session-exp.jsonl"
      - type: "doc"
        path: "docs/specs/issue-389-editor-session.md"
      - type: "doc"
        path: "docs/designs/issue-389-editor-session.md"
      - type: "doc"
        path: "docs/plans/issue-389-editor-session.md"
      - type: "doc"
        path: "docs/tasks/issue-389-editor-session.md"
      - type: "doc"
        path: "docs/test-specs/issue-389-editor-session.md"
      - type: "doc"
        path: "docs/discovery/issue-389-editor-session.md"
      - type: "doc"
        path: "docs/impl/ir/issue-389-editor-session.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/bin/plugin-dock-console.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-389-editor-session.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-389-editor-session.md"
    - "docs/designs/issue-389-editor-session.md"
    - "docs/plans/issue-389-editor-session.md"
    - "docs/test-specs/issue-389-editor-session.md"
    - "docs/discovery/issue-389-editor-session.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/impl/exp/issue-389-editor-session-exp.jsonl"
    - "evidence/rf389/verification.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #389 real-session-terminate 実装記録

## Summary

`rf631.plugin-dock-real` の real-window カタログを 12 行から 13 行へ
拡充し、`real-session-terminate` case を追加した。test_editor の実
`IPlugView` が real container HWND lease 配下に attach した状態で
`HostCommand::Deactivate` + `HostCommand::Destroy` を実行し、registry
`PluginEditorSessions::notify_terminated(HostLost)` が session を
`Closed`+`concluded` に収束させ、event 列がちょうど
`[Attached{Docked}, Terminated{HostLost}]`（2 回目 notify は no-op）
であること、instance drop 由来の child HWND teardown
（`IsWindow(child)==0`）、および host 存続（`Scan` 成功 + 破棄済み
handle の `InvalidHandle`）を OS 観測で検証する。実走 lane で
`pass` / `environment_kind=real-window` を記録した。

## Context

`docs/tasks/issue-389-editor-session.md`（spec → design → plan →
task → test-spec チェーン `issue-389-editor-session`）に基づく
Issue `scarecrow173/ruviform#389`（tracker、auto-close しない）の
real-window session lifecycle L2 slice。headless lane の
`host-terminate`/`session-lifecycle` 先例（fake-slot/real-ABI 経路）
と同じ session API を、real-window lane の実 container lease 経路で
検証する。DSP/plugin-host lifecycle と editor-session lifecycle を
混同しないことが本 case の核心。

## Implementation

### Changes

- `src/real_catalog.rs`: `REAL_CATALOG` に
  `real-session-terminate`（`required: &[ScenarioCapability::
  NativePluginDock]`）を `real-editor-redock` の直後に追加（12→13
  行）。module doc の行数記述を追従。
- `src/real_dock.rs`:
  - module doc の case 表に `real-session-terminate` 行を追加、
    "twelve cases" → "thirteen cases"。
  - `real_entries()` を `[ScenarioEntry<RealDockContext>; 13]` 化し、
    catalog と同位置に新 entry を追加（`editor_attach_extras` と
    同形の spec extras）。
  - `blocked_reports`・`blocked_probe_violations`・`mod tests` の
    配列長・assert を 12→13 に追従。
  - `editor_attach_body` の verdict 写像を共有
    `editor_child_body` に一般化（`observe_open` が
    `ChildVerification` を返す）。case 識別文字列は
    `ChildCaseIdentity` struct に集約（clippy
    `too_many_arguments` 対応 — 9→7 引数）。
  - 新規 `real-session-terminate` 実装:
    `destroy_editor_instance`（`Deactivate`→`Destroy`、各 command
    を独立 step として記録、Deactivate 失敗でも Destroy を試行）、
    `terminate_editor_session`（`notify_terminated(HostLost)` →
    `session_closed_concluded` → drain event 列がちょうど
    `[Attached{Docked}, Terminated{HostLost}]` → 2 回目 notify で
    追加 event なし = exactly-once）、`check_host_survives`
    （`Scan`→`Scanned` + stale handle→`InvalidHandle`）、
    `observe_session_terminate_lifecycle`（attach 中 child HWND
    記録 → host loss → registry termination → `IsWindow==0` →
    host 存続）、`session_terminate_body`/
    `real_session_terminate_case`。
- `tests/console.rs`: `list_include_real_appends_real_lane_rows` を
  20→21 行・期待 case 配列に `real-session-terminate` を追加。
- `src/bin/plugin-dock-console.rs`・`README.md`: "twelve"→
  "thirteen"、verdict 表に新行追加。
- `evidence/rf389/`: lane JSONL 4 本（headless・blocked-probe・
  real・console）+ `--list --include-real` 投影 + `verification.md`。
  `evidence/rf631/` の lane 固定 `--out` 生成物は `git restore` で
  復元し commit に含めない。

### Decisions During Implementation

- verdict 写像は新規ロジックを足さず `editor_child_body` に集約 —
  refused open→`Unsupported`（`attach_attempt_failure` の既存規律）、
  観測 verified→`Pass`、platform 非対応観測→`Unsupported`、契約
  不一致→`Fail` を `real-editor-attach` と同一経路で共有。
- exactly-once の断言は 2 段構成: session event 列がちょうど
  `[Attached{Docked}, Terminated{HostLost}]` であること + 2 回目
  `notify_terminated` 後の drain が空であること（registry が
  instance binding を除去済みのため no-op — headless
  `host-terminate` 先例と同じ oracle）。
- host 存続は第 2 instance を立てず `Scan` 成功 + 破棄済み handle の
  `InvalidHandle` で観測 — host command surface が生きていることと
  instance 破棄の両方を 1 case 内で証明。
- graceful `session.close` は呼ばない — terminal event が
  `Terminated` であることが case の主張。`attempt_real_attach`
  teardown の 2 回目 `Destroy` は `InvalidHandle` で無害（`let _ =`
  既存規律）。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-389-editor-session-exp.jsonl` — 調査・設計
  判断・検証結果の時系列記録。

### Rejected

- なし。

## Validation

`evidence/rf389/verification.md` に全記録。要約:

- `mise run check/lint/test ruviform-plugin-dock-test`（default・
  `real-dock`・`lab-gui` 各 feature）: 全て exit 0。
- `mise run test ruviform-plugin-host --test editor_session
  --test editor_session_driver`: exit 0（session 契約非回帰）。
- `mise run test-ui-plugin-dock`: exit 0（headless 8 case verdict
  不変）。
- `mise run test-ui-plugin-dock-real-headless`: exit 0 — 13 行全て
  `not_run`+`environment_blocked`、violation 0。
- `mise run test-ui-plugin-dock-real`: exit 1 — `real-dcomp-pixel`
  の既知 `fail`（正直な regression 信号、変更なし）。新規
  `real-session-terminate` は `pass` / `environment_kind=
  real-window`。
- `mise run test-ui-plugin-dock-console` /
  `check-ui-driver-boundary` / `prek run --all-files`: 全て exit 0。

## Risks

- `real-dcomp-pixel` の既知 `fail` は `scarecrow173/ruviform#532`
  決裁待ちのまま — lane exit 1 は設計どおりであり本 slice で
  緩和しない。
- `HostCommand::Deactivate`/`Destroy` の legal-state 規則は
  plugin-host 側の契約に依存 — 将来 adapter verb が増えても本 case
  の oracle（session event・HWND・host 応答）は不変のはずだが、
  host lifecycle 仕様変更時は再検証が必要。

## Follow-ups

- Issue `scarecrow173/ruviform#389` 残受入: `real-session-move`・
  `real-session-degraded`・crash/hang/recovery case・実 Window RT
  非干渉・adapter verb（`InProcessEditorOps::reattach`/`set_size`/
  `set_visibility`）実装・consumer gates — 後続 slice。
- docs index（`docs/**/README.md`）は生成物 — Coordinator が統合時に
  再生成。
