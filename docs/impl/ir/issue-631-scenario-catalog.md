---
id: "IMPL-034XVTsUl2EPLLP9JqeEGS"
type: "impl"
status: "in-progress"
title: "Plugin-dock scenario catalog + driver projection (issue-631 third slice)"
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
    - "docs/tasks/issue-631-scenario-catalog.md"
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
      - "docs/impl/exp/issue-631-scenario-catalog.md"
    rejected: []
---

# Plugin-dock scenario catalog + driver projection (issue-631 third slice)

## Summary

`ruviform-ui-test` に driver 非依存の scenario catalog
（`ScenarioEntry<C>`/`DriverBinding`/`ScenarioMatrix`/`EntryProjection` +
`run_entry`/`run_entries`/`run_matrix`）を `scenario/catalog.rs` として
新設し、`ruviform-plugin-dock-test` の headless lane を `HEADLESS_MATRIX`
（`&'static`、Fake/PluginHostProbe 2 binding・8 entry）へ、real-dock
lane を session-scoped `real_entries()`（4 entry）へテーブル化した。
`run_all` は `run_matrix` の薄い投影になり、bin・test・将来の console が
同一表を消費する。environment-blocked report も同じ表から導出するため
live/blocked 経路の drift がない。

## Context

`docs/tasks/issue-631-scenario-catalog.md`（plan
`docs/plans/issue-631-scenario-catalog.md`、spec
`docs/specs/issue-631-scenario-catalog.md`、design
`docs/designs/issue-631-scenario-catalog.md`）の第三 slice。
Test Lab / Scenario Console の headless 前半分。catalog は executor
ではなく `run_scenario` 呼び出しの列挙・投影 — `issue-464` design の
「runner は 1 関数で executor ではない」制約を維持する。

## Implementation

### Changes

- `crates/ruviform-ui-test/src/scenario/catalog.rs`（新規、427 行 +
  unit test 4 件）: `ScenarioSpecExtras`、`ScenarioEntry<C>`
  （`case`/`scenario`/`required`/`spec_extras`/`run`）、
  `DriverBinding<'a, C>`、`ScenarioMatrix<'a, C>`（`cells()`/`project()`）、
  `EntryProjection`、`run_entry`/`run_entries`/`run_matrix`。
- `crates/ruviform-ui-test/src/scenario.rs`: `pub mod catalog` +
  catalog 語彙の re-export（追加のみ）。
- `crates/ruviform-ui-test/src/lib.rs`: catalog 語彙の re-export（追加のみ）。
- `crates/ruviform-plugin-dock-test/src/scenario.rs`: `HeadlessContext`
  （`LaneEnvironment` + artifact 共有 slot）導入、`HEADLESS_MATRIX`
  static、8 case body を `fn(&mut HeadlessContext, &ScenarioSpec)` に
  変換、`run_all` = `run_matrix(&HEADLESS_MATRIX, ...)`。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`: `RealDockContext`
  + `real_entries()`（4 entry、`run` は `RealOsGuiTest::run` 経由）、
  live 経路と `blocked_reports` の同一ソース化。
- `crates/ruviform-plugin-dock-test/src/lib.rs`: `HEADLESS_MATRIX`
  re-export。
- `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs`: catalog↔
  report 対応（`catalog_matches_run_all_reports`）、projection 被覆
  （`catalog_projection_covers_every_declared_requirement`）テスト追加。
- `crates/ruviform-plugin-dock-test/README.md`: catalog/projection の
  説明追記。
- `evidence/rf631/plugin-dock-scenario.jsonl`: lane 再実行で更新
  （verdict は catalog 化前と同一）。

### Decisions During Implementation

- `ScenarioEntry<C>` の `Copy`/`Clone` は手動 impl — derive すると
  `C: Copy`/`C: Clone` bound が付き、非 Copy context 向け entry を
  `static` 表から move できず E0507 になる（実測）。全 field が `C`
  非依存に Copy 可能なため手動 impl が正しい。
- `DriverBinding.capabilities` は `&'static [ScenarioCapability]` の
  宣言 — live `capabilities()` の写しを consumer が維持する契約とし、
  `project()` は宣言同士の比較に限定（実行も live 問合せもしない）。
- real-dock は session-scoped で `&'static` にできないため
  `run_entries`（flat slice 版）を併設。`blocked_reports` は
  `RealDockContext<'static>` ダミーで `real_entries()` を呼び、
  live/blocked が同一関数から表を得る。
- `HEADLESS_MATRIX` は `ScenarioMatrix<'static, HeadlessContext>` —
  `view-mount` のみ `ScenarioDriver::Fake` binding（SemanticAction +
  MultiWindow 宣言）、残り 7 case は `PluginHostProbe` binding
  （capabilities 宣言なし）。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-631-scenario-catalog.md`（+ 同 stem の
  `.jsonl` event log）— `Copy`/`Clone` 手動 impl、session-scoped
  catalog の `run_entries` 対応など実装中の試行を記録。

### Rejected

- なし。

## Validation

canonical = mise（Windows、rust toolchain は mise 管理）:

- `mise run fmt-check` — pass
- `mise run check|lint|test ruviform-ui-test` — pass（catalog unit
  test 4 件含む 83+ tests）
- `mise run check|lint|test ruviform-plugin-dock-test`（default）— pass
- `mise run check|lint|test ruviform-plugin-dock-test --features real-dock` — pass
- `mise run test-ui-plugin-dock` — pass。JSONL verdict は catalog 化前と
  同一（view-mount=pass, artifact=pass, host-lifecycle=pass,
  editor-attach=unsupported, session-lifecycle=pass,
  host-terminate=pass, open-or-degrade=pass, dock-pixels=not_run）
- `mise run test-ui-scenario-contract`（fake 17 + gpui 7）— pass
- `mise run check-ui-driver-boundary` — pass（dep-graph/signature/
  allowlist 全て）
- `mise exec -- prek run --all-files` — pass

補助証拠（canonical ではない）: `cargo check/test --locked` 個別実行も
同結果。

## Risks

- `DriverBinding.capabilities` は宣言の写しであり live
  `capabilities()` との drift を完全には防げない — `HEADLESS_MATRIX`
  の `project()` 全 runnable test で現状の整合を固定。乖離は consumer
  側 test の責務として test-spec に明記。
- `ScenarioEntry.run` が `&mut C` を取る設計は entry 間の context 共有
  を許す — artifact case が context に証拠を置き後続 case が読む前
  slice の逐次依存をそのまま保持する意図的設計。

## Follow-ups

- Scenario Console / Test Lab GUI 本体（catalog + projection の
  consumer）— 後続 slice。
- `test_editor` 実物到着後の `real-editor-attach` Pass 化・
  dock-pixels 判定 — Issue #630 残存 scope / #532 DComp 判定。
- console が必要とする追加 projection（filter/sort/group-by）—
  GUI slice で要件確定後に拡張。
