---
id: "TASK-034Sa6J9mP4sV7xB1fK5nQ"
type: "task"
status: "done"
title: "T2 consumer-verify.yml tuple証拠・SHA ref・UI headless lane・extra_tasks 接続点 + yml contract test（Issue #294）"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added:
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
    modified:
      - type: "file"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-294-cross-consumer-candidate-gate.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-294-cross-consumer-candidate-gate.md"
    - "docs/tasks/issue-294-t1-consumer-verify-evidence-script.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-294-t1-consumer-verify-evidence-script.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-294-cross-consumer-candidate-gate.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T2 consumer-verify.yml tuple証拠・SHA ref・UI headless lane・extra_tasks 接続点 + yml contract test（Issue #294）

## Work

`.github/workflows/consumer-verify.yml` を design §2 どおり拡張する:

1. 入力追加（`_expected_sha`・`_extra_tasks`・`_ref` の SHA 受理）と `concurrency`
2. preflight: 入力検証（classify-ref 連携）+ checkout@REV + `test-ci-consumer-verify` 自己検証
3. verify 両 job: checkout@REV → fetch+`FETCH_HEAD` checkout → `resolve-sha` + `tuple_match` → 既存 lane → 追加 lane（AL: check-ruviform-lock/sync-ui-deps/check-ruviform-rev --require-vendor/setup-ui/check-ui/test-ui。SV: check-ruviform-rev/test-rt/check-headless-deps/sync-ui-deps/setup-ui/check-ui/test-ui/build-ui/check-ruviform-rev --require-vendor）→ extra_tasks → evidence finalize → upload-artifact
4. summary job に tuple 表を追加し、status 語彙に `unsupported` を含める
5. `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs`（textual 構造契約）と `docs/development-environment.md` 対応表

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-294-cross-consumer-candidate-gate.md`
- `docs/designs/issue-294-cross-consumer-candidate-gate.md`
- `docs/test-specs/issue-294-consumer-verify-tuple-evidence.md`

## Dependencies

- T1（evidence.mjs が workflow から呼ばれる）

## Affected Files

- `.github/workflows/consumer-verify.yml`
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs`
- `docs/development-environment.md`

## Done When

- [ ] `mise run test-ci-consumer-verify` pass（yml contract test 含む）
- [ ] `mise exec -- actionlint .github/workflows/consumer-verify.yml` pass
- [ ] `mise exec -- prek run --all-files` pass（check-yaml 等）
- [ ] 実窓/macOS lane が `unsupported` 理由行で記録され、skip=pass にしない
- [ ] 実 runner dispatch 検証は merge 後として PR 証拠表に NotRun 記録
