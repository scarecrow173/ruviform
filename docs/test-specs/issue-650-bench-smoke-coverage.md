---
id: "TSPEC-8ba0217gpp3jck5Mf0Pw1y"
type: "test-spec"
status: "approved"
title: "Issue #650 bench-smoke lane coverage テスト仕様"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/650"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-646-numeric-bench-profile.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-650-bench-smoke-coverage.md"
    - "docs/designs/issue-650-bench-smoke-coverage.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/650"
    - "https://github.com/scarecrow173/ruviform/issues/646"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  defers: []
  deferred-by: []
metadata:
  issue: 650
---

# Issue #650 bench-smoke lane coverage テスト仕様

## Purpose

`mise run bench-smoke`（CI `bench` job の入口）が workspace の
全 bench target を対象に bench profile のコンパイル + 1 回実行を
検証することを保証する。本 spec が守る意図は「bench profile
破損を CI lane で恒久的に検出する」coverage 契約であり、
Issue #646 で発覚した破損が numeric/io-e2e を含まない smoke 範囲
のために検出経路を持たなかった gap の解消を pin する。

## Feature

`bench-smoke` mise task の coverage 契約 — `[[bench]]` target を
持つ全 crate が `cargo bench -p ... -- --test` の対象に含まれ、
smoke 性質（各 bench 1 回・計測なし）が維持される。

## Rules

- **R1**: `bench-smoke` の `run` は bench target を持つ全 5 crate
  （`ruviform-math`/`ruviform-rt`/`ruviform-viz`/
  `ruviform-numeric`/`ruviform-io-e2e`）を `-p` で列挙する。
  bench target を持たない crate は対象外（workspace 全体の lib
  test target コンパイルは `cargo bench --workspace --no-run`
  の責務）。
- **R2**: smoke 性質を維持する — `-- --test`（criterion test
  mode）を保持し、計測系オプション・`--bench` 個別指定を足さない。
- **R3**: `mise.toml` の `description` と
  `docs/development-environment.md` の `bench-smoke` 行は同じ
  `-p` 列挙を反映する（正本 = `mise.toml`）。
- **R4**: consumer-verify 記述は `ci/consumer-verify/
  evidence.mjs` の landed schema（`SCHEMA_VERSION = 2`・Case
  schema v2 lane record field）と矛盾しない。監査は該当節の
  最小修正に留め、全面書き換えはしない。
- **R5**: bench target が存在しない crate を `-p` に追加しない —
  実在確認（`[[bench]]` + benches source）を根拠とする。

## Examples

- **E1（coverage 実測）**: Given `mise run bench-smoke`、When
  実行、Then exit 0 で `corr`（numeric）・`io_composition`
  （io-e2e）を含む 5 bench target の `Running benches\...`
  行が出力される。
- **E2（workspace 非回帰）**: Given `cargo bench --workspace
  --no-run`、When workspace 全 bench target をコンパイル、Then
  exit 0（#646 修正の維持）。
- **E3（doc 一致）**: Given `mise tasks` の `bench-smoke`
  description、When dev-env doc `bench-smoke` 行と照合、Then
  `-p` 列挙が一致する。
- **E4（audit）**: Given dev-env doc の consumer-verify 節、When
  `evidence.mjs` の schema 定数・lane record field・status 導出と
  照合、Then drift は L89 のみで最小修正済み、他節は no-drift
  根拠が記録される。

## Guarantees

- bench target を持つ全 crate の bench profile コンパイル +
  1 回実行が CI `bench` job で検証される。
- `docs/development-environment.md` が `mise.toml` 正本と
  bench-smoke 記述で一致する。
- consumer-verify 記述が schemaVersion 2 / Case schema v2 と
  整合する。

## Non-goals

- bench の実計測値・性能回帰判定（smoke は計測なし）。
- bench target を持たない crate の lib test target の smoke
  包含（`--no-run` workspace compile の責務）。
- `.github/workflows/`・`ci/` の変更。
- consumer-verify schema 自体の変更・doc chain 全面同期。

## Risk

- bench target 追加時に `bench-smoke` の `-p` 列挙が再び
  drift する可能性 — dev-env doc 監査と `mise tasks` 照合で
  検出可能。`--workspace` 化は smoke 責務を超えるため採用しない
  （design の Alternatives 参照）。
