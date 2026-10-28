---
id: "TASK-034NOXIS4eJ3G8peEvr6pq"
type: "task"
status: "done"
title: "L1 supervisor E2E（実子 restart・bundle burst・crash-loop GaveUp・restore 分類・Send witness）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/116"
    - "https://github.com/scarecrow173/ruviform/issues/117"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-116-supervisor.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-116-supervisor.md"
    - "docs/tasks/l1-test-worker-fault-flags.md"
    - "docs/tasks/l1-supervisor-core.md"
  blocks:
    - "docs/tasks/l1-supervisor-final-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/l1-bundle-e2e-and-granularity-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-116-supervisor.md"
    - "docs/designs/issue-116-supervisor.md"
    - "docs/adr/issue-116-supervisor-placement.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-116-supervisor-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
  defers: []
  deferred-by: []
---

# L1 supervisor E2E（実子 restart・bundle burst・crash-loop GaveUp・restore 分類・Send witness）

## Work

`crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs` を新設し、`l1-test-worker`（`env!("CARGO_BIN_EXE_l1-test-worker")`）実子プロセス + `PluginHost` 公開面のみで supervisor の全復旧経路を検証する。**`#[path]` harness（`l1_proxy_e2e.rs` の `mod l1` include）は使わない** — 別 test target であること自体が AC1「公開 API のみ使用」の compile-level 証明（`pub(crate)` L1 machinery が resolve 不能）かつ `l1_proxy_e2e.rs` 無 diff を R-V5 の zero-diff 証明にする。`composite_e2e` パターン（`PluginHost` + `register_out_of_process` + `OutOfProcessSpec`）に倣い最小 helper を複製する（`WORKER_DESCRIPTOR` 一致・`NOTIFY_TIMEOUT` 流の bounded wait・supervisor poll-loop helper）。

検証列（`issue-116-supervisor-e2e-tests` の oracle — spec AC2/AC3/AC4/AC6/AC7・R-V1–R-V6）:

1. **happy path（AC2/R-V6）** — Dedicated/Bundled 両 route で `kill_child` → `InstanceFailed` drain → `Restarted{id,old,new,bounds,restored}` — `new` は generation bump の別 mint handle（旧 handle `Err(InvalidHandle)` stale）・`lifecycle_state(new)` が recipe 到達点（`config:None`→`Initialized`・`config:Some`→`Activated`・`+start_processing`→`Processing`）・`bounds` echo・`prepare_endpoint(new, bounds)` が動く（`config:Some` recipe — `Restarted` 時点で endpoint 未 attach・caller 再配線）
2. **checkpoint/restore（AC4/R-V3）** — `SaveState` blob → `set_checkpoint` → kill → `restored=Applied`（`SaveState{new}` が checkpoint blob を返す round-trip — `RestoreState` が実際に届いた証明）・未登録 `NotRequested`・`--state-unsupported` → `Skipped`（attempt 消費なし・1 attempt で `Restarted`）・`--fail-restore` → `Err(PluginFailed)` で budget burn → `max_attempts` で `GaveUp`・`set_checkpoint` の `Err(CheckpointTooLarge)` 拒否（oversized が wire に乗らず子 kill に到達しない）
3. **backoff/episode（AC5/R-V4）** — `Instant` 注入で `next_attempt_at` 未達は attempt なし・到達で attempt・`stable_window` 越し crash が **drain 時 pre-check で `failures` reset**（`healthy_since+stable_window` 越しの `InstanceFailed` が `Pending` へ進み旧 failures で即 `GaveUp` しない回帰 pin）・`max_attempts=0` → `GaveUp{attempts:0,last_error:None}`・crash-loop（`--crash-after-msgs` で attempt 毎に子死亡 — message counter は handshake 込み: Scan=1・`CreateInstance`=2）→ bounded `GaveUp{attempts:max,last_error:Some}`・`max_attempts_per_poll=0` → `Err(InertPolicy)`
4. **bundle burst（AC3/R-V1）** — Bundled 上 N supervised + 1 unwatched を `kill_child` → 同一 `ChildExit` N `InstanceFailed` 全 drain → per-entry `Pending` → `max_attempts_per_poll` cap まで attempt・残り繰越（繰越 `Pending` は event なし — `current_handle → None`・`Restarted` 列数で観測）→ 全 supervised が新 handle で復旧（pool が fresh 子に収束 — N 子強制せず）・unwatched は `Unsupervised{notification}`
5. **attempt 中失敗（AC7/R-V2）** — mid-`CreateInstance` 死（`--crash-after-msgs=2`）→ `Err(Ipc)`・通知なし・handle 未 mint（cleanup 不要）・`--fail-initialize` → `Err(PluginFailed)`（minted `Created` cleanup `Destroy`）・`Activate` `Err(Prepare)`（`max_frames > WIRE_MAX_FRAMES` config — flag 不要）→ cleanup・`--fail-start-processing` → `Err(PluginFailed)`（`Activated` → unwind `Deactivate`→`Destroy`）・mid-replay 子死亡（`--crash-after-msgs` 中の attempt 中子死）→ `Err(Ipc)`/`Err(Failed)`（mirror `Failed` → `Destroy` 合法・unwind 不要）
6. **lingering（AC7）** — Dedicated で `--fail-start-processing` + `--crash-after-msgs=6`（または `--hang-after-msgs=6` — 応答は 6 まで・後続 `Destroy` が timeout で同型）— fresh 子 count は checkpoint 未登録で Scan=1・CreateInstance=2・Initialize=3・Activate=4・StartProcessing=5（`RestoreState` が入れば以降 +1 — 不 commit `Err` → minted `Activated` 残留・unwind は `Deactivate` 起点 — `--fail-stop-processing` は unwind `StopProcessing` 発行に minted mirror `Processing` を要し到達不能）・unwind `Deactivate`=6 着信で子死亡 `Err(Ipc)` → minted `Failed` 収束・unwind 打ち切りで live handle `lingering` → `current_handle` 露出・次 poll の lingering retry `Destroy`（`Failed` から合法 — Phase-2・後発 `InstanceFailed` の Phase-1 drain 一致でも clear）で解消（外部 `Destroy` なら `Err(InvalidHandle)` で clear）
7. **`Failed` adopt（AC9）** — `kill_child` 後に通知を drain せず `watch`（`lifecycle_state → Some(Failed)`）→ `Destroy{h}` で slot 解放（キュー残 `InstanceFailed` は落ちる = 二重復旧しない）→ `Pending{next:None}` → 次 poll `Restarted`。caller が `try_recv_notification` で通知消費済みの系でも同じ復旧
8. **`GaveUp` 语义・drain 契約（AC6/AC9）** — mint 済み instance は unwind+`Destroy` 済み・entry は `unwatch` まで保持・`GaveUp` は `HostError::Failed` に畳まない別 variant・`last_error` は直近 attempt 失敗・lingering 後発死の再 emit（`attempts`/`last_error` 不変・`(id,attempts)` dedupe）・`Unsupervised` pass-through・`unwatch` が `Option<PluginHandle>`（`Live.current` または `lingering` — orphan させない）を返す
9. **opt-in・無退行（R-V5/AC4）** — supervisor を介さない素 `PluginHost` で `kill_child` → `InstanceFailed`（従来契約 — `Restarted` 相当は出ない）・`cargo test -p ruviform-plugin-host --test l1_proxy_e2e` 全 pass（supervisor 非混入の証明）
10. **型 witness（AC8）** — `assert_impl_all!(Supervisor: Send, Sync)`（`bound_host: usize`）・`PluginHost` `!Send`/`!Sync` 維持 pin・`poll(&mut host, now)` のみで駆動（内部 thread/timer/async なし）

注意: backoff/episode の deadline は全て `Instant` 注入で進める（実時刻依存を作らない — deadline は `poll` 呼出し粒度で解決する確定契約）。child 側の待ちは `NOTIFY_TIMEOUT` 流の bounded wait で bound。`--crash-after-msgs` の N 値は handshake 込み message counter（Scan=1・`CreateInstance`=2 — N=2 が mid-create・N=3 が `Initialize` 中死）。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-116-supervisor.md](../plans/implement-issue-116-supervisor.md)（T3 — E2E）
- [../designs/issue-116-supervisor.md](../designs/issue-116-supervisor.md) — Verification Notes（R-V 行・fixture gap）・失敗分類表・bundle fan-out・endpoint/RT 再配線（caller 責務）・adopt
- [../specs/issue-116-supervisor.md](../specs/issue-116-supervisor.md) AC2–AC4/AC6/AC7・R-V1–R-V6
- [../adr/issue-117-bundle-child-failed-granularity.md](../adr/issue-117-bundle-child-failed-granularity.md) — `Failed` 粒度・fan-out の正本（bundle burst の前提）
- [issue-116-supervisor-e2e-tests.md](../test-specs/issue-116-supervisor-e2e-tests.md) — E2E oracle
- 先行 task: [l1-test-worker-fault-flags.md](l1-test-worker-fault-flags.md)（fixture flag）・[l1-supervisor-core.md](l1-supervisor-core.md)（supervisor API）

## Dependencies

- 先行タスク: [l1-test-worker-fault-flags.md](l1-test-worker-fault-flags.md)（`--fail-restore`/`--state-unsupported`/`--fail-initialize`/`--fail-start-processing` の注入前提）+ [l1-supervisor-core.md](l1-supervisor-core.md)（`Supervisor` 公開 API — 両方確定後）
- 後続タスク: [l1-supervisor-final-verification.md](l1-supervisor-final-verification.md)（T4 — closeout）

## Affected Files

- `crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs` — 新規（supervisor 全 E2E — `#[path]` harness 不使用・public API のみ import）

## Done When

- [x] happy path（Dedicated+Bundled）・checkpoint 4 分類（`Applied`/`NotRequested`/`Skipped`/`PluginFailed` burn）・backoff 全系（drain reset 回帰・`max_attempts=0`・`InertPolicy`・crash-loop `GaveUp`）・bundle burst cap 繰越・attempt 失敗全段・lingering・adopt・`GaveUp`/`Unsupervised`/`unwatch`・opt-in・`Send`/`Sync` witness が実子経路で pin されている（実績: `e0b31f3`+`9a82dca` — `tests/l1_supervisor_e2e.rs` 22 tests 全 pass — `restarted_dedicated_*`/`restarted_bundled_*`/`checkpoint_*`/`stable_window_crash_starts_fresh_episode`/`zero_max_attempts_adopt_gives_up_immediately`/`inert_policy_rejected`/`crash_loop_bounded_gaveup`/`bundle_burst_cap_defers_and_unsupervised`/`mid_create_death_attempt_failed_without_mint`/`fail_initialize_*`/`fail_start_processing_*`/`activate_over_limit_*`/`lingering_*`/`adopt_failed_handle_recovers`/`wrong_host_*`/`plain_host_*`/`rt_l1_supervised_process_no_alloc`）
- [x] `l1_proxy_e2e.rs`/`composite.rs`/`l1/`/`host.rs`/`wire.rs`/`lib.rs`（T2 分を除く）に supervisor 関連 diff がない・`#[path]` harness 不使用（public API のみ import — AC1 の compile-level 証明）（実績: T4 literal sweep — `l1_proxy_e2e.rs`/`composite.rs`/`l1/`/`host.rs`/`wire.rs` 無 diff・`l1_supervisor_e2e.rs` に `#[path]`/`mod l1` 0 hit）
- [x] `cargo test -p ruviform-plugin-host --test l1_proxy_e2e` が無 diff green（R-V5 — supervisor 非混入・AC4 非退行）（実績: T4 gate 100/100 pass）

## Verification

- [x] `cargo test -p ruviform-plugin-host --test l1_supervisor_e2e` が全 pass（実績: T4 gate 22/22 pass）
- [x] `cargo test -p ruviform-plugin-host --test l1_proxy_e2e` が全 pass（無退行 — `l1_proxy_e2e.rs` 無 diff のまま）（実績: T4 gate 100/100 pass）
- [x] `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test`・`mise run test-rt` が全 pass（実績: T4 Full Gate 全 exit 0）
