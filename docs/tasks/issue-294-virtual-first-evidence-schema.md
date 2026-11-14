---
id: "TASK-034XC0Os34QkfxbVzBAbse"
type: "task"
status: "in-progress"
title: "Issue #294 T1 — consumer-verify Case schema v2 証拠契約拡張"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-294-virtual-first-evidence-schema.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-294-virtual-first-evidence-schema.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-virtual-first-evidence-schema.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #294 T1 — consumer-verify Case schema v2 証拠契約拡張

## Work

`ci/consumer-verify/evidence.mjs` の lane/case record を virtual-first Case schema へ additive 拡張する（`case_id`/`level`/`environment_kind`/`requirement` 必須 + optional Oracle/fixture/resolved SHA 項目、`SCHEMA_VERSION` 2、`contract` v1 不変、`readLanes` fail-closed）。`.github/workflows/consumer-verify.yml` の全 lane 記録行（`rec()` spec 化・extra task 直接 lane・fallback printf）へ case 宣言を付与し、`ci/consumer-verify/tests/` の fixture + yml contract test を拡張する。doc chain（本 slug）と `evidence/rf294/` 記録を仕上げる。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-294-virtual-first-evidence-schema.md`
- `docs/specs/issue-294-virtual-first-evidence-schema.md`
- `docs/designs/issue-294-virtual-first-evidence-schema.md`
- `docs/test-specs/issue-294-virtual-first-evidence-schema.md`

## Dependencies

- 着地済み virtual-first 検証契約（`scarecrow173/ruviform#627` Closed — POLICIES Effective 2026-11-15、Case schema 正本 `docs/designs/issue-627-virtual-first-verification-contract-design.md` §2）
- 先行 spec `docs/specs/issue-294-cross-consumer-candidate-gate.md`（tuple Gate・証拠契約 — 本 slice はその拡張）

## Affected Files

- `ci/consumer-verify/evidence.mjs`
- `ci/consumer-verify/tests/evidence.test.mjs`
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs`
- `.github/workflows/consumer-verify.yml`
- `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-294-virtual-first-evidence-schema.md` + `docs/*/README.md` index
- `evidence/rf294/`（検証記録・任意）

## Done When

- [ ] lane record が `case_id`/`level`/`environment_kind`/`requirement`/`outcome` + optional Case 項目を保持し、`requirement` と結果軸が直交
- [ ] `SCHEMA_VERSION === 2`、`CONTRACT === "consumer-verify-tuple/v1"`、既存 top-level 構造・status 導出・summary の意味不変
- [ ] `mise run test-ci-consumer-verify`・`mise exec -- actionlint .github/workflows/consumer-verify.yml`・`mise exec -- prek run --all-files` が pass
- [ ] workflow 静的 lane 行が level/environment_kind/requirement を正直に宣言（新規 Pass なし・outcome 語彙不変）
- [ ] doc chain + index 整合、Issue レベル AC の対象外項目を deferred 記録

## Verification

- [ ] `mise run test-ci-consumer-verify`
- [ ] `mise exec -- actionlint .github/workflows/consumer-verify.yml`
- [ ] `mise exec -- prek run --all-files`
- [ ] 補助（canonical=mise）: `node --test ci/` で tracker/daily consumer の非破壊確認
