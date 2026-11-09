---
id: "PLAN-2uLb895SP6hIbAneuh5Pjs"
type: "plan"
status: "approved"
title: "Issue #490 oracle late-create query epoch 実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "docs/specs/issue-490-oracle-addquery-epoch.md"
    - "docs/designs/issue-490-oracle-addquery-epoch.md"
    - "https://github.com/scarecrow173/ruviform/issues/490"
  changes:
    added: []
    modified:
      - type: "file"
        path: "ui/guitest/src/oracle.ts"
      - type: "file"
        path: "ui/guitest/test/oracle.test.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-490-oracle-addquery-epoch.md"
  implemented-by:
    - "docs/tasks/issue-490-t1-oracle-addquery-epoch.md"
  depends-on:
    - "docs/designs/issue-490-oracle-addquery-epoch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-377-query-binding-epoch.md"
    - "docs/plans/issue-378-oracle-late-create-epoch.md"
    - "docs/plans/viewlink-authoritative-generation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-490-oracle-addquery-epoch.md"
    - "docs/designs/issue-490-oracle-addquery-epoch.md"
  derived-by:
    - "docs/tasks/issue-490-t1-oracle-addquery-epoch.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-490-oracle-addquery-epoch.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/377"
    - "https://github.com/scarecrow173/ruviform/issues/378"
    - "https://github.com/scarecrow173/ruviform/issues/490"
  defers: []
  deferred-by: []
---

# Issue #490 oracle late-create query epoch 実装計画 Implementation Plan

## Goal

`ContractModel.addQuery` を修復済み SDK `ViewLink.registerQuery`（Issue #377 / PR #489）と同じ epoch 採用契約へ整合させ、late-create query の恒久 unit test を `oracle.test.ts` へ追加する。

## Scope

単一の実装ストリーム。`ui/guitest` 内の oracle 修正とその直接 unit test のみで完結し、SDK（`ui/binding/`）・replay driver・scenario 語彙・fixture/golden には触れない。分割不要。

## Upstream Documents

- `docs/specs/issue-490-oracle-addquery-epoch.md`（approved）
- `docs/designs/issue-490-oracle-addquery-epoch.md`（approved）
- `docs/specs/issue-377-query-binding-epoch.md` 系チェーン（SDK 側の正本契約。#490 はその oracle 側フォローアップ）
- `docs/specs/issue-378-oracle-late-create-epoch.md` 系チェーン（binding 側の同型先例）

## Design Inputs

- `docs/designs/issue-490-oracle-addquery-epoch.md`
- SDK 側の正本実装: `ui/binding/src/link.ts` `registerQuery`（`if (this.attached_) query.attach(this.generation_)`）
- oracle 側の既存の正しい経路: `ContractModel.attach` の `this.queryModel?.attach(this.linkGeneration)`、および #378 で整合済みの `addBinding`（`setContext` 無条件 → attached なら `attach(linkGeneration)` → `subscribe`）
- テスト移植元: `ui/guitest/test/oracle.test.ts`「ContractModel late-created bindings」3 ケース、`ui/binding/test/link.test.ts`「ViewLink late-created queries」

## File Map

- `ui/guitest/src/oracle.ts` - `ContractModel.addQuery` への epoch 採用追加とヘッダ `QueryBinding` 契約記述の更新。唯一のプロダクションコード変更。
- `ui/guitest/test/oracle.test.ts` - late-create query ケースの追加（既存ファイルへの追記）。
- `docs/specs|designs|plans|test-specs|tasks/issue-490-*.md` - 本チェーンの文書。`docs/*/README.md` index は doc-maintenance で再生成。

## Implementation Sequence

- [ ] `ui/guitest/test/oracle.test.ts` に late-create query 3 ケース（初回 attach 後 / Dock 後 / reattach 後）を追加し、attach 前登録の対照ケースも含める。
- [ ] `mise run test-ui-guitest` で期待どおり RED（late-create query の発行 generation が link epoch を採用せず、live-epoch echo reply が stale 判定）を確認する。
- [ ] `oracle.ts` の `ContractModel.addQuery` に `linkAttached` 時の `attach(linkGeneration)` を追加し、ヘッダの `QueryBinding` 契約記述を更新する。
- [ ] `mise run test-ui-guitest` で GREEN を確認し、`mise run check-ui-guitest`（tsc --noEmit）を通す。
- [ ] `git fetch origin main && git merge origin/main` で最新 main を取り込み、docs index 等の生成物を再生成して再検証する。
- [ ] review-protocol レビュー（blocker なしを確認）→ 論理単位 commit → push → 日本語 PR 作成。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | `addQuery` の adopt 追加が既存 replay の観測 trace を変える | low | med | `runScenario` は `addQuery` → `attach` の順で呼ぶため `linkAttached === false` の分岐は replay では未到達。`test-ui-guitest` 全件で確認。 |
| 2 | `QueryModel.attach(epoch)` の `cancelAll` が意図せず in-flight を cancel する | low | low | `addQuery` 直後は in-flight が空で `cancelAll` は no-op。SDK `registerQuery` の join=epoch 遷移契約とも整合。 |
| 3 | テストが oracle 実装の写しを検証してしまう | low | med | assertion は「発行 generation が link epoch と一致」「live-epoch echo reply が stale でない」という契約条件に限定し、内部手順に依存しない。 |
| 4 | query には subscription がなく binding テストと観測面が異なる | low | low | 発行 rec（`trace.queries[].generation`）と `queryDiagnostics.generation`/`queryOutcomes` を観測面とし、SDK 側テスト（request generation・diagnostics）と対応付ける。 |

Rollback: 変更は `oracle.ts` 1 関数 + test 追記 + docs のみで、revert で完全に戻る。永続データ・公開 API・fixture への影響なし。

## Task Breakdown

- T1（`docs/tasks/issue-490-t1-oracle-addquery-epoch.md`）: oracle.test.ts へ late-create query ケース追加（RED）→ `addQuery` 修復 + ヘッダ更新（GREEN）→ 検証一式。単一タスクに収まる粒度。

## Verification

| Step | Verification command or criteria | Pass condition |
|------|----------------------------------|----------------|
| テスト追加 | `mise run test-ui-guitest` | 新規 late-create query ケースが FAIL（RED）— epoch 未採用・stale 誤判定を示す |
| oracle 修復 | `mise run test-ui-guitest` | 全テスト PASS（GREEN）。既存 scenario replay の trace 一致を維持 |
| 型検査 | `mise run check-ui-guitest` | tsc --noEmit exit 0 |
| フォーマット | `mise run fmt-check` | cargo fmt --check exit 0（Rust 非対象だが共通 gate として実行） |
| main 取込 | `git fetch origin main && git merge origin/main` 後に上記を再実行 | conflict なし・再検証 green |

## Open Questions / Gaps

- なし。対象範囲・受入条件は Issue #490 本文と spec で確定済み。

## Self-Review

- [x] Every spec requirement maps to a task. — spec の In Scope 3 項目（addQuery epoch 採用・ヘッダ更新・unit test 追加）はすべて T1 に写像。Out of Scope（ui/binding・replay/schema・fixture/golden・guards 緩和）は変更対象に含めない。
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一タスク・単一ファイル変更のため delegation 不要）
- Independent work streams: none
- User approval required before dispatch: 本 dispatch では review-protocol による blocker なし確認を承認として扱う
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提: SDK `registerQuery`（#377 / PR #489 merge 済み）の形状が正本。#378 の `addBinding` が同型先例。
- reviewer focus: `addQuery` の adopt が `registerQuery` と同形か、attach 前登録経路が不変か、テストが契約条件（epoch 採用・stale 非誤判定・detach 挙動）を検証しているか。
