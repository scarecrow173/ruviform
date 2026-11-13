---
id: "TSPEC-034WJHzDIayt1oSwCvPcSi"
type: "test-spec"
status: "approved"
title: "Issue #594 テスト仕様 — MSRV lane の CI 実行・ci-summary fail-closed 伝播・検証意図の保持"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/594"
    - "docs/specs/ci-msrv-daily-lane.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/ci-msrv-daily-lane.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/ci-msrv-daily-lane.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ci-msrv-daily-lane.md"
    - "docs/specs/ci-msrv-daily-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ci-msrv-daily-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/ci-msrv-daily-lane.md"
  derived-by: []
  verifies:
    - "docs/specs/ci-msrv-daily-lane.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #594 テスト仕様 — MSRV lane の CI 実行・ci-summary fail-closed 伝播・検証意図の保持

## Purpose

MSRV 検証がローカル必須レーンから日次 CI lane へ移るにあたり、(1) `ci.yml` の `msrv-verify` job が 4 タスクを `rust@1.89.0` で実際に実行し、(2) その結果が `ci-summary` の fail-closed 集約と tracker 伝播経路に既存 job と同一の形で乗り、(3) 既存 lane の検証意図を一切弱めないことを保証する。

## Feature

ci MSRV lane — `ci.yml` の `msrv-verify` job と `ci-summary` 集計・`ci/summary` 評価器の宣言 lane 登録。

## Rules

- INV-1(4 タスク実行): `msrv-verify` job は `test-ofx-msrv`・`test-ui-test-msrv`・`test-observability-msrv`・`test-io-msrv` の 4 タスクを全て実行し、各タスクは `mise exec rust@1.89.0` 経由で MSRV toolchain 上で走る。各 step は `if: ${{ !cancelled() }}` とし、先行タスク失敗時も後続タスクが実行される（cancel 時のみ停止）。
- INV-2(宣言 lane): `ci-summary.needs`・`summary.mjs` `REQUIRED_JOBS`・`REQUIRED_CHECKOUT_LEGS`・`summary.test.mjs` 列挙の 4 箇所が `msrv-verify` で一致し、needs からの削除・evidence 欠落が fail-closed で検出される。
- INV-3(sha evidence): `msrv-verify` は `CHECKOUT_LEG=msrv-verify` で `checkout-sha-msrv-verify-attempt-<n>` artifact を upload し、他 lane と同一の実測 sha を供給する。
- INV-4(lane evidence): `if: always()` の step で `GITHUB_STEP_SUMMARY` に run/attempt・commit・宣言 rust-version・4 step outcome を記録する（失敗時も証跡が残る）。
- INV-5(検証意図の維持): 既存 job 定義・`mise.toml` の `*-msrv` タスク・他 workflow・`crates/` を変更せず、lane 削除・skip・検査緩和を行わない。

## Examples

- Given `ci.yml` に `msrv-verify` job が定義されている, When `gh workflow run ci` で dispatch される, Then job が ubuntu-latest で起動し `mise install rust@1.89.0` 後に 4 タスクが実行され、全て success なら `msrv-verify` job は success になる。
- Given `msrv-verify` が `ci-summary.needs` と `REQUIRED_JOBS`/`REQUIRED_CHECKOUT_LEGS` に登録されている, When `msrv-verify` が failure/cancelled/skipped で終了する, Then `ci-summary` が `job-<result>` reason で Fail となり run 全体が失敗し、`ci-issue-tracker` が run 結論経由で tracker に伝播する。
- Given `msrv-verify` が success だが checkout-sha artifact を upload しなかった, When `ci-summary` が評価する, Then `msrv-verify:checkout-sha-missing` で Fail（宣言 leg の evidence 欠落は fail-closed）。

## Guarantees

- `actionlint` が workflow 構文を受理する。
- `mise run test-ci-summary`（`node --test ci/summary/`）が宣言 lane 追加後も pass する。
- `gh workflow run ci`（`expected_sha` = ブランチ先端）の run で `msrv-verify` job が success になり、run ログで 4 タスクの実行と lane evidence 出力を確認できる。
- ローカル必須検証（`fmt-check`/`check`/`lint`/`test`/`test-rt`/`build`）が exit 0 を維持する。

## Non-goals

- `*-msrv` タスク自体のローカル実行可能性・内容の検証（既存タスクをそのまま使う。MSRV lane は CI job が担うため本タスクのローカル必須検証から除外）。
- MSRV clippy（Issue #223 情報 lane）の CI 常設。
- `daily-ci.yml` の dispatch 判定、他 workflow、`docs/development-environment.md` の lane 記述更新。

## Risk

- `msrv-verify` が `needs` にあるが `summary.mjs` 未登録のまま merge されると `undeclared-needs-entry` の二級扱いになり、checkout-sha evidence 欠落が fail しない — 本 spec は 4 箇所同期を保証対象に含める。
- `rust@1.89.0` toolchain install や registry cache の初回冷起動で lane が遅延・失敗し得る — 手動 dispatch の実測で確認し、失敗時は run ログの step 単位で原因を切り分ける。
