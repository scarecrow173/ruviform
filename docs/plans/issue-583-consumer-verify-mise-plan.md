---
id: "PLAN-034Wv7IN094RjMqLSM1Bzx"
type: "plan"
status: "approved"
title: "Issue #583 実施計画 — mr_boxington 失敗の原因確定・検証・記録"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/583"
    - "docs/specs/issue-583-consumer-verify-mise.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-583-consumer-verify-mise-plan.md"
      - type: "file"
        path: "docs/tasks/issue-583-consumer-verify-mise.md"
      - type: "file"
        path: "docs/test-specs/issue-583-consumer-verify-mise.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
  implements:
    - "docs/specs/issue-583-consumer-verify-mise.md"
  implemented-by:
    - "docs/tasks/issue-583-consumer-verify-mise.md"
  depends-on:
    - "docs/specs/issue-583-consumer-verify-mise.md"
    - "docs/designs/issue-583-consumer-verify-mise-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-583-consumer-verify-mise.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-583-consumer-verify-mise-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-583-consumer-verify-mise.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #583 実施計画 — mr_boxington 失敗の原因確定・検証・記録

## Goal

consumer-verify preflight 失敗の直接原因を確定し、現行 main での回帰がないことをローカル再現・mise install 検証・workflow_dispatch の 3 層で確認して記録する。設定変更は行わない(design の「設定変更なし」判断に基づく)。

## Scope

単一 stream。変更は doc-chain 5 件と各 docs dir の生成 index のみ。`mise.toml` / `mise.ci.toml` / workflow / 製品コードは変更しない。

## Upstream Documents

- spec: `docs/specs/issue-583-consumer-verify-mise.md`
- design: `docs/designs/issue-583-consumer-verify-mise-design.md`
- test-spec: `docs/test-specs/issue-583-consumer-verify-mise.md`

## Steps

1. doc-chain(spec/design/plan/task/test-spec)を作成する。
2. 失敗 run(43140257916)と ci run(43140254535 / 43270011255)のログを取得し、エラー箇所・lane 差・修正済み根拠を確定する。
3. 旧設定の再現: sha `4f31460e` の `mise.toml`/`mise.lock` を clean global config で mise 2026.11.7 windows-x64 にかけ、`mise install --locked` が同一エラーで exit 1 することを確認する。
4. 現行 main の検証: mise 2026.11.7 で `mise install --locked node`、`MISE_ENV=ci mise install --locked`、`MISE_ENV=ci mise install --locked rust git-lfs node`、fresh `MISE_DATA_DIR` での `MISE_ENV=ci mise install --locked node` を実行し exit code を記録する。
5. `git fetch origin main && git merge origin/main` で最新 main を取り込み、触った docs dir の管理 index を `doc-maintenance` の index apply で再生成する。
6. Conventional Commits で commit → `git push -u origin bugfix/583-consumer-verify-mise` → `gh pr create`(`Refs #583`)。
7. `gh workflow run consumer-verify.yml --ref bugfix/583-consumer-verify-mise` で dispatch し、`gh run watch` で preflight job の success を確認。結果 URL を PR 本文に追記する。

## File map

- 新規: `docs/{specs,designs,plans,tasks,test-specs}/issue-583-consumer-verify-mise*.md`(5 件)
- 再生成: `docs/{specs,designs,plans,tasks,test-specs}/README.md`(管理 index)
- 変更なし: `mise.toml` / `mise.ci.toml` / `mise.lock` / `mise.ci.lock` / `.github/workflows/*` / 製品コード

## Verification matrix

| 検証 | コマンド | 合格条件 |
| --- | --- | --- |
| 旧設定の再現 | clean env で mise 2026.11.7 `mise install --locked`(@4f31460e config) | exit 1 + `mr_boxington` エラー(CI と同一) |
| node install | `mise install --locked node` | exit 0 |
| CI env install | `MISE_ENV=ci mise install --locked rust git-lfs node` | exit 0 |
| 全 tool install | `MISE_ENV=ci mise install --locked` | exit 0(7 tool 解決) |
| fresh 解決 | 空 `MISE_DATA_DIR`/`MISE_CACHE_DIR` で `MISE_ENV=ci mise install --locked node` | node@26.12.1 を download・install |
| 実 CI preflight | `gh workflow run consumer-verify.yml --ref <branch>` → `gh run watch` | preflight job = success |
| index 整合 | `doc_maintenance.js index apply`(引数なし) | exit 0 |

## Risks

- workflow_dispatch の consumer lane(audiolens/synvelis)が mise 以外の要因で失敗する可能性 → preflight job の success をもって本 Issue の合格条件とし、lane 個別失敗は証拠つきで切り分けて PR に記録する。
- docs index 再生成が他 Worker の並行変更と競合する可能性 → merge 後に再生成して同一コミットに含める。
