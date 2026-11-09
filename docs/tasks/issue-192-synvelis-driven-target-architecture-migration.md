---
id: "TASK-034UCcxDGa2UGOpkpXCbQJ"
type: "task"
status: "done"
title: "Issue #192 Synvelis-driven Target Architecture & Migration — umbrella migration 設計正本"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
      - type: "doc"
        path: "docs/specs/issue-192-synvelis-driven-target-architecture-migration.md"
      - type: "doc"
        path: "docs/plans/issue-192-synvelis-driven-target-architecture-migration.md"
      - type: "doc"
        path: "docs/tasks/issue-192-synvelis-driven-target-architecture-migration.md"
      - type: "doc"
        path: "docs/test-specs/issue-192-synvelis-driven-target-architecture-migration.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-192-synvelis-driven-target-architecture-migration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-192-synvelis-driven-target-architecture-migration.md"
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
    - "docs/test-specs/issue-192-synvelis-driven-target-architecture-migration.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #192 Synvelis-driven Target Architecture & Migration — umbrella migration 設計正本

## Work

Issue #192（Synvelis-driven Ruviform Target v0.7.1
umbrella migration tracking Issue）の合意済み設計を
`docs/` チェーンに正本化する docs のみのスライス。
spec → design → test-spec → plan → task を作成し、
umbrella 設計文書と #193 を設計/実行の権威とする
tracking canonical として、TA-001〜TA-012 gate 対応・
capability→正本 map（M0〜M7・GUI lane・IO・
Observability・Cross-Consumer gate）・3 者
Ownership boundary（Ruviform/Synvelis/AudioLens
対等）・G0〜G7/E2E-L/E2E-R 構造・ADR/互換 policy・
Consumer 接続面・設計合意と実装/検証の分離を記述する。
コード・テスト・CI・依存設定・Consumer repo・umbrella
設計文書・#193 本文は変更しない。M0〜M7 の実装・gate
受入・Consumer 実測は対象外。

## Classification

- `normal-plan-task`

## Upstream Documents

- [docs/plans/issue-192-synvelis-driven-target-architecture-migration.md](../plans/issue-192-synvelis-driven-target-architecture-migration.md)
- [docs/specs/issue-192-synvelis-driven-target-architecture-migration.md](../specs/issue-192-synvelis-driven-target-architecture-migration.md)
- [docs/designs/issue-192-synvelis-driven-target-architecture-migration.md](../designs/issue-192-synvelis-driven-target-architecture-migration.md)
- [docs/test-specs/issue-192-synvelis-driven-target-architecture-migration.md](../test-specs/issue-192-synvelis-driven-target-architecture-migration.md)
- Issue #192（受入正本: umbrella 設計・TA-001〜TA-012・
  M0〜M7・2026-11-02 両 Consumer 対等原則追補）
- Issue #193（M0〜M7 実行計画・G0〜G7・GUI lane 補足 —
  設計/実行の権威）

## Dependencies

- 参照正本（読取のみ）: umbrella 設計文書
  `docs/designs/synvelis-driven-target-architecture-and-
  migration.md`（approved）、`docs/discovery/
  issue-194-m0-baseline-audit.md`、`docs/specs/
  issue-219-consumer-compat-contract.md`（proposed）、
  領域別 `docs/designs|specs|test-specs|plans|tasks/
  issue-18x〜19x・28x〜30x・38x〜46x-*.md`、ADR 各件
  （`synvelis-driven-generic-execution-boundaries`・
  `configurable-ppqn-…`・`realtime-contract-…`・
  `plugin-rt-ownership-split`・`media-io-backend-
  abstraction`・`audio-engine-snapshot-swap`・
  `musical-time-model`・`ui-architecture-webview-
  hybrid`・`driver-neutral-gui-…-issue-384`）。
- 既存の範囲外 broken relation（#243/#313/#410 系）は
  本タスクでは修正しない。

## Affected Files

- `docs/specs/issue-192-synvelis-driven-target-architecture-migration.md`（新規）
- `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`（新規）
- `docs/test-specs/issue-192-synvelis-driven-target-architecture-migration.md`（新規）
- `docs/plans/issue-192-synvelis-driven-target-architecture-migration.md`（新規）
- `docs/tasks/issue-192-synvelis-driven-target-architecture-migration.md`（新規）
- `docs/{designs,specs,plans,tasks,test-specs}/README.md`（生成 index）

## Done When

- [x] 5 文書が作成され front matter の relation が相互に
  解決する（spec↔design↔test-spec↔plan↔task）。
- [x] 設計文書が Issue #192 の受入条件（権威非侵害・
  TA-001〜TA-012 対応・capability map・3 者
  boundary・ADR/互換・Consumer 接続面・設計/実装分離・
  docs のみ変更）を網羅する。
- [x] 参照先（Issue/PR/test-spec/mise task/crate/doc
  パス）が作成時点で実在する。
- [x] docs index が再生成され、audit で新規文書の error
  が 0 である（既存の範囲外 finding は分離して報告）。
- [x] `git fetch origin main && git merge origin/main`
  後に index 再生成・再検証を実施（`b36f6de2` まで merge・
  index 再生成済み）。
- [x] `mise run fmt-check` が pass。
- [x] diff が `docs/` の新規 `issue-192-*` 文書と生成
  index のみであることを確認。
- [x] review-protocol レビューで blocker がない。
- [ ] commit → `feature/192-migration-design` push →
  日本語 PR 作成（`Refs #192`）。

## Verification

- [x] `gh`/`git log` による関連 Issue・PR・merge 証拠の
  棚卸し（2026-11-09 snapshot）— TA 対応表・capability
  map・consumer 接続面の根拠として記録。
- [x] `.agents/skills/doc-maintenance/scripts/
  doc_maintenance.js apply` で index 再生成（5 index 更新・
  issue-192 finding 0）。
- [x] `audit_docs`（doc-status 系 lint）で新規文書
  error 0（issue-192/新規 ID に対する finding 0・
  全体 finding は全て既存の範囲外問題）。
- [x] `mise run fmt-check`（canonical 検証入口）— pass。
- [x] `git status`/`git diff --stat` で変更範囲が docs
  のみであることを確認。
