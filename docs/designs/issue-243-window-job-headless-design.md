---
id: "DESIGN-034WZGgweiRw2qZcHmfIbr"
type: "design"
status: "approved"
title: "Issue #243 — Window/Job 不変条件 Headless 検証設計"
created: "2026-11-13"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/window_job_headless.rs"
      - type: "file"
        path: "evidence/rf243/window-job-headless-coverage.md"
      - type: "file"
        path: "evidence/rf243/window-job-headless-verification.md"
      - type: "file"
        path: "docs/test-specs/issue-243-window-job-headless.md"
      - type: "file"
        path: "docs/plans/issue-243-window-job-headless-plan.md"
      - type: "file"
        path: "docs/tasks/issue-243-window-job-headless-task.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-243-window-job-headless.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-243-window-job-headless.md"
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-243-windows-multiwindow-smoke.md"
    - "docs/designs/issue-243-gpui-realwin-windows.md"
    - "docs/designs/issue-181-multi-window.md"
    - "docs/designs/issue-182-dock-tree.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-243-window-job-headless.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #243 — Window/Job 不変条件 Headless 検証設計

## Context

`UiRuntime`（`crates/ruviform-ui`）は driver-neutral な
Window/Job runtime で、job の owner（`Option<WindowId>`）は
**event routing のみに使われ、lifetime 制御には使われない**
（Issue #181 契約）。`cancel_window_jobs` は consumer が
close policy として明示的に呼ぶ opt-in 語彙であり、window
の `CloseRequested`/`Destroyed` が自動 cancel を意味しない。

Issue #386 で導入された driver-neutral 経路
（`WindowDriver`/`DriverEvent`/`WindowLifecycle::Action`/
`dispatch_window_intent`/`drain_jobs_routed`/`RoutedEvent`）
は GPUI・WebView・headless consumer が共有する現行契約で
ある。`ruviform-ui-test` の `FakeDriver` はこの契約の
deterministic 実装で、`dummy_app`（`src/bin/dummy_app.rs`）
と GPUI `windowed_test_app` が本番 consumer 配線
（normalize → dispatch → deliver）を体現している。

本スライスはこの 2 層を**そのまま合成**し、既存テストが
個別に固定していない不変条件を Headless Scenario として
追加する。production コードは変更しない。

## Scope

### In Scope

- `crates/ruviform-ui-test/tests/window_job_headless.rs`
  （新規 integration test）: `UiRuntime` + `FakeDriver` の
  consumer 配線ハーネスと 4 テスト
- `evidence/rf243/window-job-headless-coverage.md`:
  受入不変条件 × 既存被覆テストの対応表
- `evidence/rf243/window-job-headless-verification.md`:
  実行コマンド・OS・toolchain・exit code・期待/観測・
  NotRun 残件と Open Gate 接続

### Out of Scope

- `UiRuntime`/`WindowDriver`/`FakeDriver`/`HeadlessIo`/
  scenario runner の変更
- `ruviform-ui-gpui` 側の変更（GPUI simulated/real lane は
  既存被覆の参照のみ）
- 実 OS・実入力・Native Dock・consumer repo
- CI workflow・`mise.toml`・manifest 類

## Components and Boundaries

- **Harness（テスト内 `Harness` struct）**: `FakeDriver` +
  `UiRuntime`（`DemoService` 登録済み）を保持し、consumer
  配線を `pump()` に集約する。
  - `DriverEvent::Windowed{Action}` →
    `rt.dispatch_window_intent` → 各 `RoutedEvent` を
    journal 記録後 `deliver_routed`（= `post_event`）
  - `DriverEvent::Tick` → `rt.drain_jobs_routed` → 同上
  - `CloseRequested` → `respond_close(Approve|Veto)`
    （テストが policy を切替）+ `lifecycle(Closing)` を
    配信（`dummy_app` と同じ副作用）
  - `Destroyed` → `cancel_jobs_on_destroy` が真なら
    `rt.cancel_window_jobs(window)`（consumer の window-job
    cleanup policy。`dummy_app` parity。偽にすると orphan
    job の観測が可能）
  - `routed: Vec<RoutedEvent>` — **delivery 前**の
    runtime 産出イベントを全件記録し、「runtime がどこへ
    route したか」を delivery 層の drop と独立に検証する
- **Scenario helpers**: `open`（create + Ready）、
  `send_intent`（Action inject + pump）、`spawn_sleep`
  （`demo.sleep` job を window 経由で発行し ack から job id
  を回収）、`spawn_headless`（`dispatch_op` 経由 = ownerless
  job）、`drain_until`（deadline 付き Tick ポーリング）。
- **Assertion helpers**: `terminal_routed_to`（指定 job の
  terminal `job` event が指定 window 宛か）、
  `receipt_routed_to`（Issue #456 command receipt の
  `Result{id, ok}` が指定 window 宛か）、`last_job_state`、
  `routed_unrouted`。

## Data and Control Flow

1. `open(w)` で window を生成・Ready 化（`Created`/`Ready`
   は `DriverEvent::Windowed` として pump 経由で consumer
   に届く）。
2. `spawn_sleep` が `demo.sleep{ms}` intent を window 宛に
   inject → `dispatch_window_intent` → ack `result`
   `{job, label}` が `post_event` で window に配信され、
   job worker thread が起動（owner = `Some(window)` が
   `JobRecord` に記録される）。
3. `spawn_headless` は `dispatch_op`（owner = `None`）で
   同じ `demo.sleep` を発行 — Application/Project-owned
   job の headless 相当。
4. `cancel_window_jobs(w)` / `job.cancel` / window
   `Destroyed` + consumer policy の各経路で cancel signal
   が job worker に届き、`JobSignal::Finished` が mailbox
   に積まれる。
5. `Tick` → `drain_jobs_routed` が signal を `RoutedEvent`
   に変換（owner に基づく `window` 付与 + Issue #456
   terminal receipt 併産）→ journal 記録 → `deliver_routed`
   → `post_event`（生存 window は `delivered` へ、死んだ
   window は silent drop）。

## Non-Functional Constraints

- **Deterministic / non-RT**: `FakeDriver` は queue ベース
  の決定的 driver。job worker は実 thread だが、待ちは
  `drain_until` の deadline 付き bounded poll（10s 上限、
  5ms 間隔）に限定し、flaky な固定 sleep に依存しない。
- **OS-L1 honesty**: 本テストの Pass は Fake/Headless の
  契約適合（OS-L1）のみを意味し、実 Window・実入力・
  Native Dock の受入（GUI-L2/EXT-L3）を構成しない。
- **lint**: workspace `clippy::pedantic` + `-D warnings`
  適合（`collapsible_match`・`double_ended_iterator_last`
  等は実装時に修正済み）。

## Risks and Trade-offs

| リスク | 対応 |
| --- | --- |
| job thread のタイミング依存で flaky 化 | `demo.sleep` の cancel 観測間隔（10ms）と deadline 10s の余裕。短命 job（200ms）は destroy 直後に終端するよう spawn→destroy を連続実行し、margin を確保 |
| 既存被覆との重複テスト化 | 追加前に `multi_window.rs`/`fake_driver.rs`/`driver_contract.rs`/`lifecycle_fixture.rs`/`e2e_process.rs`/`ipc_conformance.rs` の被覆を棚卸しし、未固定の不変条件のみ追加。対応表を `evidence/rf243/` に記録 |
| 「App/Project-owned」の runtime 上の表現が `owner: None` のみ | runtime に project 概念はなく、window 非紐付け job が同じ語彙。テスト・docs でこの対応を明示し、consumer 側の区別（cancel policy）は close-policy テストで被覆 |
| headless 合成が `dummy_app`/`windowed_test_app` と乖離 | harness は `dummy_app::on_event` の分岐を忠実に写し、違いは journal 記録と policy knob のみ（テスト観測用） |

## Deferred Design Concerns

- GPUI simulated 層（`gpui-test`）での同一不変条件: 既存
  `lab_binding.rs` UI-DVR-08・`scenario_contract.rs` が
  部分被覆。本スライスは Fake/Headless 層に限定し、GPUI
  側の追加は別 slice。
- `HeadlessIo`/`TestApp` 経由の fixture scenario 化:
  `window-lifecycle.v1.json` の拡張は共有 fixture の
  変更を伴うため本スライスでは行わず、in-process 合成
  テストに留める（fixture 変更は別 Issue 候補）。

## Verification Notes

- [x] `mise run test ruviform-ui-test` — 新規 4 テストを
  含む全 test target が pass
- [x] `mise run lint ruviform-ui-test` — clippy `-D
  warnings` clean
- [x] `mise run check ruviform-ui-test` / `mise run
  fmt-check` — clean
- [x] `evidence/rf243/` に被覆対応表・実行記録・NotRun
  残件を記録
