---
id: "TASK-034YJ3y9Bg5CeSmMMjs3g1"
type: "task"
status: "completed"
title: "Issue #631 T1 — RealOsGuiTest window-close/DPI 観測面"
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
    - "docs/plans/issue-631-window-surface.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-631-window-surface.md"
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

# Issue #631 T1 — RealOsGuiTest window-close/DPI 観測面

## Work

`RealOsGuiTest` に window-close lifecycle と DPI/geometry の観測面を
追加し、実 OS window で検証する slice。

1. `real_window.rs`: `WindowLifecycleEvent`（`#[non_exhaustive]`・
   `Copy`・`PartialEq`）を追加し、app thread event handler が全
   `Windowed` lifecycle 遷移を session log（`Arc<Mutex<Vec<(WindowId,
   WindowLifecycleEvent)>>>`）へ append する。`CloseRequested` 分岐で
   armed queue（`close_responses`）を消費し `drv.respond_close` を
   同期 flush 内に呼ぶ。public methods `window_events` /
   `request_close` / `respond_close` / `arm_close_response` /
   `window_geometry` を追加。
2. `lib.rs`: `WindowLifecycleEvent` を `real-scenario` feature の
   re-export に追加（Coordinator 承認済みの 1 行）。
3. `tests/real_window_lifecycle.rs`: sentinel window を含む実 window
   session で close lifecycle（veto/defer/deferred-approve/
   default-approve・terminal 退役・sibling 分離）と DPI geometry
   （live `OsWindowGeometry`・`Resized` 一致・dead-window error）を
   検証。`RUVIFORM_RF631_SURFACE_EVIDENCE` env gate で JSONL evidence
   を emit。
4. `evidence/rf631/real-window-surface.jsonl` に実走結果を記録。
5. doc chain（discovery/spec/design/plan/task/test-spec/ir/exp）を
   完成させ、検証 lane を実行する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-631-window-surface.md`
- `docs/specs/issue-631-window-surface.md`
- `docs/designs/issue-631-window-surface.md`
- `docs/discovery/issue-631-window-surface.md`
- `docs/test-specs/issue-631-window-surface.md`

## Dependencies

- `WindowDriver::request_close`/`respond_close`（`lib.rs` 実装済み）
- `GpuiDriver::window_geometry`/`OsWindowGeometry`（`capture.rs` 実装済み）
- `WindowLifecycle`/`CloseDecision`（`ruviform-ui`、変更なし）
- `lib.rs` re-export の Coordinator 承認（取得済み）

## Affected Files

- `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs`
- `crates/ruviform-ui-gpui/src/lib.rs`（re-export 1 行のみ）
- `crates/ruviform-ui-gpui/tests/real_window_lifecycle.rs`（新規）
- `evidence/rf631/real-window-surface.jsonl`（新規）
- `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-631-window-surface.*`

## Done When

- [x] `WindowLifecycleEvent` と 5 つの public methods が
  `real-scenario` feature で compile・公開される。
- [x] 実 window で close lifecycle 全経路・terminal 退役・sibling
  分離が観測・assert される。
- [x] live geometry と lifecycle `Resized` が実測で一致し、
  dead-window geometry が honest error になる。
- [x] `cargo test -p ruviform-ui-gpui --features real-scenario
  --test real_window_lifecycle` が interactive desktop で Pass し、
  evidence JSONL に pass 行が記録される。
- [x] 非回帰 lane が green。

## Verification

- [x] `mise run fmt-check` — pass（初回 diff を `cargo fmt` で修正後）
- [x] `mise run check ruviform-ui-gpui --features real-scenario` — pass
- [x] `mise run lint ruviform-ui-gpui --features real-scenario` — pass
  （`too_many_lines`/`needless_pass_by_value`/`struct_field_names`/
  `unnecessary_wraps` を指摘され、handler 抽出と test helper 分割で
  解消。`#[allow]` 不使用）
- [x] `mise run test ruviform-ui-gpui --features real-scenario` — pass
  （全 target。`real_os_input_scenario` が初回 `NotRun`（OS focus
  非保持）で flaky fail → 単独再実行と lane 再実行で pass を確認、
  既存テストの環境依存 flake で本変更起因ではない）
- [x] `mise run test ruviform-ui-gpui` — pass
- [x] `mise run test-ui-scenario-contract` — pass（7 tests）
- [x] `mise run check-ui-driver-boundary` — pass
  （dep-graph/signature/allowlist 全 pass）
- [x] `mise run test-ui-real-scenario` — pass（3 tests、interactive
  Windows desktop。`evidence/issue-314-real-scenario/` へ lane 証拠
  追記）
- [x] `cargo test -p ruviform-ui-gpui --features real-scenario
  --test real_window_lifecycle`（実 Windows desktop、exit 0、
  `RUVIFORM_RF631_SURFACE_EVIDENCE` で evidence 追記）
