---
id: "SPEC-rf389-real-session-lifecycle-01"
type: "spec"
status: "approved"
title: "Issue #389 real session lifecycle remainder（real-window session lifecycle L2 cases）"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "POLICIES.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/designs/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/test-specs/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/plans/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/tasks/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/impl/ir/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/impl/exp/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/impl/exp/issue-389-real-session-lifecycle-exp.jsonl"
      - type: "doc"
        path: "docs/discovery/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "evidence/rf389/"
    modified:
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "doc"
        path: "docs/designs/issue-389-plugin-editor-session.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-389-real-session-lifecycle.md"
    - "docs/plans/issue-389-real-session-lifecycle.md"
  depends-on:
    - "docs/specs/issue-389-editor-session.md"
    - "docs/specs/issue-631-real-row.md"
    - "docs/specs/issue-631-real-editor-attach.md"
    - "docs/specs/issue-631-real-window-dock.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-real-session-lifecycle.md"
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 real session lifecycle remainder（real-window session lifecycle L2 cases）

## What Should Be Built

`rf631.plugin-dock-real` シナリオの catalog に session lifecycle
残件の real-window case を追加する（15 → 21 行）。`real-session-
terminate`（PR #690）で確立した real child HWND / session event /
lease 観測の型を再利用し、現行 session API
（`crates/ruviform-plugin-host/src/editor_session.rs`）だけで実走
可能な経路を Pass oracle として、定義済み拒否を `Unsupported`
境界として、in-process lane が生存不能な fault を正直な
`Unsupported` 行として catalog に載せる。

追加行（`real-session-terminate` の次、catalog 末尾）:

- `real-session-degraded` — `open_or_degrade(Floating → Docked)`:
  Floating は `InProcessEditorOps` が lease redeem 前に defined
  refusal、Docked fallback が実 `OpenGui` で child HWND を attach
  し `OpenOutcome::Degraded{Docked}` + `Degraded{Docked}` event
  （`Attached` ではない）を観測して `Pass`。
- `real-session-recovery` — `open(Floating)` refused →
  `Failed{Unsupported}`（`editor_live=false`）→ `recover(Docked)`
  → 実 `OpenGui` → `Recovered{Docked}` + child contract +
  `[Failed{Unsupported}, Recovered{Docked}, Closed]` で `Pass`。
- `real-session-timeout` — docked attach →
  `SessionDeadlinePolicy{timeout: ZERO, MonotonicSessionClock}` →
  `begin_detach` → `request_deadline` → `tick_all` →
  `Failed{Timeout}` → close → `[Attached{Docked},
  Failed{Timeout}, Closed]` で `Pass`（tick/deadline の実 Window
  結線）。
- `real-editor-visibility` — `set_visibility` の defined
  `Unsupported` refusal・session `Attached` 維持・`Hidden`/`Shown`
  event 0 件を観測し `Unsupported` verdict（resize/redock と同型）。
- `real-session-crash` / `real-session-hang` — 正直 `Unsupported`
  境界行。`test_fault` の abort/exit/hang は host process を
  kill/wedge し in-process lane は生存不能、`L1EditorOps` は全 op
  を defined refusal する — fake crash/hang Pass は作らない。

`real-session-move` は採用しない: `session.reattach` は
`InProcessEditorOps` が lease の中身を検査する前に無条件
`Unsupported` を返すため、第 2 slot/lease への move 試行は
`real-editor-redock`（同 lease reattach → `Failed{Unsupported}`）
と観測が完全に同一であり、distinct value を持たない。採否の設計
判断は design/discovery に明記する。

## Why Now

Issue `scarecrow173/ruviform#389` は tracker Issue で、real-window
session lifecycle の L2 検証は `real-session-terminate` までが
完了済み。dispatch 探索
（`.devin/dispatch-specs/rf389-explore.explore.json`）が
Degraded/Recovery/Timeout は現行 API のみで real child HWND まで
実走できる Pass 可能 case、`set_visibility` は最後の未実走
session verb、crash/hang は in-process lane 非生存のため正直
`Unsupported` 行として記録可能と判定し `DISPATCHABLE` を返した。
本 slice はその残件を catalog に載せ、Issue #389 の real-window
受入を前進させる。

## Users and Value

- Scenario Console / Test Lab / lane consumer:
  `--list --include-real` と `ScenarioMatrix::project` が
  degrade/recover/timeout/visibility/crash/hang の判定状態を
  そのまま投影できる。
- 後続 slice: adapter verb（`set_size`/`set_visibility`/`reattach`/
  Floating）実装時に、defined refusal → 実装後 Pass への verdict
  遷移を catalog 行のまま観測できる。
- 運用者: `environment_kind=real-window` を名乗る証拠だけが実走
  verdict を持ち、headless/fake の成功を実 Window Pass に転用
  しない POLICIES.md §2.1 の規範を維持する。

## Scope

### In Scope

- `crates/ruviform-plugin-dock-test/`:
  - `src/real_catalog.rs` — `REAL_CATALOG` に 6 行追加
    （15 → 21、metadata のみ、GPUI-free を維持）。
  - `src/real_dock.rs` — `real_entries()` を 21 entry 化、
    `attempt_real_attach` の open 駆動を case 供給の
    `drive_open` seam へ一般化（既存 case は
    `drive_open_docked` で挙動不変）、degraded/recovery/timeout/
    visibility/crash/hang の case 本体、blocked/test の行数
    assertion 追従、module doc の case 表・行数記述追従。
  - `tests/console.rs` — `list_include_real_appends_real_lane_rows`
    の行数（23 → 29）と期待 case 一覧の追従。
  - `README.md` — "fifteen"/15 行記述と verdict 表の追従。
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp,
  discovery}/issue-389-real-session-lifecycle.*` の doc chain。
- `docs/designs/issue-389-plugin-editor-session.md` — capability
  matrix / verification-status の本 slice 分 reconcile。
- `evidence/rf389/` — 本 slice の検証記録（lane 実走 JSONL・
  verification report）を追加。

### Out of Scope

- `real-session-move` — `reattach` refusal が lease 非依存のため
  `real-editor-redock` と観測同一（design で採否確定・本 spec は
  不採用を固定）。
- adapter verb（`set_size`/`set_visibility`/`reattach`/Floating/
  resize）の実装 — `crates/ruviform-plugin-host`・
  `plugin-vst3`・`plugin-clap` を触る別 slice。
- `real-dcomp-pixel` の画素 Pass 解禁（`scarecrow173/ruviform#532`
  決裁待ち — 現行の正直 verdict を維持）。
- 実 Window での RT 非干渉 oracle、consumer gates
  （`scarecrow173/synvelis#176`・`scarecrow173/ruviform#294`）、
  Issue `scarecrow173/ruviform#389` の close（tracker）。
- `mise.toml` の lane 構造変更、`Cargo.toml`/`Cargo.lock` の依存
  変更、`docs/**/README.md` index の編集・再生成。

## Requirements

- `REAL_CATALOG` と `real_entries()` は case/scenario/required が
  pairwise 一致し、既存 test が drift を検出する。両者の行数は
  21 になる。新規 6 行は `real-session-terminate` の次に置き、
  report 順も一致させる。
- 新規 6 行はすべて `required` が
  `&[ScenarioCapability::NativePluginDock]` — capability preflight
  を通して body が正直な `Unsupported` detail を実 session 内に
  記録する形（crash/hang 行も同じ — `real-window-close` 系の
  boundary row 先例）。spec extras は editor 系 4 行が editor
  case と同形（lane marker + plugin artifact identity）、
  crash/hang 2 行は `real_window_extras`（lane marker のみ —
  fixture を消費しない正直な依存宣言）。
- `real-session-degraded`: `open_or_degrade(Floating, Docked)` が
  `Ok(OpenOutcome::Degraded{mode: Docked,..})` を返し、drain
  event がちょうど `[Degraded{Docked}]`（attach 相当の
  `Attached` event を含まない）で、child HWND 契約と close 収束
  `[Degraded{Docked}, Closed]` が全て成立したときのみ `Pass`。
  `OpenOutcome::Attached`（degrade 不発）や event 不一致は `Fail`。
- `real-session-recovery`: `open(Floating)` が defined
  `Ops(Unsupported)` refusal で `Failed{Unsupported}` に落ちる
  ことを確認後、`recover(Docked)` が `Ok` で `Recovered{Docked}`
  + child HWND 契約 + `[Failed{Unsupported}, Recovered{Docked},
  Closed]` が成立したときのみ `Pass`。途中 step の契約違反は
  `Fail`。
- `real-session-timeout`: attach 後に `set_deadline_policy`
  （`timeout: Duration::ZERO` + `MonotonicSessionClock`）→
  `begin_detach` → `request_deadline()==Some` → `tick_all()==1`
  → `Failed{Timeout}` state+event → close → child teardown +
  `[Attached{Docked}, Failed{Timeout}, Closed]` が成立したとき
  のみ `Pass`。timeout は deadline/tick API が原因であり、
  任意の synthetic verdict ではない。
- `real-editor-visibility`: `set_visibility(false)` が defined
  `Ops(Unsupported)` refusal を返し、session が `Attached` に
  留まり `Hidden`/`Shown` event を emit しないことを観測して
  `Unsupported` verdict。`Ok` 受理は `NotRun`（effect oracle
  なし）、その他の失敗・event 混入は `Fail`。
- `real-session-crash`/`real-session-hang`: body は GUI・session
  に触れず `Unsupported` を返し、detail に「in-process lane は
  fixture の process-killing/wedging fault（`test_fault` の
  abort/exit/hang）を生存できず、`L1EditorOps` は crash/hang
  経路を defined refusal する。out-of-process plugin host は
  未実装」を明記する。
- verdict 写像は既存規律どおり: refused open（定義済み
  `Unsupported`）→ `Unsupported`、その他の失敗 → `Fail`、setup
  失敗 → `Fail`、観測 verified → case 別 verdict、環境阻害 →
  `NotRun`+`environment_blocked`。実走した step のみ `Pass` と
  し、headless/fake evidence を実 Window Pass に昇格しない。
- `attempt_real_attach` teardown（`end_session` → slot/lease
  release → `HostCommand::Destroy`）は全 case で維持する。
- `blocked_reports`/`blocked_probe`/`blocked_probe_violations` が
  新行を含む 21 行を `NotRun`+`observed.environment_blocked`・
  `environment_kind=real-window` 不請求で生成し、
  `--blocked-probe` lane の self-check が violation 0・exit 0。
- `EnvironmentKind::RealWindow` は実 session 内でのみ emit する。
  GPUI/Win32 型を公開 API・evidence schema に漏らさない
  （`check-ui-driver-boundary` green）。

## Acceptance Criteria

- [ ] `REAL_CATALOG`/`real_entries()` が 21 行で一致し、
  `real_catalog_matches_entries`/`blocked_reports_fail_closed`
  test が新行数で green。
- [ ] `plugin-dock-console --list --include-real` が新行を
  projection 表示し、real selector の `--run` 拒否（exit 2）が
  不変。
- [ ] `plugin-dock-real-scenario --features real-dock` が 21 case
  の JSONL を出力する（interactive host では新 case が実判定、
  headless host では全行 `NotRun`+`environment_blocked`）。
- [ ] `--blocked-probe` が 21 行の fail-closed 実証 JSONL を出力
  し、自己検査違反 0・exit 0。
- [ ] `mise run test-ui-plugin-dock-real-headless` が evidence を
  生成し exit 0（本 slice の証拠は `evidence/rf389/` へ記録）。
- [ ] interactive 実走可能な環境では `mise run
  test-ui-plugin-dock-real` を実走し、新 case の実 verdict と
  `environment_kind=real-window` を `evidence/rf389/` に記録
  する。実走不可なら全行 `NotRun`+`environment_blocked` をその
  まま証拠化し、実行していない行を Pass にしない。
- [ ] 既存 real 15 case と headless lane（`test-ui-plugin-dock`
  8 case）の verdict を本 slice の変更で動かさない。
- [ ] `check`/`lint`/`test`（default feature・`real-dock`・
  `lab-gui`）・`build ruviform-plugin-test-vst3`・
  `test ruviform-plugin-host --test editor_session
  --test editor_session_driver`・`test-ui-scenario-contract`・
  `check-ui-driver-boundary`・`prek run --all-files` が green。
- [ ] `review-protocol` を最低 1 cycle 実施し、blocker が残って
  いないことを PR 本文に記録する。
- [ ] Issue `scarecrow173/ruviform#389` は OPEN のまま（tracker —
  残 L2/L3 case・`#532`・consumer gates は後続 slice）。

## Deferred Design Concerns

- `real-session-move`・adapter verb 実装・crash/hang の
  out-of-process 化・実 Window RT 非干渉 — verb/host 基盤が前提
  | 後続 slice | verb 未到着のまま Pass を主張するリスク。
- `real-dcomp-pixel` 画素 Pass — `scarecrow173/ruviform#532` 決裁
  待ち | 決裁後の専用 slice | 既知 occlusion のまま Pass 誤記の
  リスク（現行は正直 `Fail` を維持）。

## Sources and Relations

- Issue `scarecrow173/ruviform#389`（tracker — auto-close しない。
  PR 本文は `Refs #389` のみ）。
- dispatch 探索記録:
  `G:/repositry/audio-products/.devin/dispatch-specs/rf389-explore.explore.json`
  （`DISPATCHABLE`・slice `rf389-real-session-lifecycle`）。
- 前段 slice: `docs/specs/issue-389-editor-session.md`
  （`real-session-terminate`）、`docs/specs/issue-631-real-row.md`
  （catalog 構造）。
- session API（read-only 参照）:
  `crates/ruviform-plugin-host/src/editor_session.rs`
  （`open_or_degrade` L1317・`recover` L1267・`set_visibility`
  L1177・`begin_detach`/`tick`/`tick_all`/`SessionDeadlinePolicy`
  L779/1041/1526/439・`InProcessEditorOps` refusal L1660-1683・
  `L1EditorOps` L1695-1752）。
- host open 契約: `crates/ruviform-plugin-vst3/src/host.rs`
  `open_gui`（view 既存時 `PluginFailed` — recovery は refused
  open 由来の `Failed` からのみ到達可能）。
- 実装先例: `attempt_real_attach` の `observe_open` seam、
  `observe_attached_children`/`check_detached_children`、
  `check_redock_events` の case 別 event 期待値、
  `editor_op_body` の defined-refusal → `Unsupported` 写像、
  `real_window_dpi_case`/`real_window_close_case` の無条件
  `Unsupported` body。
