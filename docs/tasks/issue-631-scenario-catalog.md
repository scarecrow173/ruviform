---
id: "TASK-034XVTrDLIayJZvRDYBcAb"
type: "task"
status: "in-progress"
title: "Issue #631 T1 — Plugin-dock scenario catalog + driver projection"
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
    - "docs/plans/issue-631-scenario-catalog.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-631-scenario-catalog.md"
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

# Issue #631 T1 — Plugin-dock scenario catalog + driver projection

## Work

`ruviform-ui-test` に driver 非依存の scenario catalog モジュールを新設し、plugin-dock-test の headless/real-dock 両 lane の case 列挙をその表へ移す。

1. `crates/ruviform-ui-test/src/scenario/catalog.rs` 新規 —
   `ScenarioSpecExtras`/`ScenarioEntry<C>`/`DriverBinding`/`ScenarioMatrix`/
   `EntryProjection`/`run_entry`/`run_entries`/`run_matrix` + unit test。
   `scenario.rs`/`lib.rs` は `pub mod`/`pub use` 追加のみ。
2. `crates/ruviform-plugin-dock-test/src/scenario.rs` — `HeadlessContext`
   （artifact 共有状態を集約）+ `HEADLESS_MATRIX`（`&'static`、Fake /
   PluginHostProbe の 2 binding・8 entry）を構築し、`run_all` を
   `run_matrix` の薄い投影に。case body は
   `fn(&mut HeadlessContext, &ScenarioSpec) -> ScenarioReport` に変換。
3. `crates/ruviform-plugin-dock-test/src/real_dock.rs` —
   `RealDockContext` + `real_entries()`（session-scoped 4 entry）を
   構築し、live 実行経路と `blocked_reports` の両方を同一表から導出。
   blocked report は `NotRun` + `environment_blocked` を維持。
4. `lib.rs` re-export・`tests/plugin_dock.rs` に catalog 一貫性
   （表↔report 対応・case 数/id）・`project()` 被覆・fail-closed
   テストを追加。README に catalog/projection を追記。
5. canonical 検証・`evidence/rf631/` 更新・doc chain 完成・index 再生成。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-631-scenario-catalog.md`
- `docs/specs/issue-631-scenario-catalog.md`
- `docs/designs/issue-631-scenario-catalog.md`

## Dependencies

- 前 slice merge 済: `issue-631-plugin-dock-scenario`（headless lane）、
  `issue-631-real-window-dock`（real-window lane、`origin/main` = `f03390d0`）。

## Affected Files

- `crates/ruviform-ui-test/src/scenario/catalog.rs`（新規）
- `crates/ruviform-ui-test/src/scenario.rs`・`src/lib.rs`（追加のみ）
- `crates/ruviform-plugin-dock-test/`（`src/scenario.rs`、`src/real_dock.rs`、`src/lib.rs`、`tests/plugin_dock.rs`、`README.md`）
- `evidence/rf631/plugin-dock-scenario.jsonl`
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-631-scenario-catalog.*`

## Done When

- [x] `HEADLESS_MATRIX` が 8 case を単一表として保持し、`run_all` が
  `run_matrix` 投影になっている。
- [x] real-dock の live/blocked 両経路が `real_entries()` を共有し、
  blocked report が `NotRun` + `environment_blocked` を維持する。
- [x] catalog↔report 一貫性・projection 被覆・fail-closed テストが
  追加され pass する。
- [x] `test-ui-plugin-dock` の JSONL verdict が catalog 化前と同一。
- [x] `check-ui-driver-boundary` green（GPUI 型の漏洩なし）。

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check|lint|test ruviform-ui-test`
- [x] `mise run check|lint|test ruviform-plugin-dock-test`（default + `--features real-dock`）
- [x] `mise run test-ui-plugin-dock` / `test-ui-scenario-contract`（fake/gpui）/ `check-ui-driver-boundary`
- [x] `mise exec -- prek run --all-files`
