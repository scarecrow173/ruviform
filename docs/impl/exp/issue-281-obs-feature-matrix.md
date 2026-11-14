---
id: "EXP-issue-281-obs-feature-matrix"
title: "Issue #281 OBS G0 feature matrix lane — feature gating・lane 先例・summary 宣言の実測メモ"
created: "2026-11-14"
updated: "2026-11-14"
---

# Issue #281 OBS G0 feature matrix lane 実測メモ(2026-11-14)

- 計測契約: `docs/specs/issue-281-obs-feature-matrix.md` +
  `docs/designs/issue-281-obs-feature-matrix.md`
- 正本証拠: `evidence/rf275/obs-acceptance-ledger.md` §6

## 1. 問い

`ruviform-observability` の 4 optional feature
（`logging`/`tracing-export`/`allocation-probe`/`integration`）
について、既存 lane が到達しない feature 単独組合せを
どう検証 lane 化するか。特に (a) matrix task の形、
(b) CI job の OS 展開、(c) `ci-summary` 宣言同期の要否、
の 3 点を実測で固定する。

## 2. 調査と実測

### feature gating（`crates/ruviform-observability/Cargo.toml`）

- `default = []`。optional feature は 4 つ。
- `[[test]]`/`[[example]]` の `required-features`:
  `logging.rs`+`logging_host` ← `logging`、
  `integration.rs` ← `integration`、
  `rt_boundary.rs` ← `tracing-export`。
  `allocation-probe` は `src/profiling/*` の module gate
  （test target ではなく lib 内 unit test）。
- 帰結: default `cargo test` では feature-gated test target
  3 本が compile すらされない。`--all-features` 単独では
  組合せ分離の検証にならない（#275 exp の feature unify
  実測と整合）。

### lane 先例（`mise.toml`・`ci.yml`）

- `check/lint/test-io-compression`・`check/lint/test-io-structured`
  が `&&` 連結の feature matrix task の確立パターン。
  `compression-verify`/`io-structured-verify` は ubuntu-latest
  単独 job、`verify`/`ui-verify`/`ui-gpui-verify` は
  ubuntu+windows matrix + macOS 別 job（runner 逼迫回避の
  物理分離規約）。
- `test-observability-msrv` は `--all-features`・rust@1.89.0
  の MSRV lane であり、stable 系・組合せ分離の代替にならない
  （タスク契約のユーザー指定どおり MSRV 系は使わない）。

### ci-summary 宣言（`ci/summary/summary.mjs`）

- `REQUIRED_JOBS`（編集前 15）と `REQUIRED_CHECKOUT_LEGS`
  （編集前 19）が hardcode。新 lane を `needs` に足すだけでは
  undeclared 経路で集約され、checkout SHA 証跡の fail-closed
  検証対象からも外れる。
- coordinator が `ci/summary/`（summary.mjs + tests）の範囲
  拡張を承認 → `compression-verify`/`io-structured-verify` と
  同じ宣言パターンで同期（job 17・leg 22）。

## 3. 結論

- task 形: 6 組合せ（default・各 feature 単独・
  `--all-features`）を `&&` 連結。`--no-default-features` は
  `default = []` で default と等価のため省略。
- CI 展開: `observability-verify`（ubuntu+windows matrix・
  `fail-fast: false`）+ `observability-verify-macos`
  （macos-latest 別 job）で OS-L1 の 3 OS native を被覆。
  pairwise 組合せは `--all-features` が統合面を被覆するため
  Deferred。
- summary: 宣言同期を実施し undeclared 経路に頼らない。
  fail-closed 判定（success 必須・SHA 一致検証）は不変。

## 4. 実走結果（ローカル補助証拠）

- Windows 11 x64・dev toolchain: `mise run
  check-observability`/`lint-observability`/
  `test-observability` 全て exit 0。6 組合せ全て Pass。
- `mise run check|lint|test ruviform-observability`・
  `mise run fmt-check`・`actionlint`・ci 契約 node test 3 系
  も全て Pass。
- CI（workflow_dispatch）実走は `NotRun` — lane 定義と
  ローカル実走は統合受入の Pass を意味しない。
