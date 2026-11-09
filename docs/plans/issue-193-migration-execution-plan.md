---
id: "PLAN-034UCTG1MAGz2NU32GulcZ"
type: "plan"
status: "approved"
title: "Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/193"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-193-migration-execution-plan.md"
      - type: "doc"
        path: "docs/specs/issue-193-migration-execution-plan.md"
      - type: "doc"
        path: "docs/plans/issue-193-migration-execution-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-193-migration-execution-plan.md"
      - type: "doc"
        path: "docs/test-specs/issue-193-migration-execution-plan.md"
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
    - "docs/specs/issue-193-migration-execution-plan.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-193-migration-execution-plan.md"
    - "docs/specs/issue-193-migration-execution-plan.md"
    - "docs/test-specs/issue-193-migration-execution-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-193-migration-execution-plan.md"
    - "docs/designs/issue-193-migration-execution-plan.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-193-migration-execution-plan.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/192"
  defers: []
  deferred-by: []
---

# Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 計画 Implementation Plan

## Goal

Issue #193（Migration Execution Plan v1.0 — M0〜M7・依存
DAG・受入 Gate・互換移行、実行計画として承認済み）の合意
済み計画を、M0〜M7 実行順・依存 DAG・G0〜G7 受入 Gate・
Cross-Consumer 受入・Rollback RB0〜RB6・GUI lane・WebView
撤去 lane・進捗/Close 条件を含む正本設計書として `docs/`
チェーンに文書化する（docs のみ。コード変更なし）。

## Scope

単一実装ストリーム・1 plan・1 task。変更は `docs/` の
`issue-193-*` 新規文書 5 件と生成 index のみ。
`crates/`・`.github/`・`mise.toml`・`Cargo.*`・既存他
Issue 文書・umbrella 設計文書・Issue #193 本文には触れ
ない。M0〜M7 の実装・gate 受入・Consumer 実測は対象外
（#193・#219/#281/#288/#294・各 Consumer Issue の
Owner 領域）。

## Upstream Documents

- [docs/specs/issue-193-migration-execution-plan.md](../specs/issue-193-migration-execution-plan.md)（approved）
- Issue #193 本文（Migration Execution Plan v1.0・
  M0〜M7・G0〜G7・GUI lane 補足・2026-11-09 版 — 受入
  正本）
- Issue #192（umbrella migration tracking。設計権威は
  umbrella 設計文書、実行権威は #193）
- [docs/designs/synvelis-driven-target-architecture-and-migration.md](../designs/synvelis-driven-target-architecture-and-migration.md)（approved・設計権威）

## Design Inputs

- [docs/designs/issue-193-migration-execution-plan.md](../designs/issue-193-migration-execution-plan.md)（approved）
- 参照正本: umbrella 設計文書、`docs/discovery/
  issue-194-m0-baseline-audit.md`、`docs/specs/
  issue-219-consumer-compat-contract.md`（proposed）、
  `docs/specs/issue-384-driver-neutral-gui-architecture.md`、
  `docs/designs/issue-384-driver-neutral-gui-contract.md`、
  `docs/designs/issue-257-driver-neutral-ui-contract.md`、
  領域別 `docs/*/issue-18x〜19x/38x〜46x-*.md`、ADR
  `synvelis-driven-generic-execution-boundaries`・
  `realtime-contract-and-verification-policy`・
  `plugin-rt-ownership-split`・`configurable-ppqn-…`・
  `media-io-backend-abstraction`・`audio-engine-snapshot-
  swap`・`ui-architecture-webview-hybrid`・
  `driver-neutral-gui-…-issue-384`・
  `issue-465-legacy-webview-retirement-status`。

## File Map

- `docs/specs/issue-193-migration-execution-plan.md`
  — spec（何を・なぜ・範囲・受入条件 R-1〜R-10）。
- `docs/designs/issue-193-migration-execution-plan.md`
  — Execution Plan 設計正本（権威関係・状態分離・
  M0〜M7 表・依存の意味・G0〜G7・Cross-Consumer 受入・
  前提判断 6 項目・RB0〜RB6・E2E-L/R/X・GUI lane・
  WebView 撤去 lane・横断正規化・Signal Flow 境界・
  進捗/Close 条件・traceability）。
- `docs/test-specs/issue-193-migration-execution-plan.md`
  — 設計文書の正しさを検証する test-spec（R1–R12）。
- `docs/plans/issue-193-migration-execution-plan.md`
  — 本計画。
- `docs/tasks/issue-193-migration-execution-plan.md`
  — 単一 task（docs チェーン作成の実施記録）。
- `docs/{designs,specs,plans,tasks,test-specs}/README.md`
  — 生成 index。

## Implementation Sequence

- [x] Issue #193/#192 本文と関連 Issue 状態（#181〜#198・
  #199/#201・#219・#243・#251・#253・#262・#275・#280・
  #282・#288・#294・#308〜#314・#384〜#390・#409・#419・
  #455・#465・Synvelis #12〜#16/#21〜#23/#28/#83/
  #104〜#114/#175〜#177・AudioLens #105/#127/#131/#132/
  #140〜#143/#166）を `gh`/`git log` で棚卸し
  （2026-11-09 snapshot）。
- [x] 既存正本（umbrella 設計・#194 discovery・#219 spec・
  #257/#384 設計文書・領域別 issue-* 文書・ADR 各件）
  の実在と status を確認。
- [x] spec を作成（Issue #193 AC の dispatch スコープ
  対応付け）。
- [x] design（Execution Plan 設計正本）を作成 — 権威
  関係・状態分離・M0〜M7・G0〜G7・Cross-Consumer・
  RB0〜RB6・GUI/WebView lane・進捗/Close 条件。
- [x] test-spec / plan / task を作成。
- [ ] docs index を再生成（`doc_maintenance.js apply`）。
- [ ] review-protocol によるレビュー（参照実在性・権威
  関係・受入条件対応・状態分離の整合を確認）。
- [ ] `git fetch origin main && git merge origin/main`
  で最新化し index を再生成・再検証。
- [ ] `mise run fmt-check` + doc-status 系 lint を実行。
- [ ] commit → push → PR 作成（`Refs #193`）。

## Dependencies and Risks

- 参照先の Issue 状態は変動し得る（2026-11-09 snapshot）。
  本書は snapshot として記録し、対応表の「役割」記述が
  将来古くなる可能性を test-spec Risk に明記済み。
- `docs/specs/issue-219-consumer-compat-contract.md` は
  status `proposed` — 影響分類語彙の参照正本として
  `depends-on` で接続するが、本 slice の承認対象では
  ない。
- umbrella 設計文書は 2026-11-01 時点の approved で
  あり、その後の GUI lane 追加・ADR supersede を本文に
  反映していない。本書はそれらを「現状 snapshot」と
  して記録し、umbrella 本文の改訂は行わない（別 Issue
  の判断）。
- 別 Worker が並行作業中の #192 設計チェーン
  （`issue-192-*` 文書、未 merge）は参照しない。
  merge 後に相互参照を追加するかは別途判断。
- 既存の broken relation（`docs/impl/ir/issue-243-…`・
  `issue-313-…`・`issue-410-…` 等）は本タスク範囲外の
  既存問題であり、doc-status 全体 lint がこれを報告する
  可能性がある — 本 PR では修正せず証拠として報告する。
- #193 は進行中の tracking Issue であり、GUI lane・
  consumer 別受入表が追記され続ける。本書の記述は
  2026-11-09 時点の版に基づく。

## Task Breakdown

- T1: spec 作成（本 task に包含 — docs チェーンは 1 単位
  の論理変更）。
- T2: design（Execution Plan 設計正本）作成。
- T3: test-spec / plan / task 作成 + index 再生成。
- T4: review-protocol レビューと blocker 解消。
- T5: main 取り込み・最終検証・commit/push/PR。

## Verification

- [ ] `mise run fmt-check` pass。
- [ ] doc-status 系 lint/audit で本 PR 追加分に broken
  relation・未解決参照がないこと（既存の範囲外 finding
  は分離して記録）。
- [ ] 設計文書の参照先（Issue/PR/test-spec/mise
  task/crate/doc パス）が実在することを作成時に確認済
  みであること。
- [ ] PR diff が `docs/` の新規 `issue-193-*` 文書と
  生成 index のみであることを `git status`/`git diff`
  で確認。

## Open Questions / Gaps

- G0〜G7 の正式 Close 判定・#193 自体の Close は gate
  証拠の棚卸し時の Owner 判断であり本 slice では扱わ
  ない（Deferred Design Concerns に記録）。
- umbrella 設計文書本文への GUI lane/ADR 変更の反映は
  本 slice の対象外（同文書は approved の設計権威とし
  て維持。改訂要否は別 Issue/Owner 判断）。
- #192 設計チェーン merge 後の相互参照追加は別途判断
  （本 slice では未 merge 文書を参照しない）。
- Consumer 実測・実機受入は実装/受入段階の要件であり
  本 slice では `not run`/`NotRun`/`Unsupported` として
  記録する（解消しない gap ではなく意図的な分離）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and
  terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: no
  （review-protocol で代替）
- If approved: discover available implementation and
  delegation capabilities in the current environment,
  then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the
  same dependency graph and verification matrix.

## Review Handoff

- Context 節が「umbrella 設計文書 = 設計権威、#193 本文
  = 実行権威、本書 = canonical 記録」の 3 層を relations
  と本文の両面で固定しているか。
- M0〜M7 表の対応先 Issue が Issue #193 本文と 1:1 で
  一致し、優先度（P0/P1/P2）と開始/完了境界が保持され
  ているか。
- G0〜G7 表の判定条件と証拠リンク必須要件、Cross-
  Consumer Gate 別証拠表（Synvelis/AudioLens 両列）が
  本文どおりか。
- GUI lane の Start/Integration/Acceptance 3 区分と
  「#262 採用完了への循環禁止」「暗黙依存の新設禁止」
  「Owner 分界・二重実装/CI 重複禁止」が受入条件どおり
  記録されているか。
- WebView 撤去 lane が #465 Owner の独立 lane として
  3 区分で記録され、Closed 履歴保持対象が列挙されてい
  るか。
- RB0〜RB6・前提判断 6 項目・E2E-L/R/X・進捗/Close 条件
  が本文の要点を保持しているか。
- 状態分離（Design Approved〜Migration Accepted）と
  「未検証を合格に昇格させない」原則が明記され、
  snapshot 日付（2026-11-09/2026-11-04）が付いているか。
- 参照が merge 済み正本のみで、未 merge の `issue-192-*`
  文書を参照していないか。
