---
id: "PLAN-BPJj0VR61YLMy1KGoMC8jJ"
type: "plan"
status: "approved"
title: "Issue #598 実施計画 — ui-gpui-verify-macos lane 追加と workflow_dispatch 検証"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/598"
    - "docs/specs/ci-os-l1-gpui-macos.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/ci-os-l1-gpui-macos.md"
      - type: "file"
        path: "docs/tasks/ci-os-l1-gpui-macos.md"
      - type: "file"
        path: "docs/test-specs/ci-os-l1-gpui-macos.md"
      - type: "file"
        path: "docs/impl/ir/ci-os-l1-gpui-macos.md"
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
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/specs/ci-os-l1-gpui-macos.md"
  implemented-by:
    - "docs/tasks/ci-os-l1-gpui-macos.md"
  depends-on:
    - "docs/specs/ci-os-l1-gpui-macos.md"
    - "docs/designs/ci-os-l1-gpui-macos.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/ci-os-l1-gpui-macos.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ci-os-l1-gpui-macos.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ci-os-l1-gpui-macos.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #598 実施計画 — ui-gpui-verify-macos lane 追加と workflow_dispatch 検証

## Goal

`ci.yml` に `ui-gpui-verify-macos` job を追加し、GPUI adapter の `gpui`/`gpui-test`/`uitest` feature 検証と driver-neutral 契約 fixture を macOS の OS-L1 として日次・手動 Full CI に載せ、`windowed_test_app` bin の Build/Link 空白を `test-ui-gpui` の feature 拡張で塞ぐ。

## Scope

単一実装ストリーム。`.github/workflows/ci.yml` の job 追加と `ci-summary` needs 更新、`ci/summary/summary.mjs` + test の宣言 lane 登録、`mise.toml` の `test-ui-gpui` feature 拡張、doc-chain 6 件と管理 index 再生成、ブランチ SHA での手動 `ci.yml` dispatch 検証。`daily-ci.yml`・`consumer-verify.yml`・`ci-issue-tracker*.yml`・`mise.ci.toml`・`crates/`・`POLICIES.md` は変更しない。

## Upstream Documents

- spec: `docs/specs/ci-os-l1-gpui-macos.md`
- design: `docs/designs/ci-os-l1-gpui-macos.md`
- test-spec: `docs/test-specs/ci-os-l1-gpui-macos.md`

## Steps

1. doc-chain（spec/design/plan/task/test-spec + IR）を作成する。
2. `.github/workflows/ci.yml` に `ui-gpui-verify-macos` job を追加する（`ui-gpui-verify` の直後・`ui-gpui-offscreen-verify` の前）: `ui-gpui-verify` と同一 step 構成から Linux 依存導入 step を除去。`CHECKOUT_LEG=ui-gpui-verify-macos`。
3. `ci-summary.needs` に `ui-gpui-verify-macos` を追加する（`ui-gpui-verify` の直後）。
4. `ci/summary/summary.mjs` の `REQUIRED_JOBS` / `REQUIRED_CHECKOUT_LEGS` に `"ui-gpui-verify-macos"` を追加し、`ci/summary/tests/summary.test.mjs` の 2 列挙を整合させる。
5. `mise.toml` の `test-ui-gpui` を `--features "gpui-test uitest"` へ拡張し description を更新する。
6. ローカル検証: `mise exec -- actionlint`、`mise run test-ci-summary`。scoped cargo 検証は CI/文書変更のため対象 crate なし（`test-ui-gpui` の feature 組合せ実走は macOS/Windows/Linux の CI lane が担う — ローカル Windows での `cargo test -p ruviform-ui-gpui --features "gpui-test uitest"` は変更範囲比例の補助証拠として実施可能なら行う）。
7. `git fetch origin main && git merge origin/main` → 触った docs dir の管理 index を `doc-maintenance` の `index apply` で再生成。
8. Conventional Commits で commit → `git push -u origin chore/598-ci-os-l1-gpui-macos` → `gh pr create`。
9. `gh workflow run ci.yml --ref chore/598-ci-os-l1-gpui-macos -f expected_sha=<ブランチ先端 SHA>` → `gh run watch` で `ui-gpui-verify-macos` の success・3 OS lane の非退行・`ci-summary` verdict を確認。変更前 run（直近成功 run）との walltime/runner 時間を比較し、結果 URL と共に PR 本文・IR に記録する。

## File map

- 変更: `.github/workflows/ci.yml`、`ci/summary/summary.mjs`、`ci/summary/tests/summary.test.mjs`、`mise.toml`
- 新規: `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/ci-os-l1-gpui-macos*.md`
- 再生成: `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md`

## Verification matrix

| 検証 | コマンド | 合格条件 |
| --- | --- | --- |
| workflow 構文 | `mise exec -- actionlint` | exit 0 |
| summary 評価器 | `mise run test-ci-summary` | exit 0（`node --test ci/summary/`） |
| GPUI feature 実走（補助・任意） | `cargo test -p ruviform-ui-gpui --features "gpui-test uitest"` | exit 0（Windows ローカル・補助証拠） |
| index 整合 | `doc_maintenance.js index apply` | exit 0 |
| 実 CI | `gh workflow run ci.yml --ref <branch> -f expected_sha=<sha>` → `gh run watch` | `ui-gpui-verify-macos` = success、全 lane 非退行、`ci-summary` = pass |

## Risks

- **macOS での gpui compile 失敗**: CI 初走行でのみ確定。失敗時は step 単位で切り分け、crate 側原因なら別 Issue へ。
- **`uitest` 併用の相互作用**: cfg 衝突なし確認済みだが compile は CI で確定。失敗時は feature を `gpui-test` に戻し bin build を専用 step へ退避。
- **macOS runner starvation（`abandoned`）**: 分離 job のため他 lane 証拠は保全され、`ci-summary` が fail-closed で検出。再実行で回復可能。
- **並行 Worker との index 競合**: merge → index 再生成 → push で解消（生成物のみの競合はフルレーン再検証不要）。
