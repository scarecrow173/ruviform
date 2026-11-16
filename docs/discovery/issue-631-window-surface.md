---
id: "DISC-034YJ08C2FLjqUQpNN0PcG"
type: "discovery"
status: "draft"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 RealOsGuiTest window-close/DPI 観測面

## Summary

Issue #631 AC1 残行 `real-window-close` / `real-window-dpi` の判定化に先立ち、
`RealOsGuiTest` が実 OS window の close lifecycle と物理 geometry/DPI を
観測する公開面を `ruviform-ui-gpui`（`real-scenario` feature）へ追加する
slice の探索記録。driver-neutral な `WindowDriver::request_close`/
`respond_close` 契約と `WindowLifecycle` イベントは実装済みで、本 slice は
それらを scenario adapter の観測面として公開し、実 window で検証する。
結論: 観測面は app thread の event handler が記録する
`WindowLifecycleEvent` 列 + `UiJob` seam 経由の `window_geometry` で
実現可能（confirmed — 実装・実走済み）。

## Exploration Goal

`RealOsGuiTest`（`ScenarioDriver::RealOs` の
`SemanticGuiTest` + `VisualGuiTest` adapter）に対し、次を実 OS window で
観測・検証する最小の公開面を確定する。

- window close: `CloseRequested` → `Destroyed` の全判定経路
  （`Veto` / `Defer` / `Approve` / 未回答時の既定 `Approve`）。
- DPI/geometry: live な `OsWindowGeometry`（原点・物理 client サイズ・
  scale factor）と `Resized`/`Moved`/`ScaleFactorChanged` の
  physical-pixel 記録、破棄後の honest error。

## Key Issues and Assumptions

- `Veto`/`Defer` は `CloseRequested` の同期 flush 内でしか間に合わない
  — driver は未回答を同じ drain 内で `Approve` 既定にするため、
  caller 側の `respond_close` UiJob は常に競合に負ける（confirmed:
  `lib.rs` `Op::RequestClose` / `on_should_close` の実装で確認）。
- lifecycle イベントは app thread の `flush_events` が handler へ同期配送
  する — `windows()` などの派生状態から推測してはならない（confirmed）。
- GPUI platform loop は最後の window が閉じると終了し、`UiJob` pump が
  死ぬ — 最終 window 破棄後の観測は sentinel window を残して行う必要が
  ある（confirmed — 実走で timeout を再現・修正済み）。
- `lib.rs` への公開 re-export は当初 scope 外だったが、Coordinator 承認で
  `WindowLifecycleEvent` 1 行のみ追加が許可された（confirmed）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| handler が `WindowLifecycle` をそのまま公開型として記録 | 変換不要 | `WindowLifecycle` は `Action(Intent)` を含み観測面として重い。`WindowId` キーの外部 Mutex 共有が必要 | no |
| 専用 `WindowLifecycleEvent`（`Action` 除外・`Copy`）を handler が append | 軽量・assert しやすい・`Action` は既存 intents channel に残る | `WindowLifecycle` への写像関数が要る | yes |
| caller 側 `respond_close` のみで veto/defer を制御 | API が単純 | 同期 flush に間に合わず既定 Approve に必ず負ける — 不可能 | no |
| armed queue（`arm_close_response`）を handler が flush 内で消費 | 唯一タイムリーに veto/defer を届けられる | armed だが未消費の decision が残りうる（window が close しない場合） | yes |
| `window_geometry` を `UiJob` 経由で都度 query | live OS handle・scale factor の実測値 | UiJob 1 往復のコスト（settle 予算内で問題なし） | yes |

## Tentative Conclusions and Hypotheses

- confirmed: `WindowLifecycleEvent`（`#[non_exhaustive]`・`Copy`）を
  `real_window.rs` に追加し、app thread handler が全 `Windowed`
  lifecycle 遷移を session log へ append する設計で要件を満たす。
- confirmed: `arm_close_response(window, decision)` が handler の
  `CloseRequested` 分岐へ queued decision を供給し、`Veto`/`Defer`/
  `Approve` を同期 flush 内で `drv.respond_close` へ転送できる。
- confirmed: `window_geometry`/`request_close`/`respond_close` は
  既存 `UiJob`/`call` seam 経由の method として公開でき、
  `ScenarioError`/`UiError` の honest error をそのまま維持できる。
- confirmed: 実走で `window-close-lifecycle` / `window-dpi-geometry`
  の 2 case が Pass（`evidence/rf631/real-window-surface.jsonl`）。

## Open Questions

- なし — slice の観測面は確定・実装・実走済み。catalog row flip
  （`real-window-close`/`real-window-dpi` の判定化）は後続 slice。

## Promotion Candidates

- [x] spec-doc needed: `docs/specs/issue-631-window-surface.md` —
  観測面の公開 contract と honest-evidence 要件を確定した。
- [ ] adr-doc needed: なし — 既存 `WindowDriver`/`WindowLifecycle`
  契約の範囲内で、新規の architecture decision は発生しなかった。

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — auto-close しない）。
- `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs` —
  `RealOsGuiTest::run` の app thread spawn・`UiJob`/`call` seam・
  既存 slot ops の先例。
- `crates/ruviform-ui-gpui/src/lib.rs` — `WindowDriver`
  `request_close`/`respond_close`/`destroy_window`、`Op::RequestClose`/
  `Op::Destroy` drain、`on_should_close`、`flush_events`、
  `emit_window_state`（physical-pixel 変換）。
- `crates/ruviform-ui-gpui/src/gpui_driver/capture.rs` —
  `GpuiDriver::window_geometry`/`capture_window` と Win32
  `client_geometry` backend。
- `crates/ruviform-ui-gpui/examples/real_window_lifecycle.rs` —
  handler 内 `respond_close` の先例。
- `POLICIES.md` — GUI-L2・Pass/Fail/NotRun/Unsupported・fail-closed
  規範。

## Evidence

### 2026-11-16 — close/DPI 観測面の実装と実走

- `WindowLifecycleEvent` を `real_window.rs` に追加し `lib.rs` で
  re-export（Coordinator 承認済みの 1 行）。
- `tests/real_window_lifecycle.rs` を新設 — 実 GPUI window/session で
  veto/defer/deferred-approve/default-approve・terminal id 退役・
  sibling/sentinel 分離・live geometry・dead-window error を検証。
- 実走で判明した構造的事実: GPUI platform loop は最後の window が
  閉じると終了し `UiJob` pump が死ぬ。最終破棄後の観測は sentinel
  window を残す設計で決定的にした（当初の 2 window 構成は最終
  `settle_until` が 15s timeout — sentinel 追加で 2 test が
  serial/parallel ともに安定 Pass）。
- `evidence/rf631/real-window-surface.jsonl` — `window-close-lifecycle`
  /`window-dpi-geometry` とも verdict=pass、実測 geometry
  `{origin:(120,132), 1280x800 physical px, scale_factor 1.0}`、
  `Resized` 記録 `[1280,800]` と一致。
