# Issue #465 WebView 移行台帳現行化 — 試行記録（2026-11-14）

`docs/tasks/wv-removal-inventory-task.md` の監査・台帳化で試行した
手法と結果の記録。コード挙動の変更を伴わないため、実験は
「証拠収集・分類手法の選択と補正」に限定される。正本は
[`evidence/rf465/inventory.json`](../../../evidence/rf465/inventory.json)
と [IR](wv-removal-inventory.md) を参照。

## 試行 1: GitHub code search による consumer 列挙

- **目的**: AL/SV の WebView 参照を `gh api search/code` で列挙する。
- **試行**: `repo:scarecrow173/{audio-lens,synvelis} webview`、
  `ruviform-ui-webview`、`rvf`、`filename:Cargo.toml` 等の query で
  `search/code` API を実行。
- **観測**: すべて `total_count: 0` かつ `incomplete_results: true`。
  repo は public で `gh repo view` は成功するが、code search index が
  当該 repo を返さない（index 遅延または対象外の可能性）。
- **結論**: file-level 列挙は origin/main HEAD の clean checkout
  （`F:/repositry/audio-products/{audio-lens,synvelis}`、fetch 済み・
  `git status` clean）を正本とし、`gh issue view` のみ API で補完。
  台帳の `enumeration`/`README.md` 収集方法に手法と制約を明記。
  read-only であり consumer repo への変更はなし。

## 試行 2: P0/P1 との差分抽出範囲の決定

- **目的**: PR #475（merge `4e59494a`）以降の差分を網羅的に拾う。
- **試行**: `git log --oneline 4e59494a..HEAD -- crates/ruviform-ui*
  crates/ruviform-viz crates/ruviform-poc-native-dock ui/ poc/ mise.toml
  .github/workflows/ Cargo.toml` で path 限定の commit 一覧を取得し、
  続けて `ui/ poc/ crates/ruviform-ui-webview/` に絞って再確認。
- **観測**: `ruviform-ui-webview`/`poc/` は 0 commit。`ui/` は
  binding/guitest の契約修正 3 件（#377/#378/#490）— package 構造は
  不変だが「保守継続中」の証拠となる。driver-neutral 側に #464/#313/
  #314/#243/#552/#598/#418/#429 系の大規模追加。
- **結論**: 差分は「WebView 資産の増減」ではなく「driver-neutral
  側の拡張が台帳の reuse 側を増やした」構造。entry の
  `diff_vs_p0p1` に個別記録し、README に集約した。

## 試行 3: 分類単位の決定（package vs 契約）

- **目的**: `ui/guitest`・`poc/`・`ipc_conformance.rs` のように
  共有部分と legacy 部分が同居する対象の分類を決める。
- **試行**: `crates/ruviform-ui/tests/ipc_conformance.rs` が
  `../../ui/guitest/fixtures` を直接読むこと、`ipcwire.ts` が
  `parse_intent`/`serve_data` を mirror することを確認。
- **観測**: package 単位で `ui/guitest` を legacy 撤去候補にすると、
  Rust conformance test が壊れる — P0/P1 の 3 分類では表現できない
  分離点。
- **結論**: 分類単位を「削除で壊れる契約」へ変更し、
  `ui/guitest/fixtures`→`reuse`、`ui/guitest/src`→`provisional`、
  `poc/*` コード→`remove_candidate`・RESULTS/ADR→`exception` に分割。
  `ipc_conformance.rs` は entry notes に schema/`rvf://data` の
  内訳を記録（P2 時に検証対象単位で再分割が必要）。

## 試行 4: consumer の webview lane 存続状況の確認

- **目的**: SV の webview bin が CI で検証されているか確認する。
- **試行**: `grep -rin "webview" .github/workflows/*.yml`（SV）、
  `grep -n "webview\|gpui" .github/workflows/ci.yml`（AL）。
- **観測**: SV は workflow 内 webview 参照 0 件（mise task のみ残存）。
  AL は `webview-verify`/`webview-verify-macos` job + composite
  action で `check/lint/test-ui-webview` を full CI 実行中。
- **結論**: consumer 側の「撤去しやすさ」が非対称 — SV は CI 証拠
  なしで local lane のみ、AL は CI で現行検証中。台帳の
  `retirement_trigger`/notes に反映した。
