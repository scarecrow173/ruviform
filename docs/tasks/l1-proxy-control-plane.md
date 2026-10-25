---
id: "TASK-034LNYf8Qj1NeGAKmaP7B4"
type: "task"
status: "done"
title: "plugin-host L1 プロキシ制御面（spawn/handshake/コマンド転送/クラッシュ検出）"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/shm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
  implemented-by:
    - "docs/impl/ir/issue-76-l1-proxy-control-plane-task-3-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/tasks/l1-core-contracts-and-wire-codec.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# plugin-host L1 プロキシ制御面（spawn/handshake/コマンド転送/クラッシュ検出）

## Work

approved plan `implement-issue-76-plugin-l1-out-of-process.md` の **actions 9–11 を 1 task の ordered substeps として実行**。plugin-host に `pub(crate)` の L1 インスタンス機構を実装する — spawn・handshake・driver/reader 2 スレッド・ローカル lifecycle ミラー・`Failed` 冪等収束・通知キュー — 制御面（コマンド往復・クラッシュ検出・`Failed` 契約）に限定し、shm データ面/RT 脚は次 task。

**先行確認**: `shared_memory` の実効 API（`ShmemConf`/`os_id`/`as_mut_slice`・Drop 時 unlink 規則）を docs.rs/`cargo doc` で確認（task 2 と同じ先行確認 — 相違なら blocker）。`std::process::Child` の `try_wait`/`kill`/`wait` と stdin/stdout パイプの形状を確認。

### A9 — deps + `l1` 骨格

- `crates/ruviform-plugin-host/Cargo.toml`: `ruviform-rt = { path = "../ruviform-rt" }`（Primitives 層・A.2 許可範囲）+ `shared_memory = "0.12"`（design「依存追加の判断」どおり — MIT OR Apache-2.0・安全 slice API・依存木小・Windows+POSIX named object。停滞リスクは internal `shm` モジュール隔離で封じ込め）+ `[[bin]] name = "l1-test-worker"`
- `src/l1/mod.rs`（`pub(crate)`）+ `src/l1/shm.rs`（`pub(crate)`）新設 — `shared_memory` をここに閉じ込め（公開 API・feature 名に露出しない・自前 `unsafe` ゼロ）・一意 key 生成（pid + slot + counter）・segment 作成・header 書込/検証

### A10 — L1 インスタンス機構（spawn/handshake・driver/reader・`Failed` 収束）

- spawn: `Command`（`OutOfProcessSpec.program`/`args` + `--ruviform-plugin-id <id>`/`--ruviform-shm-key <key>`）→ `Hello` 受信 → `Scan` → `Scanned` の `wire::Descriptor` を `OutOfProcessSpec.descriptor` とフィールド等価照合 → `Instantiated{handle}`。いずれかの失敗・version 不一致・`command_timeout` → `Err(HostError::Ipc)` + child kill/reap
- `driver` スレッド: `Shared.child`（`Mutex<Child>` — driver が操作するが `Arc<Shared>` 経由で共有。wedged 状態で `write_all` ブロック中でも収束側が直接 `kill()` できるよう design の単独所有から変更）・stdin・コマンド mpsc・`try_wait` 監視（~200µs `DRIVER_POLL_INTERVAL`）・shutdown watchdog（猶予超過で `child.kill()`）
- `reader` スレッド: stdout・`CommandReply` → 待機 waiter 起床・EOF → `seen_eof`（`Failed` 収束の一経路）。teardown では join せず detach（stdio を継承した孫プロセスが stdout を持ち続けると `read_exact` が永久ブロックし得るため）
- ローカル lifecycle ミラー（非 RT コマンドで更新 — `process` 等価の const 検査に使う）
- `Failed` 冪等収束: `seen_eof`・`try_wait → Some(ExitStatus)` の先着で 1 回のみ — `HostNotification::InstanceFailed { handle, exit: ChildExit }` を通知キュー（std mpsc）へ + pending コマンド waiter を `HostError::Ipc` で起床。ライブ中 exit は code 不問 `Failed`（voluntary exit 0 もプロトコル違反）・`Shutdown` 後 exit のみ clean teardown・自動再起動なし
- `Destroy` → `Shutdown` 送信 + reap + スロット解放

### A11 — `l1-test-worker` + 制御面 E2E（tests/l1_proxy_e2e.rs）

- `src/bin/l1-test-worker.rs`: wire 適合の最小 worker — `--crash-after-msgs=N`（N メッセージ後に `std::process::abort`/`exit` で死亡）・`--hang-after-msgs=N`（応答停止）で確定的異常系を駆動
- `tests/l1_proxy_e2e.rs`: `env!("CARGO_BIN_EXE_l1-test-worker")` で spawn（`tests/` 内に閉じる）— 制御面経路を検証:
  - コマンド parity: `Scan`→`Instantiate`→`Initialize`→`Activate`→`StartProcessing`→`SaveState`/`RestoreState`→`StopProcessing`→`Deactivate`→`Destroy` が L0 と同一の `HostReply`/`HostError` 意味（違法遷移 → `Err(Lifecycle(_))` — 子側 `Lifecycle` 判定）
  - クラッシュ契約: `kill`/crash → `Failed` 遷移 → `try_recv_notification` が `InstanceFailed` を返す → 以後コマンド（`Destroy` 除く）→ `Err(Failed)`・`Destroy` → `Ok`・自動再起動なし・pending waiter は `Ipc` 起床
  - handshake 失敗: descriptor 不一致・version 不一致・`command_timeout` → `Err(Ipc)` + kill/reap
  - GUI defined rejection: `OpenGui`/`CloseGui` → `Err(Unsupported)`（子へ転送しない）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-76-plugin-l1-out-of-process.md](../plans/implement-issue-76-plugin-l1-out-of-process.md)
- [../designs/plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)
- [../test-specs/plugin-l1-proxy-contract-tests.md](../test-specs/plugin-l1-proxy-contract-tests.md)
- [../test-specs/plugin-l1-wire-codec-tests.md](../test-specs/plugin-l1-wire-codec-tests.md)

## Dependencies

- 先行 task: [l1-core-contracts-and-wire-codec.md](l1-core-contracts-and-wire-codec.md) — wire codec・`LifecycleState::Failed`/`is_failed`・`HostError::{Failed,Ipc}` が前提
- `ruviform-rt`（path — Primitives 層・RT 脚の SPSC channel は次 task で配線・本 task は dep 追加のみ）
- `shared_memory = "0.12"` — design「依存追加の判断」どおり（internal `shm` 隔離）

## Non-Goals

- shm データ面の slot 配置・doorbell fence・RT 脚・`process` 等価フロー・pass-through（次 task — 本 task は制御面のみ）
- `PluginHost` コンポジット公開面（後続 task — 本 task の `l1` は `pub(crate)`）・`plugin_mut`/`register_out_of_process` 公開 API
- 自動再起動・bundle 子・共有リーパー/io 多重化・stderr 配管・L1 GUI 実装（`Unsupported` 拒否のみ）

## Affected Files

- `crates/ruviform-plugin-host/Cargo.toml`（`ruviform-rt` + `shared_memory` + `[[bin]]`）
- `crates/ruviform-plugin-host/src/l1/mod.rs`（新規 `pub(crate)` — spawn/handshake/driver/reader/lifecycle ミラー/`Failed` 収束/通知）
- `crates/ruviform-plugin-host/src/l1/shm.rs`（新規 `pub(crate)` — `shared_memory` 閉じ込め・key 生成・segment/header）
- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`（新規 — `--crash-after-msgs`/`--hang-after-msgs` fixture）
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（新規 — 制御面 E2E）
- `crates/ruviform-plugin-host/src/lib.rs`（`mod l1` — `pub(crate)`）
- `Cargo.lock`（`shared_memory` + transitive を記録）

## Completion Gate

3 entry 全 green 実 log（`check`/`lint`/`test`）+ lead review の後にのみ task 完了・buildable scope commit。review 焦点: `Failed` 収束が EOF/`try_wait` 両経路で冪等か（voluntary exit 0 を `Failed`・`Shutdown` 後のみ clean・pending waiter `Ipc` 起床・通知一回性）・`shared_memory` が `l1/shm` に閉じ込められ公開面非露出・自前 `unsafe` ゼロ — fake success なし。

## Verification

- [x] `mise run check` — exit 0（`l1`/`l1-test-worker` の型チェック・実 log）
- [x] `mise run lint` — exit 0（clippy `--all-targets -D warnings` clean）
- [x] `mise run test` — exit 0（制御面 E2E 20 tests 全 pass — parity・クラッシュ→`Failed`+通知+拒否・handshake 失敗・GUI `Unsupported` — workspace 55 suites・実 log）
- [x] `cargo tree -p ruviform-plugin-host` — `ruviform-rt` + `shared_memory`（+ transitive）が含まれる
- [x] `rg -n "unsafe" crates/ruviform-plugin-host/src/l1` — scoped `#[expect(unsafe_code)]` ×2（`l1/shm.rs` `bytes`/`bytes_mut`・SAFETY コメント付き — deviation: `shared_memory` 0.12.4 の `as_slice`/`as_slice_mut` は `unsafe fn`・`l1/mod.rs` は unsafe ゼロ — 詳細は IR）
- [x] `rg -n "pub.*shm|shared_memory" crates/ruviform-plugin-host/src/lib.rs` — `shared_memory` が公開 API に露出しない（`mod l1` は `pub(crate)`・`shared_memory` 型は `l1/shm.rs` に閉じ込め）

補足（task doc の Verification 範囲外・IR に記録）: `mise run fmt-check` exit 0・`mise run test-rt` exit 0（既存レーン無退行 — `l1` は非 RT のみ）。レビュー修正（P1: `Shared.child` 共有 + converge 側 force-kill・P2: driver-only `shutdown_initiated`・`try_wait` Err の kill+reap・reader detach・shm key retry・driver write 前 terminal check・doc 群）は mutation 検証済み — 詳細は `docs/impl/ir/issue-76-l1-proxy-control-plane-task-3-implementation.md`。
