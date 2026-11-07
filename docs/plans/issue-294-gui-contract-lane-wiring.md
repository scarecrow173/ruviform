---
id: "PLAN-ARAErEdgpSOlWBQBofZQ2t"
type: "plan"
status: "approved"
title: "Issue #294 GUI-CONTRACT lane 接続 実装計画 — 候補 checkout/probe step・record 置換・contract test 更新"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-gui-contract-lane-wiring.md"
    - "docs/designs/issue-294-gui-contract-lane-wiring.md"
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
    - "docs/designs/issue-294-gui-contract-lane-wiring.md"
  implemented-by:
    - "docs/tasks/issue-294-t3-gui-contract-lane-wiring.md"
  depends-on:
    - "docs/designs/issue-294-gui-contract-lane-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-gui-contract-lane-wiring.md"
    - "docs/designs/issue-294-gui-contract-lane-wiring.md"
  derived-by:
    - "docs/tasks/issue-294-t3-gui-contract-lane-wiring.md"
    - "docs/test-specs/issue-294-gui-contract-lane-wiring.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-gui-contract-lane-wiring.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #294 GUI-CONTRACT lane 接続 実装計画

## Goal

`consumer-verify.yml` の両 verify job に候補 tree 評価 step
（`gui_candidate` / `gui_boundary` / `gui_semantic`）を追加し、Record step の
固定 `not_run` 行 2 本を step outcome 由来の lane 行へ置き換える。
workflow 構造契約 test と `docs/development-environment.md` を整合更新する。

## Scope

In: `.github/workflows/consumer-verify.yml`、
`ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs`、
`docs/development-environment.md`、chain docs（本 plan/task/spec/design/
test-spec、impl record）。
Out: `evidence.mjs` 本体、`ci/daily`、`ci.yml`、Consumer repo、`crates/`。

## Task Breakdown

| task | 内容 | 検証 |
|---|---|---|
| [T3](../tasks/issue-294-t3-gui-contract-lane-wiring.md) | workflow 両 job へ step 追加 + Record 置換、contract test・docs 更新 | actionlint / `mise run test-ci-consumer-verify` / `mise run check-ui-driver-boundary` / `mise run test-ui-driver-contract` |

## Risk Register

- 候補に task がない旧 SHA で `not_run` ではなく `fail` になる混入
  → probe の grep 判定を record 分岐に pin し contract test で固定。
- `mise install --locked rust` が候補の mise.lock 欠落で失敗 → step fail =
  正直な fail 記録（偽 pass 側には倒れない）。
- deploy_key mode の `url.insteadOf` rewrite に依存する fetch → `dep` step
  より後の配置を contract test で pin。

## Rollback

`.github/workflows/consumer-verify.yml` と test file を PR 前の revision へ
revert。固定 `not_run` 行に戻るだけで lane 構成上の永続影響なし。
