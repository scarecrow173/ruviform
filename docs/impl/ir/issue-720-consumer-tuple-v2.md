---
id: "IMPL-720tupleV207"
type: "impl"
status: "completed"
title: "Issue #720 consumer-verify-tuple contract v1→v2 bump — 実装記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-720-consumer-tuple-v2.md"
    - "docs/plans/issue-720-consumer-tuple-v2.md"
    - "docs/specs/issue-720-consumer-tuple-v2.md"
    - "docs/designs/issue-720-consumer-tuple-v2.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-720-consumer-tuple-v2.md"
      - type: "file"
        path: "evidence/rf720/verification.md"
    modified:
      - type: "file"
        path: "ci/consumer-verify/evidence.mjs"
      - type: "file"
        path: "ci/daily/decide.mjs"
      - type: "file"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: ".github/workflows/daily-ci.yml"
      - type: "file"
        path: "ci/consumer-verify/tests/evidence.test.mjs"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
      - type: "file"
        path: "ci/daily/tests/decide.test.mjs"
      - type: "file"
        path: "ci/daily/tests/daily-ci-yml-contract.test.mjs"
      - type: "file"
        path: "docs/development-environment.md"
  implements:
    - "docs/tasks/issue-720-consumer-tuple-v2.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-720-consumer-tuple-v2.md"
  blocks: []
  supersedes: []
  superseded-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-720-consumer-tuple-v2-exp.jsonl"
    rejected: []
---

# Issue #720 consumer-verify-tuple contract v1→v2 bump — 実装記録

## 実装したもの

### producer: `ci/consumer-verify/evidence.mjs`

- `CONTRACT` export を `consumer-verify-tuple/v1` → `/v2`。
  bump 経緯（Case schema v2 語彙の tuple 識別への反映・hard
  cutover・非対象化の注記）をコメントで記録。`SCHEMA_VERSION = 2`・
  Case schema v2 語彙・export 名・証拠 JSON フィールド形状は不変。

### consumer: `ci/daily/decide.mjs`

- 既定 `tupleContract`（`DAILY_TUPLE_CONTRACT` 未設定時）を v2 へ。
  `--tuple-contract` flag・env override 経路は不変。
- tuple mode ヘッダコメントを v2 へ更新し、hard cutover（v1 証拠は
  non-target・両受理窓なし）と `schemaVersion`/`lanes` 未検査維持の
  判断をコメントとして記録。usage コメントも v2 へ。
- 判定ロジック（`classifyRunEvidence`）は無変更 — contract 完全一致
  の既存規則が hard cutover をそのまま実装する。

### workflows

- `.github/workflows/daily-ci.yml`: `DAILY_TUPLE_CONTRACT` env → v2、
  ヘッダ・job コメント → v2。
- `.github/workflows/consumer-verify.yml`: ヘッダコメント・AL/SV
  `evidence_finalize_error` fallback `printf`（両方）・summary echo
  → v2。lane 行・step 構成・owner 注記は不変。

### contract tests

- `evidence.test.mjs`: `CONTRACT`/証拠 JSON の v2 pin。
- `consumer-verify-yml-contract.test.mjs`: summary 文面・fallback
  JSON の v2 pin。加えて consumer-side GUI contract 行の owner regex
  を `#1(66|75)` → `#[0-9]+` へ修復（main `7124f0b2` で再現確認した
  既存破損 — `70b48304` の owner 注記更新に test が未追従。
  「owner 帰属を持つ not_run 行」という契約は維持）。
- `decide.test.mjs`: `CONTRACT` 定数・tuple コメント・report regex
  → v2。contract 不一致 fixture を v2→v1（legacy）へ変更し
  「legacy v1 証拠 → non-target」の hard cutover pin として明記。
  `resolveConfig` 既定 v2 + env override の pin を新規追加。
- `daily-ci-yml-contract.test.mjs`: `DAILY_TUPLE_CONTRACT` env pin
  → v2。

### docs

- `docs/development-environment.md`: 証拠契約記述 3 箇所を v2 へ
  同期し、hard cutover（v1 artifact は non-target・両受理窓なし）と
  consumer の `schemaVersion`/`lanes` 未検査維持を `decide-consumer-
  verify` job の記述へ明記。

## 実行結果（canonical）

- `mise run test-ci-consumer-verify` → 84 pass / 0 fail, exit 0
- `mise run test-ci-daily` → 113 pass / 0 fail, exit 0
- `mise run test-ci-tracker` → 108 pass / 0 fail, exit 0
- `mise exec -- actionlint .github/workflows/consumer-verify.yml
  .github/workflows/daily-ci.yml` → exit 0
- `mise exec -- node --test ci/`（補助・canonical=mise）→
  354 pass / 0 fail, exit 0
- `evidence.mjs finalize` smoke: 発行証拠 JSON の `contract`/
  `tuple.contract` が `consumer-verify-tuple/v2` であることを実測。

詳細は `evidence/rf720/verification.md`。Issue AC5（post-merge 実
run gate）は NotRun — Coordinator 所有の post-merge dispatch で
追跡する。
