---
id: "TASK-034RBNKfGF6WS4OfHfW1of"
type: "task"
status: "done"
title: "Issue #327 CI計測Baseline 計測実施・文書化"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/327"
  changes:
    added:
      - type: "directory"
        path: "ci/metrics"
      - type: "file"
        path: "docs/impl/exp/issue-327-ci-baseline.md"
      - type: "file"
        path: "docs/impl/ir/issue-327-baseline-measurement.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/data/issue-327-ci-baseline-runs.jsonl"
  implements:
    - "docs/plans/issue-327-cross-repo-ci-baseline.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-327-cross-repo-ci-baseline.md"
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
    - "docs/test-specs/issue-327-cross-repo-ci-baseline.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #327 CI計測Baseline 計測実施・文書化

## Work

`ci/metrics/collect-ci-metrics.mjs` を実装し、3 repo（ruviform/audio-lens/synvelis）の直近 Run を `gh api` で実測する。計測契約は `docs/designs/issue-327-cross-repo-ci-baseline.md` の正本に従う。成果は experiment log（Baseline）+ raw JSONL + IR として記録する。CI/workflow/依存の変更は行わない。

実施順:
1. `collect-ci-metrics.mjs` 実装（workflows/runs/jobs/steps 収集、指標算出、lane 分類、needs 静的解決による critical path、required contexts 再導出、`--logs` で cache 解析、JSONL+report 出力）。
2. `ci/metrics/README.md` — 実行手順・オプション・出力形式。
3. 実測: timing 収集（各 repo 直近 run）+ 代表 run の `--logs` cache 解析。
4. `docs/impl/exp/issue-327-ci-baseline.md` に Baseline 記録（代表 run・条件・指標・cache・必須検証集合・Before/After 利用手順）。
5. raw JSONL を `docs/impl/exp/data/` に保存、IR 作成。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-327-cross-repo-ci-baseline.md`（approved）
- `docs/specs/issue-327-cross-repo-ci-baseline.md`（approved・AC1〜AC6）
- `docs/designs/issue-327-cross-repo-ci-baseline.md`（approved・計測契約）
- `docs/discovery/issue-327-cross-repo-ci-baseline.md`（resolved・前提実測）

## Dependencies

- ambient `gh` CLI（public repo の read token。実測済み）
- `mise exec -- node`（node 26.12.1 pin）

## Affected Files

- `ci/metrics/collect-ci-metrics.mjs`（新規）
- `ci/metrics/README.md`（新規）
- `docs/impl/exp/issue-327-ci-baseline.md`（新規）
- `docs/impl/exp/data/issue-327-ci-baseline-runs.jsonl`（生成・保存）
- `docs/impl/ir/issue-327-baseline-measurement.md`（新規）
- docs 各 index（自動生成）

## Done When

- [x] spec AC1〜AC6 を全て満たす（test-spec R1〜R8 の検証を通過）。
- [x] スクリプトが exit 0 で `runs.jsonl`/`report.md` を生成し、3 repo の Baseline が exp log に記録されている。
- [x] 必須検証集合（required contexts + job 集合）が API 出典つきで記録され、Before/After 差分監査に使える。
- [x] NotRun/Unsupported/unknown が理由つきで明記され、Pass と混同されていない。
- [x] IR に実行コマンド・結果・未実施項目が記録されている。

## Verification

- [x] `mise exec -- node ci/metrics/collect-ci-metrics.mjs --out /tmp/ci327-out` が exit 0 で runs.jsonl + report.md を生成（R1）。
- [x] `mise exec -- node ci/metrics/collect-ci-metrics.mjs --logs-run <representative_run_id>` で cache Hit/Miss が report に出る（R5/R3）。
- [x] report.md に wall/wait/exec/span/critical_path/runner_total が別欄で出る（R2）。
- [x] `mise run fmt-check` / `mise run check` / `mise run lint` が green（または未実行を IR に明記）。
- [x] `git status` で `.github/`・audio-lens/synvelis 側に差分なし（R8）。
- [x] `node .agents/skills/doc-status/scripts/doc_status.js --cwd . lint` で doc suite の index/relation エラーなし。
