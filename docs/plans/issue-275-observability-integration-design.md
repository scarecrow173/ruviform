---
id: "PLAN-H6keHUx9U3sTwtH2ngWKyQ"
type: "plan"
status: "approved"
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
    - "docs/specs/issue-275-observability-integration-design.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-275-observability-integration-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-275-observability-integration-design.md"
    - "docs/designs/issue-275-observability-integration-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-275-observability-integration-design.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #275 Observability統合設計正本 — 単一クレート・OBS-01〜11・両Consumer 設計 Implementation Plan

## Goal

Issue #275（Observability統合設計正本 — 単一クレート・
OBS-01〜11・両Consumer、2026-11-04 合意版）の合意済み
設計を、単一クレート構成・論理依存 DAG・横断合意事項・
feature 突合・RT/non-RT 境界・Consumer 利用面・証拠項目
対応・#101/#103 前提更新を含む正本設計書として `docs/`
チェーンに文書化する（docs のみ。コード変更なし）。

## Scope

単一実装ストリーム・1 plan・1 task。変更は `docs/` の
`issue-275-*` 新規文書 5 件・前提設計書 2 件への改訂
注記・生成 index のみ。`crates/`・`.github/`・
`mise.toml`・`Cargo.*`・既存他 Issue 文書の本文には
触れない。per-module の実装詳細・Consumer 実測・gate
実走は対象外（実装/受入段階の別 Issue/dispatch）。

## Upstream Documents

- [docs/specs/issue-275-observability-integration-design.md](../specs/issue-275-observability-integration-design.md)（approved）
- Issue #275 本文（Observability統合設計正本 2026-11-04
  合意版 — 受入正本）
- Issue #101（共通基盤・Observability 設計方針）・
  Issue #103（Foundation/Observability 候補の最小設計 —
  本 slice で合意版への改訂注記の対象）

## Design Inputs

- [docs/designs/issue-275-observability-integration-design.md](../designs/issue-275-observability-integration-design.md)（approved）
- 参照正本: `docs/designs/issue-276-…`・`issue-277-…`・
  `issue-278-…`・`issue-279-…`・`issue-280-…`・
  `issue-289-…`・`issue-290-observability-allocation-probe.md`・
  `issue-290-observability-rt-timing.md`・`issue-291-…`・
  `issue-292-…`・`issue-293-…`（per-module 詳細正本）、
  `docs/adr/obs-05-observability-leaf-primitive-deps.md`
  （leaf 依存 ADR）、`docs/designs/issue-294-…`
  （candidate gate）、`docs/specs|designs/issue-219-…`
  （cross-repo 検証）、`crates/ruviform-observability/`
  （実装済み crate — feature matrix・module 配置の根拠）。

## File Map

- `docs/specs/issue-275-observability-integration-design.md`
  — spec（何を・なぜ・範囲・受入条件）。
- `docs/designs/issue-275-observability-integration-design.md`
  — Observability 統合設計正本（単一クレート・DAG・
  横断合意 1〜10・feature 突合・RT/non-RT・Source/
  Generation/Clock/Loss 非偽装・Consumer 利用面・証拠
  項目対応・#101/#103 前提更新・設計/実装分離）。
- `docs/test-specs/issue-275-observability-integration-design.md`
  — 設計文書の正しさを検証する test-spec（R1–R11）。
- `docs/plans/issue-275-observability-integration-design.md`
  — 本計画。
- `docs/tasks/issue-275-observability-integration-design.md`
  — 単一 task（docs チェーン作成の実施記録）。
- `docs/designs/foundation-observability-minimal-api.md`・
  `docs/designs/shared-foundations-and-modularization-policy.md`
  — #101/#103 前提の合意版改訂注記（本文の歴史的分析は
  保持）。
- `docs/{designs,specs,plans,tasks,test-specs}/README.md`
  — 生成 index。

## Implementation Sequence

- [x] Issue #275/#101/#103 本文と関連 Issue 状態
  （#276〜#281・#289〜#294・AL #138・SV #81）を `gh`/
  `git log` で棚卸し（2026-11-09 snapshot）。
- [x] 既存 Observability 文書チェーン（#276〜#293 の
  spec/design/test-spec/plan/task/ADR obs-05）と
  `crates/ruviform-observability` の実装（`src/` module
  配置・`Cargo.toml` feature matrix・`lib.rs` の
  `cfg(feature)` gate）を走査し、参照先の実在を確認。
- [x] spec を作成（Issue #275 AC の dispatch スコープ
  対応付け）。
- [x] design（Observability 統合設計正本）を作成 —
  単一クレート・DAG・横断合意・feature 突合・RT 境界・
  Consumer 利用面・証拠項目対応・前提更新。
- [x] test-spec / plan / task を作成。
- [x] 前提設計書 2 件へ合意版改訂注記を追加。
- [x] docs index を再生成（`doc_maintenance.js apply`）。
- [x] review-protocol によるレビュー（参照実在性・契約
  語彙の整合・証拠項目対応の網羅・前提更新の妥当性を
  確認）。
- [ ] `git fetch origin main && git merge origin/main` で
  最新化し index を再生成・再検証。
- [ ] `mise run fmt-check` + doc-status 系 lint を実行。
- [ ] commit → push → PR 作成（`Refs #275`）。

## Dependencies and Risks

- 参照先の Issue 状態は変動し得る（#276/#277/#279/#280
  CLOSED、#278/#281/#289〜#294 OPEN、AL #138/SV #81
  CLOSED — 2026-11-09 snapshot）。本書は snapshot と
  して記録し、接続表の「役割」記述が将来古くなる可能性
  を test-spec Risk に明記済み。
- Issue #275 本文の「implementation NOT STARTED」は
  登録時点の Status 宣言であり、per-module 実装はその後
  main 上で進行済み。本書は「設計承認 ≠ 実装/検証完了」
  の分離を保ちつつ実装 snapshot を別節で記録する構成と
  した。
- 合意 feature 構想（`collector`/`diagnostics`/
  `debug-tools`）と実装済み feature（`logging`/
  `tracing-export`/`allocation-probe`/`integration` +
  無条件モジュール）の差異は、Issue が「実装時にコンパ
  イル検証」と明記した範囲内の確定値。本書は差異を
  「未実装の不備」ではなく「合意→確定の対応表」として
  記録し、正本を `Cargo.toml` + per-module 設計に置く。
- 既存の broken relation（`docs/impl/ir/issue-243-…`・
  `issue-313-…`・`issue-410-…` 等、`route_graph.js` が
  報告）は本タスク範囲外の既存問題であり、doc-status 全体
  lint がこれを報告する可能性がある — 本 PR では修正せず
  証拠として報告する。
- per-module の詳細（型・variant・閾値・test 規則）は
  #276〜#293 が正本。本書は横断契約と索引のみを所有し、
  記述の drift は参照構造で緩和する。

## Task Breakdown

- T1: spec 作成（本 task に包含 — docs チェーンは 1 単位
  の論理変更）。
- T2: design（Observability 統合設計正本）作成。
- T3: test-spec / plan / task 作成 + 前提 2 文書への改訂
  注記 + index 再生成。
- T4: review-protocol レビューと blocker 解消。
- T5: main 取り込み・最終検証・commit/push/PR。

## Verification

- [ ] `mise run fmt-check` pass。
- [ ] doc-status 系 lint/audit で本 PR 追加分に broken
  relation・未解決参照がないこと（既存の範囲外 finding
  は分離して記録）。
- [ ] 設計文書の参照先（Issue/PR/test-spec/docs パス/
  crate/module/feature 名）が実在することを作成時に確認
  済みであること。
- [ ] PR diff が `docs/` の新規 `issue-275-*` 文書・前提
  2 文書の注記・生成 index のみであることを
  `git status`/`git diff` で確認。

## Open Questions / Gaps

- Consumer 実測・consumer-verify tuple run・実機 RT・
  compile matrix 全組合せは実装/受入段階の要件であり本
  slice では `NotRun`/`not run`/`Unsupported` として記録
  する（解消しない gap ではなく意図的な分離）。
- `collector`/`diagnostics`/`debug-tools` feature の再
  検討は core 使用時のコンパイル時間・依存最小化の実需
  待ち（design Deferred Design Concerns に記録）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology
  are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: no（review-
  protocol で代替）
- If approved: discover available implementation and
  delegation capabilities in the current environment,
  then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the
  same dependency graph and verification matrix.

## Review Handoff

- 単一クレート構成節が「独立サブクレート新設禁止・#101
  内部モジュール先行・論理依存 DAG・標準 tracing のみ・
  leaf 依存 ADR 参照」を漏れなく固定しているか。
- feature 突合表が `Cargo.toml`/`lib.rs` の実測と一致し、
  合意構想→実装確定の差異を「不備」でなく「対応表」と
  して記録しているか。
- 横断合意事項 1〜10 の各項目が per-issue 正本への参照を
  持ち、詳細語彙の複写で drift していないか。
- 証拠項目対応表の G-1〜G-9 が実在する正本（per-issue
  test-spec・#281 G0〜G4・#219/#294・Consumer Issue）
  へ解決できるか。
- Consumer 利用面が AL parity 7 要件と SV 境界
  （Subscriber 所有・認可/Audit 分離・Consumer 所有
  schema）を #281 と矛盾なく記述しているか。
- 前提 2 文書の改訂注記が歴史的本文を改変せず、影響
  変更を記録しているか。
