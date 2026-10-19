---
id: "TASK-034IRZFUgbTZlBX2kH7psN"
type: "task"
status: "todo"
title: "Issue #1 リポジトリ反映の総合検証と spec/design バックリンク"
created: "2026-10-20"
updated: "2026-10-20"
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
  implements:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
  implemented-by: []
  depends-on:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
    - "apply-publish-false-policy-to-workspace.md"
    - "update-readme-for-issue-1-baseline.md"
    - "update-agents-interim-principles-to-fixed-contracts.md"
    - "align-designs-overview-status-annotations.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #1 リポジトリ反映の総合検証と spec/design バックリンク

## Work

plan `PLAN-034IR3qOr15LGNY9mEindJ` のスライス T-VERIFY（S-1 のベースライン共有＋Phase F-1〜F-8）。全実装スライス完了後に plan の検証一覧 V-1〜V-8 を実行し、spec/design の `derived-by` 双方向メタデータ編集を行う。S-1 のベースラインは pre-dispatch で記録されたものを消費して最終比較を行う（S-1 自体の実行は本タスクの担当範囲外）。実行した検証と未検証条件を区別して実施記録に報告する。

- [ ] F-1. `mise exec -- cargo metadata --format-version 1 --no-deps` で `ruviform-core` の `publish` が `[]` であることを再確認する（V-1）
- [ ] F-2. `mise exec -- prek run --all-files` を実行する（V-2。auto-fix 系フックの修正が出たら取り込んでから再実行）
- [ ] F-3. `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test` を実行する（V-3 の挙動不変スモーク。本変更はビルド入力に影響しないが manifest parse・lint ゲートを確認する）
- [ ] F-4. README/AGENTS の内容監査を行う（V-4・V-5。design A.8 タスク・採否表との突合）
- [ ] F-5. spec `docs/specs/architecture-design-baseline-deliverables.md` と design `docs/designs/ruviform-architecture-design-baseline.md` の front matter `relations.derived-by` に本 plan（`../plans/apply-issue-1-architecture-baseline-to-repo.md`）へのバックリンクを追加し、本 plan の `derives-from`（spec・design）と双方向リンクを一致させる（relation 更新は規約上許容される in-place edit。`derived-by` 追記に伴い両文書の `updated:` は適用日に更新する。本文・status は無変更）
- [ ] F-6. `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` を実行し docs グラフの不整合がないことを確認する（V-6。`node` は環境前提のため導入済みであることを先に確認する）
- [ ] F-7. spec `SPEC-034INYy2aDLxse9dy8MgGZ` の Acceptance Criteria 7 件を突合し、本計画の変更による充足/非退行を記録する（V-7）
- [ ] F-8. `git diff --name-only` と `git diff Cargo.toml crates/ruviform-core/Cargo.toml` で diff スコープを確認する（V-8: 宣言ファイルのみ・edition/rust-version/license 値行不変・mise.toml/ci.yml/Cargo.lock 無変更・`LICENSE` 非作成）し、実行した検証と未検証条件を区別して実施記録に報告する

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/apply-issue-1-architecture-baseline-to-repo.md](../plans/apply-issue-1-architecture-baseline-to-repo.md) — `PLAN-034IR3qOr15LGNY9mEindJ`（approved）。Phase T-VERIFY / V-1〜V-8 の正本
- [../specs/architecture-design-baseline-deliverables.md](../specs/architecture-design-baseline-deliverables.md) — `SPEC-034INYy2aDLxse9dy8MgGZ`（approved）。Acceptance Criteria 7 件の突合対象（V-7）。F-5 で `derived-by` に plan バックリンクを追加（metadata-edit）
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。A.8 差分タスク・採否表の監査対象。F-5 で `derived-by` に plan バックリンクを追加（metadata-edit）

## Dependencies

- 前提タスク（全て完了していること）:
  - `apply-publish-false-policy-to-workspace.md`（`TASK-034IRYXA0oq14nlgmC4Bch`）— V-1 の対象を提供
  - `update-readme-for-issue-1-baseline.md`（`TASK-034IRZk613ql8oL3Y2Yxli`）— V-4・V-5 の監査対象を提供
  - `update-agents-interim-principles-to-fixed-contracts.md`（`TASK-034IRZB7D249qUO1bLldy4`）— V-4・V-5 の監査対象を提供
  - `align-designs-overview-status-annotations.md`（`TASK-034IRZAn8ce6BYjOG6KhrZ`）— V-6 の注記更新を提供
- 共有前提: plan S-1 のベースライン記録（`git grep` ヒット一覧・`cargo metadata` の `publish` 現行値 `null`・overview.md の `(draft)` 注記箇所）を pre-dispatch の記録から引き継ぐ
- `node` は環境前提（F-6 の audit_docs.js 実行に必要。未導入なら先に確認する）

## Affected Files

- `docs/specs/architecture-design-baseline-deliverables.md` — F-5 で `relations.derived-by` に plan バックリンク追加＋`updated:` 更新（metadata-edit。本文・`status: "approved"` 無変更）
- `docs/designs/ruviform-architecture-design-baseline.md` — F-5 で `relations.derived-by`（現行 `[]`）に plan バックリンク追加＋`updated:` 更新（metadata-edit。本文・`status: "approved"` 無変更）
- `Cargo.toml`・`crates/ruviform-core/Cargo.toml`・`README.md`・`AGENTS.md`・`docs/designs/overview.md` — 検査のみ（F-1・F-4・F-8）
- `mise.toml`・`.github/workflows/ci.yml`・`Cargo.lock` — no-change 確認（F-8 で diff が空であること）

## Done When

- [ ] V-1〜V-8 の全検証がパスするか、未検証条件が区別されて実施記録に報告されている
- [ ] spec・design の `derived-by` に plan へのバックリンクが追加され、plan の `derives-from` と双方向整合している（diff は `derived-by` 行と `updated:` 行のみ）
- [ ] `git diff --name-only` が宣言 7 ファイル＋`docs/plans/README.md` に限定され、no-change ファイル（mise.toml・ci.yml・Cargo.lock）の diff が空・`LICENSE` が非作成である
- [ ] spec AC 7 件の充足/非退行が記録されている

## Verification

- [ ] `mise exec -- cargo metadata --format-version 1 --no-deps` で `ruviform-core` の `publish` が `[]` を返す（F-1/V-1）
- [ ] `mise exec -- prek run --all-files` exits 0（F-2/V-2）
- [ ] `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test` が全て exit 0（F-3/V-3）
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` が不整合を報告しない（F-6/V-6）
- [ ] `git diff mise.toml .github/workflows/ci.yml Cargo.lock` が空、`test ! -f LICENSE` exits 0（F-8/V-8）
- [ ] Manual: README/AGENTS の design A.8・採否表との突合（F-4/V-4・V-5）、spec AC 7 件の充足/非退行記録（F-7/V-7）
