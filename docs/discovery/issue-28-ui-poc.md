---
id: "DISC-034JbXZSba3SvOavRFfs66"
type: "discovery"
status: "draft"
title: "Issue #28 UI 技術検証 PoC の事前調査"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../adr/ui-architecture-webview-hybrid.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/platform-and-toolchain-targets.md"
    - "../adr/licensing-and-distribution-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/28"
  derived-by:
    - "../specs/issue-28-ui-poc.md"
    - "../adr/ui-tech-selection-poc.md"
  verifies: []
  verified-by: []
  references:
    - "https://crates.io/crates/wry"
    - "https://crates.io/crates/tao"
    - "https://crates.io/crates/wgpu"
    - "https://github.com/novonotes/wxp"
    - "https://github.com/httnn/nih-plug-webview"
    - "https://github.com/tauri-apps/wry/pull/1762"
    - "https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution"
    - "https://blogs.gnome.org/mcatanzaro/2025/04/28/webkitgtk-api-versions/"
  defers: []
  deferred-by: []
---

# Issue #28 UI 技術検証 PoC の事前調査

## Exploration Goal

本書は GitHub Issue [#28 [poc] UI 技術検証（wry・描画バックエンド・フロントエンド FW・配布）](https://github.com/scarecrow173/ruviform/issues/28) の briefing／探索フェーズの中間成果を記録する canonical ドキュメントである。

Issue #28 は [UI アーキテクチャ ADR](../adr/ui-architecture-webview-hybrid.md)（accepted）が PoC に委任した「WebView ファースト・ハイブリッド方針の実現性検証」を担い、[設計ベースライン](../designs/ruviform-architecture-design-baseline.md) の要 PoC 表（WebView 実装・描画バックエンド・フロントエンドフレームワーク・配布の各行）に対応する。成果物はコンポーネント単位の採否 ADR であり、検証順は固定フォールバック階層ではない。

本書が記録するもの:

- この PoC が回答すべき問い（Issue 文と設計ベースラインの要 PoC 行からの展開）
- コンポーネント別候補の現行ファクト（最新 stable バージョン・保守状況・ライセンス・MSRV・プラットフォームカバレッジ・既知の caveat。出典 URL 付き）
- ローカル環境（Windows 11 実機）で実検証できる範囲と、macOS / Linux で文書評価に留まる範囲
- 後続 spec が採用できる受け入れ基準候補

> **位置づけ**: 本書は探索の一次情報と仮説を記録するものであり、採否の最終決定を含まない。個々の候補の「有力」表記はすべて仮説であり、最終決定は PoC 実施後の ADR に委ねる。`ruviform-ui`・`ruviform-ui-webview` クレートは計画上の将来クレートであり現時点では存在しない（workspace メンバーは Phase 1 基盤 6 クレートのみ）。

## PoC が回答すべき問い

Issue #28 文と設計ベースライン（要 PoC 表・UI 節・プラグイン GUI 節）から展開した検証問い:

1. **WebView 実装**: `wry` 系でスタンドアロンアプリの WebView ホストを構築できるか。複数 WebView・既存ウィンドウへの子ビュー attach（`build_as_child` 系）・リサイズ/フォーカス・ドラッグ＆ドロップが 3 OS 方針内で実現できるか。プラグイン用途では `wxp` のパッチ適用済み wry が要るか、素の `wry` で足りるか。
2. **WebView 内描画バックエンド**: Canvas2D / WebGL2 / WebGPU が各 WebView エンジン（WebView2 / WKWebView / WebKitGTK）でどこまで使えるか。波形・メータ・スペクトラム級の高頻度更新（目安 30〜60fps・更新レート・データ量は spec で数値化）にどれが耐えるか。
3. **ネイティブ GPU 描画（`wgpu`）**: `tao`/`wry` と同一ウィンドウまたは別サーフェスで `wgpu` 描画が成立するか。MSRV（wgpu 1.87）と本リポジトリ `rust-version = "1.85"` の衝突をどう解くか。semver churn（約 3〜4 ヶ月毎の破壊的リリース）を adapter 層で吸収できるか。
4. **ネイティブサーフェス + WebView 合成**: ネイティブ描画面は DOM 要素ではないため、WebView「内側」の合成・埋め込み・イベント処理（ヒットテスト・フォーカス・z-order・リサイズ追従）が OS ごとに実現できるか。実現できない場合の簡易案（WebView と隣接する別ネイティブサーフェス/子ウィンドウ）のイベント・フォーカス・z-order 挙動。
5. **メッセージ境界**: Rust ⇄ フロントエンド間の型付き直列化メッセージ（初期 JSON）と、高頻度データ（メータ・波形ピーク列）向けバイナリ経路のスループット・遅延・実現手段（`ipc_handler` 文字列 / `evaluate_script` / カスタムプロトコルのバイナリ応答）。
6. **フロントエンドフレームワーク**: メータ/波形の高頻度更新に適する fine-grained reactivity・TS 親和性・バンドルサイズ・ビルド経路（Vite 等）の比較。
7. **配布形態**: `with_custom_protocol` + アセット埋め込みによる単一バイナリ配布の実現性。OS ごとのランタイム依存（Windows WebView2 evergreen、macOS WKWebView 同梱、Linux `libwebkit2gtk-4.1` 実行時依存）とインストーラ/パッケージ方式。プラグイン配布（バンドル内 WebView 初期化・データディレクトリ）への適用。

## 候補状況（コンポーネント別ファクト）

以下は調査時点（2026-10-20）のファクト。`Lean` は採否決定ではない仮説。

### WebView 実装・ウィンドウ基盤

| Option | 現行ファクト | Pros | Cons / Caveats | Lean |
| --- | --- | --- | --- | --- |
| `wry` 0.57.0（2026-09-08 公開、Apache-2.0 OR MIT、MSRV 1.85） | Tauri チーム保守。バックエンド: Windows=WebView2、macOS/iOS=WKWebView、Linux=WebKitGTK（4.1/GTK3・libsoup3 系）、Android。`os-webview` feature は将来の CEF/Servo ポート用の拡張点だが現時点で代替ポートは未 landing | API が安定方向（`WebViewBuilder` + プラットフォーム拡張 trait）。`build(&window)` / `build_as_child(&window)` / `build_gtk(vbox)` で既存ウィンドウへの子ビュー attach を公式サポート。`WebViewExtWindows::{controller,webview,environment,hwnd,reparent}` 等で WebView2 生ハンドルに到達可能 | 0.x で継続的に破壊的変更あり（0.57 で `windows` 0.62/`webview2-com` 0.39 へ更新し Windows 7 切り捨て）。Linux は `gtk::init`・GTK イベントループ統合が前提。feature `linux-body`（WebKit2GTK ≥2.40 のリクエストボディ）は opt-in | yes（第 1 候補。設計ベースラインの `ruviform-ui-webview` adapter 想定と一致） |
| `tao` 0.37.1（2026-09-26、Apache-2.0、MSRV 1.85） | wry と同じ tauri-apps 組織のウィンドウ/イベントループ基盤。`raw-window-handle` 0.6 の `HasWindowHandle` を実装し `wgpu` の `Surface` ターゲットに利用可能。マルチウィンドウ対応（wry の reparent 例は 2 ウィンドウ構成） | wry との組合せ実績が最大（dev-dependency として wry 0.57 が `tao ^0.37` を使用） | 0.x で churn あり。`winit` への差し替えを選択肢に残すなら adapter 境界が必要 | yes（wry 採用時の実質標準） |
| `wxp`（novonotes/wxp、0.1.0-alpha.x、MIT） | WebView ベースのオーディオプラグイン UI 基盤。`crates/wry` に上流追従 + プラグインホスト向けライフサイクルパッチ（エディタ生成・親ウィンドウ attach・フォーカスの順序差）を保持して同梱。Tauri 風 `invoke`/`Channel` 双方向 IPC と `@novonotes/webview-bridge` TS パッケージを提供。NovoNotes 製品で本番利用（WRAC stack、15,000+ ユーザー。`wrac-plugin-template` = wxp + clap-sys + clap-wrapper で CLAP/VST3/AU/AAX） | 「プラグインホストのウィンドウライフサイクル」という素の wry の欠落領域を実績付きで埋める。MIT でコード参照・再利用可 | alpha 自称・公開 API 不安定。パッチ適用済み wry のフォーク保守に依存。規模小（stars ~19） | maybe（プラグイン GUI 統合で最有力の参照/採用候補。素 wry 不足が確認された場合の現実解） |
| `nih-plug-webview`（httnn、ISC） | nih-plug 向け実験的 WebView エディタ。baseview 上に wry（公式版、raw window handle attach）を載せる構成。Serde JSON メッセージ・リサイズ・ファイル D&D・devtools | baseview + wry の構成例として参照価値あり | 作者自身が「WIP・本番未対応」を明記。macOS + Windows のみ（Linux 未検証）。既知不具合: Windows 11 でホスト freeze（issue #11）、FL Studio crash（#15）、Ableton の Escape キー crash 回避策。最終更新 2024-09、以降はコミュニティ PR のみ | no（参照実装としてのみ） |

- IPC/描画以外の `wry` 主要 API（0.57 時点 docs.rs）: `with_ipc_handler(Fn(Request<String>))`（JS 側 `window.ipc.postMessage`、0.57 から `with_ipc_handler` 設定時のみ `window.ipc` を注入）、`evaluate_script`（文字列 JS 実行）、`with_custom_protocol` / `with_asynchronous_custom_protocol`（`Request<Vec<u8>>` → `Response<Cow<'static,[u8]>>`、非同期は `RequestAsyncResponder`）、`with_drag_drop_handler(Fn(DragDropEvent) -> bool)`、`with_transparent`、`with_bounds`/`set_bounds`、`focus`/`focus_parent`、`with_initialization_script`、`WebContext`（データディレクトリ共有）。
- 出典: [crates.io/crates/wry](https://crates.io/crates/wry)、[docs.rs/wry](https://docs.rs/wry/latest/wry/)、[crates.io/crates/tao](https://crates.io/crates/tao)、[github.com/novonotes/wxp](https://github.com/novonotes/wxp)、[github.com/novonotes/wrac-plugin-template](https://github.com/novonotes/wrac-plugin-template)、[github.com/httnn/nih-plug-webview](https://github.com/httnn/nih-plug-webview)、[NovoNotes WRAC 解説（Zenn）](https://zenn.dev/novonotes_dev/articles/1af5048df08188)。

### WebView 内描画バックエンド（エンジン別実状）

| 描画 API | WebView2（Windows） | WKWebView（macOS） | WebKitGTK（Linux） | 評価含意 |
| --- | --- | --- | --- | --- |
| Canvas2D | 対応 | 対応 | 対応（2.45 系で accelerated canvas / OffscreenCanvas が既定有効） | 互換性下限・全 OS 等価。高頻度の上限は実測で確認 |
| WebGL2 | 対応（Chromium 系） | 対応（WebKit） | 対応（ANGLE ベース、2.39.1 で有効化・2.39.5 で既定再有効。2026-01 時点で GTK/WPE の WebGL2 テストは安定パス） | **3 エンジン共通の実用下限**として設計ベースラインの記述を追認 |
| WebGPU | 対応（Edge/Chrome ≥113 以降 evergreen で既定。Edge 146 系では Compatibility mode 等の拡張も追加） | **macOS 26（Tahoe）以降のみ**。Safari 26.0 で WebKit WebGPU が shipped だが `navigator.gpu` は macOS 26+/iOS 26+ 限定（macOS 15 Sequoia + Safari 26 では非公開。WebKit bug 299237） | **非対応**（upstream 未実装。GPU process 実験は WebGL のみ） | WebGPU は「Windows 確実・macOS は OS バージョン条件付き・Linux 不可」— 単一の共通上限にはならず、特性別選択の対象 |

- 出典: [caniuse WebGPU](https://caniuse.com/webgpu)、[WebKit Features in Safari 26.0](https://webkit.org/blog/17333/webkit-features-in-safari-26-0/)、[WebKit bug 299237](https://bugs.webkit.org/show_bug.cgi?id=299237)、[WebKitGTK NEWS（ANGLE/WebGL2 有効化）](https://github.com/WebKit/WebKit/blob/webkitgtk-2.46.5/Source/WebKit/gtk/NEWS)、[WebKitGTK WebGL2 テスト安定化](https://github.com/WebKit/WebKit/pull/56930)、[Edge 146 release notes（WebView2 Runtime 146 記載）](https://learn.microsoft.com/en-us/microsoft-edge/web-platform/release-notes/146)、[caniwebview WebGPU](https://caniwebview.com/features/web-feature-webgpu/)。

### ネイティブ GPU 描画（`wgpu`）

| Option | 現行ファクト | Pros | Cons / Caveats | Lean |
| --- | --- | --- | --- | --- |
| `wgpu` 30.0.1（2026-08-22、MIT OR Apache-2.0） | MSRV 1.87。新 MSRV ポリシー（v29 系〜）: 実益ある場合のみ bump し「stable − 3」超は bump しない。リリース周期: v27（2025-10）→ v28（2025-12）→ v29（2026-03）→ v30（2026-07）で約 3〜4 ヶ月毎の破壊的更新が継続 | 3 OS + Web 対応の純 Rust GPU API。描画上限が高くネイティブサーフェス/オフスクリーン両方に使える | **MSRV 1.87 は本ワークスペース宣言値 `rust-version = "1.85"` を上回る衝突**（toolchain pin 1.99.0 自体は充足。宣言 MSRV の見直しは設計ベースラインが既に専用タスクへ送付済み）。semver churn → `ruviform-gpu` adapter 層での吸収必須（baseline 方針と一致）。wry の dev-deps に `wgpu` 系と `glow` が存在し、同一ウィンドウ共存の前例が上流リポジトリ内にある | yes（adapter 層 + opt-in 前提。baseline の `ruviform-gpu` 想定と一致） |

- 出典: [crates.io/crates/wgpu](https://crates.io/crates/wgpu)、[wgpu releases / MSRV ポリシー](https://github.com/gfx-rs/wgpu/releases)。

### ネイティブサーフェス + WebView 合成（OS 別実現性）

描画バックエンドと DOM の間に「ネイティブ描画面」を挟む構成の実現性は OS ごとに大きく異なる。

| OS | 構成 | 現行ファクトと実現性 |
| --- | --- | --- |
| Windows（windowed WebView2 = 現行 wry の経路） | WebView2 は `WRY_WEBVIEW` 子 HWND として描画。**子 HWND は同矩形内のネイティブ sibling コンテンツの上に常に描画され、DOM overlay（モーダル等）も HWND 上では z-order 敗北する**（airspace 問題。実例: loomux の対応は wry fork せず `SetWindowRgn` で子 HWND を矩形クリップする workaround を採用） | 「ネイティブ面を WebView の下/中に挟む」合成は windowed モードでは不可。隣接する別子 HWND/`wgpu` サーフェスの並置は可能（イベント・フォーカス・z-order は自前管理）。DOM overlay がネイティブ面に被る場合は `SetWindowRgn` クリップ等の回避策が必要（実例あり） |
| Windows（CompositionController / DirectComposition） | `ICoreWebView2CompositionController` + `IDCompositionVisual` ターゲットなら WebView を visual tree に載せ、ネイティブ wgpu/D3D コンテンツを透明 WebView の下に正式合成可能 | **wry は未対応**。ただし wry PR [#1762](https://github.com/tauri-apps/wry/pull/1762)（open・2026-07 起票）が opt-in の `with_composition_visual_target(IUnknown)` を実装済み（実務利用の報告あり: ネイティブ wgpu 面の上に透明 wry webview を重ねる PDF アプリ）。採用には wry fork/パッチ適用か上流 merge 待ちが必要 |
| macOS | WKWebView は layer-backed な `NSView` で、wry `build`/`build_as_child` は content view の subview として追加。`WebViewExtMacOS::webview()` でハンドル到達可 | sibling `NSView`/`CAMetalLayer` を layer 順序で合成する方式は macOS の標準的構成であり 3 OS 中最も見通しが良い（baseline の「layer-backed で合成しやすい」を追認）。wry レベルでの公式 API はないため NSView/CALayer 操作は自前実装 |
| Linux（WebKitGTK） | WebView は GTK ウィジェット（wry `build_gtk` で `gtk::Fixed`/`vbox` に配置）。GTK3/4 に DirectComposition 相当の公開 visual-tree hosting はなく、WebView 描画面の下に任意 GPU コンテンツを挟む公式経路はない | `GtkOverlay` + `GtkGLArea` 等での重ね合わせは限定的（WebView 自身の不透明描画・透過対応の制約）。**真の意味での下層合成は実質不可寄り**で、隣接配置または WebView 内 Canvas 系への収束が現実的 |

- 簡易案（全 OS 共通の第 2 候補構成）: WebView と隣接する別ネイティブサーフェス（子ウィンドウ/子ビュー）を並置する。wry の `with_bounds`/`set_bounds` + `build_as_child` 系で同一ウィンドウ内に同居可能だが、イベントヒットテスト・フォーカス遷移・リサイズ追従・z-order・（Windows では）DOM overlay との重なりは全て自前管理。複数独立トップレベルウィンドウを位置同期する方式はフォーカス/スナッピング/OS 差異のコストが大きい。
- 出典: [wry PR #1762（DirectComposition visual hosting）](https://github.com/tauri-apps/wry/pull/1762)、[CoreWebView2CompositionController（MS Learn）](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/winrt/microsoft_web_webview2_core/corewebview2compositioncontroller?view=webview2-winrt-1.0.3912.50)、[loomux の windowed モード制約と SetWindowRgn 対応の記録](https://github.com/willem445/loomux/commit/a099d27154a31105a9b1b6c935526790a4cb9bf0)、[wry WebViewExtWindows](https://docs.rs/wry/latest/x86_64-pc-windows-msvc/wry/trait.WebViewExtWindows.html)。

### メッセージ境界（Rust ⇄ フロントエンド）

設計ベースラインの契約: Rust 側を状態の正本、フロントは intent 送信・状態差分/高頻度データ受信。初期は JSON 系直列化、高頻度はバイナリ検討（要 PoC）。

| 経路 | 実能力（wry 0.57） | 高頻度適性 |
| --- | --- | --- |
| JS → Rust: `window.ipc.postMessage` → `with_ipc_handler` | ペイロードは `Request<String>`（**文字列のみ**。Windows は文字列、macOS/Linux は JSValue を渡せるが wry の公開境界は文字列に正規化される） | JSON intent/状態通知には十分。バイナリは直接不可（base64 化は非効率） |
| Rust → JS: `evaluate_script` | 文字列 JS 評価のみ（戻り値チャネルなし、文字列シリアライズが前提） | 低〜中頻度の差分 push は可。高頻度の大量データは JS 文字列化コストが支配的になり得る |
| 双方向代替: カスタムプロトコル（`with_custom_protocol` / 非同期 `RequestAsyncResponder`） | HTTP 風 `Request<Vec<u8>>` → `Response<Cow<'static,[u8]>>`。**バイナリボディをそのまま返せる**ため、フロント側 `fetch()`/`ArrayBuffer` で高頻度バイナリ取得が可能（リクエスト駆動型）。Linux のリクエストボディは `linux-body` feature（WebKit2GTK ≥2.40）が必要 | 高頻度バイナリの現実解。pull 型のため push 型とは別設計（ポーリング間隔・coalesce をフロント側で制御）。真のストリーミング/WS 相当はなく 1 リクエスト = 1 応答 |

- 含意: 「intent = JSON over IPC」「状態差分 = JSON push（evaluate_script）」「高頻度描画データ = カスタムプロトコル経由バイナリ pull」という 3 系統併用が wry 素の機能で構成可能。型付き直列化（serde 系 + TS 側のスキーマ対応）はアプリ層の責務。
- 出典: [docs.rs wry WebViewBuilder](https://docs.rs/wry/latest/wry/struct.WebViewBuilder.html)、[wry issue #767（IPC 実能力の整理）](https://github.com/tauri-apps/wry/issues/767)、[RequestAsyncResponder](https://docs.rs/wry/latest/wry/struct.RequestAsyncResponder.html)、[wry feature 一覧（`linux-body` 等）](https://docs.rs/wry/latest/wry/)。

### フロントエンドフレームワーク

| Option | 現行ファクト | Pros | Cons | Lean |
| --- | --- | --- | --- | --- |
| Solid.js（solid-js 1.9.x 系、MIT） | fine-grained リアクティビティ（signals）、VDOM なし。TodoMVC 実測でランタイム ~14KB・アプリ gzip ~8KB 級、10k 行生成/部分更新とも最速級 | メータ/波形の高頻度更新に構造的に適合（更新箇所だけが走る）。TS 完全対応 | JSX 慣習の学習。エコシステムは React より小 | yes（第 1 候補の仮説） |
| Svelte 5（5.55.x 系、MIT） | runes による fine-grained 反応、コンパイル時出力。ランタイム ~35KB（v5 はランタイム方式に回帰し v4 より重い）| 単一要素更新が最速級・宣言的記述力 | `.svelte` 専用コンパイラ・Svelte 固有記法 | maybe |
| Preact（10.29.x 系、MIT） | React API 互換の ~4KB コア（アプリ gzip ~9KB 級）。signals 拡張あり | React 知識転用・小さい | VDOM コストは残る。fine-grained でない（signals 導入で緩和） | maybe |
| vanilla + TS + Vite（Vite 8.x / TypeScript 6.x 系） | FW ランタイムゼロ | 最小バンドル・依存最小・制御完全 | 状態管理・差分更新は自前。コンポーネント体系化のコスト | maybe（小規模 UI や描画専用部品に） |

- いずれも MIT でライセンス面の障害なし。選定軸は「高頻度部分更新の構造的適合」「TS/ビルド経路」「学習・保守コスト」。バンドルサイズ出典: [frontend-framework-bundle-size](https://mlgq.github.io/frontend-framework-bundle-size/?lang=en)、[realworld-js-framework-comparison](https://github.com/naufalafif/realworld-js-framework-comparison)。バージョン例は上流依存更新コミットの記録値（[pyreon の依存更新](https://github.com/pyreon/pyreon/commit/77606042575a32f4a3018780f8311d4463ded6e9)）。

### 配布形態

| OS | ランタイム依存 | 現行ファクト | リスク評価 |
| --- | --- | --- | --- |
| Windows | WebView2 evergreen Runtime | Windows 11 全機にプリインストール、Windows 10 の大半にも展開済み。未存在時は online bootstrapper / offline installer / Fixed Version（~100〜250MB 同梱）の逃げ道あり。起動時の存在確認（レジストリ `HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}` の `pv` 値等）は公式推奨 | 低（本機も 155.0.4283.62 存在確認済み） |
| macOS | WKWebView（OS 同梱） | 追加ランタイム不要。`.app` + 署名/公証が配布条件 | 低 |
| Linux | `libwebkit2gtk-4.1` + GTK3（wry 依存: `webkit2gtk`/`soup3`/`javascriptcore-rs`/`gtk` crates） | API 版は 4.0（GTK3+libsoup2）/4.1（GTK3+libsoup3）/6.0（GTK4+libsoup3）の 3 系統。**libsoup2 サポートは WebKitGTK 2.52.0（2026-03）で削除済み** → 4.0 経路は将来性なし、4.1 が wry/Tauri の現行前提。deb 依存は `libwebkit2gtk-4.1-0` + `libgtk-3-0`。ベースラインは Ubuntu 22.04/Debian 12（glibc 下限は最古対象 OS でビルドが定石）。AppImage は同梱可だが重量・GTK 統合の caveat あり | **最大リスク**（baseline 指摘を追認: ディストロ格差・ランタイム必須・GTK3 依存） |
| 共通（アセット） | — | `with_custom_protocol` で埋め込みバイナリから直接配信可能（dev-server 不要）。`rust-embed` 8.x / `include_dir` 0.7.x が定石（release ビルドで埋め込み・debug で FS 読みの運用可） | 低 |
| 共通（インストーラ） | — | 本リポジトリは独自 wry スタックを構築するため Tauri bundler（NSIS/WiX・dmg/app・deb/rpm/AppImage）は参照実装扱い。自前: Windows = NSIS/Inno/WiX 系、macOS = productbuild/dmg、Linux = deb/rpm/AppImage を個別整備 or cargo-packager 系の採否 | 中（整備コスト自体は既知の領域） |
| プラグイン配布 | 同上 + `WebContext` データディレクトリ | プラグインはホストプロセス内で WebView を起こすため、データディレクトリ配置・プロセス共存・ライフサイクル（wxp のパッチ領域）が固有論点 | 中〜高（PoC 範囲ではスタンドアロン先行、プラグイン統合は `wxp` 等の実績を評価材料に） |

- 出典: [WebView2 配布（MS Learn）](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)、[Evergreen vs Fixed](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/evergreen-vs-fixed-version)、[WebKitGTK API versions demystified](https://blogs.gnome.org/mcatanzaro/2025/04/28/webkitgtk-api-versions/)、[libsoup2 sunset（WebKitGTK blog）](https://www.webkitgtk.org/2025/10/07/webkitgtk-soup2-deprecation.html)、[Tauri prerequisites（依存パッケージ実例）](https://v2.tauri.app/start/prerequisites/)、[Tauri Debian 配布（glibc ベースライン）](https://v2.tauri.app/distribute/debian/)、[rust-embed](https://docs.rs/crate/rust-embed/latest)、[include_dir](https://docs.rs/include_dir/latest/include_dir/)。

## プラットフォーム実情（ローカル検証可能性）

本機（Windows 11 x64）で確認した環境ファクト:

| 項目 | 実測値 | 確認方法 |
| --- | --- | --- |
| Rust toolchain（mise pin） | `rustc 1.99.0 (b940084d7 2026-09-28)` / `cargo 1.99.0` | `mise.toml` の `rust = "1.99.0"`、`mise exec -- rustc --version` |
| WebView2 Runtime | `155.0.4283.62`（インストール済み） | レジストリ `HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}` の `pv`、`C:\Program Files (x86)\Microsoft\EdgeWebView\Application\155.0.4283.62` |
| Node.js / npm | `v26.10.0` / `11.19.1` | `node --version` / `npm --version` |
| ワークスペース宣言 MSRV | `rust-version = "1.85"`（Cargo.toml・AGENTS.md 記載） | **wgpu MSRV 1.87 と衝突**（toolchain pin 自体は 1.99.0 で充足。宣言値の整合は別タスク扱いが baseline で確定済み） |

ローカルで実検証できる範囲:

- `tao` + `wry` スタンドアロン WebView アプリのビルド・動作（WebView2 155 実機）
- WebView 内 Canvas2D / WebGL2 / WebGPU（WebView2 = Chromium 系で全て利用可）の高頻度更新実測
- `wgpu` ネイティブ描画（同一 tao ウィンドウ or 別サーフェス）と WebView の同居・隣接構成、`SetWindowRgn` クリップ回避策の検証
- DirectComposition 合成は **wry PR #1762 を fork 適用する形でのみ検証可能**（upstream 未 merge）
- IPC/カスタムプロトコルのスループット・遅延計測、アセット埋め込み配布構成
- フロントエンド候補（Solid/Svelte/Preact/vanilla+Vite）の同条件ハーネス比較
- Windows 向けインストーラ/起動時ランタイム検出の検証

macOS / Linux で文書評価に留まる範囲（本機に環境なし）:

- WKWebView の layer-backed sibling 合成、WebGPU の macOS 26+ 限定という実機挙動
- WebKitGTK（`libwebkit2gtk-4.1`）の実行時依存・GTK イベントループ統合・合成限界・ディストロ別パッケージング

## Tentative Conclusions and Hypotheses

仮説（採否は PoC 結果と ADR で確定）:

- **WebView 実装**: `wry`（+ `tao`）を `ruviform-ui-webview` adapter の第 1 候補として維持できるファクトが揃った（活発な保守・3 OS バックエンド・attach/子ビュー API・生ハンドル到達・文字列 IPC + バイナリ対応のカスタムプロトコル）。プラグイン用途では `wxp`（MIT・本番実績）が「素 wry では不足するホストライフサイクル対応」の具体的な中身を示しており、素 wry + 自前補強 vs wxp 採用/参照は PoC の比較軸になる。
- **描画下限**: WebGL2 は 3 エンジン共通の実用下限として確認できた。WebGPU は「Windows 確実・macOS 26 以降のみ・Linux 不可」— **baseline の「現状 Windows（WebView2）のみ信頼可」は macOS 条件付き対応へ一部変化**しているが、共通上限にはならない点は変わらない。
- **ネイティブ合成**: Windows で「ネイティブ面を WebView の下に敷く」理想形は wry 未対応（PR #1762 = fork/パッチ or merge 待ち）。当面の現実解は隣接する別ネイティブサーフェス + 自前イベント/z-order 管理（Windows は `SetWindowRgn` 回避策が要る場合あり）。macOS は layer 合成が最も現実的、Linux は真の下層合成が不可寄り → **「ネイティブ面は DOM 内でなく隣接領域に置く」方向が 3 OS 共通の最小共通構成**という仮説が有力。
- **メッセージ境界**: wry の IPC は文字列（JSON）が基調で、高頻度バイナリはカスタムプロトコル pull が現実解。baseline の「初期 JSON・高頻度はバイナリ検討（要 PoC）」と整合する具体的経路が特定できた。
- **wgpu**: MSRV 1.87 と `rust-version = "1.85"` の衝突は確定事項（pin toolchain 1.99.0 ではビルド可能だが、wgpu 依存クレートが宣言 MSRV を守れない）。PoC では pin toolchain で先行し、宣言 MSRV の見直しは既存の専用タスクへ連携する。
- **配布**: Windows/macOS は既知の定石。Linux `libwebkit2gtk-4.1` 実行時依存は依然として最大リスクで、4.1 前提（soup3）が唯一の現実的経路（4.0/soup2 は upstream が 2.52 で削除済み）。Servo/CEF 等の代替 WebView ポートは wry 内で未 landing のため状況変化なし。

### リスク・未解決事項

- wry PR #1762（DirectComposition hosting）の採用可否 = fork 保守コスト or 上流 merge 依存。未 merge の間は「下層ネイティブ面」を Windows で公式には作れない。
- WebKitGTK の WebView 描画面とネイティブ GPU 面の合成限界は実機評価ができず、PoC 後半で Linux 環境を用意するか文書評価 + 構成制約（隣接/内蔵 Canvas への収束）で処理するかの判断が要る。
- `wxp` のパッチ適用済み `crates/wry` の上流追従速度・パッチ内容の詳細は未精査（採用判断前に差分レビューが必要）。
- `evaluate_script`/IPC の高頻度スループット、カスタムプロトコル pull の遅延特性は未計測（PoC の計測対象）。
- インストーラ/パッケージング方式の自前整備コスト（Tauri bundler はライセンス・構成とも参照止まり）。
- `linux-body` feature・`WebContext` データディレクトリのプラグイン配布時の振る舞い（ホスト共存・権限）は未検証。

## spec が要求すべき事項（受け入れ基準候補）

後続の PoC spec が採用できる受け入れ基準の候補（コンポーネント単位・固定フォールバック階層にしない）:

1. **WebView ホスト**: `wry` + `tao` で Windows 上のスタンドアロンアプリとして WebView を構築・表示・リサイズ追従・フォーカス/D&D が動作する。`build_as_child`（または同等）で既存ウィンドウ/親ビューへの attach 経路を確認する。複数 WebView 共存と `WebContext` 分離の可否を確認する。
2. **WebView 内描画**: 同一の高頻度更新ハーネス（波形ピーク列・レベルメータ相当）で Canvas2D / WebGL2 / WebGPU を計測し、目標更新レート・レイテンシ・CPU/GPU コストを比較記録する（目標値は spec で数値化: 例 60fps・更新間隔・データ点数）。WebGPU は機能検出フォールバック込みで評価する。
3. **ネイティブ GPU（wgpu）**: `wgpu` を同一プロセスで初期化し、tao ウィンドウ（または別サーフェス）へ描画できることを確認する。wgpu 依存クレートが宣言 MSRV 1.85 を守れないことを記録し、toolchain pin 1.99.0 での検証と MSRV 見直しタスクへの接続を明記する。
4. **合成/同居**: 「隣接ネイティブサーフェス + WebView」構成でイベントヒットテスト・フォーカス・リサイズ・z-order（DOM overlay がネイティブ面に被る場合の挙動含む）を Windows 実機で検証する。DirectComposition 下層合成は wry PR #1762 fork での探索的評価（採否材料として記録）。macOS/Linux は文書評価結果を採否 ADR の入力として整理する。
5. **メッセージ境界**: intent=JSON over `ipc_handler`、差分 push=`evaluate_script`、高頻度データ=カスタムプロトコル経由バイナリの 3 経路を実装し、スループット・遅延・フロント側 coalesce 方針を計測する。
6. **フロントエンド**: Solid/Svelte/Preact/vanilla(+Vite) の少なくとも 2 候補で同一ハーネスを構築し、高頻度部分更新性能・ビルド経路（バンドル→埋め込み）・TS 統合を比較する。
7. **配布**: カスタムプロトコル + アセット埋め込みの単一バイナリを構成し、Windows では WebView2 存在検出 + evergreen 前提（bootstrapper フォールバック手順）を確認する。macOS/Linux の配布要件は文書で記録する。
8. **RT 契約整合**: UI スレッド ↔ エンジン間は非ブロッキング経路のみ（RT スレッドから UI への同期呼び出し禁止）を PoC コードでも遵守し、スレッド契約の検証可能性を確認する（[RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md) 整合）。
9. **境界整合**: 検証コードは将来の `ruviform-ui`（ポート定義）/`ruviform-ui-webview`（adapter）の分割を意識し、wry/wgpu 等の外部型を公開境界の背後に置く構成で書く（adapter → ポート定義元の依存方向）。

## Open Questions

- wry PR #1762 の上流 merge 見通しと、採用する場合の fork/patch 保守体制（`wxp` 同梱パッチ済み wry が同種コストの前例）
- WebKitGTK の合成限界をどこまで実機検証するか（PoC スコープに Linux 実機/VM を含めるか）
- `wxp` の同梱 wry パッチ差分の精査結果（採用/参照判断の入力）
- 高頻度バイナリ経路の実測性能（カスタムプロトコル pull の間隔下限・JSON IPC のスループット上限）
- プラグイン GUI 統合を本 PoC でどこまで触るか（スタンドアロン先行か、`baseview`/`wxp` 経路の attach 検証まで行うか）
- `rust-version = "1.85"` の引き上げ判断（wgpu 採用確定前に先行するか、専用タスクの結論を待つか）
- フロントエンドのビルド成果物をどの段階から `Cargo`/mise タスクに統合するか（Vite ビルド → `rust-embed` の配線）
- Linux 配布で AppImage/Flatpak/deb のどこまでを PoC 評価対象にするか

## Promotion Candidates

- [x] spec-doc needed: 本調査の問い・候補・受け入れ基準候補をもとに、Issue #28 の PoC 範囲・検証環境・採否基準を定義する spec を後続で作成する
- [ ] adr-doc needed: PoC 実施後、コンポーネント単位（WebView 実装 / 描画バックエンド / 合成構成 / メッセージ境界 / フロントエンド FW / 配布）の採否を記録する ADR 群（ui ADR が委任した委託先）
- [ ] followup: `rust-version` MSRV 見直しタスク（baseline 済み送付）へ wgpu MSRV 1.87 衝突の事実を入力として引き渡す

## Findings Summary

- `wry` 0.57.0（Apache-2.0 OR MIT、MSRV 1.85）は第 1 候補として十分な API 面を持つ: 子ビュー attach、生ハンドル到達、文字列 IPC + バイナリ対応カスタムプロトコル、D&D、透明化。`tao` 0.37.1 は wgpu の `Surface` ターゲットに使えるウィンドウ基盤。
- プラグイン用途は `wxp`（MIT・alpha・NovoNotes 本番実績）が「素 wry で不足するホストライフサイクル対応」の中身と最小構成を具体的に示す。`nih-plug-webview` は WIP・参照止まり。
- 描画: WebGL2 は 3 エンジン共通の下限として追認。WebGPU は Windows（WebView2）確実 + macOS 26 以降のみ + Linux 不可。Canvas2D は全 OS のフォールバック。
- `wgpu` 30.0.1 は MSRV 1.87 で本ワークスペース宣言 `rust-version = "1.85"` と衝突（pin toolchain 1.99.0 では可。宣言値見直しは既存タスク連携）。semver churn は約 3〜4 ヶ月毎で継続中 → adapter 層必須。
- ネイティブ合成: Windows の真の下層合成は wry 未対応（PR #1762 が open で実装例あり・fork 前提）。当面は「隣接ネイティブサーフェス」が 3 OS 共通の最小構成。Windows では DOM overlay との重なりに `SetWindowRgn` 回避策が要る場合がある。macOS は layer 合成が最有力、Linux は真の合成が不可寄り。
- メッセージ境界: `ipc_handler` は文字列のみ、`evaluate_script` は文字列のみ。高頻度バイナリはカスタムプロトコル pull（`RequestAsyncResponder` 可）が現実解。baseline の「初期 JSON・高頻度はバイナリ検討」に整合する具体経路あり。
- フロントエンド: Solid 1.9 / Svelte 5 / Preact 10 / vanilla+Vite はいずれも MIT で高頻度部分更新適性は Solid/Svelte が構造上有利。バンドルは Solid/vanilla 最小。
- 配布: Windows = WebView2 evergreen（本機も 155.0.4283.62 在）。macOS = 同梱 WKWebView。Linux = `libwebkit2gtk-4.1`（soup3）が唯一の現実経路で最大リスク変わらず（4.0/soup2 は upstream で削除済み）。アセット埋め込みは `with_custom_protocol` + `rust-embed`/`include_dir` が定石。
- ローカル検証可能性: Windows 実機で wry/tao・WebView 内全描画・wgpu 同居・IPC/プロトコル計測・埋め込み配布・フロント比較まで実施可。macOS/Linux は文書評価（+ Linux 実機の要否を spec で判断）。

## Sources and Relations

- 一次ソース: GitHub Issue [#28](https://github.com/scarecrow173/ruviform/issues/28)（`relations.source` / `relations.derives-from`）。PoC の範囲・評価対象・成果物（採否 ADR）の正本。
- 上流ドキュメント（`relations.related`）: [UI アーキテクチャ ADR](../adr/ui-architecture-webview-hybrid.md)（本 PoC への委任元・accepted）、[設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（要 PoC 表・UI 節・Phase 3・後続 Issue 一覧の正本）、[RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)（UI = 非 RT・RT から UI への同期呼び出し禁止）、[プラットフォーム ADR](../adr/platform-and-toolchain-targets.md)（3 OS・stable-only・GPU opt-in）、[ライセンス/配布 ADR](../adr/licensing-and-distribution-policy.md)（外部依存ライセンスの個別調査・公開前レビュー必須）、[AGENTS.md](../../AGENTS.md)（暫定開発規約・検証入口）。
- 外部調査ソース（`relations.references` + 本文中 URL）: crates.io/docs.rs（wry・tao・wgpu・rust-embed・include_dir）、tauri-apps/wry（PR #1762・issue #767・feature 一覧）、novonotes/wxp・wrac-plugin-template・httnn/nih-plug-webview、MS Learn（WebView2 配布・Evergreen/Fixed・CompositionController・Edge release notes）、WebKitGTK blog/NEWS・WebKit bugzilla、caniuse/caniwebview、バンドル比較データ（frontend-framework-bundle-size・realworld 比較）。
