---
id: "TASK-OjT0wWawFEmwEe6bibhaMq"
type: "task"
status: "done"
title: "T3 compression-verify CI lane + classifier + impl record (Issue #300)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-300-io-compression-gzip-deflate.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-300-io-compression-gzip-deflate.md"
    - "docs/tasks/issue-300-t2-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-300-io-compression-gzip-deflate.md"
  references: []
  defers: []
  deferred-by: []
---

# T3 compression-verify CI lane + classifier + impl record (Issue #300)

## Work

feature-gated code を既存 CI 契約のまま検証するため、icc-verify と
同型の専用 lane `compression-verify` を導入する:

- `mise.toml` に `check-io-compression`・`lint-io-compression`・
  `test-io-compression` を追加。3 task とも default/`gzip`/`deflate`/
  `gzip,deflate` の全 feature matrix を `&&` 連結で実行する（削減
  feature 集合での dead code は `-D warnings` 化した各組合せでのみ
  検出できる）。array 形 `run` は使わない — `ci/impact` の
  lane-owner 導出が task body を最初の `[` で打ち切る regex parse
  するため。
- `ci/impact/map.mjs` に `compression-verify` を `LANE_IDS`/`LANES`
  （ubuntu・`features: ["gzip","deflate"]`・`requiredContexts` 自身）と
  `OWNER_RULES`（`mise-task-pkgs` で上記 3 task）に登録。
- `.github/workflows/ci.yml` に `compression-verify` job を icc-verify
  型で追加（`needs: [impact]`・`if: always() &&
  needs.impact.outputs.lane_compression_verify != 'false'`・ubuntu・
  mise task 3 本実行）し、`impact` job の outputs と `ci-gate` の
  needs に接続。
- `ci/impact/tests/classify.test.mjs` に
  `crates/ruviform-io-compression/...` -> `verify + compression-verify`
  の期待ケースを追加（workflow.test.mjs は LANE_IDS を自動で拾う）。
- `docs/designs/issue-333-impact-based-ci.md` の lane 表と crate→
  lane-owner 箇所へ `compression-verify` を追記（正本整合）。
- `docs/impl/ir/issue-300-io-compression-gzip-deflate.md` を作成。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-300-io-compression-gzip-deflate.md`
- `docs/designs/issue-333-impact-based-ci.md`

## Dependencies

- 依存: T2 の試験・crate 完成形
- workflow.test.mjs が LANE_IDS と ci.yml の整合を自動検証する

## Affected Files

- `mise.toml`・`ci/impact/map.mjs`・`ci/impact/tests/classify.test.mjs`・
  `.github/workflows/ci.yml`・`docs/designs/issue-333-impact-based-ci.md`・
  `docs/impl/ir/issue-300-io-compression-gzip-deflate.md`

## Done When

 - [ ] `node --test ci/impact/tests/` が全緑
 - [ ] `compression-verify` lane job が ci.yml と map.mjs で整合し
       workflow.test.mjs を通過
 - [ ] `mise run check-io-compression`/`lint`/`test` がローカル緑
 - [ ] impl record が作成済み

## Verification

 - [ ] `mise run fmt`/`check`/`lint`/`test` 全緑 + `node --test
       ci/impact/tests/` 緑
 - [ ] `git merge origin/main` で共有面を最終 merge して再検証
