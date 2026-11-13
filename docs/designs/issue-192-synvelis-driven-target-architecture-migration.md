---
id: "DESIGN-034UBX7Lbr3m4u4ZGzQpHA"
type: "design"
status: "approved"
title: "Issue #192 Synvelis-driven Target Architecture & Migration — umbrella migration 設計正本"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
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
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/media-io-backend-abstraction.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/musical-time-model.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-192-synvelis-driven-target-architecture-migration.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/synvelis/issues/22"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/181"
    - "https://github.com/scarecrow173/ruviform/issues/182"
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/186"
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/189"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/191"
    - "https://github.com/scarecrow173/ruviform/issues/194"
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/197"
    - "https://github.com/scarecrow173/ruviform/issues/198"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/251"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/276"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/287"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/296"
    - "https://github.com/scarecrow173/ruviform/issues/297"
    - "https://github.com/scarecrow173/ruviform/issues/298"
    - "https://github.com/scarecrow173/ruviform/issues/299"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/409"
    - "https://github.com/scarecrow173/ruviform/issues/464"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/synvelis/issues/17"
    - "https://github.com/scarecrow173/synvelis/issues/18"
    - "https://github.com/scarecrow173/synvelis/issues/19"
    - "https://github.com/scarecrow173/synvelis/issues/20"
    - "https://github.com/scarecrow173/synvelis/issues/21"
    - "https://github.com/scarecrow173/synvelis/issues/22"
    - "https://github.com/scarecrow173/synvelis/issues/23"
    - "https://github.com/scarecrow173/synvelis/issues/25"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "https://github.com/scarecrow173/synvelis/issues/82"
    - "https://github.com/scarecrow173/synvelis/issues/83"
    - "https://github.com/scarecrow173/synvelis/issues/84"
    - "https://github.com/scarecrow173/synvelis/issues/106"
    - "https://github.com/scarecrow173/synvelis/issues/109"
    - "https://github.com/scarecrow173/synvelis/issues/175"
    - "https://github.com/scarecrow173/synvelis/issues/176"
    - "https://github.com/scarecrow173/synvelis/issues/177"
    - "https://github.com/scarecrow173/audio-lens/issues/95"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/audio-lens/issues/139"
    - "https://github.com/scarecrow173/audio-lens/issues/140"
    - "https://github.com/scarecrow173/audio-lens/issues/141"
    - "https://github.com/scarecrow173/audio-lens/issues/142"
    - "https://github.com/scarecrow173/audio-lens/issues/143"
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  defers: []
  deferred-by: []
---

# Issue #192 Synvelis-driven Target Architecture & Migration — umbrella migration 設計正本

## Context

Issue #192 は「Synvelis-driven Ruviform Target v0.7.1 —
Dependency DAG / Graph / State Ownership」を扱う umbrella
tracking Issue であり、Synvelis が全体設計として採用した
Target Architecture v0.7.1（Synvelis #22）を起点に、
Ruviform の汎用構造・責務移動・API 境界・Migration Gate
を管理する。2026-11-02 追補により **Synvelis と AudioLens
を対等な Consumer とする共通基盤**としての設計原則が
追加されている。

**設計・実行の権威（本書が置き換えないもの）**

- 目標アーキテクチャの設計正本:
  [synvelis-driven-target-architecture-and-migration.md](synvelis-driven-target-architecture-and-migration.md)
  （approved・2026-11-01 合意）。crate 配置・責務境界・
  概念 API・M0〜M7 表・TA gate の要求証拠・ADR 方針・
  Rollback の本文は同文書が所有する。
- 実行計画: Issue #193（M0〜M7 の依存 DAG・G0〜G7 受入
  gate・GUI lane 補足・consumer 別受入表の正本）。
- Synvelis 側正本: `synvelis-driven-ruviform-target-
  architecture-v07.md`（Synvelis repo）。
- 境界 ADR: `docs/adr/synvelis-driven-generic-execution-
  boundaries.md`（accepted）。

**本書の役割**: Issue #192 の受入管理面を docs チェーン
（spec → design → test-spec → plan → task）として正本化
し、TA-001〜TA-012・M0〜M7・G0〜G7・GUI lane・Consumer
接続・ADR/互換 policy について「どの正本文書・Issue・
証拠が対応するか」を一箇所に統合する **tracking
canonical** として機能する。領域別の詳細設計は各 Issue
の正本が所有し、本書はそれらへの参照索引と現状
snapshot（2026-11-09 時点）を記録する。

**本書が固定するもの / 固定しないもの**

- 固定する: 正本関係（umbrella 設計 + #193 が権威）、
  3 者 Ownership boundary、capability → 正本・証拠の
  対応索引、TA-001〜TA-012 の gate 対応、G0〜G7/E2E/GUI
  lane の実行構造、ADR/互換/Rollback policy の現状反映、
  Consumer 接続面、設計合意と実装/検証の分離。
- 固定しない: 目標アーキテクチャの再決定、新 crate/API
  の確定、領域別設計の詳細（#184〜#198・IO #283〜#306・
  Observability #275〜#281（#275/#278/#281 OPEN・
  #276/#277/#279/#280 CLOSED）・GUI #384〜#390/#465 等の各
  正本が所有）、Consumer repo の実測、gate の正式 Close
  判定。

**実装の現状（2026-11-09 時点の snapshot）**

- M0〜M7 の実装 Issue #181/#182/#184〜#191/#194〜#198 は
  全て CLOSED（各 Issue の設計+実装 slice が merge 済み）。
  merge 証拠は #193 の進捗表と git log の merge commit
  （#225〜#240・#250・#252 等）で確認できる。
- 未解決の後続: #243（GPUI 実機 GUI 検証・OPEN）・
  #251（実 Audio Input・OPEN）・#253（OS MIDI/VST3・
  OPEN）・#262（GPUI Native 採否・OPEN）。
- Cross-Consumer / Gate: #219（OPEN・spec proposed・
  PR #224/#274 の CI 基盤は部分 merge）・#281（OPEN・
  OBS-06）・#288（OPEN・design approved）・#294（OPEN・
  consumer-verify 拡張）。
- 共通基盤（#192 の consumer 接続で消費される正本）:
  IO #282 CLOSED・#283 CLOSED・#284/#285/#286 OPEN
  （設計正本 merge 済み）・#287 CLOSED・#288 OPEN・
  #295〜#304 実装 CLOSED・#305/#306 OPEN・#409 CLOSED。
  Observability #275/#278/#281 OPEN・#276/#277/#279/#280
  CLOSED（統合設計 #275・slice 実装 #276〜#280 が merge
  済み）。
- GUI lane: #384 CLOSED（後継 ADR は proposed=仮決定）・
  #385/#386/#387/#390 CLOSED・#388/#389 OPEN・#464 OPEN・
  #465 OPEN（Legacy WebView 撤去）。Consumer 側:
  Synvelis #175 CLOSED・#176/#177 OPEN、AudioLens #166
  CLOSED。
- Issue 状態は変動し得る snapshot として記録し、正式な
  判定は各 Issue と gate 証拠を正とする。

文書チェーン: [spec](../specs/issue-192-synvelis-driven-target-architecture-migration.md)・
[plan](../plans/issue-192-synvelis-driven-target-architecture-migration.md)・
[test-spec](../test-specs/issue-192-synvelis-driven-target-architecture-migration.md)・
[task](../tasks/issue-192-synvelis-driven-target-architecture-migration.md)。

## Scope

### In Scope

- Issue #192 本文（umbrella 設計・TA-001〜TA-012・
  2026-11-02 両 Consumer 対等原則追補）と Issue #193
  （M0〜M7・G0〜G7・GUI lane 補足）の要求事項の
  tracking canonical としての文書化。
- umbrella 設計文書・#193・関連 ADR・領域別正本文書群・
  Consumer Issue の参照統合と現状 snapshot の記述。

### Out of Scope

- 目標アーキテクチャの再決定・umbrella 設計文書の本文
  改訂・領域別設計の再定義・新規 crate/API の確定。
- M0〜M7 の実装作業・Consumer repo の変更/実測・CI 変更
  ・証拠基盤の新設。
- gate（TA/G/consumer-verify）の正式 Close 判定。

## Components and Boundaries — 本書の責務と 3 者 Ownership

### 本書（tracking canonical）の責務

- Issue #192 の受入項目（TA-001〜TA-012）と実行計画
  （M0〜M7・G0〜G7・GUI lane・E2E）について、担当する
  正本文書・Issue・証拠レーン・現在の証拠状態を対応付ける。
- umbrella 設計文書・#193・領域別正本・Consumer Issue
  への参照索引を提供し、設計判断の内容自体は複製しない。
- 状態は必ず日付付き snapshot として記録し、変動する
  Issue 状態を恒久的事実として扱わない。

### 3 者 Ownership boundary（2026-11-09 時点）

umbrella 設計 §2 の Synvelis 中心表を、2026-11-02 追補
（両 Consumer 対等）と #219 互換契約に基づき 3 者表へ
更新した対応。詳細な能力別の所有は umbrella 設計 §2 と
各領域別正本が正本であり、本表は umbrella レベルの
責務分界の要約である。

| Capability | Ruviform owns | Synvelis owns | AudioLens owns |
| --- | --- | --- | --- |
| Time | Rational Beat/PPQN/TempoMap/Clock 変換の汎用型・検証（media-core、#189 で実装） | Project PPQN 初期値・Timebase 選択・Timeline/Clip Source Mapping | source-native media time・raw PTS/DTS/timebase の保持。音楽 Beat への暗黙変換を要求しない |
| Graph | Typed Node/Port/Edge/Signal Domain/Capability 検証と Audio Runtime Buffer/PDC/Processor Schedule（#184/#195） | Track/Bus/Send/Automation 等 Semantic Graph・ID 対応・Project Command（#106 SF-03 lowering） | 既存 Graph/CLI 利用範囲の棚卸しと非退行確認（#127） |
| Processor / RT | Immutable Prepared Schedule・Stateful Processor Lifetime・単一 RT Owner・安全な非 RT Retire（#195/#196） | Plugin を含む Project Instance Identity・Undo/Persistence 意味 | 共有 crate/API/feature の非退行（#127） |
| MIDI / Control | Sample Offset Event・Transport Span・Bounded Delivery・Note Ledger・fault/overflow policy（#186/#188/#190） | Note Source/Editing/Recording・Automation Write・Comp | —（直接利用なし。共有型の非退行のみ） |
| Media | Generic Format/Reader/Decoder/Prefetch/Backend・WAV Encode/Mux/Drain/Finalize（#191/#197・IO #283〜#306） | Remote Provider・Pinned Asset・Project Verified Blob・Source/Asset/Clip 登録（#82/#84） | native PCM/元 sample format・bit depth・f64 解析・Dataset schema/provenance（#139/#143） |
| Offline / Output | Stateful Sequential Render・Output Writer・Safe Publish（#198・IO-05 #287） | Master/Stem/Bounce/Freeze Recipe・Project Commit | 独立 chunk 並列解析・f64 Dataset/export 契約（#139/#143） |
| UI / Job / Probe | Generic UI Host/Dock/Binary Data・Generic Probe Evidence・長寿命 Job Primitive（#181/#182/#185・GUI lane #384〜#390） | Application/Project JobRegistry・Workspace/Inspector・Theme（#175〜#177） | 単一 Window/Close/cancel 挙動・GUI/CLI headless 分離（#166） |
| Diagnostics | Generic Counter/Probe/Frame/Generation/Loss Evidence（#185・Observability #275/#276/#280） | 原因分析/Provenance/Signal Flow UI/Configured vs Inferred | 解析 provenance/validity/confidence の製品意味論 |
| Compatibility / Gate | 互換 API・影響判定の共通語彙・Cross-Repo CI 基盤（#219/#294） | consumer-verify 独立実施（#28・#83） | consumer-verify 独立実施（#127・#140〜#142） |

**非目標（3 者共通）**: Ruviform へ Synvelis の
Project/Track/Take/Comp/Automation Write/Undo/Export
Recipe、AudioLens のゲーム解析意味論（Locate 合議・
段階解析・Validity/Confidence/Dataset）を導入しない。
Plugin ABI・Audio Device・OS/UI 具象実装を Core の必須
依存にしない。

## 正本ドキュメント体系

| 層 | 正本 | 状態 |
| --- | --- | --- |
| umbrella 設計 | `docs/designs/synvelis-driven-target-architecture-and-migration.md` | approved（2026-11-01） |
| 実行計画 | Issue #193（M0〜M7・G0〜G7・GUI lane・consumer 別受入表） | OPEN（進行管理） |
| 境界 ADR | `docs/adr/synvelis-driven-generic-execution-boundaries.md` | accepted |
| 旧 baseline | `docs/designs/ruviform-architecture-design-baseline.md`・`docs/designs/shared-foundations-and-modularization-policy.md` | approved（履歴 Baseline・補足関係） |
| 本書 | `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md` | tracking canonical（本 PR で新設） |

## Capability → 正本・証拠 map（2026-11-09 snapshot）

「merge 証拠」は #193 進捗表の記録と git log の merge
commit で確認したもの。「未解決」は Issue OPEN または
gate 未受入を示す。領域別の詳細は各正本を参照。

| Stage / 領域 | Issue | 正本 docs（代表） | merge 証拠 | 未解決・残件 |
| --- | --- | --- | --- | --- |
| M0 互換 Baseline | #194 CLOSED | `docs/discovery/issue-194-m0-baseline-audit.md` | PR #200（#193 記録）・commit `2ad60a4b` | G0 監査条件充足済み。CI 自動 gate は #201/#294 系で継続 |
| M1 Typed Graph IR | #184 CLOSED | `docs/specs|designs|plans|tasks|test-specs/issue-184-typed-multi-bus-*.md` | PR #225（`8ad88f51`） | 全 Consumer topology/PDC の実測・実 Plugin 統合（SV #106） |
| M2 Audio Runtime | #195 CLOSED | `docs/*/issue-195-m2-typed-graph-*.md` | PR #227（`85f5caba`） | RT 予算・実機/Plan activation の最終受入 |
| M3 Time/Event | #189/#190/#188/#186 CLOSED | `docs/*/issue-189-*`・`issue-190-*`・`issue-188-*`・`issue-186-*` | PR #226/#228/#234・#236・#252 | #253（OS MIDI/VST3 backend）OPEN |
| M4 Plugin Adapter | #196 CLOSED（旧 #113 は参考） | `docs/*/issue-196-typed-graph-plugin-adapter-*.md` | PR #232（`1305538c`） | 実 Backend/Plugin 互換の受入継続 |
| M5 Device/Reader | #187/#197 CLOSED | `docs/*/issue-187-*`・`issue-197-*` | PR #231・#250（`4b311476`） | #251（実 Audio Input）OPEN |
| M6 Output/Offline | #191/#198 CLOSED | `docs/*/issue-191-*`・`issue-198-*` | PR #229・#235（`322a661a`） | OS/FS/Plugin/output edge の全検証継続 |
| M7 UI/Probe | #181/#182/#185 CLOSED | `docs/*/issue-181-*`・`issue-182-*`・`issue-185-*` | PR #237・#240・#230（`d27b9572`） | #243（実 GUI 検証）・#262（Native 採否）OPEN。GUI lane は別表 |
| GUI Driver lane | #384〜#390・#464・#465 | `docs/designs/issue-384-*`〜`issue-390-*`・`issue-464-*`・`issue-465-*`・ADR #384 | #384/#385/#386/#387/#390 CLOSED（#388/#389/#464/#465 OPEN） | #262 採否判定・#465 WebView 撤去完了が最終 gate |
| Cross-Consumer 基盤 | #219 OPEN・#281 OPEN・#294 OPEN | `docs/specs/issue-219-consumer-compat-contract.md`（proposed）・#288 design approved・#294 design 群 | PR #224（#219 CI 基盤）・#274（consumer-verify HTTPS） | 両 Consumer 独立検証の完了・#294 tuple gate 実装 |
| IO 共通基盤（consumer 接続で消費） | #282 CLOSED・#283 CLOSED・#284/#285/#286 OPEN・#287 CLOSED・#288 OPEN・#295〜#306・#409 CLOSED | `docs/*/issue-28x/29x/30x/40x-*.md`（IO-01〜IO-06・per-format・E2E・rooted） | 実装 #295〜#304 CLOSED・E2E #305/#306 OPEN・#409 CLOSED | #284/#285/#286/#288 の設計 gate 受入・#305/#306 E2E 完了・consumer 実測 |
| Observability | #275/#278/#281 OPEN・#276/#277/#279/#280 CLOSED | `docs/designs|specs|test-specs|plans|tasks/issue-275-*`〜`issue-280-*`（#278 は slice 2 まで merge 済み） | PR #519〜#523（#275 統合・#276 logging・#277/#279 core/plugin source・#278 tracing export・#280 RT Record） | #281（OBS-06 cross-consumer gate）OPEN |

## TA-001〜TA-012 受入 gate 対応

umbrella Issue #192 の受入条件と、#193 G0〜G7・関連 Issue/
正本/検証レーンの対応。証拠状態は 2026-11-09 snapshot。
「merge 済み」は headless/CI/PoC 層の実装が main にある
ことを示し、Consumer 受入・実機検証の完了を意味しない。

| Gate | 要求（要約） | 担当正本・Issue | 証拠レーン | 現状（2026-11-09） |
| --- | --- | --- | --- | --- |
| TA-001 | Cargo 通常依存 DAG 非循環・dev/optional/feature 別記録 | #194 M0 audit・#193 G0・CI（#201/#294） | `cargo metadata` 監査 + CI lane | M0 で通常 path 67 edges/循環 0 を記録（#194 CLOSED）。継続監視は CI 依存 |
| TA-002 | Generic Foundation/Graph IR に Synvelis Project/Take/Undo を持ち込まない | #184・#195 設計・`synvelis-driven-generic-execution-boundaries` ADR・#464 consumer-neutral contract | 設計レビュー + 型/依存監査 | 設計・実装 merge 済み。継続的な混入防止は review/CI 責務 |
| TA-003 | Device/Host 非依存で Graph Topology/Port/Signal 検証 | #184（M1）・PR #225・SV #106（lowering） | Mock/headless test + consumer 統合 | 実装 merge 済み。全 Consumer topology の実測は未完 |
| TA-004 | Fan-out/Fan-in・Sidechain・Channel Mapping・PDC・Buffer Lifetime | #184/#195・PR #225/#227・#193 G1/G2 | unit/integration test + RT stress | headless PoC merge 済み。実機 RT 予算は未受入 |
| TA-005 | Plugin Host が Audio Engine を通常依存せず Adapter が正方向 | #196・M0 audit（plugin-host→audio は dev-only）・#195 | 依存 DAG 監査 + adapter test | M0 で dev-only 依存を確認済み。#196 merge 済み |
| TA-006 | Stateful Plugin/DSP が旧/新 RT Plan から同時実行されない | #195/#196・`plugin-rt-ownership-split` ADR・`audio-engine-snapshot-swap` ADR | ownership/state test + 実 Plugin 検証 | 単一 RT Owner 設計・実装 merge 済み。実 Plugin での最終確認は継続 |
| TA-007 | RT NoAlloc/NoBlocking/NoRTDestructor・Retirement/Overflow 保証 | `realtime-contract-and-verification-policy` ADR・rt crate・RT verification lane（`docs/development-guidelines.md`） | RT test lane + stress + review ゲート | lane/test merge 済み。実機 RT 認証は継続項目 |
| TA-008 | Plan/Transport/Target/Instance Generation 跨ぎの Event 誤配送なし | #186/#188/#190・PR #226/#228/#234/#236/#252・Synvelis v0.5 §14 契約 | generation/offset test + TT-025〜030 | 実装 merge 済み。Plugin min_frames 問題は #190/Synvelis 側で追跡 |
| TA-009 | 可変 PPQN・Tempo・Loop Span → Sample Frame/Event の精度・順序維持 | #189・`configurable-ppqn-resolution-independent-musical-time` ADR・PPQN-01〜12 | math/precision test + CLAP/VST3 adapter PoC | #189 merge 済み（旧 960 固定から移行）。Adapter 丸め PoC は #189 系で継続 |
| TA-010 | PCM WAV 実 Encode→Drain→Mux Finalize→再 Open 検証 | #191・#198・IO #287/#305/#306・SV #25（E2E-L2 CLOSED） | encode/finalize/reopen test + E2E | #191/#198 merge 済み。SV #25 で Export E2E-L2 完了。IO E2E #305/#306 は OPEN |
| TA-011 | Window Close で Application/Project-owned Job が無断 Cancel されない | #181・#182・PR #237/#240・job lifetime 契約 | window/job lifecycle test | 実装 merge 済み。実 GUI での最終確認は #243 に連携 |
| TA-012 | Audio/Video/Plugin/CLI/UI/Legacy Consumer が互換 API で回帰 | #219（spec proposed）・#294・#281・consumer issues（SV #28・AL #127・AL #140〜#142・SV #83） | consumer-verify CI + consumer repo 回帰 | CI 基盤部分 merge（#224/#274）。両 Consumer 独立受入は未完 |

> **現行 API/Issue/L1テスト対応表（正本）**: 各 TA gate の「現行
> API/crate・設計判断・実装 Issue・既存 OS-L1 テスト・未達
> Headless Gap → Open Gate」の 1 対 1 対応付けと、feature 別依存の
> 非循環性の静的確認（`cargo metadata` 実測）は
> [`docs/discovery/issue-192-ta-invariant-map.md`](../discovery/issue-192-ta-invariant-map.md)
> を正本とする（2026-11-14 時点の 42 member・循環 0 の実測を含む）。
> 本表は 2026-11-09 snapshot のまま維持し、対応表は受入管理の索引で
> あって invariant の再定義ではない。

## G0〜G7・E2E・GUI lane の実行構造（#193 現行版）

- **G0〜G7**: #193 が各 M stage の受入 gate を定義する
  （G0=baseline 監査、G1=Graph、G2=RT、G3=Time/Event、
  G4=Plugin、G5=Device/Reader、G6=Offline/Output、
  G7=Cutover）。2026-11-02 追補で **各 Gate に両
  Consumer 別の証拠**（impacted/no-impact/unknown・
  pass/fail/not run・resolved revision・Runner/OS/
  feature）が追加要件となっている。
- **E2E 構造**: F0 shared foundation（Project Controller/
  Stable ID/Save/Reload）の後、E2E-L（WAV Playback/
  Export）と E2E-R（Mock RT Capture/Staging/Recording
  Commit）は並列、完了後に E2E-X で cross-integrated
  検証。M0〜M7 は能力移行単位であり E2E を直列化する
  作業順序ではない。
- **GUI lane（2026-11-07/08 追補）**: 既存 M0〜M7 とは
  独立の GPUI Driver 移行 lane。Start（#384〜#390 の
  driver-neutral 基盤）→ Integration（#388/#389/
  #464）→ Acceptance（#262 採否・#465 WebView 撤去・
  両 Consumer #175〜#177/#166 の受入）。旧 WebView 路線は
  移行前の歴史であり、Code を一度に捨てない段階的廃止。
- **Close 条件**: #193 は「設計完了だけで実装 Issue を
  Close しない」「全実装の完了宣言には G0〜G7 の該当
  証拠と Synvelis 統合結果が必要」と規定する。

## ADR・互換・Rollback policy（2026-11-09 時点の反映）

umbrella 設計 §8 の ADR 表を現状へ更新した対応。umbrella
文書本文は approved のまま維持し、本表は追跡上の現状
記録である（silent override ではなく状態の明示）。

| ADR | umbrella §8 の扱い | 2026-11-09 status / 現状 |
| --- | --- | --- |
| Realtime Contract / RT ownership | 維持。新 Processor/Scheduler にも適用 | accepted。#195/#196 で typed runtime/plugin adapter に適用済み |
| Plugin RT Ownership Split | 維持。Host と Audio 接続は Port Adapter | accepted。#196 で具現化 |
| Audio Snapshot Swap | 維持・再評価。方式変更時のみ後継 ADR | accepted。現行 `RwLock<Arc<Plan>>`/`try_read` は存続。typed runtime は互換 path と並行。置換には後継 ADR が必要（未発生） |
| Media I/O Backend Abstraction | 維持。WAV 実 Backend で必要な API 変更のみ判断 | accepted。IO-05（#287）で旧 `StagedOutput`/`PublishOptions` を互換 Facade なしに削除する breaking change が合意・実装進行（#305 OPEN）。Consumer 移行は AL #143 CLOSED・SV #84 OPEN |
| Variable PPQN successor | Accepted のまま実装移行（#189） | accepted → **#189 CLOSED で実装済み**（Rational Beat/Validated Ppqn/TickPosition/Beat 基準 TempoMap、PR #226） |
| UI WebView Hybrid | 維持。Window/Job Scope の PoC 結果で補完 | **superseded-by `driver-neutral-gui-gpui-driver-webview-first-issue-384`（proposed=仮決定）**。GUI lane が #193 に追加済み。#262 の採否 PoC evidence が最終判定 |
| NLE Editing Model | 維持。Synvelis Track/Comp を持ち込まない | 変更なし（`ruviform-edit` は NLE model のまま） |
| General Architecture Baseline | 目標 Design/Accepted boundary ADR で補足 | `ruviform-architecture-design-baseline.md` は履歴 Baseline として維持 |
| synvelis-driven-generic-execution-boundaries | Accepted boundary ADR（一般原則のみ規定） | accepted。未実証の具体方式を Accepted と誤認させない原則を保持 |
| musical-time-model（旧） | —（umbrella 記載なし） | superseded（歴史的本文を保持）。CLAP Fixedpoint 2^31 の Erratum 追記済み（#189 系）。後継は `configurable-ppqn-…` |
| driver-neutral-gui（#384） | —（2026-11-01 以降の追加） | **proposed（仮決定）**。GPUI 標準 Driver 方針・WebView-first 後継。#262 evidence で最終承認 |

**Rollback / Recovery / Incremental Adoption（umbrella §9 の
継続原則）**:

- 新 Graph IR/Typed Runtime は Mock device/feature-gated
  新 API として独立実装し、旧 Graph/Session/PlanSwap を
  急に削除しない。
- NonRT Prepare 失敗時は Active Last-good Plan を維持し、
  Requested/Prepared/Applied/Rejected を区別する。
- Plugin State/Native Source/Project Saved Revision は
  Migrated state と別保存し、旧 Source Snapshot を直接
  破壊しない。
- Retire Queue Saturation・Stale Event・Clock Epoch 断絶・
  File Finalization 失敗は明示 Fault/Degraded とし、RT
  安全な Failure と観測可能な State を分離する。
- 互換 API の deprecation は Consumer 採用と実テスト後。
  未検証の設計を後方互換破壊で強制しない。

## Consumer 接続面（Synvelis / AudioLens）

### Synvelis（DAW Consumer）

| Issue | 状態 | Ruviform 側の接続正本 |
| --- | --- | --- |
| #17〜#21 | OPEN（設計領域） | umbrella 設計・#193・領域別正本（#184〜#198） |
| #22 | OPEN | Target v0.7.1 正本・本書 |
| #23 | OPEN | F0/E2E-L/R/X 実行計画との照合 |
| #25 | CLOSED | E2E-L2 Master Sequential Offline PCM WAV Export・Reopen/Verify（TA-010 の consumer 証拠） |
| #28 | OPEN | consumer-verify 独立 CI・継続追従（#219/#294 に接続） |
| #82 | CLOSED | SYN-IO-01 Ruviform 共通 I/O 適合境界（Project Snapshot/HEAD/Backup/Import）→ IO #282/#283/#284 正本 |
| #83 | CLOSED | Synvelis 独立 PR/Push CI（DAW/Recording/Gateway/RT/Consumer 回帰 gate） |
| #84 | OPEN | SYN-IO-02 DirectoryBundleStore・Snapshot/HEAD → `StagedFile` 移行（IO-05 #287 breaking change の consumer 側対応） |
| #106 | OPEN | SF-03 Semantic→Ruviform Typed Graph Lowering・Revision Map（TA-003 の consumer 統合） |
| #109 | OPEN | SF-06 Routing/Plan/Probe/GUI Cross Gate（#219/#294・#281 に接続） |
| #175 | CLOSED | GPUI Native Desktop 移行（GUI lane Start） |
| #176/#177 | OPEN | 実 Native Plugin Dock・Theme/Workspace Preset（GUI lane Acceptance） |

### AudioLens（高精度解析 Consumer）

| Issue | 状態 | Ruviform 側の接続正本 |
| --- | --- | --- |
| #95 | CLOSED | Ruviform 基盤化の移行 umbrella（責務境界・段階移行・検証） |
| #127 | OPEN | 継続追従・高精度解析/GUI/CLI Consumer 回帰（#219/#294 に接続） |
| #139 | CLOSED | AL-IO-01 Manifest/Dataset/Archive 要件と回帰条件（IO #282/#284 正本へ接続） |
| #140 | CLOSED | Cargo.lock 正本への Git 依存移行（#192 追補の「SHA 固定で凍結しない」原則の実装） |
| #141 | CLOSED | Export 絶対パス拒否 bug（cross-platform path sanitization） |
| #142 | CLOSED | PR/Push 独立 CI・UI/Golden 回帰 gate |
| #143 | CLOSED | AL-IO-02 fsutil・Manifest/Dataset → `StagedFile` 移行完了（IO-05 consumer 側） |
| #166 | CLOSED | GPUI Native GUI 移行・DTCG（GUI lane consumer 側完了） |

### Consumer 境界の共通ルール（#192 2026-11-02 追補・#219）

- Ruviform の変更/PR ごとに両 Consumer を独立に
  `impacted / no impact / unknown` とし、根拠・検証
  コマンド・Runner/OS/feature・resolved revision・結果を
  記録する。片方の pass でもう片方を pass と推定しない。
- schema/version/revision/precision/DAW 意味論は各
  Consumer が所有し、Ruviform は汎用能力のみを提供する
  （IO #284 の「schema は Consumer 所有」原則と同型）。
- Git 依存は固定 SHA で開発を凍結せず、Cargo.lock の
  resolved commit と CI 再現性を保ちながら継続追従する。
- 影響がある場合のみ各 Consumer repo に対応 Issue を起票
  し相互リンク。影響なしは根拠を記録する。

## Data and Control Flow

本書（tracking canonical）における情報の流れ:

1. **入口**: Issue #192（受入管理）→ spec（何を・なぜ・
   範囲）→ 本 design（対応索引と現状）。
2. **gate 解決**: TA-001〜TA-012 または G0〜G7 の各項目
   → 「担当正本・Issue」列で領域別正本（`docs/*/issue-*
   .md`）と実装 Issue へ遷移 → 各正本の test-spec/impl
   記録で証拠を確認。
3. **consumer 接続**: Consumer Issue（SV/AL）→ Ruviform
   側は #219/#281/#288/#294 の gate と領域別正本 →
   consumer-verify 証拠の送付先を特定。
4. **状態判定**: 「merge 済み（headless/CI/PoC）」と
   「Consumer 受入・実機検証」を別状態として読み、
   未検証を pass とみなさない。

## Non-Functional Constraints

- **Snapshot の日付管理**: Issue/PR/文書 status は変動
  するため、本書の状態記述は必ず「2026-11-09 時点の
  snapshot」と明示し、恒久的事実として扱わない。
- **参照整合性**: 記載する Issue 番号・PR 番号・docs
  パス・crate/機能名は作成時点で実在するものに限定し、
  存在しない参照を根拠にしない。
- **権威の非侵害**: umbrella 設計文書・#193・領域別正本・
  ADR の設計判断を本書で再決定・改変しない。差異を発見
  した場合は参照関係の修正か別 Issue で扱う。
- **正直な未検証記録**: 実施していない検証・受入を実施済み
  と報告しない。`not run`/`NotRun`/`Unsupported` を許容
  する。

## Design Done / Implementation Done の分離

- **Design done（本 slice）**: spec/design/test-spec/plan/
  task の docs チェーンが作成され、TA gate・M0〜M7・
  Consumer 接続・ADR/互換の対応索引が文書化される。
- **Implementation done（対象外・継続）**: TA-001〜012・
  G0〜G7 の正式受入、両 Consumer の独立検証、実機/実
  Plugin/実 GUI の受入は #193・#219/#281/#288/#294・
  各 Consumer Issue が Owner であり、本 slice では
  `NotRun`/未受入として記録する。
- **判定原則（#192 本文より）**: merge/PoC/単体 CI 成功 ≠
  全 OS 実機検証 ≠ 両 Consumer 受入。AudioLens の f64
  精度・観測事実、Synvelis の RT/PDC/DAW 意味論を個別に
  検証し、失敗を基盤側へ無条件転嫁しない。

## Risks and Trade-offs

- **文書 drift**: snapshot 型の対応表は Issue 状態の変動で
  陳腐化し得る → 日付明示・参照は正本へ誘導・test-spec
  で参照実在性を検証。
- **二重権威**: umbrella 設計文書と本書の役割混同 →
  「設計権威 vs tracking canonical」を Context と
  Non-Functional Constraints で固定。
- **過剰な網羅性**: 領域別詳細を本書へ複製すると正本が
  分散 → 対応索引に留め、詳細は各正本へのリンクで代替。
- **GUI lane の並行管理**: M0〜M7 と独立 lane の併存で
  全体像が複雑化 → #193 の lane 構造をそのまま記述し、
  独自の再編をしない。

## Deferred Design Concerns

- Consumer 実測・consumer-verify tuple run・実機 OS での
  受入 | cross-repo gate（#219/#281/#288/#294）と各
  Consumer Issue が Owner | Consumer 候補 tuple の実走・
  Consumer repo の受入実施時 | 設計どおりの互換性が実測で
  裏付けられないまま採用されるリスク。
- GUI lane の最終採否（#262）・Legacy WebView 撤去完了
  （#465） | #193 GUI lane と各 Issue が Owner | #262 の
  採否判定と #465 の撤去完了時 | WebView/GPUI 併存の
  長期化による二重保守リスク。
- TA gate の正式 Close 判定と #192 自体の Close | #192
  Owner と gate 証拠の棚卸し時 | 各 gate の証拠が揃った
  時点 | 証拠未完のまま Issue を実装完了扱いで閉じる
  リスク。

## Verification Notes

- [ ] 本書の参照先（Issue/PR/docs パス/ADR）が作成時点で
  実在すること。
- [ ] TA-001〜TA-012 の全項目が「担当正本・Issue・
  証拠レーン・現状」を持つこと。
- [ ] M0〜M7・GUI lane・IO/Observability・Cross-Consumer
  の各領域が正本 docs パスと状態へ対応付けられている
  こと。
- [ ] umbrella 設計文書・#193 が権威であり本書が
  supersede しない旨が明記されていること。
- [ ] ADR 表の status が front matter の実値
  （accepted/proposed/superseded）と一致すること。
- [ ] 「設計合意と実装/検証の分離」と未実施項目の
  `NotRun` 記録方針が明記されていること。
