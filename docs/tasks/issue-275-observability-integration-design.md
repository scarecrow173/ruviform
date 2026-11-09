---
id: "TASK-y0ulI5SiiOT2XfWozCd8uK"
type: "task"
status: "done"
title: "Issue #275 Observability統合設計正本 — 単一クレート・OBS-01〜11・両Consumer 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/101"
    - "https://github.com/scarecrow173/ruviform/issues/103"
    - "docs/specs/issue-275-observability-integration-design.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/specs/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/plans/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/tasks/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-275-observability-integration-design.md"
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
      - type: "doc"
        path: "docs/designs/foundation-observability-minimal-api.md"
      - type: "doc"
        path: "docs/designs/shared-foundations-and-modularization-policy.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-275-observability-integration-design.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-275-observability-integration-design.md"
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
    - "docs/test-specs/issue-275-observability-integration-design.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #275 Observability統合設計正本 — 単一クレート・OBS-01〜11・両Consumer 設計

## Work

Issue #275（Observability統合設計正本 — 単一クレート・
OBS-01〜11・両Consumer、2026-11-04 合意版）の合意済み
設計を `docs/` チェーンに正本化する docs のみのスライ
ス。spec → design → test-spec → plan → task を作成し、
単一クレート構成・論理依存 DAG・横断合意事項 1〜10・
feature 突合・RT/non-RT 境界・Source/Generation/Clock/
Loss 非偽装・Consumer 利用面（AudioLens/Synvelis）・
受入 Gate 9 項目との証拠項目対応・#101/#103 前提更新を
記述する。コード・テスト・CI・依存設定・Consumer repo
は変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- [docs/plans/issue-275-observability-integration-design.md](../plans/issue-275-observability-integration-design.md)
- [docs/specs/issue-275-observability-integration-design.md](../specs/issue-275-observability-integration-design.md)
- [docs/designs/issue-275-observability-integration-design.md](../designs/issue-275-observability-integration-design.md)
- [docs/test-specs/issue-275-observability-integration-design.md](../test-specs/issue-275-observability-integration-design.md)
- Issue #275（受入正本: Observability統合設計正本
  2026-11-04 合意版）

## Dependencies

- 参照正本（読取のみ）: `docs/designs/issue-276-…`〜
  `issue-293-…`（per-module 詳細）、`docs/adr/
  obs-05-observability-leaf-primitive-deps.md`、
  `docs/designs/issue-294-…`・`issue-219-…` 系文書、
  `crates/ruviform-observability/`（実装済み crate）。
- 変更対象（注記のみ）: `docs/designs/
  foundation-observability-minimal-api.md`・`docs/designs/
  shared-foundations-and-modularization-policy.md`。
- 既存の範囲外 broken relation（#243/#313/#410 系）は
  本タスクでは修正しない。

## Affected Files

- `docs/specs/issue-275-observability-integration-design.md`（新規）
- `docs/designs/issue-275-observability-integration-design.md`（新規）
- `docs/test-specs/issue-275-observability-integration-design.md`（新規）
- `docs/plans/issue-275-observability-integration-design.md`（新規）
- `docs/tasks/issue-275-observability-integration-design.md`（新規）
- `docs/designs/foundation-observability-minimal-api.md`（改訂注記）
- `docs/designs/shared-foundations-and-modularization-policy.md`（改訂注記）
- `docs/{designs,specs,plans,tasks,test-specs}/README.md`（生成 index）

## Done When

- [x] 5 文書が作成され front matter の relation が相互に
  解決する（spec↔design↔test-spec↔plan↔task）。
- [x] 設計文書が Issue #275 の受入条件（単一クレート/
  Feature DAG・RT 境界・契約試験対応・Source 非偽装・
  両 Consumer 利用面・外部故障耐性・Evidence・#101/#103
  前提更新）を網羅する。
- [x] 参照先（Issue/PR/test-spec/docs パス/crate/
  module/feature 名）が作成時点で実在する。
- [x] docs index が再生成され、audit で新規文書の error
  が 0 である（既存の範囲外 finding は分離して報告）。
- [x] `git fetch origin main && git merge origin/main`
  後に index 再生成・再検証を実施。
- [x] `mise run fmt-check` が pass。
- [x] diff が `docs/` の新規 `issue-275-*` 文書・前提
  2 文書の注記・生成 index のみであることを確認。
- [x] review-protocol レビューで blocker がない。
- [x] commit → `feature/275-obs-design` push → 日本語
  PR 作成（`Refs #275`）。

## Verification

- [x] `mise run fmt-check`（canonical 検証入口）— pass。
- [x] `.agents/skills/doc-maintenance/scripts/doc_maintenance.js
  apply` で index 再生成 — 5 index 更新、issue-275 関連
  finding 0。
- [x] `audit_docs`（doc-status 系 lint）で新規文書 error 0
  — `audit_docs.js --type all` で issue-275 path/ID に
  対する finding 0（全体 finding は全て既存の範囲外
  問題）。
- [x] `git status`/`git diff --stat` で変更範囲が docs の
  みであることを確認 — 新規 `issue-275-*` 5 文書 + 前提
  2 文書注記 + 5 生成 index。
- [x] 実装テスト・cargo check/test/lint・Consumer 実測・
  compile matrix・実機 RT は本 slice 対象外のため
  `NotRun`/`Unsupported` として記録（未実施を実施済みと
  報告しない）。
