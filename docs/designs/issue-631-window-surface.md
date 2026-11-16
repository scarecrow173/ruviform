---
id: "DESIGN-034YJ0ge8DoTURTLqPKx6J"
type: "design"
status: "approved"
title: "Issue #631 RealOsGuiTest window-close/DPI 観測面 設計"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-window-surface.md"
    - "docs/discovery/issue-631-window-surface.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 RealOsGuiTest window-close/DPI 観測面 設計

## Context

`RealOsGuiTest::run` は caller thread と GPUI app thread を `UiJob`
channel で繋ぐ session adapter。window op は `GpuiDriver` が
`inner.ops` queue 経由で app thread の `drain` 内に適用され、
`DriverEvent` は app thread の `flush_events` が handler へ同期配送
する。`WindowDriver` の close 契約（`request_close` →
`CloseRequested` → 未回答は既定 `Approve` → `Destroyed`、
`respond_close` で `Veto`/`Defer`/`Approve`）と `window_geometry`
（live OS handle + scale factor の実測 client rect）は driver 層に
実装済みだが、scenario adapter の観測面・記録・検証がなかった。

本設計はその観測面を `real-scenario` feature 内に閉じて追加する。

## Scope

### In Scope

- `WindowLifecycleEvent` — `WindowLifecycle` の観測用写像
  （`Action` 除外、`Copy`/`PartialEq`/`#[non_exhaustive]`）。
- session log `events: Arc<Mutex<Vec<(WindowId, WindowLifecycleEvent)>>>`
  — handler が全 `Windowed` lifecycle 遷移を append。
- `close_responses: Arc<Mutex<BTreeMap<WindowId, VecDeque<CloseDecision>>>>`
  — handler が `CloseRequested` flush 内で消費する armed queue。
- public methods: `window_events` / `request_close` / `respond_close` /
  `arm_close_response` / `window_geometry`。
- `tests/real_window_lifecycle.rs` — 実 window 検証 2 case。
- evidence JSONL（`RUVIFORM_RF631_SURFACE_EVIDENCE` env gate）。

### Out of Scope

- catalog row flip・plugin-dock-test 側の判定化・evidence 再生成。
- `WindowLifecycle`/`CloseDecision`/`WindowDriver` contract 自体の変更
  （`ruviform-ui` 無変更）。
- OS close（user が window の X を押す経路）の scenario 駆動 —
  `on_should_close` は同じ veto-able 契約を通るが、本 slice は
  `request_close` 駆動のみを検証する。
- DPI 変更・window move/resize verb・IME・consumer 投影。

## Components and Boundaries

- `real_window.rs` `WindowLifecycleEvent` + `observe()` —
  `WindowLifecycle` → 観測 record の純粋写像。`Action` と将来 variant
  は `None`（観測対象外を明示）。
- `RealOsGuiTest::run` の app thread handler — 既存の
  `Action`→intents / `Focused`→focused map に加え、全 lifecycle を
  `events` log へ append し、`CloseRequested` で armed queue を消費して
  `drv.respond_close` を同期 flush 内に呼ぶ。
- `RealOsGuiTest` public methods — `window_events` は local Mutex read
  のみ（UiJob 不要・destroyed 後も読める）。`request_close`/
  `respond_close`/`window_geometry` は既存 `call`/`UiJob` seam 経由。
  `arm_close_response` は local Mutex write のみ（UiJob 不要 —
  handler が読む共有状態への供給であり、window op ではない）。
- `lib.rs` — `WindowLifecycleEvent` の re-export 1 行のみ
  （Coordinator 承認済み）。driver internals は無変更。
- `tests/real_window_lifecycle.rs` — sentinel window を含む実
  window session で全経路を検証。

## Data and Control Flow

close 判定のタイミングが本設計の核心:

1. caller: `arm_close_response(w, Veto)` → local queue に積む
   （UiJob 不要・即時）。
2. caller: `request_close(w)` → UiJob → `Op::RequestClose` queue。
3. app thread drain: `CloseRequested` を `inner.events` へ push →
   `flush_events` が handler を同期呼出 → handler が `events` log に
   `CloseRequested` を記録 → armed queue から `Veto` を pop →
   `drv.respond_close(w, Veto)` → `close.pending=false`。
4. flush 復帰後、driver が `awaiting_response` を確認 —
   `Veto` 済みなら `approved=false` → window は開いたまま。
   未回答なら `approved=true` → `apply_destroy`。
5. `respond_close(Approve)`（defer 後の回答）は UiJob 経由で
   `Op::Destroy` を queue → 次 drain で `Destroyed`。

geometry/DPI:

- `window_geometry(w)` → UiJob → `GpuiDriver::window_geometry` →
  `inner` の slot（`handle`/`raw_parent`/`scale_factor`）→ Win32
  `GetClientRect`+`ClientToScreen`（Linux は X11 `GetImage` 系 backend）
  → `OsWindowGeometry`（physical px）。
- `Resized`/`Moved`/`ScaleFactorChanged` は `AppRoot` の
  `observe_window_bounds` → `emit_window_state` が physical px で
  emit → handler が `WindowLifecycleEvent` へ写像して log に記録。

sentinel window（test 側の構造的要件）:

- GPUI platform loop は最後の window が閉じると終了し `UiJob` pump が
  死ぬ。最終破棄を観測する test は sentinel window を残し、session
  teardown の `cx.quit` job で閉じる。

## Non-Functional Constraints

- RT 境界: 全追加は non-RT（UI thread / caller thread の Mutex のみ、
  RT audio path に触れない）。
- honesty: lifecycle・geometry・event を一切合成しない。armed queue
  の未消費 decision は window が close しなければ残るだけで、他
  window へ漏れない（per-window queue）。
- driver boundary: 公開面は backend-neutral 型のみ
  （`WindowId`/`WindowLifecycleEvent`/`OsWindowGeometry`/
  `CloseDecision`/`ScenarioError`）。GPUI 型は `real-scenario`
  feature 内に閉じる。
- `#[non_exhaustive]` on `WindowLifecycleEvent`（公開 enum policy）。

## Risks and Trade-offs

- armed queue の decision が消費されず残る（window が close されない
  まま session 終了）→ 害はない: per-window queue で他 window に
  漏れず、session と共に破棄される。doc comment で明示。
- `window_events` が session 全履歴を保持 → session 寿命内のみ・
  window 数は少数のため問題なし。破棄後も append-only で terminal
  `Destroyed` を保持するのが意図的設計。
- handler 内 `drv.respond_close` が `inner.borrow_mut()` を取る →
  `flush_events` は handler 呼出中 `inner` を借用しない設計
  （handler を take してから呼ぶ）のため安全。`RefCell` 二重借用は
  発生しない（実装確認済み）。
- sentinel window なしで最終 window を破棄する test は pump 死亡で
  timeout する（実走で再現）→ test 設計で sentinel を必須化。

## Deferred Design Concerns

- OS 起点 close（user の window X）の scenario 駆動 — `on_should_close`
  は同契約を通るが、外部 WM_CLOSE 注入は本 slice 対象外 | 後続 slice
  | 実 OS 入力 lane 整備時 | OS 経路の veto が未検証のまま。
- `WindowLifecycleEvent` への将来 variant 追加（例: `Minimized`）—
  `#[non_exhaustive]` で後方互換を確保 | 必要時 | variant 追加時の
  observer 更新コスト。

## Verification Notes

- [x] `cargo check --locked --all-targets -p ruviform-ui-gpui
  --features real-scenario` 通過。
- [x] `cargo test -p ruviform-ui-gpui --features real-scenario
  --test real_window_lifecycle` が interactive Windows desktop で
  2 test Pass（serial/parallel とも）。
- [x] `evidence/rf631/real-window-surface.jsonl` に pass 行を記録。
