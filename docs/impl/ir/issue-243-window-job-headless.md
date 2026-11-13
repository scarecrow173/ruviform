---
id: "IMPL-034WZGD4u8P1g1M1fq3DI4"
type: "impl"
status: "completed"
title: "Issue #243 Window/Job 不変条件 Headless 検証 証拠"
created: "2026-11-13"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "docs/specs/issue-243-window-job-headless.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/window_job_headless.rs"
      - type: "file"
        path: "evidence/rf243/window-job-headless-coverage.md"
      - type: "file"
        path: "evidence/rf243/window-job-headless-verification.md"
      - type: "file"
        path: "docs/impl/ir/issue-243-window-job-headless.md"
      - type: "file"
        path: "docs/impl/exp/issue-243-window-job-headless.md"
    modified:
      - type: "file"
        path: "docs/specs/issue-243-window-job-headless.md"
      - type: "file"
        path: "docs/designs/issue-243-window-job-headless-design.md"
      - type: "file"
        path: "docs/plans/issue-243-window-job-headless-plan.md"
      - type: "file"
        path: "docs/tasks/issue-243-window-job-headless-task.md"
      - type: "file"
        path: "docs/test-specs/issue-243-window-job-headless.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements:
    - "docs/tasks/issue-243-window-job-headless-task.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-243-window-job-headless-task.md"
    - "docs/plans/issue-243-window-job-headless-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../ir/issue-243-multiwindow-closeout-slice.md"
    - "../ir/issue-243-windows-multiwindow-smoke.md"
    - "../ir/issue-243-gpui-realwin-windows.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/test-specs/issue-243-window-job-headless.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-243-window-job-headless.md"
  verified-by: []
  references:
    - "docs/specs/issue-243-window-job-headless.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #243 Window/Job 不変条件 Headless 検証 証拠

## Summary

Issue #243 残存受入のうち headless 可能なスライス
「2 Window の Ownership/Move/Close/Restore・Window-owned
対 App/Project-owned Job・stale Window/Receipt の
不変条件を Fake/Headless Scenario で既存 CI に照合する」
を実施した。production コードは変更せず、
`crates/ruviform-ui-test/tests/window_job_headless.rs`
（新規 integration test・Harness + 4 scenario）と
`evidence/rf243/` 証拠 2 件・docs チェーンで構成する。
既存被覆の棚卸しにより未固定だった不変条件
（window-scoped cancel が ownerless job に触れないこと、
owner 破棄後の terminal job event/command receipt の
stale WindowId routing、stale id 経由の orphan cleanup、
stale inbound Action の drop）のみを追加した。
Fake/Headless の Pass は OS-L1 契約証拠のみであり、
実 Window・実入力・Native Plugin Dock の受入
（GUI-L2/EXT-L3）は別 Gate のまま。

## Context

- task: `docs/tasks/issue-243-window-job-headless-task.md`
- plan: `docs/plans/issue-243-window-job-headless-plan.md`
- spec: `docs/specs/issue-243-window-job-headless.md`
- design: `docs/designs/issue-243-window-job-headless-design.md`
- test-spec: `docs/test-specs/issue-243-window-job-headless.md`
- 先行 slice: `docs/impl/ir/issue-243-multiwindow-closeout-slice.md`
  （Windows 実 OS multiwindow 16/16 pass — 本スライスとは
  別 Gate の証拠であり、本スライスの Pass 根拠には使わない）
- exp log: `docs/impl/exp/issue-243-window-job-headless.md`

## Implementation

### Changes

- `crates/ruviform-ui-test/tests/window_job_headless.rs`
  （新規）: `UiRuntime` + `FakeDriver` を本番 consumer
  配線（`dummy_app` parity: Action →
  `dispatch_window_intent` → `deliver_routed`、Tick →
  `drain_jobs_routed` → `deliver_routed`、CloseRequested →
  `respond_close`、Destroyed → opt-in `cancel_window_jobs`
  policy）で合成する Harness と 4 テスト。
  - `window_job_cancel_is_exact_and_spares_unowned_jobs`:
    `cancel_window_jobs(w1)` が `owner == Some(w1)` の job
    のみを signal し、w2 の job と `dispatch_op` 経由の
    ownerless job（App/Project-owned 相当）を cancel
    しないこと。ownerless job の terminal は
    `window: None` のまま。
  - `close_policy_cancels_only_the_closed_windows_jobs`:
    `CloseRequested` Veto で window/job が生存、Approve +
    consumer policy で `Destroyed` 後に閉じた window の
    job のみが cancel されること。sibling window は
    継続して自身宛 intent に応答。
  - `job_outliving_its_window_routes_terminal_to_the_dead_
    window_only`: owner destroy 後も job が終端し、
    terminal `job` event と command receipt が記録済みの
    stale `WindowId` 宛に routing され続け、delivery 層
    （`post_event`/`delivered`）が dead-window 宛を drop
    し sibling へ leak しないこと。stale `Action` は
    result を産出しない。`WindowId` 非再利用。
  - `cancel_window_jobs_reaches_orphans_via_stale_window_
    id`: 破棄済み window の stale id を `cancel_window_jobs`
    に渡すと、その id に記録された orphan job のみが
    cancel され、別 window の job は生存すること。
    `destroy_window` 再呼出は error、新規 window は新 id。
- `evidence/rf243/window-job-headless-coverage.md`（新規）:
  受入不変条件 × 既存被覆テスト（`multi_window.rs`/
  `fake_driver.rs`/`driver_contract.rs`/
  `lifecycle_fixture.rs`/`e2e_process.rs`/
  `ipc_conformance.rs`/`dock.rs`/`native_slot_contract.rs`/
  `driver_parity.rs`/`scenario_contract.rs`）の対応表と
  本スライスで追加した 4 テストの位置づけ。
- `evidence/rf243/window-job-headless-verification.md`（新規）:
  実行コマンド・OS・toolchain・exit code・期待/観測・
  NotRun/Deferred 残件と Open Gate（#313/#388/#389/
  Synvelis #114/#176）への接続。
- docs チェーン 6 文書（spec/design/plan/task/test-spec/
  本 IR/exp log）と管理 index 再生成。

### Decisions During Implementation

- **合成は in-process integration test を選択**: 共有
  fixture（`window-lifecycle.v1.json`）や `HeadlessIo`/
  `TestApp` を変更せず、`dummy_app`/`windowed_test_app`
  と同じ consumer 配線をテスト内 Harness に写した。fixture
  拡張は共有資産の変更を伴うため別 Issue 候補とし、本
  スライスでは行わない。
- **routing と delivery を別々に観測**: Harness は
  delivery 前の `RoutedEvent` を journal 記録するため、
  「runtime が stale id 宛に産出し続ける」ことと
  「delivery 層が dead window 宛を drop する」ことを
  独立に assert できる。既存 `multi_window.rs` の
  `delivery_to_dead_window_is_safe` は delivery drop のみ
  を固定しており、terminal routing の stale-id 維持は
  本テストが初めて固定した。
- **「App/Project-owned」の runtime 語彙**: runtime に
  project 概念はなく、window 非紐付け job =
  `owner: None` が同一語彙。consumer 側の policy 区別は
  close-policy テストで被覆し、対応関係は spec/design/
  test-spec/evidence で明示した。
- **doc graph router は使用せず個別 skill で scaffold**:
  repo-wide の既存 broken relation により router が
  blocked を返すため、`new_*.js` 系 script と手動
  front matter で文書を作成した。制約は PR 本文に記録。
- **exp log は `.md` 形式**: task の編集許可範囲が
  `docs/impl/exp/issue-243-window-job-headless.md` のみ
  のため、`issue-389-parent-lease.md` 先例に倣い prose
  検証記録とし、`.jsonl` イベントログは作成していない。

## Related Experiments

### Adopted

<!-- 実験的分岐なし（検証スライス。exp log は検証記録として
`docs/impl/exp/issue-243-window-job-headless.md` に記録） -->

### Rejected

<!-- なし -->

## Validation

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | `cargo fmt --all -- --check` clean |
| `mise run check ruviform-ui-test` | 0 | Pass |
| `mise run lint ruviform-ui-test` | 0 | clippy `-D warnings` clean |
| `mise run test ruviform-ui-test` | 0 | 全 test target pass（新規 `window_job_headless` 4 件含む） |

補助証拠: `cargo test --locked -p ruviform-ui-test --test
window_job_headless` で 4 tests / 0 failed / 約 0.22s
（canonical = mise の結果を補助する直接 cargo 実行）。
実行環境: Windows 11 x86_64、mise 管理 toolchain。
詳細は `evidence/rf243/window-job-headless-verification.md`。

## Risks

- job worker thread の timing 依存: `demo.sleep` の cancel
  観測間隔（10ms）と deadline 10s の bounded drain で
  吸収。固定 sleep には依存しない設計だが、極端に遅い
  環境では deadline 超過の可能性は残る（その場合は
  flaky ではなく環境要因として Fail を正直に記録する）。
- 「ownerless = App/Project-owned」の対応は runtime 語彙
  上の近似であり、consumer が project 概念を別途持つ場合の
  policy は consumer 責務（本スライスでは DummyApp parity
  のみ検証）。
- Fake/Headless Pass は OS-L1 契約証拠のみ。実 Window・
  実入力・Native Dock の受入（GUI-L2/EXT-L3）を構成
  しない — NotRun/Deferred は evidence と PR 本文に
  明示。

## Follow-ups

- Windows 実 OS multiwindow smoke・GPUI real-window lane の
  同一不変条件照合 — 既存 real-OS lane（#313 系）の証拠を
  参照し、本スライスの結果と混同しない。
- macOS/Linux native 実行・動的 DPI/IME・Native Plugin
  Dock 受入 — #313/#388/#389・Synvelis #114/#176 の
  Open Gate が owner。
- `HeadlessIo`/共有 fixture 経由の scenario 化
  （`window-lifecycle.v1.json` 拡張）は共有資産変更を
  伴うため別 Issue 候補。
