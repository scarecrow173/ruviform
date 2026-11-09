---
id: "TASK-034UyRQzDJa5cPKTt3NLFF"
type: "task"
status: "done"
title: "Issue #284 IO-02 Structured Formats — Serde境界・Format別契約・Consumer利用面・証拠項目対応 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/284"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/plans/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/tasks/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/test-specs/issue-284-io-02-structured-formats-serde-boundary.md"
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
    - "docs/plans/issue-284-io-02-structured-formats-serde-boundary.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-284-io-02-structured-formats-serde-boundary.md"
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
    - "docs/test-specs/issue-284-io-02-structured-formats-serde-boundary.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #284 IO-02 Structured Formats — Serde境界・Format別契約・Consumer利用面・証拠項目対応 設計

## Work

Issue #284（IO-02 Structured Formats — Serde 公開・
JSON/YAML/TOML/CSV/XML Adapter 契約）の合意済み設計を
`docs/` チェーンに正本化する docs のみのスライス。
spec → design → test-spec → plan → task を作成し、
Serde 境界・Format/Version 契約・Resource Budget・
Error 契約・Feature/Backend 確定値・Consumer 利用面
（AudioLens/Synvelis）・証拠項目対応（OWN/CSV/XML/
DOC/ERR/COMP・#288 matrix・#306 E2E）・設計/実装分離を
記述する。コード・テスト・CI・依存設定・Consumer repo
は変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- [docs/plans/issue-284-io-02-structured-formats-serde-boundary.md](../plans/issue-284-io-02-structured-formats-serde-boundary.md)
- [docs/specs/issue-284-io-02-structured-formats-serde-boundary.md](../specs/issue-284-io-02-structured-formats-serde-boundary.md)
- [docs/designs/issue-284-io-02-structured-formats-serde-boundary.md](../designs/issue-284-io-02-structured-formats-serde-boundary.md)
- [docs/test-specs/issue-284-io-02-structured-formats-serde-boundary.md](../test-specs/issue-284-io-02-structured-formats-serde-boundary.md)
- Issue #284（受入正本: 合意済み設計 v0.1・v0.2・
  2026-11-04・最終整合性レビュー追記）

## Dependencies

- 参照正本（読取のみ）: `docs/designs/issue-297-…`・
  `issue-298-…`・`issue-299-…`（per-format 詳細）、
  `issue-295-…`・`issue-296-…`（IO-01 File 契約）、
  `issue-288-…`（cross-consumer gate）、`issue-306-…`
  （合成 E2E）、`issue-409-…`（rooted capability）、
  `crates/ruviform-io-structured/`（実装済み crate）。
- 既存の範囲外 broken relation（#243/#313/#410 系）は
  本タスクでは修正しない。

## Affected Files

- `docs/specs/issue-284-io-02-structured-formats-serde-boundary.md`（新規）
- `docs/designs/issue-284-io-02-structured-formats-serde-boundary.md`（新規）
- `docs/test-specs/issue-284-io-02-structured-formats-serde-boundary.md`（新規）
- `docs/plans/issue-284-io-02-structured-formats-serde-boundary.md`（新規）
- `docs/tasks/issue-284-io-02-structured-formats-serde-boundary.md`（新規）
- `docs/{designs,specs,plans,tasks,test-specs}/README.md`（生成 index）

## Done When

- [x] 5 文書が作成され front matter の relation が相互に
  解決する（spec↔design↔test-spec↔plan↔task）。
- [x] 設計文書が Issue #284 の受入条件（Serde 境界・
  Format/Version 契約・driver/backend 中立・consumer
  利用面・証拠項目対応・設計/実装分離）を網羅する。
- [x] 参照先（Issue/PR/test-spec/mise task/crate/doc
  パス）が作成時点で実在する。
- [x] docs index が再生成され、audit で新規文書の error
  が 0 である（既存の範囲外 finding は分離して報告）。
- [x] `git fetch origin main && git merge origin/main`
  後に index 再生成・再検証を実施（Already up to date —
  `0d07471d` 以降の差分なし）。
- [x] `mise run fmt-check` が pass。
- [x] diff が `docs/` の新規 `issue-284-*` 文書と生成
  index のみであることを確認。
- [x] review-protocol レビューで blocker がない。
- [x] commit → `feature/284-io02-formats` push → 日本語
  PR 作成（`Refs #284`）。

## Verification

- [x] `mise run fmt-check`（canonical 検証入口）— pass。
- [x] `.agents/skills/doc-maintenance/scripts/doc_maintenance.js
  apply` で index 再生成 — 5 index 更新、issue-284 関連
  finding 0。
- [x] `audit_docs`（doc-status 系 lint）で新規文書 error 0
  — `audit_docs.js --type all` で issue-284 path/ID に
  対する finding 0（全体 634 finding は全て既存の範囲外
  問題）。
- [x] `git status`/`git diff --stat` で変更範囲が docs のみ
  であることを確認 — 新規 `issue-284-*` 5 文書 + 5 生成
  index のみ。
- [x] 実装テスト・cargo check/test/lint・Consumer 実測は
  本 slice 対象外のため `NotRun`/`Unsupported` として
  記録（未実施を実施済みと報告しない）。
