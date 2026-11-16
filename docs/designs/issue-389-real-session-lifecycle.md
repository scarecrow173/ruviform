---
id: "DESIGN-rf389-real-session-lifecycle-01"
type: "design"
status: "approved"
title: "Issue #389 real session lifecycle remainder 設計"
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
        path: "docs/designs/issue-389-real-session-lifecycle.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-real-session-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-389-real-session-lifecycle.md"
    - "docs/specs/issue-389-editor-session.md"
  blocks:
    - "docs/plans/issue-389-real-session-lifecycle.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-real-session-lifecycle.md"
  derived-by:
    - "docs/plans/issue-389-real-session-lifecycle.md"
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 real session lifecycle remainder 設計

## Context

`rf631.plugin-dock-real` は 15 行の catalog（`REAL_CATALOG` +
feature-gated `real_entries()` + `--blocked-probe` fail-closed
lane）を持ち、`real-session-terminate` まで session lifecycle の
host-loss 経路を実走済み。残る Degraded/Recovery/Timeout/
Visibility/Crash/Hang は、現行 session API が全てを実 Window で
駆動可能なため、本 slice で catalog に載せる。

## Scope

### In Scope

- `real_catalog.rs`: `REAL_CATALOG` 末尾へ 6 行追加（15 → 21）。
- `real_dock.rs`: `real_entries()` 21 entry 化、`attempt_real_
  attach` の open 駆動を case 供給 `drive_open` seam へ一般化、
  6 case 本体、blocked/test 行数追従、doc 記述追従。
- `tests/console.rs`: 23 → 29 行・期待 case 一覧追従。
- crate doc（`README.md`）の行数・verdict 表追従。
- `docs/designs/issue-389-plugin-editor-session.md` の該当節
  reconcile、doc chain、`evidence/rf389/` 記録。

### Out of Scope

- `real-session-move`（採否は下記で確定: 不採用）、adapter verb
  実装、`real-dcomp-pixel` 画素、RT 非干渉、consumer gates、
  Issue close、`mise.toml`/Cargo 変更、docs index。

## Components and Boundaries

- `attempt_real_attach` は slot declare→bind→project→
  `lease_parent`→`container_token`→`begin_session`→**open 駆動**
  →`observe_open`→teardown の骨格を維持し、固定だった
  `open_session_docked` 呼出しを case 供給の `drive_open`
  クロージャへ置き換える。`drive_open` は
  `(&RealOsGuiTest, &mut InProcessHost, PluginHandle,
  &mut PluginEditorSessions, PluginEditorSessionId,
  ParentSlotLease) -> Option<Result<EditorCapabilities,
  EditorSessionError>>` を返し、自らの step row を `steps` に
  記録する（`None` = session 消失 → `Setup`）。戻り値は
  `RealAttachAttempt` と同じ写像で verdict 入力になる。
- 既存 5 case（attach/resize/redock/terminate/dcomp-pixel・
  foreground・present が使う body）は `drive_open_docked`
  （= 現行 `open_session_docked` + `session.open(Docked)` step
  row をそのまま記録）を渡し、証拠 JSONL の step 名・verdict・
  detail を不変に保つ。
- `InProcessEditorOps::open` は `AttachMode::Docked` 以外を
  redeemer 呼出し前に `Unsupported` で拒否する
  （editor_session.rs:1624-1627）— degrade/recovery の
  Floating refused は lease に触れない defined refusal。
- `Vst3Plugin::open_gui` は `self.view.is_some()` で
  `PluginFailed` を返す（host.rs:3056-3061）— live editor 上の
  `recover`/`open` は成功しない。したがって recovery の `Failed`
  起点は refused open（`editor_live=false`）に限る。
- `open_or_degrade(preferred=Floating, fallback=Docked)`:
  preferred の `Unsupported` のみ 1 回 fallback retry（同一
  request id）。fallback 成功で `Attached{Docked}` +
  `Degraded{Docked}` event（`Attached` ではない）+
  `OpenOutcome::Degraded{mode: Docked}`。Degrade は Native
  Dock PASS に計上されない契約（editor_session.rs:1294-1368）。
- `SessionDeadlinePolicy{timeout: ZERO}` は request mint 時に
  `clock.now()+0` を arm し、次の `tick` で必ず expired
  （`now() >= deadline`）— `begin_detach` が mint した
  `Detaching{request}` を `tick_all` が `Failed{Timeout}` に
  収束させる。deadline が原因であり synthetic ではない。
- `set_visibility`/`set_size`/`reattach` は
  `InProcessEditorOps` が無条件 `Unsupported`（L1660-1683）。
  `set_visibility` refusal は session 不変（`Attached` 維持・
  event なし）が contract。
- crash/hang: `test_fault` の abort/exit/hang は fixture が host
  process 自体を kill/wedge する fault law で、in-process lane
  では生存不能。`L1EditorOps` は全 op を defined refusal。
  out-of-process plugin host は未実装 — case body は GUI/session
  に触れず `Unsupported` を返す（`real_window_dpi_case`/
  `real_window_close_case` と同型）。

## `real-session-move` 採否（確定: 不採用）

`session.reattach` は `InProcessEditorOps::reattach` が lease を
一切検査せず無条件 `Unsupported` を返す。第 2 slot/lease を
declare して move を試みても refusal の landing・event 列
（`[Attached{Docked}, Detached, Failed{Unsupported}, Closed]`）
・verdict（`Unsupported`）は `real-editor-redock` と完全に同一
であり、catalog 行を増やしても観測情報が増えない。adapter verb
実装後に move 固有の oracle（別 container への child 再 parent
等）が定義された時点で別 slice が行を追加する。catalog は
21 行に確定する。

## Data and Control Flow

- `real-session-degraded` drive: `open_or_degrade(Floating,
  Docked)` → step `session.open_or_degrade(Floating→Docked)`
  （outcome `degraded`/`attached`/error を detail に記録）。
  observe: child contract → `session.close` → child teardown →
  `Closed`+`concluded` + events ちょうど `[Degraded{Docked},
  Closed]` → `ChildVerification` → `editor_child_body` で
  verified→`Pass`。
- `real-session-recovery` drive: `open(Floating)`（期待
  `Ops(Unsupported)` → `Failed{Unsupported}` を step 確認、
  不一致は drive が `Setup`/`Fail` 相当を記録）→
  `recover(Docked)` → step `session.recover(Docked)`。
  observe: child contract → close → teardown → events ちょうど
  `[Failed{Unsupported}, Recovered{Docked}, Closed]` →
  `ChildVerification` → `Pass`。
- `real-session-timeout` drive: `drive_open_docked`（通常
  attach）。observe: child contract → `set_deadline_policy
  (ZERO)` → `begin_detach` → `request_deadline()==Some` →
  `tick_all()==1` → state `Failed{Timeout}` → close → teardown →
  events ちょうど `[Attached{Docked}, Failed{Timeout}, Closed]`
  → `ChildVerification` → `Pass`。
- `real-editor-visibility` drive: `drive_open_docked`。observe:
  child contract → `set_visibility(false)` → defined
  `Ops(Unsupported)` refusal + `Attached` 維持 + event 不変を
  確認 → close → teardown → events は refused 経路でちょうど
  `[Attached{Docked}, Closed]`（受理経路は `[Attached{Docked},
  Hidden, Closed]` を別途期待 — `check_redock_events` と同型の
  分岐）→ `EditorOpVerification{landing}` → `editor_op_body`
  で verified+Refused→`Unsupported`、verified+Accepted→`NotRun`。
- `real-session-crash`/`real-session-hang`: `run_scenario` body
  が `Unsupported` outcome を返すだけ（attach しない）。

## Non-Functional Constraints

- 非 RT・test-only。新規 `unsafe` なし。
- GPUI/Win32 型を公開 API・evidence schema に漏らさない
  （HWND は既存どおり整数記録）。
- verdict の正直性: degrade は `Degraded` event/
  `OpenOutcome::Degraded` の実観測でのみ `Pass`、refused open は
  `Unsupported`、未定義失敗は `Fail`、環境阻害は
  `NotRun`+`environment_blocked`。

## Risks and Trade-offs

- **`drive_open` 一般化の既存 case への影響**: `drive_open_docked`
  が現行 `session.open(Docked)` step row を逐語再現するため、
  既存 7 case の evidence 形状は不変。`attempt_real_attach` の
  teardown 順序も不変。
- **recovery の `Failed` 起点**: live editor 上の `Failed`
  （reattach refusal 等）からの `recover` は `open_gui` の
  `PluginFailed`（view 既存）で失敗する — refused open 由来の
  `editor_live=false` のみが回復可能。case はこの契約を step で
  明示する。
- **timeout の `tick_all` 対象**: registry には当該 session のみ
  → `tick_all()==1` を厳密に期待。deadline ZERO は mint 直後に
  expired となる deterministic 経路。
- **`real-dcomp-pixel` の既知 `Fail`**: lane exit 1 は設計どおり
  の正直な regression 信号であり、本 slice で緩和しない。

## Deferred Design Concerns

- `real-session-move`・adapter verb 実装・crash/hang の
  out-of-process host・RT 非干渉 — 後続 slice。
- `real-dcomp-pixel` 画素 — `scarecrow173/ruviform#532` 決裁待ち。

## Verification Notes

- [ ] `real_catalog_matches_entries`/`blocked_reports_fail_closed`
  が 21 行で green（feature `real-dock` test）。
- [ ] `plugin-dock-console --list --include-real` が 29 行を
  projection（`tests/console.rs`）。
- [ ] `--blocked-probe` が 21 行 `NotRun`+`environment_blocked`・
  violation 0・exit 0。
- [ ] interactive 実走で新 case の実 verdict を
  `evidence/rf389/` に記録（実走不可なら `NotRun`+
  `environment_blocked` をそのまま証拠化）。
- [ ] `check`/`lint`/`test`（default・`real-dock`・`lab-gui`）、
  `build ruviform-plugin-test-vst3`、plugin-host session test
  非回帰、`test-ui-plugin-dock`/`console`/`real-headless`/
  `test-ui-scenario-contract`/`check-ui-driver-boundary`/`prek`
  green。
