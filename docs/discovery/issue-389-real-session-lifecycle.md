---
id: "DISC-rf389-real-session-lifecycle-01"
type: "discovery"
status: "resolved"
title: "Issue #389 real session lifecycle remainder 調査記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "docs/specs/issue-389-real-session-lifecycle.md"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-389-real-session-lifecycle.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-389-real-session-lifecycle.md"
    - "docs/designs/issue-389-real-session-lifecycle.md"
    - "docs/plans/issue-389-real-session-lifecycle.md"
    - "docs/tasks/issue-389-real-session-lifecycle.md"
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  derived-by:
    - "docs/specs/issue-389-real-session-lifecycle.md"
  verifies: []
  verified-by: []
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 real session lifecycle remainder 調査記録

## Summary

Issue `scarecrow173/ruviform#389` の real-window session
lifecycle 残件について、`rf631.plugin-dock-real` lane に 6 行
（`real-session-degraded`/`real-session-recovery`/
`real-session-timeout`/`real-editor-visibility`/
`real-session-crash`/`real-session-hang`）を追加する slice の
調査記録。dispatch 探索（`rf389-explore.explore.json`）の
`DISPATCHABLE` 判定を、session API・host 契約・fixture fault
law の実コードで再確認した。確度: 高（全て実装で確認済みの
API 契約に基づく）。

## 調査結果

### session API の実駆動可能性（`editor_session.rs`）

- `open_or_degrade(preferred, fallback)`（L1317-1368）:
  preferred の `EditorOpError::Unsupported` のみ同一 request
  id で 1 回 fallback retry。fallback 成功で
  `Attached{fallback}` + `Degraded{fallback}` event +
  `OpenOutcome::Degraded`。`Degraded` は `Attached`/
  `Recovered`/`Reattached` のいずれでもなく Native Dock PASS
  に計上されない。
- `InProcessEditorOps::open`（L1614-1645）は `AttachMode::
  Docked` 以外を redeemer 呼出し**前**に `Unsupported` で
  拒否 — Floating refused は lease/plugin に触れない defined
  refusal。
- `recover(ops, lease, size, mode)`（L1267-1292）は `Failed`
  から `begin_recover` → `ops.open` → 成功で
  `Attached{mode}` + `Recovered{mode}` event。
- `Vst3Plugin::open_gui`（`plugin-vst3/src/host.rs` L3049-
  3102）は `self.view.is_some()` で `HostError::PluginFailed`
  — live editor 上の `open`/`recover` は成功しない。
  **recovery の `Failed` 起点は refused open
  （`editor_live=false`）に限る**ことが必須条件。
- `set_deadline_policy`/`request_deadline`/`tick`/
  `tick_all`（L1010-1072, L1526-1532）: `SessionDeadlinePolicy
  {timeout: ZERO}` は request mint 時に `clock.now()+0` を arm、
  次の `tick` で `now() >= deadline` が必ず成立し
  `Failed{Timeout}` + `Failed{Timeout}` event に収束。
  `begin_detach` は ops を呼ばない session-level 遷移のため、
  timeout 後も plugin editor は attach されたまま（close で
  解放）。
- `set_visibility`/`set_size`/`reattach` は
  `InProcessEditorOps` が無条件 `Unsupported`（L1660-1683）。
  `set_visibility` refusal は session 不変（`Attached` 維持・
  `Hidden`/`Shown` event なし）が contract（L1177-1203）。
- `L1EditorOps`（L1695-1752）は全 op を defined refusal。

### fixture fault law（`ruviform-plugin-test-vst3`）

`test_fault` descriptor の fault mode は abort/exit/hang/error
で、abort/exit/hang は fixture が host process 自体を
kill/wedge する process-destructive 動作。in-process lane
（`InProcessHost` が plugin cdylib を同一 process に load）
では生存不能 — crash/hang の実走は out-of-process plugin
host（未実装）が前提。`real-session-crash`/`real-session-
hang` は正直 `Unsupported` 境界行とする。

### `real-session-move` 採否（確定: 不採用）

`session.reattach` → `InProcessEditorOps::reattach` は lease
を一切検査せず無条件 `Unsupported`。第 2 slot/lease を
用意しても refusal landing・event 列
（`[Attached{Docked}, Detached, Failed{Unsupported}, Closed]`）
・verdict は `real-editor-redock` と完全に同一。adapter verb
実装で move 固有 oracle が定義されるまで catalog 行を増やす
distinct value がない — 21 行に確定。

### 実装 seam

`attempt_real_attach`（real_dock.rs L1766-1896）は
`open_session_docked`（L1726-1753）を固定で呼ぶ。新規 case
は `open_or_degrade`/`open(Floating)`+`recover`/通常 open +
observe 内 deadline 駆動を必要とするため、open 呼出しを case
供給の `drive_open` クロージャへ一般化する。戻り値は
`Option<Result<EditorCapabilities, EditorSessionError>>`
（`None` = session 消失 → `Setup`）で `RealAttachAttempt` の
写像を維持。既存 case は `drive_open_docked`（現行
`open_session_docked` + `session.open(Docked)` step row を
逐語再現）を渡し evidence 形状を不変に保つ。redeemer は
`open_session_docked` 内の `redeem_native_lease` 包みを
`real_editor_ops` factory として抽出し新 drive で再利用。

### verdict 写像の先例

- degrade/recovery/timeout: `observe_open` が
  `ChildVerification` を返し `editor_child_body` で
  verified→`Pass`（attach/terminate と同型）。event 期待値は
  case 別のため `check_redock_events`（L5548-5611）と同型の
  専用 check を書く。
- visibility: `EditorOpVerification{landing}` を返し
  `editor_op_body` で verified+Refused→`Unsupported`、
  verified+Accepted→`NotRun`（resize/redock と同型）。
- crash/hang: `run_scenario` body が `Unsupported` を返す
  （`real_window_dpi_case` L5290・`real_window_close_case`
  L5343 と同型）。

## 未解決・後続

- adapter verb（`set_size`/`set_visibility`/`reattach`/Floating）
  実装 — verb が実装された時点で visibility/redock 等の
  `Unsupported` verdict が `Pass`/`NotRun` へ遷移する。
- crash/hang の out-of-process plugin host — fixture fault law
  を生存できる lane ができた時点で実走 case 化。
- `real-dcomp-pixel` 画素 — `scarecrow173/ruviform#532` 決裁待ち。
- 実 Window RT 非干渉 oracle・consumer gates — 後続 slice。
