---
id: "PLAN-034UCTG1MAGz2NU32GulcZ"
type: "plan"
status: "approved"
title: "Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 計画"
created: "2026-11-09"
updated: "2026-11-14"
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

## Snapshot-2026-11-14 — M0〜M7 着手条件・Headless Slice・Owner/Gate 確定（版付き）

**版識別子:** `Snapshot-2026-11-14`（Issue 状態を `gh` で
実測した基準日版）。本節は Issue #193 残件の版付き
Snapshot であり、M0〜M7/G0〜G7/RB0〜RB6・GUI lane・
Cross-Consumer 受入の定義を**変更しない**。既存節は
2026-11-09/2026-11-04 時点の履歴 snapshot として保持
し、本節を最新の状態記録とする。証拠:
[evidence/rf193/issue-state-census-2026-11-14.md](../../evidence/rf193/issue-state-census-2026-11-14.md)。
設計根拠:
[docs/designs/issue-193-migration-plan-update-design.md](../designs/issue-193-migration-plan-update-design.md)
と本書の設計正本側
[docs/designs/issue-193-migration-execution-plan.md](../designs/issue-193-migration-execution-plan.md)
の同名 Snapshot 節（内容は一致させる）。

**状態語彙（POLICIES.md）:** 要求状態
Required/Deferred/NotApplicable と実行結果
Pass/Fail/NotRun/Unsupported は別軸。Issue の
open/closed は実装 Blocked by と同義ではない。
OS-L1（3OS Native Build/Link/Headless/RT/CLI/必要
Feature・Consumer 独立 Oracle）は Required。GUI-L2/
EXT-L3 は Windows で実際に提供・利用する機能のみ
Required、macOS/Linux は利用要件発生まで Deferred。

### Issue 状態照合の結果（2026-11-14 実測）

- M0〜M7 の実装 Issue #184〜#198・M1 着手 Gate
  #199/#201・IO 系 #282/#288/#295/#296/#305・OBS 系
  #280/#293・Probe Gap #307〜#309・GUI Test #310〜#312/
  #314・GUI lane #384〜#387/#390・3OS OS-L1 CI #598 は
  **CLOSED**。Consumer 側も SV #13〜#16/#24/#25/#83/
  #100/#104/#105/#110/#111/#113/#175/#190/#377、
  AL #105/#131/#132/#140〜#143/#166/#206 が CLOSED。
- 同日再実測で RF #243・#313 が CLOSED（COMPLETED）へ
  遷移（2026-11-13T19:24-19:25Z closeout — OS-L1 Pass・
  Windows 利用範囲 GUI-L2 Pass、残存 Deferred 能力は
  SV #114・AL #127・RF #294 等の製品 Owner で追跡）。
- OPEN の残存 Owner/Gate: RF #219/#251/#253/#262/
  #275/#281/#294/#306/#388/#389/#419/#465/#532/
  #553/#558/#559/#560/#562/#315〜#318、SV #22/#23/#28/
  #84/#101/#106〜#109/#112/#114〜#117/#119/#124/#126/
  #127/#130/#138/#176/#177/#259/#264/#266/#271〜#274/
  #232/#277〜#296、AL #127/#189。
- 確認した全 Open Issue で `着手条件（Blocked by）` は
  「なし」（2026-11-13 ゲート正規化）。実 OS GUI/
  Device/Plugin Gate は `統合・受入で関係する Issue
  （着手は止めない）` 側に分離済みであり、Closed/
  Deferred/NotRun を実装 Blocked by として誤記して
  いないことを確認した（検証記録は evidence 参照）。

### M0〜M7 Snapshot（開始条件・Headless Slice・完了定義・Integration Gate・責務・Blocked by・Owner）

共通: 各 Milestone の OS-L1 必須範囲は 3OS の Native
Build/Link・Headless/RT/CLI・必要 Feature を既存の
変更検知付き Daily Full CI と手動 Full CI で対象 SHA
実走すること（新規 Workflow は増設しない）。Blocked by
は全 Milestone「なし」。

#### M0 Baseline（#194 CLOSED）

- **開始条件**: 充足済み（G0 監査条件全項充足・報告
  PR #200 merge・#194 Close 済み）。
- **着手可能 Headless Slice**: 変更ごとの Consumer 影響
  分類（impacted/no impact/unknown）の記録運用
  （#219 運用、対象 SHA は #294 tuple）。
- **完了定義**: G0 証拠の維持と変更影響ごとの継続比較。
- **Integration Gate**: なし（継続運用。個別変更の
  tuple 判定は #294）。
- **責務**: RF #219/#294（分類・tuple 判定 Owner）、
  AL #127/SV #28（Consumer 別 Oracle）。
- **Blocked by**: なし（Issue 本文の `着手条件
  （Blocked by）` は空。実 OS/実機 Gate は Integration
  Gate・Deferred/Activation 側）。
- **独立 Owner Issue**: [#219](https://github.com/scarecrow173/ruviform/issues/219)、
  [#294](https://github.com/scarecrow173/ruviform/issues/294)、
  [AL #127](https://github.com/scarecrow173/audio-lens/issues/127)、
  [SV #28](https://github.com/scarecrow173/synvelis/issues/28)。

#### M1 Graph（#184 CLOSED、PR #225）

- **開始条件**: 充足済み。
- **着手可能 Headless Slice**: Typed Graph IR/Validator/
  非 RT 検証の継続拡張。SV #106 Semantic→Typed Graph
  Lowering の Consumer 側 headless 作業。
- **完了定義**: G1 — Typed Domain/Direction/Channel/
  Multiplicity/Capability と Cycle/無効接続の非 RT 検証、
  合法な既存単一 Bus Graph の Compat 経路継続（Synvelis
  EX-001〜006）。
- **Integration Gate**: SV #106（Semantic lowering 接続）、
  #294 tuple での両 Consumer G1 証拠。
- **責務**: RF typed graph 実装済み（#184）。Consumer
  lowering は SV #106 が Owner。
- **Blocked by**: なし。
- **独立 Owner Issue**: [SV #106](https://github.com/scarecrow173/synvelis/issues/106)、
  [#294](https://github.com/scarecrow173/ruviform/issues/294)。

#### M2 Audio Runtime（#195 CLOSED、PR #227）

- **開始条件**: 充足済み。
- **着手可能 Headless Slice**: Compiler/PDC/Prepared
  Schedule/RT Ownership の headless・CI 検証継続。RT
  時間 budget は L1 CI lane（#598 CLOSED で 3OS L1 証拠
  補完済み）と headless fault case で扱う。
- **完了定義**: G2 — DAG 順序・Buffer lifetime/PDC・
  無割当/非 blocking/no OS I/O・Single RT Owner・
  Last-good・Generation 不一致 Event 拒否・Retire・
  過負荷 Fault の再現（EX-007〜020、TA-001〜012）。
- **Integration Gate**: SV #106/#109（Graph/Probe/RT
  Oracle）、SV #28/AL #127 の RT・精度独立判定、
  #294 tuple。
- **責務**: RF #195 実装済み。RT fault/負荷の残 Gate は
  各 Owner Issue と #294。
- **Blocked by**: なし。
- **独立 Owner Issue**: [SV #109](https://github.com/scarecrow173/synvelis/issues/109)、
  [#294](https://github.com/scarecrow173/ruviform/issues/294)、
  [SV #28](https://github.com/scarecrow173/synvelis/issues/28)、
  [AL #127](https://github.com/scarecrow173/audio-lens/issues/127)。

#### M3 Time/Event（#189/#190/#188/#186 CLOSED）

- **開始条件**: 充足済み。
- **着手可能 Headless Slice**: Rational Beat/Variable
  PPQN・Transport Span・Timed MIDI・Parameter Events の
  headless 継続。#253 の OS 非依存部分（backend 抽象・
  VST3 翻訳の headless 側）。
- **完了定義**: G3 — PPQN 480/960/1000/1920・1/7 Beat・
  Tempo/Loop/Seek/負 Pre-roll・Clock Epoch・Transport
  Applied Frame・Note Off/Overflow/Sustain（PPQN-01〜12、
  TT-001〜024）。
- **Integration Gate**: [#253](https://github.com/scarecrow173/ruviform/issues/253)
  （OS MIDI backend・VST3 翻訳）、SV #126（MIDI
  Import/Export native timestamp/chase）・SV #127
  （Plugin Process 境界精度）。
- **責務**: RF #253 が OS backend Owner。SV #126/#127 が
  Consumer 側受入 Owner。
- **Blocked by**: なし。
- **独立 Owner Issue**: [#253](https://github.com/scarecrow173/ruviform/issues/253)、
  [SV #126](https://github.com/scarecrow173/synvelis/issues/126)、
  [SV #127](https://github.com/scarecrow173/synvelis/issues/127)。

#### M4 Plugin（#196 CLOSED、PR #232）

- **開始条件**: 充足済み。
- **着手可能 Headless Slice**: plugin host L1/feature
  test・CLAP/VST3 能力差/Unsupported/明示 Degraded の
  headless 記録継続。
- **完了定義**: G4 — ProcessConfig min/max 厳守・256
  Frame callback 内 100 Frame Loop Wrap の正確性検証・
  EngineFrame→callback/span/physical-process offset
  対応（TT-025〜030）。
- **Integration Gate**: #388/#389/#532（NativeSurfaceSlot/
  Editor Session/DComp 制約）、SV #176（実 Plugin Dock
  製品 Gate）、SV #127（Plugin Process 境界）、
  #294 tuple。
- **責務**: RF #196 実装済み。Native Dock/実 Plugin は
  RF #388/#389/#532（共通）と SV #176（製品）が Owner。
- **Blocked by**: なし。
- **独立 Owner Issue**: [#388](https://github.com/scarecrow173/ruviform/issues/388)、
  [#389](https://github.com/scarecrow173/ruviform/issues/389)、
  [#532](https://github.com/scarecrow173/ruviform/issues/532)、
  [SV #176](https://github.com/scarecrow173/synvelis/issues/176)、
  [SV #127](https://github.com/scarecrow173/synvelis/issues/127)。

#### M5 Device/Reader（#187/#197 CLOSED、PR #231/#250）

- **開始条件**: 充足済み。
- **着手可能 Headless Slice**: bounded Reader/Prefetch/
  RT 消費/underrun の headless 継続（Mock Input
  経路）。#251 の OS 非依存部分。
- **完了定義**: G5 — Clock 根拠/Confidence/
  Discontinuity・xrun・Decode/Prefetch underrun の明示、
  録音 Lossless と Probe Lossy の区別。
- **Integration Gate**: [#251](https://github.com/scarecrow173/ruviform/issues/251)
  （real backend input plumbing・実機検証）、SV #115
  （実 Audio Device 録音統合）・SV #117
  （Monitoring/Punch/Latency）。
- **責務**: RF #251 が real backend Owner。SV #115/#117
  が製品録音受入 Owner。
- **Blocked by**: なし。
- **独立 Owner Issue**: [#251](https://github.com/scarecrow173/ruviform/issues/251)、
  [SV #115](https://github.com/scarecrow173/synvelis/issues/115)、
  [SV #117](https://github.com/scarecrow173/synvelis/issues/117)。

#### M6 Output/Offline（#191/#198 CLOSED、PR #229/#235）

- **開始条件**: 充足済み。
- **着手可能 Headless Slice**: stateful-safe sequential
  offline・WAV drain/finalize→reopen の headless 継続。
  StagedFile publish（#295/#296/#305 CLOSED）経由の FS
  edge・#306 の fault/fuzz headless 行。
- **完了定義**: G6 — stateful DSP/Plugin の無条件時間
  並列化禁止・Export→drain/finalize→close/sync→
  reopen/verify・失敗/取消/容量不足で Completed や破損
  した公開先を作らない（MR-001〜028 該当ケース）。
- **Integration Gate**: [#306](https://github.com/scarecrow173/ruviform/issues/306)
  （IO-IMP-12 合成 E2E・Fault/Fuzz/OS/FS・Consumer 受入）、
  SV #28・AL #127（Consumer 別非回帰）。
- **責務**: RF #306 が共通 I/O 合成 Gate Owner。
  Consumer 側は SV #28/AL #127。
- **Blocked by**: なし。
- **独立 Owner Issue**: [#306](https://github.com/scarecrow173/ruviform/issues/306)、
  [SV #28](https://github.com/scarecrow173/synvelis/issues/28)、
  [AL #127](https://github.com/scarecrow173/audio-lens/issues/127)。

#### M7 UI/Probe（#181/#182/#185 CLOSED、PR #237/#240/#230）

- **開始条件**: 充足済み。
- **着手可能 Headless Slice**: Window/Job 寿命分離・
  Dock/Restore・Generic Probe の headless/fake 継続。
  GUI Test 基盤 #310〜#312/#314・実 OS Driver/
  Capability Matrix #313 は CLOSED（#313 は 2026-11-14
  現行スコープ Closeout）。
- **完了定義**: G7 — 既存 API/Project/他 Consumer の
  回帰試験・platform evidence・Legacy Deprecation の
  理由/利用者通知/復旧手順・未検証事項の明記。後継 ADR
  なしに Accepted ADR と相反する方式へ変更しない。
- **Integration Gate**: GUI lane Acceptance — GPUI 採否
  #262→#419、Native Dock #388/#389/#532、SV #112/#114/
  #176/#177、AL #127/#189、#219/#294 tuple、WebView
  撤去 #465。実 Window/OS Driver 検証 #243/#313 は
  2026-11-14 CLOSED（OS-L1 + Windows 利用範囲 GUI-L2
  Pass）。残存の実 Window/製品 GUI 受入は SV #114・
  AL #127 と #294 tuple が担う。
- **責務**: RF #262/#388/#389/#419/#465/#532 が共通
  Owner（#243/#313 は現行 Required 範囲を完了し
  CLOSED）。SV #112/#114/#176/#177 と AL #127/#189 が
  製品 Owner。
- **Blocked by**: なし。
- **独立 Owner Issue**: [#262](https://github.com/scarecrow173/ruviform/issues/262)、
  [#388](https://github.com/scarecrow173/ruviform/issues/388)、
  [#389](https://github.com/scarecrow173/ruviform/issues/389)、
  [#419](https://github.com/scarecrow173/ruviform/issues/419)、
  [#465](https://github.com/scarecrow173/ruviform/issues/465)、
  [#532](https://github.com/scarecrow173/ruviform/issues/532)、
  [SV #112](https://github.com/scarecrow173/synvelis/issues/112)、
  [SV #114](https://github.com/scarecrow173/synvelis/issues/114)、
  [SV #176](https://github.com/scarecrow173/synvelis/issues/176)、
  [SV #177](https://github.com/scarecrow173/synvelis/issues/177)、
  [AL #127](https://github.com/scarecrow173/audio-lens/issues/127)、
  [AL #189](https://github.com/scarecrow173/audio-lens/issues/189)。

### 並行 Lane（M0〜M7 の Blocked by にしない）

- **Script**: RF [#553](https://github.com/scarecrow173/ruviform/issues/553)
  （AUTX-01 共通化設計レビュー — 共通 Engine 実装を要求
  しない）→ [#558](https://github.com/scarecrow173/ruviform/issues/558)→
  [#559](https://github.com/scarecrow173/ruviform/issues/559)/
  [#560](https://github.com/scarecrow173/ruviform/issues/560)。
  SV [#259](https://github.com/scarecrow173/synvelis/issues/259)（AUTX-00）
  → [#271](https://github.com/scarecrow173/synvelis/issues/271)〜
  [#274](https://github.com/scarecrow173/synvelis/issues/274)、
  [#264](https://github.com/scarecrow173/synvelis/issues/264)/
  [#266](https://github.com/scarecrow173/synvelis/issues/266)。
  共有契約変更時のみ #294 tuple と SV #28/AL #127 で検証。
- **Creative Branch / Render Checkpoint**: SV
  [#277](https://github.com/scarecrow173/synvelis/issues/277)（CB-00）
  → [#278](https://github.com/scarecrow173/synvelis/issues/278)〜
  [#288](https://github.com/scarecrow173/synvelis/issues/288)、
  RCP [#289](https://github.com/scarecrow173/synvelis/issues/289)〜
  [#296](https://github.com/scarecrow173/synvelis/issues/296)。
  RF [#562](https://github.com/scarecrow173/ruviform/issues/562)
  （Generic Alternate Evaluation Context PoC —
  Headless/Fake 優先、実 Plugin は Deferred）。
- **Future capability**: RF
  [#315](https://github.com/scarecrow173/ruviform/issues/315)〜
  [#318](https://github.com/scarecrow173/ruviform/issues/318)、
  SV #119/#124/#130/#138 — 各 Issue の着手 Trigger 未達で
  Deferred/NotStarted。Blocked by にしない。

### Deferred/Activation 記録（L2/L3 が必要な製品利用条件）

| 製品利用条件 | 検証レベル | Owner Issue | 現行分類 |
| --- | --- | --- | --- |
| Windows 実 Window/Dock/IME/DPI/Focus | GUI-L2 | SV #114、AL #127（RF #243/#313 は 2026-11-14 CLOSED — 現行利用範囲 Pass） | Required（提供機能のみ） |
| 実 Plugin Native Dock（VST3/CLAP/AU） | EXT-L3 | RF #388/#389/#532、SV #176 | Required（Windows 利用時） |
| 実 Audio Input Device 録音 | EXT-L3 | RF #251、SV #115/#117 | Required（利用時） |
| OS MIDI backend・VST3 翻訳 | EXT-L3 | RF #253、SV #126/#127 | Required（利用時） |
| OBS 実 Provider/外部環境 | EXT-L3 | RF #281 | Deferred（利用要件まで） |
| macOS/Linux の GUI-L2/EXT-L3 | — | 各 Owner Issue | Deferred（利用要件まで） |
| Future capability（RF #315〜#318、SV #119/#124/#130/#138） | — | 各 Future Issue | Deferred/NotStarted |

本 Snapshot の完了（版付き状態記録の作成）と、上表
Required 項目の将来の実機受入（各 Owner Issue での
Pass）は**別の完了条件**である。前者を後者の Pass と
みなさない。

### AL/SV 別 Oracle・候補 SHA・既存 CI の M0〜M7 受入への接続

- **AL Oracle**（f64 精度・長尺・PTS/timebase・NaN/
  gap・Dataset/Export・CLI/headless 分離）: AL #127 が
  Owner。M0〜M7 の各完了定義で Consumer 別証拠の
  AudioLens 側を担う。
- **SV Oracle**（f32/RT・Project/Undo/Save・Plugin
  Model・DAW 意味論）: SV #28 が Owner、実行計画は
  SV #23。Synvelis 側証拠を担う。
- **候補 SHA tuple**: Ruviform/AL/SV 3 repo の resolved
  SHA を Cargo.lock 正本で管理し、RF #294 が tuple ごと
  に両 Consumer を独立判定する。片方の Pass を他方へ
  流用しない。
- **既存 CI**: 変更検知付き Daily Full CI と手動 Full
  CI（`workflow_dispatch`）を使用。RF #598・SV #377・
  AL #206（3OS OS-L1 CI 証拠）は CLOSED。OS 別実走証拠
  は各 Run URL/SHA で #294/#219 経由に記録し、Job
  定義や green のみを個別 Case の Pass としない。

### DAG と Owner の表（実装順序の正本・2026-11-14 確定）

| 領域 | Owner（Issue） | 先行関係 |
| --- | --- | --- |
| M0 Baseline 継続分類 | RF #219/#294、AL #127、SV #28 | 全 Milestone の前提（充足済み） |
| M1 Graph | RF #184（済）、SV #106、#294 | G0 充足済み。SV #106 が Consumer 接続 |
| M2 Audio Runtime | RF #195（済）、SV #109、SV #28/AL #127、#294 | G1 契約 + RT Oracle |
| M3 Time/Event | RF #189/#190/#188/#186（済）、#253、SV #126/#127 | G2 契約。#253 は OS backend 残件 |
| M4 Plugin | RF #196（済）、#388/#389/#532、SV #176/#127 | G2+G3 契約。実 Plugin Dock は EXT-L3 |
| M5 Device/Reader | RF #187/#197（済）、#251、SV #115/#117 | Mock 先行。実 Device は EXT-L3 |
| M6 Output/Offline | RF #191/#198（済）、#306、SV #28/AL #127 | E2E-L 最低限済。#306 が合成 Gate |
| M7 UI/Probe | RF #181/#182/#185/#243/#313（済）、#262/#388/#389/#419/#465/#532、SV #112/#114/#176/#177、AL #127/#189 | GUI lane Acceptance と直結 |
| Script Lane | RF #553/#558/#559/#560、SV #259/#264/#266/#271〜#274 | M0〜M7 を Block しない。共有契約は #294 で検証 |
| Creative Branch Lane | SV #277〜#296、RF #562 | M0〜M7 を Block しない。RF #562 は Headless PoC |
| Future capability | RF #315〜#318、SV #119/#124/#130/#138 | Trigger 未達で Deferred/NotStarted |
| GPUI 採否 | #262 → #419 | #388/#389/#390 Evidence + 両 Consumer 受入。着手 Blocker にしない |
| 両 Consumer 互換判定 | #219 / #294 | 候補 SHA tuple ごとの対象能力別証拠 |
| Legacy WebView 撤去 | #465（RF）、AL #189、SV #232 | 破壊的撤去は製品 Native 代替 + Windows 利用 GUI/Plugin 実受入 + 両 Consumer 独立回帰の後 |

### Verification Notes（Snapshot 節の検証）

- [x] M0〜M7 全 Milestone に開始条件・着手可能 Headless
  Slice・完了定義・Integration Gate・責務・Blocked by・
  独立 Owner Issue を Issue URL 付きで対応付けた。
- [x] Issue 状態は 2026-11-14 の `gh` 実測
  （`evidence/rf193/issue-state-census-2026-11-14.md`）
  に基づき、Closed/Deferred/NotRun を実装 Blocked by
  として誤記していない。
- [x] L2/L3 の製品利用条件を Deferred/Activation とし
  て記録し、計画 Snapshot 完了と将来実機受入を分離した。
- [x] AL f64/長尺/CLI・SV f32/DAW/RT の別 Oracle、候補
  SHA tuple、既存 Daily/手動 CI、OS 別実走証拠の接続先
  を M0〜M7 受入へ対応付けた。
- [x] `docs/` 以外に差分がないことを `git status`/
  `git diff` で確認（PR #615・本更新 PR で実施）。

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
