---
id: "TASK-034XzFXWSyasCY5XlkPC8Q"
type: "task"
status: "done"
title: "Issue #281 OBS G0 feature matrix lane 実装"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/281"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-281-obs-feature-matrix.md"
      - type: "file"
        path: "docs/specs/issue-281-obs-feature-matrix.md"
      - type: "file"
        path: "docs/designs/issue-281-obs-feature-matrix.md"
      - type: "file"
        path: "docs/plans/issue-281-obs-feature-matrix.md"
      - type: "file"
        path: "docs/test-specs/issue-281-obs-feature-matrix.md"
      - type: "file"
        path: "docs/impl/ir/issue-281-obs-feature-matrix.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "ci/summary/summary.mjs"
      - type: "file"
        path: "ci/summary/tests/summary.test.mjs"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "evidence/rf275/obs-acceptance-ledger.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-281-obs-feature-matrix.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-281-obs-feature-matrix.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-281-obs-feature-matrix.md"
    - "docs/designs/issue-281-obs-feature-matrix.md"
    - "docs/test-specs/issue-281-obs-feature-matrix.md"
    - "evidence/rf275/obs-acceptance-ledger.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-281-obs-feature-matrix.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #281 OBS G0 feature matrix lane 実装

## Work

`ruviform-observability` の feature compile matrix lane を
新設する（Issue #281 G0 slice）:

1. `mise.toml` に `check-observability` / `lint-observability` /
   `test-observability` を追加 — io-compression matrix
   （`&&` 連結・6 組合せ）と同型、stable toolchain。
2. `.github/workflows/ci.yml` に `observability-verify`
   （ubuntu+windows matrix）と `observability-verify-macos` を
   追加し、`ci-summary` の `needs` に登録。
3. Windows ローカルで 3 task を実走（補助証拠）。
4. `evidence/rf275/obs-acceptance-ledger.md` の G0 行を更新
   （lane 定義済み・CI 未実走は `NotRun` 維持）。
5. impl/ir・impl/exp 記録と docs index 再生成。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-281-obs-feature-matrix.md`（approved）
- `docs/specs/issue-281-obs-feature-matrix.md`（approved）
- `docs/designs/issue-281-obs-feature-matrix.md`（approved）
- `docs/test-specs/issue-281-obs-feature-matrix.md`

## Dependencies

- なし（既存コードへの変更を伴わない lane 追加）。

## Affected Files

- `mise.toml` — 3 task 追加。
- `.github/workflows/ci.yml` — 2 job 追加 + needs 更新。
- `evidence/rf275/obs-acceptance-ledger.md` — G0 行更新。
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/
  issue-281-obs-feature-matrix.md` — doc chain。
- `ci/summary/summary.mjs`・`ci/summary/tests/summary.test.mjs` —
  REQUIRED_JOBS/REQUIRED_CHECKOUT_LEGS 宣言同期（coordinator 承認の
  範囲拡張）。
- `docs/development-environment.md` — lane inventory 更新
  （同上の範囲拡張）。
- 各 docs dir の `README.md` — index 再生成（生成物）。

## Done When

- [x] 3 mise task が定義され Windows ローカルで exit 0。
- [x] 2 CI job が ci.yml に定義され `ci-summary` needs に登録。
- [x] actionlint・workflow 契約 test・TOML 検査が通る。
- [x] 台帳 G0 行が lane 新設を反映し CI 未実走を `NotRun` と記録。
- [x] impl/ir・impl/exp が実績を記録し index が整合。

## Verification

- [x] `mise run check-observability` / `lint-observability` /
  `test-observability`（Windows、exit 0）。
- [x] `mise run check ruviform-observability` /
  `lint ruviform-observability` / `test ruviform-observability`。
- [x] `mise exec -- actionlint`、`node --test` の ci 契約 test 群。
- [x] `mise run fmt-check`。
