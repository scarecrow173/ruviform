---
id: "TASK-aN7eM2gEouRJ8oS3WbU4Ee"
type: "task"
status: "done"
title: "Issue #650 bench-smoke lane coverage 拡張 — mise.toml/dev-env doc/evidence 是正"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "devin-worker"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/650"
  changes:
    added:
      - type: "file"
        path: "evidence/rf650/bench-smoke-excerpt.txt"
      - type: "file"
        path: "evidence/rf650/verification.md"
      - type: "file"
        path: "docs/specs/issue-650-bench-smoke-coverage.md"
      - type: "file"
        path: "docs/designs/issue-650-bench-smoke-coverage.md"
      - type: "file"
        path: "docs/plans/issue-650-bench-smoke-coverage.md"
      - type: "file"
        path: "docs/tasks/issue-650-bench-smoke-coverage.md"
      - type: "file"
        path: "docs/test-specs/issue-650-bench-smoke-coverage.md"
      - type: "file"
        path: "docs/impl/ir/issue-650-bench-smoke-coverage.md"
      - type: "file"
        path: "docs/impl/exp/issue-650-bench-smoke-coverage.jsonl"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-650-bench-smoke-coverage.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-650-bench-smoke-coverage.md"
    - "docs/specs/issue-650-bench-smoke-coverage.md"
    - "docs/designs/issue-650-bench-smoke-coverage.md"
    - "docs/test-specs/issue-650-bench-smoke-coverage.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-650-bench-smoke-coverage.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-650-bench-smoke-coverage.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #650 bench-smoke lane coverage 拡張 — mise.toml/dev-env doc/evidence 是正

## Work

`mise.toml` `[tasks.bench-smoke]` の `run` に
`-p ruviform-numeric -p ruviform-io-e2e` を追加し、
`description` を 5 package 列挙へ更新する。
`docs/development-environment.md` の `bench-smoke` 行を追随させ、
consumer-verify 記述（L89）へ Case schema v2 の最小追記を行う
（PR #639 landed schema との drift 修正）。実測証拠を
`evidence/rf650/` に記録し、内部文書チェーンを作成する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-650-bench-smoke-coverage.md`（approved）
- `docs/designs/issue-650-bench-smoke-coverage.md`（approved）
- `docs/plans/issue-650-bench-smoke-coverage.md`（approved）
- `docs/test-specs/issue-650-bench-smoke-coverage.md`（approved）
- `POLICIES.md`

## Dependencies

- `crates/ruviform-numeric` `[[bench]] corr`（Issue #265）・
  `crates/ruviform-io-e2e` `[[bench]] io_composition`
  （Issue #306 G05）の実在 — 確認済み。
- Issue #646（PR #649）の bench profile 修正が前提 — merge 済み
  （`d5aa1b4b` = 作業 base）。
- CI `bench` job は `mise run bench-smoke` を呼ぶ既存経路
  （workflow 変更不要）。

## Affected Files

- `mise.toml` — `bench-smoke` task の `run`/`description`
- `docs/development-environment.md` — `bench-smoke` 行 + L89
- `evidence/rf650/` — 実測証拠
- `docs/{specs,designs,plans,tasks,test-specs,impl}/
  issue-650-bench-smoke-coverage.*` — 内部 doc chain
- `docs/*/README.md` — doc-maintenance index 再生成（生成物）

## Done When

- [x] `mise run bench-smoke` が exit 0 で numeric/io-e2e の bench
      target を含めて実行される（実測）
- [x] `mise exec -- cargo bench --workspace --no-run` が exit 0
- [x] `mise run fmt-check` / `mise exec -- prek run --all-files` が
      exit 0
- [x] `mise tasks` の `bench-smoke` description が 5 package 列挙を
      含む
- [x] consumer-verify audit 結果を証拠に記録

## Verification

- [x] `mise run bench-smoke` — Pass（exit 0、5 bench target 実行）
- [x] `mise exec -- cargo bench --workspace --no-run` — Pass
- [x] `mise run fmt-check` — Pass
- [x] `mise exec -- prek run --all-files` — Pass
- [x] `mise tasks` — Pass（description 更新を確認）
- [ ] `*-msrv`・`*-full`・3 OS 横断 — 本 slice の明示対象外
      （Daily Full CI 委譲・NotRun）
