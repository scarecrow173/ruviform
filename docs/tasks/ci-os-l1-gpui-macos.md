---
id: "TASK-gRRWzc0F8au2cehYadUxhR"
type: "task"
status: "in-progress"
title: "Issue #598 T1 — ui-gpui-verify-macos lane 追加・ci-summary 宣言登録・test-ui-gpui feature 拡張・workflow_dispatch 検証"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/598"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/ci-os-l1-gpui-macos.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/ci-os-l1-gpui-macos.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ci-os-l1-gpui-macos.md"
    - "docs/specs/ci-os-l1-gpui-macos.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/ci-os-l1-gpui-macos.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/ci-os-l1-gpui-macos.md"
  derived-by: []
  verifies:
    - "docs/test-specs/ci-os-l1-gpui-macos.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #598 T1 — ui-gpui-verify-macos lane 追加・ci-summary 宣言登録・test-ui-gpui feature 拡張・workflow_dispatch 検証

## Objective

`ci.yml` に `ui-gpui-verify-macos` job を追加し、GPUI feature 検証と driver-neutral 契約 fixture を macOS の OS-L1 として Full CI に載せ、`windowed_test_app` bin の Build/Link 空白を `test-ui-gpui` の feature 拡張で塞ぎ、ブランチの `ci` workflow run で `ui-gpui-verify-macos` の success と 3 OS lane の非退行を確認する。

## Checklist

- [ ] doc-chain（spec/design/plan/task/test-spec + IR）作成
- [ ] `ci.yml` に `ui-gpui-verify-macos` job 追加（macos-latest・`ui-gpui-verify` 同一 step から Linux 依存導入を除去・`CHECKOUT_LEG=ui-gpui-verify-macos`・checkout-sha evidence）
- [ ] `ci-summary.needs` に `ui-gpui-verify-macos` を追加
- [ ] `ci/summary/summary.mjs` の `REQUIRED_JOBS` / `REQUIRED_CHECKOUT_LEGS` と `summary.test.mjs` の列挙に `ui-gpui-verify-macos` を追加
- [ ] `mise.toml` の `test-ui-gpui` を `--features "gpui-test uitest"` へ拡張し description 更新
- [ ] `mise exec -- actionlint` exit 0
- [ ] `mise run test-ci-summary` exit 0
- [ ] 触った docs dir の管理 index を doc-maintenance `index apply` で再生成
- [ ] `git merge origin/main` → commit → push（`chore/598-ci-os-l1-gpui-macos`）→ PR 作成
- [ ] `gh workflow run ci.yml --ref <branch> -f expected_sha=<sha>` で `ui-gpui-verify-macos` success・3 OS lane 非退行・`ci-summary` pass を確認し、walltime 比較と共に URL を PR/IR に記載
