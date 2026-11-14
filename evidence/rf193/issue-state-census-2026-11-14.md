# Issue 状態 census — Issue #193 Migration Plan Snapshot 裏付け（2026-11-14）

対象: [Issue #193](https://github.com/scarecrow173/ruviform/issues/193)
残件「版付き Snapshot・DAG 矛盾解消・Owner/Gate 確定」の証拠記録。
計画正本は `docs/designs/issue-193-migration-execution-plan.md` と
`docs/plans/issue-193-migration-execution-plan.md` のまま。本書は
Snapshot 記述の根拠となる Issue 状態の実測記録のみを行い、
Issue 本文・計画本文・GitHub 依存関係（blockedBy）は変更しない。

## 収集方法と条件

- 実施日: 2026-11-14（ローカル JST）
- 環境: Windows 11 x64 / Git Bash / `gh` CLI
- コマンド（補助証拠。canonical な検証入口は `mise.toml` タスク。
  本タスクは文書のみの変更であり、Issue 状態照合に `gh` を使用）:
  - `gh issue list -R scarecrow173/ruviform --state all --limit 1000 --json number,title,state`
  - `gh issue list -R scarecrow173/synvelis --state all --limit 500 --json number,title,state`
  - `gh issue list -R scarecrow173/audio-lens --state all --limit 500 --json number,title,state`
  - `gh issue view <N> -R <repo> --json number,title,state,body`（Blocked by 節の照合対象）
- crate 依存 DAG の照合は対象外: M0（#194, CLOSED）が
  `cargo metadata` による棚卸しを完了済みであり、本 Snapshot の
  DAG は Issue 間の計画依存を対象とする。
- 再実測（同日追記）: 初回 census・PR #615 merge 後の
  同日再実測（`gh issue list --state all` 3 repo 全件照合）で
  RF #243・#313 が CLOSED（COMPLETED）へ遷移したことを
  確認し、本表の状態を更新した。両 Issue の Close は
  2026-11-13T19:24-19:25Z（JST 2026-11-14 04:24-04:25）で、
  OS-L1 Pass・Windows 利用範囲 GUI-L2 Pass の
  現行スコープ Closeout（各 Issue 本文の 2026-11-14 節）。
  未実測の Deferred 能力は各製品 Owner Issue
  （SV #114・AL #127・RF #294 等）で追跡される。

## Ruviform Issue 状態（計画関連分）

| Issue | 役割 | State (2026-11-14) |
| --- | --- | --- |
| #193 | 本計画 tracking Issue | OPEN |
| #192 | umbrella Target Architecture v0.7.1 | OPEN |
| #194 | M0 互換 Baseline 監査（G0） | CLOSED |
| #184 | M1 Typed Graph IR/Validator | CLOSED |
| #195 | M2 Audio Compiler/RT Ownership | CLOSED |
| #196 | M4 Plugin Port/Lifecycle | CLOSED |
| #197 | M5 Media Reader/Prefetch | CLOSED |
| #198 | M6 Stateful Offline | CLOSED |
| #199 | M1 着手 Gate lint 緑化 | CLOSED |
| #201 | M1 着手 Gate CI 自動化 | CLOSED |
| #219 | Cross-Consumer 互換契約・Cross-Repo CI Gate | OPEN |
| #243 | GPUI Multiwindow/Dock・Native Surface 実機 GUI 検証 | CLOSED（2026-11-14 再実測。OS-L1 + Windows 利用範囲 GUI-L2 Pass の現行スコープ Closeout） |
| #251 | real backend audio input plumbing・実機検証 | OPEN |
| #253 | OS MIDI backend・VST3 翻訳 | OPEN |
| #262 | GPUI Native First 採用 Gate・OS/性能/互換性評価 | OPEN |
| #275 | Observability 統合設計正本 | OPEN |
| #281 | OBS-06 Cross-Consumer 互換/RT/精度検証 Gate | OPEN |
| #294 | consumer-verify 横断 Gate・候補 SHA tuple | OPEN |
| #306 | IO-IMP-12 共通 I/O 合成 E2E・Fault/Fuzz/OS/FS・Consumer 受入 | OPEN |
| #313 | GPUI Native Desktop/Window Driver・OS Capability Matrix・real-GUI Smoke | CLOSED（2026-11-14 再実測。OS-L1 + Windows 利用範囲 GUI-L2 Pass の現行スコープ Closeout） |
| #388 | NativeSurfaceSlot 契約・GPUI Dock 実 GUI 埋め込み PoC/採用 Gate | OPEN |
| #389 | Plugin Editor Session・L0/L1 Native Dock/Floating/Recovery | OPEN |
| #419 | #262 PoC 判定に基づく後継 ADR 最終採用（#384 残 DoD） | OPEN |
| #465 | Legacy WebView 基盤の段階的廃止・撤去 | OPEN |
| #532 | GPUI 0.2.2 DComp topmost 合成が child HWND plugin 画素を覆う制約 | OPEN |
| #553 | RF-AUTX-01 Consumer-neutral Workflow/CLI orchestration 契約レビュー | OPEN |
| #558 | RF-SCRIPT-00 Script/Analyzer 言語非依存契約 | OPEN |
| #559 | RF-SCRIPT-01 Luau+Rhai 単一バイナリ共存 PoC | OPEN |
| #560 | RF-SCRIPT-02 Luau.Analysis Native Bridge・3OS 配布 PoC | OPEN |
| #562 | RF-CB-01 Generic Alternate Evaluation Context PoC | OPEN |
| #315 | RF-CLK-F01 Multi-device Clock/Drift/Resampling（Future） | OPEN（Deferred/NotStarted） |
| #316 | RF-GRAPH-F01 Explicit Delay Feedback Graph（Future） | OPEN（Deferred/NotStarted） |
| #317 | RF-MIDI-F01 MIDI 2.0 UMP/MPE（Future） | OPEN（Deferred/NotStarted） |
| #318 | RF-CTRL-F01 Audio-rate Control Delivery（Future） | OPEN（Deferred/NotStarted） |
| #280 | OBS-05 RT 固定長 Record Bridge | CLOSED |
| #282 | 汎用 I/O 基盤設計正本 | CLOSED |
| #288 | IO-06 Cross-Consumer・OS/FS・Security・性能・互換性 Gate | CLOSED |
| #293 | OBS-11 Host 統合・Lifecycle・Export・GUI 購読境界 | CLOSED |
| #295 | IO-IMP-01 ruviform-io-file StagedFile | CLOSED |
| #296 | IO-IMP-02 Atomic Publish・Durability・Recovery | CLOSED |
| #305 | IO-IMP-11 旧 StagedOutput 削除・WAV→StagedFile 移行 | CLOSED |
| #307 | PluginNode attach-time frame_base 型付き検証 | CLOSED |
| #308 | Probe Generation 切替再 Binding | CLOSED |
| #309 | 複数 Probe Frame/Clock/Epoch/Latency 整列契約 | CLOSED |
| #310 | RF-GT-01 Browser GUI Test Harness | CLOSED |
| #311 | RF-GT-02 IPC Schema Conformance | CLOSED |
| #312 | RF-GT-03 Test-only GUI Host Lifecycle | CLOSED |
| #314 | RF-GT-05 GPUI Semantic/Visual Golden | CLOSED |
| #384 | GUI Driver-neutral Architecture | CLOSED |
| #385 | DTCG/ResolvedTheme | CLOSED |
| #386 | GUI Host Contract・GPUI Adapter | CLOSED |
| #387 | Component/View Specs・GPUI Controls/Viz | CLOSED |
| #390 | Driver Contract/Fake Driver・GPUI Component Lab | CLOSED |
| #409 | Rooted capability 再 root 化・bounded 列挙 | CLOSED |
| #455 | ViewSpec webview 描画経路方針 | CLOSED |
| #598 | 3OS OS-L1 CI 証拠補完（GPUI macOS Feature 等） | CLOSED |

## Synvelis Issue 状態（計画関連分）

| Issue | 役割 | State (2026-11-14) |
| --- | --- | --- |
| #22 | Target Architecture v0.7.1（Consumer 設計権威） | OPEN |
| #23 | Synvelis Execution Plan v1.0（F0・E2E-L/R/X） | OPEN |
| #28 | Ruviform 継続追従・DAW Consumer 回帰 | OPEN |
| #106 | SF-03 Semantic→Ruviform Typed Graph Lowering | OPEN |
| #107 | SF-04 Semantic Probe Adapter | OPEN |
| #108 | SF-05 Signal Flow Editor | OPEN |
| #109 | SF-06 Routing/Plan/Probe/GUI Cross Gate | OPEN |
| #112 | SV-GUI-03 Core Workspace GUI Slice（GPUI Native 移行） | OPEN |
| #114 | SV-GUI-05 GPUI Native Product E2E・Plugin Dock Gate | OPEN |
| #115 | RCD-01 実 Audio Device 録音統合 | OPEN |
| #117 | RCD-03 Monitoring/Punch/Latency UX | OPEN |
| #176 | SV-GPUI-02 実 VST3/CLAP/AU GPUI Dock 統合・OS 別受入 | OPEN |
| #177 | SV-GPUI-03 Figma/Design Tokens 製品 Theme | OPEN |
| #259 | AUTX-00 Action/Macro/Workflow/Script/Headless CLI 契約 | OPEN |
| #264 | AUTX-05 YAML Workflow DSL v1 | OPEN |
| #266 | AUTX-07 GPUI Action Palette・Macro/Workflow View | OPEN |
| #271 | SV-SCRIPT-01 Gateway Capability→ScriptApiModel | OPEN |
| #272 | SV-SCRIPT-02 Luau Action Script・Atomic Recipe | OPEN |
| #273 | SV-SCRIPT-03 YAML/Luau/Rhai Workflow 共通 Plan 化 | OPEN |
| #274 | SV-SCRIPT-04 GPUI Script Editor | OPEN |
| #277 | CB-00 Creative Branch 統合設計 | OPEN |
| #278〜#288 | CB-01〜CB-11 Creative Branch 実装・Cross Gate | OPEN |
| #289〜#296 | RCP-00〜RCP-07 Render Recipe/Checkpoint | OPEN |
| #84 | SYN-IO-02 DirectoryBundleStore→StagedFile 移行 | OPEN |
| #101 | PC-C4 実 Plugin Parameter Binding | OPEN |
| #119 | RCD-F01 異 Clock Device 同時録音（Future） | OPEN（Deferred） |
| #124 | CTL-F02 Advanced Solo Safe・Delayed Feedback（Future） | OPEN（Deferred） |
| #126 | TIM-02 MIDI 1.0/SMF Import/Export・Native Timestamp/Chase | OPEN |
| #127 | TIM-03 Plugin Process 境界・Loop/Tempo/Automation 精度 Capability | OPEN |
| #130 | TIM-F02 MIDI 2.0/UMP/MPE（Future） | OPEN（Deferred） |
| #138 | AUT-F04 Audio-rate Modulation（Future） | OPEN（Deferred） |
| #232 | Synvelis 側 legacy WebView 物理撤去 | OPEN |
| #13〜#16 | VS-A〜VS-D Recording Foundation〜E2E | CLOSED |
| #24/#25 | E2E-L1 Local WAV / E2E-L2 Offline Export | CLOSED |
| #83 | 独立 PR/Push CI | CLOSED |
| #100 | PC-C3 ControlTrace | CLOSED |
| #104/#105 | SF-01/SF-02 Semantic Routing・Commands | CLOSED |
| #110/#111 | SV-GUI-01/02 旧 WebView Shell/View Models | CLOSED（履歴） |
| #113 | SV-GUI-04 GPUI Multiwindow/Dock 統合 | CLOSED |
| #175 | SV-GPUI-01 GPUI Native Desktop 移行 | CLOSED |
| #190 | project length 変更の update_view 接続 | CLOSED |
| #377 | SV-CI-L1-01 3OS CI 受入補完 | CLOSED |

## AudioLens Issue 状態（計画関連分）

| Issue | 役割 | State (2026-11-14) |
| --- | --- | --- |
| #127 | Ruviform 継続追従・高精度解析/GUI/CLI Consumer 回帰 | OPEN |
| #189 | 旧 WebView 物理撤去（al-gui webview・CI/配布） | OPEN |
| #105 | migration/gui Ruviform 基盤 GUI 構築 | CLOSED |
| #131/#132 | Controls/Gesture/Binding・可視化部品導入 | CLOSED |
| #140 | Ruviform Git 依存 Cargo.lock 正本化 | CLOSED |
| #141 | Export 絶対パス拒否 bug 修正 | CLOSED |
| #142 | 独立 PR/Push CI・UI/Golden 回帰 Gate | CLOSED |
| #143 | AL-IO-02 fsutil/Manifest→StagedFile 移行 | CLOSED |
| #166 | AL-GPUI-01 GPUI Native GUI 移行・DTCG Theme | CLOSED |
| #206 | AL-CI-L1-01 GPUI Feature 3OS CI 統合 | CLOSED |

## Blocked by 照合結果（誤記検証）

Snapshot が「実 OS GUI/Device/Plugin Gate を実装 Blocked by
として一律設定しない」ことの裏付けとして、計画に関係する
Open Issue の `### 着手条件（Blocked by）` 節を `gh issue
view` で照合した。照合対象と結果:

| Issue | 照合結果 |
| --- | --- |
| RF #219, #243, #251, #253, #262, #275, #281, #294, #306, #313, #388, #389, #419, #465, #532, #553, #558, #559, #560, #562, #315, #316, #317, #318 | `着手条件（Blocked by）: なし`（#559/#560 は「なし（2026-11-13 ゲート整理 — 着手契約と統合受入の分離）」表記） |
| SV #23, #28 | Stage 3 分離方針を明記（Start blocker は明示欄のみ、L2/L3 は統合・受入側） |
| SV #106, #112, #114, #176, #259, #277 | `着手条件（Blocked by）: なし` |
| AL #127, #189 | Stage 3 分離方針を明記 |

結論: 確認した全 Open Issue で、実 OS GUI/Device/Plugin
Gate は `### 統合・受入で関係する Issue（着手は止めない）`
側に記録され、`着手条件（Blocked by）` は空。「Blocked by
なし」は 2026-11-13 のゲート正規化（POLICIES.md §4:
着手必須依存と統合・受入依存の分離）と整合する。
照合後に CLOSED となった #243・#313 は Closeout により
現行 Required 範囲の受入を完了しており、Blocked by 照合
結果へ影響しない（Closed Issue は着手 Blocker になり得
ない）。

GitHub 正式依存（blockedBy）の登録・削除は本タスクの範囲外
（Coordinator 責務）。本文上の `着手条件` 記述と GitHub
blockedBy の齟齬が見つかった場合は PR 本文「未対応の作業」
へ列挙する — 今回の照合では本文側の一律 Blocked by 記述は
検出されなかった。
