---
id: "PLAN-034Smuf3EM4rzdHln1pYaz"
type: "plan"
status: "approved"
title: "Issue #366+#364 Full CI Stage 1 実装計画"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/366"
    - "https://github.com/scarecrow173/ruviform/issues/364"
    - "docs/specs/issue-366-full-ci-stage1.md"
    - "docs/designs/issue-366-full-ci-stage1.md"
  changes:
    added:
      - type: "directory"
        path: "ci/summary"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted:
      - type: "directory"
        path: "ci/impact"
      - type: "file"
        path: ".github/workflows/consumer-impact-check.yml"
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-366-full-ci-stage1.md"
    - "docs/designs/issue-366-full-ci-stage1.md"
  implemented-by:
    - "docs/tasks/issue-366-t1-ci-summary-evaluator.md"
    - "docs/tasks/issue-366-t2-ci-yml-full-ci.md"
    - "docs/tasks/issue-366-t3-consumer-impact-removal.md"
  depends-on:
    - "docs/designs/issue-366-full-ci-stage1.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-366-full-ci-stage1.md"
    - "docs/designs/issue-366-full-ci-stage1.md"
  derived-by:
    - "docs/tasks/issue-366-t1-ci-summary-evaluator.md"
    - "docs/tasks/issue-366-t2-ci-yml-full-ci.md"
    - "docs/tasks/issue-366-t3-consumer-impact-removal.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #366+#364 Full CI Stage 1 実装計画 Implementation Plan

## Goal

`ci.yml` を全 lane 常時実行の Full CI に変換し、`ci-summary` fail-closed 集約・`io-structured-verify` lane・`expected_sha` 契約を導入する（required context・PR/push trigger は不変の安全な第一段階）。

## Scope

1 本の実装ストリーム。T1（評価器）→ T2（workflow 本体 + 選択実行撤去 + docs 同期）→ T3（consumer-impact-check 撤去）の順。T2 と T3 は独立したコミットに分けられるが同一 PR 単位。

## Upstream Documents

- `docs/specs/issue-366-full-ci-stage1.md`（SPEC-034Smu6fnzDQaoN4box3qk・approved）

## Design Inputs

- `docs/designs/issue-366-full-ci-stage1.md`（DESIGN-034SmuehTqDxp5K68M5dSG・approved）

## File Map

- `ci/summary/summary.mjs` — needs JSON 評価・verdict・Job Summary・report JSON 出力（新設）
- `ci/summary/tests/summary.test.mjs` — verdict 行列・expected_sha・report 形状の contract tests（新設）
- `.github/workflows/ci.yml` — `impact` job 撤去・lane `if:`/`needs` 撤去・`io-structured-verify` 新設・`*-macos` 無条件化・`macos-report`/`ci-gate` → `ci-summary`・`expected_sha` input
- `mise.toml` — `test-ci-impact` 削除・`test-ci-summary` 追加・stale comment 更新
- `ci/impact/` — 削除（選択実行専用。`ci/metrics/` は残す）
- `.github/workflows/consumer-impact-check.yml` — 削除（T3）
- `docs/development-environment.md` — lane 構成・`ci-summary`・`expected_sha`・consumer-impact-check 廃止・task 表を同期

## Implementation Sequence

- [ ] T1: `ci/summary/summary.mjs` + contract tests + `test-ci-summary` mise task。verdict 行列（all-success→pass / failure/cancelled/skipped/missing/unknown→fail）、expected_sha（空=照合なし・一致・不一致）、NEEDS_JSON 不整合（parse 失敗→fail）、report JSON 形状を `node --test` で検証
- [ ] T2: `ci.yml` 編集 — `impact` job・全 `needs: [impact]`・全 lane `if:`・`CI_IMPACT_DISABLE` 撤去、`io-structured-verify` job 追加（compression-verify 直後）、`*-macos` 3 job を無条件化、`macos-report`/`ci-gate` を `ci-summary`（needs 11 件・`if: always()`）へ置換、`workflow_dispatch.inputs.expected_sha` 追加。`ci/impact/` 削除・`test-ci-impact` → `test-ci-summary` 置換・`docs/development-environment.md` 同期
- [ ] T3: `consumer-impact-check.yml` 削除 + docs 同期（分類要件は PR template + human gate 維持を明記）
- [ ] 検証: `mise exec -- actionlint .github/workflows/ci.yml`・`mise exec -- prek run --all-files`・`mise run test-ci-summary`・`mise run check/lint/test-io-structured`・残留参照 grep
- [ ] Conventional Commits で論理単位コミット（push なし）

## Dependencies and Risks

- T2 は T1 の `ci/summary/` 存在に依存（workflow が参照するため同 commit or 先行 commit）。
- `verify`/`l1-real-verify`/`ui-verify` の job id・matrix os 配列・`name:` を編集しない（required context 不変条件）。編集後に `git diff` で check 名の非変更を机上確認。
- `prek run --all-files` は vendored 除外ルール済み。Windows ローカルでの actionlint は mise pin 版を `mise exec` 経由で使用。

## Verification Matrix

| 検証 | 入口 | 期待 |
|---|---|---|
| workflow 構文 | `mise exec -- actionlint .github/workflows/ci.yml` | pass |
| 判定器契約 | `mise run test-ci-summary` | 全 contract test pass |
| yaml/toml 構文・hook | `mise exec -- prek run --all-files` | pass |
| io-structured lane 入口 | `mise run check-io-structured`（+lint/test） | exit 0 |
| 選択実行残留 | grep `ci/impact`・`CI_IMPACT_DISABLE`・`needs.impact`・`ci-gate`・`macos-report` | 履歴 docs 以外 0 件 |
| required context 不変 | `git diff` で `name:`/job id/matrix 確認 | 9 context 名不変 |

## Rollback

- 全変更は worktree 内の git commit。問題時は当該 commit の revert（`git revert`）で base `93801df` へ復帰可能。`workflow_dispatch`/`push`/`pull_request` trigger・required context は不変のため、revert で旧選択実行運用へ即時復帰する。

## Task Breakdown

- T1: `docs/tasks/issue-366-t1-ci-summary-evaluator.md`（TASK-034Smunkp1KgV6Pfg3HUvX）
- T2: `docs/tasks/issue-366-t2-ci-yml-full-ci.md`（TASK-034SmuoocFyRqTmasB818H）
- T3: `docs/tasks/issue-366-t3-consumer-impact-removal.md`（TASK-034SmupEbAlO5LQee4I04v）
