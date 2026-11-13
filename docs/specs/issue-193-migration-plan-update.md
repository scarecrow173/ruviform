---
id: "SPEC-034WZ7XezzT5NZf0iyG6Wn"
type: "spec"
status: "draft"
title: "Issue #193 Migration Plan Update — 版付き Snapshot・M0〜M7 Owner/Gate 確定 仕様"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/193"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-193-migration-execution-plan.md"
    - "docs/designs/issue-193-migration-execution-plan.md"
    - "docs/plans/issue-193-migration-execution-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-193-migration-execution-plan.md"
    - "docs/plans/issue-193-migration-execution-plan.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "docs/specs/issue-193-migration-execution-plan.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/558"
    - "https://github.com/scarecrow173/ruviform/issues/559"
    - "https://github.com/scarecrow173/ruviform/issues/560"
    - "https://github.com/scarecrow173/ruviform/issues/562"
    - "https://github.com/scarecrow173/synvelis/issues/23"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "https://github.com/scarecrow173/synvelis/issues/277"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  defers: []
  deferred-by: []
---

# Issue #193 Migration Plan Update — 版付き Snapshot・M0〜M7 Owner/Gate 確定 仕様

## What Should Be Built

Issue #193（Migration Execution Plan v1.0）の残件を解消する
**版付き Snapshot** を、計画正本
`docs/designs/issue-193-migration-execution-plan.md` と
`docs/plans/issue-193-migration-execution-plan.md` へ追記する。
Snapshot は M0〜M7 の各 Milestone について、開始条件・
着手可能な Headless Slice・完了定義・Integration Gate・
責務・Blocked by・独立 Owner Issue を Issue URL 付きで
確定し、現行の GPUI Native First（#262/#388/#419）・
IO（#288/#306）・OBS（#275/#281）・Cross-Consumer
（#219/#294）・Script（#558〜#560）・Creative Branch
（Synvelis #277〜#296・Ruviform #562）の最新 Issue 状態と
照合して DAG 矛盾を解消した記録である。

あわせて、本更新自体の spec → design → task → test-spec →
ir/exp の文書チェーン（`issue-193-migration-plan-update*`）
を `docs/` に作成し、Issue 状態照合の証拠を
`evidence/rf193/` に記録する。

## Why Now

- Issue #193 本文の 2026-11-13 決定（Stage 1〜3）により、
  計画正本は PR #526 で統合済みだが、「各 Milestone の
  着手条件・Headless Slice・Owner の確定」と「現行
  GPUI/IO/OBS/Script/Creative Branch 新要件との DAG 矛盾
  解消」が未チェックのまま残っている。
- 2026-11-09 時点の計画 snapshot 以降、M0〜M7 の実装
  Issue（#184〜#198）・M1 着手 Gate（#199/#201）・IO/OBS
  の各 Gate・GUI Test 基盤・3OS OS-L1 CI（#598）が Closed
  になり、Script・Creative Branch・Future capability の
  新規 Issue 群が追加された。計画正本の Issue 状態記述を
  現状へ同期しないと、Closed 済み項目を未着手扱いする・
  受入 Gate を着手 Blocker と誤記する等の DAG 矛盾が残る。
- POLICIES.md（2026-11-13）の OS-L1/GUI-L2/EXT-L3 と
  Required/Deferred/NotApplicable・Pass/Fail/NotRun/
  Unsupported の分離を、計画 Snapshot へ明示的に接続する
  必要がある。

## Users and Value

- 後続の実装・受入 Worker と Coordinator: Milestone ごとの
  着手可否・並行可能 Slice・統合/受入 Owner を Issue URL
  付きで即座に参照でき、Blocked by の誤読による不要な
  一括停止や循環依存を防げる。
- 両 Consumer（Synvelis・AudioLens）の追従 Owner: 別
  Oracle（AL f64/長尺/CLI、SV f32/DAW/RT）と候補 SHA
  tuple の接続先が M0〜M7 ごとに明確になる。
- Issue #193 の進行管理: 新 Snapshot が状態の正本となり、
  実装・実 OS・Consumer 受入は各 Owner Issue の責務と
  して分離される。

## Scope

### In Scope

- 計画正本 2 文書への版付き Snapshot 節の追記
  （M0〜M7 の開始条件/Headless Slice/完了定義/
  Integration Gate/責務/Blocked by/独立 Owner Issue、
  DAG/Owner 表、Deferred/Activation 記録、Issue 状態
  照合の検証記録、AL/SV 別 Oracle・候補 SHA・既存
  Daily/手動 CI・OS 別実走証拠の M0〜M7 受入への接続）。
- `issue-193-migration-plan-update*` 文書チェーン
  （spec/design/task/test-spec/ir/exp）の作成。
- `evidence/rf193/` への Issue 状態 census 証拠の記録。
- 生成 docs index の再生成（生成可能な dir は
  `doc-maintenance`、`docs/impl/ir`・`docs/impl/exp` は
  手動追記）。

### Out of Scope

- M0〜M7 の実装・PoC・CI 変更・Consumer repo の変更
  （各 Owner Issue の責務）。
- 実 OS・実 Device・実 Plugin・実 GUI の受入実行
  （L2/L3 は Deferred/Activation として記録するのみ）。
- Issue 本文・計画正本本文の M0〜M7/G0〜G7/RB0〜RB6 の
  定義変更、GitHub 正式依存（blockedBy）の登録・削除
  （Coordinator 責務）。
- `crates/`・`mise.toml`・`.github/workflows/`・
  `Cargo.toml`/`Cargo.lock`・`docs/discovery/`・
  `docs/designs/issue-192-*`・`docs/plans/issue-192-*`
  等の編集範囲外ファイル。
- Issue #193・各 Gate Issue の Close 判定。

## Requirements

- R-1: Snapshot は版識別子（版番号・基準日・基準
  Issue/文書版）を持ち、追記型で既存記述を改変しない。
- R-2: M0〜M7 の各 Milestone に、開始条件・着手可能
  Headless Slice・完了定義・Integration Gate・責務・
  Blocked by・独立 Owner Issue（Issue URL 付き）を
  対応付ける。
- R-3: 各 Milestone の OS-L1 必須範囲（Windows/macOS/
  Linux の Native Build/Link/Test・必要 Feature）と、
  統合 Owner を明示する。L2/L3 が必要な製品利用条件は
  Deferred/Activation として記録し、現行計画 Snapshot
  の完了と将来の実機受入を分離する。
- R-4: GPUI Native First（#262/#388/#419）・IO
  （#288/#306）・OBS（#275/#281）・Cross-Consumer
  （#219/#294）・Script（#558〜#560）・Creative Branch
  （SV #277〜#296・RF #562）および AL/SV Issue の最新
  状態を `gh` で照合し、Closed/Deferred/NotRun を実装
  Blocked by として誤記していないことを検証記録する。
- R-5: AL f64/長尺/CLI と SV f32/DAW/RT の別 Oracle、
  候補 SHA tuple、既存 Daily/手動 CI、OS 別実走証拠の
  接続先を M0〜M7 受入へ対応付ける。片方の Consumer
  Pass を他方へ流用しない。
- R-6: DAG と Owner の表を実装順序の正本として維持し、
  両 Consumer の最新 Issue へリンクする。実 Device/OS
  Native Dock/両 Consumer 実測の未完了は対応 Open Gate
  Issue（#251/#253/#388/#313/#281/#294 等）へ具体的に
  接続する。
- R-7: 本スライスは文書のみ。コード・テスト・CI・依存
  設定・Issue 本文・GitHub 依存関係を変更しない。

## Acceptance Criteria

- [ ] 計画正本 2 文書に、版付き Snapshot 節（R-1〜R-3,
  R-5〜R-6 の内容を含む）が追記されている。
- [ ] M0〜M7 全 Milestone の Snapshot 表に R-2 の 7 項目
  が Issue URL 付きで揃っている。
- [ ] Issue 状態照合の検証記録（`gh` 実測に基づく
  Open/Closed/Deferred 分類と Blocked by 誤記なしの
  確認）が Snapshot と `evidence/rf193/` に存在する
  （R-4）。
- [ ] AL/SV 別 Oracle・候補 SHA・既存 CI・OS 別実走証拠
  の M0〜M7 受入への接続が記録されている（R-5）。
- [ ] L2/L3 の製品利用条件が Deferred/Activation として
  記録され、計画 Snapshot 完了と将来実機受入が分離
  されている（R-3）。
- [ ] `issue-193-migration-plan-update*` 文書チェーンと
  impl 記録が作成され、docs index が整合している。
- [ ] 差分が編集許可範囲内の文書・証拠・生成 index のみ
  であることを `git status`/`git diff` で確認する。

## Deferred Design Concerns

| Concern | Reason | Re-engagement trigger | Risk if ignored |
| --- | --- | --- | --- |
| 実装・実 OS・Consumer 受入の実行 | 各 Owner Issue（#219/#251/#253/#262/#281/#294/#306/#313/#388/#389/#419/#465/#532・SV/AL 各 Issue）の責務 | 各 Issue の実行開始指示 | Snapshot を受入完了と誤認 |
| GitHub 正式 blockedBy の登録・削除 | Coordinator 責務。本タスクは本文記述の整合のみ | Issue 本文と GitHub 依存の齟齬検出時 | 文書と GitHub 依存の二重管理齟齬 |
| L2/L3 実機受入（Windows 実 GUI/実 Device/実 Plugin） | 利用要件発生時に Required へ昇格（POLICIES §2） | 製品利用条件の確定 | 未実測を Pass 扱い |
| macOS/Linux の GUI-L2/EXT-L3 | 利用要件発生まで Deferred | macOS/Linux での GUI/実機利用決定 | 設計対象外と誤認 |
| Future capability（RF #315〜#318・SV #119/#124/#130/#138） | 各 Issue の着手 Trigger 未達・Deferred/NotStarted | Consumer 側の具体 Scenario 提出 | 将来要件を現行 Gate 化 |

## Sources and Relations

- [Issue #193](https://github.com/scarecrow173/ruviform/issues/193)
  本文（2026-11-13 Stage 1〜3 決定・現行受入方針を含む
  進行中の計画正本）。
- [POLICIES.md](../../POLICIES.md) — OS-L1/GUI-L2/EXT-L3、
  Required/Deferred/NotApplicable、Pass/Fail/NotRun/
  Unsupported、着手必須依存と統合・受入依存の分離。
- 正本計画文書:
  `docs/specs|designs|plans|tasks|test-specs/
  issue-193-migration-execution-plan.md`。
- 証拠: `evidence/rf193/issue-state-census-2026-11-14.md`。
- 後続文書: `docs/designs/issue-193-migration-plan-update-design.md`、
  `docs/tasks/issue-193-migration-plan-update-task.md`、
  `docs/test-specs/issue-193-migration-plan-update.md`、
  `docs/impl/ir|exp/issue-193-migration-plan-update.md`。
