# Issue #631 headless scenario console — 実装実験記録（2026-11-16）

`docs/tasks/issue-631-scenario-console.md` の console slice
（`plugin-dock-console` headless consumer）実装で試行・観測した
事項の記録。実装結果の正本は
[`docs/impl/ir/issue-631-scenario-console.md`](../ir/issue-631-scenario-console.md)、
イベントログは
[`issue-631-scenario-console.jsonl`](issue-631-scenario-console.jsonl)
を参照。

## 試行 1: `--run` の部分実行に必要な context 構築経路

- **目的**: console の `--run --case <one>` が lane context を
  必要とするが、`HeadlessContext` は fields private で `run_all`
  のみが構築していた。
- **観測**: bin target は lib とは別 crate で、`pub(crate)` の
  constructor/helper は bin から見えない。`HeadlessContext` を
  `pub` field 化すると lane state を外部から任意に組み立てられて
  しまい、artifact 前提の逐次依存を壊す余地ができる。
- **結論**: `scenario::run_selected(env, keep)` を `pub` で追加し、
  `keep: impl Fn(&ScenarioEntry<HeadlessContext>, ScenarioDriver)
  -> bool` の selector 述語のみを外に出した。context 構築は
  scenario module 内に閉じたまま、`run_all` は
  `run_selected(env, |_, _| true)` へ委譲して挙動を不変に保った。

## 試行 2: real lane selector を全 build で認識する方法

- **目的**: `--run --case real-window-mount` や `--driver real-os`
  を「unknown」ではなく「real lane 所有」の正しい案内で拒否したい。
  また `--list --include-real` の metadata も欲しい。
- **観測**: `real_dock::real_entries()` は `#[cfg(feature =
  "real-dock")]` 配下で `RealDockContext<'a>`（private・live
  session 借用）を返す — default build では存在せず、feature
  build でも private 型を public interface に出せない。
- **結論**: ctx を持たない metadata-only の `real_catalog` module
  （`RUN_ID`/`SCENARIO`/`LANE`/`DRIVER`/`REAL_CATALOG`）を非
  feature-gate で新設。`real_dock.rs` は `RUN_ID`/`SCENARIO`/lane
  文字列を `real_catalog` から共有し、feature 付き build の
  `real_catalog_matches_entries` test が `real_entries()` との
  case/scenario/required 一致を固定する。console は GPUI 型を
  一切 import しない。

## 試行 3: `--include-real` を非 `real-dock` build でどう扱うか

- **目的**: real lane が compile-in されていない build で
  `--list --include-real` を実行したときの正直な振る舞い。
- **観測**: metadata table（`real_catalog`）は全 build にあるが、
  「real lane が利用可能」という意味ではない。行を出すと「この
  build で real lane がある」かのような誤認を与え、行を落とすと
  要求を静かに無視する。
- **結論**: exit 2 で拒否し「`real-dock` feature が必要」と
  stderr に明示。`session_required` 行は「一覧に出るが console
  経路では実行しない」ことを `runnable:null`/`missing:null` +
  `session_required:true` で表現し、verdict 語彙は出さない。

## 試行 4: selector 検証の順序と OR/AND 意味論

- **目的**: `--case`/`--scenario`/`--driver` の fail-closed 規則を
  一貫させる。
- **観測**: real case 名（`real-*`）は headless catalog に存在
  しないため、unknown 判定を先に行うと「unknown case」として
  誤案内され、real lane への誘導が消える。また flag 内 OR・flag
  間 AND は catalog の `EntryProjection` filter と同じ語彙に
  揃えるのが自然。
- **結論**: 検証順序を real-lane 認識 → 各値の unknown → 組合せの
  空判定に固定。`cell_selected` を 1 関数に集約し、事前 count
  （空判定）と `run_selected` の predicate が同一述語を共有する
  ため「検証した集合」と「実行した集合」が構造的に一致する。

## 試行 5: JSONL `lane` field の provenance

- **目的**: `--run` の部分実行出力が full lane の証拠
  （`test-ui-plugin-dock`）と混同されないこと。
- **観測**: run id は `rf631-plugin-dock` を共有するのが自然
  （同じ scenario・同じ case body）だが、lane field が同一だと
  「8 case 全部走った」証拠と「1 case だけ走った」出力を
  downstream が区別できない。
- **結論**: `lane:"plugin-dock-console"` を header に記録。
  `plugin-dock-scenario.jsonl`（lane）と
  `plugin-dock-console.jsonl`（console）は別 file・別 lane 値で
  保存し、evidence の読み手が provenance を誤認しない形にした。
