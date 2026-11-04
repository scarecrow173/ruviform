---
id: "DISC-3qnA5E8SxxIxxEfA9LSI1k"
type: "discovery"
status: "resolved"
title: "Issue #333 CI-OPT-06 変更影響ベース検証 DAG・Fail-Closed Required Gate の棚卸し"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/333"
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/ruviform/issues/326"
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
  related:
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
    - "docs/designs/issue-331-check-clippy-test-dedup.md"
    - "docs/adr/cross-repo-ci-revision.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-333-impact-based-ci.md"
    - "docs/designs/issue-333-impact-based-ci.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  defers: []
  deferred-by: []
metadata:
  discovery:
    confidence: "high"
---

# Issue #333 CI-OPT-06 Impact Classifier・集約 Required Gate の棚卸し

## Summary

`ci.yml` は 6 job / 12 required contexts で PR・main push・手動の全 trigger で
全 lane を毎回実行している。Issue #333 は変更ファイルから必要検証集合を決める
汎用 Impact Classifier と、選択実行を安全にする集約 required gate の設計を求める。
本棚卸しは (a) required contexts と lane↔検証単位の対応、(b) 分類に使える
依存 DAG の機械取得源、(c) GitHub skipped/required の意味論、(d) 段階移行で
壊してはいけない境界を確定する。

## Exploration Goal

- 12 required contexts の各々が「何を検証するか」を job/mise task 単位で固定する。
- 変更ファイル→lane 写像に必要な依存情報を、静的リストではなく
  リポジトリから機械導出できる形で特定する（導出元の陳腐化 = 偽陰性リスク）。
- GitHub Actions の `needs`/`skipped`/required check の意味論を前提整理し、
  fail-closed gate の判定規則の土台を作る。

## Key Issues and Assumptions

- required contexts（ruleset `main-protect` id 26529830・active）は
  `verify`/`l1-real-verify`/`ui-verify` の 3OS matrix 各 3 + `rt-verify` +
  `icc-verify` + `bench` の計 12。context 名は job `name` と一致する。
- `verify` job は prek（全ファイルの言語非依存 hook）+ fmt-check + lint-ci +
  test-ci を持つ唯一の lane で、ワークスペース全 member の compile/lint/test
  （default feature）を網羅する。**prek は docs/*.md の trailing whitespace・
  EOF も検査するため、docs-only 変更でも verify は意味を持つ**（`prek` 専用
  job は存在しない）。
- `rt-verify`（ubuntu）: `mise run test-rt` = `cargo test --workspace rt_`。
  rt_ プレフィックスの RT 契約テストを持つ crate が対象。実測スキャンで
  `fn rt_` を持つ crate は 17 個（audio, audio-analysis, audio-core,
  audio-device, audio-plugin-bridge, dsp, math, media-core, media-reader,
  numeric, observability, plugin-clap, plugin-core, plugin-host, plugin-vst3,
  rt, video-core, video-proc）。
- `icc-verify`（ubuntu）: `test-video-proc-icc`/`lint-video-proc-icc` =
  `ruviform-video-proc --features icc` のみ。
- `ui-verify`（3OS）: Rust 側 `ruviform-ui-webview --features webview` と
  `ui/{core,components,viz,binding,guitest,showcase,browser-test}` の
  npm 7 pkg（check/test/build + Playwright 実ブラウザ）。
- `l1-real-verify`（3OS）: `ruviform-plugin-clap`/`ruviform-plugin-vst3`
  `--features host` の build + `--lib --bins` test + `#[ignore]` E2E + lint。
- `bench`（ubuntu）: `bench-smoke` = `cargo bench -p ruviform-math
  -p ruviform-rt -p ruviform-viz -- --test`。**benches ディレクトリを持つ
  crate（math/numeric/rt/viz）≠ bench-smoke 対象（math/rt/viz）** — numeric の
  benches は verify の `--all-targets` で compile されるが実行はされない。
- Cargo workspace member は `crates/*` の 33 crate で `members` 明示列挙。
  `poc/plugin-gui`・`poc/ui` は workspace 外で CI の cargo lane に入らない。
- ui/* 相互依存は package.json deps ではなく (a) tsconfig `include`/`paths`
  の `../<pkg>` 参照（guitest→binding/core・browser-test→showcase/dist）、
  (b) `../<pkg>/src` 相対 import（components→core 等）、(c) mise task の
  `sources`/`depends` の `ui/<pkg>` 参照（build-ui-* tasks）の 3 系統に分布。
- `cargo metadata --no-deps --format-version 1` で workspace member と
  member 間 path 依存を機械取得可能（依存逆方向伝播の機械化源）。
- `pull_request` 既定 activity type は opened/synchronize/reopened のみ。
  base retarget は `edited` で発火するため、既定では base 変更で再分類されず
  stale な check が残り得る。draft→ready は `ready_for_review`。
- `push: main`/`workflow_dispatch` は Full CI 維持が契約（Issue 実装範囲 3）。
- 別 workflow `consumer-verify.yml`（#219 dispatch 専用・非 required）と
  `consumer-impact-check.yml`（PR 本文機械検査・非 required）は独立であり、
  本 Issue の lane 選択とは無関係に維持する（欠落禁止の Acceptance）。

## Alternatives and Comparison

- **(a) `dorny/paths-filter` 等の既存 action**: glob→boolean は合うが
  crate 逆依存伝播・ui DAG・lane 契約の監査可能出力は自前になる。
  third-party action を新規 pin するコストに対し、ロジックの大半が残る。
- **(b) リポジトリ内 stdlib-only Node スクリプト（採用候補）**:
  `ci/metrics/collect-ci-metrics.mjs` と同型（`.mjs`・依存なし・`node --test`
  で fixture 検証可能）。分類規則を data として reviewable に保て、
  Consumer 側が contract を再利用しやすい。
- **(c) 分類表の完全静的メンテ**: 33 crate×lane の静的リストは crate 追加/
  依存変更で陳腐化し静かな偽陰性になる。cargo metadata・rt_ スキャン・
  mise task の `-p` 引数から導出する方が fail-closed。

## Tentative Conclusions and Hypotheses

- lane 選択単位は **job 粒度**（matrix leg 分割は非目標 — 複雑性に対し
  利得が薄い）。期待出力は lane×OS 行列を情報として保持する。
- classifier 失敗時は lane 側を fail-open（`if` は「明示的に要らない時だけ
  skip」）し、gate が分類自体の健全性を fail-closed で検査する二層にする。
- 集約 gate は「expected(impact 出力) と actual(`needs` results) の純粋関数」
  にすると、shadow 期（全 lane 実行）と enforce 期（選択実行）で同一コードが
  機能する — mode 切替を gate に持たせない。
- 緊急 rollback は repo variable `CI_IMPACT_DISABLE=1` で classifier が
  expected=ALL に退化（sccache の `CI_SCCACHE_DISABLE=1` と同型、
  workflow 変更不要）。
- Shadow 期に「classifier が不要と判定した lane が失敗した」事象 =
  偽陰性シグナルとして gate が表面化できる（全 lane 走る shadow だから
  観測可能）。

## Open Questions

- gate が per-OS 情報を出す粒度: `needs.<matrix job>.result` は leg 集約値
  （fail-fast: false で全 leg 実行後の合否）のみ。per-leg 内訳は各 leg 自身の
  check run に任せ、gate は job 粒度の expected/actual/reason を summary 表に
  出す（checks API 追加権限を避ける）— design で確定。
- base retarget 再検証に `edited` を trigger 追加するか（title/body 編集でも
  全 lane が走る代償）— design で確定。
- `poc/`（workspace 外）・ルート dotfile 類の分類: 影響 lane が実質 prek のみ
  → docs 同様 verify のみ、または共有設定として full — design で確定。

## Promotion Candidates

- spec: 実装範囲 1-8 を要件化、Acceptance を受入基準へ写像。
- design: 分類契約（class→lane 表・伝播規則・fallback 規則）、gate 判定規則
  （Pass/Fail/NotRun/Unsupported/NotApplicable）、Ruleset 段階移行 runbook、
  rollback、edge cases。
- test-spec: classifier fixture マトリクス + gate 判定マトリクス + shadow
  期の運用検証規約。

## Sources and Relations

- Issue: #333（本体）、#327（baseline 計測契約）、#326（ruleset 移行追跡 —
  解消済み、ruleset id 26529830 active）、#219/#294（Consumer 検証・独立維持）。
- 参照設計: `docs/designs/issue-327-cross-repo-ci-baseline.md`（Before/After
  計測契約・証拠語彙）、`docs/adr/cross-repo-ci-revision.md`（fail-closed・
  not run を pass にしない原則）。

## Evidence

### 2026-11-04 ローカル棚卸し（Windows 11 x64 / Git Bash）

- `gh api repos/scarecrow173/ruviform/rulesets/26529830`: required_status_checks
  の context は上記 12 件と一致（pull_request rule は review 0 件・
  thread resolution 必須・merge/squash/rebase 許可）。
- `ci.yml` 実読: 6 job、全 trigger で条件分岐なし。`concurrency` は
  PR 単位 group + cancel-in-progress（Issue #328）。
- `mise.toml` 実読: 各 feature lane の `-p` 対象と feature は上記のとおり。
  `bench-smoke` は `-p ruviform-math -p ruviform-rt -p ruviform-viz`。
- `fn rt_` スキャン実測: 17 crate（上記 Key Issues 欄）。
- ui/* 相互依存実測: components→core（`../../core/src` import）、
  guitest→binding+core（tsconfig include）、browser-test→showcase/dist
  （tsconfig paths）、showcase→core/components/viz/binding（mise task
  `sources` 記述どおり）。
- ワークスペース外: `poc/plugin-gui`、`poc/ui` に Cargo.toml あり・
  `members` 未収録。
