---
id: "SPEC-034YJ09qxi0m5ujyYTjO3u"
type: "spec"
status: "approved"
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
    - "docs/discovery/issue-631-window-surface.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 RealOsGuiTest window-close/DPI 観測面

## What Should Be Built

`RealOsGuiTest`（`real-scenario` feature）に、実 OS window の
window-close lifecycle と scale-factor/DPI・geometry を観測する公開面を
追加する。close protocol は driver-neutral な
`WindowDriver::request_close`/`respond_close` を adapter method として
転送し、`WindowLifecycle::CloseRequested`→`Destroyed`（`Veto`/`Defer`/
未回答時既定 `Approve` の全経路）を観測する。DPI/geometry は live な
`OsWindowGeometry`（原点・物理 client サイズ・scale factor）と
`Resized`/`Moved`/`ScaleFactorChanged` の physical-pixel 記録を公開する。
あわせて `tests/real_window_lifecycle.rs` で実 window 検証し、
`evidence/rf631/real-window-surface.jsonl` へ実走 evidence を記録する。

## Why Now

Issue #631 AC1 の残行 `real-window-close` / `real-window-dpi` を判定化
（catalog row flip）する前提として、`RealOsGuiTest` の観測面が必要。
`WindowDriver` の close 契約と `window_geometry` は driver 層に実装済み
だが、scenario adapter の公開面・観測 record・実走検証がなかった。
本 slice はその観測面のみを対象とし、判定化・catalog flip は後続 slice
に残す。

## Users and Value

- GUI Test Driver / Scenario Runner の consumer — driver-neutral な
  観測 API 経由で close/DPI 挙動を case 判定できる。
- 後続 slice（catalog row flip・plugin-dock evidence 再生成）の実装者
  — 本 slice の観測面と evidence 形式をそのまま利用できる。

## Scope

### In Scope

- `WindowLifecycleEvent` — app thread event handler が記録する
  観測用 lifecycle/geometry 遷移（`Created`/`Ready`/`Resized`/`Moved`/
  `Focused`/`ScaleFactorChanged`/`CloseRequested`/`Destroyed`）。
  `#[non_exhaustive]`・`Copy`・`PartialEq`。`Action` intent は含めず
  既存 `drain_intents` channel に残す。
- `RealOsGuiTest::window_events(window)` — session log の非破壊
  snapshot（destroyed window の terminal `Destroyed` も観測可能）。
- `RealOsGuiTest::request_close(window)` /
  `respond_close(window, decision)` — `WindowDriver` close op の
  `UiJob` seam 転送。
- `RealOsGuiTest::arm_close_response(window, decision)` — handler が
  `CloseRequested` の同期 flush 内で消費する decision queue。
  `Veto`/`Defer` が driver の未回答 `Approve` 既定に競合しないための
  唯一のタイムリー経路。
- `RealOsGuiTest::window_geometry(window)` — live OS client geometry
  （`OsWindowGeometry`、physical px + scale factor）。
- `tests/real_window_lifecycle.rs` — 2 test（close lifecycle /
  DPI geometry）を実 GPUI window/session で検証。
- `evidence/rf631/real-window-surface.jsonl` — `rf631-real-window-
  surface/v1` schema の run/case evidence 行。
- `lib.rs` の re-export 1 行（Coordinator 承認済み）。

### Out of Scope

- `real-window-close`/`real-window-dpi` の catalog row flip
  （`crates/ruviform-plugin-dock-test/` — 別 Worker 占有）。
- pixel Pass（#532 gate — `real-dcomp-pixel` verdict=fail を維持）。
- resize/redock/input/focus/IME verb（#389 領域）。
- consumer 投影（Synvelis/AudioLens 実 GUI — synvelis#114/#176・
  audio-lens#127 gate）。
- DPI 変更・window move 等の変更 API。
- `WindowLifecycle`/`CloseDecision` の `ruviform-ui` 側 contract 変更。

## Requirements

- 観測は app thread の event handler / `UiJob`（`App` context 内）から
  のみ行い、caller 側で lifecycle・geometry・event を合成しない。
- `Veto`/`Defer` は `CloseRequested` flush 内の armed decision でのみ
  適用され、未回答 request は driver 既定どおり当該 window のみ
  `Approve` される。
- `window_events` は append-only の session log で、window 破棄後も
  terminal `Destroyed` を含めて読める。`Action` intent は含まない。
- `window_geometry` は live OS handle + scale factor の実測値を返し、
  unknown/not-open/handle 無し/backend 無しは honest error
  （`ScenarioError` 経由の `UiError` 写像）。
- 公開 enum `WindowLifecycleEvent` は `#[non_exhaustive]`。
- GPUI 型を consumer/公開 API boundary へ漏らさない
  （`check-ui-driver-boundary` green 維持、全公開面は
  `real-scenario` feature 内）。
- 実デスクトップ不在・session 開始失敗は `environment_blocked` /
  `NotRun` を正直に記録し、Required case を偽 Pass にしない。
- evidence は `RUVIFORM_RF631_SURFACE_EVIDENCE` env gate で
  JSONL append、run/case row に repo SHA・OS・実測値を含める。

## Acceptance Criteria

- [ ] `WindowLifecycleEvent` と `window_events`/`request_close`/
  `respond_close`/`arm_close_response`/`window_geometry` が
  `real-scenario` feature で compile・公開される。
- [ ] 実 window で `CloseRequested`→`Destroyed` の全経路
  （veto/defer/deferred-approve/default-approve）・terminal id
  退役・sibling 分離が観測・assert される。
- [ ] live `OsWindowGeometry`（physical px + scale factor）と
  lifecycle `Resized` 記録が実測で一致し、dead window の geometry
  query が honest error になる。
- [ ] `cargo test -p ruviform-ui-gpui --features real-scenario
  --test real_window_lifecycle` が interactive desktop で Pass し、
  `evidence/rf631/real-window-surface.jsonl` に run/case 行が記録
  される。
- [ ] 非回帰 lane（`test-ui-real-scenario` 系・`check`/`lint`/`test`/
  `fmt-check`・`check-ui-driver-boundary`・`test-ui-scenario-contract`）
  が green。

## Deferred Design Concerns

- catalog row flip・plugin-dock evidence JSONL 再生成 — 後続 slice
  （本 slice は観測面のみ）| 後続 slice 着手時 | row が Unsupported
  のまま残る。
- DPI 変更（mixed-monitor move）・window move/resize verb — verb
  不在（#389 領域）| #389 verb 拡張後 | DPI 変更経路が未検証のまま。
- pixel Pass・consumer 投影 — #532 / synvelis#114/#176 / audio-lens#127
  gate | 各 gate 解消時 | 判定が先行して Pass を主張するリスク。

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — auto-close しない）。
- `docs/discovery/issue-631-window-surface.md` — 探索記録・実走で
  判明した sentinel window 要件。
- 前 slice spec `docs/specs/issue-631-real-window-dock.md` 他
  `issue-631-*` 系列。
- `POLICIES.md`（検証レベル・Pass/Fail/NotRun/Unsupported 規範・
  fail-closed）。
- `WindowDriver`/`WindowLifecycle`/`CloseDecision`（`ruviform-ui`）、
  `GpuiDriver::window_geometry`/`OsWindowGeometry`（`capture.rs`）、
  `examples/real_window_lifecycle.rs`（handler 内 respond_close 先例）、
  `test-ui-real-scenario` mise task。
