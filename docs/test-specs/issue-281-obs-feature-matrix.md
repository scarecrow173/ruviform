---
id: "TSPEC-034XzBqngp0Gx0FdDaHhyc"
type: "test-spec"
status: "approved"
title: "Issue #281 OBS G0 feature matrix lane テスト仕様"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/281"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-281-obs-feature-matrix.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-281-obs-feature-matrix.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-281-obs-feature-matrix.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-281-obs-feature-matrix.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-281-obs-feature-matrix.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-281-obs-feature-matrix.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #281 OBS G0 feature matrix lane テスト仕様

## Purpose

本 lane が「feature 組合せごとの compile/lint/test を実際に
実行し、失敗を fail-closed で CI verdict に反映する」ことを
保証する。lane が定義だけ存在して実行されない、あるいは
fail-open になる状態を防ぐのが目的であり、個々の feature の
機能正しさ（Logging parity・RT no-alloc 等）は各 OBS Issue の
既存 test-spec が所有する。本 spec が退役できるのは G0 Case が
対象 SHA の CI 証拠で Pass 判定され、かつ feature 構成が
変更されて matrix が不要になった場合のみ。

## Feature

ruviform-observability の feature compile matrix lane
（mise task 3 本 + CI job 2 本）。

## Rules

- **R1 組合せ網羅**: `check-observability` /
  `lint-observability` / `test-observability` の各 task は
  `default`（`default = []` のため `--no-default-features` と
  等価）・`logging`・`tracing-export`・`allocation-probe`・
  `integration`・`--all-features` の 6 組合せを全て実行する。
  いずれか 1 組合せでも脱落・typo があれば lane は不完全であり
  Fail とみなす。
- **R2 feature-gated target の到達**: `tests/logging.rs` と
  `examples/logging_host.rs` は `logging` 組合せで、
  `tests/integration.rs` は `integration` 組合せで、
  `tests/rt_boundary.rs` は `tracing-export` 組合せで
  compile・実行される（`required-features` の契約どおり）。
  `--all-features` では全 target が同時に有効になる。
- **R3 lint 強度**: `lint-observability` は全組合せで
  `cargo clippy --all-targets -- -D warnings` を実行し、
  feature 分離時にのみ出る dead code / warning を検出する。
- **R4 CI fail-closed**: `observability-verify` /
  `observability-verify-macos` は `ci-summary` の `needs` に
  含まれ、いずれかの OS leg が failure/cancelled/skipped/
  not-run なら run 全体が Fail になる。checkout SHA 証拠
  （`checkout-sha-<leg>-attempt-<n>.txt`）を ship し、
  `ci-summary` の laneCommon 検証に参加する。
- **R5 OS カバレッジ**: lane は ubuntu-latest・windows-latest・
  macos-latest の 3 OS で実行される（POLICIES §2 OS-L1）。
  macOS は物理的に別 job（`*-macos` 慣行 — runner 逼迫の
  巻き込み防止と per-OS `needs` 集約のため）。
- **R6 toolchain**: lane は stable toolchain（CI では
  `mise.ci.toml` pin、`MISE_ENV=ci`）で実行し、`*-msrv` task
  を経由しない。
- **R7 証拠の正直さ**: 受入台帳は lane 定義・ローカル実走・
  対象 SHA の CI Run 証拠を区別する。CI 未実走の Case を
  `Pass` と記録しない。

## Examples

- **E1**: `mise run test-observability` を Windows で実行 →
  6 組合せ全てで `cargo test -p ruviform-observability` が
  exit 0。`--features logging` 実行時に `tests/logging.rs`
  28 test が実行されることを出力で確認できる。
- **E2**: `ci.yml` の `observability-verify` job が
  `strategy.matrix.os` に ubuntu/windows を持ち、
  `observability-verify-macos` が `runs-on: macos-latest` を
  持つ。両 job が `ci-summary` の `needs` 配列に列挙される。
- **E3**: `ci-summary` の evaluator に新 lane の needs entry
  （result=success）と checkout-sha leg を与えると verdict は
  pass、result=failure なら fail — undeclared entry でも
  fail-closed が維持される（`summary.mjs` の supported 経路）。

## Guarantees

- feature 組合せのいずれかで compile error・clippy warning・
  test failure があれば、対応する mise task が非 0 で終了し
  CI job が Fail する。
- `required-features` で gate された test/example target が
  対応 feature 組合せで確実に build・実行される。
- 新 lane の失敗・未実行・skipped が `ci-summary` 経由で
  run 全体の Fail として観測される。
- 台帳の G0 記述が lane の存在と CI 証拠の有無を混同しない。

## Non-goals

- 各 feature の機能的正しさ（Logging parity・RT no-alloc・
  privacy redaction 等）— per-issue test-spec と consumer
  Oracle が所有。
- pairwise feature 組合せ（2 feature 同時指定）の網羅 —
  `--all-features` unify で被覆、Deferred。
- `ci/summary/summary.mjs` の `REQUIRED_JOBS` 宣言同期 —
  undeclared 経路で fail-closed は成立、編集範囲外。
- consumer-verify / daily-ci workflow 自体の変更。

## Risk

- lane が「定義されているが実行されない」状態（needs 未登録・
  job 名 typo・matrix 記述ミス）を R4/R5 の契約 test と
  actionlint で検出できなければ、G0 の証拠経路が空洞化する。
- 台帳が lane 定義をもって CI Pass と誤記すると、POLICIES §3
  の「job 定義・green から個別 Case の Pass を推論しない」
  に違反する — R7 で防ぐ。
