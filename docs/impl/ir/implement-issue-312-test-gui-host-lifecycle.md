---
id: "IMPL-034S312gtUiTestImplDD44"
type: "impl"
status: "completed"
title: "Implement Issue #312 test-only GUI host lifecycle・secure control adapter・fixture/clock/observation"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/312"
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-ui-test"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "../../tasks/task-312-t1-contract-headless-core.md"
    - "../../tasks/task-312-t2-control-plane.md"
    - "../../tasks/task-312-t3-tests-docs.md"
  implemented-by: []
  depends-on:
    - "../../plans/issue-312-test-gui-host-lifecycle.md"
    - "../../designs/issue-312-test-gui-host-lifecycle.md"
    - "../../adr/issue-312-test-control-transport-security.md"
    - "../../specs/issue-312-test-gui-host-lifecycle.md"
    - "../../discovery/issue-312-test-gui-host-lifecycle.md"
  blocks:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/issue-312-test-gui-host-lifecycle.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement Issue #312 test-only GUI host lifecycle・secure control adapter・fixture/clock/observation

## Summary

task `TASK-034S312gtUiTestT1GG77`〜`TASK-034S312gtUiTestT3II99`（plan `PLAN-034S312gtUiTestPlnEE55`）として、設計正本 `DESIGN-034S312gtUiTestDsnDD44`・ADR `ADR-034S312gtUiTestAdrCC33` どおりにテスト専用制御基盤 `crates/ruviform-ui-test` を実装した。`ruviform-ui` の `WindowHandle`/`WindowEvent` 境界を消費側層から利用する形で、`rvf.uitest.v1` 制御ワイヤ・stdio/loopback-TCP トランスポート・`TestAgent`・`TestIo`/`HeadlessIo`/`FakeHost`・fixture・有界観測 journal・`TestDriver`・受入用 `dummy_app` を構成する。**本番クレートへの変更はゼロ**（diff 面は root `Cargo.toml` の members 追記・`Cargo.lock`・`crates/README.md` のみ）で、production release 経路では env opt-in なしに制御ポートが serve しない fail-closed 構造とした。

## Context

- 実施タスク: [task-312-t1](../../tasks/task-312-t1-contract-headless-core.md)（contract + headless core）、[task-312-t2](../../tasks/task-312-t2-control-plane.md)（transport + agent + driver + dummy_app）、[task-312-t3](../../tasks/task-312-t3-tests-docs.md)（テスト + 文書 + 登録）
- 上流: [plan](../../plans/issue-312-test-gui-host-lifecycle.md)、[design](../../designs/issue-312-test-gui-host-lifecycle.md)、[ADR](../../adr/issue-312-test-control-transport-security.md)、[spec](../../specs/issue-312-test-gui-host-lifecycle.md)、[test-spec](../../test-specs/issue-312-test-gui-host-lifecycle.md)、[discovery](../../discovery/issue-312-test-gui-host-lifecycle.md)
- 前提: `ruviform-ui`（Issue #156 の Host 抽象 + #181 の multi-window `WindowEvent`/`WindowHandle` 契約 + `rvf.ipc.v1` envelope）
- 環境: Windows 11 x64・rust 1.99.0・GUI 実機（wry/tao/WebView2）は本タスク対象外

## Implementation

### Changes

- `Cargo.toml`（root）— `members` に `crates/ruviform-ui-test` を利用側層として追記（依存方向コメント付き）。`Cargo.lock` は依存解決による自動更新
- `crates/ruviform-ui-test/Cargo.toml` — `ruviform-ui` path 依存・serde/serde_json・`os_pipe`（serve ループの pipe テスト用）。`publish.workspace = true`・`[lints] workspace = true`。feature なし・wry/tao/GUI/RT 依存を持ち込まない
- `src/proto.rs` — `rvf.uitest.v1` 契約型。閉じた `ops::test.*` 語彙（hello/capabilities/fixture.prepare/clock.advance/fault.set/fault.clear/window.request_close/observe.windows/observe.receipts/observe.screenshot/notices/shutdown）、`test_codes`（`unauthenticated`/`disabled`）、`AppMeta`/`BackendMeta`/`Capabilities`（`DeniedCapabilities` flatten で `input_injection`/`dom_access` を常時 `false` 宣言）/`Welcome`/`ReadyState`/`ClockDomain`（`HostTick` のみ）/`FaultKind`（`WindowCrash`・`IpcDisconnect` の allowlist）/`FixtureSpec`/`FixtureReceipt`/`Direction`/`Receipt`/`ReceiptPage`/`Notice`/`Screenshot`/`ShutdownMode`/`CleanupReport`/`ServeOutcome`/`WindowMeta`
- `src/journal.rs` — 有界 ring の観測 journal。両方向 receipt を `seq` 連番で記録（payload は byte 長のみ、bulk 本文は複写しない）。`entries_since(since, limit)` は `seq > since` + `truncated`（`since + 1 < floor` で実際に evict された場合のみ true — 未溢れ journal への `since=0` は truncated にしない）。`JournalHandle` で emit 側・`deliver_routed` で `UiRuntime` routed 配送をラップ
- `src/fake.rs` — `FakeHost`/`FakeWindow`:`WindowHandle` の headless 実装。`ruviform-ui-webview::session` と同意味論（未 `Ready` eval は有界 drop-oldest backlog・破棄済み宛ては silent drop・`CloseRequested` は veto 可能で未応答は対象 window のみ既定 approve・`Destroyed` 終端・`exit` は app 所有）
- `src/fixture.rs` — `TestApp`（`on_event`）/`FixtureController`（`name`+`prepare`）trait。fixture は `TestIo` 経由で window 生成・host 側 event 送出・fault arm のみ可能 — OS 操作や app 状態の裏口変更は提供しない
- `src/io.rs` — `TestIo` ポート: ready_state/capabilities/backend/windows/emit/notices/receipts/prepare_fixture/set_fault/screenshot/request_shutdown/create_window/host_mut。agent と AUT の橋
- `src/headless.rs` — `HeadlessIo`:`FakeHost` 上の `TestIo` 実装。`pump_events` で inbound receipt 記録・lifecycle notice 生成・`JournalHandle` 経由の app 配送・`ipc_disconnect` 抑制の observable 記録。`request_shutdown` は graceful cleanup/`Now` 即時終了を型付き `CleanupReport` で返す。`set_ready_state` で fail-init handshake を再現
- `src/transport.rs` — u32le length + JSON 有界フレーム（`MAX_INTENT_BYTES` 超過は宣言長分を drain して `TooLarge` を返しストリーム同期を維持）。env 選択（`RVF_TEST_CONTROL`/`RVF_TEST_ADDR`/`RVF_TEST_SECRET`/`RVF_TEST_DISABLE`）は fail-closed（未設定・`off`・不正値・tcp で addr/secret 欠落は `Off`/拒否）。`is_loopback`・run-bound `gen_token`
- `src/agent.rs` — `TestAgent`: hello-first 強制・ready/failed handshake・session/run id・capability 強制（`unavailable`）・未認証/不正 op/disabled の型付きエラー・各 op 後の `io.notices()` を `Event::Notify` へ eager drain・`shutdown` で `ended` マーク。`serve` ループは shutdown 応答・clean EOF・書込み失敗で `DriverGone`/`Shutdown`/`Failed` を返し、driver 消失は graceful cleanup を必ず実行。`serve_stdio`/`serve_tcp`（loopback peer・単一接続・secret 必須）/`serve_from_env`（env 解決 + disabled 優先）
- `src/driver.rs` — `TestDriver`: `spawn_stdio`/`spawn_tcp`（ephemeral listener・accept は loopback peer のみ・deadline 付き・**accepted socket は `set_nonblocking(false)` で blocking 復帰** — Windows で listener の nonblocking フラグを継承するため）・handshake・typed op（capabilities/prepare_fixture/advance_clock/set_fault/clear_fault/request_close/observe_windows/observe_receipts/screenshot/notices/shutdown）・`notifications()` で buffered `Notify` 排出・`DriverError`（Io/Protocol/Handshake/Disconnected/Timeout）・`Drop` で子プロセス reap
- `src/bin/dummy_app.rs` — 受入用 dummy consumer。通常の `UiRuntime`/`WindowEvent` 配線（`Windowed` は `runtime.handle_window` 経由・`Tick` で routed job drain）で、fixture `windows`（複数 window + close_policy unanswered/approve/veto/defer）・`ipc_emit`・`--fail-init`・stdio/TCP/disabled モードを提供。stdout は制御 channel 専用・診断は stderr・`DummyState` は `Arc<Mutex>`（serve が thread を跨るため）
- `crates/ruviform-ui-test/README.md` + `crates/README.md` 利用側層テーブルへ登録
- `tests/headless_contract.rs` — in-process 契約 32 本（handshake/capability gate/fixture/clock scope/fault allowlist/観測 read-only/journal pagination+truncation/close 各 policy/shutdown graceful+now/malformed/unknown op/disabled/oversized frame 継続/serve loop over pipes）
- `tests/e2e_process.rs` — cross-process 15 本（stdio lifecycle 受入・2 window+close cancel・crash で他 window 存続・ipc_disconnect 抑制観測・driver EOF cleanup・異常終了 reap・restart 新 session・fail-init handshake・no-env で制御不在・disable で serve 拒否・未認証 op 拒否・TCP 認証 lifecycle・wrong secret・secret 欠落で serve 拒否・`ShutdownMode::Now` は teardown skip）

### Decisions During Implementation

- **本番 API 非変更で設計を満たす** — `TestIo` を「app 内制御橋」として定義し、agent は既存 `WindowEvent` emit/`WindowHandle` 呼出しだけで動く。結果として `ruviform-ui`/`ruviform-ui-webview` のソース変更が完全にゼロになり、release 未到達性は「誰も依存しない」依存方向で構造保証される
- **notice は eager `Notify` drain** — `test.notices` op だけに頼ると op 間の非同期イベントが取りこぼされるため、`handle` は各コマンド処理後に `io.notices()` を `Notify` event へ drain する。`test.notices` op は残量 drain（明示 flush 経路）として維持
- **journal seq は 1 起点** — `entries_since` は `seq > since` なので `since=0` を新規 cursor として全 receipt が観測できるよう `next_seq` は 1 開始。`truncated` は `since + 1 < floor`（実際に evict された receipt が cursor 範囲に存在する場合）に修正 — 初期実装の `since < floor` は未溢れ journal で誤って truncated を立てた
- **oversized frame は drain** — `TooLarge` 即 return では宣言長分の body が残って後続 frame を desync させるため、`take(declared)` で本体を捨て切ってから error event を返す（serve 継続を headless 契約テストで検証）
- **accepted socket は blocking 復帰** — Windows で `accept()` した socket が listener の nonblocking を継承し `WouldBlock`（10035）で handshake 失敗した。`set_nonblocking(false)` を accept 直後に入れて修正（driver・テスト双方）
- **`DeniedCapabilities` flatten** — `input_injection`/`dom_access` はトグルではなく port 構造上の恒久的拒否。`#[serde(flatten)]` で wire の `input_injection:false`/`dom_access:false` を保ちつつ clippy `struct_excessive_bools` を回避し「never granted」の意味を型で明示
- **`FixtureController: Send`・`DummyState: Arc<Mutex>`** — serve を別スレッドで駆動する契約テスト経路（pipe 越し）のため。trait 境界で実要因に対応

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `mise run fmt-check` — Passed（`cargo fmt --all -- --check`、post-merge）
- `mise run check` — Passed（`cargo check --workspace --all-targets`、post-merge）
- `mise run lint` — Passed（`cargo clippy --workspace --all-targets -- -D warnings`、post-merge。`#[allow]` 不使用）
- `mise run test` — Passed（`cargo test --workspace`、post-merge、120 test binary 全て `0 failed`）
- `cargo test -p ruviform-ui-test` — headless contract 32 + e2e_process 15、全 47 本 Passed
- 途中観測した `ruviform-plugin-host` の `render_malformed_output_converges_instance` 1 件の失敗は並行実行 flake で、単独再実行・最終 workspace run ともに Passed（本変更とは無関係 — 依存なし）

### 未検証/引き渡し（正直報告）

- **実 window/実 OS 経路は NotRun** — 本実装の検証は headless `FakeHost` + dummy_app の cross-process 経路のみ。`wry`/`tao`/WebView2 実窓への `TestIo` 結線、実 OS での screenshot/実時計/GPU/vsync 制御は**未検証であり本スコープ外**。RF-GT-04（Issue #313・OS/Browser driver）と #243 へ引き渡す
- **screenshot は Unsupported 宣言** — headless backend は `screenshot:false` で `unavailable` を返し合成画像は生成しない。`observe.screenshot` の capability 契約のみ定義済み
- **`ipc_disconnect` のモデル** — frontend→host の inbound `Message` 抑制を意味し、双方向 IPC 断や webview プロセス死亡の全形を完全に再現するものではない
- **`ShutdownMode::Now` の「即時」性** — headless では「teardown/cleanup を skip して exited 遷移」の契約として検証済み。実 event loop での強制中断粒度は backend 次第（#313 で検証対象）

## Consumer 影響分類材料

- **依存方向**: `ruviform-ui-test` → `ruviform-ui`（公開 API のみ）。`ruviform-ui`・`ruviform-ui-webview`・RT クレート・CLI/consumer いずれも本クレートに依存しない
- **本番 API 変更**: なし（`ruviform-ui`/`ruviform-ui-webview` のソース diff ゼロ）
- **feature/optional 依存**: 新規クレートは feature なし。consumer 向け新規必須依存なし（`os_pipe` は本クレート内 dev/serve テスト用途）
- **RT 影響**: なし（全 API 非 RT・RT クレートから参照されない）
- **Test Host を使わない CLI/DSP/RT・WebView-only consumer**: ビルド/実行への影響なし（members 追加のみ）
