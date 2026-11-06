---
id: "DESIGN-aYGxhiXLZHim70lzzr4olp"
type: "design"
status: "approved"
title: "ViewLink.attach authoritative generation 採用設計"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "docs/superpowers/specs/2026-11-07-viewlink-authoritative-generation-design.md"
    - "docs/specs/viewlink-authoritative-generation.md"
    - "https://github.com/scarecrow173/synvelis/pull/171"
  changes:
    added: []
    modified:
      - type: "file"
        path: "ui/binding/src/link.ts"
      - type: "file"
        path: "ui/binding/test/link.test.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/viewlink-authoritative-generation.md"
  implemented-by:
    - "docs/plans/viewlink-authoritative-generation.md"
  depends-on:
    - "docs/specs/viewlink-authoritative-generation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-181-multi-window.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/viewlink-authoritative-generation.md"
    - "docs/superpowers/specs/2026-11-07-viewlink-authoritative-generation-design.md"
  derived-by:
    - "docs/plans/viewlink-authoritative-generation.md"
    - "docs/test-specs/viewlink-authoritative-generation-tests.md"
  verifies: []
  verified-by:
    - "docs/test-specs/viewlink-authoritative-generation-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/376"
    - "https://github.com/scarecrow173/ruviform/issues/377"
    - "https://github.com/scarecrow173/ruviform/issues/378"
  defers: []
  deferred-by: []
---

# ViewLink.attach authoritative generation 採用設計

## Overview

`ViewLink.attach(windowId, generation?)` にoptionalなauthoritative generation引数を追加し、backend attach応答のepochをlink・全binding・query・subscription contextへ一貫して採用するadditive seam。epochの正本はbackendであり、SDKは渡された値を伝搬するだけでcounterを複製しない。詳細な検討・選択肢・受入条件の正本は `docs/superpowers/specs/2026-11-07-viewlink-authoritative-generation-design.md`（2026-11-07 ユーザー承認済み）。

## 詳細

- シグネチャ: `attach(windowId: string, generation?: number): void`。
- 入力検証: `generation` が指定された場合、非負のsafe integerのみ受理。無効値は状態変更前に `RangeError` で拒否し、linkの状態（attached/windowId/generation/subscriptions/bindings）を一切変更しない（原子性）。
- 冪等: attached かつ同 `windowId` で、generationが未指定または現行値と一致 → 既存どおりno-op。
- epoch遷移: attached 中に「異なるwindowId」または「同windowId・指定generationが現行値と不一致」→ 既存の detach+attach 手順を踏む。異windowは従来の `dock-transfer`、同windowのepoch差異は `unmount`（再mount semantic）を detach reason として渡す。
- 採用: `generation` 指定時は `this.generation_ = generation`、省略時は既存の `generation_ += 1`。
- 伝搬: `binding.attach(this.generation_)`・`query.attach(this.generation_)`・`SubscriptionContext.generation` は既存経路のまま（#379で確立済み）。CommandBinding/QueryBinding の `attach(epoch?)` は変更不要。
- LinkRegistry（`attachView`/`moveView`/`detachView`/`closeView`）のシグネチャは変更しない — Synvelis consumer は `link.attach` を直接呼ぶ設計。

## 検討した方法

正本design doc「検討した方法」節に従う: 1. attach応答generation採用（本設計・選択）、2. LinkRegistry保持（不採用）、3. backend guard緩和（不採用）。

## Verification

- [ ] `mise run check-ui-binding` / `test-ui-binding` / `build-ui-binding` green
- [ ] link.test.ts に authoritative 採用・epoch遷移・無効原子性・local fallback の恒久テスト
- [ ] `mise run check-ui-guitest` / `test-ui-guitest` green（replay は generation 未指定の既存経路のみ — oracle変更不要を確認）
