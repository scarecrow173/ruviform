---
id: "SPEC-thW5o4Z4OMp1n1quFRk4uh"
type: "spec"
status: "proposed"
title: "Consumer-neutral Workflow/CLI orchestration契約・両Consumer適用性レビュー（Issue #553 RF-AUTX-01）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/553"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/synvelis/issues/259"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-553-workflow-orchestration.md"
      - type: "doc"
        path: "docs/designs/issue-553-workflow-orchestration-design.md"
      - type: "doc"
        path: "docs/adr/issue-553-workflow-cli-contract.md"
      - type: "doc"
        path: "docs/test-specs/issue-553-workflow-orchestration.md"
      - type: "doc"
        path: "docs/plans/issue-553-workflow-orchestration-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-553-workflow-orchestration-task.md"
      - type: "doc"
        path: "docs/impl/ir/issue-553-workflow-orchestration.md"
      - type: "doc"
        path: "docs/impl/exp/issue-553-workflow-orchestration.md"
      - type: "doc"
        path: "evidence/rf553/contract-trace.md"
      - type: "doc"
        path: "evidence/rf553/consumer-applicability.md"
      - type: "doc"
        path: "evidence/rf553/verification.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-553-workflow-orchestration-design.md"
    - "docs/plans/issue-553-workflow-orchestration-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-558-script-contract.md"
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/adr/issue-558-script-backend-contract.md"
    - "docs/designs/issue-155-headless-cli-foundation.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/553"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-553-workflow-orchestration.md"
  references:
    - "evidence/rf553/contract-trace.md"
    - "evidence/rf553/consumer-applicability.md"
    - "evidence/rf553/verification.md"
  defers: []
  deferred-by: []
---

# Consumer-neutral Workflow/CLI orchestration契約・両Consumer適用性レビュー（Issue #553 RF-AUTX-01）

## Problem

Ruviform は Synvelis・AudioLens を支える共通基盤である。Synvelis は AUTX-00〜08（synvelis#259〜#267）で Action/Atomic Macro/Workflow/Headless CLI の統合契約を設計中であり、AudioLens は #127 でオフライン解析・Job/Stage Resume/CLI/Export の継続追従を要求している。両者とも「複数 Step を束ねて実行・取消・再開する」orchestration の形を持つが、その意味論（Capability/Grant/Project/Job/精度）は Consumer ごとに異なる。

Issue #553 は、既存 Ruviform Headless CLI 基盤（#155）と Synvelis AUTX-00〜08 の設計を比較し、**AudioLens にも独立適用できる Workflow 計画・有界 Step 実行・Cancel/Resume・CLI Port の汎用部分だけ**を抽出して、Reuse/Extend/New/Consumer-only/Deferred の決裁を記録する DESIGN-ONLY レビューである。新しい Workflow Engine や共通 CLI crate の実装は本 Issue の Done ではない。

## Audience

- Ruviform 共通基盤の設計・レビュー担当（本契約の Owner 候補を決める）
- Synvelis AUTX lane（synvelis#259〜#274）の実装担当 — 汎用契約を Consumer 側へどう写像するかの判断材料
- AudioLens 追従担当（audio-lens#127, #189） — 共通化が AudioLens へ不要な依存・精度/性能負担を増やさないことの確認材料

## Scope

- 現行 `ruviform-exec`・`ruviform-cli-sample`・`docs/designs/issue-155-headless-cli-foundation.md` の実在 API の棚卸し
- Synvelis `docs/automation/`（AUTX-00 統合契約、PR #420 マージ済み）と synvelis#259〜#267/#271〜#274 の契約境界との照合
- AudioLens の解析/CLI/Export 代表 Scenario（audio-lens#127, #189）との適用性評価
- Workflow/Action/Script の責務が rf#558〜#560（Script/Analyzer 契約）と重複しない対応表
- 汎用候補ごとの Reuse/Extend/New/Consumer-only/Deferred 判定、最小 generic slice・Owner・後続 Issue 候補・再評価 Trigger の決裁
- 正例・反例を含む 2 Consumer 代表 Scenario での Plan/Step/Cancel/Resume/Budget/Receipt 適用性

## Non-goals

- Workflow Engine・共通 CLI crate・`synvelis-automation`/`synvelis-cli`/`al-cli` の実装
- Synvelis Capability Gateway・Grant/Approval・Project/CommitGate/Composite・JobPort の共通基盤への移動
- AudioLens への Workflow/Script 機能の強制導入
- GPUI 依存の導入、RT callback での orchestration、OS/FS/Network/Shell の汎用権限の付与
- #558〜#560 の Script Runtime/Analyzer PoC の実施またはそれらを Blocker とすること
- 実装・両 Consumer 移行・OS Native 実走の Pass 主張

## Contract outcome

本スライスが生む成果物は「文書化された契約と決裁」である:

1. **非重複・責務境界表**: 既存 Ruviform Exec/Job/CLI/IO・#155・Synvelis AUTX・rf#558〜560・AudioLens の各責務が第二正本を作らない対応表（`evidence/rf553/contract-trace.md`）
2. **Consumer-neutral 契約語彙**: Versioned Bounded Plan・Step Port・Run/Step State・Cancel Requested/Confirmed・Indeterminate・Idempotency/Digest・Budget・CLI Result 境界の型レベル契約（design §C）
3. **適用性判定**: 各候補の Reuse/Extend/New/Consumer-only/Deferred 判定と最小実装責務・後続 Issue・再評価 Trigger（`evidence/rf553/consumer-applicability.md`、ADR）
4. **反例固定**: Capability/取消/Resume/Budget/Receipt の正例と Fail-Closed 反例（test-spec の Rule/Example）

## Acceptance criteria

- [ ] 現行 Ruviform Exec/Job/CLI/IO と #155 の実在 API を棚卸しし、Workflow/Action/Script の責務が #558〜560 と重複しない対応表を作成する。
- [ ] Synvelis Action/Workflow/Headless と AudioLens 分析/CLI/Export の代表 Scenario 各 1 件以上で、同一 Plan/Step/Cancel/Resume/Budget/Receipt が通用する正例・反例を記録する。
- [ ] Generic Port の公開型・Dependency/NonRT/Grant/Trust/Job ownership 境界と、Consumer 型の流入禁止、両 Consumer の精度/性能回帰基準を固定する。
- [ ] Reuse/Extend/New/Consumer-only/Deferred 別に最小実装責務と必要な後続 Issue を決裁し、共通化不能の場合は No-go 理由と再評価 Trigger を記録する。
- [ ] 本設計だけで Workflow Engine 実装・両 Consumer 移行・OS Native 実走が Pass したとは主張しない。

## Evidence

- `evidence/rf553/contract-trace.md` — 受入条件→契約条項→根拠参照の対応表
- `evidence/rf553/consumer-applicability.md` — 2 Consumer 適用性判定ログ（Reuse/Extend/New/Consumer-only/Deferred）
- `evidence/rf553/verification.md` — 実行した検証コマンド・OS・toolchain・exit code・Pass/NotRun/Unsupported の記録
