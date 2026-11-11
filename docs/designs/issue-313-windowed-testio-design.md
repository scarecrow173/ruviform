---
id: "DESIGN-34WIND0WEDTESTI0S2DE"
type: "design"
status: "approved"
title: "Issue #313 — windowed `TestIo` 設計（neutral 語彙統一 + GPUI adapter）"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-313-windowed-testio-plan.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/windowed_io.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/bin/windowed_test_app.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/io.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/fixture.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/headless.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/journal.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/agent.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/fake_driver.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-313-windowed-testio.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-313-windowed-testio.md"
    - "docs/designs/issue-314-real-window-adapter.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-465-legacy-webview-p0p1.md"
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — windowed `TestIo` 設計

## 設計判断 1: 語彙境界 — Option A neutral 再表現（固定）

`TestIo` が legacy 語彙（`WindowEvent`/`WindowHandle`/`WindowConfig`）
のままでは、`GpuiDriver`（`DriverEvent`/`WindowDriver`/`WindowSpec`
/typed `Event` のみ返す）上に adapter を書けない。選択肢は:

| 案 | 内容 | 判定 |
| --- | --- | --- |
| A. neutral 再表現 | `TestIo`/`TestApp`/`Journal` を neutral 語彙へ統一し `HeadlessIo` を `FakeDriver` 上に再基盤化 | **採用**（task で固定済） |
| B. legacy shim | GPUI 側に `WindowDriver → WindowHandle` の逆変換 shim を置く | **不採用** — #465「driver-neutral lane が正本」と矛盾。typed `Event` 情報を legacy `String` body へ逆変換する地点で情報損失 + 第 3 経路の維持コスト |
| C. `TestIo` を generic 化 | `TestIo<Ev, H>` の型パラメータ化 | **不採用** — proto op と 1:1 の契約に多態性は不要（YAGNI）。2 backend が同じ語彙を話せばよい |

Option A の境界:

```rust
pub trait TestIo {
    fn ready_state(&self) -> ReadyState;
    fn capabilities(&self) -> Capabilities;
    fn backend(&self) -> BackendMeta;
    fn emit(&mut self, ev: DriverEvent);
    fn emit_ipc_body(&mut self, window: WindowId, body: &str);
    fn windows(&self) -> Vec<WindowMeta>;
    fn create_window(&mut self, spec: &WindowSpec) -> Result<WindowId, ErrorBody>;
    fn receipts(&self, since: u64, limit: usize) -> ReceiptPage;
    fn notices(&mut self) -> Vec<Notice>;
    fn set_fault(&mut self, kind: FaultKind, window: Option<WindowId>, on: bool)
        -> Result<(), ErrorBody>;
    fn prepare_fixture(&mut self, spec: &FixtureSpec) -> Result<FixtureReceipt, ErrorBody>;
    fn screenshot(&mut self, window: WindowId) -> Result<Screenshot, ErrorBody>;
    fn request_shutdown(&mut self, mode: ShutdownMode) -> CleanupReport;
}

pub trait TestApp {
    fn on_event(&mut self, dev: DriverEvent, drv: &mut dyn WindowDriver);
    fn shutdown(&mut self, mode: ShutdownMode, drv: &mut dyn WindowDriver) -> Value;
}
```

`TestIo`/`TestApp`/`JournalHandle` が露出する語彙は全て
`ruviform_ui` の neutral 型。GPUI 具象型（`App`/`Entity`/handle）は
`windowed_io.rs` 内部に閉じる。

## 設計判断 2: `emit_ipc_body` — malformed body の行方

既存 `IpcEmitFixture` は `"not-json"` を含む任意 raw body を
`HostEvent::Message(String)` として注入し、runtime の malformed
経路（error event）を検証している。neutral 化で
`Action(Intent)` は decode 済みなので、raw body 注入には別の
契約穴が必要。選択肢:

| 案 | 内容 | 判定 |
| --- | --- | --- |
| A. `emit(Action)` のみ | fixture は typed Intent のみ注入可 | **不採用** — malformed 入力の fail-closed 検証が消え、e2e_process の `"not-json"` 契約を再現できない |
| B. `emit_ipc_body(window, &str)` を port に追加 | raw body を app 境界で decode: armed → suppressed、valid → `Action` を通常経路へ、malformed → driver 側 `post_event(Event::error)` で応答 | **採用** |

B の semantics は **実 adapter（webview session.rs）の実挙動と同一**:
malformed な inbound body は session 層で `Event::error` に応答
され、consumer の `on_event` には届かない。receipt 語彙:

- armed（`ipc_disconnect`）: `In "suppressed"` receipt +
  `ipc.suppressed` notice（decode 前に drop — real boundary の
  channel-down と同一）。
- unarmed + valid body: decode → `Action` として通常配送 →
  interceptor が `In "message"` receipt を記録。
- unarmed + malformed body: `In "message"` receipt（body は
  境界を越えた）+ `Out "post_event"` receipt +
  `Event::error` を window inbox へ。

`emit_ipc_body` は infallible — 結果は journal/notice/inbox で
観測する（`emit` と同じ observe 契約）。

## 設計判断 3: Journal の neutral 化

- `Journal::record_in(&DriverEvent)`: `Tick` → `"tick"`,
  `Windowed{Created→"created", Ready→"ready", Action→"message",
  Resized→"resized", Moved→"moved", Focused→"focused",
  ScaleFactorChanged→"scale_factor_changed",
  CloseRequested→"close_requested", Destroyed→"destroyed",
  _→"other"}`。
- `JournalHandle: WindowDriver` — `create_window`/`resize`/`focus`/
  `request_close`/`respond_close`/`destroy_window`/`post_event`/
  `exit` を `Out` receipt として記録してから forward。
  `windows`/`monitors`/`capabilities` は read-only で journal
  しない（legacy `WindowHandle` 版と同じ方針）。
- `deliver_routed(drv, &RoutedEvent)` → `drv.post_event(window,
  &re.event)`（legacy `eval` の typed 置き換え）。
- `record_suppressed(window, bytes)` — bytes は呼び出し側が決める
  （Intent なら params 直列化長、raw body なら body.len()）。

## 設計判断 4: `HeadlessIo` on `FakeDriver`

`FakeDriver` は neutral 语义を既に提供する（bounded backlog・
drop-oldest・monotonic id・pending close・terminal `Destroyed`・
`ipc_disconnect` suppression）が、headless の semantic 差分を
以下で吸収する:

| semantic | headless での実現 |
| --- | --- |
| `emit` | `host.inject(dev)` + `pump_events()`（synchronous・FakeHost と同じ determinism） |
| `pump_events` | `host.pump(|dev, drv| intercept)` — suppressed `Intent` は pump が `(WindowId, Intent)` で返し、io が suppressed receipt + notice を記録 |
| `windows()` | `host.windows()` + `spec_of`（title/size）+ `ready()`（**新規 accessor**）+ `close_pending` + `is_ipc_down` — fake の実 state を読む（shadow なし） |
| `create_window` | `host.create_window(spec)` → `Created` queue → 即 pump → interceptor が notice `window.created` + `In "created"` receipt |
| `window_crash` | `host.destroy_window(w)` + pump → terminal `Destroyed` |
| `ipc_disconnect` | `host.ipc_disconnect(w)`（window 単位）+ io 側 `all: bool`（global flag—`FakeDriver` は per-window のみ） |
| `request_shutdown` | `app.shutdown(mode, jh)` → 全 window `destroy_window` + pump → `host.exit()` → CleanupReport |
| screenshot | 変わらず `unavailable`（headless は capture を持たない） |

`FakeDriver` に追加するのは `ready(&self, WindowId) -> bool`
accessor 1 点のみ（slot ready を read）。`ipc_disconnect` は
FakeDriver 側の pump-suppression をそのまま使う — fault 状態は
「host 側の channel 断」を fake が model する方が real boundary
に近い。

### lifecycle notice の発生源を統一

`window.created`/`window.close_requested`/`window.destroyed`/
`ipc.suppressed`/`app.closing` の各 notice を、両 backend で
**interceptor の lifecycle event 配送**から生成する
（`HeadlessIo::create_window` が直接 push する旧形をやめる）。
Created notice は「host が window を受け付けた」event そのものを
契機とするのが正直で、windowed 側（`Op::Create` → `open_window`
成功 → `Created`）と同じ観測契約になる。

## 設計判断 5: windowed `TestIo` の構造 — control thread ⇄ UI thread

`GpuiDriver` は `Rc<RefCell<Inner>>` で `Send` でない — control
thread（`TestAgent` serve 側）が driver を直接触れない。既存
`UiJob` seam（`set_ui_jobs`/`drain_ui_jobs`、job = `Send` closure
が UI thread の `App` context で `Inner`/`GpuiDriver` を触る）を
再利用する。

```text
control thread (WindowedIo : TestIo)          UI thread
  io.windows()/emit()/create_window() ──jobs──> drain_ui_jobs
  <── oneshot reply / shared state ──    ├─ UiSide::deliver
  receipts()/notices()  (shared read)    │   (interceptor: journal
                                         │    + notices + ipc_down
                                         └─► filter + app.on_event)
```

- `WindowedIo::new(driver) -> (WindowedIo, UiSide)`:
  `set_ui_jobs` に receiver を install し、shared state（`Arc<Mutex>`）
  を両側に持たせる。AUT は `UiSide::deliver` を `GpuiDriver::run` の
  handler として渡す。
- shared state: `journal`・`notices`(+seq)・`ipc_down:
  BTreeSet<WindowId>`（per-window のみ — `window=None` の global
  arm は現契約どおり `invalid_params`）・
  `specs: BTreeMap<WindowId,(title, LogicalSize)>`・
  `pending_shutdown: Option<{mode, opened_before}>`。

### deterministic job パターン

- `call(job) -> Result<T, ErrorBody>`: oneshot reply 付き job を送り
  `recv_timeout` で待つ。read op は job 内で即 reply。
- **action job + confirm job** の 2 段を全 mutating op に適用する:
  action job が driver `Op` を enqueue すると、次 drain の ops
  loop で適用される。`Op::RequestClose`/`apply_destroy` が queue
  した `Destroyed` 等の `DriverEvent` は次の `flush_events` で
  delivery されるため、**confirm job は `flush_events` を呼んで
  から reply する** — reply 到着時に「op 適用 + 派生 event 配送 +
  journal/notice 記録」が全て済んでいることが保証される。
- `respond_close` は bookkeeping を `borrow_mut` 内で即時適用
  する（`awaiting_response`/`pending`/`approved` の mutation）ので、
  `Op::RequestClose` arm の post-flush default-approve 判定は
  veto/defer を正しく見る — `emit(CloseRequested)` →
  `driver.request_close` + confirm で FakeDriver::pump と同じ
  semantics になる。
- `emit`（Tick/Focused/Action）は `inner.events.push` +
  `flush_events` で **job 内で同期配送** — real event loop が
  行う delivery と同一経路（interceptor 経由で app へ）。routed
  reply（app→`post_event`）は Op queue に残るので、`emit` も
  confirm job で次 drain の適用 + flush を待つ。

### `emit` の per-event 正直 mapping（windowed）

`emit` の契約は「host 側 event を注入し、real event loop と同じ
delivery を受けさせる」。windowed で **driver-state bookkeeping を
持たない event** は `inner.events` への注入が誠実に成立する:

| event | mapping | 理由 |
| --- | --- | --- |
| `Tick` | `inner.events.push` + flush | 合成 wakeup は余分な drain を起こすだけで state 偽装なし |
| `Windowed{Action}` | 同上 | `IntentSink` と同じ typed ingress。dead window は silent drop（FakeDriver parity） |
| `Windowed{Focused}` | 同上 | slot が focus 要求以外を bookkeep しない。app 側 state は app の責任 |
| `Windowed{CloseRequested}` | `driver.request_close(w)`（real op）+ settle | slot.close.pending/awaiting bookkeeping を経ない注入は veto/defer/default-approve を壊す。real op なら全 semantic が成立 |
| `Created`/`Ready`/`Destroyed` | drop + `emit.unsupported` notice | lifecycle bookkeeping が slot にある（`slot.ready` gate・retire）。注入すると app と driver state が乖離する |
| `Resized`/`Moved`/`ScaleFactorChanged` | 同上 | `last_size`/`last_pos`/scale bookkeeping と units（physical vs logical）が食い違う |

「drop + notice」は silent ではない — journal には何も書かず
（boundary を越えていない）notice で拒否を観測可能にする。
FakeDriver 側は legacy 同様全 event を inject 受理する（fake の
model として正当）— backend 差分は capability ではなく inject
可否の範囲であり、op 経路（Tick/CloseRequested/Action）では
両者一致する。

### `windows()` は実 slot state を読む（shadow なし）

`ready`/`close_pending` を event 駆動 shadow で持つと、veto で
`close.pending` が戻る遷移（event が出ない）を追えない。
`windows()` job が `inner.windows` の `slot.ready`/
`slot.close.pending` を直接読み、`specs` map（create 時記録の
title/logical size）と `ipc_down` set を合成する。app 起源の
window（`JournalHandle::create_window`）は spec map に無いので
`title: None`・`w/h: 0` の「untracked」を正直に出す
（FakeDriver の spec-less window と同じ fallback）。

### `request_shutdown` — app 到達は interceptor 経由

`UiJob` は `Send` closure で、`UiRuntime`（非 `Send`）を capture
できない。graceful cleanup で必要な `app.shutdown(mode, jh)` は
app を所有する `UiSide::deliver` 内でしか呼べない。そこで
shutdown を **pending request + Tick wakeup** で実現する:

1. `request_shutdown(mode)`: shared `pending_shutdown` を arm →
   job1「Tick 注入 + flush」を送る → confirm 用 `call`（job2）を
   enqueue → report を oneshot で待つ。
2. `UiSide::deliver` は配送後に `pending_shutdown` を確認: arm 済
   なら `app.shutdown(mode, jh)`（Graceful）→ 全 live window
   `drv.destroy_window` → `drv.exit()`（全て Op queue へ）。
3. job2 は次 drain の `drain_ui_jobs`（ops 適用後）で
   `inner.windows.len()`・`driver.exited()` を読み
   `CleanupReport` を reply。`Op::Exit` → `cx.quit()` は同 drain
   の job 実行を妨げない。

`Now` は shutdown hook と teardown を skip して `exit` のみ。
`app.closing` notice は request 受理時に記録。

### fault / fixture / screenshot

- `ipc_disconnect`: control thread が shared `ipc_down` set を toggle
  （interceptor + `emit_ipc_body` job が参照）。window_crash:
  `destroy_window` + settle（terminal `Destroyed` が real retire
  path から流れる）。
- fixture: `FixtureController` は control thread で既存のまま
  `io.create_window`/`io.emit` を呼ぶ — 内部で job 化されるので
  controller 自体は backend 非依存。AUT 側 fixture は real Ready
  を待つ（bounded poll `io.windows()`）— headless の `emit(Ready)`
  注入は windowed では unsupported に落ちるため。
- screenshot: job で `driver.capture_window(w)` → `rgba-raw` +
  base64 → `proto::Screenshot`。claim は `cfg!(windows) ||
  cfg!(target_os = "linux")`（capture backend 存在 OS のみ）。
  `UiError` → `ErrorBody`（Host/Unsupported → `unavailable`/`io`
  相当）で正直に返す。
- **frame cap 制約（実走で判明・追記）**: reply は
  `MAX_INTENT_BYTES`（1 MiB）frame cap 配下でなければ
  `write_frame` が失敗し serve が `DriverGone` で session を
  閉じる — 800×600 級の RGBA → base64 reply は cap を超える。
  adapter は encode 後に cap を pre-flight し、超過時は
  `service_error` で **op だけ** 失敗させ（channel は生かす）、
  lane は小サイズ window（fixture `width`/`height` params 追加）
  を capture 対象にする。pixel 自体は常に実 capture で正直。

## 設計判断 6: feature gate — `uitest` feature を新設

`UiJob` seam は現在 `#[cfg(feature = "real-scenario")]`。選択肢:

| 案 | 内容 | 判定 |
| --- | --- | --- |
| A. `real-scenario` を共有 | AUT を `real-scenario` で build | **不採用** — scenario adapter（SemanticGuiTest）を不要に引き込み、2 つの test surface を feature で conflate する |
| B. `uitest` feature 新設 + seam を `any(real-scenario, uitest)` に拡張 | `uitest = ["gpui", "dep:ruviform-ui-test", "dep:base64"]` | **採用** |

seam 自体は 1 本の channel で、同一 process で 2 consumer が
競合することはない（AUT は uitest のみ、scenario adapter は
real-scenario のみで build）。`gpui`-only 本番 build には
どちらも入らない。

- `[[bin]] windowed_test_app` `required-features = ["uitest"]`。
- `pub mod windowed_io` を `#[cfg(feature = "uitest")]` で
  `mod gpui_driver` 内に宣言（`Inner`/`UiJob`/`flush_events`/
  `apply_destroy` 等の module-private item へ子 module から到達
  するため `gpui_driver` の子に置く）。
- `base64` は `Screenshot.data_base64` の wire 契約に必要
  （`rgba-raw` encode）。`ruviform-ui-test` 経由で lock に既出。

## 設計判断 7: AUT binary

`bin/windowed_test_app.rs`（`dummy_app.rs` の windowed 版）:

- 同じ `TestApp` 構成（`UiRuntime` dispatch・close policy・job
  cancel）と同じ fixture 名（`windows`/`ipc_emit`/`close_policy`）。
  `--fail-init`/`--windows N` flag も同一。
- **thread 配置**: main thread で `serve_from_env(&mut io)`、
  app thread で `driver.run(tick, deliver)`。`run` を
  `catch_unwind` で包み、panic/Err → `eprintln` +
  `process::exit(3)` — UI loop の死は AUT の死（fail-closed。
  main が stdin で hang したまま driver が timeout するのを防ぐ）。
- stdout は control channel 専用 — log/diagnostics は stderr。
- `windows` fixture は create 後に real `Ready` を bounded poll
  （`io.windows()` の `ready` flag、timeout → `unavailable`）。
- `ipc_emit` fixture は `io.emit_ipc_body(window, &body)` —
  malformed を含む任意 body（dummy_app parity）。

## 検証二層

- **headless（CI/headless 可）**: `TestIo` neutral 化後の
  dispatch/journal/metadata/fault/e2e を FakeDriver 経路で検証
  （`headless_contract`/`e2e_process`/`lifecycle_fixture`/
  `driver_contract` 移行 + 新規 neutral dispatch test）。
- **windowed（対話 desktop manual lane）**:
  `tests/windowed_uitest.rs` が `CARGO_BIN_EXE_windowed_test_app`
  を `TestDriver::spawn_stdio` で spawn し、hello → windows →
  fixture → close → screenshot → shutdown を実 window で実走。
  `RVF_WINDOWED_UITEST=1` opt-in gate（desktop session 必須のため
  CI/headless では skip）。verdict を
  `evidence/issue-313-windowed-uitest/` に記録。

## 既存 S1 境界との非干渉

- `RealOsGuiTest`/`os_input.rs`/`test-ui-real-os-scenario` には
  触れない。`real-scenario` feature の振る舞いは UiJob gate の
  `any(...)` 拡張で変わらない（cfg は widen でも機能は不変）。
- `TestIo` に OS input/DOM の op は追加しない —
  `DeniedCapabilities` の `input_injection`/`dom_access` は
  false のまま。
