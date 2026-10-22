---
id: "PLAN-XyRGyqOXQg05BHSeuQmtEC"
type: "plan"
test-spec-skip: "design-only Issue — コード変更なし。検証は baseline/plugin-isolation 整合確認 + doc 監査で行う"
status: "approved"
title: "Issue #32 実施計画 — 第 2 ターゲット設計"
created: "2026-11-03"
updated: "2026-11-03"
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
  implements: ["docs/specs/second-target-design.md"]
  implemented-by: []
  depends-on:
    - "docs/designs/second-target-design.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/second-target-design.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/tasks/second-target-design-verify.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #32 実施計画 — 第 2 ターゲット設計

design-only チェーン。spec → ADR ×3 → design → 本 plan → task（検証）で完結。

## Steps

1. spec 確定（3 論点の scope/AC）— done
2. ADR ×3 確定（NLE 編集モデル / OFX 範囲 / 色管理）— done
3. design 確定（責務分割・写像規則・後続 Issue 分解）— done
4. task: baseline 整合検証 + Issue #32 クローズ — `second-target-design-verify.md`

## test-spec-skip

設計のみでコード実装なし。検証 = baseline 整合・relation 監査・graph 完走（task で実施）。
