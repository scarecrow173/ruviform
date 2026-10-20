---
id: "ADR-034Jc9Cguud7nZ2Tt88Ptu"
type: "adr"
status: "proposed"
title: "UI 技術選定（wry・描画バックエンド・フロントエンド FW・配布）— PoC 採否"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/28"
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
    - "ui-architecture-webview-hybrid.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "realtime-contract-and-verification-policy.md"
    - "platform-and-toolchain-targets.md"
    - "licensing-and-distribution-policy.md"
    - "../discovery/issue-28-ui-poc.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../specs/issue-28-ui-poc.md"
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
    - "https://docs.rs/wry/latest/wry/"
    - "https://crates.io/crates/tao"
    - "https://crates.io/crates/wgpu"
    - "https://wgpu.rs/"
    - "https://github.com/tauri-apps/wry/pull/1762"
    - "https://github.com/novonotes/wxp"
    - "https://github.com/httnn/nih-plug-webview"
    - "https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution"
    - "https://blogs.gnome.org/mcatanzaro/2025/04/28/webkitgtk-api-versions/"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# UI 技術選定（wry・描画バックエンド・フロントエンド FW・配布）— PoC 採否

## Context and Problem Statement

[UI アーキテクチャ ADR](ui-architecture-webview-hybrid.md)（accepted）が「WebView ファースト・ハイブリッド」を第一候補として確定し、wgpu 等の具体採用範囲・フロントエンドフレームワーク・描画バックエンド・配布形態の最終選択を GitHub Issue [#28](https://github.com/scarecrow173/ruviform/issues/28) の PoC に委任した。本 ADR はその委任先であり、[Issue #28 PoC spec](../specs/issue-28-ui-poc.md) で定義された検証のエビデンスに基づき、コンポーネント単位の採否を記録する。

採否は **コンポーネント単位**で行い、ui ADR の確定方針どおり固定順序・単一フォールバック階層としては扱わない。検証可能な範囲は Windows 11 実機（WebView2 155 系）での実測が中心であり、macOS / Linux は文書評価をもって判断材料とする（実機未検証項目は「文書評価」と明示する）。[事前調査（discovery）](../discovery/issue-28-ui-poc.md) が各候補の現行ファクトを収集済みであり、本 ADR の Decision Outcome はその仮説を下書き採否として記録し、PoC エビデンスで confirm または revise する。

## Decision Drivers

* ui ADR の確定範囲との整合: WebView ファースト・ハイブリッド、描画方式とコンポーネントの分離、共通描画抽象の非強制、RT スレッドへの UI 処理・ブロッキング同期の持ち込み禁止
* 高頻度描画要件: 波形・メータ・スペクトラム級の更新（目安 30〜60fps）に耐える描画経路が必要で、DOM 更新だけでは不足し得る
* 3 OS 対応（[platform ADR](platform-and-toolchain-targets.md): Windows / macOS / Linux・x86_64+aarch64・stable-only）。単一 OS でしか成立しない構成は共通上限にならない
* Linux 配布リスク: `libwebkit2gtk-4.1` 実行時依存が最大リスク（[licensing/配布 ADR](licensing-and-distribution-policy.md)・baseline 指摘を discovery が追認）
* 外部型の隔離: wry / wgpu 等の外部型は公開 API 境界の背後に置く（`ruviform-ui` ポート定義 ↔ `ruviform-ui-webview` adapter 分割）。採用候補は adapter 境界に閉じ込められることが採否の軸になる
* MSRV 制約: ワークスペース宣言 `rust-version = "1.85"` に対し `wgpu` MSRV 1.87 が衝突する（pin toolchain 1.99.0 ではビルド可能。宣言値見直しは専用タスク連携）
* 依存の保守性・ライセンス: メンテナンス状況・ライセンス（原則 MIT / Apache-2.0 系）・依存木サイズを確認し理由を記録する（暫定規約・[licensing ADR](licensing-and-distribution-policy.md)）

## Considered Options

### WebView 実装・ウィンドウ基盤

* **`wry` 0.57 + `tao` 0.37**（Apache-2.0 OR MIT / Apache-2.0、MSRV 1.85）: Tauri チーム保守。Windows=WebView2、macOS/iOS=WKWebView、Linux=WebKitGTK 4.1、Android をカバー。`build_as_child` 系の子ビュー attach・`with_custom_protocol`・`with_ipc_handler`・`WebViewExtWindows` による WebView2 生ハンドル到達を公式サポート。`tao` は `raw-window-handle` 経由で `wgpu` の `Surface` ターゲットに利用可能
* **`wxp`**（novonotes/wxp、0.1.0-alpha.x、MIT）: 上流追従 + プラグインホスト向けライフサイクルパッチを適用した `crates/wry` を同梱するオーディオプラグイン UI 基盤。Tauri 風 `invoke`/`Channel` IPC と TS ブリッジを提供。NovoNotes 製品で本番実績
* **`nih-plug-webview`**（httnn、ISC）: baseview 上に wry（公式版・raw window handle attach）を載せる実験的プラグインエディタ。作者自身が WIP・本番未対応を明記、macOS+Windows のみ、最終更新 2024-09
* **純粋 WebView2 直叩き**（`webview2-com` 等）: wry を介さず Windows WebView2 を直接操作。Windows 専用になり 3 OS 方針と矛盾するため、3 OS 抽象の中間層として独自実装する変形のみ意味を持つ

### 高頻度描画バックエンド

* **WebView 内 Canvas2D**: 3 エンジン共通の互換性下限。高頻度更新での上限は実測次第
* **WebView 内 WebGL2**: 3 エンジン共通の実用下限（WebView2=Chromium 系・WKWebView・WebKitGTK いずれも対応）
* **WebView 内 WebGPU**: Windows（WebView2 evergreen）は確実、macOS は 26（Tahoe）以降のみ、Linux（WebKitGTK）は非対応。共通上限にはならず特性別選択の対象
* **`wgpu` ネイティブ描画**（30.x、MIT OR Apache-2.0、MSRV 1.87）: 描画上限が高くネイティブサーフェス/オフスクリーン両対応。semver churn（約 3〜4 ヶ月毎の破壊的更新）を adapter 層で吸収する前提。MSRV 衝突の扱いは要連携
* **ネイティブサーフェス + WebView ハイブリッド**: wgpu 等のネイティブ描画面と WebView を同一ウィンドウ内で同居させる構成（合成方式は次の軸で決める）

### ネイティブサーフェス合成方式

* **隣接別サーフェス（adjacent separate surface）**: WebView 領域と隣接/並置する別ネイティブサーフェス（子 HWND / 子ビュー）にネイティブ描画を置く。3 OS 共通で構成可能な最小構成だが、イベントヒットテスト・フォーカス・リサイズ追従・z-order・DOM overlay との重なり（Windows では `SetWindowRgn` クリップ等の回避策が要る場合あり）は自前管理
* **DOM 内合成（inside-DOM compositing / 下層合成）**: ネイティブ描画面を WebView の下層に敷き DOM を上に合成する理想形。Windows は `ICoreWebView2CompositionController` + DirectComposition が必要で **wry は未対応**（未 merge の PR [#1762](https://github.com/tauri-apps/wry/pull/1762) が opt-in API を実装済み・fork/パッチまたは上流 merge 待ち）。macOS は layer-backed `NSView`/`CAMetalLayer` の sibling 合成が標準的で最有力（wry 公式 API はなく自前実装）。Linux（WebKitGTK）は `GtkOverlay`+`GtkGLArea` 等でも限定的で真の下層合成は実質不可寄り

### メッセージ境界（Rust ⇄ フロントエンド）

* **3 経路併用**: intent = JSON over `with_ipc_handler`（`window.ipc.postMessage`、文字列のみ）、状態差分 push = `evaluate_script`（文字列 JS 評価）、高頻度描画データ = カスタムプロトコル経由バイナリ pull（`Request<Vec<u8>>` → `Response<Cow<'static,[u8]>>`。Linux のリクエストボディは `linux-body` feature・WebKit2GTK ≥2.40 が必要）。wry 素の機能で構成可能
* **JSON IPC への一本化**: バイナリ経路を持たず全データを JSON 文字列でやり取りする簡易構成。高頻度・大容量データで文字列化コストが支配的になる見込み
* **外部 IPC/ネットワーク機構の併用**（localhost WS 等）: WebView 外の通信路を追加する構成。配布・セキュリティ・ポート管理の複雑性が増す

### フロントエンドフレームワーク

* **Solid**（solid-js 1.9.x、MIT）: signals による fine-grained リアクティビティ・VDOM なし。高頻度部分更新に構造的に適合しランタイム・バンドル最小級。TS 完全対応
* **Svelte 5**（5.55.x、MIT）: runes による fine-grained 反応・コンパイル時出力。宣言的記述力が高い。ランタイムは v5 で ~35KB と v4 より増加
* **Preact**（10.29.x、MIT）: React 互換 ~4KB コア + signals 拡張。React 知識を転用できるが VDOM コストが残る
* **vanilla TS + Vite**（Vite 8.x / TypeScript 6.x）: FW ランタイムゼロ・依存最小。状態管理・差分更新は自前で、コンポーネント体系化コストが増す

### 配布形態

* **アセット埋め込み + 単一バイナリ**: `with_custom_protocol` で埋め込みアセット（`rust-embed` / `include_dir`）を直接配信。dev server 不要。OS 別ランタイム依存: Windows=WebView2 evergreen（未存在時は bootstrapper / Fixed Version の逃げ道）、macOS=WKWebView 同梱、Linux=`libwebkit2gtk-4.1`+GTK3 実行時依存（4.0/soup2 は WebKitGTK 2.52 で削除済み → 4.1 が唯一の現実経路）
* **dev-server 依存構成**: 開発時のみ許容し配布形態からは除外する選択肢（実行時に外部サーバを要するため単一バイナリ配布と両立しない）
* **OS 別インストーラ/パッケージの整備方式**: Windows = NSIS / Inno / WiX または portable zip、macOS = `.app` + 署名/公証（productbuild / dmg）、Linux = deb / rpm / AppImage。Tauri bundler は独自 wry スタックのため参照実装扱い（cargo-packager 系の採否を含む）

## Decision Outcome

**現時点のステータスは `proposed`。以下は discovery のファクトに基づく下書き採否であり、全条項 `要 PoC 確証` とする。** [Issue #28 PoC spec](../specs/issue-28-ui-poc.md) の Acceptance Criteria に対応するエビデンスが得られた条項から confirm し、反証があれば revise する。全条項が confirm または revise によって確定した時点で status を `accepted` に遷移させる。

* **WebView 実装**: `wry` 0.57 + `tao` 0.37 を採用予定（`要 PoC 確証`）。活発な保守・3 OS バックエンド・子ビュー attach・生ハンドル到達・文字列 IPC + バイナリ対応カスタムプロトコルが揃い、baseline の `ruviform-ui-webview` adapter 想定と一致する。`wxp` は採用候補ではなく「素 wry で不足するプラグインホストライフサイクル対応」の参照実装として評価材料にする（同梱パッチ差分の精査は別途）。`nih-plug-webview` は WIP・参照実装止まりで不採用方向。WebView2 直叩きは 3 OS 方針と矛盾するため不採用方向
* **高頻度描画**: 特性別併用を予定（`要 PoC 確証`）— `wgpu` ネイティブ描画を重量級・上限向け、WebGL2 を 3 エンジン共通の実用下限、Canvas2D を互換性下限のフォールバックとする構成が第一仮説。WebGPU（in-WebView）は Windows 限定・macOS 条件付き・Linux 不可のため共通バックエンドにはせず、Windows 向け opt-in 特性として扱う見込み。wgpu 採用は MSRV 衝突の解消（宣言値見直しタスク連携）と adapter 層による churn 吸収を条件とする
* **ネイティブサーフェス合成**: 「隣接別サーフェス + WebView」のハイブリッドを 3 OS 共通の最小構成として採用予定（`要 PoC 確証`）。DOM 内合成は Windows で wry PR #1762 依存（fork 保守コスト vs 上流 merge 待ちの採否材料として記録）、macOS は layer 合成で実現可能性が高いが自前実装、Linux は実質不可寄りのため、共通構成には採用しない方向
* **メッセージ境界**: 3 経路併用を採用予定（`要 PoC 確証`）— intent = JSON over `with_ipc_handler`、状態差分 push = `evaluate_script`、高頻度描画データ = カスタムプロトコル経由バイナリ pull。型付き直列化（serde 系 + TS 側スキーマ対応）はアプリ層の責務
* **フロントエンドフレームワーク**: Solid を第一候補、Svelte 5 を次点として採用予定方向（`要 PoC 確証`）。高頻度部分更新への構造的適合・最小級バンドル・TS 対応が理由。最終採否は同一ハーネスでの比較計測（バンドルサイズ・更新粒度・ビルド経路）で確定する
* **配布**: アセット埋め込み + カスタムプロトコルによる単一バイナリ配布を採用予定（`要 PoC 確証`）。Windows は WebView2 evergreen 前提 + 起動時存在検出（bootstrapper フォールバック手順）で hands-on 検証。macOS / Linux は文書評価。Linux `libwebkit2gtk-4.1` 実行時依存は最大リスクとして定量化し、deb 依存・glibc ベースライン・AppImage 等の方式を文書評価で整理する
* **採否の構造**: 上記はすべてコンポーネント単位の独立した採否であり、固定フォールバック階層ではない。実機で検証できない項目（macOS / Linux の実挙動）は「文書評価」と明示して採否する

### Consequences

* 良い点: 採否の各条項が PoC エビデンスにリンクされ、決定根拠が追跡可能になる。下書き採否を先に構造化したことで、PoC は「仮説の confirm/revise」に集中できる。特性別併用の描画構成は ui ADR の「固定順序・単一フォールバック階層にしない」方針と整合する
* 悪い点: 採否が複数軸の組合せになり、PoC 工数が単一選定より増える。下書き採否が反証で revise される場合、`ruviform-ui` / `ruviform-ui-webview` の想定構造（adapter 分割・合成方式）の再検討が要る。wry PR #1762 未 merge の間、Windows での DOM 内合成は fork 前提のまま残る

### Confirmation

下記 Verification のチェックリストを [Issue #28 PoC spec](../specs/issue-28-ui-poc.md) の Acceptance Criteria（AC1〜AC8）と対応付けて確認する。各採否条項は PoC で記録した計測値・検証結果へのリンクを持つことを confirm の条件とし、実機未検証項目は「文書評価」ラベルを必須とする。全条項確定後に status を `accepted` に遷移させ、遷移はレビュー（review-protocol）で確認する。

## Implementation Plan

* Affected paths:
  * PoC 検証ハーネス: `poc/ui/` 等のワークスペース外ディレクトリ（正確な配置・依存隔離方式は設計段階で決定。`publish = false` の非公開メンバーとする選択肢も spec で許容）
  * 本 ADR（`docs/adr/ui-tech-selection-poc.md`）: PoC エビデンスで各条項を confirm/revise し `accepted` へ遷移
  * 計測記録: PoC ハーネス内または `poc/ui/` 配下の記録ファイル（再現手順・環境付き）
* Patterns to follow: 将来の `ruviform-ui`（ポート定義）/ `ruviform-ui-webview`（adapter）分割を意識し、wry / tao / wgpu 等の外部型を境界の背後に置く構成で書く。UI スレッド ↔ エンジン間は非ブロッキング経路のみ（RT 契約）。計測は数値+再現手順で記録する
* Patterns to avoid: 既存ワークスペースクレートの公開 API 変更。採否条項の固定フォールバック階層化。文書評価項目を実機検証済みと誤記すること。性能計測値を SLA として扱うこと
* Dependencies or configuration: `wry` 0.57・`tao` 0.37・`wgpu` 30.x・`rust-embed`/`include_dir`・serde 系・フロントエンド候補（Solid 1.9 / Svelte 5 / Preact 10 / vanilla+Vite）・Windows パッケージング候補（NSIS/WiX/Inno または portable zip）。wry fork（PR #1762 適用）は探索的評価時のみ。依存追加はライセンス・保守状況の確認付き（暫定規約）

## Verification

各採否条項を confirm する PoC エビデンス（spec の Acceptance Criteria との対応）:

* [ ] **WebView 実装条項**: wry+tao ハーネスが Windows 11 でビルド・起動し、埋め込みアセットの `with_custom_protocol` ロード・`window.ipc.postMessage` → Rust ハンドラ・リサイズ/フォーカスが動作する（spec AC1）。`wxp` パッチ差分の精査結果が参照材料として整理される
* [ ] **高頻度描画条項**: Canvas2D / WebGL2 / WebGPU の同一ワークロード（波形・メータ級、60fps 目標）でのフレーム統計がバックエンド別に数値記録される（spec AC2）
* [ ] **合成条項**: 隣接別サーフェス構成のイベント/フォーカス/z-order/リサイズ挙動が Windows 実機で記録される。DOM 内合成の Windows 実現性が wry PR #1762 への依存として記録され、fork 評価または「未 merge 上流エビデンス」が添付される。macOS / Linux は文書評価と明示される（spec AC3）
* [ ] **メッセージ境界条項**: JSON IPC の round-trip・カスタムプロトコル バイナリ pull のスループットが数値記録され、3 経路併用の実現性が確認される（spec AC4）
* [ ] **フロントエンド条項**: 少なくとも 2 候補の同一ベンチマーク UI でバンドルサイズ・更新粒度・ビルド経路が比較記録される（spec AC5）
* [ ] **配布条項**: Windows での単一バイナリ + WebView2 検出が hands-on 検証され、OS 別パッケージング方針・Linux `libwebkit2gtk-4.1` リスク定量化が記録される（spec AC6）
* [ ] **ADR 確定条項**: 全採否条項が confirm/revise 済みで status が `accepted` に遷移し、実機未検証項目が「文書評価」と明示される（spec AC7）
* [ ] **隔離条項**: 既存クレートの公開 API 非改変・PoC コードの依存隔離が確認される（spec AC8）

## Pros and Cons of the Options

### WebView 実装 — wry 0.57 + tao 0.37

* Good, because 3 OS バックエンドを単一 API でカバーし、子ビュー attach・カスタムプロトコル（バイナリ応答可）・WebView2 生ハンドル到達まで公式に揃う。MSRV 1.85 で宣言値と衝突しない。`ruviform-ui-webview` adapter の想定と一致。
* Bad, because 0.x で継続的に破壊的変更があり（0.57 で Windows 7 切り捨て等）、Linux は GTK イベントループ統合が前提。DirectComposition 合成は未対応（PR #1762 待ち）。

### WebView 実装 — wxp

* Good, because プラグインホスト向けライフサイクルパッチ（エディタ生成・親ウィンドウ attach・フォーカス順序）の具体例と本番実績があり、MIT。
* Bad, because alpha 品質で同梱パッチ済み wry の上流追従・差分精査コストが残る。スタンドアロン UI 基盤としては過剰で、プラグイン Issue の参照材料が適切な位置づけ。

### WebView 実装 — nih-plug-webview / WebView2 直叩き

* Good, because nih-plug-webview は baseview + 公式 wry の構成例として参照価値がある。直叩きは中間層を持たず依存が最小。
* Bad, because nih-plug-webview は WIP・macOS+Win 限定・既知クラッシュ不具合が未解消（保守停滞）。直叩きは Windows 専用で 3 OS 方針と矛盾し、自前抽象層の構築コストが wry 採用理由を消す。

### 高頻度描画 — Canvas2D / WebGL2 / WebGPU（WebView 内）

* Good, because ネイティブ統合なしに WebView 内で完結し、WebGL2 は 3 エンジン共通の実用下限、Canvas2D は全 OS の互換下限。WebGPU は Windows で高い上限を持つ。
* Bad, because Canvas2D は高頻度更新の上限が低い可能性、WebGPU は macOS 26+ 限定・Linux 非対応で共通バックエンドにできない。ネイティブ描画と比べ GPU 制御の粒度が粗い。

### 高頻度描画 — wgpu ネイティブ + ハイブリッド

* Good, because 描画上限が最も高く、波形・スペクトログラム級の重量描画に余裕がある。`raw-window-handle` 経由で tao ウィンドウに直結できる。
* Bad, because MSRV 1.87 が宣言値 1.85 と衝突（pin toolchain 1.99.0 では可・宣言値見直しは別タスク）。semver churn を adapter 層で吸収する前提コストが要る。WebView との合成は OS 差が大きい。

### 合成方式 — 隣接別サーフェス

* Good, because 3 OS 共通で構成可能な最小構成であり、wry 素の機能（`build_as_child`/`set_bounds`）で実現できる。
* Bad, because イベントヒットテスト・フォーカス・z-order・リサイズ追従・DOM overlay 重なり（Windows は `SetWindowRgn` 回避策）を全て自前管理する。DOM 内にネイティブ面を「埋める」理想構造ではない。

### 合成方式 — DOM 内合成

* Good, because ネイティブ描画面を WebView の下に敷く理想構造で、DOM overlay をネイティブ面の上に正しく合成できる。
* Bad, because Windows は wry 未対応（PR #1762 = fork/パッチまたは上流 merge 依存）、macOS は自前の NSView/CALayer 実装、Linux は実質不可寄り — 3 OS 共通構成にできない。

### メッセージ境界 — 3 経路併用 vs JSON 一本化

* Good, because 3 経路併用は intent/差分/高頻度バイナリをそれぞれ適した経路に置け、wry 素の機能のみで構成可能。JSON 一本化は単純で初期実装が速い。
* Bad, because 3 経路は pull 型の間隔・coalesce 制御をフロント側で自前設計する必要がある（真のストリーミングはない）。JSON 一本化は高頻度・大容量で文字列化コストが支配的になり、バイナリの直接送付ができない（base64 は非効率）。

### フロントエンド — Solid / Svelte 5 / Preact / vanilla

* Good, because Solid は fine-grained 更新・最小バンドル・TS 対応で高頻度 UI に構造的に適合。Svelte 5 は宣言的記述力が高い。Preact は React 転用可。vanilla は依存ゼロ。
* Bad, because Solid は JSX 慣習とエコシステム規模が課題。Svelte 5 は専用コンパイラ・固有記法・ランタイム ~35KB。Preact は VDOM コストが残る。vanilla は状態管理・体系化を全て自前にする。

### 配布 — 埋め込み単一バイナリ vs dev-server 依存

* Good, because 埋め込み + カスタムプロトコルは実行時に dev server 不要で単一バイナリ配布と整合し、wry 標準機能で実現できる。OS 別インストーラ整備は既知の領域（Tauri bundler は参照実装）。
* Bad, because Linux の `libwebkit2gtk-4.1` 実行時依存は単一バイナリでも消えず最大リスクのまま。dev-server 依存は配布形態として成立しない。プラグイン配布（ホストプロセス内 WebView・データディレクトリ）は固有論点が残る。

## More Information

* 一次ソース: GitHub Issue [#28 [poc] UI 技術検証](https://github.com/scarecrow173/ruviform/issues/28)（`relations.source`）、Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（方針確定元）
* 上流ドキュメント: [事前調査 DISC-034JbXZSba3SvOavRFfs66](../discovery/issue-28-ui-poc.md)（`relations.derives-from`。候補ファクト・Lean 仮説の正本）、[PoC spec](../specs/issue-28-ui-poc.md)（検証手順・受け入れ基準の正本）、[UI アーキテクチャ ADR](ui-architecture-webview-hybrid.md)（`depends-on`・委任元）
* 関連 ADR: [RT 契約](realtime-contract-and-verification-policy.md)、[プラットフォーム](platform-and-toolchain-targets.md)、[ライセンス/配布](licensing-and-distribution-policy.md)、[設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（要 PoC 表）
* 外部参照（`relations.references`）: [wry docs.rs](https://docs.rs/wry/latest/wry/)、[tao](https://crates.io/crates/tao)、[wgpu](https://crates.io/crates/wgpu) / [wgpu.rs](https://wgpu.rs/)、[wry PR #1762](https://github.com/tauri-apps/wry/pull/1762)、[wxp](https://github.com/novonotes/wxp)、[nih-plug-webview](https://github.com/httnn/nih-plug-webview)、[WebView2 配布](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)、[WebKitGTK API versions](https://blogs.gnome.org/mcatanzaro/2025/04/28/webkitgtk-api-versions/)
* 連携事項: `rust-version = "1.85"` と wgpu MSRV 1.87 の衝突は既存の MSRV 見直し専用タスクへ入力として引き渡す（本 ADR では引き上げ自体は決めない）
