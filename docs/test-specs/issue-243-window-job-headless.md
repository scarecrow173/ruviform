---
id: "TSPEC-034WZH4z1vyfix6vJI76aI"
type: "test-spec"
status: "approved"
title: "Issue #243 — Window/Job 不変条件 Headless 検証契約"
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
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-243-window-job-headless.md"
    - "docs/designs/issue-243-window-job-headless-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-243-windows-multiwindow-smoke.md"
    - "docs/test-specs/issue-243-gpui-realwin-windows.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-243-window-job-headless.md"
    - "docs/designs/issue-243-window-job-headless-design.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-243-window-job-headless.md"
    - "docs/designs/issue-243-window-job-headless-design.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #243 — Window/Job 不変条件 Headless 検証契約

## Purpose

Issue #243 の残存受入「Window/Job 不変条件を Fake/Headless
Scenario で既存 CI に照合する」のうち headless 可能な
スライスについて、`UiRuntime` + `FakeDriver` の driver-
neutral 合成経路で検証すべき不変条件と、その判定方法
（runtime routing と delivery drop の分離、ownerless job
の扱い、stale id の意味）を固定する契約。既に他テストで
被覆済みの不変条件は本 spec の対象外とし、対応表で記録
するに留める。

## Feature

Window/Job ownership invariants on the Fake/Headless driver-
neutral path — 2 window の独立 ownership、window close と
job lifetime の分離、window-scoped job cancel の正確性、
owner 破棄後の job terminal/receipt routing、stale
`WindowId` の安全性が、deterministic な Fake/Headless
scenario で成立すること。

## Rules

- **R1 合成経路の忠実性**: テストは `dummy_app`/
  `windowed_test_app` と同じ consumer 配線
  （Action → `dispatch_window_intent` → `deliver_routed`、
  Tick → `drain_jobs_routed` → `deliver_routed`、
  CloseRequested → `respond_close`、Destroyed → consumer
  close policy）で `UiRuntime` + `FakeDriver` を駆動する。
  legacy `handle_window`/`WindowHandle` 経路との混在は
  parity テスト既存のため行わない。
- **R2 routing と delivery の分離**: 「runtime が event を
  どの `WindowId` 宛に産出したか」（`RoutedEvent.window`）
  と「delivery 層がそれを実際に window に届けたか」
  （`FakeDriver::delivered`/`post_event` の dead-window
  drop）を別々に観測する。dead window 宛 event が
  `delivered` に現れないことと、runtime が依然として
  stale `WindowId` 宛に産出し続けることの双方を検証する。
- **R3 window-scoped cancel の正確性**:
  `cancel_window_jobs(w)` は `owner == Some(w)` の job のみ
  を signal し、他 window の job と ownerless
  （`dispatch_op`/headless 経由 = Application/Project-owned
  相当）job を決して cancel しない。戻り値は新規 signal
  数を正確に返す。
- **R4 close ≠ cancel**: `CloseRequested`（veto/approve）
  と `Destroyed` 自体は job を cancel しない。window-job
  cleanup は consumer の close policy
  （`cancel_window_jobs` の明示呼出）が担う —
  `dummy_app` parity で policy を切替えて検証する。
- **R5 stale Window/Receipt**: owner window が destroy
  されても、実行中 job は終端し、その terminal `job`
  event と Issue #456 command receipt は記録済みの
  （stale な）`WindowId` 宛に routing され続ける。
  sibling window への leak・unrouted 化・誤配送はない。
  stale id は自身の orphan job の cleanup にのみ到達し、
  別 window の job に触れない（`WindowId` 非再利用と
  合わせて stale handle の alias 化を防ぐ）。
- **R6 stale inbound drop**: 破棄済み window 宛の
  `Action` intent は driver の dead-window drop で
  dispatch に到達せず、result を産出しない。
- **R7 honesty**: Fake/Headless の Pass は OS-L1 契約
  証拠のみ。実 Window・実入力・Native Dock の受入
  （GUI-L2/EXT-L3）を意味しない。既存被覆の不変条件を
  重複テスト化しない（対応表に記録）。

## Examples

- **E1 cancel scoping**: Given w1・w2 が各々 `demo.sleep`
  job を持ち、headless `dispatch_op` が ownerless job を
  持つ When `cancel_window_jobs(w1)` Then w1 の job のみが
  Cancelled terminal（routed `Some(w1)` + receipt
  `ok:false`）に至り、w2 の jobと ownerless job は
  running のまま。ownerless job の terminal は
  `window: None` のまま。
- **E2 close policy**: Given w2 が job を持つ When
  `CloseRequested` に Veto を返す Then window も job も
  生存。Approve に切替えて再送 Then `Destroyed` 後に
  consumer policy が w2 の job のみを cancel し、w1 の
  job と ownerless job は生存。w1 は引き続き自身宛の
  intent に応答する。
- **E3 orphan job**: Given w1 の `demo.sleep(200ms)` が
  running のまま w1 が destroy される（consumer は
  auto-cancel しない）When job が自然終端 Then terminal
  `job` event と `cmd-doom` の applied receipt が
  `window: Some(w1)`（dead id）宛に産出され、`delivered`
  には w1 宛が一切追加されず、w2 の job/receipt は
  `Some(w2)` 宛に正しく配送される。w1 宛の stale
  `Action` は result を産出しない。
- **E4 stale id cleanup**: Given w1 破棄済みでその job が
  orphan のまま When `cancel_window_jobs(w1)` Then 戻り値
  1 で orphan job のみが Cancelled に至り（routed
  `Some(w1)`、delivered には出ない）、w2 の job は
  running のまま。`destroy_window(w1)` の再呼出は error、
  新規 window は新しい `WindowId` を得る。

## Guarantees

- 4 テスト全てが `mise run test ruviform-ui-test` で
  deterministic に pass する（deadline 10s の bounded
  drain、wall-clock 依存は job worker の実時間のみ）。
- 各不変条件の判定は「runtime routing（`RoutedEvent.
  window`）」と「delivery 観測（`delivered`/`post_event`
  drop）」の 2 面で行い、片方の観測で他方を代替しない。
- 既存被覆テスト（`multi_window.rs`・`fake_driver.rs`・
  `driver_contract.rs`・`lifecycle_fixture.rs`・
  `e2e_process.rs`・`ipc_conformance.rs`・`dock.rs`・
  `native_slot_contract.rs`・`driver_parity.rs`・
  `scenario_contract.rs`）との対応が
  `evidence/rf243/window-job-headless-coverage.md` に
  記録される。

## Non-goals

- 実 Window・実 OS 入力・実 GUI での同一不変条件
  （GUI-L2。既存 real-OS lane の証拠を参照するのみ）。
- Native Plugin Dock・実 plugin/device（EXT-L3）。
- macOS/Linux native 実行（環境外、NotRun 記録）。
- `UiRuntime`/`FakeDriver`/scenario 基盤・共有 fixture
  （`window-lifecycle.v1.json`）の変更。
- 物理手動入力・consumer 実機非回帰。

## Risk

- job worker thread の timing 依存で flaky 化する危険
  → `demo.sleep` の cancel 観測間隔と deadline 余裕で
  吸収し、固定 sleep には依存しない。
- 「ownerless = App/Project-owned」の対応が runtime 語彙
  上は `Option<WindowId>::None` のみである点を誤読する
  危険 → test-spec/design/evidence で対応を明示し、
  consumer 側の policy 区別は close-policy テストで
  被覆する。
