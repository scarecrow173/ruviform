---
id: "TASK-034RJ02dFTbxsMkC5rKofw"
type: "task"
status: "in-progress"
title: "Issue #331 T1 — verify lane の check-ci を lint-ci に統合 + 等価性・実測記録"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/331"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-331-check-clippy-test-dedup.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-331-check-clippy-test-dedup.md"
    - "docs/specs/issue-331-check-clippy-test-dedup.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-331-check-clippy-test-dedup.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-331-check-clippy-test-dedup.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #331 T1 — verify lane の check-ci を lint-ci に統合 + 等価性・実測記録

## Objective

design(`docs/designs/issue-331-check-clippy-test-dedup.md`)に従い、`ci.yml` verify job から `check-ci` step を除去して `lint-ci` に型検査を統合し、`mise.toml`/`AGENTS.md` を同期する。注入実験・ユニット計測の証拠を test-spec/exp に記録し、CI 実測差分を収集する。

## Checklist

- [x] discovery/spec/design/plan/test-spec の doc chain を作成(実測・等価性証明を記録)
- [x] `ci.yml` `verify` job: `Type-check workspace (mise run check-ci)` step を除去、lint step を `Type-check & lint workspace (mise run lint-ci)` に改名 + コメント
- [x] `mise.toml`: `verify-ci` から `mise run check-ci` を除去・description 更新、`check-ci`/`lint-ci` description を lane 帰属込みに更新
- [x] `AGENTS.md`: 検証入口表と ci.yml bullet を新構成に同期
- [x] `mise exec -- actionlint .github/workflows/ci.yml` exit 0
- [x] `mise exec -- prek run --all-files` green
- [x] `mise run lint-ci` + `mise run test-ci`(新列)が green(ローカル)— `mise run verify-ci` 全体で確認
- [x] doc index 再生成(`doc_maintenance.js apply`)→ 新規 blocking finding なし
- [x] 等価性証明: 注入実験 8 ケース記録済み(型エラー lib/cfg(test)/example/bench・rustc 警告・clippy lint・失敗テスト・check 対称確認)
- [x] ユニット集合同一: check/clippy 292 ユニット・kind 内訳一致(JSONL 集計・discovery/exp 記録)
- [x] CI run 観測: 3OS verify green + required 集合不変 + step 時間差分(PR #343・run 41825566750、check+lint 合計比 −28〜−37s)
- [x] `collect-ci-metrics.mjs` で対象 run 収集 → `docs/impl/exp/data/issue-331-verify-step-runs.jsonl`
- [x] IR(`docs/impl/ir/t1-check-step-dedup-issue-331.md`)+ exp(`docs/impl/exp/issue-331-check-clippy-test-dedup.md`)に検証・実測・NotRun 記録
