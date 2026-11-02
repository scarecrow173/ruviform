---
id: "SPEC-IM5TwWbpq1rrgX81HVk3bN"
type: "spec"
status: "proposed"
title: "Issue #181 Multi-Window Host — per-Window event & lifecycle contract"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/181"
    - "https://github.com/scarecrow173/synvelis/issues/2"
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
    - "docs/designs/issue-156-gui-host-foundation.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-181-multi-window-inventory.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-181-multi-window-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/182"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/synvelis/issues/21"
    - "https://github.com/scarecrow173/synvelis/issues/22"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  defers: []
  deferred-by: []
metadata:
  gate: "M7"
  issue: 181
---

# Issue #181 Multi-Window Host — per-Window event & lifecycle contract

## What Should Be Built

**DAW 固有機能を持たない、複数 Window/WebView を利用できる再利用可能なホスト基盤**を `ruviform-ui`（抽象契約）と `ruviform-ui-webview`（wry/tao adapter）に実装する。Window の識別・生成・破棄・フォーカス・DPI/モニター・イベント配送を汎用契約として扱い、利用側の Project や View の意味には依存しない。

1. **Window 識別とイベント発信元**: 単一ネイティブイベントループが複数 Window/WebView を管理し、全イベントに安定した `WindowId` の発信元を持つ。`WindowId` は同一セッション内で一意・単調採番・破棄後も再利用しない。
2. **IPC 応答の宛先識別**: `rvf.ipc.v1` の呼出 `id` は各 WebView ローカル採番で衝突しうるため、応答は発信元 Window へ routing する。`WindowId` は wire に載せない（v1 wire 形式不変）。
3. **ライフサイクル抽象化**: create/focus/move/resize/DPI/monitor 情報/CloseRequested/destroy をバックエンド非依存型で表す。wry/tao/WebView2/WebKitGTK/WKWebView 型を `ruviform-ui` 公開面へ漏らさない。
4. **CloseRequested / Destroy 分離**: 終了要求を承認（Approve）・中止（Veto）・延期（Defer）できる契約。サブ Window の Close はアプリ終了・Job 取消を意味しない。最終 Window/アプリ終了ポリシーは consumer が選ぶ。
5. **失敗フォールバック**: Window 生成/表示失敗・WebView not-ready・Window 消失・破棄済み宛先への配送を安全に処理し、プロセスを壊さない。
6. **Job 寿命の分離**: `UiRuntime` の Job 管理は Window 終了と混同しない。Job event は発信元 Window 宛に routing するが、Job の寿命は Window と独立（長寿命 Job は Window Close で自動取消しない）。「Window-owned Job」は opt-in API（`cancel_window_jobs`）で実装する語彙。
7. **単一 Window baseline の維持**: 既存 `host::run` + `UiRuntime::handle` の挙動（CloseRequested→全 Job cancel→exit）は変更しない（AudioLens baseline、追加 gate）。

## Why Now

- Synvelis が Arrangement/Mixer/Inspector の複数ネイティブ Window・複数モニター配置を正式要件とし（Synvelis #2）、#181 がその基盤 Issue。
- Synvelis #21/#22（v0.6 Window Close Contract / v0.7.1 Window-Job Lifetime）と Ruviform #192/#193（Migration M7 gate）で、Window Close と Job Scope の分離契約が合意済み — 具体 API が本 Issue の PoC で確定する位置。
- 現行の単一 Window 前提（`HostEvent` に発信元なし・Close で全 Job cancel）のままでは、サブ Window 閉鎖を安全に表現できない（discovery 棚卸し）。

## Users and Value

- **Synvelis**: 複数 Window/モニター配置・Dock/View 移動（#182 の前提基盤）・Window Close ≠ Job Cancel の分離。
- **audio-lens 等の単一 Window consumer**: 既存挙動の維持（追加 gate: baseline を黙って破壊しない）。multi-window 拡張は additive。
- **Ruviform 自身**: M7 gate（Window/Job Lifetime 実装）の受入証拠。再利用可能 Docking（#182）の前提契約。

## Scope

### In Scope

- `ruviform-ui` への multi-window 契約追加（本 spec の R 群）。
- `ruviform-ui-webview` への wry/tao multi-window host（`webview` feature ゲート）。
- headless/fake-host テスト（複数 Window イベント分離・Close 中止/延期・破棄済み宛先配送・WebView 間相関 ID 衝突）。
- multi-window demo example（実機手動検証用）と既存 `demo_app`/`demo_cli` の非回帰。
- doc チェーン（discovery/spec/ADR/design/plan/test-spec/tasks/impl record）。
- 実機検証の実施範囲と未検証範囲の正直な記録（capability honesty）。

### Out of Scope

- Arrangement、Mixer、DAW の選択状態、Command Router（Synvelis 所有）。
- Dock Tree の具体的な配置・ドラッグ操作・Window 間 View 移動（#182）。
- プラグイン規格ネイティブ GUI の親子関係再設計（#30/#115 の契約と整合）。
- 特定 OS での動作を実機検証なしに保証すること。
- WebView レンダラクラッシュの検出・自動復帰方針（`Destroyed` で終端化する規約のみ）。
- メニュー/トレイ等のシェル機能、in-WebView WebGPU、ネイティブ wgpu サーフェス（#156 と同じ境界）。
- `UiRuntime` の `notify` broadcast 機構（v1 で runtime が通知を自発生成しない既存仕様のまま）。

## Requirements

- R1: `WindowId` — 非ゼロ・同一ホストセッション内一意・単調採番・破棄後も再利用しない安定識別子。Serialize/Deserialize 可能（host-side routing 用、wire には載らない）。
- R2: 全ての Window 由来イベントは `WindowId` の発信元を持つ envelope で consumer へ届く。`Tick` はループ全体共有で Window に紐付かない。
- R3: `HostEvent` へ `Moved{x,y}`（物理座標）・`Destroyed`（破棄終端 — 承認済み close・programmatic destroy・異常 teardown のすべてを含む）を additive 追加。`#[non_exhaustive]` 維持。
- R4: `WindowConfig`（title/論理サイズ/初期 URL/devtools）・`MonitorInfo`（name/物理サイズ/物理位置/scale factor）・`CloseDecision{Approve,Veto,Defer}` のバックエンド非依存型。`WindowHandle` port（eval/resize/focus/create_window/request_close/respond_close/destroy_window/monitors/windows/exit）で backend 型を露出しない。
- R5: `CloseRequested` は `CloseDecision` への要求であり即破棄ではない。`Approve` → 破棄 + `Destroyed` 発行、`Veto` → 取消して Window 生存、`Defer` → close-pending で生存（consumer が後で `destroy_window` または次回要求の承認）。未回答の既定 = `Approve`（その Window のみ — ループ・アプリ終了はしない）。
- R6: アプリ終了・最終 Window ポリシーは consumer 所有 — host は Window 全滅でもループを継続し、`exit()` port でのみ終了する。
- R7: `UiRuntime` に multi-window 経路を追加: `handle_window(window, event) -> Vec<RoutedEvent>`（`RoutedEvent{window: Option<WindowId>, event}`）。`Message`→宛先 = 発信 Window、`CloseRequested`→`lifecycle closing` を宛先 Window へ返すが **Job は自動取消しない**。`Tick`→`drain_jobs_routed()`（Job event は owner Window 宛、owner なしは `None` unrouted）。
- R8: Job の owner は「イベント配送先」としてのみ記録され、寿命制御には使われない。owner Window 破棄後の Job event は宛先不在で配送層が安全に drop（panic/abort しない）。`cancel_window_jobs(window)` は opt-in の Window-scope Job 取消 API。
- R9: `ServiceContext` に `origin: Option<WindowId>` を additive 追加し、Service が発信 Window を識別可能にする。
- R10: `rvf.ipc.v1` wire 形式は不変 — `v:1`、envelope、error codes、`id` 相関規約を変更しない。`WindowId`・`RoutedEvent` は host-side routing のみで serialize しない。wire 変更が不要であることを design/ADR で明示する。
- R11: 失敗規約 — `create_window` 失敗は `Result` エラー（Window は残存しない）。`eval` は fire-and-forget で宛先不明/破棄済み/not-ready でも panic しない（not-ready は bounded 保留 push、drop-oldest）。`resize`/`focus`/`respond_close`/`destroy_window` の不明/破棄済み Window は `UiError` を返す。Window 消失（backend teardown）は `Destroyed` で終端化。
- R12: `ruviform-ui-webview` の multi-window host は 1 `EventLoop` + Window レジストリ + 共有 `WebContext`（単一 UDF — WebView2 シングルトンロック）で構成し、`data_dir` はセッション設定。`WebContext` は全 WebView より長く生存する。
- R13: RT 契約維持 — RT 側から UI/Window 操作を呼ばない。UI スレッド上で同期 I/O・長時間処理をしない（既存 HostHandle 契約の延長）。
- R14: 単一 Window 経路不変 — `UiRuntime::handle`/`dispatch_op`/`drain_jobs`/`poll` と `host::run` のシグネチャ・セマンティクス（`CloseRequested`→全 Job cancel→`closing`→exit）は不変。`demo_app`/`demo_cli` は非回帰。
- R15: 公開 enum は `#[non_exhaustive]`、公開 API に doc、unsafe 不使用、追加 feature なし（`webview` 既存ゲート内に収める）。
- R16: 消費者影響分類（synvelis/audio-lens: impacted/no impact/unknown）を記録。実機未検証範囲は明示的に `unverified` と記録。

## Acceptance Criteria

- [ ] AC-1: `docs/discovery/issue-181-multi-window-inventory.md` が存在し、現行 API の単一 Window 前提と 5 つの実ギャップを実コード根拠付きで記録済み。
- [ ] AC-2: multi-window 契約（WindowId・イベント発信元・close 承認/中止/延期・IPC 応答 routing・Job/Window 寿命分離）が ADR + design doc で確定され、wire 変更なしの明示結論を含む。
- [ ] AC-3: `ruviform-ui` の `window` 契約型（WindowId/WindowEvent/RoutedEvent/CloseDecision/WindowConfig/MonitorInfo/WindowHandle）と `UiRuntime` multi-window 経路（handle_window/drain_jobs_routed/cancel_window_jobs）が実装される。
- [ ] AC-4: headless/fake-host テストが (a) 2 Window のイベント分離（発信 Window のみ宛先）、(b) CloseRequested の Approve/Veto/Defer 全経路、(c) 破棄済み宛先への配送がプロセスを壊さないこと、(d) 複数 WebView での `id` 衝突下の正しい routing、(e) Window Close で Job が自動取消されないこと・Job event の owner routing を検証する。
- [ ] AC-5: `ruviform-ui-webview` に `SessionConfig` + `run_multi`（単一ループ・Window レジストリ・共有 WebContext・close 決定機構）が実装され、`webview` feature 配下で `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` が green。
- [ ] AC-6: `demo_multi` example（2 Window 生成・Window 追加・サブ Window 閉鎖・last-window policy 例）が用意され、実機実行結果は実施範囲のみ `docs/impl` に記録（未検証 OS/条件は明示）。
- [ ] AC-7: 既存 `demo_app`/`demo_cli`・単一 Window テスト（`close_requested_cancels_jobs_and_emits_lifecycle` 等）が非回帰。`mise run fmt-check`/`check`/`lint`/`test` が green。
- [ ] AC-8: 利用側に依存しない API/実装のみであること（DAW/Dock/Project 語彙を含まない）と、消費者影響分類の記録。

## Deferred Design Concerns

- `ruviform-gpu` wgpu サーフェスの multi-window 対応（`NativeSurfaceHost` ポートは既存のまま）。
- Dock Tree・Window 間 View 移動の汎用化（#182 — 本契約合格後に接続）。
- プラグイン GUI 埋め込みの multi-window 親子関係。
- WebView レンダラクラッシュ検出・自動復帰。
- `notify` の per-Window/broadcast 方針（consumer が RoutedEvent で制御する v1 規約の拡張余地）。
- WebView2 UDF の複数プロファイル/セッション分離（共有 WebContext の拡張余地として記録）。
- macOS/Linux 実機回帰・配布物実製作（#156 と同じ deferred 範囲）。

## Sources and Relations

- Issue: ruviform#181（要件正本）、ruviform#156（単一 Window GUI 基盤・design 正本）、ruviform#182/#192/#193（依存・migration gate）、ruviform#219（consumer 影響分類運用）。
- Synvelis: #2（統合設計）、#21/#22（v0.6 Window Close / v0.7.1 Window-Job Lifetime 合意）、#28（受入先）。AudioLens #127（単一 Window baseline 受入先）。
- 棚卸し: `docs/discovery/issue-181-multi-window-inventory.md`（derives-from 元）。
