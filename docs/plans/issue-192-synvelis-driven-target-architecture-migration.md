---
id: "PLAN-034UCajy65hNl8zISc6rvu"
type: "plan"
status: "approved"
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
    - "docs/specs/issue-192-synvelis-driven-target-architecture-migration.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-192-synvelis-driven-target-architecture-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-192-synvelis-driven-target-architecture-migration.md"
    - "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-192-synvelis-driven-target-architecture-migration.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #192 Synvelis-driven Target Architecture & Migration — umbrella migration 設計正本 Implementation Plan

## Goal

Issue #192（Synvelis-driven Ruviform Target v0.7.1
umbrella migration tracking Issue）の合意済み設計を、
TA-001〜TA-012 gate 対応・capability→正本 map・3 者
Ownership boundary・G0〜G7/E2E/GUI lane 構造・ADR/互換
policy・Consumer 接続面・設計/実装分離を含む tracking
canonical 設計書として `docs/` チェーンに文書化する
（docs のみ。コード変更なし）。

## Scope

単一実装ストリーム・1 plan・1 task。変更は `docs/` の
`issue-192-*` 新規文書 5 件と生成 index のみ。
`crates/`・`.github/`・`mise.toml`・`Cargo.*`・既存他
Issue 文書・umbrella 設計文書・#193 には触れない。
M0〜M7 の実装・gate 受入・Consumer 実測は対象外
（#193・#219/#281/#288/#294・各 Consumer Issue の
Owner 領域）。

## Upstream Documents

- [docs/specs/issue-192-synvelis-driven-target-architecture-migration.md](../specs/issue-192-synvelis-driven-target-architecture-migration.md)（approved）
- Issue #192 本文（umbrella 設計・TA-001〜TA-012・
  2026-11-02 両 Consumer 対等原則追補 — 受入正本）
- Issue #193（M0〜M7 実行計画・G0〜G7・GUI lane 補足）
- [docs/designs/synvelis-driven-target-architecture-and-migration.md](../designs/synvelis-driven-target-architecture-and-migration.md)（approved・設計権威）

## Design Inputs

- [docs/designs/issue-192-synvelis-driven-target-architecture-migration.md](../designs/issue-192-synvelis-driven-target-architecture-migration.md)（approved）
- 参照正本: umbrella 設計文書、`docs/discovery/
  issue-194-m0-baseline-audit.md`、`docs/specs/
  issue-219-consumer-compat-contract.md`（proposed）、
  領域別 `docs/*/issue-18x〜19x/28x〜30x/38x〜46x-*.md`、
  ADR `synvelis-driven-generic-execution-boundaries`・
  `configurable-ppqn-…`・`realtime-contract-…`・
  `plugin-rt-ownership-split`・`media-io-backend-
  abstraction`・`audio-engine-snapshot-swap`・
  `musical-time-model`・`ui-architecture-webview-
  hybrid`・`driver-neutral-gui-…-issue-384`。

## File Map

- `docs/specs/issue-192-synvelis-driven-target-architecture-migration.md`
  — spec（何を・なぜ・範囲・受入条件）。
- `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
  — umbrella migration tracking canonical（権威関係・
  3 者 boundary・capability→正本 map・TA-001〜TA-012
  対応・G0〜G7/E2E/GUI lane・ADR/互換・Consumer 接続面・
  設計/実装分離）。
- `docs/test-specs/issue-192-synvelis-driven-target-architecture-migration.md`
  — 設計文書の正しさを検証する test-spec（R1–R10）。
- `docs/plans/issue-192-synvelis-driven-target-architecture-migration.md`
  — 本計画。
- `docs/tasks/issue-192-synvelis-driven-target-architecture-migration.md`
  — 単一 task（docs チェーン作成の実施記録）。
- `docs/{designs,specs,plans,tasks,test-specs}/README.md`
  — 生成 index。

## Implementation Sequence

- [x] Issue #192/#193 本文と関連 Issue 状態（#181〜#198・
  #219・#243・#251・#253・#262・#275〜#306・#384〜#390・
  #409・#464・#465・Synvelis #17〜#28/#82〜#84/#106/#
  109/#175〜#177・AudioLens #95/#127/#139〜#143/#166）を
  `gh`/`git log` で棚卸し（2026-11-09 snapshot）。
- [x] 既存正本（umbrella 設計・#194 discovery・#219
  spec・領域別 issue-* 文書・ADR 各件）の実在と status
  を確認。
- [x] spec を作成（Issue #192 AC の dispatch スコープ
  対応付け）。
- [x] design（umbrella migration tracking canonical）を
  作成 — 権威関係・3 者 boundary・capability map・
  TA-001〜TA-012 対応・G0〜G7/E2E/GUI lane・ADR/互換・
  Consumer 接続面・設計/実装分離。
- [x] test-spec / plan / task を作成。
- [x] docs index を再生成（`doc_maintenance.js apply`）。
- [x] review-protocol によるレビュー（参照実在性・権威
  関係・ADR status・TA 網羅・consumer 分界の整合を
  確認）。
- [ ] `git fetch origin main && git merge origin/main`
  で最新化し index を再生成・再検証。
- [ ] `mise run fmt-check` + doc-status 系 lint を実行。
- [ ] commit → push → PR 作成（`Refs #192`）。

## Dependencies and Risks

- 参照先の Issue 状態は変動し得る（2026-11-09
  snapshot）。本書は snapshot として記録し、対応表の
  「役割」記述が将来古くなる可能性を test-spec Risk に
  明記済み。
- `docs/specs/issue-219-consumer-compat-contract.md` は
  status `proposed` — 影響分類語彙の参照正本として
  `depends-on` で接続するが、本 slice の承認対象では
  ない。
- umbrella 設計文書は 2026-11-01 時点の approved で
  あり、その後の GUI lane 追加・ADR supersede を本文に
  反映していない。本書はそれらを「現状 snapshot」と
  して記録し、umbrella 本文の改訂は行わない（別 Issue
  の判断）。
- 既存の broken relation（`docs/impl/ir/issue-243-…`・
  `issue-313-…`・`issue-410-…` 等、`route_graph.js` が
  報告）は本タスク範囲外の既存問題であり、doc-status
  全体 lint がこれを報告する可能性がある — 本 PR では
  修正せず証拠として報告する。
- #193 は進行中の実行計画であり、GUI lane・consumer
  別受入表が追記され続ける。本書の G/E2E 構造記述は
  2026-11-09 時点の版に基づく。

## Task Breakdown

- T1: spec 作成（本 task に包含 — docs チェーンは 1 単位
  の論理変更）。
- T2: design（umbrella migration tracking canonical）
  作成。
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
- [ ] PR diff が `docs/` の新規 `issue-192-*` 文書と
  生成 index のみであることを `git status`/`git diff`
  で確認。

## Open Questions / Gaps

- TA gate の正式 Close 判定・#192 自体の Close は gate
  証拠の棚卸し時の Owner 判断であり本 slice では扱わ
  ない（Deferred Design Concerns に記録）。
- umbrella 設計文書本文への GUI lane/ADR 変更の反映は
  本 slice の対象外（同文書は approved の設計権威とし
  て維持。改訂要否は別 Issue/Owner 判断）。
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

- Context 節が「umbrella 設計文書 + #193 = 設計/実行の
  権威、本書 = tracking canonical」を relations と本文
  の両面で固定しているか。
- TA-001〜TA-012 表の全項目が umbrella Issue #192 本文
  の受入条件と 1:1 で対応し、証拠レーンと現状が分離
  されているか。
- capability map の merge 証拠が #193 進捗表または
  git log の merge commit に基づき、未検証の merge を
  主張していないか。
- ADR 表の status が各 ADR front matter の実値
  （accepted/proposed/superseded）と一致するか
  （特に webview-hybrid superseded・#384 proposed・
  musical-time-model superseded）。
- 3 者 Ownership 表が AudioLens の f64/PTS/Dataset/
  GUI-CLI 分離を Synvelis と同列に記述し、consumer
  接続面が schema/revision/precision の製品所有を
  明記しているか。
- 「設計合意と実装/検証の分離」と未実施項目の
  `NotRun` 記録方針が明記されているか。
