---
id: "SPEC-oY0DT4RkgQbMmPjZfQUKr0"
type: "spec"
status: "approved"
title: "Issue #650 bench-smoke lane coverage 拡張（numeric / io-e2e）+ dev-env residual 同期"
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
    - "docs/specs/issue-646-numeric-bench-profile.md"
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
    - "docs/specs/issue-306-io-lane-completion.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-646-numeric-bench-profile.md"
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/650"
    - "https://github.com/scarecrow173/ruviform/issues/646"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/265"
    - "https://github.com/scarecrow173/ruviform/issues/306"
  defers: []
  deferred-by: []
metadata:
  issue: 650
---

# Issue #650 bench-smoke lane coverage 拡張（numeric / io-e2e）+ dev-env residual 同期

## What Should Be Built

CI `bench` job が呼ぶ `mise run bench-smoke` の対象を、現行の
`ruviform-math` / `ruviform-rt` / `ruviform-viz` の 3 crate から、
bench target を持つ全 5 crate（`+ ruviform-numeric` の `corr`、
`+ ruviform-io-e2e` の `io_composition`）へ拡張する。
`mise.toml` の `run`/`description` と
`docs/development-environment.md` の `bench-smoke` 行を同期させ、
bench profile 破損（Issue #646 で発覚した `AllocDisabler` cfg 欠落 +
型推論エラー型）を CI lane で恒久的に検出できる状態にする。

あわせて同ファイルの consumer-verify 記述について、PR #639
（schemaVersion 2 / Case schema v2）landed 後の drift を小監査し、
drift があれば最小修正する（Refs #294 residual）。

## Why Now

Issue #646（PR #649、`d5aa1b4b`）で `ruviform-numeric` の lib test
target が bench profile でコンパイル不能だったことが発覚したが、
`bench-smoke` が numeric/io-e2e を対象に含まず、`cargo bench
--workspace` が壊れたまま検出経路を持たなかった。本 slice はその
coverage gap を恒久的に閉じる。numeric の `corr` bench（Issue #265）と
io-e2e の `io_composition` bench（Issue #306 G05）はいずれも実在を
確認済みであり、`-p` 追加のみで coverage が workspace の bench
target 全量へ揃う。

## Users and Value

- Ruviform の開発者・CI: `mise run bench-smoke`（= CI `bench` job）
  が全 bench target の bench profile コンパイル + 1 回実行を検証し、
  #646 型の潜在破損を merge 前に検出できる。
- 文書保守: `docs/development-environment.md` が `mise.toml` 正本と
  一致し、consumer-verify 記述が landed schema（Case schema v2）と
  drift しない。

## Scope

### In Scope

- `mise.toml` `[tasks.bench-smoke]` の `run` へ
  `-p ruviform-numeric -p ruviform-io-e2e` を追加し、
  `description` の package 列挙・コマンド例を同じ列挙へ更新。
- `docs/development-environment.md` の `bench-smoke` 行（L21）の
  コマンド列・説明を新列挙へ追随。
- 同ファイルの consumer-verify 系記述（L28・L89・L116・L123-124・
  L126-127）を `ci/consumer-verify/evidence.mjs` の landed schema
  （`SCHEMA_VERSION = 2`・`LEVELS`・`ENVIRONMENT_KINDS`・
  `REQUIREMENTS`・lane record field・status 導出）と照合する小監査。
  drift は最小修正、なければ audit 結果を証拠に記録。
- `evidence/rf650/` の実測証跡と内部文書チェーン。

### Out of Scope

- `crates/` 全体の変更（config/doc-only slice）。
- `.github/workflows/`・`ci/` の変更（CI `bench` job は既存
  `bench-smoke` task を呼ぶため workflow 変更不要）。
- `Cargo.toml` への `[[bench]]`・dev-dep 追加。
- bench 計測値の記録・性能 baseline 更新（smoke は計測なし）。
- `*-msrv` 系・`*-full` 系検証（Daily Full CI 委譲）。
- `AGENTS.md`・`POLICIES.md`・`docs/discovery/`・他 slug の docs。

## Requirements

- `mise run bench-smoke` が exit 0 で、`corr`（numeric）・
  `io_composition`（io-e2e）を含む全 bench target を実行すること
  （実測証拠必須）。
- `cargo bench --workspace --no-run` が引き続き exit 0。
- `mise run fmt-check` / `mise exec -- prek run --all-files` が
  exit 0。
- smoke の性質（各 bench 1 回・計測なし・`-- --test`）を維持し、
  `-p` 指定のみを追加する。
- dev-env doc の consumer-verify 記述が schema 正本と矛盾しない
  こと（drift 時は最小修正、監査範囲の全面書き換えはしない）。

## Acceptance Criteria

- [x] `mise run bench-smoke` が exit 0 で numeric/io-e2e の bench
      target を含めて実行される（実測・`evidence/rf650/` に実行行
      抜粋）。
- [x] `mise exec -- cargo bench --workspace --no-run` が exit 0。
- [x] `mise run fmt-check` / `mise exec -- prek run --all-files` が
      exit 0。
- [x] `mise tasks` の `bench-smoke` description が 5 package 列挙を
      含む（出力 snippet を証拠に採る）。
- [x] consumer-verify audit の結果（修正 or no-drift 根拠）を
      `evidence/rf650/verification.md` に記録。

## Deferred Design Concerns

- なし（#646 spec の Deferred「bench-smoke 対象拡張」は本 Issue
  で回収済み）。

## Sources and Relations

- Issue: <https://github.com/scarecrow173/ruviform/issues/650>
- 前提・動機: `docs/specs/issue-646-numeric-bench-profile.md`
  （bench profile 破損の発見と coverage gap の指摘）。
- bench 由来: Issue #265（`corr`）、Issue #306 G05
  （`io_composition`）。
- audit 照合先: Issue #294 / PR #639（Case schema v2）、
  `docs/specs/issue-294-virtual-first-evidence-schema.md`、
  `ci/consumer-verify/evidence.mjs`。
