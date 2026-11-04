---
id: "IMPL-034RBWydgCTGFLeFHqAEQp"
type: "impl"
status: "completed"
title: "Issue #330 CI-OPT-03 mise task install/freshness 分離 + ui-verify setup ステップ"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/330"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-330-ui-npm-ci-dedup.md"
      - type: "file"
        path: "docs/designs/issue-330-ui-npm-ci-dedup.md"
      - type: "file"
        path: "docs/plans/issue-330-ui-npm-ci-dedup.md"
      - type: "file"
        path: "docs/test-specs/issue-330-ui-npm-ci-dedup.md"
      - type: "file"
        path: "docs/tasks/issue-330-t1-mise-ui-install-dedup.md"
      - type: "file"
        path: "docs/impl/ir/issue-330-ci-opt-03-mise-task-install-freshness-ui-verify-setup.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-330-t1-mise-ui-install-dedup.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-330-ui-npm-ci-dedup.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-330-ui-npm-ci-dedup.md"
    - "docs/designs/issue-330-ui-npm-ci-dedup.md"
    - "docs/test-specs/issue-330-ui-npm-ci-dedup.md"
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
    adopted: []
    rejected: []
---

# Issue #330 CI-OPT-03 mise task install/freshness 分離 + ui-verify setup ステップ

## Summary

`ui-verify` ジョブ内で lockfile ごとに重複していた `npm ci`（計 20 回/job・showcase 最大 5 回）を、freshness 付き `install-ui-*` タスク（7 本）と集約 `setup-ui` へ分離し、verify 系タスクは `depends` 経由で準備済み `node_modules` を再利用する構造に変更した。`build-ui-*` にも sources/outputs を付与し、showcase dist の同一 job 内再 emit（3 回→1 回）を排除した。

## Context

task `TASK-034RBIMQzuYAewg2JXTXOk`（plan `PLAN-034RBIEnfNn4DeMAPw27Ii`、spec `SPEC-034RBIrH4H2hFNw7gfmgDa`、design `DESIGN-034RBIy0yxPWYhXdXUlsLi`、test-spec `TSPEC-034RBIMbJjTnaygGjWGrGx`）として実施。Issue #330 CI-OPT-03、#327 の Before/After テンプレートに整合する計測を併記。

## Implementation

### Changes

- `mise.toml`
  - `install-ui-{core,components,viz,binding,guitest,showcase,browser-test}` 新設。`run` は `npm --prefix ui/<pkg> ci --no-audit --no-fund` のみ、`sources` = `package-lock.json` + `package.json`、`outputs` = `node_modules/.package-lock.json`（npm ci が必ず書く hidden lockfile マーカー）。
  - `setup-ui` 新設。7 本を `depends` で集約（run なし）。CI 準備ステップ用。
  - 17 本の `check-ui-*`/`test-ui-*`/`build-ui-*` から `npm ci` 行を除去し、対応 `install-ui-*` へ `depends` を付与。`check-ui-browser-test` / `test-ui-browser` は `install-ui-browser-test` + `build-ui-showcase` へ。
  - `build-ui-{components,viz,binding,showcase}` に `sources`（自 package + tsconfig.build include の sibling src + tsconfig*.json + package.json）と `outputs` = `ui/<pkg>/dist/**/*` を付与。
- `.github/workflows/ci.yml`: `ui-verify` に `Install UI npm dependencies (mise run setup-ui)` ステップを cargo cache 復元直後へ挿入。Playwright system deps コメントを新構造に合わせて更新。検証 step 分割・順序・Evidence upload は不変。
- `AGENTS.md`: 検証入口テーブルに `install-ui-*` / `setup-ui` 行を追加し 17 行を新構造へ更新。`check-ui-browser-test`/`test-ui-browser` 行を補完（既存欠落分）。`ui-verify` 説明文に setup-ui 集約と browser-test lane を明記。

### Decisions During Implementation

- freshness 判定は mise 標準の sources/outputs freshness check を採用（独自スクリプト・artifact cache 実験機能は不使用）。`node_modules/.package-lock.json` を output マーカーに選定 — npm ci 成功時に必ず生成され、部分 install や node_modules 削除で stale になる。
- check/test 系タスクには freshness を付与しない — 検証 skip は検証能力の削減に相当するため。freshness は「副作用を持つ準備・emit」系タスクのみに限定。
- `package.json` も sources に含めた — manifest 変更時に install が stale となり、`npm ci` 自身が lock/manifest 不整合を fail 報告できる。
- verify タスク名・run 内容・CI step 分割は互換維持（移行ガイド不要の互換経路を選択）。

## Related Experiments

### Adopted

- mise freshness 挙動の scratch 検証（/tmp/mise-skip-test）: sources/outputs + depends で「初回 run→skip→source 更新で再 run→output 欠落で再 run」を確認後に採用。

### Rejected

- npm workspaces 化 / lockfile 一本化: package 境界・private lockfile 契約の再設計を伴い Issue #330 の必須範囲外（spec Deferred Design Concerns へ記録）。
- node_modules actions/cache: lockfile hash key 設計・部分復元整合が別途必要なため今回は見送り（同）。

## Validation

実測環境: Windows 11 x64 / Git Bash、mise 2026.10.1、node 26.12.1、npm 11.20.0（いずれも pin toolchain）。

| 項目 | 実行 | 結果 |
| --- | --- | --- |
| `mise run setup-ui`（node_modules 全削除後） | npm ci ×7（並列） | Pass — 7 package 各 1 回、wall 約 20s |
| `mise run setup-ui`（直後 warm） | — | Pass — 全 7 本 `sources up-to-date, skipping`、~0.5s |
| `mise run check-ui-headless` | install-ui-core skip + tsc | Pass |
| `mise run test-ui-headless` | 同上 + node --test | Pass 67/67 |
| `mise run test-ui-components` / `test-ui-viz` / `test-ui-binding` / `test-ui-guitest` / `test-ui-showcase` | install skip + node --test | Pass 94/89/74/135/73（showcase は deps.test.ts DAG audit 含む） |
| `mise run build-ui-{components,viz,binding,showcase}` | install skip + tsc emit | Pass。再実行は全 skip |
| `mise run check-ui-browser-test` | install×2 + build-ui-showcase skip + tsc | Pass。`ui/showcase/dist` 削除時は build-ui-showcase が stale→再 emit→typecheck Pass |
| `mise run test-ui-browser` | 同上 + page build + playwright install + suite | Pass 30/30（Chromium） |
| node_modules 削除後の単独 `mise run test-ui-headless` | depends 経由 install-ui-core → test | Pass（Fresh checkout 再現相当） |
| `mise exec -- actionlint .github/workflows/ci.yml` | — | Pass（exit 0） |
| `mise exec -- prek run --all-files` | — | Pass（check-toml/check-yaml 含む 8 hook） |

Before/After（#327 テンプレート整合、本機 warm npm cache 実測 + 回数は ci.yml/mise.toml からの機械的集計）:

| 指標 | Before | After |
| --- | --- | --- |
| npm ci 実行回数 / ui-verify job | 20（core2/components3/viz3/binding3/guitest2/showcase5/browser-test2） | 7（package-lock ごとに 1 回、setup-ui 並列） |
| npm ci 時間合計（本機 warm cache 実測） | 約 86s 逐次 | 約 27s 逐次換算 / setup-ui 並列 wall 約 20s |
| ui/showcase dist emit / job | 3（build step + depends×2） | 1（sources 不変なら depends 側 skip） |
| warm 再実行の install | 毎回 npm ci（lockfile wipe+reinstall） | 全 skip（~0.5s） |
| 検証 step 分割 / Evidence / required checks | — | 不変 |

CI 3 OS matrix・workflow wall time の実測は PR CI run での確認事項（ローカル検証は Windows のみ、Linux/macOS は未検証として報告）。

## Risks

- mise task-state（`~/.local/state/mise/task-sources`）が self-hosted runner で job 跨ぎ永続化される場合、freshness 判断に「前回 install 成功」の記録が残り得る。output 実在性も条件のため node_modules 非存在の job では必ず install が走り、GitHub-hosted runner（現行・job ごと新規 VM）では問題にならない。
- `node_modules` の部分破損（`.package-lock.json` 残存・中身欠損）は freshness を通り得る。発生時は `mise run --force` で強制再実行可能。
- `build-ui-*` の freshness は dist 部分削除を検出しない（dist 全体欠落は stale）。完全性保証は従来の tsc 運用と同等。

## Follow-ups

- PR CI run で 3 OS matrix の同等性（typecheck/test/build/Evidence）と workflow wall time の Before/After を採取し Issue #330 へ報告。
- CI-OPT-05（#332）: Playwright ブラウバイナリ・`playwright install-deps` のキャッシュ/集約余地の評価。
