---
id: "IMPL-034VFzhzdEyknToi5VSfS5"
type: "impl"
status: "completed"
title: "Issue #313 S2 windowed `TestIo`/`rvf.uitest.v1` GPUI backend 配線"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "docs/specs/issue-313-windowed-testio.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-313-windowed-testio.md"
      - type: "file"
        path: "docs/designs/issue-313-windowed-testio-design.md"
      - type: "file"
        path: "docs/plans/issue-313-windowed-testio-plan.md"
      - type: "file"
        path: "docs/test-specs/issue-313-windowed-testio.md"
      - type: "file"
        path: "docs/tasks/issue-313-task2-windowed-testio.md"
      - type: "file"
        path: "docs/impl/ir/issue-313-windowed-testio-slice.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/windowed_io.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/bin/windowed_test_app.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/windowed_uitest.rs"
      - type: "file"
        path: "evidence/issue-313-windowed-uitest/verdict-windowed-uitest.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/io.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/headless.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/fake_driver.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/fixture.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/journal.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/agent.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/bin/dummy_app.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/headless_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/boundary.json"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-313-task2-windowed-testio.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-313-windowed-testio-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-313-windowed-testio.md"
    - "docs/designs/issue-313-windowed-testio-design.md"
    - "docs/test-specs/issue-313-windowed-testio.md"
    - "docs/impl/ir/issue-313-real-os-input-driver-slice.md"
    - "docs/impl/ir/implement-issue-312-test-gui-host-lifecycle.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-313-windowed-testio.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #313 S2 windowed `TestIo`/`rvf.uitest.v1` GPUI backend 配線

## Summary

`rvf.uitest.v1` の closed op set を実 GPUI window に配線した。
前半は `TestIo`/`TestApp`/`Journal`/`JournalHandle`/`HeadlessIo`/
`dummy_app` を legacy host 語彙（`WindowEvent`/`WindowHandle`/
`WindowConfig`/`FakeHost`）から neutral driver 語彙（`DriverEvent`/
`WindowDriver`/`WindowSpec`/`FakeDriver`）へ再表現する移行 —
`HeadlessIo` は `FakeDriver` 上に再基盤化され、既存 headless
suite 群は全て移行後も pass。後半は
`crates/ruviform-ui-gpui` に `WindowedIo`（control thread ↔ UI
thread を `UiJob` seam で橋渡しする `TestIo` 実装）と AUT
`windowed_test_app` を新設し、`RVF_WINDOWED_UITEST=1` gate の
manual lane で全 op を実走検証した。

## Context

- Issue #313 S2（RF-GT-04）。前提節で固定された採用設計:
  backend = GPUI 一本、語彙 = neutral 再表現（Option A、
  legacy shim 不採用）、cross-thread = 既存 `UiJob` seam 再利用、
  AUT = `ruviform-ui-gpui` 側 `[[bin]]`、SendInput 不使用、
  検証二層（headless 契約 + 対話 desktop manual lane）。
- 文書チェーン: spec → design → plan → test-spec → task → 本 IR。
- `GpuiDriver` は `Rc<RefCell<Inner>>` で非 `Send` — agent serve
  loop（control thread）は driver に触れられないため、全 op は
  `UiJob` + oneshot reply で app thread へ跨ぐ。

## Implementation

### Changes

- `crates/ruviform-ui-test/src/io.rs` — `TestIo` を neutral 語彙へ:
  `emit(DriverEvent)`/`create_window(&WindowSpec)`/`screenshot` の
  シグネチャを `DriverEvent`/`WindowSpec`/`WindowId` ベースに統一。
- `crates/ruviform-ui-test/src/headless.rs` — `HeadlessIo` を
  `FakeDriver` 上に再基盤化（FakeHost 内蔵を撤去）。
  `pump_events` interceptor（journal `In` receipt → lifecycle
  notice → `ipc_disconnect` suppression → app 配送）は windowed
  版の parity 正本として維持。
- `crates/ruviform-ui-test/src/fake_driver.rs` — `ready` accessor
  追加（`WindowMeta.ready` の観測源）。
- `crates/ruviform-ui-test/src/{fixture,journal,agent}.rs` —
  `TestApp::on_event`/`JournalHandle`/`WindowDriver` 参照を
  neutral 語彙へ追従。
- `crates/ruviform-ui-test/src/bin/dummy_app.rs` — headless AUT を
  neutral 経路へ移行（`handle_window`/`HostEvent` 不使用）。
- `crates/ruviform-ui-test/tests/{headless_contract,driver_contract}.rs`
  — 移行 + 新規 `malformed_ipc_body_answers_with_error_event_observably`:
  malformed body が `In "message"` + `Out "post_event"` receipt と
  window inbox への `Event::Error` を観測可能に残し、app には
  malformed `Action` が届かないこと・dead window への body は
  receipt なしに silent drop されることを固定。
- `crates/ruviform-ui-gpui/src/gpui_driver/windowed_io.rs` — 新規
  `WindowedIo`/`UiSide`。control/UI thread 分離、action-job +
  confirm-job の決定論的順序付け（action が `Op` を queue →
  reply は job 実行中にのみ送られるため confirm job は次 drain
  の op phase 後に実行 → `flush_events` で derived event 配送・
  journal・notice が caller 復帰前に完了）、`emit` は
  `Tick`/`Action`/`Focused`/`CloseRequested`（→ 実
  `request_close` op）のみ honest 注入、他 lifecycle は
  `emit.unsupported` notice で observably drop。`ipc_disconnect`
  は per-window 限定（`window=None` は `invalid_params`）、
  `window_crash` は実 `destroy_window`、screenshot は
  `capture_window` + base64 + **`MAX_INTENT_BYTES` pre-flight**
  （後述）。unknown future `FaultKind`/`DriverEvent` は
  `invalid_params`/`emit.unsupported` で fail-closed。
- `crates/ruviform-ui-gpui/src/bin/windowed_test_app.rs` — 新規
  AUT。control thread（main）で `serve_from_env`、app thread で
  `UiSide::run`（`tick: None` → driver 既定の定期 Tick）。UI loop
  の死は `process::exit(3)` で AUT 全体の死（fail-closed —
  control channel が backend を生き延びない）。`windows` fixture
  は `count`/`ready`/`width`/`height`/`close_policy` params を持ち
  real `Ready` を bounded poll（fake 注入しない）。`ipc_emit`
  fixture で任意 raw body を `emit_ipc_body` へ。
- `crates/ruviform-ui-gpui/tests/windowed_uitest.rs` — 新規 lane:
  `TestDriver::spawn_stdio` で実 AUT を spawn し
  hello→windows→fixture→receipts→clock→ipc→fault→screenshot→
  request_close→shutdown を実走、per-op verdict を
  `evidence/issue-313-windowed-uitest/verdict-windowed-uitest.json`
  へ記録。`RVF_WINDOWED_UITEST=1` gate なしでは no-op（skip は
  verdict を書かない = 未実走を pass と偽装しない）。
  `--fail-init` handshake 失敗 path も検証。
- `crates/ruviform-ui-gpui/Cargo.toml` — `uitest = ["gpui",
  "dep:ruviform-ui-test", "dep:base64"]` feature +
  `[[bin]] windowed_test_app`（`required-features = ["uitest"]`）。
- `crates/ruviform-ui-gpui/src/lib.rs` — `UiJob`/`set_ui_jobs`/
  `drain_ui_jobs`/`flush_events`/`Inner` 等の seam を
  `#[cfg(feature = "real-scenario")]` から
  `any(feature = "real-scenario", feature = "uitest")` へ widen
  （`real-scenario` 側の振る舞いは不変）。
- `crates/ruviform-ui-test/fixtures/driver-lab/boundary.json` —
  `ruviform-ui-gpui` の `gpui`/`raw-window-handle` 許可 viaFeatures
  に `"uitest"` を追加（coordinator 承認済みの範囲拡張 —
  `live_workspace_passes_boundary` の ruleset）。
- `mise.toml` — `[tasks.test-ui-windowed-uitest]` 新規（manual
  desktop lane、`--test-threads=1`）。
- `Cargo.lock` — `ruviform-ui-test` を `ruviform-ui-gpui` の dep
  として再解決（`base64` は lock 既出）。

### Decisions During Implementation

- **`UiSide` は app を保持しない**: `TestApp` は `Send` 保証が
  ないため、`thread::spawn` に渡せる `Send` state（jobs
  receiver + shared）だけを `UiSide` に載せ、app は `run` 内で
  app thread 上に attach。shutdown hook も同じ理由で
  `pending_shutdown` を interceptor（delivery path 内、app
  thread）が consume する設計。
- **confirm-job は `flush_events` 自前で呼ぶ**: `respond_close`/
  `apply_destroy` は bookkeeping を同期適用するが derived
  `DriverEvent` は次の `flush_events` でしか interceptor に
  届かない。job+oneshot reply は job 実行中にのみ送られるため、
  action 後の confirm job は必ず「その op を apply した drain
  の次」に着地し、そこで flush すれば caller 復帰時に
  receipt/notice/destroyed が全て確定している。
- **`ipc_disconnect` は per-window のみ**: 現行契約は
  `window=None`（global arm）を `invalid_params` とする —
  windowed でも同じ。suppress は decode 前の `emit_ipc_body`
  入口と delivery interceptor の `Action` の両方で行い、
  `suppressed` receipt + `ipc.suppressed` notice を残す。
- **screenshot は `MAX_INTENT_BYTES` pre-flight（実走で判明）**:
  stdio transport の frame cap は 1 MiB。800×600 級の RGBA→base64
  reply は cap を超え、`write_frame` 失敗 → serve が
  `DriverGone` で session 終了 → AUT が死に後続 op 全て
  `Disconnected`/`BrokenPipe` になる（初回実走で観測）。adapter
  は encode 後に cap を pre-flight し超過時は `service_error`
  で **op のみ** 失敗させ channel を生かす。lane は fixture の
  `width`/`height` で小サイズ window（320×240）を capture 対象に
  する — pixel は常に実 capture で、量だけを transport に合わせる。
- **notice 観測は `Notify` event 経由**: agent は毎コマンド後に
  `io.notices()` を drain して reply に `Notify` として同梱する
  ため、`test.notices` op は既 drain で `[]` を返す。lane の
  notice 検証は `TestDriver::notifications()`（buffered `Notify`
  payload）で行う（`e2e_process.rs` の `pending_notices` と同型）。
  初回実走で `d.notices()` が常に空を返し `notice: false` と
  誤判定したことから修正。

## Related Experiments

### Adopted

なし — Experiment Log は作成していない（採用設計は spec/design
で事前に固定済みの実装タスク）。

### Rejected

なし。

## Validation

### 実測（Windows 11 x64、対話 desktop、GPUI real window）

- `RVF_WINDOWED_UITEST=1 mise run test-ui-windowed-uitest` →
  `windowed_uitest_lane` + `windowed_fail_init_reports_failed_handshake`
  共に PASS。verdict `pass`（全 11 op）を
  `evidence/issue-313-windowed-uitest/verdict-windowed-uitest.json`
  に記録: hello（backend `gpui-windowed`・screenshot claim・
  input_injection/dom_access denied）、windows（real `Ready`）、
  fixture.prepare、receipts（`created`/`ready` `In`）、
  clock.advance（`tick` receipt・未宣言 domain は `unavailable`）、
  ipc_emit（`message` receipt）、fault.set（`ipc_down`+`suppressed`
  receipt+`ipc.suppressed` notice）、fault.window_crash
  （対象 window のみ `Destroyed`+`window.destroyed` notice）、
  screenshot（320×240 `rgba-raw`、307200 bytes 整合）、
  request_close（veto 維持 → 未応答 default-approve で destroy）、
  shutdown（`windows_open:2`/`windows_closed:2`/`exited`、AUT は
  `ServeOutcome::Shutdown` で code 0 終了）。
- headless（FakeDriver 経路）回帰なし: `cargo test -p
  ruviform-ui-test` 204 tests pass（補助証拠 — canonical は下記
  mise lane）を含み、malformed IPC・suppression・metadata・
  crash notice・close veto/approve/defer・pagination・shutdown
  を網羅。
- canonical lanes: `mise run fmt-check` / `check` / `lint` /
  `test-rt` / `check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` /
  `test-ui-scenario-contract` / `check-ui-driver-boundary` 全 pass。
- 補助証拠（非 canonical）: `cargo clippy -p ruviform-ui-gpui
  --features uitest --all-targets -- -D warnings` pass —
  canonical `lint` は `uitest` feature を含まないため併走。

### 観測した既存 flake（本 slice 範囲外・main 由来）

- `mise run test` で `ruviform-plugin-host` の
  `hang_converges_on_command_timeout`（`SpawnFailed{Timeout}`）が
  並列 build/test 負荷下で 2 連続 fail → 単独では 3/3 pass。
  本 slice の diff は `crates/ruviform-plugin-host` に無接触
  （`git diff main...HEAD --stat` 空）で、並列負荷下の既存
  spawn-timeout flake と判定。再現は load 依存・結果は flaky、
  原因は scope 外のため修正せず記録のみ（報告事項）。

## Risks

- `MAX_INTENT_BYTES`（1 MiB）を超える大 window の screenshot は
  `service_error` で op 失敗（channel は生存）。wire 側の
  chunked reply は closed op set 不変の制約で本 slice 未着手 —
  必要なら後続で proto 拡張の要否を検討。
- `ruviform-plugin-host` の並列負荷 flake は main 由来・scope 外。
  CI の workspace `test` で再発しうる（既存の不安定さとして報告）。

## Follow-ups

- macOS/Linux 実機行: `screenshot` claim は `cfg!(linux)` も含むが
  Linux 実機未検証 — lane verdict は `unsupported`/`environment_blocked`
  で正直に記録する設計。
- S2 対象外（前提節 8）のまま: macOS/Linux 実機・動的 DPI・
  IME/a11y・pointer 右/中ボタン・wheel・2-window real-OS・
  `ScenarioDriver::RealOs` adapter 本体（#464 系）・Linux
  Xvfb+openbox+lavapipe CI 化。
