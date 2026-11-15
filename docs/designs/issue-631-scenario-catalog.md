---
id: "DESIGN-034XVTqsKVWAgXMI3CBA6o"
type: "design"
status: "approved"
title: "Issue #631 Plugin-dock scenario catalog + driver projection 設計"
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
    - "docs/specs/issue-631-scenario-catalog.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Plugin-dock scenario catalog + driver projection 設計

## Context

`ruviform-ui-test` は driver-neutral な scenario contract
（`ScenarioSpec`/`ScenarioReport`/`ScenarioCapability`/`ScenarioDriver`/
`run_scenario`）を所有する。`ruviform-plugin-dock-test` はその consumer
composition root であり、headless lane（`src/scenario.rs`）と
real-window lane（`src/real_dock.rs`、`real-dock` feature）の 2 系統の
case を持つ。

catalog 化前は headless 8 case が `run_all` 内に手書き列挙され、real
dock 4 case も `run_all` 内に個別記述されていた。case id・scenario
id・required capabilities・実行順が表として存在せず、bin・test・
将来の Scenario Console が同じ列挙を共有できなかった。本設計は
case 列挙を driver 非依存の catalog 表へ移し、`run_scenario` 呼び出し
の projection として再構成する。

## Scope

### In Scope

- `crates/ruviform-ui-test/src/scenario/catalog.rs` 新規モジュール。
- `scenario.rs`/`lib.rs` への `pub mod`/`pub use` 追加。
- plugin-dock-test headless lane の `HEADLESS_MATRIX` テーブル化と
  `run_all` の薄い投影化。
- real-dock lane の session-scoped catalog 化と `blocked_reports`
  の同一ソース化。
- catalog 一貫性・projection・fail-closed のテスト追加。

### Out of Scope

- Scenario Console / Test Lab GUI、`test_editor`、実 Native Dock
  Pass、製品 oracle、3OS、`mise.toml`/CI 変更、新規依存。
- `ruviform-ui-test` の scenario モジュール外ファイル、`ruviform-ui-gpui`
  本体の変更。

## Components and Boundaries

- `ScenarioSpecExtras`（`environment: Option<Value>`・
  `timeout_ms: Option<u64>`）— entry が spec へ載せる per-case 追加
  field。spec-level declaration であり、consumer が post-run に
  `report.environment` を finalize する場合は上書きされる。
- `ScenarioEntry<C>` — `case`/`scenario`（`&'static str`）、
  `required: &'static [ScenarioCapability]`、`spec_extras: fn(&C)`、
  `run: fn(&mut C, &ScenarioSpec) -> ScenarioReport`。`C` は consumer
  定義の lane context で、catalog は `C` を検査せず thread するだけ
  — これが driver 非依存性の根拠。`Copy`/`Clone` は手動 impl
  （全 field が `C` 非依存に Copy 可能なため derived bound は不要かつ
  `static` 表からの move を妨げる）。
- `DriverBinding<'a, C>` — `driver: ScenarioDriver`、
  `capabilities: &'static [ScenarioCapability]`（正直な宣言）、
  `cases: &'a [ScenarioEntry<C>]`。
- `ScenarioMatrix<'a, C>` — `bindings` のみを持つ薄い集約。
  `cells()` は binding-major・case 順の `(entry, binding)` 列挙。
- `EntryProjection<'a, C>` — `entry`/`driver`/`runnable`/`missing`
  （`ScenarioCapability` の `Ord` 順で最初の欠落 cap）。
- `run_entry`/`run_entries`/`run_matrix` — `entry.spec(run_id, ctx)`
  で spec を組み立て `entry.run` を呼ぶだけの sequencing helper。
  verdict ロジックを持たない。

## Data and Control Flow

1. consumer が `C`（lane context: `HeadlessContext` /
   `RealDockContext` 等）を用意し、`ScenarioMatrix`（`&'static` または
   session-scoped の `DriverBinding` 配列）を構築する。
2. `run_matrix`/`run_entries`/`run_entry` が catalog 順に
   `entry.spec()` → `entry.run()` を呼び、`Vec<ScenarioReport>` を返す。
   各 `run` body は内部で `run_scenario` を呼び、pre-flight・verdict・
   report 構築は既存 runner が担う。
3. console 相当の consumer は `matrix.project()` で「この driver の
   宣言 capability で pre-flight が通るか」を実行なしに一覧できる。
   `project` は `binding.capabilities`（宣言）と `entry.required`
   （宣言）のみを比較し、live adapter の `capabilities()` への問い
   合わせでも実行でもない。

## Non-Functional Constraints

- GPUI 非依存: `ruviform-ui-test` は GPUI/OS-driver 型を import せず、
  `C` generic のみで consumer 型を受ける。`check-ui-driver-boundary`
  green を維持。
- 正直性: catalog 収録 ≠ Pass。`required` は `ScenarioSpec.required`
  に verbatim コピーされ live pre-flight が再判定する。`project` の
  `runnable` は「宣言上 pre-flight が通る」の意味であり実行結果を
  含まない。
- fail-closed: real lane の environment-blocked report は
  `real_entries` と同じ表から case id/scenario/required を導出し、
  `NotRun` + `environment_blocked` observation を維持。
  `EnvironmentKind::RealWindow` は実走経路のみ。
- 公開 enum は `#[non_exhaustive]`（本 slice の新規公開 enum は
  struct のみ — `EntryProjection` は struct。enum 追加なし）。
- 互換性: `run_all`/bin/JSONL schema の外部 contract 不変。

## Risks and Trade-offs

- `DriverBinding.capabilities` は live `capabilities()` の写しであり
  drift し得る → doc で「adapter の `SemanticGuiTest::capabilities`
  と一致させる宣言」と明記し、test で `HEADLESS_MATRIX.project()` の
  全 entry runnable を検証。
- `ScenarioEntry` の `run` が `&mut C` を取るため entry 間で context
  状態が共有される（artifact case が context に証拠を置き、後続
  case が読む）→ これは意図した設計（前 slice の逐次依存関係を保持）。
  `spec_extras` は `&C` のみ。
- session-scoped catalog（real-dock）は `&'static` にできない →
  `run_entries`/`run_entry` が flat slice 版として存在する理由。
  `blocked_reports` は `'static` context ダミーで `real_entries()` を
  呼び、live/blocked 両経路が同一関数から表を得る。

## Deferred Design Concerns

- console が必要とする filtering/sorting/UI projection の richer API —
  console GUI slice で要件確定後に拡張 | GUI slice 着手時 |
  現在の `cells()`/`project()` で不足する投影軸が出た場合の再設計
- 複数 scenario id を跨ぐ matrix の group-by — 現状 binding×case の
  2 次元のみ | console 要件確定時 | catalog 表現力不足

## Verification Notes

- [x] `cargo test -p ruviform-ui-test`（catalog unit test 4 件含む）
- [x] `cargo test -p ruviform-plugin-dock-test`（catalog 一貫性・
  projection・fail-closed 統合テスト）
- [x] `cargo test -p ruviform-plugin-dock-test --features real-dock`
- [x] `mise run test-ui-plugin-dock`（JSONL verdict 非回帰）
- [x] `mise run test-ui-scenario-contract`（fake/gpui 両 lane）
- [x] `mise run check-ui-driver-boundary`
