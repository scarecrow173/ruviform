---
id: "DESIGN-rf874-rt-oracle-01"
type: "design"
status: "approved"
title: "Issue #874 実 Window RT 非干渉 oracle 設計"
created: "2026-11-22"
updated: "2026-11-22"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/874"
    - "docs/specs/issue-874-rt-oracle.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-874-rt-oracle.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-874-rt-oracle.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-874-rt-oracle.md"
    - "docs/discovery/issue-874-rt-oracle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-874-rt-oracle.md"
    - "docs/designs/issue-389-real-session-lifecycle.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-874-rt-oracle.md"
  derived-by:
    - "docs/test-specs/issue-874-rt-oracle.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-874-rt-oracle.md"
  references:
    - "evidence/rf874/"
    - "evidence/rf631/"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/872"
  deferred-by: []
---

# Issue #874 実 Window RT 非干渉 oracle 設計

## Context

`rf631.plugin-dock-real` lane に Issue #389 系 AC「実 Window での
RT 非干渉」の Required 脚に対応する oracle を追加する。
headless での DSP 非干渉は `editor_session` tests で検証済み
だが、実 Window 経路の実測照合がない。設計の裁定根拠は
`docs/discovery/issue-874-rt-oracle.md`、verdict 語彙・観測
契約の正本は `docs/specs/issue-874-rt-oracle.md` R4/R5/R6。

## Scope

- 編集対象: `crates/ruviform-plugin-dock-test/src/
  real_catalog.rs`・`crates/ruviform-plugin-dock-test/src/
  real_dock.rs`・`crates/ruviform-plugin-dock-test/tests/
  console.rs`・`crates/ruviform-plugin-dock-test/README.md`。
- 条件付き lease `Cargo.toml` は不採用（R5 で使う型は全て既存
  dep で足りる）。
- oracle は観測のみ — host/plugin/GUI driver/lane 構造/CI
  workflow は変更しない。

## Components and Boundaries

### 3 行構成の確定

`real-session-crash`/`real-session-hang` が別行 `Unsupported`
のまま残る先例に倣い、verdict 単位の正直性に合わせて 3 行を
`REAL_CATALOG`/`real_entries` の末尾（`real-session-hang` 直後）
に追加する:

| case | required | spec_extras | verdict intent |
| --- | --- | --- | --- |
| `real-session-rt-samethread` | `[NativePluginDock]` | `editor_attach_extras`（`plugin_artifact` 付き） | Pass/Fail/NotRun/Unsupported |
| `real-session-rt-fault` | `[NativePluginDock]` | `editor_attach_extras` | Pass/Fail/NotRun |
| `real-session-rt-boundary` | `[NativePluginDock]` | `real_window_extras` | Unsupported（boundary のみ） |

### case body の構成

#### `real-session-rt-samethread`（same-thread leg）

`editor_child_body` と同型の `rt_samethread_body` を新設する。
`prepare_editor_host`（Scan→Instantiate→Initialize→Activate）
の後で `HostCommand::StartProcessing` を defined step として
実行し、`Processing` で `attempt_real_attach` に入る。

`AttachDrive`/`ObserveOpen` を RT 用に差し替える:

- `drive_open_rt_samethread` — `rt_block`（`rt_pre_open`）→
  `open_session_docked` → `rt_block`（`rt_post_open`）。open が
  defined 結果に収束しない場合は `StopProcessing`+`Deactivate`
  を踏んで trailing `Destroy` が合法な lifecycle に戻す。
- `observe_rt_samethread_lifecycle` — `observe_attached_children`
  → `rt_block`（`rt_attached_1`）→ `session.set_size`（
  `real_editor_ops` 経由・landing は defined 収束として記録）
  → `rt_block`（`rt_attached_2`）→ `conclude_editor_session`
  （close+detached+converge・`ATTACH_CLOSE_EVENTS` exact）→
  `rt_block`（`rt_post_close`）→ `host.lifecycle_state(handle)`
  が `Some(LifecycleState::Processing)` を確認 →
  `StopProcessing`/`Deactivate` を defined step として記録
  （trailing `Destroy` が `Initialized` から合法になる）。
- verdict tail は `editor_child_body` と同一の
  `attach_attempt_failure` + `ChildVerification` mapping を使う
  ため、`editor_child_body` の tail（attempt→verification 写像
  → `steps_outcome`）を `child_attempt_outcome` として抽出して
  共用する。

`rt_block` は mono・8 frame・input `0.5f32`・
`EventList::empty`・事前確保した stack 配列を
`PlanarBufferView`/`PlanarBufferViewMut` 経由で
`HostInput`/`HostOutput` に包む — block 内で alloc しない。
各 block の step row は `step`/`verdict`/`status`/
`pass_through`/`wall_ns`/`first_sample` を記録し、verdict 入力
は `Ok(ProcessResult)` + `status == Continue` + `pass_through`
のみ。`wall_ns` は `Instant::now()` 前後の wall time の
observed 記録。

#### `real-session-rt-fault`（negative-control leg）

`session_rt_fault_body` を新設する。`prepare_editor_host`
相当の prologue で `descriptors()` から `name == "test_fault"`
を解決し、`activate_test_gain` で `Activated` まで進めてから
`StartProcessing` を踏む。block 列は spec R2 の表どおり（
baseline → armed → still_armed → disarmed → post_disarm）で、
各 block の step row に `armed_expected`/`marker`/`pass_through`
を記録する。teardown は `StopProcessing`→`Deactivate`→
`Destroy` の defined 順序。

`ParamValue` event は `PluginEvent::new(0, EventKind::ParamValue(
ParamEvent{param_id, value}))` として構築し、`EventList::new(&
events, RT_BLOCK_FRAMES_U32)` 経由で block に渡す。`Event`/
`EventKind`/`ParamEvent`/`ParamValue`/`ParamId`/`EventList` は
`ruviform-plugin-core` から、`PlanarBufferView`/
`PlanarBufferViewMut` は `ruviform-audio-core` から import
（既存 dep・新規不要）。

#### `real-session-rt-boundary`（boundary leg）

`real-session-crash`/`real-session-hang` と同型。実 session 内
で `run_scenario` を実行し、`ScenarioOutcome::new(Scenario
Verdict::Unsupported)` に detail + `session:rt:cross-thread-
boundary` evidence tag を載せて `finalize_case` に渡す。detail
は cross-thread 不可の各根拠（`create_sendable`→`None`・
Plain slot・`attach_endpoint` pub(crate)・`PluginHost.l0` 非
公開）と deferred 先 `scarecrow173/ruviform#872` を明記する。

## `RtRuntime` co-scheduled leg 採否（確定: 不採用）

`ruviform-audio-test` の `RtRuntime` を helper thread で pump
する案は、同一 instance の観測ではなく ambient な並行負荷で
しかない。verdict 入力になる観測（status/lifecycle/出力）は
負荷の有無で変わらず、flake 要因だけが増える。GPUI event
loop timing への影響も未検証で、Issue の「cross-thread 観測」
要求を満たさない。不採用。

## Data and Control Flow

```text
run_all → run_entries → real_session_rt_*_case
  → run_scenario(&mut *ctx.gui, spec, |gui| body(...))
    samethread: prepare_editor_host → StartProcessing
      → attempt_real_attach(drive_open_rt, observe_rt_lifecycle)
      → child_attempt_outcome(attempt, verification, steps, cleanup)
    fault: activate_test_gain(test_fault) → StartProcessing
      → rt_block×5 (baseline/armed/still/disarm/post)
      → StopProcessing → Deactivate → Destroy → steps_outcome
    boundary: run_scenario → ScenarioOutcome(Unsupported, detail)
  → finalize_case(report, ctx, RealWindow, artifact, extras)
```

`host.process` は scenario caller thread 上で session verb と
逐次 interleave される — GUI pump thread とは別だが、同一
instance を同一 thread から駆動する same-thread 観測として
実現可能（`InProcessHost::process` は `pub`・Plain slot 直接
駆動・RT-suitable 契約）。

## Non-Functional Constraints

- RT 経路の block 実行は alloc/block しない（stack 配列・
  `EventList` 事前構築・`Instant` 記録のみ）。
- `wall_ns` は observed 記録のみ — verdict 入力にしない。
- `unsafe` 不使用（workspace lint で実質 deny）。
- `EnvironmentKind::RealWindow` は `finalize_case` 経由の実
  session 由来 report のみ — `blocked_reports` は session なし
  の `NotRun`+`environment_blocked`。
- fault mode は `rt_alloc` のみ（`abort`/`exit`/`hang` は
  process kill/wedge — in-process lane 禁止）。

## Risks and Trade-offs

- **block 中の GUI pump 遅延**: `host.process` を session verb
  の前後で呼ぶため、verb 実行中は RT block が来ない — これは
  「verb の前後で DSP が無傷か」を見る same-thread 観測の
  設計上の性質であり、真の RT callback thread との同時性は
  boundary leg が正直 `Unsupported` で記録する欠落面。
- **`test_fault` descriptor 欠落**: cdylib が古い等の環境要因
  は `NotRun`+`environment_blocked` に写像する（`load_plugin`
  の `test_editor` 欠落 `Fail` 先例と区別）。
- **`set_size` declined**: plugin が `canResize` を拒否すると
  defined `Unsupported` landing になる — DSP oracle とは独立
  の verb 収束として記録し、Pass を妨げない。
- **shared_file 競合**: rf#873 worker が同時期に
  `real_catalog.rs`/`real_dock.rs`/`tests/console.rs` を変更
  し得る。新 case は catalog/entries の末尾追加ブロックとし、
  既存行を改変しない。

## Deferred Design Concerns

- cross-thread 同一 instance pump の Pass 化 — `scarecrow173/
  ruviform#872`（L1 out-of-process host）または GUI-capable
  `create_sendable` 契約の将来 Issue。
- `rt_runtime` helper-thread leg — ambient 負荷で同一
  instance 観測にならないため不採用（本 doc「採否」節参照）。

## Verification Notes

- `mise run fmt-check`・`mise run check/lint/test
  ruviform-plugin-dock-test`・同 `--features real-dock`・
  `mise run test-ui-plugin-dock`/`test-ui-plugin-dock-console`/
  `test-ui-plugin-dock-real-headless`/`test-ui-scenario-contract`/
  `check-ui-driver-boundary`・`mise exec -- prek run --all-files`。
- AC1 interactive 実走は `mise run test-ui-plugin-dock-real` —
  本環境が headless なら実走せず blocked-probe で fail-closed
  を実証して coordinator gate に残す（`NotRun`+
  `environment_blocked` として正直記録）。
- 行数追従対象: `real_entries` 戻り型 `[..; 32]`・
  `blocked_reports`/`blocked_probe_violations` 配列宣言・
  `real_dock.rs` 先頭 doc table・`real_catalog.rs` の
  "twenty-nine" 記述・`tests/console.rs` の `rows.len() ==
  41` + expected list・`README.md` case 表。
