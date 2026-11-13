---
id: "SPEC-034WZG9kaa77G2ljsalr6Q"
type: "spec"
status: "implemented"
title: "Issue #243 — Window/Job 不変条件の Fake/Headless Scenario 検証（部分スコープ・後続スライス）"
created: "2026-11-13"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/181"
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/window_job_headless.rs"
      - type: "file"
        path: "evidence/rf243/window-job-headless-coverage.md"
      - type: "file"
        path: "evidence/rf243/window-job-headless-verification.md"
      - type: "file"
        path: "docs/designs/issue-243-window-job-headless-design.md"
      - type: "file"
        path: "docs/plans/issue-243-window-job-headless-plan.md"
      - type: "file"
        path: "docs/tasks/issue-243-window-job-headless-task.md"
      - type: "file"
        path: "docs/test-specs/issue-243-window-job-headless.md"
      - type: "file"
        path: "docs/impl/ir/issue-243-window-job-headless.md"
      - type: "file"
        path: "docs/impl/exp/issue-243-window-job-headless.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
    - "docs/specs/issue-243-gpui-realwin-windows.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-181-multi-window.md"
    - "docs/specs/issue-182-dock-tree.md"
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/impl/ir/issue-243-window-job-headless.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #243 — Window/Job 不変条件の Fake/Headless Scenario 検証（部分スコープ・後続スライス）

## What Should Be Built

Issue #243 の残存受入項目「2 Window の Ownership/Move/Close/
Restore・Window-owned 対 App/Project-owned Job・stale Window/
Receipt の不変条件を Fake/Headless Scenario で既存 CI に照合
する」のうち、**headless に検証可能なスライス**を対象とする。

具体的には、driver-neutral 経路（`UiRuntime` +
`WindowDriver`/`DriverEvent`/`RoutedEvent`）を `FakeDriver`
で構成する Headless Scenario テストを
`crates/ruviform-ui-test/tests/window_job_headless.rs` に追加
し、既存テストが被覆していない不変条件を契約レベルで固定
する。あわせて、既存被覆との対応表と NotRun 残件を
`evidence/rf243/` に記録する。

本スライスは検証専用であり、production コード・公開 API・
CI workflow は変更しない。

## Why Now

Issue #243 の Windows 実 OS multiwindow smoke は先行スライス
（PR #551/#570）で実証済みだが、Issue 本文の現行受入には
「Window/Job 不変条件を Fake/Headless Scenario で既存 CI に
照合する」項目が残っている。既存テスト群
（`multi_window.rs`・`fake_driver.rs`・`driver_contract.rs`・
`ipc_conformance.rs`・`lifecycle_fixture.rs`・`e2e_process.rs`
等）は広範な被覆を持つが、以下の不変条件は個別テストとして
未固定だった:

- window-scoped job cancel が ownerless（Application/
  Project-owned 相当）job を巻き込まないこと
- owner window 破棄後も job が終端し、その terminal event・
  command receipt が記録済みの（stale な）`WindowId` 宛に
  routing され続け、delivery 層で安全に drop されること
- stale `WindowId` が自身の orphan job の cleanup にだけ
  到達し、別 window の job に触れないこと
- 破棄済み window 宛の inbound intent が dispatch に到達
  しないこと

これらを Fake/Headless（OS-L1 契約証拠）として既存 CI lane
で固定し、実 OS 受入（GUI-L2）・Native Dock 受入（EXT-L3）
との証拠階層を分離したまま記録する。

## Users and Value

- Synvelis（複数 Window/RT/Project）・AudioLens（単一
  Window/Job/CLI）の consumer が依存する Window/Job 所有権
  契約の、CI で再現可能な回帰防止。
- Issue #243 の受入判定・後続の macOS/Linux/Native Dock
  スライスが「headless で何が実測済みか」を正確に参照できる。

## Scope

### In Scope

- `crates/ruviform-ui-test/tests/window_job_headless.rs`
  （新規）: `UiRuntime` + `FakeDriver` を consumer 配線
  （`dummy_app`/`windowed_test_app` 相当: Action →
  `dispatch_window_intent` → `deliver_routed`、Tick →
  `drain_jobs_routed` → `deliver_routed`、CloseRequested →
  `respond_close`、Destroyed → consumer close policy）で
  構成する Headless Scenario テスト 4 件
- `evidence/rf243/` 配下の新規証拠: 既存被覆との対応表・
  実行コマンド・期待/観測・NotRun 残件と Open Gate 接続
- 本 docs チェーン（spec/design/plan/task/test-spec/ir/exp）

### Out of Scope

- 実 Window 起動・実入力・DPI/IME・実マウス/キーボード
  （GUI-L2。既存 real-OS lane の証拠を本スライスの Pass に
  転用しない）
- Native Plugin Dock 受入（EXT-L3。#388/#389・Synvelis
  #176 が owner）
- macOS/Linux native での実行（環境外。NotRun として記録）
- `UiRuntime`/`WindowDriver`/`FakeDriver`/scenario 基盤の
  振る舞い変更（契約の固定のみ。違反が見つかった場合は
  別 Issue）
- CI workflow・`mise.toml`・`Cargo.toml`/`Cargo.lock` の
  変更（既存 `cargo test -p ruviform-ui-test` lane をその
  まま利用）
- 物理手動入力・実 plugin/device 検証

## Requirements

- 追加テストは deterministic・non-RT で、実 thread の job
  worker（`demo.sleep`）のみを使い wall-clock 待ちは
  deadline 付き bounded loop に限定する。
- 「既存テストで既に被覆している不変条件は新規追加しない」
  — 被覆状況は対応表として `evidence/rf243/` に記録する。
- 実測と推測を区別する。実行していない検証を Passed と
  書かない。NotRun/Unsupported は理由つきで記録し、
  Pass に置き換えない。
- Fake/Headless の成功は OS-L1 契約証拠であり、Windows
  実 GUI（GUI-L2）・Native Dock（EXT-L3）の成功を意味
  しないことを証拠・docs 双方に明記する。
- 検証は `mise.toml` タスクを正本とし、cargo 直接実行は
  補助証拠として明記する。

## Acceptance Criteria

- [x] `window_job_headless.rs` の 4 テストが
  `mise run test ruviform-ui-test` で pass する
- [x] 2 Window ownership・close（veto/approve）・job cancel
  scoping・stale Window/Receipt の各不変条件について、
  既存被覆テストとの対応表と新規追加分の根拠を
  `evidence/rf243/` に記録した
- [x] NotRun 残件（実 Window/実入力/DPI/IME、macOS/Linux
  native、Native Plugin Dock）を理由つきで記録し、Issue
  #313/#388/#389・Synvelis #114/#176 等の Open Gate へ
  接続した
- [x] `mise run fmt-check` / `mise run check
  ruviform-ui-test` / `mise run lint ruviform-ui-test` /
  `mise run test ruviform-ui-test` が pass する

## Deferred Design Concerns

- Windows 実 GUI での同一不変条件の再確認: 既存 real-OS
  lane（`windowed_uitest`・`real_os_multiwindow_smoke`・
  `verdict-multiwindow-windows.json`）が担う。owner:
  Issue #243 GUI-L2 スライス。
- macOS/Linux native での同等検証: 本 dispatch の環境外。
  owner: 後続の #243 dispatch（OS 別）。CI Run URL の収集は
  Coordinator の Daily/手動 CI 確認に委ねる。
- Native Plugin Dock: #388/#389・Synvelis #176 が owner。
- `WindowLifecycle::Moved` の FakeDriver 側発火: FakeDriver
  は move 要求 API を持たず `Moved` は inject のみ。
  view move（Dock 2-phase）は `dock.rs`/`native_slot_
  contract.rs`/`driver_contract.rs` UI-DVR-05 が被覆済みで
  本スライスでは対応表記録に留める。

## Sources and Relations

- Issue #243 本文（現行受入の正本。Windows 実 OS
  multiwindow/preflight 16/16 は PR #551/#570 で実証済み）
- 先行スライス spec `docs/specs/issue-243-windows-
  multiwindow-smoke.md`・`issue-243-gpui-realwin-windows.md`
- Issue #181（multi-window 論理契約）・#182（dock-tree）・
  #313（real-OS smoke/capability）・#464（consumer-neutral
  scenario contract）・#386（driver-neutral `WindowDriver`/
  `DriverEvent`/`RoutedEvent` 語彙）・#456（job command
  receipt）
- 共有 lifecycle fixture `crates/ruviform-ui-test/src/
  lifecycle.rs`・`fixtures/scenarios/host/window-lifecycle.
  v1.json`
- `evidence/issue-243-multiwindow-verify/verdict-
  multiwindow-windows.json`（実 OS SendInput 証拠 —
  automation injection であり物理入力ではない）
