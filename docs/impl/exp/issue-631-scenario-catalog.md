# Issue #631 scenario catalog + driver projection — 実装実験記録（2026-11-15）

`docs/tasks/issue-631-scenario-catalog.md` の第三 slice
（plugin-dock scenario catalog + driver projection）実装で試行・観測した
事項の記録。実装結果の正本は
[`docs/impl/ir/issue-631-scenario-catalog.md`](../ir/issue-631-scenario-catalog.md)、
イベントログは
[`issue-631-scenario-catalog.jsonl`](issue-631-scenario-catalog.jsonl)
を参照。

## 試行 1: `ScenarioEntry<C>` の `Copy`/`Clone` derive 問題

- **目的**: `static`/`const` な catalog 表から entry を move して
  `run_entry` に渡せるようにしたい。
- **観測**: `#[derive(Debug, Clone, Copy)]` は `C: Clone`/`C: Copy`
  bound を暗黙に要求し、非 Copy context（`HeadlessContext` 等）向けの
  entry を `static` 表から取り出すと E0507（cannot move out of
  static item）になる。
- **結論**: 全 field（`&'static str`・`&'static [ScenarioCapability]`・
  fn pointer 2 本）が `C` に依存せず Copy 可能なため、`Copy`/`Clone`
  を手動 impl した。derive の暗黙 bound を排除しつつ `static` 表からの
  move を許す。catalog.rs のコメントに理由を明記。

## 試行 2: real-dock の session-scoped catalog を `&'static` にできない問題

- **目的**: real-dock lane の 4 case も catalog 表に収録し、live 実行と
  environment-blocked report が同一ソースを共有する。
- **観測**: `RealDockContext<'a>` は live `RealOsGuiTest` session に
  借用を持つため `&'static` にできず、`HEADLESS_MATRIX` のような
  `static ScenarioMatrix` を構成できない。
- **結論**: `real_entries()` が `[ScenarioEntry<RealDockContext>; 4]`
  を返す関数とし、live 経路は `run_entries(&entries, ...)`、
  `blocked_reports` は `RealDockContext<'static>` ダミーで同じ
  `real_entries()` を呼んで report を組み立てる。表の生成を 1 関数に
  集約したことで live/blocked の case id・scenario・required の
  drift を構造的に排除した。

## 試行 3: `blocked_reports` の catalog 導出化

- **目的**: environment-blocked report が live case 定義と drift しない
  こと（case 数・id・scenario・required capabilities の一致）。
- **観測**: catalog 化前は blocked report が case 定義と別々に
  hand-written されており、case 追加時に片方だけ更新されるリスクが
  あった。
- **結論**: `blocked_reports` を `real_entries()` から導出する形に
  変更し、`blocked_reports_fail_closed` unit test で entries と
  reports の zip 一致（case/scenario/required）+ 全件 `NotRun` +
  `ScenarioDriver::RealOs` + `"observed": "environment_blocked"` +
  `"real-window"` 非主張を assert した。

## 試行 4: `DriverBinding.capabilities` の宣言ドリフト

- **目的**: `ScenarioMatrix::project` が live adapter の
  `capabilities()` ではなく binding の宣言を見る設計の検証。
- **結論**: 宣言は「live `SemanticGuiTest::capabilities` の写しを
  consumer が維持する」契約とし、`HEADLESS_MATRIX.project()` が全
  8 entry runnable であることを `catalog_projection_covers_every_
  declared_requirement` で固定。乖離の検出は consumer test の責務と
  して test-spec の Non-goals に明記した（catalog 層は live 問合せを
  持たない）。
