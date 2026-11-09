---
id: "TASK-034UAvL4WBHRKsOVxsrTgG"
type: "task"
status: "done"
title: "Issue #286 IO-04 ZIP/TAR Archive — Entry API・安全な展開・終了/公開契約 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/286"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/specs/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/plans/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/tasks/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/test-specs/issue-286-io-04-zip-tar-archive.md"
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
    - "docs/plans/issue-286-io-04-zip-tar-archive.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-286-io-04-zip-tar-archive.md"
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
    - "docs/test-specs/issue-286-io-04-zip-tar-archive.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #286 IO-04 ZIP/TAR Archive — Entry API・安全な展開・終了/公開契約 設計

## Work

Issue #286（IO-04 ZIP/TAR Archive — Entry API・安全な展開・
終了/公開契約）の合意済み設計を `docs/` チェーンに正本化
する docs のみのスライス。spec → design → test-spec →
plan → task を作成し、形式別 Reader/Writer/Entry 契約・
Backend 隠蔽・所有権/finish・検証済み終了 Receipt・Secure
Extraction の脅威モデル・終了/公開契約・Format 範囲・
Consumer 利用面（AudioLens/Synvelis）・証拠項目対応
（API/ZIP/TAR/SEC/EXT/INT/COMP/DEP/OWN/WRT/REC/OS・#288
matrix・#306 E2E）・設計/実装分離を記述する。コード・
テスト・CI・依存設定・Consumer repo は変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- [docs/plans/issue-286-io-04-zip-tar-archive.md](../plans/issue-286-io-04-zip-tar-archive.md)
- [docs/specs/issue-286-io-04-zip-tar-archive.md](../specs/issue-286-io-04-zip-tar-archive.md)
- [docs/designs/issue-286-io-04-zip-tar-archive.md](../designs/issue-286-io-04-zip-tar-archive.md)
- [docs/test-specs/issue-286-io-04-zip-tar-archive.md](../test-specs/issue-286-io-04-zip-tar-archive.md)
- Issue #286（受入正本: 合意済み設計 v0.1・v0.2・
  2026-11-04・最終整合性レビュー追記）

## Dependencies

- 参照正本（読取のみ）: `docs/designs/issue-302-…`・
  `issue-303-…`・`issue-304-…`（per-implementation 詳細）、
  `issue-295-…`・`issue-296-…`（IO-01 File 契約）、
  `issue-300-…`・`issue-301-…`（IO-03 Compression）、
  `issue-288-…`（cross-consumer gate）、`issue-306-…`
  （合成 E2E）、`issue-409-…`・`rooted-read-capability.md`
  （rooted capability）、`crates/ruviform-io-archive/`
  （実装済み crate）。
- 既存の範囲外 broken relation（#243/#313/#410 系）は
  本タスクでは修正しない。

## Affected Files

- `docs/specs/issue-286-io-04-zip-tar-archive.md`（新規）
- `docs/designs/issue-286-io-04-zip-tar-archive.md`（新規）
- `docs/test-specs/issue-286-io-04-zip-tar-archive.md`（新規）
- `docs/plans/issue-286-io-04-zip-tar-archive.md`（新規）
- `docs/tasks/issue-286-io-04-zip-tar-archive.md`（新規）
- `docs/{designs,specs,plans,tasks,test-specs}/README.md`（生成 index）

## Done When

- [x] 5 文書が作成され front matter の relation が相互に
  解決する（spec↔design↔test-spec↔plan↔task）。
- [x] 設計文書が Issue #286 の受入条件（形式別 Reader/
  Writer/Entry 契約・Backend 隠蔽・所有権/finish・Receipt・
  Secure Extraction・終了/公開契約・Format 範囲・Consumer
  利用面・証拠項目対応・設計/実装分離）を網羅する。
- [x] 参照先（Issue/PR/test-spec/mise task/crate/doc
  パス）が作成時点で実在する。
- [x] docs index が再生成され、audit で新規文書の error
  が 0 である（既存の範囲外 finding は分離して報告）。
- [x] `git fetch origin main && git merge origin/main`
  後に index 再生成・再検証を実施（Already up to date —
  `95b0323f` 以降の差分なし）。
- [x] `mise run fmt-check` が pass。
- [x] diff が `docs/` の新規 `issue-286-*` 文書と生成
  index のみであることを確認。
- [x] review-protocol レビューで blocker がない。
- [x] commit → `feature/286-io04-archive` push → 日本語
  PR 作成（`Refs #286`）。

## Verification

- [x] `mise run fmt-check`（canonical 検証入口）— pass。
- [x] `.agents/skills/doc-maintenance/scripts/doc_maintenance.js
  apply` で index 再生成 — tasks index 1 件適用、630
  finding 前後不変（issue-286 関連 finding 0）。
- [x] `audit_docs`（doc-status 系 lint）で新規文書 error 0
  — spec/design/test-spec/plan/task 各 type で
  issue-286 path/ID に対する finding 0（既存の範囲外
  finding は分離して報告）。
- [x] `git status`/`git diff --stat` で変更範囲が docs のみ
  であることを確認 — 新規 `issue-286-*` 5 文書 + 5 生成
  index のみ。
- [x] 実装テスト・cargo check/test/lint・Consumer 実測は
  本 slice 対象外のため `NotRun`/`Unsupported` として
  記録（未実施を実施済みと報告しない）。
