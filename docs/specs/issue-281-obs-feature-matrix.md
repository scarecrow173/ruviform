---
id: "SPEC-034XzzKfK6lkw9tuLNCGSv"
type: "spec"
status: "approved"
title: "Issue #281 OBS G0 — ruviform-observability feature compile matrix lane"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added:
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
      - type: "file"
        path: "docs/impl/exp/issue-281-obs-feature-matrix.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "evidence/rf275/obs-acceptance-ledger.md"
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
    - "docs/designs/issue-275-observability-integration-design.md"
    - "evidence/rf275/obs-acceptance-ledger.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-281-obs-feature-matrix.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #281 OBS G0 — ruviform-observability feature compile matrix lane

## What Should Be Built

`ruviform-observability` の 4 つの optional feature
（`logging` / `tracing-export` / `allocation-probe` / `integration`、
いずれも既定 off・`default = []`）について、feature 組合せごとの
compile / lint / test を実行する検証 lane を新設する。

1. `mise.toml` に io-compression lane（Issue #300/#301）と同型の
   `check-observability` / `lint-observability` / `test-observability`
   task を追加する。各 task は default・各 feature 単独・
   `--all-features` の 6 組合せを `&&` 連結で実行する。
2. `.github/workflows/ci.yml` に `observability-verify` job を追加する。
   OS-L1（POLICIES §2）が Windows/macOS/Linux 全てで Required である
   ため、既存 lane の慣行どおり `observability-verify`
   （`ubuntu-latest` + `windows-latest` matrix）と
   `observability-verify-macos`（`macos-latest` 別 job）の 2 job 構成
   とし、両方を `ci-summary` の `needs` に加えて fail-closed 集約に
   載せる。各 job は checkout SHA 証拠（`checkout-sha-*` artifact）を
   既存 lane と同じ手順で ship する。
3. `evidence/rf275/obs-acceptance-ledger.md` の G0 行
   （Feature compile matrix・Win/macOS native の feature 付き
   build/link/test）を、lane 新設の事実と残る NotRun 穴
   （対象 Revision の CI 実走証拠・consumer 側 Oracle）を区別して
   更新する。

## Why Now

- 受入台帳（`evidence/rf275/obs-acceptance-ledger.md` §3/§4.2）が
  G0 を `NotRun`（Required のまま）と記録している。既存 lane では
  `msrv-verify`/`test-observability-msrv` が `--all-features` 同時
  有効のみで feature 分離組合せを検証せず、`verify`/`verify-macos`
  の `test-full` は default+`integration` unify のみで
  `logging`/`tracing-export`/`allocation-probe` の feature-gated
  test target（`tests/logging.rs`・`tests/rt_boundary.rs`・
  `src/profiling/*` 等）を実行しない。
- `cargo test --workspace` は default feature のみをカバーするため、
  POLICIES §4「非デフォルトの Cargo Feature について通常の
  `cargo test --workspace` だけで全組合せが検証されたとみなさない」
  に照らし、明示的な feature matrix lane が必要。
- MSRV レーン（`*-msrv`）は MSRV 1.89 固定の別目的 lane であり、
  本 lane は stable 系 toolchain の通常検証として新設する
  （タスク指定: MSRV レーンは通常開発では行わない）。

## Users and Value

- Issue #281（OBS-06 Cross-Consumer 互換/RT/精度検証 Gate）の G0
  Case 組に対し、feature compile matrix と Win/macOS native の
  feature 付き build/link/test の CI 証拠経路を提供する。
- AudioLens / Synvelis 両 Consumer は #294 の candidate tuple 経由で
  Ruviform SHA の検証証拠を消費するため、lane の存在自体が
  tuple 判定の入力になる。
- 開発者は `mise run check-observability` 等でローカルから同一の
  matrix を再現できる（local 検証と CI lane の入口を共通化）。

## Scope

### In Scope

- `mise.toml`: `check-observability`・`lint-observability`・
  `test-observability` の 3 task 追加（io-compression の
  `&&` 連結 matrix パターン踏襲、stable toolchain）。
- `.github/workflows/ci.yml`: `observability-verify`（OS matrix）+
  `observability-verify-macos` job 追加、`ci-summary` の `needs`
  への登録。新規 Workflow ファイルは増設しない。
- `evidence/rf275/obs-acceptance-ledger.md`: G0 関連行の更新
  （lane 定義の事実と CI 実走待ちの NotRun を区別して記録）。
- doc chain（本 spec/design/plan/task/test-spec/impl ir/exp）。
- Windows ローカルでの新 task 実走（補助証拠。統合受入は
  merge 後 main SHA の CI 証拠が正本）。

### Out of Scope

- `crates/` 配下のコード変更（feature matrix が既存コードで
  成立することを確認するのみ。compile/test 失敗が見つかった
  場合は別途報告し、本 slice では修正しない）。
- AL #127 / SV #28 の製品 Oracle 実施、#294 tuple への結果投入、
  Windows L2/L3 実機受入、実機負荷 RT — 全て外部 gate。
- `ci/summary/summary.mjs` の `REQUIRED_JOBS` /
  `REQUIRED_CHECKOUT_LEGS` への宣言追加（undeclared needs 経路は
  evaluator が fail-closed で処理する supported 経路。必要なら
  別 slice で宣言同期する）。
- `docs/development-environment.md` の lane 一覧更新（編集範囲外。
  PR 本文の未対応作業に記録）。
- MSRV lane の変更・新設。

## Requirements

- matrix は `default`（=`--no-default-features` と等価、
  `default = []`）・`logging`・`tracing-export`・
  `allocation-probe`・`integration`・`--all-features` の 6 組合せを
  各 task で網羅する。
- `check` は `--all-targets`（example `logging_host` を含む
  feature-gated target の compile を検証）、`lint` は
  `clippy --all-targets -- -D warnings`、`test` は `cargo test -p`
  （unit/integration/doc test を各組合せで実行）とする。
- CI job は既存 lane と同じ checkout SHA 証拠 step
  （`CHECKOUT_LEG` env・`checkout-sha-<leg>-attempt-<n>.txt`
  artifact）を持ち、`ci-summary` の fail-closed verdict に載る。
- lane は stable toolchain（`mise.ci.toml` の CI pin）で実行し、
  `*-msrv` 系 task は使わない。
- 台帳の更新は正直であること: lane 定義・ローカル実走を
  `Pass` と書かず、対象 Revision の CI Run URL/SHA 証拠が
  得られるまで当該 Case は `NotRun` のままとする。

## Acceptance Criteria

- [x] `mise run check-observability` / `lint-observability` /
  `test-observability` が Windows ローカルで exit 0（6 組合せ全て）。
- [x] `actionlint` が `ci.yml` 変更を通過し、workflow 契約検査
  （`ci/daily`・`ci/consumer-verify`・`ci/summary` の contract
  test）が通る。
- [x] `mise.toml` が TOML として valid で、`mise tasks` が新 task を
  認識する。
- [x] `obs-acceptance-ledger.md` の G0 行が lane 新設を反映し、
  CI 未実走分を `NotRun` として残している。
- [ ] merge 後の対象 SHA の Daily/手動 Full CI で
  `observability-verify` / `observability-verify-macos` が実行され、
  `ci-summary` が両 job を集約する（統合検証。本 PR では
  NotRun/Pending として記録）。

## Deferred Design Concerns

- feature 組合せの更なる網羅（2 feature 同時指定の pairwise
  matrix 等）: `--all-features` が全 feature の unify 経路を被覆し、
  単独組合せが分離 compile を被覆するため現行 6 組合せで
  G0 の要求を満たす。pairwise が必要になった場合は Consumer 側で
  特定組合せの不具合が観測された時点で拡張する
  （再 engage: consumer verify で feature 組合せ起因の Fail が出た場合）。
- `ci/summary/summary.mjs` への lane 宣言同期: 当初は編集範囲外
  として deferred としたが、coordinator が `ci/summary/` の範囲
  拡張を承認したため本 slice で実施済み
  （`REQUIRED_JOBS`/`REQUIRED_CHECKOUT_LEGS` + contract test を
  compression-verify/io-structured-verify と同じ宣言パターンで
  同期。undeclared 経路には頼らない）。

## Sources and Relations

- Issue #281（OBS-06 gate Owner）・#275（OBS 受入照合の前提）。
- `evidence/rf275/obs-acceptance-ledger.md` §3「Feature compile
  matrix」行・§4.2 未対応 Case 棚卸し — 本 slice が埋める穴の正本。
- `evidence/rf275/obs-acceptance-audit-{spec,design,plan,task}.md`
  — 台帳を生んだ前 slice の内部 doc chain（形式の先例）。
- `mise.toml` L223-256（io-compression / io-structured matrix
  task の踏襲先例）・L781-788（`test-observability-msrv`）。
- `.github/workflows/ci.yml` `compression-verify` /
  `io-structured-verify` / `verify`+`verify-macos` job 構成の
  踏襲先例。
- `POLICIES.md` §2（OS-L1 全 OS Required）・§4（非 default
  feature の明示検証）・§5（lane 責務分離）。
