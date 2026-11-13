---
id: "PLAN-034WJG3B2sBoAiz4O4YbAf"
type: "plan"
status: "approved"
title: "Issue #594 実施計画 — ci.yml MSRV lane 追加と workflow_dispatch 検証"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/594"
    - "docs/specs/ci-msrv-daily-lane.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/ci-msrv-daily-lane.md"
      - type: "file"
        path: "docs/tasks/ci-msrv-daily-lane.md"
      - type: "file"
        path: "docs/test-specs/ci-msrv-daily-lane.md"
      - type: "file"
        path: "docs/impl/ir/ci-msrv-daily-lane.md"
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
    - "docs/specs/ci-msrv-daily-lane.md"
  implemented-by:
    - "docs/tasks/ci-msrv-daily-lane.md"
  depends-on:
    - "docs/specs/ci-msrv-daily-lane.md"
    - "docs/designs/ci-msrv-daily-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/ci-msrv-daily-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ci-msrv-daily-lane.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ci-msrv-daily-lane.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #594 実施計画 — ci.yml MSRV lane 追加と workflow_dispatch 検証

## Goal

`ci.yml` に `msrv-verify` job を追加し、4 つの `*-msrv` タスクを `rust@1.89.0` で日次 CI（daily-ci → ci.yml dispatch）担保に載せ、ローカル必須検証からの除外を成立させる。

## Scope

単一実装ストリーム。`.github/workflows/ci.yml` の job 追加と `ci-summary` needs 更新、`ci/summary/summary.mjs` + test の宣言 lane 登録、doc-chain 6 件と管理 index 再生成。`mise.toml` の `*-msrv` タスクは既存のまま（調整不要と判断 — `mise exec rust@1.89.0` 内蔵で CI からそのまま呼べる）。`daily-ci.yml`・`consumer-verify.yml`・`ci-issue-tracker*.yml`・`crates/`・`mise.ci.toml` は変更しない。

## Upstream Documents

- spec: `docs/specs/ci-msrv-daily-lane.md`
- design: `docs/designs/ci-msrv-daily-lane.md`
- test-spec: `docs/test-specs/ci-msrv-daily-lane.md`

## Steps

1. doc-chain（spec/design/plan/task/test-spec + IR）を作成する。
2. `.github/workflows/ci.yml` に `msrv-verify` job を追加する（`ui-gpui-offscreen-verify` の後・`ci-summary` の前）: checkout pin → checkout-sha evidence → mise 復元 → cargo registry cache（`-cargo-msrv-` key）→ `mise install rust@1.89.0` → 4 タスク個別 step → `if: always()` lane evidence。
3. `ci-summary.needs` に `msrv-verify` を追加する。
4. `ci/summary/summary.mjs` の `REQUIRED_JOBS` / `REQUIRED_CHECKOUT_LEGS` に `"msrv-verify"` を追加し、`ci/summary/tests/summary.test.mjs` の 2 列挙を整合させる（coordinator 承認済みの範囲拡張）。
5. ローカル検証: `mise exec -- actionlint`、`mise run test-ci-summary`、および変更に関係する通常 lane（`fmt-check` / `check` / `lint` / `test` / `test-rt` / `build`）。MSRV 系タスク（`test-*-msrv`）は CI job が担うためローカル必須検証から除外。
6. `git fetch origin main && git merge origin/main` → 触った docs dir の管理 index を `doc-maintenance` の `index apply` で再生成。
7. Conventional Commits で commit → `git push -u origin feature/594-ci-msrv-lane` → `gh pr create`（`Closes #594`）。
8. `gh workflow run ci`（`expected_sha` = ブランチ先端）→ `gh run watch` で `msrv-verify` の success と他 lane の非退行を確認。結果 URL を PR 本文に追記。

## File map

- 変更: `.github/workflows/ci.yml`、`ci/summary/summary.mjs`、`ci/summary/tests/summary.test.mjs`
- 変更なし確認: `mise.toml`（4 タスクは変更不要）
- 新規: `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/ci-msrv-daily-lane*.md`
- 再生成: `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md`

## Verification matrix

| 検証 | コマンド | 合格条件 |
| --- | --- | --- |
| workflow 構文 | `mise exec -- actionlint` | exit 0（既存警告は事前記録して区別） |
| summary 評価器 | `mise run test-ci-summary` | exit 0（`node --test ci/summary/`） |
| fmt | `mise run fmt-check` | exit 0 |
| 型検査 | `mise run check` | exit 0 |
| lint | `mise run lint` | exit 0（`-D warnings`） |
| test | `mise run test` | exit 0 |
| RT | `mise run test-rt` | exit 0 |
| build | `mise run build` | exit 0 |
| index 整合 | `doc_maintenance.js index apply` | exit 0 |
| 実 CI | `gh workflow run ci` → `gh run watch` | `msrv-verify` = success、新規失敗なし |

## Risks

- **MSRV lane の初回実行時間**: `-cargo-msrv-` cache は新規 prefix のため初回は registry 全取得 + rust@1.89.0 toolchain install を伴う。日次 dispatch のみで実行され、手動 dispatch 検証（AC4）で実測する。
- **`mise install rust@1.89.0` と `MISE_ENV=ci` pin の相互作用**: 明示 version install は env pin と独立（synvelis で実績あり）。万が一失敗した場合は run ログで原因を切り分ける。
- **workflow_dispatch run の他 lane への影響**: 新 job 追加のみで既存 job 定義は不変のため、既存 lane の成否に影響しない。`ci-summary` は needs に 1 件増えるだけ。
- **並行 Worker との index 競合**: merge 後に index を再生成して同一コミットに含める。生成物のみの競合は merge → 再生成 → push で解消（フルレーン再検証不要）。
