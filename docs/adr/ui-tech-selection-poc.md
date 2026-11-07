---
id: "ADR-034Jc9Cguud7nZ2Tt88Ptu"
type: "adr"
status: "superseded"
title: "UI 技術選定（wry・描画バックエンド・フロントエンド FW・配布）— PoC 採否"
created: "2026-10-20"
updated: "2026-11-07"
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
        path: "docs/specs/issue-28-ui-poc.md"
      - type: "doc"
        path: "docs/plans/issue-28-ui-poc.md"
      - type: "doc"
        path: "docs/tasks/poc-adr-finalize.md"
      - type: "doc"
        path: "docs/adr/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../impl/ir/poc-adr-finalize.md"
  depends-on:
    - "ui-architecture-webview-hybrid.md"
  blocks: []
  supersedes: []
  superseded-by:
    - "driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
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
    - "https://github.com/scarecrow173/ruviform/issues/34"
    - "https://github.com/scarecrow173/ruviform/issues/57"
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

> **Status: superseded（2026-11-07）**。本 ADR の採否結果（wry+tao・Solid・WebGL2 実測 floor・embedded assets）は旧 WebView 実装の履歴証拠として保持するが、新規 GUI 実装の採用方針としては Issue #384 の **Driver-neutral GUI Architecture + GPUI 標準 Driver** が後継となる。後継: [driver-neutral-gui-gpui-driver-webview-first-issue-384.md](driver-neutral-gui-gpui-driver-webview-first-issue-384.md)。WebView2 composition・描画実測値・配布形態の記録は移行検証の参照資料として維持し、本文は改訂しない。

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
* **WebView 内 WebGPU**: Windows（WebView2 evergreen）は確実、macOS は 26（Tahoe）以降のみ、Linux（WebKitGTK）は非対応。共通上限にはならず特性別選択の対象（PoC 実測で WebView2 155 は `navigator.gpu` 非公開・flags でも有効化せず — Decision Outcome 参照）
* **`wgpu` ネイティブ描画**（30.x、MIT OR Apache-2.0、MSRV 1.87）: 描画上限が高くネイティブサーフェス/オフスクリーン両対応。semver churn（約 3〜4 ヶ月毎の破壊的更新）を adapter 層で吸収する前提。MSRV 衝突の扱いは要連携
* **ネイティブサーフェス + WebView ハイブリッド**: wgpu 等のネイティブ描画面と WebView を同一ウィンドウ内で同居させる構成（合成方式は次の軸で決める）

### ネイティブサーフェス合成方式

* **隣接別サーフェス（adjacent separate surface）**: WebView 領域と隣接/並置する別ネイティブサーフェス（子 HWND / 子ビュー）にネイティブ描画を置く。3 OS 共通で構成可能な最小構成だが、イベントヒットテスト・フォーカス・リサイズ追従・z-order・DOM overlay との重なりは自前管理（PoC 実測で `SetWindowRgn` は WebView2 子 HWND ツリーを切り抜けず回避策として不十分と判明 — Decision Outcome 参照）
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

**status は `accepted`（2026-10-21）。以下の全採否条項は [Issue #28 PoC](../../poc/ui/RESULTS.md)（Windows 11 x64・WebView2 155.0.4283.62・wgpu `NVIDIA GeForce RTX 4070 SUPER (Vulkan)`・rustc 1.99.0 pin）のエビデンスで confirm または revise 済み。** エビデンスの正本は [poc/ui/RESULTS.md](../../poc/ui/RESULTS.md) の AC → 実験 → エビデンス対応表と `docs/impl/exp/*.jsonl` の実験ログ、実施記録は `docs/impl/ir/poc-*.md` にある。実機未検証項目（macOS / Linux 実挙動・PR #1762 依存構成）は「文書評価」と明示し、採否はコンポーネント単位で固定フォールバック階層としない原則を維持する。

* **WebView 実装**: `wry` 0.57 + `tao` 0.37 を**採用**（`confirm` — [RESULTS.md AC1](../../poc/ui/RESULTS.md)・[ac1 exp log](../impl/exp/ac1-host-experiment-wry-tao-webview.jsonl)・[IR poc-ui](../impl/ir/poc-ui.md)）。埋め込みアセットの `with_custom_protocol` ロード・`window.ipc.postMessage` → `with_ipc_handler` の JSON intent 往復・リサイズ追従（±1px ジッタ付き `set_bounds` 再適用リトライで確実追従）・`focus_parent()` 経由のフォーカス遷移（`mechanism:"window"`）を実機で確認し、`poc-metric.v1` 全項目 `passed`。活発な保守・3 OS バックエンド・子ビュー attach・生ハンドル到達・文字列 IPC + バイナリ対応カスタムプロトコルが揃い、baseline の `ruviform-ui-webview` adapter 想定と一致する。留意事項: `build_as_child` 経路ではリサイズ時の明示 `set_bounds` 再適用が viewport 追従に必要（AC3 で検証済み）、WebView2 の子 HWND ツリーはコンテナ矩形に厳密に限定される。`wxp` は採用候補ではなく「素 wry で不足するプラグインホストライフサイクル対応」の参照実装として評価材料にする（同梱パッチ差分の精査はプラグイン GUI Issue へ引き渡し）。`nih-plug-webview` は WIP・参照実装止まりで**不採用**。WebView2 直叩きは 3 OS 方針と矛盾するため**不採用**
* **高頻度描画**: 特性別併用を**採用**（`revise` — [RESULTS.md AC2](../../poc/ui/RESULTS.md)・[ac2 exp log](../impl/exp/ac2-render-canvas2d-webgl2-webgpu.jsonl)・[IR poc-render-backends](../impl/ir/poc-render-backends.md)）。`wgpu` ネイティブ描画を重量級・上限向け、WebGL2 を 3 エンジン共通の実用下限、Canvas2D を互換性下限のフォールバックとする構成を採用する。実測: 同一ワークロード（~2,000 点ピーク列 + 8ch メータ）で Canvas2D / WebGL2 ともに 60fps・dropped=0・フレーム内作業時間 avg 0.41-0.46ms / p99 ≤0.7ms（vsync 16.7ms に対し 30 倍以上のヘッドルーム）。**revise 内容**: in-WebView WebGPU は WebView2 155 で `navigator.gpu` が非公開（`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` の flags 2 試行でも有効化せず）であり実計測不能だったため、下書きの「Windows 向け opt-in 特性」から「現時点で有効化経路未確立・採用構成に含めない（機能検出 → WebGL2 フォールバックは設計どおり動作を確認）」へ修正する。有効化経路（SDK 側 opt-in API 等）の調査は後続課題。wgpu ネイティブ描画の実現性は AC3 の surface 実験で確認済み（53 フレーム present・エラー 0）。wgpu 採用は MSRV 衝突の解消（宣言値見直しタスク連携・下記「連携事項」）と adapter 層による churn 吸収を条件とする
* **ネイティブサーフェス合成**: 「隣接別サーフェス + WebView」のハイブリッドを 3 OS 共通の最小構成として**採用**（`revise` — [RESULTS.md AC3](../../poc/ui/RESULTS.md)・[ac3 parent exp log](../impl/exp/ac3-surface-adjacent-parent.jsonl)・[ac3 child exp log](../impl/exp/ac3-surface-adjacent-child.jsonl)・[IR poc-native-surface](../impl/ir/poc-native-surface.md)）。`adjacent-parent` / `adjacent-child` 両モードで検査マトリクス全項目を記録し、WebView 描画・IPC・wgpu present の同時継続を実機確認。**revise 内容**（下書きの予想との差異）: (a) 既定 z-order は WebView コンテナが最上位に留まり、sibling ネイティブ面は下に座る — `SetWindowPos(HWND_TOP)` の明示 raise で一時的な上書きは可能だが「ネイティブ面が WebView より上」は既定ではない。(b) `SetWindowRgn` はコンテナ自体へのリージョン設定には成功するが、子 HWND ツリー（`Chrome_WidgetWin_*`）を構造的に切り抜けないため DOM overlay 回避策として**不十分と判明**（下書きで想定した回避策としては不採用。実用には子 HWND 毎のリージョン設定か `ICoreWebView2CompositionController` 経路が要る）。(c) `WindowFromPoint` は外来 top-level ウィンドウを返し得るためヒットテスト一次判定に使えず、自ツリー被覆クエリで判定する必要がある。DOM 内合成: Windows は wry PR [#1762](https://github.com/tauri-apps/wry/pull/1762) 依存（**文書評価**・本 PoC で fork スパイク未実施。上流 merge 待ち vs fork 保守コストの採否材料として記録）、macOS は layer 合成で実現可能性が高いが自前実装（**文書評価**）、Linux は実質不可寄り（**文書評価**）— 共通構成には採用しないことを確定
* **メッセージ境界**: 3 経路併用を**採用**（`confirm` — [RESULTS.md AC4](../../poc/ui/RESULTS.md)・[ac4 exp log](../impl/exp/ac4-ipc-json-rtt-pull-evaluate-script-push.jsonl)・[IR poc-message-boundary](../impl/ir/poc-message-boundary.md)）。intent = JSON over `with_ipc_handler`、状態差分 push = `evaluate_script`、高頻度描画データ = カスタムプロトコル経由バイナリ pull。実測: JSON RTT は ≤1KiB で p50 0.4ms / p99 ≤3.3ms（64KiB で avg ~4.4ms）、単方向 ingest ~40k msg/s（500 msg 排水窓）、バイナリ pull は 1MiB ~94-115MB/s・16MiB ~88-146MB/s、`evaluate_script` push は dropped=0、3 経路併用時も rtt/pull の遅延劣化ほぼ無し。**採用条件としての設計含意を追記**: `evaluate_script` push は高レートで暗黙に coalesce される（最大 ~105-109 件/タスク・受信間隔が ~0ms と ~15.6ms に二峰化）ため「1 push = 1 UI 更新」の粒度は成立しない — 高頻度状態同期は pull 型（間隔・併合はフロント側制御）+ seq 付き差分通知（latest-wins で coalesce 耐性）に棲み分ける。小さな pull 要求は初回 ~330ms のコールドパスを持つ（ウォームアップ要求を含めるか初回コストを許容する設計が必要）。Rust 側定速送信は OS タイマ量子（実測 ~11.1ms/tick）に支配される。型付き直列化（serde 系 + TS 側スキーマ対応）はアプリ層の責務
* **フロントエンドフレームワーク**: **Solid を採用**（`confirm` — [RESULTS.md AC5](../../poc/ui/RESULTS.md)・[ac5 exp log](../impl/exp/ac5-frontend-compare.jsonl)・[IR poc-frontend-and-dist](../impl/ir/poc-frontend-and-dist.md)）。同一ベンチ UI（vanilla TS 参照との 2 候補比較）で、Solid の fine-grained 更新が手書き vanilla と**完全に同一の部分適用粒度**（mutations/meter 更新 = 1.0・childList churn ゼロ・signal → `style.transform` 直接適用）を持つことを実測確認。コスト差は dist raw +8,823B（+78%）/ gzip-9 +3,530B・ビルド +44%（1,417ms → 2,035ms）・glue LOC +2.1 倍（91 → 188 行）に収まり、ホスト全体（release exe 8.1MB）に対し無視できる水準。高頻度部分更新への構造的適合・最小級バンドル・TS 対応を根拠に採用。Svelte 5・Preact は本採否では**不採用**（Svelte 5 は YAGNI 延期 — 同一計装経路への追加容易性は確認済みで、候補見直し時の再評価材料として残す）。vanilla TS は FW ランタイムゼロの下限として有効であり、小規模・単発 UI では選択肢に残る
* **配布**: アセット埋め込み + カスタムプロトコルによる単一バイナリ配布を**採用**（`confirm` — [RESULTS.md AC6](../../poc/ui/RESULTS.md)・[ac6 exp log](../impl/exp/ac6-dist-single-binary.jsonl)・[IR poc-frontend-and-dist](../impl/ir/poc-frontend-and-dist.md)）。Windows hands-on 検証済み: release `ui-poc.exe` **8,100,864B** が埋め込みアセットのみ（`embed_mode=embedded-release`）で temp dir 単体動作、WebView2 evergreen 検出（155.0.4283.62・レジストリ `pv` 経路）+ 未検出時の bootstrapper 案内（公式 fwlink / winget）を実装、portable zip **3,039,116B** を別 temp dir 解凍で動作確認（spec の「NSIS/WiX/Inno 等または portable zip のいずれか」を portable zip で充足。NSIS/WiX は参照評価）。macOS（`.app` + Developer ID 署名 + notarytool 公証・productbuild/dmg）・Linux（`libwebkit2gtk-4.1` + GTK3 実行時依存・deb 依存宣言・glibc ベースライン・AppImage 肥大化）は**文書評価**。Linux `libwebkit2gtk-4.1` 実行時依存は最大リスクとして [RESULTS.md の API バージョン表](../../poc/ui/RESULTS.md)で定量化済み（4.1/soup3 が唯一の現実経路・4.0/soup2 は WebKitGTK 2.52 で削除済み・soup2/soup3 プロセス内混在はクラッシュ）
* **採否の構造**: 上記はすべてコンポーネント単位の独立した採否であり、固定フォールバック階層ではない — この構造を維持して確定（`confirm` — 全条項のエビデンスリンク + [RESULTS.md AC8](../../poc/ui/RESULTS.md) の隔離実測・[RESULTS.md AC7](../../poc/ui/RESULTS.md) の条項別 verdict 表）。実機で検証できない項目（macOS / Linux の実挙動・PR #1762 依存構成）は「文書評価」と明示して採否した

### Consequences

* 良い点: 採否の各条項が PoC エビデンスにリンクされ、決定根拠が追跡可能になった。下書き採否を先に構造化したことで、PoC は「仮説の confirm/revise」に集中でき、実際に 2 条項（高頻度描画・合成方式）で実測による revise が発生した。特性別併用の描画構成は ui ADR の「固定順序・単一フォールバック階層にしない」方針と整合する
* 悪い点: 採否が複数軸の組合せになり、PoC 工数が単一選定より増えた。revise により、in-WebView WebGPU の現行 WebView2 での非可用性・`SetWindowRgn` による DOM overlay 回避の不成立が確定事実として残った（`ruviform-ui` / `ruviform-ui-webview` の設計ではこれらの制約を前提にする）。wry PR #1762 未 merge の間、Windows での DOM 内合成は fork 前提のまま残る
* Follow-ups（本 ADR の確定後に引き渡す事項）: (a) `rust-version = "1.85"` と wgpu MSRV 1.87 の衝突は MSRV 見直し専用タスクへの入力として記録済み（当初の委譲先 [Issue #34](https://github.com/scarecrow173/ruviform/issues/34) は closed のため [Issue #57](https://github.com/scarecrow173/ruviform/issues/57) で追跡）— wgpu を採用構成に含める条件となる。(b) WebView2 内 WebGPU の有効化経路（SDK opt-in API の有無等）の調査。(c) wry PR #1762 の上流 merge 状況の追跡 / fork 保守採否（DOM 内合成が必要になった場合）。(d) macOS / Linux の実機検証（文書評価項目の実測化）。(e) `wxp` 同梱パッチ差分の精査（プラグイン GUI Issue の参照材料）。(f) Linux 実配布方式（deb / AppImage / Flatpak）の選定は配布対象 OS 確定時

### Confirmation

下記 Verification のチェックリストを [Issue #28 PoC spec](../specs/issue-28-ui-poc.md) の Acceptance Criteria（AC1〜AC8）と対応付けて確認した。各採否条項は PoC で記録した計測値・検証結果へのリンクを持つことを confirm の条件とし、実機未検証項目は「文書評価」ラベルを必須とした。全条項が confirm / revise で確定したため 2026-10-21 に status を `accepted` へ遷移させた（エビデンス集約・逐条 verdict の記録は [ac7 exp log](../impl/exp/ac7-adr-finalize.jsonl)・[IR poc-adr-finalize](../impl/ir/poc-adr-finalize.md)）。

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

* [x] **WebView 実装条項**: wry+tao ハーネスが Windows 11 でビルド・起動し、埋め込みアセットの `with_custom_protocol` ロード・`window.ipc.postMessage` → Rust ハンドラ・リサイズ/フォーカスが動作する（spec AC1 → [RESULTS.md AC1](../../poc/ui/RESULTS.md) `passed`）。`wxp` は参照材料として整理済み（パッチ差分の精査自体はプラグイン GUI Issue への引き渡し事項として Follow-ups に記録）
* [x] **高頻度描画条項**: Canvas2D / WebGL2 / WebGPU の同一ワークロード（波形・メータ級、60fps 目標）でのフレーム統計がバックエンド別に数値記録される（spec AC2 → [RESULTS.md AC2](../../poc/ui/RESULTS.md) バックエンド別 3 行。WebGPU は `navigator.gpu` 非公開により機能検出 → WebGL2 フォールバックとして記録 — spec の「機能検出フォールバック込みで評価」どおり）
* [x] **合成条項**: 隣接別サーフェス構成のイベント/フォーカス/z-order/リサイズ挙動が Windows 実機で記録される（spec AC3 → [RESULTS.md AC3](../../poc/ui/RESULTS.md) `adjacent-parent` / `adjacent-child` 両モード `passed`）。DOM 内合成の Windows 実現性は wry PR #1762 依存として「未 merge 上流エビデンス」で記録（fork 評価は時間外オプションとして未実施と明示）。macOS / Linux は文書評価と明示済み
* [x] **メッセージ境界条項**: JSON IPC の round-trip・カスタムプロトコル バイナリ pull のスループットが数値記録され、3 経路併用の実現性が確認される（spec AC4 → [RESULTS.md AC4](../../poc/ui/RESULTS.md) 全 5 行 `passed`・coalesce / コールドパス / タイマ量子の観察記録付き）
* [x] **フロントエンド条項**: 少なくとも 2 候補の同一ベンチマーク UI でバンドルサイズ・更新粒度・ビルド経路が比較記録される（spec AC5 → [RESULTS.md AC5](../../poc/ui/RESULTS.md) vanilla / solid 2 候補・dist バイト・build_ms・glue LOC・DOM mutations の全項目で `passed`）
* [x] **配布条項**: Windows での単一バイナリ + WebView2 検出が hands-on 検証され、OS 別パッケージング方針・Linux `libwebkit2gtk-4.1` リスク定量化が記録される（spec AC6 → [RESULTS.md AC6](../../poc/ui/RESULTS.md) debug/release/zip 3 行 `passed` + macOS/Linux 文書評価行 + リスク定量化表）
* [x] **ADR 確定条項**: 全採否条項が confirm/revise 済みで status が `accepted` に遷移し、実機未検証項目が「文書評価」と明示される（spec AC7 → [RESULTS.md AC7](../../poc/ui/RESULTS.md) verdict 表 + [ac7 exp log](../impl/exp/ac7-adr-finalize.jsonl)）
* [x] **隔離条項**: 既存クレートの公開 API 非改変・PoC コードの依存隔離が確認される（spec AC8 → [RESULTS.md AC8](../../poc/ui/RESULTS.md) `passed`。`git status --porcelain -- Cargo.toml Cargo.lock crates mise.toml` が空・`poc/ui` 独立ワークスペース）

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
* 連携事項: `rust-version = "1.85"` と wgpu MSRV 1.87 の衝突は MSRV 見直し専用タスクへ入力として引き渡した（当初の委譲先 [Issue #34](https://github.com/scarecrow173/ruviform/issues/34) — `[workspace.package]` の `rust-version`・`edition` 確定の正本 — は 2026-10-20 に closed のため後続追跡は [Issue #57](https://github.com/scarecrow173/ruviform/issues/57)。本 ADR では引き上げ自体は決めない。wgpu 採用条項はこの衝突解消を条件とする旨を Decision Outcome に明記）
