# Issue #192 TA Invariant Map — 監査手法の試行記録（2026-11-14）

`docs/tasks/issue-192-ta-invariant-map-task.md` の文書監査で試行した手法と
結果の記録。コード変更を伴わないため、実験は「監査手法の選択と補正」に
限定される。正本の監査結果は
[`docs/discovery/issue-192-ta-invariant-map.md`](../../discovery/issue-192-ta-invariant-map.md)
を参照。

## 試行 1: `cargo metadata` の dep_kinds 解釈

- **目的**: workspace member 間の normal/build/dev edge を正確に集計し、
  feature ゲート optional edge と dev-only edge を区別する。
- **試行**: `resolve.nodes[].deps[].dep_kinds` を走査し、`kind` が
  `null`/`"normal"`/`"build"`/`"dev"`、`target` が `null`/cfg 文字列で
  あることを確認。
- **観測**: 初回スクリプトは `kind = null` を non-normal として扱い
  normal edge を 0 と誤報告した。`dep_kinds` の `kind: null` は normal
  dependency を意味する（serde 表現）。
- **結論**: `kind` が `null` または `"normal"` を normal、`"build"` を
  build、`"dev"` を dev として分類し直して再集計。default 98 /
  all-features 103 normal+build edge・dev 26・循環 0 を得た。
  採用（discovery §依存非循環性監査の方法に反映）。

## 試行 2: manifest 宣言 vs resolver 活性の区別

- **目的**: M0（#194）が記録した「manifest 宣言 67 edge」と現行の
  「resolver 活性 edge」の差分を説明する。
- **試行**: `packages[].dependencies`（manifest 宣言・`optional`/`kind`/
  `target` 属性付き）と `resolve.nodes[].deps[].dep_kinds`（活性 edge）を
  突合。
- **観測**: all-features でのみ活性化する ws→ws edge が 5 件
  （plugin-{clap,vst3}→plugin-export、poc-native-dock→plugin-vst3、
  ui-gpui→{ui-test,ui-theme}）。default では optional feature 未活性の
  ため edge に現れない。M0 の `host` ゲート 2 edge と整合。
- **結論**: 宣言↔解決の差分は feature ゲート optional edge の活性化で
  説明可能。default/all-features の 2 集合で報告する方式を採用。

## 試行 3: dev-dependency 上位参照の循環誤認回避

- **目的**: `plugin-host → ruviform-audio` 等の dev-dep を umbrella §3 の
  禁止通常依存と誤認しない。
- **試行**: 全 kind 込みの edge で DFS 循環検出を実行。
- **観測**: dev 込み全 kind でも循環 0。dev-dep の上位参照
  （plugin-host→audio、plugin-{clap,vst3}→plugin-host、io-e2e の横断
  dev 依存）は存在するが循環を構成しない。umbrella §3 は「既存テスト用
  dev-dependencies の上位参照を同じ意味の循環と扱わない」と規定。
- **結論**: kind 別集計（normal+build を本番 DAG、dev を別集計）で報告。
  plugin-host→audio は dev-only として TA-005 の根拠に採用。

## 試行 4: 文書生成スクリプトの出力先

- **目的**: discovery/spec 等の skeleton を正しい `docs/<type>/` に生成。
- **観測**: `new_discovery.js` 等を skill package パスから実行した際、
  出力が `.agents/skills/<skill>/docs/<type>/` に生成された（cwd 依存）。
  リポジトリ root から `--dir docs/<type>` を明示して再生成・配置した。
- **結論**: 生成スクリプトは必ずリポジトリ root を cwd とし `--dir` で
  出力先を明示する。生成物の front matter は script 出力を正とし、
  手動で relations/status を整合させた。

## 試行 5: `/tmp` パスの Windows/Git Bash 不整合

- **観測**: `cargo metadata` 出力を `/tmp/cargo-meta.json` に書き、
  Windows Python で読むと `FileNotFoundError`。Git Bash の `/tmp` と
  Windows Python の `/tmp` 解決が異なる。
- **結論**: `C:\Users\akiyoshi\AppData\Local\Temp\` 配下の実パスを
  Python に渡す方式に変更。以降の集計は全て成功。

## 結論

全試行で得られた手法・補正は discovery 正本 §Audit Method and Evidence
Basis・§依存非循環性監査に反映済み。棄却した代替手法（`cargo tree`・
CI 実走証拠収集・crate 全読み深掘り）は discovery §Alternatives and
Comparison に記録。
