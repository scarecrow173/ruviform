---
id: "DESIGN-BdiZY3GtD44RrVDtpLFcFx"
type: "design"
status: "approved"
title: "Issue #181 Multi-Window Host — per-Window event & lifecycle contract"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/181"
  changes:
    added:
      - "crates/ruviform-ui/src/window.rs"
      - "crates/ruviform-ui/tests/multi_window.rs"
      - "crates/ruviform-ui-webview/examples/demo_multi.rs"
    modified:
      - "crates/ruviform-ui/src/event.rs"
      - "crates/ruviform-ui/src/runtime.rs"
      - "crates/ruviform-ui/src/service.rs"
      - "crates/ruviform-ui/src/lib.rs"
      - "crates/ruviform-ui-webview/src/host.rs"
      - "crates/ruviform-ui-webview/src/lib.rs"
      - "crates/ruviform-ui/README.md"
      - "crates/ruviform-ui-webview/README.md"
      - "crates/README.md"
      - "crates/ruviform-ui-webview/Cargo.toml"
      - "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-181-multi-window.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-156-gui-host-foundation.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/adr/plugin-gui-embedding.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-181-multi-window.md"
    - "docs/adr/issue-181-multi-window-contract-placement-close-decision-ipc-routing.md"
    - "docs/discovery/issue-181-multi-window-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/182"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/synvelis/issues/21"
    - "https://github.com/scarecrow173/synvelis/issues/22"
  defers: []
  deferred-by: []
metadata:
  gate: "M7"
  issue: 181
---

# Issue #181 Multi-Window Host — per-Window event & lifecycle contract

## Context

Issue #156 で導入した `ruviform-ui`/`ruviform-ui-webview` は単一 Window・単一 WebView 前提だった。Issue #181 は Synvelis の正式要件（複数ネイティブ Window・複数モニター配置）に基づき、**単一ネイティブイベントループ上で複数 Window/WebView を管理する汎用基盤**へ拡張する。ADR `issue-181-multi-window-contract-placement-close-decision-ipc-routing`（accepted）で「additive envelope 方式」を選択済み — `HostEvent` 自体は変えず、`WindowId` とイベント envelope、宛先付き `RoutedEvent`、`WindowHandle` port を追加し、単一 Window 経路は構造的に不変とする。

## Scope

### In Scope

- `ruviform-ui` 新モジュール `window`: `WindowId`、`WindowEvent`、`RoutedEvent`、`CloseDecision`、`WindowConfig`、`MonitorInfo`、`WindowHandle` trait。
- `HostEvent` への additive variant: `Moved{x,y}`、`Destroyed`。
- `UiRuntime` multi-window 経路: `handle_window`、`drain_jobs_routed`、`cancel_window_jobs`、Job owner 記録、`ServiceContext::origin`。
- `ruviform-ui-webview` への `SessionConfig` + `run_multi`（`webview` feature 配下）: Window レジストリ・共有 `WebContext`・動的 Window 生成・close 決定機構・pre-`Ready` bounded 保留 push。
- headless/fake-host テスト（`crates/ruviform-ui/tests/multi_window.rs`）。
- `demo_multi` example（2 Window 生成・追加・サブ Window 閉鎖・last-window policy 例。実機手動検証用）。
- doc チェーンと索引・README・mise/CI の整合確認（新規タスクは不要 — 既存 `*-ui-webview` レーンに収まる）。

### Out of Scope

- Synvelis 固有: Arrangement/Mixer/Inspector 配置・Dock Tree・View 移動（#182）・Command Router。
- プラグイン規格ネイティブ GUI 親子関係（#30/#115 の既存契約）。
- WebView レンダラクラッシュ自動復帰、メニュー/トレイ等シェル機能。
- `rvf.ipc.v1` wire 形式の変更（明示的に行わない — ADR 参照）。
- `host::run` / `UiRuntime::handle` 等の単一 Window 既存経路の変更。

## Components and Boundaries

### `ruviform-ui::window`（新規モジュール）

```rust
/// 非ゼロ・セッション内一意・単調採番・破棄後再利用しない Window 識別子。
/// host-side routing token であり rvf.ipc.v1 payload には乗らない。
pub struct WindowId(NonZeroU64);  // Serialize/Deserialize + Display

/// Multi-window host が emit するイベント envelope。
pub enum WindowEvent {          // #[non_exhaustive]
    /// 単一 Window 由来のイベント。
    Windowed { window: WindowId, event: HostEvent },
    /// ループ全体共有の周期起床（job drain 点）。Window に紐付かない。
    Tick,
}

/// Runtime が返す「宛先付きイベント」。
pub struct RoutedEvent {
    /// 宛先 Window。None = unrouted（headless/単一Window由来、
    /// Window 非所有 job）— consumer が配送先を決める。
    pub window: Option<WindowId>,
    pub event: Event,
}

/// CloseRequested への回答。
pub enum CloseDecision {        // #[non_exhaustive]
    /// 即破棄。`Destroyed` が続く。
    Approve,
    /// close を取消。Window は通常状態で生存。
    Veto,
    /// 延期: Window を close-pending で生存させる。consumer は後で
    /// `destroy_window` か次回 CloseRequested の承認を行う。
    Defer,
}

/// 新規 Window 生成パラメータ。
pub struct WindowConfig {       // #[non_exhaustive]
    pub title: String,
    pub width: u32,             // logical
    pub height: u32,            // logical
    pub url: String,
    pub devtools: bool,
}

/// バックエンド非依存のモニター情報。
pub struct MonitorInfo {        // #[non_exhaustive]
    pub name: Option<String>,
    pub width: u32,             // physical px
    pub height: u32,            // physical px
    pub x: i32,                 // physical, virtual desktop coords
    pub y: i32,
    pub scale_factor: f64,      // 1.0 = 100%
}

/// Multi-window host の駆動ポート。adapter と fake-host が実装。
pub trait WindowHandle {
    /// fire-and-forget eval。不明/破棄済み宛先は silent drop —
    /// プロセスを壊さない。not-ready は bounded 保留。
    fn eval(&mut self, window: WindowId, script: &str);
    fn resize(&mut self, window: WindowId, w: u32, h: u32) -> Result<(), UiError>;
    fn focus(&mut self, window: WindowId, focused: bool) -> Result<(), UiError>;
    /// 新規 Window+WebView 生成。失敗は Err で Window を残さない。
    fn create_window(&mut self, config: &WindowConfig) -> Result<WindowId, UiError>;
    /// ユーザー X ボタン相当の close 要求 — veto-able な
    /// `CloseRequested` 経路へ回す（即破棄しない）。
    fn request_close(&mut self, window: WindowId);
    /// pending な CloseRequested への回答。
    fn respond_close(&mut self, window: WindowId, d: CloseDecision) -> Result<(), UiError>;
    /// 即破棄（CloseRequested を bypass）。`Destroyed` が続く。
    fn destroy_window(&mut self, window: WindowId) -> Result<(), UiError>;
    /// 生存 Window ID の列挙（last-window policy 用）。
    fn windows(&self) -> Vec<WindowId>;
    /// 接続モニター一覧。
    fn monitors(&self) -> Vec<MonitorInfo>;
    /// ループ終了（アプリ終了 — consumer ポリシー）。
    fn exit(&mut self);
}
```

### `HostEvent` への追加（additive、`#[non_exhaustive]` 維持）

```rust
/// Window が移動した。物理座標（virtual desktop）。
Moved { x: i32, y: i32 },
/// Window/WebView が破棄された終端通知 — 承認済み close・
/// programmatic destroy・異常 teardown のすべてを含む。
/// この WindowId は以後再利用されず、宛先指定配送は drop される。
Destroyed,
```

`HostEvent::CloseRequested` の doc を更新 — 単一 Window 経路では従来どおり（v1: veto なし・ループ終了）だが、multi-window 経路では `CloseDecision` 対象の要求であり即破棄/Job 取消を意味しないことを明記。

### `UiRuntime` への追加

```rust
impl UiRuntime {
    /// Multi-window 入口。`handle` と異なり:
    /// - 返却は全て `RoutedEvent`（宛先 = 発信 Window）。
    /// - `Message` から spawn した Job は owner = 発信 Window を記録
    ///   （イベント routing のみに使用、寿命制御には使わない）。
    /// - `CloseRequested` は `lifecycle closing` を宛先 Window へ
    ///   返すが Job は一切取消しない（window close ≠ job cancel）。
    /// - `Tick` は `drain_jobs_routed()` と同義（ループ共有）。
    pub fn handle_window(&mut self, w: WindowId, ev: HostEvent) -> Vec<RoutedEvent>;

    /// Job mailbox drain → owner Window 宛の `RoutedEvent`。
    /// owner なし（`dispatch_op`/`handle` 由来）は `None` unrouted。
    pub fn drain_jobs_routed(&mut self) -> Vec<RoutedEvent>;

    /// Window 所有 Job の opt-in 取消 — `CloseRequested`/`Destroyed`
    /// からは自動で呼ばれない。「Window-owned Job」語彙の提供。
    /// 戻り値は cancel 要求を立てた Job 数。
    pub fn cancel_window_jobs(&mut self, w: WindowId) -> usize;
}
```

- `JobRecord` に `owner: Option<WindowId>` を内部追加。`spawn_job` は dispatch 経路ごとの owner を記録（`handle_window`→`Some(w)`、`handle`/`dispatch_op`→`None`）。
- `ServiceContext` に `origin: Option<WindowId>` を additive 追加 — Service が発信 Window を識別できる（複数 Window op の前提）。
- 既存 `handle`/`dispatch_op`/`drain_jobs`/`poll` はシグネチャ・セマンティクス不変。**2 経路の混用は usage error** — `handle(Tick)` は owner に関係なく全 Job のイベントを untagged で返すため、multi-window 利用側は `handle_window`/`drain_jobs_routed` のみを使う。

### `ruviform-ui-webview` への追加（`webview` feature）

```rust
pub struct SessionConfig {
    /// 共有 WebContext の user-data dir（WebView2 UDF シングルトン —
    /// per-Window ではなくセッション設定）。None でプロセス固有 temp。
    pub data_dir: Option<PathBuf>,
    /// `WindowEvent::Tick` の周期（job drain 起床）。None = なし。
    pub tick: Option<Duration>,
    /// 起動時に開く Window 群（1 基以上必須ではない — 0 も許可）。
    pub windows: Vec<WindowConfig>,
}

pub fn run_multi<F>(
    session: &SessionConfig,
    assets: Arc<dyn AssetSource>,
    data: DataRegistry,
    on_event: F,
) -> Result<(), UiError>
where
    F: FnMut(WindowEvent, &mut dyn WindowHandle);
```

実装構造（`host.rs` 内、`host::run` とは別経路）:

- **Window レジストリ**: `HashMap<WindowId, Entry{window: tao::Window, webview: WebView, ready: bool, pending: VecDeque<String>, close_state}>` + `HashMap<tao::window::WindowId, WindowId>` の双方向写像。`next_id` 単調採番（`1` 開始、再利用しない）。
- **共有 `WebContext`**: `run_multi` 内で 1 基生成（`data_dir` 既定 = `rvf-udf-multi-<pid>`）。全 `WebView` が `new_with_web_context` で同一 `WebContext` を共有（WebView2 UDF シングルトン・WebKitGTK セッション共有の正規構成）。`WebContext` は `run_return` クロージャ外の `run_multi` ローカルに置き、全 WebView より長く生存させる。
- **イベント配線**: 各 WebView の `with_ipc_handler`/`with_on_page_load_handler` が `WindowId` 付きで共有キューへ push → `EventLoopProxy` 起床 → `UserEvent` で `Windowed{w, ...}` を発生順に emit。`WindowEvent{window_id}` → tao id → `WindowId` 変換 → `Windowed` emit。
- **close 決定機構**: `WindowEvent::CloseRequested` で `close_state = Requested` にして `Windowed{w, CloseRequested}` emit。`respond_close` は `Approve`→即 `destroy_window`（entry drop + `Destroyed` enqueue）、`Veto`→state クリア、`Defer`→`ClosePending` へ。emit 後に `Requested` のまま残った Window は既定 `Approve` で破棄。
- **動的生成**: `create_window` は `&EventLoopWindowTarget`（`run_return` クロージャの第 2 引数）で `WindowBuilder::build` → `build_webview`（`run` の webview 構築を共有 WebContext 版へ一般化）→ entry 登録 → `Ok(id)`。失敗時は Window drop で巻き戻し `Err`。
- **pre-`Ready` 保留**: `eval` は `ready=false` なら `pending` VecDeque（上限 `PENDING_EVAL_CAP=64`、drop-oldest）へ積み、`Ready` 受信で flush。宛先不明/破棄済みは silent drop。
- **ループ終了**: `WindowHandle::exit()` で `want_exit` → `MainEventsCleared` で `ControlFlow::Exit`。**Window 全滅では終了しない** — consumer が `windows()` を見て `exit()` を呼ぶポリシー。
- `request_close` は `Windowed{w, CloseRequested}` の再 emit（決定経路を通す）。tao にプログラム的 close-request 発行 API はなく、同一経路の再現で代替する。

## Data and Control Flow

### イベント経路（multi-window）

```text
tao WindowEvent{window_id} ─┐
webview IPC/pageload cb     ├→ shared VecDeque → proxy wake →
                            │   on_event(Windowed{w, ev}, &mut dyn WindowHandle)
ResumeTimeReached (tick) ───┘   on_event(Tick, ...)
```

```text
consumer:
  Windowed{w, Message}  → rt.handle_window(w, Message) →
                          RoutedEvent{Some(w), result/job-ack} → wh.eval(w, dispatch_script)
  Windowed{w, CloseRequested} → rt.handle_window → RoutedEvent{Some(w), lifecycle:closing}
                        → consumer が respond_close(w, Approve|Veto|Defer)
                        → Approve で Destroyed emit
  Tick → rt.drain_jobs_routed() → RoutedEvent{Some(owner)|None} → eval
```

### Wire 形式（不変の明示）

`rvf.ipc.v1` は `v:1`・envelope・error codes・`id` 相関規約を一切変更しない。`WindowId`/`RoutedEvent` は host-side routing のみで serialize しない — JS `id` 相関（`i-<seq>`）は WebView ローカル一意で十分であり、応答は発信元 WebView へ返される。WebView 間で `i-1` が衝突しても routing が分離するため正しい宛先へ届く。

### Job / Window 寿命の分離

- Job は `owner: Option<WindowId>` を routing 用にのみ持つ。Window 破棄後の Job event は宛先不明として配送層が silent drop — Job 自体は継続する。
- `handle_window(CloseRequested)` は Job を取消さない（`handle` との決定的差異）。
- 「Window-owned Job」は consumer が `Destroyed`/`CloseRequested` に対して `cancel_window_jobs(w)` を呼ぶ opt-in。長寿命 Job（Application/Project 所有）は何もしなければ Window Close と無関係に動き続ける。

### 失敗・フォールバック規約

| 状況 | 規約 |
| --- | --- |
| `create_window` 失敗 | `Err(UiError)`、Window は残存しない、イベントは発行されない |
| `eval` → 不明/破棄済み | silent drop（fire-and-forget、プロセス不壊） |
| `eval` → not-ready | bounded `pending`（cap 64、drop-oldest）、`Ready` で flush |
| `resize`/`focus`/`respond_close`/`destroy_window` → 不明/破棄済み | `Err(UiError::Host)` |
| `respond_close` → pending でない Window | `Err(UiError::Host)`（`Defer` 済み Window は `Approve`/`destroy_window` を受理） |
| backend teardown / Window loss | `Destroyed` で終端化（クラッシュ復帰は本 Issue 外） |
| 未回答 `CloseRequested` | 既定 `Approve`（その Window のみ破棄、ループ継続） |

### 順序・ID 規約

- adapter はイベント発生順にキュー配送する。`Destroyed` は同 Window の最終イベント — 以後同 ID のイベント・配送は発生しない/受理されない。
- `WindowId` は `1` 始まりの単調採番、破棄後も再利用しない（stale ID の誤配送を防止）。
- 同一ターン内の `CloseRequested` 再発行は抑止しない — consumer の `Veto`/`Defer` がなければ既定 `Approve` が効く（連打でも冪等）。

## Non-Functional Constraints

- `unsafe` 不使用。全公開 enum は `#[non_exhaustive]`。
- `ruviform-ui` は backend 非依存を維持 — `WindowId`/`WindowConfig`/`MonitorInfo`/`CloseDecision`/`WindowHandle`/`RoutedEvent`/`WindowEvent` に wry/tao/dpi/http 型を含めない。
- `ruviform-ui-webview` の追加は `webview` feature 配下のみ — headless `cargo check --workspace` を壊さない。新規 feature・optional 依存は追加しない。
- UI スレッド上で同期 I/O・長時間処理をしない。Job worker は UI/Window を直接触らない（既存契約の延長）。
- RT 側から UI/Window 操作を呼ばない — 両クレート非 RT 専用のまま。
- DAW/Dock/Project 語彙を API に含めない（generic のみ）。

## Verification

- headless/fake-host（`crates/ruviform-ui/tests/multi_window.rs`、CI 対象）:
  - 2 Window のイベント分離 — w1 の intent 応答は w1 のみ、w2 へは届かない。
  - 複数 WebView での `id` 衝突 — 両 Window が `i-1` を使っても各々の宛先へ routing。
  - `CloseRequested` の全経路 — `Approve`→`Destroyed`+宛先 drop、`Veto`→生存+配送継続、`Defer`→close-pending→後続 `destroy_window`/`Approve` で終端。
  - 未回答既定 — `CloseRequested` 放置で Window は `Approve` 扱い。
  - 破棄済み宛先配送 — `eval`/fallible op が panic せず silent drop/`Err`。
  - Job routing — owner Window 宛の `RoutedEvent`、owner 破棄後は安全 drop、Window Close で Job が自動取消されない、`cancel_window_jobs` の opt-in。
  - 単一 Window baseline — 既存 `close_requested_cancels_jobs_and_emits_lifecycle` が非回帰。
- adapter（`webview` feature）: `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` green — `run_multi`/`SessionConfig` の型・lint は headless 検証、窓付き実動作は手動。
- 実機手動: `demo_multi` で 2 Window 生成・独立フォーカス・サブ Window 閉鎖・別モニター移動（DPI）を実施した範囲のみ `docs/impl` に記録。未検証 OS/条件は `unverified` と明示。
- `demo_app`/`demo_cli`・`runtime` テスト群の非回帰（`mise run test` green）。

## Deferred Design Concerns

- `NativeSurfaceHost` の multi-window 対応（`ruviform-gpu` 候補と同じ境界）。
- WebView レンダラクラッシュ検出・自動復帰。
- `notify` の per-Window/broadcast ポリシー拡張。
- WebView2 UDF の複数プロファイル分離。
- メニュー/トレイ/タスクバー等シェル機能。
- 破棄済み宛先配送の可観測性（drop counter — PoC では silent drop 規約）。
