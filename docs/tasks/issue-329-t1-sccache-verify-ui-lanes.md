---
id: "TASK-yFczRmBPJEUZkKyX3bngmg"
type: "task"
status: "in-progress"
title: "Issue #329 T1 — ci.yml verify/ui-verify へ sccache 導入 + Cold/Warm 実測 + IR"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/329"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-329-sccache-compile-cache.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-329-sccache-compile-cache.md"
    - "docs/specs/issue-329-sccache-compile-cache.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-329-sccache-compile-cache.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-329-sccache-compile-cache.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #329 T1 — ci.yml verify/ui-verify へ sccache 導入 + Cold/Warm 実測 + IR

## Objective

design(`docs/designs/issue-329-sccache-compile-cache.md`)の snippet を `verify`・`ui-verify` job に適用し、静的検証・CI 上の cold/warm 実測・ローカル補助実験・IR/exp 記録まで行う。

## Checklist

- [x] `verify` job: cargo cache restore 直後に sccache setup + github-script export + env check の 3 step を追加
- [x] `verify` job: `test-ci` 後に `sccache --show-stats` 統計 step を追加
- [x] `ui-verify` job: 同 3 step を cache restore 直後に + stats step を evidence upload 前に追加
- [x] `git diff` で job 名・job 数・trigger・permissions・既存 step が非変更(INV-1)
- [x] `mise exec -- actionlint .github/workflows/ci.yml` が exit 0
- [x] `mise exec -- prek run --all-files` green(check-yaml 等)
- [ ] AGENTS.md の ci.yml 記述 bullet に sccache の一文を追加
- [ ] doc index 再生成(`doc_maintenance.js apply`)→ 新規 blocking finding なし
- [ ] CI cold run: 3 OS verify/ui-verify green + `sccache --show-stats` 記録(run URL)
- [ ] CI warm run(2 連続 push): hit 率・wall/runner 時間を cold/#327 baseline と比較
- [ ] repo variable `CI_SCCACHE_DISABLE=1` で無効化 run 検証(権限あれば)
- [ ] ローカル実験: Cargo.lock 更新・feature 切替・破損・無効(非破壊に実施、lockfile は復元)
- [ ] `collect-ci-metrics.mjs` で対象 run 収集 → `docs/impl/exp/data/issue-329-sccache-runs.jsonl`
- [ ] IR(`docs/impl/ir/issue-329-sccache-compile-cache-implementation.md`)+ exp(`docs/impl/exp/issue-329-sccache-compile-cache.md`)に検証・実測・NotRun 記録
