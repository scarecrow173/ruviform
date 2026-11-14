---
id: "DESIGN-EEySPRA3APW4bbQzAFGv0Y"
type: "design"
status: "approved"
title: "Issue #650 bench-smoke lane coverage 拡張 — 設計"
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
  implements:
    - "docs/specs/issue-650-bench-smoke-coverage.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-650-bench-smoke-coverage.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-646-numeric-bench-profile.md"
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-650-bench-smoke-coverage.md"
  derived-by: []
  verifies: []
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

# Issue #650 bench-smoke lane coverage 拡張 — 設計

## Context

`mise.toml` `[tasks.bench-smoke]` は
`cargo bench -p ruviform-math -p ruviform-rt -p ruviform-viz -- --test`
で、workspace に存在する 5 つの bench target のうち 3 つしか
カバーしていなかった。Issue #646（PR #649）で `ruviform-numeric`
の lib test target が bench profile でコンパイル不能だったことが
発覚したが、`bench-smoke` が numeric/io-e2e を含まず
`--bench corr` フィルタでも lib test target を通らないため、
`cargo bench --workspace` の破損は検出経路を持たなかった。

bench target の実在は両 crate で確認済み:

- `crates/ruviform-numeric/Cargo.toml` — `[[bench]] name = "corr"
  harness = false` + `benches/corr.rs` + criterion dev-dep
  （Issue #265）。
- `crates/ruviform-io-e2e/Cargo.toml` — `[[bench]] name =
  "io_composition" harness = false` + `benches/io_composition.rs`
  + criterion dev-dep（Issue #306 G05）。

## Scope

### In Scope

- `mise.toml` `[tasks.bench-smoke]`: `run` へ
  `-p ruviform-numeric -p ruviform-io-e2e` を追加、
  `description` の列挙を同じ 5 package へ更新。
- `docs/development-environment.md`: `bench-smoke` 行のコマンド列・
  説明を追随 + consumer-verify 記述の drift 最小修正。
- `evidence/rf650/`・内部文書チェーン。

### Out of Scope

- `crates/`・`.github/workflows/`・`ci/`・`Cargo.toml` の変更。
- bench 計測・性能 baseline・msrv/full lane。

## Design

### 決定 1: `-p` 追加のみで smoke 性質を維持

`run` を `cargo bench -p ruviform-math -p ruviform-rt
-p ruviform-viz -p ruviform-numeric -p ruviform-io-e2e -- --test`
へ拡張する。package 列挙は dev-env doc `bench` 行（L20）の既存
列挙順（math → rt → viz → numeric → io-e2e）に揃える。
`-- --test`（criterion の test mode = 各 bench 1 回・計測なし）と
「計測系オプションを足さない」性質は不変。CI `bench` job は同じ
`mise run bench-smoke` を呼ぶため workflow 変更は不要。

### 決定 2: dev-env doc `bench-smoke` 行の追随

コマンド列を新しい `-p` 列挙と一致させ、説明は「`bench` と同じ
対象の各ベンチを計測なしで 1 回実行」として `bench` 行の対象列挙
を参照する形にする — 対象一覧の二重管理を避け、`bench` 行更新時に
smoke 行が自動的に正しいままになる。

### 決定 3: consumer-verify audit — L89 のみ drift、最小修正

照合基準は PR #639 landed の `ci/consumer-verify/evidence.mjs`
（`SCHEMA_VERSION = 2`・`LEVELS = [OS-L1, GUI-L2, EXT-L3]`・
`ENVIRONMENT_KINDS` 5 値・`REQUIREMENTS = [Required, Deferred,
NotApplicable]`・lane record の `case_id`/`level`/
`environment_kind`/`requirement` + optional field・
`deriveStatus` 行列）と `evidence.test.mjs` の fixture test。

- **L89（`test-ci-consumer-verify` 詳細行）: drift あり** —
  contract test の内訳を詳細列挙しているが、Case schema v2 の
  lane record field 群（`case_id`/`level`/`environment_kind`/
  `requirement` + optional oracle/fixture/`resolved_ruviform_sha`、
  requirement/outcome 直交、v1 row の fail-closed 拒否）への言及が
  無い。`schemaVersion`・`contract: consumer-verify-tuple/v1` の
  区別には触れるが version 値と Case field を欠く。同じ粒度で
  最小追記する。
- **L28（task 表行）: drift なし** — 「lane 記録」「tuple 証拠
  JSON/summary 形状」は汎用表現で、Case schema v2 を含む lane
  record を否定も不正確化もしない。表の粒度では追記不要。
- **L116/L123（`decide-consumer-verify`）: drift なし** — 証拠
  JSON から復元する tuple key（`tuple.ruviformRev`/
  `tuple.consumerResolvedSha`/`tuple.contract`/`consumer.{name,repo}`/
  `run.runId`/`status`）は schemaVersion 2 でも不変で実在する。
  daily scheduler が読む面の記述として正確。
- **L124（`consumer-verify.yml`）: drift なし** — lane 結果語彙
  （pass/fail/not_run/unsupported/skipped）と Consumer status
  語彙は `OUTCOMES`/`deriveStatus` と一致。「証拠 JSON
  （`contract: consumer-verify-tuple/v1`）」も正確（contract は
  v1 のまま、schemaVersion とは別軸）。同節は lane record field
  を列挙する粒度ではない。
- **L126-127（`ci-issue-tracker` scope）: drift なし** — Issue
  集約の scope 記述であり schema v2 の影響を受けない。

## Alternatives Considered

- **`--workspace` 化**: `bench-smoke` を `cargo bench --workspace
  -- --test` にすると将来の bench 追加を自動包含するが、
  `-p` 列挙は現行 task の確立された形であり、workspace 全体の
  bench target 増加（lib test target を含む `--no-run` 相当の
  コンパイル範囲）を smoke の責務へ暗黙に広げる。Issue の見込み
  （`-p` 追加）どおり最小変更を採用。bench target が増えた際の
  再 drift は dev-env doc 監査の対象として残る。
- **consumer-verify 節の全面同期**: 範囲外。L89 の 1 句追記に留め、
  残りは「no drift」根拠を証拠へ記録する。

## Risks

- 追加 2 crate の bench compile 時間が CI `bench` job を伸ばすが、
  `-- --test` の 1 回実行のみで計測は行わず、#646 修正済みのため
  破損リスクは低い（実測で exit 0 を確認）。
- L89 追記は既存の密な列挙 style に合わせた最小修正であり、
  contract test 実装の変更を伴わない。

## Verification

- `mise run bench-smoke` → exit 0・5 bench target の実行行を証拠化
- `mise exec -- cargo bench --workspace --no-run` → exit 0
- `mise run fmt-check` / `mise exec -- prek run --all-files` → exit 0
- `mise tasks` → `bench-smoke` description の 5 package 列挙を確認
- 証跡: `evidence/rf650/`
