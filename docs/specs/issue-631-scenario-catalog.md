---
id: "SPEC-034XVTiZqMMaDcOo7LyHws"
type: "spec"
status: "approved"
title: "Issue #631 Plugin-dock scenario catalog + driver projection（driver 非依存 ScenarioEntry カタログ + run_scenario projection）"
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
  depends-on:
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "docs/specs/issue-631-real-window-dock.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-464-scenario-runner.md"
    - "POLICIES.md"
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

# Issue #631 Plugin-dock scenario catalog + driver projection（driver 非依存 ScenarioEntry カタログ + run_scenario projection）

## What Should Be Built

Issue #631（tracker）の第三 slice: Test Lab / Scenario Console の **headless 前半分**として、driver 非依存の scenario catalog 語彙と `run_scenario` への projection を `ruviform-ui-test` に整備し、`ruviform-plugin-dock-test` の case 列挙をその表へ移す。

本 slice が提供するもの:

- `crates/ruviform-ui-test/src/scenario/catalog.rs` — driver 非依存の `ScenarioEntry`（case id・scenario id・required capabilities・`ScenarioSpec` 組立・case body）+ `DriverBinding` / `ScenarioMatrix` 型 + `run_scenario` を binding 配列へ写す projection helper（`run_entry` / `run_entries` / `run_matrix`）。
- capability 宣言だけを見る副作用なしの `ScenarioMatrix::project`（`EntryProjection` — runnable / missing capability を catalog 順で返す）。
- `ruviform-plugin-dock-test` の headless lane 8 case を `&'static ScenarioMatrix`（`HEADLESS_MATRIX`）へテーブル化し、`run_all` を `run_matrix` の薄い投影にする。bin・test・将来の console が同じ表を消費する単一ソース化。
- `real_dock.rs` の 4 case を session-scoped catalog（`real_entries`）へ収録し、environment-blocked report を同じ表から導出して drift を防ぐ。
- catalog 一貫性・projection 網羅・fail-closed を検証する統合テストと、`evidence/rf631/` の lane 再実行証拠。

本 slice が**提供しない**もの（後続 slice / Issue 残 scope）:

- Scenario Console / Test Lab GUI 本体（catalog + projection はその headless 前半分）。
- `test_editor` plugin class 実装、実 Native Dock Pass、dock-pixels・child HWND input/resize/focus/re-dock の Pass 判定、DComp #532 判定。
- Synvelis/AudioLens 製品 oracle 照合、2Window Focus/DPI/IME 実検証、3OS 対応、`*-msrv` lane、Issue auto-close。

## Why Now

前 2 slice で plugin-dock scenario の headless lane（8 case 手書き列挙）と real-window lane（4 case）を整備したが、case の列挙が各所に散在し、driver ごとの capability 被覆や「どの case がどの driver binding で走るか」を一箇所から投影できない。Test Lab / Scenario Console が複数 driver（Fake / PluginHostProbe / 将来の RealOs）へ scenario を投影するには、driver 非依存の catalog が先に必要である。

catalog は executor ではなく `run_scenario` の呼び出しを列挙・投影する関数レジストリとして設計する — `docs/designs/issue-464-scenario-runner.md` が禁じる JSON DSL・独自 Test Executor・二重 Runner の禁止範囲に入れない。

## Users and Value

- Test Lab / Scenario Console の実装者: driver 非依存の `ScenarioMatrix` から capability 被覆 projection と実行順序を得て、console UI を driver 実装から切り離せる。
- plugin-dock-test の保守者: case id・scenario id・required caps・実行順が単一表に集約され、bin/test/console の間で drift しない。
- reviewer / evidence 消費者: catalog membership は Pass を意味しないことが型と doc で明示され、verdict の正直性が保たれる。

## Scope

### In Scope

- `crates/ruviform-ui-test/src/scenario/catalog.rs` 新規 + `scenario.rs`/`lib.rs` への `pub mod`/`pub use` 追加のみ。
- `ScenarioEntry<C>`（`case`/`scenario`/`required`/`spec_extras`/`run`）、`DriverBinding<C>`、`ScenarioMatrix<'a, C>`、`EntryProjection`、`run_entry`/`run_entries`/`run_matrix`。
- `ruviform-plugin-dock-test/src/scenario.rs` の `run_all` 8 case の `HEADLESS_MATRIX` テーブル化（`HeadlessContext` に artifact 共有状態を集約）。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs` の 4 case の session-scoped catalog 化と `blocked_reports` の同一ソース化。
- `lib.rs` re-export、`tests/plugin_dock.rs` の catalog 一貫性・projection・fail-closed テスト追加、README 更新。
- 本 slice の spec/design/plan/task/test-spec/impl 文書チェーンと `evidence/rf631/` 記録。

### Out of Scope

- GUI console 本体、`test_editor` 実装、実 Native Dock Pass・dock-pixels・child HWND 判定、DComp #532。
- `crates/ruviform-ui-gpui` 本体の変更、`mise.toml` の task 追加・変更（別 Worker 占有）、CI workflow、`Cargo.lock` 依存差分（本 slice は新規依存なし）。
- `crates/ruviform-ui-test` の `scenario/`・`scenario.rs`・`lib.rs` 以外のファイル変更（driver.rs / fake.rs / fixture.rs 等は参照のみ）。
- Issue #631 の auto-close（tracker — `Refs #631` のみ）。

## Requirements

- catalog/projection は driver 非依存であり、GPUI・OS-driver 型を `ruviform-ui-test` の公開 API へ漏らさない（`check-ui-driver-boundary` green）。
- `ScenarioMatrix::project` は宣言された capability のみを見る副作用なしの投影であり、scenario を実行せず verdict を昇格しない。
- catalog への収録は Pass を意味しない。各 entry の `run` body が `run_scenario` 経由で正直な `ScenarioReport` を返す。
- headless/非対話環境では real lane を `NotRun` + `environment_blocked` で fail-closed に記録し、`EnvironmentKind::RealWindow` を実走していない経路で名乗らない。
- クレート外から到達可能な公開 enum に enum レベル `#[non_exhaustive]`（`EntryProjection` 等）。`unsafe` は使わない。
- 既存 JSONL evidence schema・`run_all`/bin の外部 contract を維持する（`test-ui-plugin-dock` lane 非回帰）。

## Acceptance Criteria

- [ ] `crates/ruviform-ui-test/src/scenario/catalog.rs` に `ScenarioEntry`/`DriverBinding`/`ScenarioMatrix`/projection helper が存在し、`run_scenario` 呼び出しの列挙として設計されている（独自 executor・JSON DSL なし）。
- [ ] `ruviform-plugin-dock-test` の headless 8 case が `HEADLESS_MATRIX` 単一表から実行され、`run_all` が薄い投影になっている。
- [ ] real-dock 4 case が session-scoped catalog から実行され、blocked report が同じ表から導出される。
- [ ] `tests/plugin_dock.rs` に catalog↔report 対応・projection 被覆・fail-closed のテストが追加され pass する。
- [ ] `mise run test-ui-plugin-dock` の JSONL verdict が catalog 化前と同一（view-mount=pass, artifact=pass, host-lifecycle=pass, editor-attach=unsupported, session-lifecycle=pass, host-terminate=pass, open-or-degrade=pass, dock-pixels=not_run）。
- [ ] `mise run check/lint/test ruviform-ui-test`・`ruviform-plugin-dock-test`（default + `--features real-dock`）、`test-ui-scenario-contract` 系、`check-ui-driver-boundary`、`fmt-check`、`prek` が実走して成功する。

## Deferred Design Concerns

- Scenario Console / Test Lab GUI 本体 — catalog + projection は headless 前半分のみ | console GUI slice の着手時 | projection API が console 要件とずれた場合の手戻り
- `test_editor` 到着後の `real-editor-attach` Pass 化・dock-pixels 判定 — 実物 fixture 未到着 | `test_editor` merge 時 | NotRun/Unsupported の長期滞留
- Synvelis/AudioLens 製品 oracle・3OS 対応 — Deferred（POLICIES.md） | 各 product/OS slice 着手時 | driver-neutral contract の実証不足

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — 本 spec はその第三 slice のみを対象）。
- 前 slice: `docs/specs/issue-631-plugin-dock-scenario.md`（headless lane）、`docs/specs/issue-631-real-window-dock.md`（real-window lane）。
- `docs/designs/issue-464-scenario-runner.md` — 「runner は 1 関数で executor ではない」制約（catalog は禁止範囲外の関数レジストリ + projection）。
- `POLICIES.md` — 検証レベル・Pass/Fail/NotRun/Unsupported 規範・仮想優先・fail-closed。
- `docs/adr/public-enum-non-exhaustive-policy.md` — 公開 enum の `#[non_exhaustive]` 方針。
