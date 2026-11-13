---
id: "DESIGN-034WZ7Y0r628xfq8Nhga00"
type: "design"
status: "draft"
title: "Issue #193 Migration Plan Update — 版付き Snapshot 設計"
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
  implements:
    - "docs/specs/issue-193-migration-plan-update.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-193-migration-plan-update.md"
    - "docs/designs/issue-193-migration-execution-plan.md"
    - "docs/plans/issue-193-migration-execution-plan.md"
    - "docs/specs/issue-193-migration-execution-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-193-migration-execution-plan.md"
    - "docs/plans/issue-193-migration-execution-plan.md"
    - "docs/test-specs/issue-193-migration-execution-plan.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-193-migration-plan-update.md"
    - "docs/specs/issue-193-migration-execution-plan.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-193-migration-plan-update.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/558"
    - "https://github.com/scarecrow173/ruviform/issues/562"
    - "https://github.com/scarecrow173/synvelis/issues/23"
    - "https://github.com/scarecrow173/synvelis/issues/277"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "evidence/rf193/issue-state-census-2026-11-14.md"
  defers: []
  deferred-by: []
---

# Issue #193 Migration Plan Update — 版付き Snapshot 設計

## Context

計画正本
[docs/designs/issue-193-migration-execution-plan.md](issue-193-migration-execution-plan.md)
（以下「計画設計正本」）と
[docs/plans/issue-193-migration-execution-plan.md](../plans/issue-193-migration-execution-plan.md)
（以下「計画実行正本」）は、Issue #193 本文の
Migration Execution Plan v1.0 を 2026-11-09 時点の
snapshot として正本化したものである。その後の Issue 状態
変化（M0〜M7 実装 Issue の Close、IO/OBS/GUI Gate の
進捗、Script・Creative Branch・Future capability の新規
Issue 追加、2026-11-13 の OS-L1/GUI-L2/EXT-L3 方針決定）
に対し、Issue #193 の残件は「版付き Snapshot の作成」
である。

**権威関係（本更新でも固定）:**

- 設計権威: umbrella 設計文書
  `docs/designs/synvelis-driven-target-architecture-and-migration.md`
  （approved）。
- 実行権威: Issue #193 本文（進行中の tracking 正本）。
- 計画正本: 計画設計正本・計画実行正本。本 Snapshot 節は
  両者へ**追記**され、M0〜M7/G0〜G7/RB0〜RB6 の定義を
  変更しない。Snapshot は「2026-11-14 時点の検証済み
  状態記録」であり、計画の再定義ではない。
- 本更新の spec/design/task/test-spec/ir/exp
  （`issue-193-migration-plan-update*`）は本更新スライス
  自体の文書チェーンであり、計画正本の一部ではない。

**状態語彙（POLICIES.md 準拠）:** 要求状態
`Required`/`Deferred`/`NotApplicable` と実行結果
`Pass`/`Fail`/`NotRun`/`Unsupported` は別軸。Issue の
open/closed 状態は実装 Blocked by と同義ではない。
OS-L1（Native Build/Link/Headless/RT/CLI/必要 Feature・
Consumer 独立 Oracle）は 3OS で Required。GUI-L2/
EXT-L3 は Windows で実際に提供・利用する機能のみ
Required、macOS/Linux は利用要件発生まで Deferred。

## Scope

### In Scope

- 版付き Snapshot の構造・版識別・記録規則の設計。
- M0〜M7 各 Milestone の Snapshot 項目（開始条件・
  着手可能 Headless Slice・完了定義・Integration Gate・
  責務・Blocked by・独立 Owner Issue）の確定値。
- Issue 状態照合の方法と検証記録の形式。
- DAG 矛盾の解消規則（Blocked by と統合・受入の分離、
  Closed/Deferred/NotRun の誤記防止、新規 Lane の接続）。
- Deferred/Activation 記録の対象と接続先。
- AL/SV 別 Oracle・候補 SHA・既存 CI・OS 別実走証拠の
  M0〜M7 受入への接続設計。

### Out of Scope

- M0〜M7 の実装・gate 受入・Consumer 実測（各 Owner
  Issue の責務）。
- GitHub 正式依存（blockedBy）の登録・削除
  （Coordinator 責務。齟齬は PR 本文へ列挙）。
- 計画正本本文の M0〜M7/G0〜G7/RB0〜RB6 定義の改訂、
  Issue #193 本文の改訂。
- コード・テスト・CI・依存設定の変更（docs のみ）。

## Components and Boundaries — Snapshot の責務と権威の分界

### Snapshot（本設計の成果物）が所有するもの

- M0〜M7 各 Milestone の「いま着手できるか」「何が
  残っているか」「どの Issue が Owner か」を Issue URL
  付きで示す版付き状態表。
- 現行要件（GPUI/IO/OBS/Script/Creative Branch/Future）
  と計画 DAG の整合確認結果と、その解消規則。
- L2/L3 を要する製品利用条件の Deferred/Activation
  記録と、Open Gate Issue への接続。

### Snapshot が所有しないもの

- 各 Owner Issue の実装内容・PoC 判断・テスト
  （各 Issue とその docs チェーン）。
- crate 配置・公開 API・RT 契約（umbrella 設計文書と
  領域別設計正本）。
- Consumer 側の schema/revision/precision・DAW/解析
  意味論（Synvelis/AudioLens 各 repo）。
- gate の正式 Pass/Fail 判定（証拠は各 Issue/CI Run に
  記録され、Snapshot は接続先を示すのみ）。

## Snapshot の版識別と記録規則

- **版識別子**: `Snapshot-2026-11-14`（基準日版）。
  Issue #193 本文の版ではなく、Issue 状態を `gh` で
  実測した日付を版とする。計画正本の「2026-11-09
  snapshot」記述は履歴として保持し、本節を最新の
  版付き状態記録とする。
- **追記型**: 既存節・表・受入条件は改変しない。
  Snapshot 節は文書末尾近辺の独立節として追加し、
  front matter の `updated` のみ更新する。
- **証拠**: Issue 状態 census
  （`evidence/rf193/issue-state-census-2026-11-14.md`）
  を根拠として参照する。`gh issue list/view` は補助
  証拠であり、canonical な検証入口は `mise.toml` タスク
  （本タスクは文書のみのため `fmt-check` + `prek`）。
- **状態の分離**: Snapshot は「計画 Snapshot の完了」
  （本タスクの Done）と「将来の実機受入」（Deferred/
  Activation 先の各 Issue）を別物として記録する。
  NotRun/Fail/Unsupported を Pass に昇格させない。

## M0〜M7 Snapshot 項目の確定値（設計）

各 Milestone について以下の 7 項目を確定する。値の根拠は
計画正本の M0〜M7 表・G0〜G7・GUI lane・横断正規化・
実装ステータス同期、および 2026-11-14 の Issue 状態
census である。

| Milestone | Owner Issue（実装・完了済み） | 残存 Owner/Gate Issue（Open） |
| --- | --- | --- |
| M0 Baseline | #194（CLOSED） | #219/#294（継続分類・tuple 判定）、AL #127/SV #28 |
| M1 Graph | #184（CLOSED、PR #225） | SV #106（Consumer lowering）、#294 |
| M2 Audio Runtime | #195（CLOSED、PR #227） | #294、SV #28/AL #127（RT/精度 Oracle） |
| M3 Time/Event | #189/#190/#188/#186（CLOSED） | #253（OS MIDI/VST3）、SV #126/#127 |
| M4 Plugin | #196（CLOSED、PR #232） | #388/#389/#532、SV #176/#127、#294 |
| M5 Device/Reader | #187/#197（CLOSED） | #251、SV #115/#117 |
| M6 Output/Offline | #191/#198（CLOSED） | #306、SV #28、AL #127 |
| M7 UI/Probe | #181/#182/#185（CLOSED） | #243/#262/#313/#388/#389/#419/#465/#532、SV #112/#114/#176/#177、AL #127/#189、#219/#294 |

（詳細な 7 項目の値は計画正本 2 文書の Snapshot 節を正本
とする。本設計書は項目の意味と確定規則を定義する。）

### 項目の定義

- **開始条件**: 当該 Milestone の残存作業を着手するための
  Hard Blocker。2026-11-13 のゲート正規化により、確認
  した全 Open Issue で `着手条件（Blocked by）` は
  「なし」。実 OS GUI/Device/Plugin Gate は
  `統合・受入で関係する Issue（着手は止めない）` に
  分離済みであり、開始条件へ転記しない。
- **着手可能 Headless Slice**: 実機・実 OS を要さない
  Headless/Fake/CI レーンで現在着手・継続可能な作業。
  OS-L1（3OS Native Build/Link/Headless/RT/CLI/必要
  Feature）は既存 Daily/手動 CI で対象 SHA を実走する
  範囲であり、新規 Workflow は増設しない。
- **完了定義**: 対応する G Gate（G0〜G7）の判定条件と、
  両 Consumer 別証拠（impacted/no impact/unknown +
  pass/fail/not run）または no-impact 根拠。
- **Integration Gate**: 機能接続・受入のための Open
  Issue（実 OS/実機を含む）。着手は止めない。
- **責務**: Ruviform 共通契約の Owner と、Synvelis/
  AudioLens 固有意味論の Owner の分界。
- **Blocked by**: Issue 本文の `着手条件（Blocked by）`
  の転記。空なら「なし」。GitHub 正式 blockedBy との
  齟齬は Coordinator へ報告し、本 Snapshot では変更
  しない。
- **独立 Owner Issue**: 当該 Milestone の残存作業を
  所有する Open Issue（URL 付き）。Closed Issue は
  実装済み証拠としてのみ参照する。

## DAG 矛盾の解消規則

1. **Blocked by と統合・受入の分離**: POLICIES.md §4
   および 2026-11-13 ゲート正規化どおり、実 OS
   GUI/Device/Plugin Gate（#262/#388/#419・#288/#306・
   #275/#281・#219/#294・AL/SV Issue）を一律の実装
   Blocked by にしない。必要な L1 Fail/未実装は残す
   （例: #306 の OS/FS/fault 未検証行、#313 の OS
   Capability 未実走行）。
2. **Closed/Deferred/NotRun の誤記防止**: census で
   Closed を確認した項目（#288・#310〜#312・#314・
   #280・#282・#293・#295・#296・#305・#307〜#309・
   #384〜#387・#390・#598、SV #13〜#16・#24・#25・#83・
   #100・#104・#105・#110・#111・#113・#175・#190・
   #377、AL #105・#131・#132・#140〜#143・#166・#206）
   を「未着手」「Blocked」として記述しない。後続の
   Open Gate（#306・#313・#281 等）を正本として接続
   する。
3. **新規 Lane の接続**: Script（RF #553/#558〜#560、
   SV #259/#264/#266/#271〜#274）・Creative Branch
   （SV #277〜#296、RF #562）・Future capability
   （RF #315〜#318、SV #119/#124/#130/#138）は
   M0〜M7 の Blocked by にしない。共有契約の変更が
   生じた場合のみ #294 の候補 SHA tuple と各 Consumer
   追従 Owner（SV #28・AL #127）で検証する。RF #553 は
   共通化の設計レビューであり共通 Engine 実装を要求
   しない（Issue 本文どおり）。
4. **循環禁止**: Issue 着手を #262 の採用完了
   （#419）に依存させる循環を作らない。#419 は #262
   PoC 判定と #388/#389/#390 の Evidence を入力とする
   Acceptance 側の決定である。
5. **Consumer 対等性**: AL（f64・長尺・CLI・Dataset/
   Export）と SV（f32・DAW/RT・Project/Plugin）の Oracle
   を別系統として維持し、片方の Pass を他方へ流用
   しない。接続 Owner は RF #219（契約・CI）/
   RF #294（候補 SHA tuple・OS 証拠）/AL #127/SV #28。

## Deferred/Activation 記録の設計

L2/L3 が必要となる製品利用条件を以下の対応で記録する
（POLICIES.md §2 の Required/Deferred と §4 の
Activation を計画へ接続）:

| 製品利用条件 | 検証レベル | Owner Issue | 現行分類 |
| --- | --- | --- | --- |
| Windows 実 Window/Dock/IME/DPI/Focus | GUI-L2 | RF #313/#243、SV #114 | Required（提供機能のみ） |
| 実 Plugin Native Dock（VST3/CLAP/AU） | EXT-L3 | RF #388/#389/#532、SV #176 | Required（Windows 利用時） |
| 実 Audio Input Device 録音 | EXT-L3 | RF #251、SV #115/#117 | Required（利用時） |
| OS MIDI backend・VST3 翻訳 | EXT-L3 | RF #253、SV #126/#127 | Required（利用時） |
| OBS 実 Provider/外部環境 | EXT-L3 | RF #281 | Deferred（利用要件まで） |
| macOS/Linux の GUI-L2/EXT-L3 | — | 各 Owner Issue | Deferred（利用要件まで） |
| Future capability（RF #315〜#318、SV #119/#124/#130/#138） | — | 各 Future Issue | Deferred/NotStarted（Trigger 未達） |

「現行計画 Snapshot の完了」（本タスクの Done =
Snapshot 作成）と「将来の実機受入」（上表の Required
項目が各 Owner Issue で Pass になること）は別の完了
条件として記録し、前者を後者の Pass とみなさない。

## OS-L1 と既存 CI の接続設計

- OS-L1 の必須範囲: 3OS の Native Build/Link、Headless/
  RT/CLI・必要 Cargo Feature、Consumer 独立 Oracle を
  既存の変更検知付き Daily Full CI と手動 Full CI で
  対象 SHA 実走する。Job 定義・green のみを個別 Case
  の Pass としない。
- CI 基盤の現状: RF #598（3OS OS-L1 CI 証拠補完）・
  SV #377・AL #206 は CLOSED。RF #219/#294 が候補 SHA
  tuple と OS 別証拠の運用 Owner。
- AL Oracle: f64 精度・長尺・PTS/timebase・NaN/gap・
  Dataset/Export・CLI/headless 分離（AL #127）。
- SV Oracle: f32/RT・Project/Undo/Save・Plugin Model・
  DAW 意味論（SV #28、実行計画 SV #23）。
- 候補 SHA tuple: Ruviform/AL/SV 3 repo の resolved SHA
  を Cargo.lock 正本で管理し、#294 が tuple ごとに両
  Consumer を独立判定する。

## Risks and Trade-offs

- **Issue 状態の陳腐化**: census は 2026-11-14 時点。
  Issue 更新で状態が変われば Snapshot 節の追記更新が
  必要 → 版識別子と証拠ファイルで基準日を固定し、
  Issue #193 本文更新時の同期義務を維持する。
- **Snapshot の権威誤認**: Snapshot を計画再定義や受入
  Pass と誤読するリスク → 「追記型の状態記録」である
  こと、M0〜M7/G0〜G7 定義を変更しないこと、実機受入は
  Deferred/Activation 先の責務であることを節冒頭に明記。
- **表の肥大化**: M0〜M7 × 7 項目の表は大きいが、
  Milestone 単位の完了定義を 1 行に潰すと Gate 条件が
  失われる → 完了定義は G Gate 番号と証拠種別で参照し、
  判定条件の本文は計画正本の G0〜G7 表を正本として
  重複記述しない。
- **新規 Lane の範囲膨張**: Script/Creative Branch を
  M0〜M7 へ無理に組み込むと DAG が破綻する → 並行 Lane
  として接続規則のみを定義し、Blocked by にしない。

## Deferred Design Concerns

| Concern | Reason | Re-engagement trigger | Risk if ignored |
| --- | --- | --- | --- |
| GitHub blockedBy の正式登録・削除 | Coordinator 責務 | Issue 本文と GitHub 依存の齟齬検出 | 文書と GitHub 依存の二重管理齟齬 |
| Snapshot の次版更新 | Issue 状態は変動する | Issue #193 関連 Issue の状態変化検知 | Snapshot 陳腐化 |
| L2/L3 実機受入の実施 | 各 Owner Issue の責務 | 製品利用条件の確定・Required 昇格 | 未実測を Pass 扱い |
| #192 設計チェーン（issue-192-*）との相互参照 | 別 Worker 占有・merge 状態に依存 | 当該文書の main merge | 未 merge 文書への依存 |

## Verification Notes

- [x] spec の R-1〜R-7 が本設計の Snapshot 構造・項目
  定義・DAG 解消規則・Deferred/Activation・CI 接続で
  カバーされることを対応付けた。
- [x] Issue 状態の根拠は `gh issue list/view` の
  2026-11-14 実測（`evidence/rf193/
  issue-state-census-2026-11-14.md`）であり、推測で
  Open/Closed を記述していない。
- [x] 参照する docs パス・Issue 番号が merge 済み正本
  または実在 Issue に解決されることを作成時に確認。
- [ ] `docs/` 以外に差分がないことを `git status`/
  `git diff` で確認（PR 前に実施）。
- [ ] test-spec の検証観点（Snapshot の 7 項目完備・
  URL 実在・状態分類の正しさ・追記型・分離原則）を
  `docs/test-specs/issue-193-migration-plan-update.md`
  で固定する。
