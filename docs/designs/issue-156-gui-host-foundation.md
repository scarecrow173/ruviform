---
id: "DESIGN-034P8EHh2IlZd14yNSOreO"
type: "design"
status: "draft"
title: "Issue #156 再利用可能デスクトップ GUI ホスト基盤"
created: "2026-10-30"
updated: "2026-10-31"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/155"
    - "https://github.com/scarecrow173/audio-lens/issues/105"
  changes:
    added:
      - "crates/ruviform-ui"
      - "crates/ruviform-ui-webview"
    modified:
      - "Cargo.toml"
      - "crates/README.md"
      - "mise.toml"
      - ".github/workflows/ci.yml"
      - "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/ui-tech-selection-poc.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/issue-28-ui-poc.md"
    - "docs/adr/plugin-gui-embedding.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-156-gui-host-foundation-inventory.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/ui-tech-selection-poc.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "poc/ui/RESULTS.md"
    - "poc/ui/README.md"
  defers: []
  deferred-by: []
---

# Issue #156 再利用可能デスクトップ GUI ホスト基盤

## Context

`poc/ui`（Issue #28 / #57）で wry + tao の WebView ホスト、3 経路
メッセージ境界、埋め込みアセット単一バイナリ配布が Windows 実機で
検証され、技術選定は accepted ADR で確定済み（wry 0.57 + tao 0.37、
Solid、3 経路 IPC、rust-embed 埋め込み、特性別描画）。本設計は
Issue #156 の要求どおり、PoC 専用でない再利用可能な GUI ホスト
基盤をワークスペースの本番クレートとして新設する。

設計ベースライン A.2 は本構成をすでに予約している:
`ruviform-ui`（拡張層のポート定義元・WebView 非依存）と
`ruviform-ui-webview`（Optional platform adapter・ポート定義元への
依存は合法）。採択済み選定の再審査は行わない。AudioLens 側責務
マップの契約 — Ruviform は GUI ホスト/フロントエンド通信・汎用描画
インフラのみを所有し、アプリ固有の Job・診断型・画面構成に依存
しない — が公開 API の境界条件。

## Scope

### In Scope

- `ruviform-ui` 新設: WebView 非依存の UI 基盤ポート。
  ウィンドウ/ホストライフサイクル契約、`HostEvent` 正規化イベント、
  `rvf.ipc.v1` メッセージスキーマ、Intent → Service → Job の
  実行モデル（進捗・取消・エラー）、`AssetSource` /
  `DataSource`（bounded binary transfer）ポート、JS ブリッジ
  スニペット、capability/バックエンド検出型、参照実装 `demo`
  モジュール、描画責務分離の型（`render`）。
- `ruviform-ui-webview` 新設: `ruviform-ui` が定義するホスト境界の
  wry/tao 実装アダプタ。`rvf://` カスタムプロトコル応答
  （`app`/`data` 名前空間）、`evaluate_script` event push、
  バックエンド検出（`wry::webview_version`）。`webview` feature で
  実装全体をゲート（非既定）。
- `ruviform-ui` example `demo_cli`（headless CLI）と
  `ruviform-ui-webview` example `demo_app`（実機 GUI）— 同じ
  `DemoService` を両入口から駆動する証跡。
- 検証: schema/ディスパッチ/ジョブ状態機械/データ転送/プロトコル
  応答の headless テスト、`webview` feature レーン（mise タスク +
  CI ジョブ）、ローカル実機検証の記録。
- 文書: 本設計、棚卸し discovery、crate README ×2（利用手順・OS 別
  依存・配布要件）、索引・mise・CI の整合更新。

### Out of Scope

- `ruviform-gpu`（wgpu ネイティブサーフェス adapter）の実装 —
  ポート契約の型だけ定義し、実装は独立 Issue 候補。
- Solid/Vite フロントエンドの scaffold や型付き TS クライアント
  パッケージ（利用側アプリの選択。ブリッジ JS 契約のみ提供）。
- 複数ウィンドウ拡張（具体的利用要求が出てから）。
- プラグイン GUI 埋め込み（#30/#115 の別プロセス検証と分離）。
- AudioLens 固有の Job/診断/画面構成・業務ワークフロー・
  `al-service`/`al-gui`（AudioLens #105 の責務）。
- データセットの計測結果・単位・精度の意味（Ruviform UI は決めない）。
- macOS / Linux の実機検証・配布物の実製作（文書評価の範囲を記録）。
- #157 相当の汎用 LOD/タイル計算カーネル（別 Issue 候補として記録）。

## Components and Boundaries

### クレート構成と依存 DAG

```text
利用側アプリ（al-gui 等。本リポジトリ外）
    │ depends on                    ┌─ Optional platform adapters ─┐
    ▼                               │                              │
ruviform-ui ◄────────────────────── │ ruviform-ui-webview          │
    │ (拡張層・ポート定義元)         │  (wry/tao を内部に封じ込め)   │
    ▼                               └──────────────────────────────┘
ruviform-exec + ruviform-rt（Primitives。非 RT のみ使用）
```

- 依存方向は 上位→下位 のみ。`ruviform-ui` → Primitives
  （`ruviform-exec`・`ruviform-rt`）。`ruviform-ui-webview` →
  `ruviform-ui` + 外部（wry/tao/http/dpi/rust-embed）。
  Domain foundations への依存は現時点では作らない（`demo` の合成
  波形は `ruviform-rt::Processor` 経由で `ruviform-exec` を駆動する
  に留め、ドメイン型を公開面に持ち込まない）。
- foundations → adapters / adapter → adapter 依存は作らない。
- headless 契約: `ruviform-ui` は GUI/WebView/window/GPU 依存を
  一切持たず、`ruviform-ui-webview` を既定 feature なしで依存する
  経路も存在しない（実装は `webview` feature ゲート）。
- RT 契約: 両クレートとも非 RT 専用（UI スレッド・非 RT ワーカー）。
  RT スレッドへのブロッキング同期・UI 処理の持ち込みは契約違反。

### `ruviform-ui` モジュール責務

| モジュール | 責務 |
| --- | --- |
| `event` | `HostEvent` — adapter が正規化するウィンドウ/ページイベント（`Ready`/`Message`/`Resized`/`Focused`/`ScaleFactorChanged`/`CloseRequested`/`Tick`）と `Host` 操作ポート trait（`HostHandle`: eval push・resize・focus・close 要求）。wry/tao 型を含まない |
| `ipc` | `rvf.ipc.v1` スキーマ — `Intent`/`Event` の serde 型、`ErrorBody`、schema バージョン、size cap、op 名前空間規約（`sys.*` 予約） |
| `service` | `Service` trait（op 一覧・Intent 処理）、`CallOutcome`（即時応答 or Job 起動）、`Job`（worker スレッド + `CancellationToken` + progress 報告） |
| `runtime` | `UiRuntime` — Intent ルーティング、`sys.*` 内蔵 op、Job レジストリ、`HostEvent` 消費 → `Event` 送出の駆動器。headless でも GUI でも同一駆動 |
| `assets` | `AssetSource` trait（path → `Asset{bytes,mime}`）、`MapAssetSource`、`mime_for`。配布形態は利用側が選ぶ |
| `data` | `DataSource`/`DataRegistry`/`DataRequest` — 名前付きバイナリソースへの有界読出し（offset+len、上限キャップ）。`VecDataSource` |
| `bridge` | `BRIDGE_JS` — `window.__rvf`（`call`/`onEvent`/`dataUrl`/`dispatch`）を注入する初期化スクリプト定数 |
| `render` | 描画責務分離の型 — `RenderTier`（Dom/Canvas2d/Webgl2/NativeSurface）、`SurfaceRegion`/`NativeSurfacePort` のポート契約スケッチ（実装は `ruviform-gpu` 候補） |
| `detect` | `UiCapabilities`/`BackendReport` — バックエンド検出結果の型（検出自体は adapter） |
| `demo` | `DemoService` — 参照実装（波形ピーク合成 Job + peaks `DataSource` 登録）。examples/tests の共有 fixture |
| `error` | `UiError`/`ServiceError`/`DataError` |

### `ruviform-ui-webview` モジュール責務（すべて `webview` feature ゲート）

| モジュール | 責務 |
| --- | --- |
| `host` | `run(config, ports, handler)` — tao `EventLoop` + `WindowBuilder` + wry `WebViewBuilder` の組立て、`EventLoopProxy` 起床、resize `set_bounds` 追従、close/shutdown、`HostHandle` 実装 |
| `scheme` | `rvf://` カスタムプロトコル応答 — `app` 名前空間 = `AssetSource`、`data` 名前空間 = `DataRegistry` の範囲 pull。MIME・Range・上限・CORS・パストラバーサル拒否を単一実装。headless テスト可能な純粋関数 |
| `push` | `EvalPush` — `IpcEvent` JSON を `evaluate_script`（`window.__rvf.dispatch`）へシリアライズする `EventSink` |
| `detect` | `detect_backend()` — `wry::webview_version()` + OS/表示環境から `BackendReport` を生成 |

外部型（wry/tao/http/dpi）は本クレートの内部にのみ存在し、
公開 API には `ruviform-ui` の型のみ現れる。

### feature・依存構成

- `ruviform-ui`: `serde`+`serde_json`（IPC スキーマ。導入理由を
  manifest コメントに記録）、`ruviform-exec`（`CancellationToken`/
  `Progress`/`OfflineDriver` 再利用）、`ruviform-rt`（`Processor` 型。
  `demo` で `OfflineDriver` を駆動）。default feature 最小。
- `ruviform-ui-webview`: feature `webview` = 非既定。有効時のみ
  `wry`/`tao`/`http`/`dpi`/`rust-embed` を依存解決する
  （`dep:` 宣言の optional dep）。無効時は空に近いクレートとして
  コンパイルされ、`cargo check --workspace` の headless 経路を
  壊さない。feature 名は外部固有名詞を含まない意図命名
  （ベースラインの「バックエンドクレート自身の内部 feature は
  許容」に基づく）。
- 検証レーン: `check-ui-webview` / `lint-ui-webview` /
  `test-ui-webview` の mise タスクを追加し、CI に `ui-verify` ジョブ
  （3 OS matrix。ubuntu は `libwebkit2gtk-4.1-dev` を apt 導入 —
  ALSA ヘッダ導入の既存先例と同じ方式）を追加する。

## Data and Control Flow

### `rvf.ipc.v1` メッセージスキーマ

Frontend → Rust（Intent。`window.ipc.postMessage` の JSON 文字列）:

```json
{"v":1,"kind":"intent","id":"i-42","op":"demo.synthesize","params":{"seconds":30}}
{"v":1,"kind":"intent","id":"i-43","op":"job.cancel","params":{"job":7}}
{"v":1,"kind":"intent","id":"i-1","op":"sys.capabilities","params":{}}
```

Rust → Frontend（Event。`evaluate_script` で `__rvf.dispatch` 呼び出し）:

```json
{"v":1,"kind":"result","id":"i-1","ok":true,"data":{"backend":"webview2","version":"..."}}
{"v":1,"kind":"result","id":"i-99","ok":false,"error":{"code":"unknown_op","message":"..."}}
{"v":1,"kind":"job","job":7,"state":"progress","done":12,"total":64}
{"v":1,"kind":"job","job":7,"state":"finished","data":{...}}
{"v":1,"kind":"job","job":7,"state":"cancelled"}
{"v":1,"kind":"job","job":7,"state":"failed","error":{"code":"worker_failed","message":"..."}}
{"v":1,"kind":"notify","seq":15,"data":{...}}
{"v":1,"kind":"lifecycle","state":"closing"}
{"v":1,"kind":"error","error":{"code":"malformed_message","message":"..."}}
```

規約:

- `v` はスキーマバージョン（`1`）。不一致・欠落は `malformed_message`。
- `id` は呼出側相関キー（`__rvf.call` が採番し Promise と対応付け）。
- `op` は `ns.name` 文字列。`sys.*` は基盤予約（`sys.ping`・
  `sys.capabilities`）、`job.cancel` は Job 制御予約。アプリ op は
  `Service` が登録する。未知 op は `unknown_op`、params 型不整合は
  `invalid_params` で応答する（panic させない）。
- Intent のボディサイズ上限は 1MiB（超過は `message_too_large`）。
- エラーは `{code,message,details?}` の `ErrorBody`。code は
  安定した文字列集合（`malformed_message`/`message_too_large`/
  `unknown_op`/`invalid_params`/`service_error`/`worker_failed`/
  `cancelled`/`unavailable`）。非公開 detail を露出しない。
- `notify` は seq 付き latest-wins の状態 push。PoC 実測どおり
  evaluate_script は高頻度で合体されるため、高頻度状態は
  「最新だけ意味を持つ差分」として設計し、大きな描画データは
  notify に載せず pull 経路へ回す。

### ジョブ実行モデル

- `Service::call` が `CallOutcome::Job` を返すと、UiRuntime は
  `JobId` を採番し worker スレッドへ spawn（`std::thread` +
  `std::sync::mpsc` bounded）。worker は `progress(done,total)` /
  `finished(data)` / `failed(error)` / `cancelled` を送信。
- UI スレッドは `HostEvent::Tick`（HostConfig の周期起床。PoC と同じ
  `WaitUntil` 機構）ごとに mailbox を drain して `job` Event を
  push。worker は UI スレッドを直接触らず、push の詰まりが worker を
  ブロックしないよう mailbox は bounded + latest-progress 合体
  （progress は最新のみ保持、terminal は全件配信）。
- `job.cancel` は `ruviform_exec::CancellationToken` を立てる。
  worker はチャンク境界で `is_cancelled` を観測して `cancelled` で
  終了する（強制 kill はしない）。
- `OfflineDriver` 連携: `demo` の合成 Job は
  `ruviform_exec::{OfflineDriver,Progress,CancellationToken}` をその
  まま使い、GUI/CLI 共通の非 UI 実行契約を実証する。
- panic は `catch_unwind` で worker 内に封じ `worker_failed` に写像。

### `rvf://` カスタムプロトコル

- `rvf://app/<path>`: 埋め込みアセット。`/`・空パスは `index.html`、
  拡張子→MIME、`..`/絶対パス/プレフィックス拒否、未登録は 404。
- `rvf://data/<name>?offset=N&len=M`: `DataRegistry` に登録済みの
  名前付きバイナリを範囲 pull。`offset`/`len` の検証（非数・負・
  範囲外 → 400）、`len` は `max_chunk_bytes`（既定 16MiB）で
  キャップ、`X-Rvf-Total` ヘッダで全長を返し `len` 省略時は先頭
  chunk、`application/octet-stream` + `Access-Control-Allow-Origin`
  で `app` オリジンからの `fetch().arrayBuffer()` を許可。
- Windows では wry が `rvf://app/x` を `http://rvf.app/x` に書き換える
  ため、host 名の `rvf.` プレフィックスを剥がして名前空間を判定する
  （PoC 方式の継承）。
- 初回リクエストの ~330ms cold path は実測済み（RESULTS）。小さな値は
  `result.data` の JSON で返し、バルクのみ pull を使う運用を
  README で明記する。

### セキュリティ境界

- フロントエンドが到達できるのは (a) 登録済み `Service` op、
  (b) `rvf://app` の埋め込みアセット、(c) `rvf://data` の登録済み
  `DataSource` のみ。ローカル FS・任意 URL・shell は経路を提供しない。
- `with_navigation_handler` で `rvf://` 以外（および adapter が
  書き換える http 仮想 host 以外）への遷移を拒否。
- devtools は debug ビルドのみ有効（`cfg!(debug_assertions)`）。
- `with_file_drop`・クリップボード・外部スキーム開放は提供しない
  （本 Issue の API 面に含めない）。
- Intent/params のサイズ上限・op 登録制・`DataSource` は Rust 側が
  生成した名前のみ登録可能（フロントエンドから新規ソースを開けない）。

### ホストライフサイクル契約

- `HostConfig`: title・論理サイズ・初期 URL（`rvf://app/...`）・
  `data_dir`（WebView2 UDF。既定はプロセス固有の一時パス —
  UDF シングルトンロック対策）・`tick`（周期起床。既定なし）・
  `max_chunk_bytes`・`devtools`。
- `HostEvent`: `Ready{url}` / `Message(String)` / `Resized{w,h}` /
  `Focused(bool)` / `ScaleFactorChanged{factor}` / `CloseRequested` /
  `Tick`。
- `HostHandle`: `eval`/`push_event`/`resize`/`focus`/`close`。
  失敗は `Result` で返し panic しない。
- close 契約: `CloseRequested` は観測通知（v1 では拒否権なし —
  アプリが終了処理をしたい場合はイベント内で同期実行）。ループは
  `CloseRequested` または `HostHandle::close` で終了し `run` が返る。
- 高 DPI: tao の `ScaleFactorChanged` をそのまま正規化して転送。
  初期論理サイズは scale 非依存、Resized は物理 px で報告する
  （`__rvf` は `devicePixelRatio` を併記してフロントへ伝える）。

### 描画責務分離

- DOM/コントロール（フォーム・一覧・操作）: フロントエンド
  （Solid 採択・バニラは小 UI 下限）。
- 高頻度描画（波形・スペクトル等）: WebView 内 Canvas2D/WebGL2 —
  Canvas2D は小中規模の互換フォールバック、WebGL2 は 3 エンジン
  共通の実用下限（PoC: 32k 列で Canvas2D 破綻・WebGL2 ~58fps）。
  in-WebView WebGPU は現行スコープ外（secure context 制約）。
- ネイティブ wgpu サーフェス: 重負荷上限向け。`render` モジュールに
  `NativeSurfacePort`（隣接サーフェスの領域記述 + ライフサイクル
  契約）をポートとして定義するが、実装 adapter は `ruviform-gpu`
  として独立 Issue 候補。隣接構成の制約（WebView コンテナ既定
  最上位・DOM 内側合成は PR #1762 待ち）は README/本書に記録。

## Non-Functional Constraints

- `unsafe` 原則禁止（本 Issue の新規コードは unsafe を使わない）。
- 全公開 enum は `#[non_exhaustive]`（公開 enum 規約 ADR）。
- 依存追加は理由を manifest コメントに記録。`Cargo.lock` 追跡継続。
- GUI 依存は `ruviform-ui-webview` + `webview` feature に封じ込め、
  `cargo check --workspace`（既定 feature）が headless でも通ること。
- RT スレッドへの UI 処理・ブロッキング同期の持ち込み禁止。
- AudioLens 固有型（Job/Validity/Confidence/FeatureRecord 等）を
  公開 API に持ち込まない。payload は `serde_json::Value` と
  汎用 `ErrorBody` に限定する。

## Verification

- headless（CI 対象）:
  - `ruviform-ui` 単体: schema round-trip・malformed/過大 intent・
    unknown op・即時応答・Job 状態機械（progress→finished、
    cancel→cancelled、failure→failed、panic→worker_failed、
    latest-progress 合体）・`DataRegistry` 境界値・`MapAssetSource`
    パス安全性・`demo` 決定性・dispatcher を CLI ハーネス経路と
    fake-host 経路で同一イベント列にする GUI/CLI 一致テスト。
  - `ruviform-ui-webview --features webview`: `scheme` ハンドラの
    純粋関数テスト（app 配信・MIME・index 解決・traversal 404・
    data range/cap/CORS・未知名前空間・http 書き換え host 吸収）、
    push シリアライズの JSON エスケープ。ウィンドウ生成を要する
    経路はテストに含めない（CI は表示環境なし）。
- 実機手動（ローカル）: `demo_app` を起動し、プロトコルロード・
  Intent→Job→進捗表示→結果→peaks の `rvf://data` 範囲 pull と
  Canvas2D 描画・cancel・close・フォーカス遷移・リサイズ（DPI 含む）
  を確認し、`docs/impl/` に記録する。`demo_cli` を headless で走らせ
  同一サービスの結果一致を確認する。
- 性能: PoC 実測（pull ~94-146MB/s・RTT・push 合体）を設計前提として
  再利用し、`demo` で 300 秒級 × 2ch の peaks（~23MiB）を生成して
  `rvf://data` 範囲 pull の実測を impl 記録へ残す。メモリは
  `X-Rvf-Total` + 範囲 pull がバルク全量の常駐転送を要求しないことを
  確認する程度に留める。
- バックエンド制限: WebView2 未導入環境では `detect_backend()` が
  `unavailable` を返し、`run` は `BackendUnavailable` で失敗する
  ことを確認（起動時エラー表示の検証）。

## Deferred Design Concerns

独立 Issue 候補（汎用 UI 機能の不足・実機検証の残課題）:

- `ruviform-gpu` wgpu ネイティブサーフェス adapter（`render` ポートの
  実装。隣接構成 + z-order/リージョン制約の取り扱い）。
- in-WebView WebGPU の https 仮想ホスト経路（Windows 実機 +
  macOS/Linux 検証）。
- wry PR #1762 追跡 / ICoreWebView2CompositionController による
  DOM 内側コンポジット（フォーク保守の可否含む）。
- macOS / Linux の実機回帰と配布形態選定（deb/AppImage/Flatpak）。
- WebView レンダラプロセスのクラッシュ検出・自動復帰方針。
- 複数ウィンドウ・メニュー/トレイ等のシェル機能（具体的利用要求後）。
- 型付き TS クライアント（Solid 向け `__rvf` 薄ラッパ）の npm
  パッケージ化。
- #157 相当の汎用表示プリミティブ（LOD/タイル/ビューポート計算）—
  本基盤の `DataSource` 消費側。
- UI 側からのメトリクス/診断送信（`ruviform-observability` 連携）。
