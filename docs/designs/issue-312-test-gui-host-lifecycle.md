---
id: "DESIGN-034S312gtUiTestDsnDD44"
type: "design"
status: "approved"
title: "Issue #312 ruviform-ui-test — test-only GUI host lifecycle・secure control adapter 設計"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/312"
    - "docs/specs/issue-312-test-gui-host-lifecycle.md"
    - "docs/adr/issue-312-test-control-transport-security.md"
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-ui-test"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-312-test-gui-host-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/adr/issue-312-test-control-transport-security.md"
    - "docs/designs/issue-156-gui-host-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-312-test-gui-host-lifecycle.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-312-test-gui-host-lifecycle.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/243"
  defers: []
  deferred-by: []
---

# Issue #312 ruviform-ui-test — test-only GUI host lifecycle・secure control adapter 設計

> **歴史/旧方式（legacy）注記**: `ruviform-ui-test` の FakeDriver /
> driver-lab / `rvf.uitest.v1` lane は現行の driver-neutral 契約検証に使用
> される。`FakeHost` / `HeadlessIo` / `run_multi` 配線など WebView adapter
> 経路の lane は legacy/preview。物理撤去は Issue #465 P2 の別判断。
> 棚卸し正本:
> [issue-465-legacy-webview-inventory](../discovery/issue-465-legacy-webview-inventory.md)。

## Overview

`ruviform-ui-test` は GUI テスト共有基盤クレート。app-under-test 内に埋め込む **agent**（`TestAgent` + `TestIo` ポート）、テスト側 **driver**（`TestDriver`）、headless **fake host**（`FakeHost` + `HeadlessIo`）、`rvf.uitest.v1` wire 契約、fixture/clock/fault/observation の最小ポート、および stdio/TCP transport を提供する。`ruviform-ui` 公開 API のみに依存し、本番クレートは無変更。

## Architecture

```text
test 側 (out-of-process)          app-under-test (in-process)
┌──────────────────┐   frames    ┌──────────────────────────────────┐
│ TestDriver       │  u32le+JSON │ TestAgent::serve(Read,Write)     │
│  spawn_stdio/tcp │◄───────────►│   └─ dispatch test.* ops ──►     │
│  typed ops       │  stdio|TCP  │        TestIo (port)             │
└──────────────────┘             │        ├─ FakeHost(headless)     │
                                 │        └─ run_multi wiring(#313)│
                                 │  Journal + JournalHandle (tee)   │
                                 │  FixtureController (app-supplied)│
                                 └──────────────────────────────────┘
```

## Module map (`crates/ruviform-ui-test/src/`)

| module | contents |
| --- | --- |
| `proto.rs` | `PROTOCOL_VERSION`、op 名定数（`test.*`）、`test_codes` 追加エラーコード、`Welcome`/`Capabilities`/`BackendMeta`/`AppMeta`/`WindowMeta`/`CleanupReport`/`ServeOutcome`/`ShutdownMode`/`ReadyState`/`FaultKind`/`ClockDomain`/`FixtureSpec`/`FixtureReceipt`/`Receipt`/`ReceiptPage`/`Screenshot`/`Notice` の serde 型。wire envelope は `ruviform_ui::ipc::{Intent,Event,ErrorBody}` 再利用 |
| `journal.rs` | `Journal`（bounded `VecDeque`・monotonic seq）+ `JournalHandle<'a>`（`WindowHandle` decorator: eval/resize/focus/create/close/destroy/host ops を `Out`/`Host` receipt として記録してから forward） |
| `fake.rs` | `FakeHost: WindowHandle`（windows map・ready/backlog・close_pending・`ipc_down` set・`inject(WindowEvent)` queue・`pump(&mut FnMut)` で app へ配信・delivered eval 履歴・`FakeWindow` metadata） |
| `headless.rs` | `HeadlessIo<A: TestApp>`（FakeHost + app + journal + fixture map + exited）。`TestIo` 実装: emit→journal+pump、create_window、faults、receipts、request_shutdown |
| `fixture.rs` | `FixtureController` trait + `TestApp` trait（`on_event`/`shutdown` hooks） |
| `agent.rs` | `TestAgent`（session state・caps・enabled・notify seq）+ `handle(&mut dyn TestIo, &str) -> Vec<Event>` + `serve(agent, io, r, w) -> ServeOutcome` + `serve_stdio`/`serve_tcp` + `from_env` |
| `transport.rs` | `read_frame`/`write_frame`（u32le・`MAX_INTENT_BYTES` cap）、env 名定数、`from_env` での transport/secret/disable 解釈、loopback 判定、secret 生成 |
| `driver.rs` | `TestDriver`（child + reader/writer + hello/welcome + `call` 同期 request/response・Notify buffering・typed op API・notifications・`shutdown`/`wait_exit`/kill-on-drop） |
| `bin/dummy_app.rs` | 参照 dummy consumer: `UiRuntime`+`DemoService`+`FakeHost`+`HeadlessIo`+fixtures→`serve_stdio`/`serve_tcp`。`RVF_TEST_CONTROL` で transport 選択 |

## `rvf.uitest.v1` ops（`test.*`・`Intent`/`Event` 再利用）

| op | params → result data | gate |
| --- | --- | --- |
| `test.hello` | `{run_id, driver?, secret?}` → `Welcome` | 必ず最初。session 済・secret 不一致 → fail-closed |
| `test.capabilities` | `{}` → `Capabilities`+runtime(`sys.capabilities` 経由の jobs/ops 数) | auth |
| `test.fixture.prepare` | `{name, seed?, params?}` → `FixtureReceipt` | cap `fixture` |
| `test.clock.advance` | `{domain:"host_tick", ticks}` → `{domain, advanced}` | cap `clock_domains` 含有 |
| `test.fault.set` | `{kind:"window_crash"|"ipc_disconnect", window}` → `{applied}` | cap `faults` 含有 |
| `test.fault.clear` | `{window}` → `{cleared}` | auth |
| `test.window.request_close` | `{window}` → `{requested}` | auth（host-level close = `WindowHandle::request_close` 模倣。ユーザー操作注入ではない） |
| `test.observe.windows` | `{}` → `{windows:[WindowMeta]}` | cap `observation.windows` |
| `test.observe.receipts` | `{since?, limit?}` → `ReceiptPage` | cap `observation.receipts` |
| `test.observe.screenshot` | `{window}` → `Screenshot` または `unavailable` | cap `screenshot` |
| `test.notices` | `{since?}` → `{notices:[Notice]}` | cap `observation.notices`（async 窓口・window created/destroyed/closing） |
| `test.shutdown` | `{mode:"graceful"|"now"}` → `{bye:true, cleanup:CleanupReport}` → stream close | auth |

- 非 `test.*`・未知 op → `unknown_op`。`test.hello` 前の他 op → `unauthenticated`。`shutdown` 後 → session ended で stream close。
- `Event::Notify` を agent→driver の非同期 notice に再利用（`notice` 名 + window/seq）。

## TestIo port（app/host 側 trait）

```rust
pub trait TestIo {
    fn ready_state(&self) -> ReadyState;            // Ready | Failed{reason}
    fn capabilities(&self) -> IoCapabilities;       // fixture/clock_domains/faults/observation/screenshot — 事実のみ
    fn backend(&self) -> BackendMeta;               // "headless-fake" | "wry+tao/..." 等
    fn emit(&mut self, ev: WindowEvent);            // host-originated event を app へ注入
    fn windows(&self) -> Vec<WindowMeta>;
    fn create_window(&mut self, config: &WindowConfig) -> Result<WindowId, ErrorBody>;
    fn receipts(&self, since: u64, limit: usize) -> ReceiptPage;
    fn notices(&mut self) -> Vec<Notice>;
    fn set_fault(&mut self, kind: FaultKind, window: Option<WindowId>, on: bool) -> Result<(), ErrorBody>;
    fn prepare_fixture(&mut self, spec: &FixtureSpec) -> Result<FixtureReceipt, ErrorBody>;
    fn screenshot(&mut self, window: WindowId) -> Result<Screenshot, ErrorBody>; // 既定 unavailable
    fn request_shutdown(&mut self, mode: ShutdownMode) -> CleanupReport;
}
```

`TestApp`（consumer が実装）: `on_event(&mut self, WindowEvent, &mut dyn WindowHandle)`（= 通常配線と同じ）+ `shutdown(&mut self, mode, wh) -> Value`（app 側 cleanup 報告）+ `close_decision(&mut self, WindowId) -> CloseDecision`（dummy は title `keep-*` → Veto）。

## FakeHost 設計

- `windows: HashMap<WindowId, FakeWindow{config, ready, close_pending, backlog}>`、`inject: VecDeque<WindowEvent>`、`ipc_down: HashSet<WindowId>`、`delivered: Vec<(WindowId,String)>`、`exited`。
- `inject(ev)` → queue;`pump(drain)` → queued `Windowed{w,Message}` && `ipc_down∋w` → journal に `suppressed` receipt を残し app へ届けない。`Tick`/`Windowed` → `on_event`。
- `destroy_window` → remove + `Windowed{w,Destroyed}` を inject queue へ（実 host の queue 経由 `Destroyed` と同型）。
- `request_close` → `Windowed{w,CloseRequested}` inject → app の close policy（`respond_close`）で Approve/Veto/Defer。
- `mark_ready(w)` → `ready=true` + backlog flush + `Windowed{w,Ready}` inject（host 側 bookkeeping は実 host の `emit` 前処理と同型）。

## Serve ループ（`agent::serve`）

```text
loop {
  frame = read_frame(r)?           // EOF → on_disconnect: request_shutdown(graceful) → ServeOutcome::DriverGone
  intents = parse_intent(frame)?   // malformed → Event::error を返して継続
  events = agent.handle(io, intent)// handshake gate → op dispatch → Vec<Event>（Notify は io.notices() drain を毎回添える）
  write_frame(events each)         // test.shutdown → bye 書き込み後 break → ServeOutcome::Shutdown{report}
}
```

- `serve_stdio(agent, io)`: stdin/stdout をそのまま使う（app は stderr へ log）。`RVF_TEST_DISABLE=1` → `ServeOutcome::Disabled`。
- `serve_tcp(agent, io, addr, expected_secret)`: `TcpStream::connect(addr)` → `expected_secret: Some` 必須（`None` → 拒否）。peer addr が loopback でない → 拒否。secret は `Hello` 照合で agent が検査。
- `TestAgent::from_env()`: `RVF_TEST_CONTROL`（`stdio`|`tcp`|`off`/unset）・`RVF_TEST_ADDR`・`RVF_TEST_SECRET`・`RVF_TEST_DISABLE` を解釈し `AgentTransport` を返す。app はこれで serve 経路を選ぶ。

## TestDriver

- `spawn_stdio(cmd)`: `Command::stdin(piped).stdout(piped).stderr(inherit)` → `test.hello{run_id}` → `Welcome` or fail。
- `spawn_tcp(cmd)`: `TcpListener::bind(127.0.0.1:0)` → env 設定 → spawn → `accept()` timeout → peer loopback 検査 → `test.hello{run_id, secret}`。
- `call(op, params)`: `Intent{id}` 書き → frame を読み `Result{id}` まで loop、途中の `Notify`/`Error` は buffer/返却。
- `shutdown(mode)` → bye reply + EOF + `child.wait()` を確認し `CleanupReport` を返す。`wait_exit(timeout)`、`Drop` で生きている子を `kill()`。
- `restart(cmd)`: shutdown → spawn → 新 `session_id`。

## Security / fail-closed 一覧

- 認証前 op → `unauthenticated`;session 中の再 `test.hello` → `unauthenticated`（再 handshake 不可）;TCP secret 不一致/欠落 → `unauthenticated` + close。
- `RVF_TEST_SECRET` 未設定の TCP serve → 拒否（`ServeOutcome::Disabled{reason}` 相当の Err）。
- capability 未宣言 op（`fixture`/`clock domain`/`fault kind`/`observation`/`screenshot`）→ `unavailable`。
- `MAX_INTENT_BYTES` 超 frame → `message_too_large`（transport 層で read を失敗扱いにせず frame skip? — 設計: cap 超過は `ErrorBody` を返して接続継続。driver 側も同規則）。
- `RVF_TEST_DISABLE` → serve 即時拒否。`TestAgent::set_enabled(false)` 同効。
- op 集合は enum — wire から任意 eval/shell/file へ到達する経路なし。

## Clock 制御の正直な宣言

`capabilities.clock_domains = ["host_tick"]` のみ。`test.clock.advance` は `WindowEvent::Tick` の合成注入（job drain point を前進）で、OS timer・wall clock・GPU/vsync・webview 内 timer は一切制御しない — `ClockDomain` 拡張時もこの範囲を超える宣言をしない。

## HeadlessIo の TestIo 実装要点

- `emit`: journal `In` 記録 → `host.inject` → `pump`（app.on_event を `JournalHandle` wrap した host で呼ぶ）。
- `windows`: FakeHost 管理 metadata（id/title/size/ready/close_pending/ipc_down）。
- `set_fault`: `window_crash`→`destroy_window`+pump;`ipc_disconnect`→`ipc_down` toggle。
- `request_shutdown`: `app.shutdown(mode, wh)` で cleanup 詳細を回収 → 残 window を全 destroy + exited=true → `CleanupReport{windows_open_before, jobs_running, exited}`（jobs_running は `sys.capabilities` 経由で取得）。
- `screenshot`: `IoCapabilities.screenshot=false` を宣言し既定 `unavailable`。
- `notices`: create/destroy/closing を pump 中に収集。

## dummy_app（src/bin）

`UiRuntime`+`DemoService`+`FakeHost`+`HeadlessIo`。`on_event` = `rt.handle_window` → `wh.eval(dispatch_script)` 配送 + `host.inject` 経路を介した Ready/Tick 処理。close policy: title `keep-*` → Veto、他 → Approve。fixtures: `single`（1 window+seed source）/`dual`（2 window）/`keep`（veto window）/`sleep-job {ms}`（demo.sleep intent 注入）。transport は `TestAgent::from_env`。

## 依存

`ruviform-ui = { path = "../ruviform-ui" }`、`serde`、`serde_json` のみ（新規外部依存なし — 全て既存 workspace で使用中）。

## RT・layer 整合

利用側層（cli-sample 同順位）。全 API 非 RT。`ruviform-ui` → `ruviform-ui-test` の逆向き依存なし・他クレートからの依存なし（DAG 無環境維持）。
