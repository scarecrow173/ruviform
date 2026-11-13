---
id: "IMPL-034WJNAwo0m2Ia4NNALd1w"
type: "impl"
status: "in-progress"
title: "Issue #594 MSRV lane implementation record"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/594"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/ci-msrv-daily-lane.md"
      - type: "file"
        path: "docs/specs/ci-msrv-daily-lane.md"
      - type: "file"
        path: "docs/designs/ci-msrv-daily-lane.md"
      - type: "file"
        path: "docs/plans/ci-msrv-daily-lane.md"
      - type: "file"
        path: "docs/tasks/ci-msrv-daily-lane.md"
      - type: "file"
        path: "docs/test-specs/ci-msrv-daily-lane.md"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "ci/summary/summary.mjs"
      - type: "file"
        path: "ci/summary/tests/summary.test.mjs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/tasks/ci-msrv-daily-lane.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/ci-msrv-daily-lane.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #594 MSRV lane implementation record

## Summary

`ci.yml`（workflow_dispatch 専用・daily-ci 経由で日次 dispatch）に `msrv-verify` job を追加し、`mise.toml` の 4 つの `test-*-msrv` タスクを rust 1.89.0 toolchain で CI 実行する経路を新設した。併せて fail-closed 集計の `ci-summary` に lane を登録するため、`ci/summary/summary.mjs` の `REQUIRED_JOBS` / `REQUIRED_CHECKOUT_LEGS` と `ci/summary/tests/summary.test.mjs` の列挙 test を更新した（Coordinator 承認の範囲拡張）。ローカル必須検証（`fmt-check` / `check` / `lint` / `test` / `test-rt` / `build`）には MSRV task を追加せず、第二 toolchain の再コンパイルは CI lane に委譲する。

## Context

Issue #594 の受入条件に基づく。synvelis の `msrv-verify` job を同型参照とし、ruviform では 4 task（`test-ofx-msrv`・`test-ui-test-msrv`・`test-observability-msrv`・`test-io-msrv`）を個別 step として実行する設計とした。`ci-summary` は `needs` だけでなく `summary.mjs` の宣言 lane 一覧と checkout-SHA evidence 宣言の両方を fail-closed で検査するため、job 追加だけでは不足し、evaluator 側の宣言更新が必須だった。

## Implementation

### Changes

#### `.github/workflows/ci.yml`

- `ci-summary` 直前に `msrv-verify` job を追加。`runs-on: ubuntu-latest`、`CARGO_INCREMENTAL: "0"`、`CHECKOUT_LEG: msrv-verify`。
- checkout は `${{ github.sha }}` pin、実測 `git rev-parse HEAD` を `checkout-sha-msrv-verify-attempt-<n>.txt` として upload（既存 lane と同一の evidence 契約）。
- mise 復元（`mise trust` + `mise install --locked rust prek`）、専用 cargo cache（`ubuntu-cargo-msrv-<Cargo.lock hash>` / restore key `ubuntu-cargo-msrv-`）、`mise install rust@1.89.0` を配置。
- 4 task を個別 step（`msrv-ofx` / `msrv-ui-test` / `msrv-observability` / `msrv-io`）で実行。各 step に `if: ${{ !cancelled() }}` を付し、先行 task 失敗時も後続 task が skip されず全件実行される（review で指摘された coverage gap への対応 — matrix lane の `fail-fast: false` と同じ意図）。`if: always()` の lane evidence step が run/attempt・実測 commit・宣言 rust version・各 task outcome を `$GITHUB_STEP_SUMMARY` に記録。
- `ci-summary.needs` に `msrv-verify` を追加。

#### `ci/summary/summary.mjs`

- `REQUIRED_JOBS` と `REQUIRED_CHECKOUT_LEGS` に `"msrv-verify"` を追加。これにより job 未登録・非 success 結果・checkout evidence 欠落が既存 lane と同じく fail-closed で検出される。

#### `ci/summary/tests/summary.test.mjs`

- required-job 列挙 test と checkout-leg 列挙 test の双方に `"msrv-verify"` を追加。

#### doc-chain（`docs/{specs,designs,plans,tasks,test-specs}/ci-msrv-daily-lane.md`）

- spec → design → plan → task → test-spec の canonical chain を作成。範囲拡張（`ci/summary/*`）を design/plan/task に明記。

### Decisions During Implementation

- **`mise.toml` は無変更**: 4 つの `test-*-msrv` task は既に `mise exec rust@1.89.0` で第二 toolchain を明示選択し、target dir も分離済み（`target/msrv-*`）。task 調整は不要と判断。
- **`ci-summary` 宣言の 3 箇所更新**: workflow `needs` だけでは evaluator が undeclared job として扱い、checkout evidence の fail-closed 検査も効かない。`summary.mjs` + test の両方を更新するのが唯一の整合的変更。
- **4 task を個別 step 化**: synvelis は単一 `test-msrv` task だが、ruviform は task が 4 分割済み。個別 step + ID により、先行 task 失敗時も後続 task の outcome を evidence に残せる。
- **4 step に `if: ${{ !cancelled() }}`（review-protocol 対応）**: 初版は default の `success()` ゲートで、先行 task 失敗時に後続 3 task が skip される設計だった。1 件の MSRV 破損が修正されるまで他 crate set の独立破損を不可視にする coverage gap があるため、失敗後も全 task を実行する形に修正。cancel 時は停止する `!cancelled()` を選び `always()` は避けた（cancelled run で証跡だけ残る誤判定を防ぐ）。
- **IR file 名を doc-chain slug に統一**: `new_impl_record.js` の既定名 `issue-594-msrv-lane-implementation-record.md` から `ci-msrv-daily-lane.md` へ rename し、spec/plan が宣言していた `docs/impl/ir/ci-msrv-daily-lane.md` への参照と一致させた（review 指摘）。
- **ローカル必須検証から MSRV を除外**: 第二 toolchain の全量再コンパイルはコストが高く、日次 CI lane が担保する。Issue の意図どおりローカル必須セットは不変。

## Related Experiments

### Adopted

- なし（実験 log は作成していない — 設計判断は design doc に集約）。

### Rejected

- なし。

## Validation

実施済み:

- `mise exec -- actionlint`（変更前 baseline）→ exit 0、警告なし。
- `mise exec -- actionlint`（変更後）→ exit 0。
- `mise run test-ci-summary` → exit 0（31 tests / 31 pass / 0 fail / 0 skipped）。

実施中 / 未実施（結果はコミット前に更新）:

- `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `build` — ローカル必須検証チェーン実行中。
- `gh workflow run ci`（`expected_sha` = ブランチ先端）→ dispatch 後に `msrv-verify` の green を run ログで確認予定。結果は PR 本文に記載。

## Risks

- `msrv-verify` は `rust@1.89.0` の初回 install と第二 toolchain の全量ビルドを伴い、他 lane より実行時間が長い。専用 cargo cache（`cargo-msrv` key）で緩和するが、cold cache 時はさらに延びる可能性がある。
- MSRV task が CI で初めて 1.89.0 実機検証されるため、既存の潜在的 MSRV 破損が顕在化する可能性がある。その場合は本 lane の失敗として正しく検出される（fail-closed の意図どおり）。
- `docs/development-environment.md` の lane 数記述（旧来の宣言数）が本変更で stale になるが、編集範囲外のため未対応。PR 本文の「未対応の作業」に記載。

## Follow-ups

- `gh workflow run ci` dispatch による `msrv-verify` 実機 green 確認（PR 作成後に実施・結果を PR 本文へ記載）。
- `docs/development-environment.md` の lane 構成記述を新 lane 数に合わせて更新する別タスク。
