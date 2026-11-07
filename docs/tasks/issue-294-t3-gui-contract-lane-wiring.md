---
id: "TASK-wTtcOyGIqnDDr4nB4paIK3"
type: "task"
status: "done"
title: "T3 consumer-verify.yml GUI-CONTRACT lane 接続 — 候補 tree guard/semantic step + record 置換 + 構造契約 test（Issue #294）"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-294-gui-contract-lane-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-294-gui-contract-lane-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-294-t1-consumer-verify-evidence-script.md"
    - "docs/tasks/issue-294-t2-consumer-verify-workflow-tuple-ui-lanes.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-294-gui-contract-lane-wiring.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T3 GUI-CONTRACT lane 接続（Issue #294）

## Work

1. 両 verify job に `gui_candidate`（候補 checkout + task probe +
   `mise install --locked rust`）・`gui_boundary`・`gui_semantic` step を追加。
2. Record step の `GUI-CONTRACT checks` / `GUI semantic/fake-oracle checks`
   固定 `not_run` 行を、3 分岐（実行 outcome / task 不在 not_run /
   checkout 未完了）の実 lane 行へ置換。Consumer 側 `not_run` 行を残し、
   `unsupported` 固定行は維持。
3. workflow ヘッダ comment に候補 tree 評価の分離を追記。
4. `consumer-verify-yml-contract.test.mjs` の GUI gate test を新構造へ更新。
5. `docs/development-environment.md` の consumer-verify 記述を整合。

## Verification

- `mise exec -- actionlint .github/workflows/consumer-verify.yml`
- `mise run test-ci-consumer-verify`（61+ 件の構造契約・evidence test）
- `mise run check-ui-driver-boundary` / `mise run test-ui-driver-contract`
  （lane が呼ぶ task 自身の現 tree での pass）

## Classification

- `normal-plan-task`
