---
id: "IMPL-034YJ5ZPjma7y6yufrmtYd"
type: "impl"
status: "completed"
title: "Issue #631 RealOsGuiTest window-close/DPI 観測面"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-631-window-surface.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-631-window-surface.md"
    rejected: []
---

# Issue #631 RealOsGuiTest window-close/DPI 観測面

## Summary

`RealOsGuiTest`（`real-scenario` feature）に window-close lifecycle と
DPI/geometry の観測面を追加した。app thread の `WindowLifecycle` event
handler が全 window の lifecycle/geometry 遷移を session 内 append-only
log に記録し、`window_events` で公開する。`request_close` /
`respond_close` / `arm_close_response` / `window_geometry` を `UiJob`
seam 経由で公開し、close の全判定経路（Veto/Defer/deferred-Approve/
default-Approve）と live OS geometry を実 window で観測・検証した。
実 Windows desktop で 2 case が Pass し、`evidence/rf631/
real-window-surface.jsonl` に verdict=pass の実測行を記録した。

## Context

`docs/tasks/issue-631-window-surface.md`（`normal-plan-task`）に従う。
上流: `docs/plans/issue-631-window-surface.md`（approved）、
`docs/specs/issue-631-window-surface.md`（approved）、
`docs/designs/issue-631-window-surface.md`（approved）、
`docs/test-specs/issue-631-window-surface.md`（approved）。
Issue `scarecrow173/ruviform#631` の AC1 残行
（`real-window-close` / `real-window-dpi`）の判定化に先立つ観測面の
slice。catalog row flip・plugin-dock evidence 再生成・consumer 投影は
後続 slice / gate。

## Implementation

### Changes

- `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs`
  - `WindowLifecycleEvent`（`#[non_exhaustive]`、`Copy`、`Clone`、
    `PartialEq`）を追加。`WindowLifecycle` から `Action(Intent)` を
    除く全 variant を写像（`Created`/`Ready`/`Resized`/`Moved`/
    `Focused`/`ScaleFactorChanged`/`CloseRequested`/`Destroyed`）。
  - `RealOsGuiTest` に `lifecycle_events: Arc<Mutex<Vec<(WindowId,
    WindowLifecycleEvent)>>>` と `close_responses: Arc<Mutex<
    BTreeMap<WindowId, VecDeque<CloseDecision>>>>` を追加。
  - `run()` の event handler で `Windowed` lifecycle を観測 log へ
    append。`CloseRequested` では armed queue の先頭を消費し、
    存在すれば `drv.respond_close` を同期 flush 内に呼ぶ。
  - public methods: `window_events(window)`（append-only snapshot、
    destroyed 後も terminal を保持）、`request_close(window)`、
    `respond_close(window, decision)`、`arm_close_response(window,
    decision)`（caller thread から直接 queue へ push、UI round-trip
    不要）、`window_geometry(window)`。
- `crates/ruviform-ui-gpui/src/lib.rs`
  - `WindowLifecycleEvent` を `real-scenario` feature の re-export に
    追加（Coordinator 承認済み、1 行）。
- `crates/ruviform-ui-gpui/tests/real_window_lifecycle.rs`（新規、
  `#![cfg(all(any(windows, target_os = "linux"), feature =
  "real-scenario"))]`）
  - `window_close_lifecycle`: sentinel + target A/B の 3 window
    session。A に armed `Veto` → `request_close` → `CloseRequested`
    観測・`Destroyed` なし。armed `Defer` → pending 維持 →
    `respond_close(Approve)` → `Destroyed` + id 退役。B に armed
    なし `request_close` → default `Approve` → `Destroyed`。
    sibling/sentinel の非干渉を assert。
  - `window_dpi_geometry`: 実 window の live `OsWindowGeometry`
    （origin/物理 size/scale factor）を繰り返し読み安定性を確認、
    lifecycle `Resized` の物理 pixel と一致を assert、破棄後の
    `window_geometry` が honest error、を検証。
  - `RUVIFORM_RF631_SURFACE_EVIDENCE` env gate で
    `evidence/rf631/real-window-surface.jsonl` に run/case 行を emit。
    session 開始失敗は `environment_blocked`、body 内 Driver error
    は `fail` として正直に記録。
- `evidence/rf631/real-window-surface.jsonl`（新規）: 実走 2 case の
  verdict=pass 行（schema `rf631-real-window-surface/v1`）。

### Decisions During Implementation

- **armed queue の消費位置**: `request_close` は `CloseRequested` の
  emit と同じ同期 flush 内で unanswered を default `Approve` する。
  caller 側の `UiJob` で後追い `respond_close` しても必ず負けるため、
  armed `CloseDecision` を event handler 内で消費する設計にした
  （`examples/real_window_lifecycle.rs` の既存 veto 経路と同型）。
- **観測 log の分離**: `WindowLifecycleEvent` は intent を含まない
  `Windowed` 専用 enum とし、`Action(Intent)` は既存 intents channel
  に残す。観測面の責務を lifecycle/geometry 遷移に限定した。
- **sentinel window（実走で発見した根本原因）**: 最終 window を
  destroy すると GPUI platform loop が終了し UiJob pump が死ぬため、
  破棄後の `settle_until`/`window_geometry` が timeout した
  （serial 実行でも再現、baseline story test は pass — 並列干渉説を
  否定）。target 破棄中も sentinel を生存させる構成に修正し、
  flaky ではなくアーキテクチャ上の必須条件として test-spec に記録。
- **`arm_close_response` は `&self`**: 共有 `Arc<Mutex<…>>` のみを
  触り UI round-trip を伴わないため、`window_focused` 等の local
  query API と同じ receiver 規約に揃えた。

## Related Experiments

### Adopted

<!-- No experiment log entries adopted. -->

### Rejected

<!-- The parallel-execution interference hypothesis was rejected by
serial reproduction; see Validation. -->

## Validation

- `mise run fmt-check` — pass（初回 diff を `cargo fmt` で修正後）。
- `mise run check ruviform-ui-gpui --features real-scenario` — pass。
- `mise run lint ruviform-ui-gpui --features real-scenario` — pass。
  初回は `clippy::too_many_lines`（`run()` 108/100）を指摘され、
  event handler を `window_event_handler` 自由関数へ抽出して解消。
  test file 側も `needless_pass_by_value`/`struct_field_names`/
  `too_many_lines`/`unnecessary_wraps` を helper 分割・参照化・
  配列化で解消（`#[allow]` 不使用）。
- `mise run test ruviform-ui-gpui --features real-scenario` — pass
  （全 target。`tests/real_window_lifecycle.rs` 2 passed を含む）。
  なお初回実行では既存 `real_os_input_scenario` が
  `NotRun — window w1 is not the focused window` で fail したが、
  単独再実行と lane 再実行で pass — 並列 real-window test の OS
  focus 競合による既知の環境 flake で本変更起因ではない。
- `mise run test ruviform-ui-gpui` — pass。
- `mise run test-ui-scenario-contract` — pass（7 tests）。
- `mise run check-ui-driver-boundary` — pass
  （dep-graph/signature/allowlist 全 pass — GPUI 型の境界非漏出を
  維持）。
- `mise run test-ui-real-scenario` — pass（3 tests、interactive
  Windows desktop。`evidence/issue-314-real-scenario/` の
  driver-lab/golden JSONL へ lane 証拠を追記）。
- `cargo test -p ruviform-ui-gpui --features real-scenario --test
  real_window_lifecycle`（補助証拠）— 実 Windows desktop で
  2 passed / 0 failed、exit 0。`RUVIFORM_RF631_SURFACE_EVIDENCE`
  指定で `evidence/rf631/real-window-surface.jsonl` に
  verdict=pass の run/case 行を追記（最終コードで再実行済み）。
- 実走で観測した環境ノイズ（Win32 invalid-HWND
  `HRESULT(0x80070578)`、`RefCell already borrowed` ログ）は
  baseline 既存テストでも同様に出る既知の環境ノイズであり、本変更
  起因ではない（baseline story test が同環境で pass）。

## Risks

- sentinel window を外すと最終破棄後の観測が pump 死亡で timeout
  する — test-spec/design に構造的必須条件として記録済み。
- DPI 変更（mixed-monitor move）・window move/resize verb・IME・
  OS 起点 close は本 slice 対象外。`real-window-close`/
  `real-window-dpi` の catalog row flip は後続 slice。
- pixel Pass は #532 gate（`real-dcomp-pixel` verdict=fail を正直に
  保持）。本 slice は pixel capture に踏み込まない。

## Follow-ups

- 後続 slice: `crates/ruviform-plugin-dock-test/` の
  `real-window-close`/`real-window-dpi` catalog row flip と
  plugin-dock evidence JSONL 再生成。
- #389 領域: resize/redock/input/focus/IME verb。
- consumer 投影: synvelis#114/#176・audio-lens#127 gate。
- docs index（`docs/**/README.md`）は Coordinator が統合時に再生成。
