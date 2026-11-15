---
id: "TSPEC-034XVTsew8bdAg2cxa7FPJ"
type: "test-spec"
status: "approved"
title: "Issue #631 Plugin-dock scenario catalog + driver projection のテスト仕様"
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
    - "docs/plans/issue-631-scenario-catalog.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-631-scenario-catalog.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Plugin-dock scenario catalog + driver projection のテスト仕様

## Purpose

scenario catalog + driver projection が「`run_scenario` 呼び出しの
列挙・投影」であり verdict 権威を持たないこと、catalog 表と実行結果が
drift しないこと、fail-closed が維持されることを保証する。本 spec は
console GUI slice 到着後も catalog 層の契約として残す。

## Feature: scenario catalog の spec 組立と実行投影

### Rule: `entry.spec` は宣言 identity・required・extras のみから spec を組み立てる

- **Example**: `case`/`scenario` が `CaseId`/`ScenarioId` になり、
  `required` が `BTreeSet` として verbatim コピーされ、
  `spec_extras` の `environment`/`timeout_ms` が spec field に載る。
- **Example**: `spec_extras` が `none()` の entry では
  `environment`/`timeout_ms` が unset のまま。

### Rule: `run_entry`/`run_entries`/`run_matrix` は catalog 順に実行し verdict を改変しない

- **Example**: binding-major・case 順で `run` が呼ばれ、各 `run` が
  受け取った spec が `entry.spec` の組立結果と一致する。
- **Example**: `run` が返した report の verdict がそのまま返る
  （catalog 層は Pass 昇格・降格を行わない）。

## Feature: `ScenarioMatrix::project` の capability 投影

### Rule: `project` は宣言データのみを見て副作用なく runnable/missing を返す

- **Example**: binding の `capabilities` が `entry.required` を全て
  含むとき `runnable=true`・`missing=None`。
- **Example**: required cap が欠けるとき `runnable=false`・
  `missing` が `ScenarioCapability` の `Ord` 順で最初の欠落 cap
  （`require_capabilities` が名指すのと同じ cap）。
- **Example**: `project()` は scenario を実行しない — `run` が呼ばれ
  ず context に副作用が残らない。

## Feature: plugin-dock-test の catalog 一貫性

### Rule: `HEADLESS_MATRIX` の cells と `run_all` の report が 1:1 対応する

- **Example**: `HEADLESS_MATRIX.cells()` の case id/scenario id/
  driver/required capabilities が `run_all` の report 列と順序含めて
  一致する（表↔実行のドリフト検出）。
- **Example**: `HEADLESS_MATRIX.project()` が全 8 entry を runnable
  と判定する（宣言 capability と required の整合）。

### Rule: catalog 化で verdict が変わらない

- **Example**: artifact 未解決時に `missing_artifact_fails_closed_as_not_run`
  が NotRun を返し、pass へ化けない。
- **Example**: `test-ui-plugin-dock` lane の JSONL verdict が
  catalog 化前と同一（editor-attach=unsupported、dock-pixels=not_run
  等）。

## Feature: real-dock の session-scoped catalog と fail-closed

### Rule: live/blocked 両経路が `real_entries()` を共有する

- **Example**: `blocked_reports` の report 数・case id・scenario・
  required capabilities が `real_entries()` の 4 entry と一致する。
- **Example**: 全 blocked report が `NotRun` + driver `RealOs` +
  `"observed": "environment_blocked"` を持ち、`"real-window"` を
  environment に名乗らない。

## Guarantees

- catalog/projection は driver 非依存（`ruviform-ui-test` は GPUI・
  OS-driver 型を公開 API に持たない — `check-ui-driver-boundary` green）。
- 公開型は `#[non_exhaustive]` enum 方針に違反しない（本 slice の新規
  公開型は struct のみ）。
- 既存 JSONL evidence schema・`run_all`/bin の外部 contract は不変。

## Non-goals

- Scenario Console / Test Lab GUI の描画・操作。
- 実 Native Dock Pass・dock-pixels・child HWND の Pass 判定
  （`test_editor` 未到着のため NotRun/Unsupported 維持）。
- `DriverBinding.capabilities` と live `capabilities()` の完全一致
  証明（宣言の写しであり、乖離は consumer test の責務）。

## Risk

- catalog 表と実行経路が分離すると「表にあるが走らない/走るが表にない」
  drift が起き得る → cells↔report 対応テストと `run_all` の `run_matrix`
  単一経路化で抑止。drift が混入すれば verdict 偽装か lane 破壊として
  検出される。
