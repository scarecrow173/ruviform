---
id: "TASK-034WJHI8uOfxwmbj8vGwDa"
type: "task"
status: "done"
title: "Issue #594 T1 — ci.yml MSRV lane 追加・ci-summary 宣言登録・workflow_dispatch 検証"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/594"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/ci-msrv-daily-lane.md"
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
    - "docs/test-specs/ci-msrv-daily-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/ci-msrv-daily-lane.md"
  derived-by: []
  verifies:
    - "docs/test-specs/ci-msrv-daily-lane.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #594 T1 — ci.yml MSRV lane 追加・ci-summary 宣言登録・workflow_dispatch 検証

## Objective

`ci.yml` に `msrv-verify` job を追加し、4 つの `*-msrv` タスクを `rust@1.89.0` で実行する日次 CI lane を成立させ、ブランチの `ci` workflow run で `msrv-verify` の success を確認する。

## Checklist

- [x] doc-chain（spec/design/plan/task/test-spec + IR）作成
- [x] `ci.yml` に `msrv-verify` job 追加（ubuntu-latest・checkout-sha evidence・mise 復元・`-cargo-msrv-` cache・`mise install rust@1.89.0`・4 タスク個別 step・`if: always()` lane evidence）
- [x] `ci-summary.needs` に `msrv-verify` を追加
- [x] `ci/summary/summary.mjs` の `REQUIRED_JOBS` / `REQUIRED_CHECKOUT_LEGS` と `summary.test.mjs` の列挙に `msrv-verify` を追加（coordinator 承認の範囲拡張）
- [x] `mise exec -- actionlint` exit 0（baseline も exit 0・警告なし）
- [x] `mise run test-ci-summary` exit 0（31 tests pass / 0 fail）
- [x] 通常 lane 検証: `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `build`（fixture 修正 commit 後に 6 件全て再実施・全て exit 0）
- [x] `mise.toml` の `*-msrv` タスクが変更不要であることを確認（既存の `mise exec rust@1.89.0` で CI からそのまま呼べる）
- [x] 触った docs dir の管理 index を doc-maintenance `index apply` で再生成
- [x] `git merge origin/main` → commit → push（`crates/ruviform-observability/tests/logging.rs` の fixture 修正を含む — coordinator 承認の範囲拡張）
- [x] `gh workflow run ci`（`expected_sha` = ブランチ先端）で `msrv-verify` success を確認し URL を PR に記載
  - 1 回目 dispatch（run 43383950490, expected_sha 4a6026d8）: `test-observability-msrv` が `rolling_file_retention_deletes_oldest_at_build` / `rolling_file_retention_matches_prefixed_and_suffixed_files` で失敗。原因は seed file の birth time 非区別性（upstream `prune_old_logs` が `Metadata::created()` でソート、Linux で連続 write が同一 tick を共有）と特定。`ci-summary` は `msrv-verify:job-failure` で fail-closed（evidence 契約は正常動作）。
  - fixture 修正: seed 生成間に 100ms の実時間間隔（`SEED_FILE_BIRTH_GAP`）を挿入し birth time を区別可能にした（テスト無効化・allow 追加なし）。ローカルで `cargo test -p ruviform-observability --all-features --test logging` 28/28 pass。
  - 2 回目 dispatch（run 43385858552, expected_sha f43ea2c = ブランチ先端）: **run 全体 success**。`msrv-verify` success（4 task step 全て success）・`ci-summary` verdict PASS。https://github.com/scarecrow173/ruviform/actions/runs/43385858552
