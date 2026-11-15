---
id: "PLAN-034XVTr2qWvCZbojyUlWuD"
type: "plan"
status: "approved"
title: "Issue #631 Plugin-dock scenario catalog + driver projection 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
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
    - "docs/specs/issue-631-scenario-catalog.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-scenario-catalog.md"
    - "docs/designs/issue-631-scenario-catalog.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Plugin-dock scenario catalog + driver projection 実装計画 Implementation Plan

## Goal

`ruviform-ui-test` に driver 非依存の scenario catalog（`ScenarioEntry`/`DriverBinding`/`ScenarioMatrix` + `run_scenario` projection helper）を新設し、`ruviform-plugin-dock-test` の headless 8 case と real-dock 4 case をその表へ移して、bin・test・将来の Scenario Console が同一ソースを消費できる状態にする。

## Scope

単一実装 stream — ui-test への catalog モジュール追加 + plugin-dock-test 内のテーブル化 refactor + テスト/doc/evidence。分割不要。

## Upstream Documents

- `docs/specs/issue-631-scenario-catalog.md`
- `docs/specs/issue-631-plugin-dock-scenario.md`（前 slice — headless lane）
- `docs/specs/issue-631-real-window-dock.md`（前 slice — real-window lane）
- `docs/designs/issue-464-scenario-runner.md`（runner は 1 関数で executor ではない制約）

## Design Inputs

- `docs/designs/issue-631-scenario-catalog.md`（approved）

## File Map

- `crates/ruviform-ui-test/src/scenario/catalog.rs` — 新規。`ScenarioSpecExtras`/`ScenarioEntry<C>`/`DriverBinding`/`ScenarioMatrix`/`EntryProjection`/`run_entry`/`run_entries`/`run_matrix` + unit test
- `crates/ruviform-ui-test/src/scenario.rs` — `pub mod catalog` + re-export 追加のみ
- `crates/ruviform-ui-test/src/lib.rs` — catalog 語彙の re-export 追加のみ
- `crates/ruviform-plugin-dock-test/src/scenario.rs` — `HeadlessContext` + `HEADLESS_MATRIX`（`&'static`、Fake/PluginHostProbe 2 binding・8 entry）+ `run_all` の `run_matrix` 薄い投影化 + case body の `fn(&mut HeadlessContext, &ScenarioSpec)` 化
- `crates/ruviform-plugin-dock-test/src/real_dock.rs` — `RealDockContext` + `real_entries()`（session-scoped 4 entry）+ live/blocked 両経路の同一ソース化 + `blocked_reports` の catalog 導出化
- `crates/ruviform-plugin-dock-test/src/lib.rs` — `HEADLESS_MATRIX` re-export 追加
- `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs` — catalog↔report 対応・`project()` 被覆・fail-closed テスト追加
- `crates/ruviform-plugin-dock-test/README.md` — catalog/projection 説明追記
- `evidence/rf631/plugin-dock-scenario.jsonl` — lane 再実行で更新
- `docs/*/issue-631-scenario-catalog.*` — doc chain（spec/design/plan/task/test-spec/ir/exp）
- `docs/*/README.md` — index 再生成（generator のみ）

## Implementation Sequence

- [x] `catalog.rs` 新規 + `scenario.rs`/`lib.rs` 配線 + unit test
- [x] `scenario.rs`（plugin-dock-test）: `HeadlessContext`/`HEADLESS_MATRIX` 化、`run_all` を `run_matrix` 投影に
- [x] `real_dock.rs`: `RealDockContext`/`real_entries` 化、live/blocked 同一ソース化
- [x] `lib.rs` export・`tests/plugin_dock.rs` catalog テスト・README 更新
- [x] fmt/check/lint/test（両 crate ±real-dock）・非回帰 lane・boundary guard・prek
- [x] `test-ui-plugin-dock` lane 再実行 → `evidence/rf631/` 更新
- [x] doc chain 完成・index 再生成・impl IR/exp 仕上げ

## Dependencies and Risks

- catalog は `run_scenario` の呼び出し列挙であり verdict 権威を持たない — 各 entry の `run` body が既存 case ロジックをそのまま保持するため verdict は不変のはず（JSONL 非回帰で検証）。
- `ScenarioEntry<C>` の `Copy`/`Clone` は手動 impl — derived だと `C: Copy` bound が付き `static` 表から非 Copy context 向け entry を move できない（E0507 で実測）。
- `DriverBinding.capabilities` は宣言の写し — live `capabilities()` との drift は test（`project()` 全 runnable）で検出する範囲に留める。
- real-dock の `real_entries()` は session-scoped で `&'static` 不可 — `run_entries`（flat slice）で対応済み。

## Task Breakdown

- T1（`docs/tasks/issue-631-scenario-catalog.md`）: 全体を 1 task で実施。

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check|lint|test ruviform-ui-test`
- [x] `mise run check|lint|test ruviform-plugin-dock-test`（default + `--features real-dock`）
- [x] `mise run test-ui-plugin-dock`（JSONL verdict 非回帰）
- [x] `mise run test-ui-scenario-contract`（fake/gpui 両 lane）
- [x] `mise run check-ui-driver-boundary`
- [x] `mise exec -- prek run --all-files`

## Open Questions / Gaps

- console GUI が必要とする追加 projection（filter/sort/group）は本 slice では未実装 — GUI slice で要件確定後に拡張。

## Self-Review

- [x] spec の acceptance criteria を全て file map / sequence / verification に写像済み。
- [x] GUI console・実 Native Dock Pass・製品 oracle は明示的に out of scope。
- [x] file paths・relations・status values・terminology は既存 doc 慣行と一致。

## Implementation Handoff

- Recommended mode: inline execution（dispatch worker が直接実装）
- Independent work streams: none
- User approval required before dispatch: no（worker 契約により review-protocol を承認として扱う）

## Review Handoff

- verdict 正直性（catalog 収録 ≠ Pass、blocked report の fail-closed）と GPUI 非依存（`C` generic による境界）が reviewer focus。
