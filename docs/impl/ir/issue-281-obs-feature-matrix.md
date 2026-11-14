---
id: "IMPL-034XzMSBKZjfzGvbvOElaM"
type: "impl"
status: "completed"
title: "OBS-06 G0 observability feature matrix lane (Issue #281)"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "ci/summary/summary.mjs"
      - type: "file"
        path: "ci/summary/tests/summary.test.mjs"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "evidence/rf275/obs-acceptance-ledger.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/issue-281-obs-feature-matrix.md"
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
  implements:
    - "docs/tasks/issue-281-obs-feature-matrix.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-281-obs-feature-matrix.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-281-obs-feature-matrix.md"
    - "docs/designs/issue-281-obs-feature-matrix.md"
    - "docs/plans/issue-281-obs-feature-matrix.md"
    - "docs/test-specs/issue-281-obs-feature-matrix.md"
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
      - "docs/impl/exp/issue-281-obs-feature-matrix.md"
    rejected: []
---

# OBS-06 G0 observability feature matrix lane (Issue #281)

## Summary

`ruviform-observability` の feature matrix を検証する G0 lane を新設した。
`mise.toml` に `check-observability`/`lint-observability`/`test-observability`
の 3 タスクを追加し、default・`logging`・`tracing-export`・
`allocation-probe`・`integration`・`--all-features` の 6 組合せを
`&&` 連結で網羅する（io-compression/io-structured lane の既存パターン踏襲）。
`.github/workflows/ci.yml` に `observability-verify`
（ubuntu-latest/windows-latest matrix・`fail-fast: false`）と
`observability-verify-macos`（macos-latest 別 job）の 2 job を追加し、
checkout SHA 証跡 artifact・mise/cache 手順・`ci-summary` の `needs` 登録を
既存 lane と同一規約で実装した。`ci/summary/summary.mjs` の
`REQUIRED_JOBS`/`REQUIRED_CHECKOUT_LEGS` とそのテストを同期更新し、
fail-closed 判定を維持した。`docs/development-environment.md` の lane
inventory と `evidence/rf275/obs-acceptance-ledger.md` の G0 記述を更新した。
Rust 実装コードの変更はゼロ — 既存の feature-gated target を lane が
compile/lint/test するだけである。

## Context

Issue #281（OBS-06 Cross-Consumer 互換/RT/精度検証 Gate）の G0 slice。
`ruviform-observability` は `default = []` + 4 optional feature
（`logging`/`tracing-export`/`allocation-probe`/`integration`）を持つが、
既存 lane は default feature の `test-full` と MSRV `--all-features` のみで、
feature 単独組合せの分離検証と Win/macOS native の feature 付き CI が
存在しなかった（受入台帳 `evidence/rf275/obs-acceptance-ledger.md` の
G0 行が `NotRun`（lane なし）と記録済み）。本 slice は lane の存在を
作るものであり、対象 SHA での CI 実走・consumer tuple 証拠・実機 L2/L3 は
後続 Gate に残る。

## Implementation

### Changes

- `mise.toml`: `check-observability`/`lint-observability`/`test-observability`
  を io-e2e ブロック直後に追加。各タスクは 6 組合せを `&&` 連結。
  check は `cargo check -p ruviform-observability --all-targets`、
  lint は `cargo clippy ... --all-targets -- -D warnings`、
  test は `cargo test -p ruviform-observability`。`--no-default-features`
  は `default = []` のため default 実行と等価として省略。
- `.github/workflows/ci.yml`: `observability-verify`
  （`runs-on` matrix `[ubuntu-latest, windows-latest]`、`fail-fast: false`、
  `CHECKOUT_LEG: observability-verify-${{ matrix.os }}`）と
  `observability-verify-macos`（`runs-on: macos-latest`、
  `CHECKOUT_LEG: observability-verify-macos`）を `io-structured-verify`
  の後に追加。checkout pin・`git rev-parse HEAD` 証跡・artifact upload・
  mise install・`MISE_ENV=ci`・Cargo cache・`test-ci-summary` 先行等の
  step 構成は既存 lane と同一。`ci-summary.needs` に両 job を追加。
- `ci/summary/summary.mjs`: `REQUIRED_JOBS` に `observability-verify`・
  `observability-verify-macos`、`REQUIRED_CHECKOUT_LEGS` に
  `observability-verify-ubuntu-latest`・`observability-verify-windows-latest`・
  `observability-verify-macos` を追加（宣言 leg 数 19→22、job 数 15→17）。
- `ci/summary/tests/summary.test.mjs`: 期待リストを同期更新。
- `docs/development-environment.md`: lane inventory に
  `observability-verify`/`observability-verify-macos` を追加し、
  2 OS matrix・macOS 別 job の記述と宣言 leg 数（17→22）を更新。
- `evidence/rf275/obs-acceptance-ledger.md`: G0 lane 追加を追記。
  lane 定義の存在と target-SHA CI `Pass` を区別し、CI 実走前は
  `NotRun` のままとする。
- 文書チェーン: `docs/specs|designs|plans|tasks|test-specs/
  issue-281-obs-feature-matrix.md` を作成（spec/design/plan/test-spec は
  approved、task は in-progress→完了時に更新）。

### Decisions During Implementation

- 3 OS 展開を選択: coordinator 指針の「ubuntu lane のみか 3 OS か」の
  判断として、既存の macOS 物理分離規約（runner 逼迫回避・per-OS needs
  集約）に従い `observability-verify`（ubuntu+windows matrix）+
  `observability-verify-macos`（別 job）とした。OS-L1 の Win/macOS
  native feature build/test を lane 定義で被覆する。
- pairwise 2 feature 組合せは Deferred: `--all-features` が統合面を
  被覆し、6 組合せで compile/lint/test の分離検証要件を満たす。
- `ci/summary/summary.mjs` の更新は当初スコープ外と判断したが、
  coordinator が `compression-verify`/`io-structured-verify` と同じ
  宣言パターンでの同期を明示承認したため実施。undeclared lane のまま
  残すと恒常的な「undeclared」注記と checkout 証跡の fail-closed
  網羅性を損なうため。
- `docs/impl/exp/issue-281-obs-feature-matrix.md` に実装時の
  実測メモ（feature gating・lane 先例・summary 宣言・実走結果）
  を記録した。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-281-obs-feature-matrix.md` — feature
  gating・lane 先例・`ci-summary` 宣言の実測に基づく設計判断
  （6 組合せ `&&` 連結・3 OS 展開・宣言同期）を採用。

### Rejected

<!-- なし -->

## Validation

- `mise run check-observability`: Windows 11 x64・dev toolchain・exit 0
  （6 組合せ全て `cargo check --all-targets` Pass）
- `mise run lint-observability`: 同上・exit 0
  （6 組合せ全て `cargo clippy --all-targets -- -D warnings` Pass）
- `mise run test-observability`: 同上・exit 0
  （6 組合せ全て `cargo test` Pass。`--all-features` では
  lib 298・tests/integration.rs 15・tests/logging.rs 28・
  tests/rt_boundary.rs 3・doctest 15+1 ignored を確認）
- `mise run check|lint|test ruviform-observability`: パッケージ
  レベル canonical タスク（結果は完了時に記録）
- `mise run fmt-check`: （結果は完了時に記録）
- `mise exec -- actionlint .github/workflows/ci.yml`: exit 0
- `node --test ci/daily/tests/ci-yml-contract.test.mjs`: 10/10 Pass
- `node --test ci/summary/tests/summary.test.mjs`: 31/31 Pass
- `node --test ci/consumer-verify/tests/`: 64/64 Pass
- `mise tasks`: 3 新規タスク認識を確認
- CI（workflow_dispatch）実走: `NotRun` — dispatch-only のため
  push/PR では起動しない。対象 SHA の CI 証拠は merge 後の
  Daily/手動 dispatch が正本

## Risks

- CI lane 追加による Full CI 実行時間増（6 組合せ × 3 task × 3 OS）。
  cache hit 前提でも初回 run は compile コストが乗る。
- feature 組合せの compile/lint/test が対象 SHA の CI で初めて
  実行されるため、Windows/macOS 固有の失敗が merge 後に顕在化する
  可能性がある（lane の目的どおりの検出であり、弱体化はしない）。
- G0 lane の存在は個別 Case の `Pass` を意味しない。対象 SHA の
  CI run・consumer tuple・実機 L2/L3 は引き続き `NotRun`。

## Follow-ups

- merge 後 main SHA に対する Daily/手動 Full CI で
  `observability-verify`/`observability-verify-macos` の実走と
  `ci-summary` verdict を確認し、台帳の G0 行を証拠付きで更新する。
- AL #127/SV #28 の独立 Oracle・#294 tuple への結果投入・
  Windows 利用機能 L2/L3 は本 slice 外（外部 Gate）。
- pairwise feature 組合せ・`--no-default-features` 明示指定が
  必要になった場合は別 Issue で拡張を検討する。
