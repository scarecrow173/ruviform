---
id: "TASK-034RBIMQzuYAewg2JXTXOk"
type: "task"
status: "done"
title: "T1 mise task install/freshness 分離 + ui-verify setup ステップ（Issue #330）"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/330"
  changes:
    added: []
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
    - "docs/plans/issue-330-ui-npm-ci-dedup.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-330-ui-npm-ci-dedup.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-330-ui-npm-ci-dedup.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-330-ui-npm-ci-dedup.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 mise task install/freshness 分離 + ui-verify setup ステップ（Issue #330）

## Work

- `mise.toml` に `install-ui-{core,components,viz,binding,guitest,showcase,browser-test}` を新設（`run` = `npm --prefix ui/<pkg> ci --no-audit --no-fund`、`sources` = `ui/<pkg>/package-lock.json` + `ui/<pkg>/package.json`、`outputs` = `ui/<pkg>/node_modules/.package-lock.json`）。
- `mise.toml` に `setup-ui` 集約タスクを新設（`depends` = install-ui-* ×7、run なし）。
- 全 `check-ui-*` / `test-ui-*` / `build-ui-*` タスクの `run` から `npm ci` 行を除去し、対応する `install-ui-*` へ `depends` を付与。`check-ui-browser-test` / `test-ui-browser` は `install-ui-browser-test` + `build-ui-showcase` 両方へ。
- `build-ui-{components,viz,binding,showcase}` に `sources`（自 package src + compile include の sibling src + tsconfig*.json）と `outputs`（`ui/<pkg>/dist/**/*`）を付与。
- `.github/workflows/ci.yml` の `ui-verify` で cargo cache 復元後に `Install UI npm dependencies (mise run setup-ui)` ステップを挿入。
- `AGENTS.md` の検証入口テーブル（UI npm 系 17 行）と `ui-verify` 説明文を新構造へ同期。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-330-ui-npm-ci-dedup.md`
- `docs/designs/issue-330-ui-npm-ci-dedup.md`
- `docs/specs/issue-330-ui-npm-ci-dedup.md`

## Dependencies

- なし（先行タスクなし。CI-OPT-00 #327 は計測テンプレート参照のみ）。

## Affected Files

- `mise.toml`
- `.github/workflows/ci.yml`
- `AGENTS.md`

## Done When

- [x] `mise run setup-ui` が 7 package の `npm ci` を各 1 回実行する（fresh 時）。
- [x] verify タスクの `depends` 解決で install が skip され検証のみ走る（warm 時）。
- [x] `mise run test-ui-browser` が showcase dist 再利用で Playwright 緑。
- [x] node_modules 削除後の `mise run test-ui-headless` が依存解決込みで完走。
- [x] ci.yml が actionlint 通過、AGENTS.md と実装が一致。

## Verification

- [x] node_modules 全削除 → `mise run setup-ui` → npm ci 7 回（並列、wall ~20s）。再実行で全 skip（~0.5s）。
- [x] `mise run check-ui-headless`（install-ui-core skip + typecheck 緑）。
- [x] `mise run test-ui-browser`（install×2 + build-ui-showcase skip + suite 30 件緑）。
- [x] node_modules 削除後の単独実行 `mise run test-ui-headless` 緑（depends 経由 install → test 67 件）。
- [x] `mise run test-ui-showcase`（deps.test.ts DAG audit 含む 73 件緑）。
- [x] `mise exec -- actionlint .github/workflows/ci.yml`・`mise exec -- prek run --all-files` 緑。
