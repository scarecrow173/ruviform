---
id: "SPEC-034WJCD5J0DyrDlQtauRjA"
type: "spec"
status: "approved"
title: "Issue #594 daily-dispatch ci.yml に MSRV lane を追加しローカル必須検証から除外する"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/594"
  changes:
    added:
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
      - type: "file"
        path: "docs/impl/ir/ci-msrv-daily-lane.md"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "ci/summary/summary.mjs"
      - type: "file"
        path: "ci/summary/tests/summary.test.mjs"
      - type: "file"
        path: "crates/ruviform-observability/tests/logging.rs"
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
  implements: []
  implemented-by:
    - "docs/plans/ci-msrv-daily-lane.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ci-msrv-daily-lane.md"
    - "docs/test-specs/ci-msrv-daily-lane.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ci-msrv-daily-lane.md"
  references:
    - "https://github.com/scarecrow173/synvelis/blob/main/.github/workflows/ci.yml"
  defers: []
  deferred-by: []
---

# Issue #594 daily-dispatch ci.yml に MSRV lane を追加しローカル必須検証から除外する

## What Should Be Built

`workflow_dispatch` 専用の `.github/workflows/ci.yml` に MSRV 検証 job `msrv-verify` を追加し、4 つの `*-msrv` mise タスク（`test-ofx-msrv`・`test-ui-test-msrv`・`test-observability-msrv`・`test-io-msrv`）を `rust@1.89.0` toolchain で実行する。job は既存 lane と同一の checkout-sha evidence・`ci-summary` needs 集約・fail-closed 伝播経路に乗る。通常開発（Worker Spec・ローカル必須検証）から `*-msrv` を外す運用（Coordinator 側で別途実施済み、ユーザー指示 2026-11-13）に対応する CI 側の担保を成立させる。

## Why Now

- `workspace.package` の `rust-version = "1.89"` 宣言に対し、CI には MSRV を検証する job がなく、4 タスクはローカル手動実行でのみ担保されていた。
- MSRV lane は `rust@1.89.0` の別 toolchain で対象クレート群を再コンパイルするため、変更ごとのローカル必須検証では最大の単一コストになっている。日次 CI（`daily-ci.yml` が `DAILY_TARGET_WORKFLOW: ci.yml` を dispatch）で担保する方針が決定された。

## Users and Value

- 開発者・検証 Worker: ローカル必須検証から MSRV 全量再コンパイルが外れ、反復検証の所要時間が短縮される。
- プロジェクト: `rust-version` 宣言の実効性が日次 CI verdict で継続検証され、MSRV 破壊の検出が個人のローカル実行習慣への依存から外れる。

## Scope

### In Scope

- `.github/workflows/ci.yml` への `msrv-verify` job 追加（ubuntu-latest・mise 復元・`mise install rust@1.89.0`・cargo registry cache・4 タスク実行・`if: always()` lane evidence 記録）。
- `ci-summary` job の `needs` への登録と、`ci/summary/summary.mjs` の `REQUIRED_JOBS` / `REQUIRED_CHECKOUT_LEGS` への宣言 lane 登録（coordinator 承認の範囲拡張 — 「kept in sync with ci.yml needs list」契約を維持するために必要）。
- `ci/summary/tests/summary.test.mjs` の lane 列挙の整合。
- `crates/ruviform-observability/tests/logging.rs` の retention テスト fixture 修正（coordinator 承認の第 2 範囲拡張 — 初回 `workflow_dispatch` run で `test-observability-msrv` が `rolling_file_retention_*` 2 件で失敗し、原因が seed file の birth time 非区別性にあると特定されたため。テスト無効化・allow 追加は禁止条件つきで、seed file 生成を決定的にする修正のみ許可）。
- doc-chain（spec/design/plan/task/test-spec/IR）と管理 index の再生成。

### Out of Scope

- `*-msrv` mise タスク自体の削除・変更（ローカルで任意実行できる状態は維持する）。
- `daily-ci.yml` の dispatch 判定ロジック、`consumer-verify.yml`、`ci-issue-tracker*.yml` の変更。
- `crates/` 以下の実装コード（`ruviform-observability` の test fixture 修正を除く）、`mise.ci.toml` の変更。
- MSRV clippy 検証（Issue #223 の情報 lane）の CI 常設化 — 別系統として deferred。
- `docs/development-environment.md` 等の lane 一覧記述の更新（編集範囲外。残件として PR に記録）。

## Requirements

1. `msrv-verify` job は synvelis `.github/workflows/ci.yml` の `msrv-verify` と同型とし、`ubuntu-latest`・mise 復元（`mise install --locked`）・`mise install rust@1.89.0`・cargo registry cache・`if: always()` の lane evidence 記録（`GITHUB_STEP_SUMMARY`）を備える。
2. 全 lane 共通の checkout-sha evidence 契約（`CHECKOUT_LEG` env・`git rev-parse HEAD` 記録・`checkout-sha-<leg>-attempt-<n>` artifact upload）を満たし、`ci-summary` の `needs` に登録される。
3. `ci/summary/summary.mjs` の `REQUIRED_JOBS` / `REQUIRED_CHECKOUT_LEGS` に `msrv-verify` を追加し、宣言 lane として fail-closed の対象にする。失敗時の tracker 伝播経路は既存 job と同一（`ci-summary` verdict → run 結論 → `ci-issue-tracker`）とする。
4. 最新 main を取り込み、doc-chain と管理 index を再生成してコミットに含める。
5. lane 削除・skip、既存 job の検証意図を損なう変更を行わない。

## Acceptance Criteria

- AC1: `ci.yml` に `msrv-verify` job が追加され、4 つの `*-msrv` タスクすべてが実行される。
- AC2: `ci-summary` の `needs` と `summary.mjs` の `REQUIRED_JOBS` / `REQUIRED_CHECKOUT_LEGS` に `msrv-verify` が含まれ、失敗が既存 job と同じ経路で伝播する。
- AC3: `mise exec -- actionlint` が exit 0。
- AC4: `gh workflow run ci`（`expected_sha` にブランチ先端）で `msrv-verify` job が green になることを run ログで確認する。
- AC5: doc-chain（spec/design/plan/task/test-spec/IR）と管理 index の再生成がコミットに含まれる。

## Deferred Design Concerns

- MSRV clippy lane の CI 常設化 | 本 Issue の受入は 4 つの `*-msrv` test タスクのみであり、Issue #223 の MSRV clippy は `-A clippy::float_cmp -A clippy::similar_names` を伴う情報 lane として別系統 | MSRV clippy の日次常設要否が判断されたとき | MSRV でのみ発火する lint 退行を日次 CI で拾えない状態が継続する
- `*-msrv` タスクの対象 crate 拡大（workspace 全体の MSRV test） | 現行 4 タスクの対象範囲は Issue #223/#293/#306/#411 で個別に確定済み | MSRV 対象を拡大する Issue が起票されたとき | MSRV 非対応コードが対象外 crate に混入しても検出できない範囲が残る

## Sources and Relations

- Issue #594（本 spec の source）。
- 同型参照: synvelis `.github/workflows/ci.yml` の `msrv-verify` job（ubuntu-latest・`mise install rust@1.89.0`・cargo registry cache・`if: always()` evidence 記録）。
- 機構の正本: Issue #366 Stage 1 の checkout-sha evidence / `ci-summary` fail-closed 集約（`ci/summary/summary.mjs`）、`docs/designs/issue-366-full-ci-stage1.md`。
- MSRV 系の先行: Issue #223（MSRV clippy 情報 lane）、`docs/adr/workspace-msrv-1-85-1-89-fearless-simd.md`（rust-version 1.89 の根拠）。
