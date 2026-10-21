---
id: "SPEC-034Jc9BJlI7ULEBWoIlMP2"
type: "spec"
status: "implemented"
title: "Issue #28 UI 技術検証 PoC（wry・描画バックエンド・フロントエンド FW・配布）"
created: "2026-10-20"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/28"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/poc-adr-finalize.md"
      - type: "doc"
        path: "docs/impl/exp/ac7-adr-finalize.jsonl"
    modified:
      - type: "file"
        path: "poc/ui/RESULTS.md"
      - type: "doc"
        path: "docs/adr/ui-tech-selection-poc.md"
      - type: "doc"
        path: "docs/plans/issue-28-ui-poc.md"
      - type: "doc"
        path: "docs/tasks/poc-adr-finalize.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/issue-28-ui-poc.md"
    - "../plans/issue-28-ui-poc.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/platform-and-toolchain-targets.md"
    - "../adr/licensing-and-distribution-policy.md"
    - "../adr/ui-architecture-webview-hybrid.md"
    - "../adr/ui-tech-selection-poc.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/issue-28-ui-poc.md"
  derived-by:
    - "../designs/issue-28-ui-poc.md"
  verifies: []
  verified-by:
    - "../test-specs/issue-28-ui-poc.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/28"
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# Issue #28 UI 技術検証 PoC（wry・描画バックエンド・フロントエンド FW・配布）

## What Should Be Built

GitHub Issue [#28 [poc] UI 技術検証（wry・描画バックエンド・フロントエンド FW・配布）](https://github.com/scarecrow173/ruviform/issues/28) の成果物として、以下の 2 つを作る。

- **使い捨て PoC 検証ハーネス（Rust + Web フロントエンド）**: `wry` + `tao` による WebView ホスト、WebView 内描画（Canvas2D / WebGL2 / WebGPU）の高頻度更新計測、`wgpu` ネイティブサーフェスと WebView の同居・合成評価、IPC / カスタムプロトコルのメッセージ境界計測、フロントエンドフレームワーク比較、アセット埋め込み単一バイナリ配布を、Windows 11 実機（WebView2 evergreen 155 系）で検証するコード。配置は `poc/ui/` 等の **cargo ワークスペース外ディレクトリ**を想定する（正確な配置・依存隔離の方式は設計段階で決定する設計事項）。計画上の将来クレート `ruviform-ui`（ポート定義）/ `ruviform-ui-webview`（adapter）の実装ではなく、それらの分割を意識した構造で書く検証用ハーネスである。
- **採否 ADR（永続的成果物）**: [UI 技術選定（wry・描画バックエンド・フロントエンド FW・配布）— PoC 採否](../adr/ui-tech-selection-poc.md)。PoC 実施で得たエビデンスにより各採否条項を confirm または revise し、全条項が裏付けられた時点で `proposed` → `accepted` に遷移させる。採否はコンポーネント単位であり、固定フォールバック階層として扱わない（[ui ADR](../adr/ui-architecture-webview-hybrid.md) の確定方針）。

## Why Now

- [UI アーキテクチャ ADR](../adr/ui-architecture-webview-hybrid.md)（accepted）が「WebView ファースト・ハイブリッドを第一候補」とし、wgpu 等の具体採用範囲・フレームワーク・描画バックエンドの最終選択を本 PoC に委任した。[設計ベースライン](../designs/ruviform-architecture-design-baseline.md) の要 PoC 表の「WebView 実装・描画バックエンド採否・フロントエンドフレームワーク・配布形態」の各行が本 PoC の対象である。
- [事前調査（discovery）](../discovery/issue-28-ui-poc.md) がコンポーネント別の現行ファクト・プラットフォーム実情・受け入れ基準候補を収集済みであり、検証を実施段階へ進められる状態にある。
- 検証なしに技術選定を固定すると、後からの PoC 結果を無効にする（ui ADR が「UI 技術の即時確定」を不採用とした理由）。`ruviform-ui` / `ruviform-ui-webview` クレートは未作成であり、実装 Issue 起票の前に実現性と選定根拠を確定させる必要がある。

## Users and Value

- **`ruviform-ui` / `ruviform-ui-webview` / `ruviform-gpu` の将来実装者**: 採否の根拠となる実測エビデンス（フレーム統計・スループット・合成可否）と、外部型を公開境界の背後に置く adapter 構造の検証結果を得る。
- **MSRV 見直しタスクの担当者**: `wgpu` MSRV 1.87 とワークスペース宣言 `rust-version = "1.85"` の衝突という確定事実を、実測付きで引き渡せる。
- **プラグイン GUI 後続 Issue の作業者**: `wxp` 同梱パッチ済み wry・`nih-plug-webview` の評価材料（素 wry で不足するホストライフサイクル対応の具体的中身）を得る。
- **レビュアー・メンテナ**: 採否 ADR がコンポーネント単位の決定構造を持ち、各条項がエビデンスへのリンクで裏付けられるため、決定の追跡・再検討が容易になる。

## Scope

### In Scope

- 使い捨て検証ハーネスの構築（Rust ホスト + Web フロントエンド）。ワークスペース外（例: `poc/ui/`）または `publish = false` の非公開メンバーとしての配置 — 設計段階で決定
- Windows 11 実機での検証全般（WebView2 155 系・Rust 1.99.0 pin toolchain・Node.js 環境を使用）
- macOS / Linux の文書評価（実機検証なし。採否 ADR の入力として整理）
- 採否 ADR の各条項の confirm / revise と `accepted` への遷移
- 計測結果の数値記録（再現手順付き）

### Out of Scope

- `ruviform-ui` / `ruviform-ui-webview` クレートの実装（将来クレート。本 PoC はそれらの設計・実装 Issue への入力を作る）
- プラグイン GUI 統合（別 Issue のスコープ。`wxp` / `baseview` 経路の評価は参照材料としての整理に留める）
- 製品 UI コンポーネント（波形・メータ等の製品品質実装）の構築 — 計測用の代表ワークロードのみ
- macOS / Linux の実機テスト（文書評価のみ。Linux 実機/VM を PoC 後半で用意するかは discovery の Open Question として判断を残す）
- wry fork（PR #1762 適用版）の上流 merge 活動
- 性能 SLA の設定・保証（PoC は計測し記録するが、性能を保証しない）
- `rust-version = "1.85"` の引き上げ判断自体（wgpu MSRV 衝突の事実を専用タスクへ連携するに留める）

## Requirements

- **ツールチェーン**: stable Rust のみ、mise pin の `rust = "1.99.0"` でビルド・検証する。`wgpu`（MSRV 1.87）を依存に含むコードはワークスペース宣言値 `rust-version = "1.85"` を守れない事実を記録し、PoC では pin toolchain で先行検証する（宣言値見直しは専用タスクへ連携）
- **`unsafe`**: 原則禁止。必要な場合のみ SAFETY コメント付きの局所許可で例外的に認める（暫定規約どおり）
- **RT 契約**: PoC コードでも UI スレッド ↔ エンジン間は非ブロッキング経路のみとし、RT スレッドから UI への同期呼び出しを持ち込まない（[RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md) 整合）
- **外部型の隔離**: wry / tao / wgpu 等の外部型が公開境界の背後に置かれる構成で書く。PoC ハーネス自体は shipped 公開 API ではないが、「将来の port/adapter 分割で外部型を漏らさずに書けるか」は採否の評価軸の 1 つとして記録する
- **計測の客観性**: 性能主張はすべて数値（達成フレームレート・レイテンシ・スループット・バンドルサイズ等）として記録し、環境・再現手順を併記する
- **既存資産の非改変**: 既存ワークスペースクレートの公開 API を変更しない。`Cargo.toml` workspace `members` への追加を行う場合も `publish = false` で依存隔離する（配置方式は設計段階で決定）

## Acceptance Criteria

- [x] **AC1 — WebView ホスト（wry + tao）**: Windows 11 上で `wry` + `tao` のスタンドアロン WebView アプリがビルド・起動する。ウィンドウが開き、フロントエンドの HTML/JS が `with_custom_protocol` 経由で**埋め込みアセットから**ロードされる（実行時に dev server 不要）。JS 側 `window.ipc.postMessage` の JSON intent が `with_ipc_handler` の Rust ハンドラで受信・処理できる。リサイズ追従・フォーカスが動作する
- [x] **AC2 — WebView 内描画バックエンド計測**: 同一の高頻度更新ワークロード（スクロール波形・レベルメータ相当、目安 60fps 目標）を Canvas2D / WebGL2 / WebGPU の各バックエンドで実行し、達成フレーム統計（平均/p50/p99 フレーム時間・ドロップ率等）をバックエンド別に数値で記録する。WebGPU は機能検出フォールバック込みで評価する
- [x] **AC3 — ネイティブ GPU サーフェスと合成**: `wgpu` がネイティブサーフェス（同一 tao ウィンドウ内の別領域、または隣接する別サーフェス）へ描画できることを確認する。WebView との同居を Windows の許す範囲で評価し、**隣接別サーフェス構成**（WebView 領域と並置/上位置）でイベントヒットテスト・フォーカス・z-order・リサイズ追従が動作するかを記録する。DOM 内合成（ネイティブ面を WebView の下に敷く構成）が Windows で未 merge の wry PR [#1762](https://github.com/tauri-apps/wry/pull/1762)（CompositionController/DirectComposition）を要することを記録し、fork パッチ適用が現実的なら探索的評価、非現実的なら「未 merge 上流エビデンス」として文書記録する。macOS / Linux の合成実現性は文書評価として整理する
- [x] **AC4 — メッセージ境界**: JSON intent（`with_ipc_handler` 経由）の round-trip 遅延・スループットを計測する。カスタムプロトコル経由のバイナリ pull 経路（`Request<Vec<u8>>` → `Response<Cow<'static,[u8]>>` 応答）が `fetch()` / `ArrayBuffer` で動作することを検証し、往復・スループットの数値を記録する。`evaluate_script` による差分 push 経路も計測対象に含め、3 経路併用の実現性を確認する
- [x] **AC5 — フロントエンドフレームワーク比較**: 同一の小規模ベンチマーク UI（高頻度部分更新を含む）を少なくとも 2 候補（例: Solid vs vanilla TS、または Solid vs Svelte 5）で構築し、バンドルサイズ・開発ワークフロー（Vite ビルド→埋め込み配線）・更新粒度を数値付きで比較記録する
- [x] **AC6 — 配布**: アセット埋め込み + カスタムプロトコルによる単一バイナリ構成が Windows で動作することを検証する（WebView2 存在検出 + evergreen 前提・未存在時の bootstrapper 手順を含む）。OS ごとのパッケージング方針を評価する: Windows は実機で検証（NSIS/WiX/Inno 等または portable zip のいずれかを hands-on）、macOS / Linux は文書評価。Linux `libwebkit2gtk-4.1` 実行時依存リスクを定量化して記録する
- [x] **AC7 — 採否 ADR の確定**: [採否 ADR](../adr/ui-tech-selection-poc.md) の全採否条項が、リンクされたエビデンスで confirm されるか revise され、status が `accepted` に遷移する。採否はコンポーネント単位であり固定フォールバック階層としない。実機で検証できなかった項目は「文書評価」と明示する
- [x] **AC8 — 使い捨て検証の隔離**: 既存ワークスペースクレートの公開 API に変更がない。PoC コードは cargo ワークスペース外、または非公開メンバーとして依存隔離されている（配置方式は設計段階で決定し、spec との齟齬があれば設計で解消する）

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| `rust-version = "1.85"` の引き上げ | wgpu MSRV 1.87 衝突の事実を記録・連携するのみ。引き上げ判断はしない | 既存の MSRV 見直し専用タスク（baseline で送付済み） | wgpu 採用クレートが宣言 MSRV を守れず検証ゲートを通過しない |
| Linux（WebKitGTK）実機検証 | 文書評価 + 構成制約（隣接配置/Canvas 収束）で処理。実機/VM 導入は Open Question | 採否 ADR での判断・後続 Issue | Linux 合成限界・配布リスクが実測なしのまま残る |
| wry PR #1762 の upstream merge / fork 保守 | 採否材料として記録。fork 採用の可否は PoC エビデンスで判断 | 採否 ADR・必要ならフォーク保守 Issue | DOM 内合成を Windows で公式に作れない状態が固定化する |
| プラグイン GUI 統合（ホストプロセス内 WebView） | スタンドアロン先行。`wxp` / `baseview` 経路は参照評価 | プラグイン GUI Issue | プラグイン用途のライフサイクル制約が後出しで判明する |
| フロントエンドビルド成果物の Cargo/mise タスク統合 | PoC では手動/最小配線。本格統合は実装 Issue | `ruviform-ui-webview` 実装 Issue | ビルド経路が未整備のまま製品実装に入る |
| 性能 SLA の設定 | 計測値の記録のみ。保証値は設けない | UI 実装 Issue・性能要件の確定 | 計測値と要件の乖離が未管理になる |

## Sources and Relations

- 一次ソース: GitHub Issue [#28 [poc] UI 技術検証（wry・描画バックエンド・フロントエンド FW・配布）](https://github.com/scarecrow173/ruviform/issues/28)（`relations.source`）。PoC の範囲・評価対象・成果物の正本。
- 上流ドキュメント（`relations.derives-from`）: [Issue #28 UI 技術検証 PoC の事前調査](../discovery/issue-28-ui-poc.md)（`DISC-034JbXZSba3SvOavRFfs66`。コンポーネント別ファクト・プラットフォーム実情・受け入れ基準候補の正本）、[UI アーキテクチャ ADR](../adr/ui-architecture-webview-hybrid.md)（accepted。本 PoC への委任元）。
- 制約（`relations.related`）: [設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（要 PoC 表・UI 節・スレッド契約）、[RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)（UI = 非 RT・RT から UI への同期呼び出し禁止）、[プラットフォーム ADR](../adr/platform-and-toolchain-targets.md)（3 OS・stable-only・GPU opt-in）、[ライセンス/配布 ADR](../adr/licensing-and-distribution-policy.md)（外部依存ライセンスの個別調査・公開前レビュー必須）、[`AGENTS.md`](../../AGENTS.md)（暫定開発規約・unsafe 原則禁止・publish=false・検証入口）。
- 派生成果物（`relations.related`）: [採否 ADR](../adr/ui-tech-selection-poc.md)（`accepted` — 全条項を本 spec の AC1–AC8 エビデンスで confirm/revise 済み。集約エビデンスの正本は `poc/ui/RESULTS.md`）。
- 参考（`relations.references`）: [Issue #28](https://github.com/scarecrow173/ruviform/issues/28)、[Issue #1](https://github.com/scarecrow173/ruviform/issues/1)（全体アーキテクチャ詳細設計 — WebView ファースト方針の確定元）。
