---
id: "TSPEC-720tupleV206"
type: "test-spec"
status: "approved"
title: "Issue #720 consumer-verify-tuple contract v1→v2 bump — test specification"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-720-consumer-tuple-v2.md"
    - "docs/designs/issue-720-consumer-tuple-v2.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-720-consumer-tuple-v2.md"
  implements:
    - "docs/specs/issue-720-consumer-tuple-v2.md"
    - "docs/designs/issue-720-consumer-tuple-v2.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-294-daily-tuple-trigger.md"
    - "docs/test-specs/issue-294-virtual-first-evidence-schema.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #720 consumer-verify-tuple contract v1→v2 bump — test specification

## Feature: consumer-verify-tuple contract v2

### Rule: contract 文字列は producer・consumer・workflow・test で単一値に揃う

証拠発行（`evidence.mjs` `CONTRACT` → `contract`/`tuple.contract`）、
daily 判定の既定 `tupleContract`、`daily-ci.yml` の
`DAILY_TUPLE_CONTRACT`、両 workflow の fallback `printf`・summary/
ヘッダ記述は全て `consumer-verify-tuple/v2` を指す。v1 文字列が
残るのは legacy 記述（hard cutover 方針の説明・旧 artifact fixture）
として明示分離された箇所のみ。

- Example: `buildEvidence` が出力する `ev.contract` と
  `ev.tuple.contract` は `CONTRACT` export（v2）と一致する
  （`evidence.test.mjs` が pin）。
- Example: `daily-ci.yml` の `DAILY_TUPLE_CONTRACT` env が v2 である
  ことを `daily-ci-yml-contract.test.mjs` が pin。
- Example: `consumer-verify.yml` の両 `evidence_finalize_error`
  fallback JSON と summary echo が v2 contract を出力することを
  `consumer-verify-yml-contract.test.mjs` が pin。

### Rule: contract bump は hard cutover — v1 証拠は non-target

`classifyRunEvidence` は contract 文字列の完全一致のみを covering
条件とし、v1 証拠 artifact は他の tuple 要素が全て一致しても
`non-target`（proven different tuple — verified にも retry 計上に
もならない）に分類される。両受理の移行窓は存在しない。

- Example: `decide.test.mjs` の legacy fixture（両 consumer の
  `contract: "consumer-verify-tuple/v1"`）が `kind: "non-target"`
  を返すことを pin。
- Example: `resolveConfig` の既定 `tupleContract` が v2 であり、
  `DAILY_TUPLE_CONTRACT` env が override として機能することを pin。

### Rule: consumer は証拠の `schemaVersion`/`lanes` を検査しない

`decide.mjs` の証拠検査は `kind`/`contract`/`run.runId`/
`consumer.{name,repo}`/`status`/`tuple.{ruviformRev,
consumerResolvedSha}` に限定される。`schemaVersion`・`lanes` の
語彙は producer 側契約（`assertLaneRecord` が finalize 時に
fail-closed 検査）であり、consumer が読む語彙ではない — 本判断は
`decide.mjs` ヘッダコメントと `docs/development-environment.md` に
記録される。

- Example: decide test の `evidenceFor` fixture は
  `schemaVersion: 1` のまま covering 判定を通る — consumer が
  schemaVersion を見ていないことの裏付け（fixture は consumer が
  読まないフィールドを敢えて旧値にしている）。

### Rule: downstream consumer（`ci/tracker` 等）に非破壊

`evidence.mjs` の export 名（`CONTRACT`/`SCHEMA_VERSION`/`LEVELS`/
`ENVIRONMENT_KINDS`/`REQUIREMENTS`/`classifyRef`/`makeLaneRecord`/
`deriveStatus`/`buildEvidence`/`markdownSummary`/`coverage`）と証拠
JSON のフィールド形状は不変 — 変更は `CONTRACT` の文字列値のみ。
`node --test ci/` 全体で `ci/tracker`・`ci/summary`・`ci/daily` の
既存 contract test が全て pass することで固定する。

## 検証コマンド（canonical = mise）

- `mise run test-ci-consumer-verify` — producer + workflow contract
- `mise run test-ci-daily` — consumer decide + daily-ci contract
- `mise run test-ci-tracker` — downstream 非破壊確認
- `mise exec -- actionlint .github/workflows/consumer-verify.yml
  .github/workflows/daily-ci.yml` — workflow 構造検査
- `mise exec -- node --test ci/` — 補助証拠（canonical=mise、ci/
  横断の非破壊確認）
