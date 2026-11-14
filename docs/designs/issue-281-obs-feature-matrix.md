---
id: "DESIGN-034XzBioQrNr2epQZ9ETsq"
type: "design"
status: "approved"
title: "Issue #281 OBS-06 G0 — observability feature matrix lane 設計"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/281"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-281-obs-feature-matrix.md"
      - type: "file"
        path: "docs/plans/issue-281-obs-feature-matrix.md"
      - type: "file"
        path: "docs/tasks/issue-281-obs-feature-matrix.md"
      - type: "file"
        path: "docs/test-specs/issue-281-obs-feature-matrix.md"
      - type: "file"
        path: "docs/impl/ir/issue-281-obs-feature-matrix.md"
      - type: "file"
        path: "docs/impl/exp/issue-281-obs-feature-matrix.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "evidence/rf275/obs-acceptance-ledger.md"
      - type: "file"
        path: "docs/specs/issue-281-obs-feature-matrix.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-281-obs-feature-matrix.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-281-obs-feature-matrix.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-275-observability-integration-design.md"
    - "evidence/rf275/obs-acceptance-ledger.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-281-obs-feature-matrix.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #281 OBS-06 G0 — observability feature matrix lane 設計

## Context

`ruviform-observability` は `default = []` で 4 つの optional
feature（`logging` / `tracing-export` / `allocation-probe` /
`integration`）を持つ。feature-gated test target は
`tests/logging.rs`（`logging` 必須）・`tests/integration.rs`
（`integration` 必須）・`tests/rt_boundary.rs`
（`tracing-export` 必須）・`examples/logging_host.rs`
（`logging` 必須）・`src/profiling/*`（`allocation-probe` 必須）
であり、`cargo test --workspace`（default feature）や
`verify`/`verify-macos` の `test-full`（default+`integration`
unify）では大半が compile すらされない。

既存の唯一の feature 付き CI 実行経路は `msrv-verify` の
`test-observability-msrv`（`mise exec rust@1.89.0 -- cargo test
-p ruviform-observability --all-features`）だが、(a) MSRV
1.89 固定で通常開発 toolchain の検証ではない、(b)
`--all-features` 同時有効であり feature 単独組合せの分離
compile を検証しない、(c) Linux (ubuntu-latest) 単独で
Windows/macOS native の feature 付き build/link/test を
カバーしない。受入台帳はこの 2 穴を G0 `NotRun` として記録済み。

本設計は io-compression lane（Issue #300/#301、`mise.toml`
L223-238）と compression-verify job（`ci.yml` L473-546）の
確立済みパターンを observability に適用する。

## Scope

### In Scope

- `mise.toml` への 3 task 追加（feature matrix の `&&` 連結）。
- `ci.yml` への `observability-verify` +
  `observability-verify-macos` job 追加と `ci-summary` needs 登録。
- `evidence/rf275/obs-acceptance-ledger.md` の G0 行更新。
- doc chain と Windows ローカル実走。

### Out of Scope

- `crates/` のコード変更（matrix が既存コードで成立するかの
  確認のみ）。
- `ci/summary/summary.mjs`・`docs/development-environment.md`
  等の編集範囲外ファイル（判断は spec の Out of Scope と
  Deferred Design Concerns を参照）。
- consumer 側 Oracle・実機 L2/L3・MSRV lane。

## Components and Boundaries

- **mise task 層**: `check-observability` / `lint-observability` /
  `test-observability`。io-compression task と同じく単一文字列の
  `&&` 連結で 6 組合せを直列実行する。`--locked`・`--profile ci`
  等の wrapper は付けない（io-compression/io-structured の
  matrix task 先例に一致。CI 側は job env の
  `CARGO_INCREMENTAL=0` と `MISE_ENV=ci` の toolchain pin で
  実行環境を規定する）。
- **CI job 層**: `observability-verify`（`matrix.os:
  [ubuntu-latest, windows-latest]`、`fail-fast: false`、
  `CHECKOUT_LEG: observability-verify-${{ matrix.os }}`）と
  `observability-verify-macos`（`macos-latest`、
  `CHECKOUT_LEG: observability-verify-macos`）。各 job は
  checkout → `git rev-parse HEAD` 記録 → `checkout-sha-*`
  artifact upload → mise setup → cargo cache → 3 mise task
  実行の最小 step 構成（compression-verify と同型。
  sccache は compression-verify 同様に適用しない — 本 lane は
  1 crate + その依存のみで cache 投資の効果が薄い）。
- **ci-summary 層**: `needs` に 2 job を追加。`summary.mjs` の
  `REQUIRED_JOBS`/`REQUIRED_CHECKOUT_LEGS` は編集範囲外のため
  更新しない — evaluator は needs/legs の未宣言 entry を
  `undeclared-needs-entry`/`undeclared-checkout-leg` note 付きで
  同一の success 規則（fail-closed）で評価する supported 経路
  であり、verdict の正しさは損なわれない。
- **証拠層**: `evidence/rf275/obs-acceptance-ledger.md` の G0
  行を「lane 定義済み・対象 Revision の CI 実走は NotRun」
  へ更新し、ローカル Windows 実走は補助証拠（E-LOCAL 系）と
  して区別して記録する。

## Data and Control Flow

- 開発者/CI は `mise run check-observability`（等）を呼び、
  task body が `cargo check/clippy/test -p
  ruviform-observability` を 6 feature 組合せで `&&` 連結実行
  する。いずれかの組合せが失敗すれば task が非 0 で終了し、
  CI job step が fail → `ci-summary` が run 全体を Fail にする
  （fail-closed）。
- CI では各 job が `git rev-parse HEAD` を
  `checkout-sha-<leg>-attempt-<n>.txt` として artifact 化し、
  `ci-summary` の `checkout-sha-*` download → evaluator の
  laneCommon 検証に載る。全 leg が同一 `github.sha` を
  checkout するため common sha は一致する。

## Non-Functional Constraints

- **OS-L1**: POLICIES §2 により native build/link・test・
  利用 feature の検証は Windows/macOS/Linux 全てで Required。
  本 lane が 3 OS をカバーするのはこの要求への対応であり、
  ubuntu 単独では ledger §4.2 の
  「Win/macOS native の feature 付き build/link/test
  （Required）」の穴が残る。
- **MSRV 分離**: 新 lane は stable toolchain（CI では
  `mise.ci.toml` pin）。`*-msrv` task は使わない
  （タスク指定）。
- **コスト**: 6 組合せ × 3 task を 3 OS で実行する。
  observability crate は中規模（`--all-features` で lib 298
  test）で依存に tracing-subscriber 系を含むため 1 job あたり
  数十分級を見込むが、daily-ci/手動 dispatch の Full CI レーン
  としては既存 lane（verify の workspace 全体 build/test）に
  比較して小さい。
- **feature 組合せの選定**: `default`・各 feature 単独・
  `--all-features` の 6 組合せ。`default = []` のため
  `--no-default-features` は default と等価（ledger の Case
  記述にある `--no-default-features` を包含）。単独組合せは
  feature 分離時の dead code / `-D warnings` 違反を、その
  組合せでのみ検出できる。2 feature 同時指定の pairwise は
  `--all-features` の unify 経路で被覆されるため Deferred
  （spec Deferred Design Concerns 参照）。

## Risks and Trade-offs

- **Risk**: macOS/Windows で feature 単独組合せの test が
  失敗する可能性（例: `tracing-appender` の file rotation が
  Windows で file lock に遭遇、`allocation-probe` の TLS scope
  が platform 差で挙動差）。**Mitigation**: それがまさに本 lane
  の検出対象であり、失敗は lane の正しい Fail として扱う。
  ローカル Windows 実走で早期に検出する。
- **Risk**: CI 時間増加。**Mitigation**: daily-ci は未検証
  SHA のみ dispatch するため常時負荷ではない。job は
  `fail-fast: false` で sibling OS の結果を全て報告する。
- **Trade-off**: `ci-summary` で `undeclared-*` note が出る。
  verdict は正しいが summary 表の見た目上「未宣言」に見える。
  `summary.mjs` 更新は編集範囲外のため本 slice では行わず、
  PR 本文に明記する。
- **Trade-off**: 台帳の G0 行は「lane あり・CI 未実走」で
  `NotRun` のまま残る。Pass への更新は merge 後対象 SHA の
  CI 証拠を待つ（POLICIES §3/§5 の証拠規範どおり）。

## Deferred Design Concerns

- 解消済み: `ci/summary/summary.mjs` の `REQUIRED_JOBS` /
  `REQUIRED_CHECKOUT_LEGS` 宣言追加と `docs/development-environment.md`
  の lane 一覧追記は、当初「編集範囲外」として deferred としたが、
  coordinator が両範囲の拡張を承認したため本 slice で実施済み
  （compression-verify/io-structured-verify と同じ宣言パターンで
  `summary.test.mjs` の期待リストも同期。fail-closed 判定は不変）。
- pairwise feature 組合せの matrix 拡張 — spec 参照。

## Verification Notes

- [x] `mise run check-observability` / `lint-observability` /
  `test-observability` が Windows ローカルで exit 0（6 組合せ）。
- [x] `actionlint` が `ci.yml` を通過（exit 0）。
- [x] `node --test ci/daily/tests/ci-yml-contract.test.mjs`・
  `ci/summary/tests/summary.test.mjs`・
  `ci/consumer-verify/tests/` が通る（10/10・31/31・64/64 Pass）。
- [x] `mise.toml` の TOML 妥当性（`mise tasks` 認識）。
- [x] 台帳 G0 行の更新が NotRun を偽装しないことを review で確認
  （§6 追記形式で lane 定義と CI 未実走を分離記録）。
