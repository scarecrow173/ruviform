---
id: "TSPEC-rf389-real-session-lifecycle-01"
type: "test-spec"
status: "approved"
title: "Issue #389 real session lifecycle remainder テスト仕様"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "docs/specs/issue-389-real-session-lifecycle.md"
    - "docs/designs/issue-389-real-session-lifecycle.md"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-389-real-session-lifecycle.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-real-session-lifecycle.md"
    - "docs/designs/issue-389-real-session-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-real-session-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-real-session-lifecycle.md"
    - "docs/designs/issue-389-real-session-lifecycle.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-389-real-session-lifecycle.md"
  verified-by: []
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 real session lifecycle remainder テスト仕様

## Purpose

session lifecycle 残件 6 case が「degrade/recover/timeout の
実経路」「defined refusal の `Unsupported`」「in-process 非
生存 fault の正直 `Unsupported`」を real OS window 上で正直に
判定することを保証する。

## Feature

real-window lane（`rf631.plugin-dock-real`）の session lifecycle
残件: `open_or_degrade`・`recover`・deadline/`tick_all`・
`set_visibility` refusal・crash/hang 境界を catalog 21 行目以降
として判定する。

## Rules

- R1: catalog 一致 — `REAL_CATALOG` と `real_entries()` は
  case/scenario/required が pairwise 一致し、両者 21 行。新規
  6 行は `real-session-terminate` の次。新規 6 行の `required` は
  すべて `&[NativePluginDock]`（crash/hang も同じ — capability
  preflight 通過後に body が実 session 内で `Unsupported` を記録）。
- R2: open 駆動 seam — `attempt_real_attach` は case 供給の
  `drive_open` で session op を駆動し、既存 case は
  `drive_open_docked` で step row を逐語維持する。teardown
  （`end_session`→slot/lease release→`Destroy`）は不変。
- R3: degraded — `open_or_degrade(Floating, Docked)` が
  `OpenOutcome::Degraded{mode: Docked}` + events ちょうど
  `[Degraded{Docked}]`（attach 相当 `Attached` を含まない）+
  child HWND 契約 + close 収束 `[Degraded{Docked}, Closed]` で
  のみ `Pass`。`OpenOutcome::Attached` や event 不一致は `Fail`。
- R4: recovery — `open(Floating)` が `Ops(Unsupported)` refusal
  で `Failed{Unsupported}`（`editor_live=false`）に落ちることを
  step で確認後、`recover(Docked)` が `Ok` →
  `Recovered{Docked}` + child contract + events ちょうど
  `[Failed{Unsupported}, Recovered{Docked}, Closed]` でのみ
  `Pass`。live editor 上の `Failed` からの recover は
  `open_gui` の `PluginFailed` で失敗する契約のため起点を
  refused open に限定する。
- R5: timeout — `SessionDeadlinePolicy{timeout: ZERO,
  MonotonicSessionClock}` → `begin_detach` →
  `request_deadline()==Some` → `tick_all()==1` →
  `Failed{Timeout}` state+event → close → child teardown +
  events ちょうど `[Attached{Docked}, Failed{Timeout}, Closed]`
  でのみ `Pass`。timeout の原因は deadline/tick API であり
  synthetic verdict ではない。
- R6: visibility — `set_visibility(false)` が defined
  `Ops(Unsupported)` refusal・session `Attached` 維持・
  `Hidden`/`Shown` event 0 件で `Unsupported` verdict。受理は
  `NotRun`、その他失敗・event 混入は `Fail`。
- R7: crash/hang — body は GUI/session に触れず `Unsupported`
  を返し、detail に in-process lane が `test_fault` の
  abort/exit/hang（host process kill/wedge）を生存不能、
  `L1EditorOps` 全 refusal、out-of-process host 未実装を明記。
- R8: fail-closed — headless/非対応環境では全行
  `NotRun`+`observed.environment_blocked`・
  `environment_kind=real-window` 不請求。`EnvironmentKind::
  RealWindow` は実 session 内のみ emit。
- R9: 非回帰 — 既存 real 15 case・headless lane 8 case・console
  `--list` projection の verdict/行数を本 slice で動かさない
  （console は 23→29 行の意図的追従を除く）。

## Examples

- E1（degraded 実走）: Given `open_or_degrade(Floating→Docked)`
  が fallback attach に成功、When drain events =
  `[Degraded{Docked}]` で child HWND が container 配下に実在、
  Then close で `[Degraded{Docked}, Closed]`・case `Pass`・
  `environment_kind=real-window`。degrade は Native Dock PASS
  に計上されない。
- E2（recovery 実走）: Given `open(Floating)` refused →
  `Failed{Unsupported}`、When `recover(Docked)` が実 `OpenGui`
  に成功、Then `Recovered{Docked}` + child contract + events
  `[Failed{Unsupported}, Recovered{Docked}, Closed]`・case
  `Pass`。
- E3（timeout 実走）: Given docked attach 済み、When ZERO
  deadline policy + `begin_detach` + `tick_all`、Then
  `Failed{Timeout}` + events `[Attached{Docked},
  Failed{Timeout}, Closed]`・case `Pass`。
- E4（visibility refusal）: Given docked attach 済み、When
  `set_visibility(false)`、Then `Ops(Unsupported)` refusal・
  `Attached` 維持・event 不変・case `Unsupported`。
- E5（crash/hang 境界）: Given in-process lane、When crash/hang
  case が走る、Then GUI/session 非接触で `Unsupported`・
  detail に非生存根拠を明記。
- E6（headless blocked）: Given 実 Window を持たない host、
  When `--blocked-probe`、Then 全 21 行が `NotRun`+
  `environment_blocked` で violation 0・exit 0。

## Guarantees

- `Degraded`/`Recovered`/`Failed{Timeout}` event は実 session
  遷移でのみ emit され、synthetic に作られない。
- degrade は `Attached` ではなく `Degraded` event であり
  Native Dock PASS に計上されない。
- refused open 起点の recovery のみが `Recovered` に到達し、
  live editor 上の `recover` 失敗（`PluginFailed`）を誤って
  Pass にしない。
- timeout は `SessionDeadlinePolicy`+`tick_all` の実駆動で
  発生し、任意 verdict ではない。
- crash/hang の `Unsupported` は in-process 非生存の正直な
  境界記録であり、fake crash/hang Pass を生まない。
- verdict は実走した step のみ `Pass`、未実行を `Pass` にしない。

## Non-goals

- `real-session-move`（reattach refusal が lease 非依存で
  redock と観測同一 — design で不採用確定）、adapter verb
  実装、crash/hang の out-of-process 化、実 Window RT 非干渉、
  `real-dcomp-pixel` 画素、headless lane catalog 変更、Issue
  close。

## Risk

- degrade/recover/timeout のいずれかの保証が落ちると、session
  lifecycle の異常・回復・期限経路が「見かけ上動いたが実は
  誤 event/誤 state」を検出できず、Issue #389 の real-window
  受入を誤って前進させる。
- verdict を実走なしに `Pass` にすると POLICIES.md §2.1 の
  fail-closed 規範に違反する。
