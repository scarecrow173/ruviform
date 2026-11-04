---
id: "DISC-034RC37YxqeDsq3Kad550u"
type: "discovery"
status: "resolved"
title: "Issue #330 CI-OPT-03 UI npm ci 重複・dist 依存 DAG 棚卸し"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/330"
    - "https://github.com/scarecrow173/ruviform/issues/327"
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
    - "docs/designs/issue-310-browser-gui-test-harness.md"
    - "docs/designs/issue-311-ipc-conformance-replay.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-330-ui-npm-ci-dedup.md"
    - "docs/designs/issue-330-ui-npm-ci-dedup.md"
  verifies: []
  verified-by: []
  references:
    - "mise.toml"
    - ".github/workflows/ci.yml"
    - "ui/showcase/test/deps.test.ts"
    - "ui/browser-test/tsconfig.json"
    - "ui/showcase/tsconfig.build.json"
    - "ui/components/tsconfig.build.json"
    - "ui/viz/tsconfig.build.json"
    - "ui/binding/tsconfig.build.json"
  defers: []
  deferred-by: []
---

# Issue #330 CI-OPT-03 UI npm ci 重複・dist 依存 DAG 棚卸し

## Summary

`ui-verify` ジョブ内の `npm ci` 実行回数・package 間 dist 依存・Showcase build 重複を実コードから棚卸しした。現状は verify タスク内包方式で 1 job あたり npm ci 20 回・showcase dist emit 3 回。dist を実際に消費する下流は `ui/browser-test` のみで、その参照先は `ui/showcase/dist` の複合 dist（5 package src 横断 emit）と確定。

## Exploration Goal

Issue #330 実装項目 1「現状の個々の npm ci 実行回数/時間、パッケージ間の dist 依存、Showcase build 重複を採取」に答える。併せて install 分離設計に必要な「どのタスクがどの package の node_modules / どの dist を必要とするか」の実依存を確定する。

## Key Issues and Assumptions

- npm ci 回数: `mise.toml` の各 UI タスク `run` 内の `npm --prefix ... ci` と `depends` 展開（ci.yml の step 順）から機械的に集計 — confirmed（Evidence §1）。
- dist 依存 DAG: `ui/showcase/test/deps.test.ts` の ALLOWED_IMPORTS（監査の正本）+ 各 `tsconfig.build.json` の `include` + `ui/browser-test/tsconfig.json` の `paths` から集計 — confirmed（Evidence §2）。
- node_modules の独立性: src 間 import は全て相対パス（deps.test.ts が bare specifier を禁止）のため、各 package は自 package の node_modules のみ必要 — confirmed。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| mise sources/outputs freshness + depends で install 分離 | 標準機能のみ・独自機構なし・単独実行と CI 集約を両立 | mise バージョン依存（pin 済み 2026.10.1 で安定） | yes |
| npm workspaces / lockfile 一本化 | install 回数を 7→1 に更に圧縮 | package 境界・private lockfile 契約の再設計・Issue の必須範囲外 | no（defer） |
| 個別の if-guard スクリプト | 単純 | 第 2 の独自ビルドシステム化・OS 間差の保守 | no |
| actions/cache で node_modules 共有 | fresh job の install 自体を削減 | lockfile hash 設計・部分復元整合が別途必要 | no（defer） |

## Tentative Conclusions and Hypotheses

- confirmed: 1 job あたり npm ci 20 回（core2/components3/viz3/binding3/guitest2/showcase5/browser-test2）。showcase は `build-ui-showcase` の `depends` 経由で browser-test 2 タスクが都度 install+build するため最大 5 回。
- confirmed: showcase dist emit 3 回/job（build ステップ + check-ui-browser-test・test-ui-browser の depends 各 1）。
- confirmed: dist 消費 DAG は `browser-test ← showcase/dist` のみ。`build-ui-{components,viz,binding}` の dist は検証成果物であり他 package から消費されない。
- hypothesis→confirmed: mise freshness は「全 output 存在・最新 output > 最新 source・source 一覧+size+mtime が前回成功時記録と一致」でのみ skip（scratch 実測済み）。
- confirmed: `npm ci` は `node_modules/.package-lock.json` を必ず書く（output マーカーに適格）。

## Open Questions

- 3 OS matrix の実時間差分は CI run でのみ確定（ローカルは Windows のみ）。PR CI で確認。

## Promotion Candidates

- [x] spec-doc needed: install 分離・DAG 保証・互換性要件 → `docs/specs/issue-330-ui-npm-ci-dedup.md` として起票済み。
- [ ] adr-doc needed: なし（mise 標準機能の適用であり新規アーキテクチャ判断ではない）。

## Sources and Relations

- Issue #330（要件源）、#327（計測テンプレート）、#310（browser-test harness）、#263（showcase）、#311（deps.test.ts audit 契約）。

## Evidence

### 2026-11-04 npm ci 回数集計（ci.yml step 順・mise.toml run/depends から機械的集計）

| task（step 順） | npm ci 対象 |
| --- | --- |
| check-ui-headless | core |
| test-ui-headless | core |
| check/test/build-ui-components | components ×3 |
| check/test/build-ui-viz | viz ×3 |
| check/test/build-ui-binding | binding ×3 |
| check/test-ui-guitest | guitest ×2 |
| check/test/build-ui-showcase | showcase ×3 |
| check-ui-browser-test（depends: build-ui-showcase） | showcase, browser-test |
| test-ui-browser（depends: build-ui-showcase） | showcase, browser-test |

計: core2 components3 viz3 binding3 guitest2 showcase5 browser-test2 = **20 回/job**。showcase dist emit = **3 回/job**。

### 2026-11-04 dist 依存 DAG（deps.test.ts ALLOWED_IMPORTS + tsconfig.build include + browser-test paths）

```
core ─ components ─ viz          （src 相対 import、deps.test.ts 監査）
core ─ binding
guitest ─ core, binding          （test-only 基盤・showcase 系と独立）
showcase ─ core, components, viz, binding
  tsconfig.build.json rootDir ".." include: src+../core+../components+../viz+../binding
  → ui/showcase/dist/{showcase,core,components,viz,binding}/src/* を一括 emit
browser-test ─ @rvf/showcase → ../showcase/dist/showcase/src/index.d.ts（dist 宣言依存）
```

- `build-ui-{components,viz,binding}` の dist: 検証成果物（単体 emit 可能性の証明）。他 package からの消費者なし。
- typecheck は全 lane で src を直接走査（noEmit）。dist を必要とするのは browser-test の typecheck/test のみ。

### 2026-11-04 本機 npm ci 実測（Windows・warm cache・逐次）

core 1.6s / components 6.1s / viz 5.3s / binding 5.6s / guitest 1.7s / showcase 5.2s / browser-test 1.4s。現行回数で逐次合計 ≈86s、lockfile 各 1 回なら ≈27s（mise 並列実行で wall ≈20s 実測）。
