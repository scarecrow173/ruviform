---
id: "TSPEC-Vqv91J1zDZ6Qvi57khd2TT"
type: "test-spec"
status: "approved"
title: "Issue #389 Plugin Editor Session headless fixture テスト仕様（DSP/RT 非干渉 + contract edge + focus 境界）"
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
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-session-hardening.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
    - "docs/test-specs/issue-389-session-hardening.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-session-hardening.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-session-hardening.md"
    - "docs/specs/issue-389-plugin-editor-session.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 Plugin Editor Session headless fixture テスト仕様

## Purpose

driver-neutral Plugin Editor Session contract（型付き
`PluginEditorSession`/`PluginEditorSessions`/`EditorSessionOps`・
lifecycle state machine・generation/ownership・RT 非干渉）を、
**実 plugin・実 OS・GPU・GPUI を使わない headless fixture** で
固定する。本 slice が追加する EH-xx は既存 ES-xx（native-dock
contract）・SH-xx（session hardening）が pin していない残存 gap —
`set_visibility` no-op の event 抑止と、**実 `InProcessHost` の
`process` 経路を跨いだ DSP/RT 非干渉** — のみを対象とする。
contract 自体は main 上で実装済みであり、本仕様は新規契約を
定義しない。ここで保証するのは状態機械と語彙の契約であり、実
Native Dock / pixel compositing / 実 plugin editor の PASS では
ない。

## Feature

Plugin Editor Session の headless fixture — contract edge
（no-op event 抑止）と DSP/RT 非干渉（session ops が DSP
instance の lifecycle・`process` 結果に干渉しない）。

## Rules

- **EH-R1 (no-op visibility)**: `Attached`/`Hidden` と同じ
  visibility を要求する `set_visibility` は ops 呼出を行うが
  state 遷移も event 発火もしない — event consumers は実 state
  変更のみを数える。
- **EH-R2 (graceful teardown non-interference)**: `Processing`
  中の instance に対し session `open` → 非 lifecycle op 拒否
  （`Unsupported`・非致命）→ `close` を実行しても、
  `lifecycle_state` は `Processing` のまま、`process` は
  `ProcessStatus::Continue` を返し続ける。
- **EH-R3 (abnormal termination non-interference)**:
  `notify_terminated`（window loss/host crash/child exit の
  語彙）で session が `Terminated` → `Closed` に収束しても、
  DSP instance の lifecycle・`process` は不変。
- **EH-R4 (refused open non-interference)**: plugin 既定
  `Unsupported` の `open_gui` による refused open → `Failed`
  session → adapter 非呼出し `close` でも、DSP 経路は不変。
- **EH-R5 (focus boundary)**: keyboard focus の要求・
  ルーティングは session port の動詞ではなく #388
  `NativeSurfaceSlot::request_focus` / `WindowDriver::focus`
  （presentation/slot 層）が所有する。dock focus の session 側
  意味論は `Attached ↔ Hidden`（`set_visibility`）が担う —
  session contract に focus verb を追加しない（approved design
  D4・ES/NSC 棲み分けが根拠）。

## Examples

EH-xx case は `crates/ruviform-plugin-host/tests/editor_session.rs`
の test 名に対応する。

| ID | Rule | 手順 | 期待 |
| --- | --- | --- | --- |
| EH-01 | EH-R1 | open → drain → `set_visibility(true)`（Attached 中）→ `set_visibility(false)` → `set_visibility(false)`（Hidden 中） | events は `[Hidden]` のみ — no-op 2 回は event を発火しない。最終 state `Hidden` |
| EH-02 | EH-R2 | gui_supported=true の instance を `Processing` へ → session open → `set_visibility`（L0 は `Unsupported` 拒否・非致命）→ close → 各段で `process_block` | 全段で `lifecycle_state == Processing`・`process().status == Continue`。session は `Closed` 収束 |
| EH-03 | EH-R3 | `Processing` 中に session open → `sessions.notify_terminated(HostLost)` → `process_block` | session `Closed` + `Terminated{HostLost}` 1 回。`lifecycle_state == Processing`・`process` は `Continue` |
| EH-04 | EH-R4 | gui_supported=false の instance を `Processing` へ → session open（`Err(Ops(Unsupported))`）→ close（adapter 非呼出し）→ `process_block` | session `Closed` 収束。`lifecycle_state == Processing`・`process` は `Continue` |
| EH-05 | EH-R5 | （静的）`EditorSessionOps`/`PluginEditorSession` の公開面に focus 動詞が存在しないことを design D4・本表で確認 | session port の ops は capabilities/open/close/set_size/set_visibility/reattach の 6 動詞のみ。focus は slot 層契約（NSC-05 `request_focus`）の責務 |

## Guarantees

- session の全 lifecycle 経路（graceful close・abnormal
  termination・refused open）で、対象 instance の
  `lifecycle_state` と `process` 結果が不変であることが実
  `InProcessHost` 経路で検証される。
- `set_visibility` の no-op は event を発火せず、event 列が
  実 state 変更と 1:1 に対応する。
- 既存 contract test（ES-01..24・SH-01..18・L0/L1 系 28+ 件）は
  挙動不変で green を維持する（fixture 追加のみ・API 不変）。

## Non-goals

- 実 plugin editor・実 OS window・GPUI driver・GPU/実機での
  非干渉検証（`NotRun` — 実機受入は別 slice/Issue）。
- L1 out-of-process での session ops（`L1EditorOps` は全 op
  `Unsupported` の defined refusal — 変更なし）。
- focus verb の session port への追加（EH-R5 の境界記録のみ）。
- RT thread からの session 呼出のコンパイル時禁止（型設計上
  不可能 — port が RT コンテキストを受け取らない。実 RT lane
  実測は `test-rt` の既存 `rt_*` suite が担う）。
- GUI hang/フリーズが DAW/RT を巻き込まない実機検証
  （`NotRun` — Issue #389 残 DoD）。

## Risk

- `process` 駆動の lifecycle 前提（`Processing` 到達）が将来の
  host 変更で壊れる退行 → EH-02..04 が `lifecycle_state` を
  明示 assert するため検出可能。
- fixture green を実 Native Dock PASS と誤読 → test-spec/IR/PR
  で NotRun・非 PASS を明記し、fake/L0 stub の結果を実機受入と
  混同しない。
