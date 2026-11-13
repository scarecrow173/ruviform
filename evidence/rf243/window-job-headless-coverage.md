# Issue #243 Window/Job Headless — 被覆対応表

- 作成日: 2026-11-14
- 対象 SHA: ブランチ `test/243-window-job-headless` の作業 tree
- 目的: Issue #243 残存受入「2 Window の Ownership/Move/
  Close/Restore・Window-owned 対 App/Project-owned Job・
  stale Window/Receipt の不変条件を Fake/Headless Scenario
  で既存 CI に照合する」について、**既存テストの被覆状況**
  と**本スライスで追加した被覆**を対応付ける。
  既に被覆済みの不変条件は新規テストを追加していない。

## 経路の区別（重要）

本リポジトリには Window/Job を駆動する 2 経路がある。

| 経路 | API | 既存被覆の主な所在 |
| --- | --- | --- |
| legacy host 経路 | `UiRuntime::handle` / `handle_window` / `WindowHandle` / `HostEvent` / `poll` / `drain_jobs` | `crates/ruviform-ui/tests/multi_window.rs`, `runtime.rs`, `driver_contract.rs` (UI-DVR-*) |
| driver-neutral 経路（#386） | `WindowDriver` / `DriverEvent` / `WindowLifecycle::Action` / `dispatch_window_intent` / `drain_jobs_routed` / `RoutedEvent` / `post_event` | `crates/ruviform-ui-test/tests/fake_driver.rs`, `driver_parity.rs`, `lifecycle_fixture.rs`, `headless_contract.rs`, `e2e_process.rs`, `scenario_contract.rs`, `native_slot_contract.rs`, `crates/ruviform-ui/tests/ipc_conformance.rs` |

本スライスの新規テスト `window_job_headless.rs` は
**driver-neutral 経路で `UiRuntime` と `FakeDriver` を
consumer 配線（`dummy_app` parity）で合成**する点が特徴で、
legacy `handle_window` 経路との混在は行わない（parity は
`runtime.rs::dispatch_window_intent_parity_with_message_path`
が既に固定）。

## 不変条件 × 被覆対応表

### A. 2 Window ownership / event separation

| 不変条件 | 既存被覆 | 本スライス追加 |
| --- | --- | --- |
| 2 window の event が `WindowId` で分離される | `multi_window.rs::multi_window_events_stay_separated`, `colliding_correlation_ids_route_by_window`, `service_context_sees_origin_window`（legacy 経路） | —（既存被覆で十分。新規テストも 2 window 構成で間接的に確認） |
| sibling window が他方の teardown 後も自身宛 traffic を受け取る | `multi_window.rs::subwindow_close_leaves_jobs_and_loop_running`（legacy） | `close_policy_cancels_only_the_closed_windows_jobs`（`sys.ping` で w1 応答を確認、driver-neutral 合成経路） |
| scenario target が window 毎に独立解決される | `scenario_contract.rs::multi_window_targets_resolve_independently`, `unknown_window_view_and_component_are_explicit` | — |

### B. Window lifecycle（Create/Ready/Close/Restore/Destroy）

| 不変条件 | 既存被覆 | 本スライス追加 |
| --- | --- | --- |
| create → ready → close request → approve/veto/defer → destroy の lifecycle | `fake_driver.rs::two_window_lifecycle`, `close_veto_then_approve_and_destroyed`, `unanswered_close_defaults_to_approve_for_that_window_only`, `deferred_close_stays_pending_until_decided`, `parity_with_fake_host_close_semantics`; `multi_window.rs::close_request_can_be_vetoed`, `destroy_is_distinct_from_close_request`, `deferred_close_stays_alive_then_destroyed` | `close_policy_cancels_only_the_closed_windows_jobs`（合成経路で veto→approve→destroyed→consumer policy を通しで検証） |
| Ready 前の event は buffer され Ready で flush | `fake_driver.rs::post_event_buffers_until_ready_then_flushes`, `multi_window.rs::eval_before_ready_is_buffered_then_flushed` | — |
| destroy ≠ close request（veto で生存、destroyed event は退役 id にも配送） | `fake_driver.rs::destroyed_events_to_retired_ids_still_reach_consumer`, `multi_window.rs::destroy_is_distinct_from_close_request` | `job_outliving_its_window_routes_terminal_to_the_dead_window_only`（`destroy_window` 再呼出 error・id 非再利用を driver-neutral 経路で確認） |

### C. Move / Restore（view move・dock 2-phase）

| 不変条件 | 既存被覆 | 本スライス追加 |
| --- | --- | --- |
| dock 2-phase move（prepare→commit/rollback）・view id 保持・window close 時の consumer disposition | `crates/ruviform-ui/tests/dock.rs::two_phase_move_prepares_destination_then_switches_ownership`, `window_creation_failure_rolls_back_to_source_dock`, `commit_after_destination_destroyed_rolls_back`, `detach_reattach_preserves_logical_view_id`, `menu_move_fallback_matches_drop_move`, `window_close_applies_consumer_chosen_disposition`; `driver_contract.rs::ui_dvr_05_dock_two_phase_move_rollback`; `native_slot_contract.rs::dock_move_abort_rolls_back_with_the_live_lease_kept`, `dock_move_commit_reattaches_under_a_new_slot_and_revokes_the_old` | —（view move は dock/native-slot 層の契約で被覆済み） |
| `WindowLifecycle::Moved` の発火 | **FakeDriver は move 要求 API を持たず `Moved` は inject のみ** — driver 側発火の生成は対象外。GPUI 側は `windowed_uitest.rs`/`real_window.rs` が実窓 lane で被覆 | NotRun（FakeDriver move 生成は driver API 不在のため。本スライス対象外として記録） |

### D. Window-owned vs Application/Project-owned Job

| 不変条件 | 既存被覆 | 本スライス追加 |
| --- | --- | --- |
| job event が owner window 宛に routing される | `multi_window.rs::job_events_route_to_owner_window`（legacy 経路） | `window_job_cancel_is_exact_and_spares_unowned_jobs`（driver-neutral 経路で owner routing + receipt routing を確認） |
| ownerless（headless `dispatch_op`）job の event は unrouted（`window: None`）で consumer が sink を選ぶ | `multi_window.rs::unrouted_events_for_headless_dispatch`（legacy 経路） | 同上 — ownerless job の terminal が `window: None` のまま、かつ cancel 対象外であることを確認 |
| command receipt（Issue #456）が spawning intent id で terminal に併産される | `runtime.rs::job_command_receipt_follows_the_job_terminal`, `concurrent_job_commands_receive_their_own_receipts`（legacy）; `driver_parity.rs::ui_dvr_03_parity_job_command_receipt_phases` | `receipt_routed_to` helper で全 4 テストが routed receipt を assert |

### E. Window-scoped job cancel の正確性

| 不変条件 | 既存被覆 | 本スライス追加 |
| --- | --- | --- |
| `cancel_window_jobs(w)` が window の job を cancel し terminal `Cancelled` に至る | `driver_contract.rs::ui_dvr_08_teardown_cancels_jobs`（**単一 window のみ**・legacy `handle_window` 経路）, `runtime.rs::job_cancel_reaches_terminal_state` | `window_job_cancel_is_exact_and_spares_unowned_jobs` — 戻り値の正確性（`== 1`）・他 window job・ownerless job の生存を driver-neutral 合成経路で固定 |
| `cancel_window_jobs` が他 window/ownerless job に触れない | **未被覆**（UI-DVR-08 は単一 window のみ） | 同上（本スライスの中核的追加） |
| close ≠ cancel（`CloseRequested`/`Destroyed` は job を自動 cancel しない。cleanup は consumer の opt-in policy） | `multi_window.rs::close_requested_does_not_cancel_jobs`, `subwindow_close_leaves_jobs_and_loop_running`（legacy 経路）; 対照的に `runtime.rs::close_requested_cancels_jobs_and_emits_lifecycle` は legacy `handle(HostEvent::CloseRequested)` の**全 job cancel** を固定（経路が異なる点に注意） | `close_policy_cancels_only_the_closed_windows_jobs` — veto で job 生存・approve+destroyed で consumer policy が閉じた window の job のみ cancel・sibling/ownerless 生存を合成経路で固定 |

### F. stale Window / Receipt

| 不変条件 | 既存被覆 | 本スライス追加 |
| --- | --- | --- |
| 破棄済み window 宛の delivery は安全に drop される | `multi_window.rs::delivery_to_dead_window_is_safe`（legacy）, `fake_driver.rs::post_event_to_destroyed_window_drops_silently`（driver 層） | `job_outliving_its_window_routes_terminal_to_the_dead_window_only` — delivery 層 drop に加え **runtime が stale id 宛に routing し続ける**ことを journal で固定（既存は delivery drop のみ） |
| owner 破棄後も job は終端し terminal event/receipt は記録済み owner 宛に routing される | **未被覆** | 同上 — `cmd-doom` の `ok:true` receipt が `Some(w1)`（dead id）宛、`delivered` には w1 宛が追加されない、sibling `cmd-stable` は `Some(w2)` 宛に正常配送 |
| sibling window への event leak・unrouted 化・誤配送がない | 部分的（`multi_window_events_stay_separated` は生存 window 同士） | 同上 — j1/j2 の全 `Event::Job` が期待 owner のみ宛であることを全件 assert |
| stale `WindowId` が自身の orphan job の cleanup にのみ到達する（id 非再利用と合わせて alias 化しない） | **未被覆** | `cancel_window_jobs_reaches_orphans_via_stale_window_id` — 戻り値 `== 1`、orphan のみ `Cancelled`、`destroy_window(w1)` 再呼出 error、新規 window `w3 != w1` |
| 破棄済み window 宛の inbound intent は dispatch に到達しない | **未被覆**（driver 層の `post_event` drop は被覆済みだが、`Action` intent が result を産出しないことは未固定） | 同上 — stale `Action(demo.echo)` inject 後に `stale-intent` の `Result` が routed journal に存在しないことを assert |

## まとめ

- **新規追加**: 4 テスト（`window_job_headless.rs`）—
  driver-neutral 合成経路での window-scoped cancel 正確性、
  close policy と job cleanup の分離、orphan job の stale-id
  terminal/receipt routing、stale id cleanup、stale inbound
  drop。
- **既存被覆で十分と判断し追加しなかったもの**: 基本的な
  2-window event 分離、veto/approve/defer lifecycle、
  dead-window delivery drop、unrouted headless dispatch、
  dock 2-phase move、native slot 契約、IPC conformance、
  scenario contract、driver parity。
- **NotRun/対象外**: `WindowLifecycle::Moved` の FakeDriver
  側発火（driver API 不在）、実 Window・実入力・DPI/IME
  （GUI-L2）、macOS/Linux native、Native Plugin Dock
  （EXT-L3）— `window-job-headless-verification.md` 参照。
