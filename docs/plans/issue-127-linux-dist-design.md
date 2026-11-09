---
id: "PLAN-034udQraUPXx00zp4pCYaL"
type: "plan"
status: "draft"
title: "Issue #127 Linux 実配布方式 設計文書チェーン 実装計画 — docs-only 5 文書 + 索引再生成 + 検証"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/127"
    - "docs/specs/issue-127-linux-dist-design.md"
    - "docs/designs/issue-127-linux-dist-design.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/designs/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/plans/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/tasks/issue-127-linux-dist-design.md"
    modified:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-127-linux-dist-design.md"
  implemented-by:
    - "docs/tasks/issue-127-linux-dist-design.md"
  depends-on:
    - "docs/specs/issue-127-linux-dist-design.md"
    - "docs/designs/issue-127-linux-dist-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-127-linux-dist-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-127-linux-dist-design.md"
  derived-by:
    - "docs/tasks/issue-127-linux-dist-design.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-127-linux-dist-design.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #127 Linux 実配布方式 設計文書チェーン 実装計画

## Goal

Issue #127 本文の受入 5 行を、Ruviform リポジトリの正規 docs チェーン（spec → design → test-spec → plan → task）として正本化し、`docs/` のみの変更で PR 可能な状態にする。コード・テスト・CI・依存は一切変更しない。

## Scope

### In Scope

- `docs/specs/issue-127-linux-dist-design.md`（SPEC-034udQra6CS6JQJJN4e0hG）
- `docs/designs/issue-127-linux-dist-design.md`（DESIGN-034udQraUSrCxCnhZza67E）
- `docs/test-specs/issue-127-linux-dist-design.md`（TSPEC-034udQraULNWJKR6z8GYjq）
- `docs/plans/issue-127-linux-dist-design.md`（本書・PLAN-034udQraUPXx00zp4pCYaL）
- `docs/tasks/issue-127-linux-dist-design.md`（TASK-034udQraUdxLs1SxuYiaAK）
- `docs/{specs,designs,test-specs,plans,tasks}/README.md` の管理索引再生成

### Out of Scope

- `crates/`・`Cargo.toml`/`Cargo.lock`・`mise.toml`・`.github/` を含む一切の非 docs 変更
- 対象 distro 決定・Linux 実機検証・パッケージ形式最終採用・WebView 撤去・GPUI 最終採否（全て後続 Issue / Consumer 責務）
- 既存の無関係な docs audit findings の修復

## Upstream Documents

- Issue #127 本文（正本・受入 5 行）
- `docs/specs/issue-127-linux-dist-design.md` / `docs/designs/issue-127-linux-dist-design.md`
- 関連 ADR/spec/design: `issue-384`（GPUI Driver）・`issue-257`（Driver-neutral 契約）・`issue-386`（GPUI host・Linux 依存）・`issue-388`/`issue-389`（Native Dock）・`issue-465`（legacy WebView）・`issue-294`（consumer 非回帰）

## Design Inputs

- design §1–§9 が実装すべき内容の全て。特に §3 capability matrix・§4 形式比較・§5 Owner 境界・§6 WebView 分離・§7 確認手順・§9 AC 写像。
- 判定語彙 `Pass`/`Fail`/`Unsupported`/`NotRun`/`Blocked` と正直性ルール（`NotRun` を `Pass` にしない）。

## File Map

| ファイル | 役割 | 変更種別 |
| --- | --- | --- |
| `docs/specs/issue-127-linux-dist-design.md` | 要件・受入・スコープ | 新規 |
| `docs/designs/issue-127-linux-dist-design.md` | capability/依存/形式/Owner/手順の正本 | 新規 |
| `docs/test-specs/issue-127-linux-dist-design.md` | 文書検証ルール R1–R12 | 新規 |
| `docs/plans/issue-127-linux-dist-design.md` | 本実装計画 | 新規 |
| `docs/tasks/issue-127-linux-dist-design.md` | 作業記録・完了条件 | 新規 |
| `docs/*/README.md` ×5 | 管理索引 | 再生成（doc_maintenance） |

## Implementation Sequence

1. **文書作成**: spec → design → test-spec → plan → task の順に front matter relations を相互リンクして作成（済）。
2. **索引再生成**: `node scripts/doc_maintenance.js apply --type <type> --dir <path>` で 5 ディレクトリの README を再生成。
3. **監査**: `audit_docs` で新規 5 文書の error が 0 であることを確認（既存の無関係 findings は分離して報告）。
4. **フォーマット検証**: `mise run fmt-check` を実行。
5. **main 取込み**: `git fetch origin main && git merge origin/main` で最新化し、必要なら索引再生成・再監査。
6. **review-protocol**: 客観チェック → 独立レビュー pass → 確認済み P0/P1 blocker のみ修正 → blocker なしで承認。
7. **コミット・push・PR**: `feature/127-linux-dist-design` のみ push。日本語 PR（要約/変更点/検証結果/NotRun/review-protocol/`Refs #127`）。
8. **完了報告**: `worker_done` を 1 回送信。

## Dependencies and Risks

- **依存**: `doc_maintenance.js`/`audit_docs`/`mise run fmt-check` の実行環境。既存 docs の broken relation（issue-313/243/410 等）は新規文書に影響しないことを確認する。
- **リスク**: audit が新規文書の front matter を error とする可能性 → relations path の実在確認・ID 形式・status 語彙で対処。既存 findings を誤って「修正」しない。
- **リスク**: `doc_maintenance.js` が README 以外を変更する可能性 → `apply` の whitelisted 対象（managed index 再生成・path-case 正規化）のみであることを diff で確認。

## Verification Matrix

| 検証 | コマンド/方法 | 合格条件 |
| --- | --- | --- |
| 索引再生成 | `doc_maintenance.js apply` ×5 dir | 5 README が新規文書を含む |
| 文書監査 | `audit_docs`（新規文書フィルタ） | 新規文書 error 0 |
| フォーマット | `mise run fmt-check` | pass |
| スコープ | `git diff --name-only origin/main...HEAD` | `docs/` のみ |
| AC 写像 | design §9 vs Issue #127 受入 | 5 行全て対応節あり |
| 判定語彙 | design §Evidence Vocabulary | 5 語彙定義・一貫使用 |
| 分離 | design §6 | WebView が GPUI 必須条件でない・#465 分離・削除済み非主張 |

## Rollback

全変更は新規 docs ファイル + 管理索引再生成のみ。問題があれば当該コミットを revert すればよく、コード・依存への影響はない。

## Task Breakdown

| Task | 内容 | 状態 |
| --- | --- | --- |
| T1 | spec/design/test-spec/plan/task の 5 文書作成 | 完了（本 plan 作成時点） |
| T2 | docs index 再生成 + audit 新規文書 error 0 | 未着手 |
| T3 | `mise run fmt-check` + `git merge origin/main` | 未着手 |
| T4 | review-protocol（客観チェック→独立レビュー→blocker 修正） | 未着手 |
| T5 | commit/push/PR/worker_done | 未着手 |

## Implementation Handoff

本 plan は docs-only のため「実装」は文書作成と索引再生成・検証で完結する。task 文書（`docs/tasks/issue-127-linux-dist-design.md`）が作業記録の正本。

## Review Handoff

承認局面は `review-protocol` に従う: (1) 評価基準を test-spec R1–R12 に固定、(2) 客観チェック（audit/fmt/diff scope）を先に実行、(3) 独立した協調/敵対的レビュー pass を実施、(4) findings を検証し確認済み P0/P1 blocker のみ修正、(5) blocker が残らなければ承認。
