---
id: "DESIGN-034UCMjL9ps9ffJkQqwDlR"
type: "design"
status: "approved"
title: "Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 設計正本"
created: "2026-11-09"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/synvelis/issues/22"
    - "https://github.com/scarecrow173/synvelis/issues/23"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-193-migration-execution-plan.md"
  implemented-by: []
  depends-on:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
    - "docs/specs/issue-193-migration-execution-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/issue-257-driver-neutral-ui-contract.md"
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    - "docs/designs/issue-384-driver-neutral-gui-contract.md"
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "docs/adr/issue-465-legacy-webview-retirement-status.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/adr/media-io-backend-abstraction.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/musical-time-model.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-193-migration-execution-plan.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-193-migration-execution-plan.md"
    - "docs/test-specs/issue-193-migration-plan-update.md"
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
    - "https://github.com/scarecrow173/ruviform/issues/199"
    - "https://github.com/scarecrow173/ruviform/issues/201"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/251"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/308"
    - "https://github.com/scarecrow173/ruviform/issues/309"
    - "https://github.com/scarecrow173/ruviform/issues/310"
    - "https://github.com/scarecrow173/ruviform/issues/311"
    - "https://github.com/scarecrow173/ruviform/issues/312"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/409"
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/synvelis/issues/12"
    - "https://github.com/scarecrow173/synvelis/issues/13"
    - "https://github.com/scarecrow173/synvelis/issues/14"
    - "https://github.com/scarecrow173/synvelis/issues/15"
    - "https://github.com/scarecrow173/synvelis/issues/16"
    - "https://github.com/scarecrow173/synvelis/issues/21"
    - "https://github.com/scarecrow173/synvelis/issues/22"
    - "https://github.com/scarecrow173/synvelis/issues/23"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "https://github.com/scarecrow173/synvelis/issues/83"
    - "https://github.com/scarecrow173/synvelis/issues/104"
    - "https://github.com/scarecrow173/synvelis/issues/106"
    - "https://github.com/scarecrow173/synvelis/issues/107"
    - "https://github.com/scarecrow173/synvelis/issues/109"
    - "https://github.com/scarecrow173/synvelis/issues/110"
    - "https://github.com/scarecrow173/synvelis/issues/111"
    - "https://github.com/scarecrow173/synvelis/issues/112"
    - "https://github.com/scarecrow173/synvelis/issues/113"
    - "https://github.com/scarecrow173/synvelis/issues/114"
    - "https://github.com/scarecrow173/synvelis/issues/175"
    - "https://github.com/scarecrow173/synvelis/issues/176"
    - "https://github.com/scarecrow173/synvelis/issues/177"
    - "https://github.com/scarecrow173/audio-lens/issues/105"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/audio-lens/issues/131"
    - "https://github.com/scarecrow173/audio-lens/issues/132"
    - "https://github.com/scarecrow173/audio-lens/issues/141"
    - "https://github.com/scarecrow173/audio-lens/issues/142"
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  defers: []
  deferred-by: []
---

# Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 設計正本

## Context

本書は [Issue #193](https://github.com/scarecrow173/ruviform/issues/193)
本文で合意済みの「Migration Execution Plan v1.0 — M0–M7・依存
DAG・受入Gate・互換移行」を `docs/` チェーンへ構造化して記録
する設計正本である。Issue #193 は「実行計画として承認済み」
の状態で Open を継続し、M0〜M7 の計画対象と依存追跡を管理
する tracking Issue である。

**権威関係（固定）:**

- **設計権威**: umbrella 設計文書
  [docs/designs/synvelis-driven-target-architecture-and-migration.md](synvelis-driven-target-architecture-and-migration.md)
  （Target Architecture v0.7.1、approved、Issue #192 由来）。
  目標アーキテクチャ・crate 配置・概念 API・ADR/互換
  policy・TA-001〜TA-012 gate を所有する。
- **実行権威**: Issue #193 本文（本書がその canonical 記録）。
  M0〜M7 の実行順・依存 DAG・G0〜G7 受入 gate・Cross-
  Consumer 受入追加条件・Rollback・GUI lane・進捗/Close
  条件を所有する。
- 本書は #193 本文を supersede せず、領域別詳細設計
  （#184〜#198・IO・Observability・GUI lane 各 Issue の
  正本 docs）の内容を再定義しない。実装・gate 受入・
  Consumer 実測は各 Owner Issue の要件であり、本書の
  対象外である。

**状態分離の原則:** 設計を採用した状態（Design
Approved）、PoC 検証済（PoC Verified）、実装完了
（Implementation Complete）、CI 検証済（CI Verified）、
対応環境検証済（Platform Verified）、移行受入（Migration
Accepted）を別状態として扱う。Closed な旧基盤 Issue
（例: #113/#20/#24/#156）は新能力の実装完了証拠ではない。
GitHub の Issue close や Design Approved を実装完了の
証拠としない。

本書の記述は **2026-11-09 時点の Issue #193 本文 snapshot**
に基づく。Issue #193 は進行中の tracking 正本であり、本文
への追記があれば本書の同期更新が必要である。

## Scope

### In Scope

- M0〜M7 の実行順・成果物/判断・対応 Issue・開始/完了
  境界の表と依存の意味。
- 共通受入ゲート G0〜G7（証拠リンク必須）。
- Cross-Consumer 受入ゲート追加条件（Gate 別 Consumer
  証拠表・運用判定・#219 Owner 境界）。
- 実装者向け前提判断 6 項目、Rollback / 保護対象
  RB0〜RB6。
- Synvelis 側 E2E-L / E2E-R / E2E-X の受入接続。
- GUI Driver 移行 Lane の Start / Integration /
  Acceptance DAG と Owner 表。
- Legacy WebView 撤去の独立 Lane（撤去 Owner = #465）。
- 横断依存正規化 v1.1・Signal Flow generic
  Execution/Probe 所有境界。
- 実装ステータス同期（2026-11-04 snapshot）と G0/M0 の
  証拠状態。
- 進捗・Close 条件と Issue #193 の tracking 境界。

### Out of Scope

- M0〜M7 の実装・PoC・CI 変更・Consumer repo の変更
  （#194〜#198・#219・#294・Synvelis/AudioLens 各 Issue
  の Owner 領域）。
- umbrella 設計文書本文・Issue #193 本文の改訂。
- GPUI 採否の最終判定（#262 の Evidence による #419）、
  Legacy WebView の実撤去（#465 の実施）、各 gate の
  正式 Close 判定。
- コード・テスト・依存設定の変更（本 slice は docs のみ）。

## Components and Boundaries — 本書の責務と権威の分界

### 本書（Execution Plan canonical）の責務

- M0〜M7 の実行順・依存 DAG・受入 Gate G0〜G7 の判定
  条件を、Issue #193 本文どおりに記録し、後続の実装
  Issue が参照する計画正本とする。
- GUI lane・WebView 撤去 lane の Start/Integration/
  Acceptance 先行関係と Owner 表を、実装順序の正本と
  して維持する。
- Cross-Consumer 受入の Gate 別証拠要件と判定運用を
  記録する。

### 権威を持たないもの

- crate 配置・公開 API・概念 API・RT 契約の詳細 →
  umbrella 設計文書と領域別 issue-* 設計正本。
- 個別 Issue（#184〜#198・#385〜#390 等）の実装内容・
  PoC 判断・テスト → 各 Issue とその docs チェーン。
- Consumer 側の schema/revision/precision・DAW/解析
  意味論 → Synvelis / AudioLens 各 repo。
- ADR の採否・status → 各 ADR 文書と Owner 判断。

## M0〜M7 実行順と依存 DAG（Issue #193 本文の正本化）

既存 28 Workspace Crate と Accepted ADR・既存 Consumer の
互換性を維持しながら、Synvelis で必要な汎用 Time / Typed
Graph / RT Ownership / Event / Plugin / Device / Media /
Offline / UI / Probe 機能を**検証可能な順序で**導入する。
全 crate の一括書換えや DAW 固有の Project / Take / Comp /
Undo / Automation Write の Ruviform への混入を行わない。

| 区分 | 成果物・判断 | 対応先 | 開始・完了の境界 |
| --- | --- | --- | --- |
| **M0 P0** | 28 Crate 依存/公開 API/feature/MSRV/legacy Golden Fixture/CI/Consumer 影響の実測棚卸し、変更禁止/後継 ADR 判断表 | [#194](https://github.com/scarecrow173/ruviform/issues/194) | 設計 Baseline 固定 → G0 証拠確認。推測による後続開始を禁止 |
| **M1 P0** | 非 RT Typed Graph IR、Port/Bus/Connection、Validator、crate 配置比較 | [#184](https://github.com/scarecrow173/ruviform/issues/184) | G0 インベントリと API 境界確定 → G1 |
| **M2 P0** | Graph→Audio Compiler、Buffer/PDC、Prepared Schedule、Stateful Processor 単一 RT 所有、Plan Generation/Retire | [#195](https://github.com/scarecrow173/ruviform/issues/195) | Graph 契約+所有権 PoC 後 → G2 |
| **M3 P0** | Rational Beat/Variable PPQN、Transport Span/Applied Receipt、Timed MIDI、Parameter Events | [#189](https://github.com/scarecrow173/ruviform/issues/189), [#190](https://github.com/scarecrow173/ruviform/issues/190), [#188](https://github.com/scarecrow173/ruviform/issues/188), [#186](https://github.com/scarecrow173/ruviform/issues/186) | P0 Time/Span を先行、補助能力は段階化 → G3 |
| **M4 P0/P1** | Plugin multiple ports/Stateful Lifecycle/Event offsets・physical process min/max 互換 | [#196](https://github.com/scarecrow173/ruviform/issues/196)、旧 [#113](https://github.com/scarecrow173/ruviform/issues/113) は参考のみ | G2+G3 の契約を満たす検証済 PoC → G4 |
| **M5 P1** | Audio Input/Duplex Clock/Timestamp/Latency、Generic Media Reader/Prefetch | [#187](https://github.com/scarecrow173/ruviform/issues/187), [#197](https://github.com/scarecrow173/ruviform/issues/197) | Mock 先行 E2E を実 Device に不要依存させない → G5 |
| **M6 P0/P1** | 実 PCM WAV Encode/Mux/Finalize、Stateful-safe Sequential Offline、Safe Output Publish | [#191](https://github.com/scarecrow173/ruviform/issues/191), [#198](https://github.com/scarecrow173/ruviform/issues/198) | E2E-L 向け最低限を先行 → G6 |
| **M7 P1/P2** | Window/Job 寿命分離、Dock/Restore、Generic Probe、Legacy Compat Cutover | [#181](https://github.com/scarecrow173/ruviform/issues/181), [#182](https://github.com/scarecrow173/ruviform/issues/182), [#185](https://github.com/scarecrow173/ruviform/issues/185) | Generic UI/Probe 検証後、互換解除は G7 で個別判定 |

**依存の意味（Issue #193 本文どおり）:**

- M0 の契約/Fixture 先行 → M1 Graph IR → M2 Audio Runtime。
- M3 Time/Span → M4 Plugin Event/Process。
- Media Reader/WAV・Mock Recording・UI は解決済み契約を前提に
  独立作業可能。
- M4 を M1〜M3 の全面完了まで一律停止しないが、関係する
  ABI/Clock/State ownership が未確定の変更を公開しない。
- M0〜M7 は厳格な単一列ではなく依存 DAG を示す
  （umbrella 設計文書 §6 と同じ解釈）。

**確定した独立実装 Issue / Traceability（Issue #193 本文）:**

- [#194 M0 Baseline](https://github.com/scarecrow173/ruviform/issues/194)
  → M1/M2/M3/M4/M5/M6 の変更前証拠。
- [#184 Graph IR/Validator](https://github.com/scarecrow173/ruviform/issues/184)
  → [#195 Audio Compiler/RT Ownership](https://github.com/scarecrow173/ruviform/issues/195)
  → [#196 Plugin Port/Lifecycle](https://github.com/scarecrow173/ruviform/issues/196)。
- [#189 Beat/PPQN](https://github.com/scarecrow173/ruviform/issues/189)
  + [#190 Transport Span](https://github.com/scarecrow173/ruviform/issues/190)
  + [#188 MIDI](https://github.com/scarecrow173/ruviform/issues/188)
  + [#186 Control](https://github.com/scarecrow173/ruviform/issues/186)
  → #196。
- [#197 Media Reader/Prefetch](https://github.com/scarecrow173/ruviform/issues/197)
  + [#191 PCM WAV Output](https://github.com/scarecrow173/ruviform/issues/191)
  + #195 → [#198 Stateful Offline](https://github.com/scarecrow173/ruviform/issues/198)。
- [#187 Audio Input/Duplex](https://github.com/scarecrow173/ruviform/issues/187)、
  [#181 Multiwindow](https://github.com/scarecrow173/ruviform/issues/181)、
  [#182 Dock](https://github.com/scarecrow173/ruviform/issues/182)、
  [#185 Probe](https://github.com/scarecrow173/ruviform/issues/185)
  は別の検証境界。先行 Mock Recording/E2E-L に不要な hard
  dependency を追加しない。

## 共通受入ゲート G0〜G7（すべて証拠リンク必須）

| Gate | 判定条件（Issue #193 本文） |
| --- | --- |
| **G0 / M0** | `cargo metadata` の通常/feature/target 依存、public API/ABI、旧 ADR と移行先、Golden Behavior、CI 現状・不足、downstream Consumer の影響を記録。CI が手動 `workflow_dispatch` 中心であることを確認し、自動 PR Gate は**実装・検証してから**有効と判断する。 |
| **G1 / Graph** | Typed Domain/Direction/Channel/Multiplicity/Capability と Cycle/無効接続を非 RT で検証。暗黙変換なし。合法な既存単一 Bus Graph が Compat 経路で継続動作する（Synvelis EX-001〜006）。 |
| **G2 / RT** | DAG の順序、Buffer lifetime/PDC、無割当・非 blocking・no OS I/O、Single RT Owner、Graph Prepare 失敗時 Last-good、Generation 不一致 Event 拒否、Retire 経路と過負荷 Fault が再現可能（EX-007〜020、TA-001〜012）。 |
| **G3 / Time/Event** | PPQN 480/960/1000/1920 と 1/7 Beat、Tempo/Loop/Seek/負 Pre-roll、Clock Epoch、Transport Command Applied Frame、Note Off/Overflow/Sustain が検証される（PPQN-01〜12、TT-001〜024）。 |
| **G4 / Plugin** | ProcessConfig min/max 厳守。256 Frame callback 内 100 Frame Loop Wrap かつ min_frames=128 を正確性未検証の 100 Frame process へ渡さない。EngineFrame→callback/span/physical-process offset を対応検証。CLAP/VST3 能力差/Unsupported/明示 Degraded を記録（TT-025〜030）。 |
| **G5 / Device/Reader** | Clock の根拠/Confidence と Discontinuity、xrun、Decode/Prefetch underrun を明示し、録音 Lossless 契約と Probe Lossy 契約を混同しない。 |
| **G6 / Offline/Output** | State を持つ DSP/Plugin を無条件に時間並列化しない。Export→drain/finalize→close/sync→reopen/verify までを通し、失敗/取消/容量不足で Completed や破損した公開先を作らない（MR-001〜028 の該当ケース）。 |
| **G7 / Cutover** | 既存 API/Project/他 Consumer の回帰試験、platform evidence、Legacy Deprecation の理由/利用者通知/復旧手順、未検証事項を明記。後継 ADR なしに Accepted ADR と相反する方式へ変更しない。 |

## Cross-Consumer 受入ゲート追加条件（2026-11-02 追補）

本計画は、Synvelis だけでなく AudioLens も対等な Consumer と
する Ruviform 共通基盤計画として運用する。詳細運用と
Cross-Repo CI の実装 Owner は
[#219](https://github.com/scarecrow173/ruviform/issues/219)、
Consumer 追従は Synvelis [#28](https://github.com/scarecrow173/synvelis/issues/28)・
AudioLens [#127](https://github.com/scarecrow173/audio-lens/issues/127)。
本節は G0〜G7 に**追加**する受入条件であり、既存 M0/M1
状態や個別 PoC の Ready/Blocked 判定を変更しない。

### Gate 別 Consumer 証拠（Issue #193 本文の表）

| Gate | Synvelis で守る契約 | AudioLens で守る契約 |
| --- | --- | --- |
| **G0** | Rust/feature/Project と初期 Workspace 依存、Consumer build | f64 精度・media ingest・CLI/GUI feature 分離の既存 Golden/Consumer baseline |
| **G1/G2** | Typed Graph/Port/PDC/RT 所有・Plan generation | 既存 Audio/Graph/CLI 等の利用範囲を棚卸し。影響なしでも API/依存差分の根拠を記録 |
| **G3** | Rational Beat/PPQN/Tempo/Transport/Event | raw PTS/DTS/timebase と media frame を音楽 Beat へ暗黙変換しない |
| **G4** | Plugin min/max process・state/RT lifecycle | Plugin を直接使わなくても共有 crate/API/feature の非退行を確認 |
| **G5** | bounded Reader/Prefetch/RT 消費/underrun、Input Clock | native PCM/元 bit depth・f64・source timestamp/stream/packet provenance・corrupt/gap/silence の区別を維持（[#197](https://github.com/scarecrow173/ruviform/issues/197)） |
| **G6** | stateful-safe sequential offline、WAV drain/finalize→reopen | 独立 chunk 並列解析・f64 Dataset/export/既存 Decode と Golden 非退行（[#191](https://github.com/scarecrow173/ruviform/issues/191)/[#198](https://github.com/scarecrow173/ruviform/issues/198)） |
| **G7** | multiwindow/Job と legacy cutover | 既存単一 window/Close/cancel 挙動、GUI/CLI headless 分離、Stage resume/Manifest（[#181](https://github.com/scarecrow173/ruviform/issues/181)） |

### 運用と判定（Issue #193 本文どおり）

- Ruviform の各変更/PR で両 Consumer を**独立に**
  `impacted / no impact / unknown` とし、対応/非該当の根拠・
  検証コマンド・Runner/OS/feature・Ruviform resolved
  revision・結果（pass/fail/not run）を Issue/PR へ記録。
- 変更が製品側 Adapter を必要とするなら、Synvelis または
  AudioLens に専用対応 Issue を起票して元 Issue/PR と
  相互リンク。共通の欠陥は Ruviform Issue で修正する。
  影響なしの際は不要な Issue を作らない。
- Git 依存は**固定 SHA に更新を凍結する運用としない**。
  既定ブランチの継続的な追従、依存更新 PR で Cargo.lock
  を更新し、lockfile に記録された resolved commit と
  再現性を維持する。監査に使った特定 commit の記録は
  過去の実測証拠として残す。
- CI 基盤 #199/#201 が未検証の間は両 Consumer の
  Cross-Repo checks を「完了済み」と称さない。#219 で
  追従方式と実装・初回実測の証拠を整える。
- 未検証の共通契約がある経路だけ Blocked にし、
  F0/Mock E2E-R/無関係な AudioLens 分析等の独立タスクを
  一括停止させない。反対に片方の pass で他方を pass と
  推定しない。
- **M0〜M7 の実装 Close/受入:** 既存 G0〜G7 条件に加え、
  影響範囲と両 Consumer 別の pass/fail/not run 証拠または
  no-impact 根拠、後続の未解決 Consumer Issue を記載
  する。例外・既知 failure は Owner が明示判断する。

## 実装者向け前提判断（Issue #193 本文の 6 項目）

1. `ruviform-audio/src/graph.rs` の現状は順序
   `Vec<Box<dyn AudioNode>>`、`plan.rs` の
   `Mutex<Graph>`/PlanSwap は現行挙動。設計上の新
   Graph/RT 所有権を「既存実装済み」として扱わない。
2. Plan-owned Processor vs Stable Runtime Slot を**先に
   同一 DSP/Plugin State 継続・切替・Retire・RT Budget
   の PoC で比較**。採用/却下理由と代替案を記録し、
   実装は判断後。
3. Plugin Host→Audio の通常依存サイクルは現時点で確認
   されていない（`ruviform-audio` は host 側
   dev-dependency）。不要な Bridge crate は作らない。
4. Generic Reader/Output API と Provider Capability を
   調べた後、WAV Backend/新 crate の必要性を決める。
5. RT 費用・Callback/Device 精度・API compatibility は
   **未測定を合格扱いしない**。数値閾値は Baseline 測定
   +比較を先に記録する。
6. 複数 Windows の Close は Project/Application Job の
   取消を意味しない。既存 `UiRuntime::handle(
   CloseRequested)` の変更時は既存単一 Window 回帰を
   保つ。

## Rollback / 保護対象（RB0〜RB6）

| ID | 領域 | 保護・復旧方針（Issue #193 本文） |
| --- | --- | --- |
| **RB0** | 設計/API | 新 API は Additive に導入、旧 Adapter/Compat 経路を維持。失敗時は新経路を無効化し既存 Consumer のテストで検証。 |
| **RB1** | RT/Plugin | 切替失敗時 Last-good Plan 継続、Instance 二重駆動・RT 上 Drop 禁止。Retire queue 溢れ時の現行保持/リーク仕様も計測し、安全な回収方法を別に検証。 |
| **RB2** | Time/Serialization | Old-960 の Fixture と Source-native timestamp 保護。不可逆変換は自動適用せず別スナップショットで検証。 |
| **RB3** | Device/Record | Device 再起動・Clock 断絶は通知し、録音欠落を補完した成功として扱わない。 |
| **RB4** | Export | Staging の検証前に最終出力へ publish せず、既存ファイルの上書きは事前同意・復元可能性確認を条件とする。 |
| **RB5** | UI/Job | Window 消失時に長寿命 Job を破棄しない。 |
| **RB6** | Cutover | Legacy removal は利用先、ADR、Golden tests、対応 OS 結果が揃ってから独立判断。Git revert だけでデータ/公開 API が戻ると仮定しない。 |

## Synvelis 側の受入接続（E2E-L / E2E-R / E2E-X）

- [Synvelis #22](https://github.com/scarecrow173/synvelis/issues/22)
  と実行計画を相互参照。共通 F0 の後、**E2E-L: Local
  WAV→Verified Source→Playback→Sequential Offline
  WAV→Reopen** と **E2E-R: Mock Input→RT
  Capture→Journal→Finalize→Idempotent Project
  Commit/Reload** は並列。両方が検証済みのとき
  E2E-X で Recorded Source の再生・書出を行う。
- E2E-R は Ruviform 実 Device Input/Duplex、Plugin 全
  実装、WAV Output Backend 完成を前提にしない。E2E-L は
  実 WAV Output と最低限の Graph/Reader/Offline 経路を
  必要とする。
- F0/E2E の詳細な Gate 構造（F0 shared foundation・
  VS-A〜VS-D owner）は umbrella 設計文書 §10 を参照。

## GUI Driver 移行 Lane（M0〜M7 とは独立・先行関係の正本）

上位 GUI ADR [#384](https://github.com/scarecrow173/ruviform/issues/384)、
UI 設計 [#257](https://github.com/scarecrow173/ruviform/issues/257)、
GPUI 採用/Native Dock Gate [#262](https://github.com/scarecrow173/ruviform/issues/262)/
[#388](https://github.com/scarecrow173/ruviform/issues/388)
を新しい GUI Lane として参照する。関連正本:
[docs/specs/issue-384-driver-neutral-gui-architecture.md](../specs/issue-384-driver-neutral-gui-architecture.md)、
[docs/designs/issue-384-driver-neutral-gui-contract.md](issue-384-driver-neutral-gui-contract.md)、
[docs/designs/issue-257-driver-neutral-ui-contract.md](issue-257-driver-neutral-ui-contract.md)、
ADR `driver-neutral-gui-gpui-driver-webview-first-issue-384`。

### Start / Integration / Acceptance（2026-11-09 確定仕様）

- **Start（Issue 着手条件）**: #384 初期契約と #262 GPUI
  調査は並行可能。#385 DTCG、#386 GPUI 最小
  Window/Host、#389 Editor Session の state/type 設計、
  #388 Native Dock OS 方式調査を同時開始してよい。
  #388 の**実 NativeSlot PoC** は #386 の最小 GPUI
  Window Fixture を必要とするが、#386 の全 Host 完成を
  待たない。
- **Integration（契約接続）**: #385 + #386 の型付き
  テーマ/Host → #387 Component/Viz。#386 最小 Host +
  #388 NativeSurface + #389 Editor Session → Synvelis
  #176。#386/#387 → Synvelis #175/#112、AudioLens #166。
  #390 Fake+Lab / #313 OS / #314 Visual を並行接続。
- **Acceptance（最終統合 Gate）**: #262 GPUI 採否は
  #388/#389 実 Native Dock/OS と #390 の共通 semantic を
  参照して判定。Synvelis #114 は #175/#112/#113/#176/#177
  の製品 GUI 機能を確認。AudioLens #166 の解析/CLI 受入は
  独立し、#219/#294 が実 SHA tuple ごとに両 Consumer を
  判定する。**Issue 着手を #262 の採用完了に依存させる
  循環禁止**。

### GUI lane の制約（Issue #193 本文の受入条件に対応）

- #384 後継 ADR の最終承認は技術採否 PoC の Evidence で
  実施。旧 WebView 路線は移行前の歴史。旧 API/Frontend
  の削除は両 Consumer Native 合格後の明示判断。WebView
  Code を一度に捨てない。
- Start/Integration/Acceptance 以外に暗黙の Issue 依存
  （例えば #107 Probe や旧 SolidJS #111 を GUI Shell 開始
  の hard blocker にする）を新設しない。
- #257/#385〜389 は Ruviform 共通、#175/#112/#113/#176/
  #177 は Synvelis 固有、#166 は AudioLens 固有。
  Driver/Component 二重実装、Cross Consumer CI 重複作成
  禁止。
- 旧 WebView/TS/SolidJS に関する開始依存は**新規 GPUI の
  Hard blocker としない**。既存 Gesture/Window/Dock/IPC
  の確定意味論は保持し、互換移行時だけ技術 Adapter を
  置換。WASM は対象外。M0〜M7 既存業務と GUI Lane を
  不必要に相互ブロックせず、旧 WebView 縮退は両
  Consumer 受入後の別判断。
- Acceptance の証拠レーン: #313/#314/#243 ＋ 実 Plugin
  GUI Dock #388/#176 ＋ 両 Consumer #219/#294 の対象
  能力別証拠。

### DAG と Owner の表（実装順序の正本）

| 領域 | Owner（Issue） | 先行関係 |
| --- | --- | --- |
| Ruviform 共通 GUI 契約・Token・Host・Component・Viz・Fake/Lab | #257, #384, #385, #386, #387, #389, #390 | §Start/Integration のとおり |
| Native Dock OS 方式・実 NativeSlot PoC | #388 | #386 最小 Window Fixture が前提（全 Host 完成は不要） |
| Synvelis GUI Shell/Theme/製品 Dock | #175, #177, #176, #112, #113, #114 | Ruviform #386/#387/#388/#389 の契約接続後 |
| AudioLens Native GUI/CLI/解析 | #166 | Ruviform #386/#387 の契約接続後。解析/CLI 受入は独立 |
| GPUI 採否判定 | #262 → #419 | #388/#389 実 Native Dock/OS と #390 共通 semantic の Evidence |
| 両 Consumer 互換判定 | #219 / #294 | 実 SHA tuple ごとの対象能力別証拠 |
| GUI Test 基盤 | #310/#311/#312 → #313, #314 | GUI #110〜114 の着手を SF/OBS/Plugin 全完成でブロックしない |

DoD（Issue #193 本文）: DAG と Owner の表を実装順序の
正本として維持し、両 Consumer の最新 Issue へリンク。

## Legacy WebView 撤去の独立 Lane（撤去 Owner = #465）

[#465](https://github.com/scarecrow173/ruviform/issues/465)
が撤去 Owner。#384/#257/GPUI 共通実装・両 Consumer 移行と
整合させる。

- **Start（即時）**: #455 不採用 Close、旧 WebView 実装の
  新機能追加停止、legacy 資産・API/テスト/CI 棚卸し。
  GPUI/Native Dock の全合格を待たない。
- **Integration**: 共通契約を WebView/JS/DOM と分離し、
  両 Consumer の GPUI 経路へ既存 semantics/fixtures を
  移す。WebView parity/Browser Playwright を Native の
  必須 Acceptance にしない。
- **Acceptance（破壊的撤去）**: GPUI 採用判定 #419
  （#262/#388/#389）、Synvelis #114/製品 Plugin Dock #176、
  AudioLens #166 の必要能力・独立数値 Oracle と #127 継続
  追従、3 repo 互換 #219/#294。個々の未検証を Pass に
  昇格させず、終了条件を満たした範囲のみ各 repo で撤去。
- #110/#111（Synvelis）、#105/#131/#132（AudioLens）、
  Ruviform #156/#310〜312/#423/#431 は旧実装の Closed
  履歴を保持。製品側コード変更は各 repo の Issue/PR が
  Owner。
- 関連 ADR: `issue-465-legacy-webview-retirement-status`
  （撤去 status の ADR）、`ui-architecture-webview-hybrid`
  （superseded）、`driver-neutral-gui-…-issue-384`。

## 横断依存正規化 v1.1 と Signal Flow 所有境界

### 横断依存正規化 v1.1（2026-11-04）

Synvelis [#23](https://github.com/scarecrow173/synvelis/issues/23)
の正規化 DAG を Consumer 実装依存の正本とする。
Start（着手）/Integration（機能接続）/Acceptance（最終
証拠）を区別し、相互参照だけで Hard Blocker としない。
（RF = Ruviform、SV = Synvelis、AL = AudioLens）

- **Routing/Control**: SV #104 → #121 → #105 →（#106 と
  #122 を並行）。SV #100 と RF #307 → SV #101 は並行し、
  #106/#122/#101/#100 の Active Target/Generation 整合を
  受入 Gate とする。#105 と #121/#122 の旧相互依存は
  開始条件として無効。
- **Probe/UI**: SV #106 → #107（headless）と SV #104/#111
  → #108（read-only GUI）を並行。RF #308/#309 は関連
  Capture/Alignment Capability 別 Gate、SV #109 は Domain
  Oracle、SV #114 は GUI Scenario。
- **GUI Test**: RF #310/#311/#312 が並行先行、#312 →
  #313、#310 → #314。GUI #110〜114 の着手を SF/OBS/
  Plugin 全完成でブロックしない。
- **File Breaking**: RF #295 → #296 契約を前提に RF #305
  候補、AL #143 候補、SV #84 候補を並行作成。RF #305 の
  main 先行マージは禁止要件とせず、RF #294/#306 と
  AL #142/SV #83 の候補 Gate 後に互換ロールアウトを判定。
- 既存 M0〜M7/F0/E2E-L/R/X、マージ済み PoC、両 Consumer
  別の精度/RT/GUI 受入 Gate は変更しない。Issue 更新は
  実装・テスト PASS を意味しない。

### Signal Flow generic Execution / Probe 後続 Issue の所有境界（2026-11-04）

[Synvelis Signal Flow #4](https://github.com/scarecrow173/synvelis/issues/4)
の Ruviform/Synvelis 責務監査の合意に基づく。

- **Ruviform Owner 維持**: 既存 `ruviform-graph`
  Validator/Typed Port/Connection、`ruviform-audio::typed`
  Compiler/PDC/Prepared Graph/Publish/RT
  Activation/Retire/Last-good、RT Probe
  Binding/Frame/Clock/Loss/Resource Bounds。既に
  マージされた M1/M2/M7 の実装を「未実装」と扱わず、
  製品実機・fault の残 Gate を追う。
- **新規 Gap**: [#308](https://github.com/scarecrow173/ruviform/issues/308)
  は `ProbeBroker::on_publish` の `ClearAll`+個別 `Bind`
  の部分 QueueFull 時の all-or-nothing/Partial+Retry、
  世代間 Capture の分断、RT 誤 slot ゼロ。
  [#309](https://github.com/scarecrow173/ruviform/issues/309)
  は複数 Probe 間の Clock Domain/Epoch/Frame/Latency 整列
  可能性と欠測/品質の汎用契約。Domain-specific inference
  は対象外。
- **Synvelis Consumer**: SF-03 #106 が Stable Semantic
  Routing→Generic Graph IR Lowering/ProjectRevision⇄
  Prepared/Published/Active Generation mapping。SF-04 #107
  が Semantic Probe Point/Monitor/Capture UX。Ruviform に
  Track/Bus/Send/Undo を追加しない。
- **受入**: G2/G7 + Synvelis SF-06 #109 の Graph/Probe/RT/
  Plugin 実機、AudioLens の PCM f64/analysis/export 互換、
  [Cross Consumer #219](https://github.com/scarecrow173/ruviform/issues/219)/
  [#294](https://github.com/scarecrow173/ruviform/issues/294)。
  Headless のみは実 OS 受入ではない。Observability
  #280/#292 は Collector/Debug の別正本で汎用 Probe 自体を
  複製しない。

## 実装ステータス同期（2026-11-04 snapshot）

**状態:** M0 と M1〜M7 の複数 headless PoC/個別実装は
main に反映済み。全 Migration/実機/両 Consumer の統合
受入は未完了。旧「M1〜M7 未着手」の一括状態は廃止し、
**Implemented / Tested headless / Cross-Consumer Verified /
Platform Verified / Migration Accepted** を別判定とする。

| Phase | 実装進捗の証拠 | なお必要な Gate |
| --- | --- | --- |
| M0 Baseline | [PR #200](https://github.com/scarecrow173/ruviform/pull/200)（過去完了証拠） | 変更影響ごとの継続比較 |
| M1 Graph | [PR #225](https://github.com/scarecrow173/ruviform/pull/225) | 両 Consumer Routing・Plugin 全能力 |
| M2 Audio Runtime | [PR #227](https://github.com/scarecrow173/ruviform/pull/227) | 実機/State/PDC/RT 時間 budget |
| M3 Time/Event | [#226](https://github.com/scarecrow173/ruviform/pull/226), [#228](https://github.com/scarecrow173/ruviform/pull/228), [#252](https://github.com/scarecrow173/ruviform/pull/252) | [#253 OS MIDI/VST3](https://github.com/scarecrow173/ruviform/issues/253) |
| M4 Plugin | [#232](https://github.com/scarecrow173/ruviform/pull/232) | 実 Backend/Plugin 互換 |
| M5 Device/Reader | [#231](https://github.com/scarecrow173/ruviform/pull/231), [#250](https://github.com/scarecrow173/ruviform/pull/250) | [#251 実 Input](https://github.com/scarecrow173/ruviform/issues/251) |
| M6 Output/Offline | [#229](https://github.com/scarecrow173/ruviform/pull/229), [#235](https://github.com/scarecrow173/ruviform/pull/235) | OS/FS/Plugin/output edge 全検証 |
| M7 UI/Probe | [#237](https://github.com/scarecrow173/ruviform/pull/237), [#240](https://github.com/scarecrow173/ruviform/pull/240), [#230](https://github.com/scarecrow173/ruviform/pull/230) | [#243 実 GUI](https://github.com/scarecrow173/ruviform/issues/243) / [#262 Native 採否](https://github.com/scarecrow173/ruviform/issues/262) |

### 新しい横断受入 Owner（Issue #193 本文）

- [Ruviform #219](https://github.com/scarecrow173/ruviform/issues/219)
  / [#294](https://github.com/scarecrow173/ruviform/issues/294):
  consumer-verify 実走/precision/RT/UI/headless。
- AudioLens [#127](https://github.com/scarecrow173/audio-lens/issues/127)、
  [#141](https://github.com/scarecrow173/audio-lens/issues/141)、
  [#142](https://github.com/scarecrow173/audio-lens/issues/142)、
  [#140](https://github.com/scarecrow173/audio-lens/issues/140):
  Consumer 回帰・Export 不具合・CI・依存管理。
- Synvelis [#28](https://github.com/scarecrow173/synvelis/issues/28)、
  [#83](https://github.com/scarecrow173/synvelis/issues/83):
  Consumer 回帰と独立 CI。
- Observability と共通 IO はそれぞれ
  [#275](https://github.com/scarecrow173/ruviform/issues/275) /
  [#282](https://github.com/scarecrow173/ruviform/issues/282)
  の別 Owner を維持。

Issue は計画正本として Open を継続し、個別 Gate の証拠で
更新する。マージ済み PoC を「未着手」に戻さず、未実測
事項を「完了」にも昇格させない。

### G0 / M0 証拠状態の履歴（Issue #193 本文の snapshot）

- M0 調査報告の正本:
  [docs/discovery/issue-194-m0-baseline-audit.md](../discovery/issue-194-m0-baseline-audit.md)
  （マージ済み PR #200）。固定コミット
  `0cd895b86d8dcff966e0236e8b6bb02f605f4719`（Windows 11・
  mise pin rust 1.99.0）で 28 crates/通常 path 依存の静的
  DAG（67 edges・循環 0・dev 5・plugin-host→audio は
  dev-only）・`cargo test` 1784 全緑・全 feature lane・
  MSRV 1.89・実機 WASAPI 列挙を記録。
- 新規検出: `cargo clippy -D warnings` が HEAD で赤
  （`ruviform-numeric` resample.rs 2 件）。緑化は #199 で
  対応。CI: HEAD run 0 件・icc/ui/l1 lane は定義後未実行・
  main は branch protection 未設定・rulesets `[]`。
- **G0 監査条件は全項充足**（PR #200 レビューで再判定）。
  G0-H は実施済み: 詳細 Issue 12 件へ Ready/Blocked を
  反映（Ready: #184/#185/#186/#188/#189/#190/#191/#197/
  #198、条件付き Blocked: #187/#195/#196）。各「Ready」は
  G0 前提の設計/PoC 着手可能であって全面機能実装 Ready
  ではない。M1 以降の一括 Ready 化は依然行わない。
  **#194 の最終 Close は未観測項目（CI 他 OS lane/実機
  走行等）の扱いと共に Owner 判断待ち**。
- M1 着手 Gate（G0 とは別定義）: ① #199 lint 緑化
  （PR #202）、② CI 自動ゲート整備（#201 / PR #203。
  PR イベントで Actions 起動を検証したが 12 jobs 中
  6 成功/6 失敗のため未解除。解除に必要な修正: #199/
  #202、Linux/macOS OFX C ABI #204/#205、Linux L1 Clippy
  #206、macOS L1 Spawn #207）。未成功の job を成功扱い
  しない。

## 進捗・Close 条件と tracking 境界

- 詳細 Issue ごとに変更した Contract と PoC 判断、実行した
  コマンド/環境/結果、回帰リスト、未検証項目、Rollback
  検証を残す。
- **設計完了だけで実装 Issue を Close しない。**
- 本 Issue（#193）は M0〜M7 の計画対象と依存追跡を管理
  する。全実装の完了を宣言するには G0〜G7 の該当証拠と
  Synvelis 統合結果が必要。
- **開始境界:** Issue #193 の作成・合意済み計画の整理は
  M0 実施ではない。M0 を含む実装/PoC/CI 変更は個別の
  実行開始指示後。

## Data and Control Flow

本書は計画・Gate の canonical 記録であり、実行時のデータ
フローを持たない。計画の「流れ」は以下のとおり。

1. **計画権威の参照**: 実装着手時に umbrella 設計文書
   （設計）と本書（実行順・Gate・Rollback）を併読する。
2. **Gate 証拠の蓄積**: 各 M 段階の詳細 Issue が G0〜G7
   の証拠（コマンド/環境/結果/回帰リスト/未検証項目/
   Rollback 検証）を Issue/PR に記録する。
3. **Cross-Consumer 判定**: 各変更/PR で両 Consumer を
   `impacted / no impact / unknown` に分類し証拠を記録。
   #219/#294 が実 SHA tuple ごとの受入を判定する。
4. **状態昇格**: Design Approved → PoC Verified →
   Implementation Complete → CI Verified → Platform
   Verified → Migration Accepted を別状態として追跡し、
   未検証を合格に昇格させない。
5. **Cutover**: G7 で Legacy Deprecation の理由/利用者
   通知/復旧手順を揃え、RB6 に基づき独立判断で解除する。

## Non-Functional Constraints

- **互換性**: 既存 28 Workspace Crate・Accepted ADR・
  既存 Consumer（Synvelis/AudioLens/cli-sample 等）の
  互換性を維持。新 API は Additive、旧 Compat 経路を
  維持（RB0）。
- **RT 安全性**: RT に Alloc/Dealloc・Blocking・OS I/O・
  非 RT Destructor を持ち込まない（umbrella TA-007・
  ADR `realtime-contract-and-verification-policy`）。
- **証拠主義**: すべての Gate は証拠リンク必須。未測定・
  未実施を合格扱いしない。数値閾値は Baseline 測定+比較を
  先に記録する。
- **非 DAW 化**: Project/Take/Comp/Undo/Automation Write
  等の DAW 固有概念を Ruviform へ混入しない
  （umbrella §2 Non-goals）。
- **docs のみの本 slice**: コード・テスト・CI・依存設定・
  Consumer repo を変更しない。

## Risks and Trade-offs

- **計画と実装の乖離**: Issue #193 本文は進行中で追記
  され得る → 本書を 2026-11-09 snapshot として日付明示し、
  本文更新時の同期更新を義務付ける。
- **状態の混同**: merge 済み PoC を「受入済み」と誤読
  するリスク → Implemented/Tested headless/Cross-Consumer
  Verified/Platform Verified/Migration Accepted の状態
  分離を明記。
- **暗黙依存の混入**: GUI lane や他領域へ hard blocker を
  誤って新設するリスク → Start/Integration/Acceptance の
  3 区分と「新設しない」制約を表で固定。
- **Consumer 非対等**: Synvelis のみを基準に進め AudioLens
  の f64/PTS/GUI-CLI 契約を見落とすリスク → Gate 別
  Consumer 証拠表と独立判定ルールで対等性を担保。
- **未 merge 文書への依存**: 並行 Worker の #192 設計
  チェーン等、未 merge の文書を正本として参照するリスク
  → 本書は merge 済み正本（umbrella 設計文書・#194
  discovery・#219 spec・領域別 issue-* 文書・ADR）のみを
  参照する。

## Snapshot-2026-11-14 — M0〜M7 着手条件・Headless Slice・Owner/Gate 確定（版付き）

**版識別子:** `Snapshot-2026-11-14`（Issue 状態を `gh` で
実測した基準日版）。本節は Issue #193 残件の版付き
Snapshot であり、M0〜M7/G0〜G7/RB0〜RB6・GUI lane・
Cross-Consumer 受入の定義を**変更しない**。既存節は
2026-11-09/2026-11-04 時点の履歴 snapshot として保持
し、本節を最新の状態記録とする。証拠:
[evidence/rf193/issue-state-census-2026-11-14.md](../../evidence/rf193/issue-state-census-2026-11-14.md)。
設計根拠:
[docs/designs/issue-193-migration-plan-update-design.md](issue-193-migration-plan-update-design.md)。

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

## Deferred Design Concerns

| Concern | Reason | Re-engagement trigger | Risk if ignored |
| --- | --- | --- | --- |
| M0〜M7 の実装・各 gate 正式受入・Consumer 実測 | #193・#219/#281/#288/#294・各 Consumer Issue の Owner 領域 | 各詳細 Issue の実行開始指示と Gate 証拠 | 本書を実装完了の証拠と誤認 |
| GPUI 採否の最終判定（#262→#419）と WebView 実撤去（#465） | 受入段階の要件。技術採否 PoC の Evidence 待ち | #388/#389/#390 の Evidence と両 Consumer 受入 | 未検証のまま破壊的撤去 |
| umbrella 設計文書本文への GUI lane/ADR 変更の反映 | approved の設計権威として維持。改訂は別 Issue/Owner 判断 | umbrella 文書の改訂 Issue | 設計文書と実行計画の記述齟齬 |
| Issue #193 本文への今後の追記の docs への同期 | 本書は 2026-11-09 snapshot | #193 本文の更新検知時 | 計画正本の陳腐化 |

## Verification Notes

- [x] Issue #193 本文（2026-11-09 時点）の M0〜M7 表・
  G0〜G7・RB0〜RB6・GUI lane・WebView 撤去 lane・
  Cross-Consumer 受入・進捗/Close 条件・受入条件を本書
  に構造化して記録した（spec R-1〜R-10 対応）。
- [x] 参照先の docs パス・Issue 番号が merge 済み正本に
  実在することを作成時に確認（未 merge の並行 Worker
  文書は参照しない）。
- [ ] `docs/` 以外に差分がないことを `git status`/
  `git diff` で確認（PR 前に実施）。
- [ ] audit_docs で本書の error が 0、docs index 再生成・
  `mise run fmt-check` pass（検証節で実施）。
