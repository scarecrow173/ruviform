---
id: "IMPL-R0CaDUMXvtPvWlqdqmhwtT"
type: "impl"
status: "completed"
title: "issue-650-bench-smoke-coverage"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "devin-worker"
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "evidence/rf650/bench-smoke-excerpt.txt"
      - type: "file"
        path: "evidence/rf650/verification.md"
      - type: "file"
        path: "docs/specs/issue-650-bench-smoke-coverage.md"
      - type: "file"
        path: "docs/designs/issue-650-bench-smoke-coverage.md"
      - type: "file"
        path: "docs/plans/issue-650-bench-smoke-coverage.md"
      - type: "file"
        path: "docs/tasks/issue-650-bench-smoke-coverage.md"
      - type: "file"
        path: "docs/test-specs/issue-650-bench-smoke-coverage.md"
      - type: "file"
        path: "docs/impl/ir/issue-650-bench-smoke-coverage.md"
      - type: "file"
        path: "docs/impl/exp/issue-650-bench-smoke-coverage.jsonl"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-650-bench-smoke-coverage.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-650-bench-smoke-coverage.jsonl"
    rejected: []
---

# issue-650-bench-smoke-coverage

## Summary

`mise.toml` `[tasks.bench-smoke]` の対象を 3 crate から bench
target 全量の 5 crate（`+ ruviform-numeric`・`+ ruviform-io-e2e`）
へ拡張し、bench profile 破損（Issue #646 型）を CI `bench` job で
恒久的に検出できる状態にした。`docs/development-environment.md`
の `bench-smoke` 行を同期し、consumer-verify 記述の drift 監査
（Refs #294 residual）で L89 のみ drift を検出・最小修正した。

## Context

- spec: `docs/specs/issue-650-bench-smoke-coverage.md`（approved）
- design: `docs/designs/issue-650-bench-smoke-coverage.md`
  （approved）
- plan: `docs/plans/issue-650-bench-smoke-coverage.md`（approved）
- test-spec: `docs/test-specs/issue-650-bench-smoke-coverage.md`
  （approved）
- task: `docs/tasks/issue-650-bench-smoke-coverage.md`
- Issue: `scarecrow173/ruviform#650`（#646 で発覚した bench
  profile 破損の coverage gap 恒久対策）

## Implementation

### Changes

- `mise.toml` — `[tasks.bench-smoke]` の `run` を
  `cargo bench -p ruviform-math -p ruviform-rt -p ruviform-viz
  -p ruviform-numeric -p ruviform-io-e2e -- --test` へ更新し、
  `description` の package 列挙・コマンド例を同じ列挙へ合わせた。
  `-- --test`（各 bench 1 回・計測なし）の smoke 性質は不変。
- `docs/development-environment.md` — `bench-smoke` 行（L21）の
  コマンド列を新 `-p` 列挙へ更新し、説明を「`bench` と同じ対象」
  参照形へ変更（対象一覧の二重管理を回避）。
- `docs/development-environment.md` L89 —
  `test-ci-consumer-verify` 詳細行へ Case schema v2 の lane record
  field（必須 `case_id`/`level`/`environment_kind`/`requirement` +
  optional oracle・fixture・`resolved_ruviform_sha`、requirement
  は outcome と直交、v1 row は fail-closed 拒否）と
  `schemaVersion: 2` を最小追記。PR #639（`c6e4f334`）の landed
  schema との drift 修正。
- `evidence/rf650/` — bench-smoke 実行抜粋・検証サマリ。

### consumer-verify audit 結果

照合先: `ci/consumer-verify/evidence.mjs`
（`SCHEMA_VERSION = 2`・`LEVELS`・`ENVIRONMENT_KINDS` 5 値・
`REQUIREMENTS`・`makeLaneRecord`/`assertLaneRecord`・
`deriveStatus`）と `ci/consumer-verify/tests/
evidence.test.mjs` の fixture test。

- **drift（修正済み）**: L89 の contract test 内訳列挙が Case
  schema v2 field 群に未言及 → 最小追記。
- **no drift**: L28（task 表・汎用表現で矛盾なし）、L116/L123
  （tuple 証拠 JSON の key 列挙は schemaVersion 2 でも不変・実在）、
  L124（lane 結果語彙・Consumer status 語彙は `OUTCOMES`/
  `deriveStatus` と一致、contract は v1 のまま正確）、
  L126-127（tracker scope は schema v2 の影響を受けない）。

### Decisions During Implementation

- **`-p` 列挙追加のみ**: `--workspace` 化は smoke の責務を超え、
  bench target を持たない crate の lib test target まで暗黙に
  広げるため不採用（design Alternatives 参照）。
- **説明セルは `bench` 行参照形**: 対象一覧の二重管理を避け、
  `bench` 行更新時に smoke 行が自動的に正しいままになる。
- **audit は L89 のみ修正**: 他節は粒度上の言及省略であり
  false claim ではないため、全面書き換えせず根拠を証拠に記録。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-650-bench-smoke-coverage.jsonl` —
  bench target 実在確認・audit 照合・実測検証の観察記録。

### Rejected

- `cargo bench --workspace -- --test` 化 — smoke 責務を超える
  coverage 拡大であり、Issue の見込み（`-p` 追加）に反する。
- consumer-verify 節の全面同期 — scope 外の再構成は行わない。

## Validation

canonical 検証（`mise run` / `mise exec`、Windows 11 x64・
rustc/cargo 1.99.1・mise 2026.11.2・node v26.12.1）:

- `mise run bench-smoke` — pass（exit 0。`Running benches\
  io_composition.rs`・`simd_poc.rs`・`corr.rs`・
  `rt_observability_poc.rs`・`viz.rs` の 5 target 実行を確認。
  抜粋: `evidence/rf650/bench-smoke-excerpt.txt`）
- `mise exec -- cargo bench --workspace --no-run` — pass
  （Issue AC 由来の canonical 指定・補助証拠）
- `mise run fmt-check` — pass
- `mise exec -- prek run --all-files` — pass
- `mise tasks` — pass（`bench-smoke` description の 5 package
  列挙を出力で確認）

証拠: `evidence/rf650/verification.md`

## Risks

- bench target 追加時の `-p` 列挙 drift — dev-env doc 監査と
  `mise tasks` 照合で検出可能。恒久的な自動包含は本 slice の
  範囲外。
- CI `bench` job の実行時間増 — smoke（`-- --test`）のみで
  計測なし。#646 修正済みのため破損リスクは低い。

## Follow-ups

- bench target を持たない crate の lib test target は引き続き
  smoke 対象外 — 必要になれば `cargo bench --workspace --no-run`
  を定期/CI lane 化する別 Issue の検討事項。
- consumer-verify doc chain（`docs/*/issue-294-*`）側の schema
  v2 記述との整合は本 slice の範囲外（dev-env doc のみ監査）。
