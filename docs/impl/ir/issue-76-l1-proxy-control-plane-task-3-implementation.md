---
id: "IMPL-034LT7uOCDj9JkF6u9Ut0L"
type: "impl"
status: "completed"
title: "Issue #76 L1 proxy control plane — Task 3 implementation"
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
      - type: "file"
        path: "docs/impl/ir/issue-76-l1-proxy-control-plane-task-3-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/tasks/l1-proxy-control-plane.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/l1-proxy-control-plane.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #76 L1 proxy control plane — Task 3 implementation

## Summary

Issue #76 Task 3 完了 — actions 9–11（deps + `l1` 骨格・L1
インスタンス機構・`l1-test-worker` + 制御面 E2E）を全範囲実施。
plugin-host に `pub(crate)` の `l1` モジュールを新設し、
`L1Instance`（spawn + `Hello`/`Scan` handshake + driver/reader
2 スレッド + ローカル lifecycle mirror + exactly-once `Failed`
収束 + 通知キュー + bounded teardown）・`l1::shm`（一意 key
retry 付き named segment 作成 + 28B header 書込/検証）・決定的
fixture `l1-test-worker`・`tests/l1_proxy_e2e.rs` 20 tests
（`#[path]` include）を提供する。独立レビュー修正（P1: wedged
driver `write_all` を解除する `Shared.child: OnceLock<Mutex<
Child>>` + converge 側 force-kill + P2 群 — driver-only
`shutdown_initiated`・`try_wait` Err の kill+reap・reader detach・
shm key retry・driver write 前 terminal check・doc 修正群）を
全て適用・mutation 検証済み。全 gate green 実 log。

## Context

approved plan `docs/plans/implement-issue-76-plugin-l1-out-of-process.md`
の Task 3（`docs/tasks/l1-proxy-control-plane.md`・actions 9–11）。
upstream: approved spec/design
（`docs/specs/plugin-l1-out-of-process.md`・
`docs/designs/plugin-l1-out-of-process.md`）・Task 1 IR
（`issue-76-l1-core-wire-task-1-implementation.md` — wire codec・
`LifecycleState::Failed`/`is_failed`・`HostError::{Failed,Ipc}` の
共有語彙）・Task 2 IR（`issue-76-plugin-export-task-2-
implementation.md` — 子側 `run`/argv 規約/`wire::shm` 正本の対抗
側）・TSpec `docs/test-specs/plugin-l1-proxy-contract-tests.md`
（クラッシュ契約・handshake 失敗・parity の正本）。本 task は
制御面のみ — shm データ面の slot 所有・RT 脚・pass-through は
Task 4、公開 `PluginHost` composite・`register_out_of_process`・
Destroy legality gate・`Send`/`Sync` 決定は Task 5。

## Implementation

### Changes

- `crates/ruviform-plugin-host/Cargo.toml` — `ruviform-rt =
  { path = "../ruviform-rt" }`（Primitives 層・A.2 許可範囲 — 本
  task は dep 追加のみ・RT 脚の SPSC channel 配線は Task 4）+
  `shared_memory = "0.12"`（design「依存追加の判断」どおり —
  MIT OR Apache-2.0・依存木小・Windows+POSIX named object・
  `l1::shm` 閉じ込め。manifest コメントに 0.12.4 の unsafe
  accessor 事実修正を記録）+ `[[bin]] name = "l1-test-worker"`。
- `crates/ruviform-plugin-host/src/lib.rs` —
  `#[allow(dead_code)] pub(crate) mod l1`（crate 内の消費者はまだ
  無い — `PluginHost` composite は Task 5。item 個別ではなく
  module 単位の `dead_code` 許可 + コメントで理由を明記）。
- `src/l1/mod.rs`（982 行・`pub(crate)`）— L1 制御面全体:
  - `SpawnSpec` — `descriptor`（`--ruviform-plugin-id` + `Scanned`
    照合 oracle）・`program`/`args`・`lookahead_blocks`（default
    2 → shm depth `D = lookahead + 1`）・`command_timeout`
    （default 10 s — `Hello`/`Scan`/全 `Reply` wait の defined
    timeout）。`OutOfProcessSpec` の `pub(crate)` mirror。
  - `ChildExit` — `code: Option<i32>` + `signal`（unix のみ
    `#[cfg]`）— 通知 payload（後続 task の
    `HostNotification::InstanceFailed` 本体）。
  - `L1Instance::spawn` — `shm::Segment::create`（depth pair）→
    `ProcCommand`（`spec.args` + `--ruviform-plugin-id` +
    `--ruviform-shm-key`・stdin/stdout piped・stderr inherit —
    piping は deferred design concern）→ **driver thread が
    child を spawn し `Shared.child`（`OnceLock<Mutex<Child>>`）
    に install** してから `driver_main` へ — `Child` 所有は
    生成時から driver 上にあり、spawn/handshake 失敗経路は全て
    channel disconnect + shared-child kill + reap で bounded に
    回収（leak なし）。`handshake` = `Hello` 受信（
    `version == WIRE_VERSION`）→ `Scan` → `Scanned` の
    `wire::Descriptor` が `spec.descriptor` と `matches`
    フィールド等価 — 全失敗は `Err(Ipc)` + kill/reap（通知
    なし — インスタンスは live になっていない）。最悪
    `3 × command_timeout`（spawn hand-off・`Hello` wait・`Scan`
    reply — doc 明記・レビュー修正）。
  - thread model — `driver`（child stdin・`DriverCmd` mpsc
    drain・`try_wait` crash watch を `DRIVER_POLL_INTERVAL =
    200µs` で poll・shutdown watchdog: grace 超過で
    `child.kill()`）・`reader`（child stdout・`Reply` → seq
    keyed waiter 起床・stale seq drop・EOF → `seen_eof` →
    converge・2 度目の `Hello`/未送付 `BlockDone`/逆方向 kind は
    protocol violation → converge）。caller thread は bounded
    wait（`command_timeout` 上限）以外の blocking pipe I/O を
    行わない。lock order: `inner` と `child` の critical
    section は常に disjoint（同時保持なし — module doc 明記）。
  - `converge(shared, exit)` — 全 terminal path の単一収束点・
    exactly-once（`terminal.is_some()` で早期 return — ただし
    `Notify::Pending` は遅れて来た `exit` status で一度だけ
    `Sent` に完了）。`Failed` = `!shutdown_initiated`（ライブ中
    exit は voluntary exit 0 含め code 不問で protocol 違反）·
    mirror → `Failed`・pending waiters を全て
    `Err(HostError::Ipc)` で起床（unbounded send — lock 内で
    安全・exactly-once が atomic）・通知は status 既知なら即送・
    未知なら `Pending` → driver の `try_wait`/`reap` が実
    status で完了（通知は常に実 status を運ぶ）。**`Failed`
    収束は `kill_child` を自スレッドで直接実行** — driver の
    channel 経由ではない（wedged child が stdin を drain しない
    場合 driver は `write_all` 内で block され kill order は
    永久に読まれない — 直接 kill が write を EPIPE 化し driver
    を unwind させる・P1 レビュー修正）。
  - `shutdown_initiated` — **driver のみが set**（wire `Shutdown`
    write 直前・`try_wait` 非 dead・`terminal`/`seen_eof` 無しの
    条件下のみ — driver write 前の terminal check）。wire
    `Shutdown` 前の死亡は `Failed` + 通知を保持し、後の exit のみ
    clean teardown（`Terminal::Clean`・mirror → `Destroyed`・
    通知なし）。
  - `driver_main` の `try_wait` 3 分岐 — `Some(status)` →
    converge・`Err`（状態不明）→ kill + 即時 deadline reap +
    converge（POSIX zombie 防止・P2 修正）・watchdog deadline
    超過 → reap + converge。`Command` 受信時は converged/
    shutdown-in-flight を先に検査し doomed frame を書かず waiter
    を `Err(Ipc)` で起床（P2 修正）。
  - `execute` — defined rejections（wire を触らない）:
    `Terminal::Failed` → `Err(Failed)`・`Clean`/shutdown
    initiated → `Err(Ipc)`・`OpenGui`/`CloseGui` →
    `Err(Unsupported)`（転送しない — L1 GUI deferred）・
    `Activate` の `max_frames > WIRE_MAX_FRAMES` →
    `Err(Prepare(UnsupportedConfig))`。`request` — waiter
    登録 → driver へ enqueue → `command_timeout` wait →
    timeout は waiter 除去 + converge（`Err(Ipc)` — child は
    喪失と推定・converge の kill が wedged driver write も解除）。
    成功 reply で lifecycle mirror を `post_command_state` に
    更新（converged 後は resurrect しない）。
  - `destroy` — 常に成功・冪等（`Failed` インスタンスへは wire
    `Shutdown` を送らず kill — channel message は wedged
    `write_all` の後ろに詰まり得るため shared handle 経由）・
    live には `DriverCmd::Shutdown` → watchdog grace → join。
    mirror → `Destroyed`。L0 の `Destroy` legality gate
    （`Activated`/`Processing` からの拒否）は本層の責務ではなく
    Task 5 composite の gate（doc 明記・レビュー修正）。
  - `kill`/`lifecycle_state`/`try_recv_notification` —
    test/composite 用の制御面 primitive（通知は
    `notify_rx.try_recv` — `Some` は収束 1 回につき 1 度のみ）。
  - `join_threads` — driver join は bounded（shutdown grace +
    `DRIVER_JOIN_MARGIN` 1 s・`is_finished` poll → 超過で
    force-kill → join）・**reader は join せず detach** —
    stdio 継承の孫プロセスが stdout を握り続けると `read_exact`
    が永久に EOF を見ず join が hang し得るため `JoinHandle` drop
    で detach（reader は `Arc` 共有状態のみ触り、process exit
    が回収 — P2 修正・`--leave-heir-after-msgs` E2E で実検証）。
  - `Drop` — `Destroy` なしの drop は channel disconnect →
    driver の `Disconnected` arm が kill+reap+converge（通知は
    drop 済み receiver へ）・bounded。
  - `#[path]` include 制約 — `pub(crate)` のため
    `tests/l1_proxy_e2e.rs` が `#[path]` で取り込む（`src/l1/` 内
    は `crate::` 不使用・`ruviform_plugin_core::`/`std::`/
    `self::`/`super::` のみ — module doc に import discipline
    明記）。
- `src/l1/shm.rs`（283 行・`pub(crate)`）— `shared_memory`
  閉じ込め（公開 API・feature 名非露出）:
  - `unique_key` — `/rvl1-shm-<pid>-<slot>-<salt>-<counter>`:
    design の `pid + slot + counter` に per-process salt
    （`SystemTime` nanos・`OnceLock`）を追加 — counter は
    process 毎に 0 から再開するため、crash した host の PID 再
    利用で stale segment と key が衝突し得た（P2 修正）。
  - `Segment::create` — `create_keyed` seam（test が generator を
    注入して `MappingIdExists` を決定的に駆動）経由で
    `MAX_KEY_ATTEMPTS = 8` の distinct key retry — `Exists` のみ
    retryable・`Other` は即失敗。`create_named` が 28B header を
    書込み（`wire::shm` 定数から — 正本化済み）・`verify_header`
    が全フィールドを再読み検査（自己検証 half・test と Task 4
    の slot setup が使用）。
  - `Drop` が named object を unlink（POSIX `shm_unlink`・
    Windows は mapping unmap + delete-on-close rename — module
    doc 明記）。slot body（`seq`/scalars/transport/result/
    events/samples）は Task 4 のデータ面 — 本 task は制御面の
    attach 前提として segment 存在 + header のみ供給。
  - scoped `#[expect(unsafe_code)]` ×2（`bytes`/`bytes_mut` —
    `shared_memory` 0.12.4 の `as_slice`/`as_slice_mut` が
    `unsafe fn` の deviation・export `shm.rs` と同一記録 —
    SAFETY: header は owner が spawn 前に一度だけ書き以後 read
    のみ・`&mut Shmem` borrow が排他）。
- `src/bin/l1-test-worker.rs`（272 行）— wire 適合の最小
  scripted 子 fixture（`Hello` → frame loop・`Lifecycle` で
  transition legality を実判定 — proxy test が本番順序を観測・
  `Activate` も config bound を本番と同じ validate-first で
  模倣）:
  - `--crash-after-msgs=N`（Nth message 受信時に `process::exit`
    — dispatch 前なので Nth waiter は pending のまま EOF/
    `try_wait` で起床）・`--crash-exit-code=C`（default 3・
    `0` で voluntary exit 0 → `Failed` violation を模倣）・
    `--hang-after-msgs=N`（応答停止 — `Shutdown` は受理し
    teardown が決定的）・`--stop-reading-after-msgs=N`（stdin を
    二度と読まない wedged child — 大きな frame で proxy の
    `write_all` が block され kill のみが解除）・
    `--leave-heir-after-msgs=N`（stdio 継承の heir を spawn して
    exit — proxy reader に EOF が届かない経路）・`--sleep-ms=M`
    （heir mode）・`--hello-version=V`（version mismatch）。
- `tests/l1_proxy_e2e.rs`（546 行・20 tests —
  `#[path = "../src/l1/mod.rs"] mod l1` + `env!("CARGO_BIN_EXE_
  l1-test-worker")` spawn — `tests/` 内に閉じる）:
  - parity — `command_parity_lifecycle_and_state`（`Scan` →
    全 lifecycle 動詞 + `SaveState`/`RestoreState` 往復 →
    `Destroy`）・`illegal_transitions_match_l0_semantics`
    （`Err(Lifecycle(_))` — 子側 `Lifecycle` 判定）。
  - crash 契約 — `crash_transitions_failed_and_notifies`・
    `kill_transitions_failed_and_notifies`・`voluntary_exit_
    zero_is_failure`・`hang_converges_on_command_timeout` —
    `Failed` 遷移・`try_recv_notification` が `ChildExit` を
    1 度だけ返す・以後 `Destroy` 以外 `Err(Failed)`・`Destroy`
    → 成功・自動再起動なし・pending waiter `Ipc` 起床。
  - handshake 失敗 — `handshake_descriptor_mismatch`・
    `handshake_version_mismatch`・`handshake_scan_timeout`・
    `spawn_failure_is_ipc` → 全て `Err(Ipc)` + kill/reap。
  - defined rejection — `gui_defined_rejection`（`OpenGui`/
    `CloseGui` → `Err(Unsupported)`・子へ非転送）・
    `activate_over_wire_limit_rejected`。
  - teardown — `destroy_is_clean_teardown`（`Shutdown` 後 exit
    は通知なし clean）・`drop_reaps_live_child`・
    `wedged_child_write_converges_and_teardown_is_bounded`
    （`--stop-reading-after-msgs` + 大 frame — converge の
    force-kill が driver `write_all` を EPIPE 解除・teardown
    bounded — P1 修正の直接検証）・`destroy_detaches_reader_
    when_heir_holds_stdout`（heir が stdout を保持しても
    `Destroy` が bounded — reader detach の直接検証）。
  - shm — `shm_create_retries_past_stale_name`・
    `shm_create_gives_up_on_persistent_collision`（keyed seam で
    `MappingIdExists` を決定的注入）・`shm_header_roundtrip_
    matches_wire_layout`（`verify_header` + `wire::shm` const
    整合）・`shm_unique_keys_never_repeat`。
- `Cargo.lock` — `ruviform-rt` + `shared_memory`（+ transitive）
  を `ruviform-plugin-host` エントリに記録。

### Review findings and resolutions（独立レビュー P1 ×1 + P2 群 — mutation 検証済み）

- **P1（fixed）— wedged driver `write_all` が kill/destroy/drop
  を無効化**: design/初期実装は `Child` を driver 単独所有とし
  kill は channel message 経由だった — しかし stdin を drain
  しない live wedged child（`--stop-reading-after-msgs`）は
  driver を pipe capacity 超過の `write_all` 内で永久 block し、
  kill order は二度と読まれない → `destroy`/`drop`/caller-side
  `command_timeout` すら全て無効化される非 bounded teardown。
  `Shared.child: OnceLock<Mutex<Child>>` + `kill_child` を新設
  し、**収束側スレッドが自ら force-kill** する構造に変更 —
  kill が write を EPIPE 化して driver を unwind させ、driver
  の `try_wait`/`reap` が実 exit status を観測する（`Notify::
  Pending` が必ず実 status で完了する理由でもある）。
  `wedged_child_write_converges_and_teardown_is_bounded` で
  mutation 検証（単独所有に戻すと hang/timeout 失敗）。task
  doc A10 の単独所有記述は共有所有に修正済み。
- **P2（fixed）— `shutdown_initiated` を driver-only に移動**:
  caller thread の `destroy` が `Shutdown` enqueue 前に
  flag を立てていた — enqueue と wire write の間に child が
  crash すると clean teardown と誤判定され `Failed` 通知を喪失
  し得た。wire `Shutdown` write 直前の driver のみが set する
  よう移動（`try_wait` 非 dead・`terminal`/`seen_eof` 無し条件
  — driver write 前の terminal check と一体）— wire `Shutdown`
  前の死亡は必ず `Failed` + 通知を保持。
- **P2（fixed）— `try_wait` Err の kill+reap+status**: driver の
  `try_wait` が `Err`（状態不明）を返す経路が converge せず
  poll 継続だった — POSIX では zombie を残し得る。kill + 即時
  deadline `reap` + `converge(Some(exit))` に修正（2 回目の失敗
  でも default `ChildExit` で `Pending` を完了）。
- **P2（fixed）— reader を join せず detach**: POSIX では
  stdio を継承した孫プロセスが親死亡後も stdout pipe を保持し
  `read_exact` が EOF を見ず、reader join が永久 hang し得た。
  `join_threads` は `reader_join` を drop（detach）— reader は
  `Arc` 共有状態のみ触り process exit が回収する。
  `destroy_detaches_reader_when_heir_holds_stdout` +
  `--leave-heir-after-msgs` fixture で実検証（mutation: join
  に戻すと test が timeout）。task doc A10 に注記済み。
- **P2（fixed）— shm key retry + process salt**: design の
  `pid + slot + counter` は counter が process 毎に 0 から再開
  するため、crash host の PID 再利用で stale segment と key が
  衝突し得た。`process_salt`（`SystemTime` nanos・`OnceLock`）
  を key に混合 + `Segment::create` が `MappingIdExists` で
  `MAX_KEY_ATTEMPTS = 8` の distinct key retry（`create_keyed`
  seam で決定的注入 — 2 E2E）。
- **P2（fixed）— driver write 前の terminal check**: converged
  /shutdown-in-flight な instance への `Command` は doomed
  frame を書かず waiter を `Err(Ipc)` で起床するよう driver 側
  に reject 分岐を追加（spec 禁止の post-`Failed` write を構造
  で排除）。
- **doc 修正群（fixed）**: `spawn` の最悪 `3 × command_timeout`
  明記・`destroy` の L0 legality は Task 5 composite の gate 旨・
  `L1Instance` の `Send`/`Sync` は Task 5 の公開面決定として
  固定しない旨・`l1/shm.rs` の 2 scoped unsafe site（deviation
  記録）— 全て doc 明記。
- **`wire::shm` 正本化（structural fix）**: segment header/slot
  layout 定数が export `shm.rs`・host `l1::shm`・e2e 組立に手
  同期で三重化されていた — `ruviform-plugin-core::wire::shm`
  pub module に authoritative 化（Task 2 commit で供給・host 側
  は本 task で消費へ移行 — 3 手同期コピー → 単一正本。
  `shm_layout_offsets_are_self_consistent` で offset 連鎖を
  const-assert 検証）。

### Decisions During Implementation

- **`Child` 所有の共有化**（deviation・P1 の structural fix）—
  design は driver 単独所有を記述したが、それは「収束側が
  wedged driver の blocking write を解除できない」という
  bounded-teardown 違反を生む。`OnceLock<Mutex<Child>>` 共有は
  その違反を所有境界で直す最小の整合変更 — lock order
  （`inner`/`child` disjoint・child section は 1 call）で
  deadlock なしを構造保証（module doc 明記）。task doc A10 の
  deviation note に記録済み。
- **`Failed` 収束の exactly-once は 2 段構成** — `terminal`
  （`Failed`/`Clean`）が transition の一回性、`Notify`
  （`Idle`/`Pending`/`Sent`）が通知の一回性を担う — EOF path
  が先に着火しても通知は driver の reap した実 status を待つ
  （`Pending`）— 通知が常に実 `ChildExit` を運ぶ契約。
- **exit-0 も `Failed`**（design 確定どおり）— `shutdown_
  initiated` 前のライブ中 exit は code 不問で protocol 違反
  （voluntary exit 0 も — child は `Shutdown` なしに終了しては
  ならない）。`--crash-exit-code=0` + `voluntary_exit_zero_
  is_failure` で pin。
- **`#[path]` include は public API 未露出の正直な反映** —
  `l1` は `pub(crate)` で crate の公開面から到達不能（composite
  は Task 5）— e2e は `#[path]` で取り込む（`ruviform-math`
  benches の `tests/simd_poc` と同じ手法）。`src/l1/` 内の
  `crate::` 禁止を module doc の import discipline で強制。
- **frame helper は各 crate に単純コピー** — `read_message`/
  `write_message` が `l1/mod.rs`・`l1-test-worker.rs`・e2e・
  export `read_frame`/`send` と 4 箇所に同形状で散在 —
  wire 層への共通 helper 格上げは Task 4 以降の follow-up
  （現状は各境界の必要最小限 — 早い抽象化を避けた）。
- **`ruviform-rt` は dep 追加のみ** — task doc どおり本 task で
  SPSC/RT 脚は配線しない（`Primitives` 層・A.2 許可範囲の宣言
  のみ — Task 4 が消費する）。
- **`HostNotification` は `ChildExit` + std mpsc の最小形** —
  `HostNotification::InstanceFailed { handle, exit }` の公開型は
  Task 5 composite が供給 — 本 task は `ChildExit` payload と
  `try_recv_notification` の queue primitive のみ。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — `shared_memory` 隔離条件・driver/reader 2 スレッド・
`Failed` 冪等収束は design 確定事項（新規 experiment なし）。

## Validation

- gate（実 log・全 exit 0）:
  - `mise run fmt-check` — exit 0
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets` — `l1`/`l1-test-worker`/e2e の型チェック含む）
  - `mise run lint` — exit 0（clippy `--all-targets -D warnings`
    clean — `l1` 内 `#[expect]`/`#[allow]` は `l1::shm` の
    2 scoped unsafe site と lib.rs の module-level
    `#[allow(dead_code)]` のみ）
  - `mise run test` — exit 0（`cargo test --workspace` —
    **55 test suites 全 pass・0 fail** — `l1_proxy_e2e` 20 tests
    含む・既存 L0 テスト無退行）
  - `mise run test-rt` — exit 0（`rt_` 既存レーン無退行 —
    `l1` は非 RT のみ・`Mutex`/`JoinHandle` は RT-suitable API
    に到達しない）
- 構造確認（実コマンド）:
  - `cargo tree -p ruviform-plugin-host` — `ruviform-rt` +
    `shared_memory`（+ transitive）を確認
  - `rg -n "unsafe" crates/ruviform-plugin-host/src/l1` —
    `l1/shm.rs` の 2 scoped `#[expect(unsafe_code)]` のみ
    （`l1/mod.rs` は unsafe ゼロ）
  - `rg -n "shared_memory" crates/ruviform-plugin-host/src/lib.rs`
    — 公開 API 非露出（`mod l1` は `pub(crate)`・`shared_memory`
    型は `l1/shm.rs` に閉じ込め）
- レビュー修正の mutation 検証（実施済み）:
  - `Shared.child` 共有 + converge 側 kill → 単独所有 +
    channel kill へ戻すと `wedged_child_write_converges_and_
    teardown_is_bounded` が hang/timeout で失敗
  - reader join へ戻すと `destroy_detaches_reader_when_heir_
    holds_stdout` が timeout で失敗
  - key retry 除去で `shm_create_retries_past_stale_name` 失敗
  - driver write 前 terminal check 除去で post-converge write
    経路の test が失敗

## Non-Goals

- shm データ面（slot 記述/読取り・doorbell fence の正式 ordering
  ・RT try-legs・`process` 等価フロー・pass-through）— Task 4。
  `l1::shm` は segment 作成 + header のみ・`L1Instance.segment`
  は attach 前提の keep-alive。
- `PluginHost` composite 公開面・`register_out_of_process`・
  統一ハンドル空間・`Destroy` legality gate・`HostNotification::
  InstanceFailed` 公開型・`L1Instance` の `Send`/`Sync` 決定 —
  Task 5（本 task の `l1` は `pub(crate)` — destroy doc と struct
  doc に明記）。
- 自動再起動・bundle 子・共有 reaper/io 多重化・stderr piping
  （inherit のまま — deferred design concern）・L1 GUI 実装
  （`Unsupported` 拒否のみ）・wire バージョン進化。
- doorbell `seq` 空間と command `seq` 空間の disjoint 化 —
  Task 4 の seq-space partition（export 側 `BlockIn` arm の
  TODO と対応 — `BlockDone`/`Reply` correlation が command
  waiter と alias し得る現状の記録）。
- `execute` の RT 化 — 全経路は non-RT only（mutex・bounded
  wait — RT leg は Task 4 が `l1` mirror 読取りを const-cheap
  snapshot に置き換える）。

## Risks

- `Shared.child: OnceLock<Mutex<Child>>` の共有は P1 修正の
  structural change — lock order（`inner`/`child` disjoint・
  child section は `try_wait`/`kill`/`wait` 1 call・`reap` は
  per-poll 再取得）で deadlock 不在を構造で担保したが、Task 4
  の RT 脚が `Shared` を触る場合は同規則の維持が前提（module
  doc に明記済み）。
- reader detach は thread leak として観測され得る — ただし
  reader は `Arc` 共有のみ触り process exit が回収する。heir が
  stdout を保持し続ける限り reader は `read_exact` に残る —
  `destroy`/`drop` 後も instance 寿命を越えて残り得る点は
  EOF starvation と引換えの deliberate 選択（`join_threads`
  doc 明記）。
- `DRIVER_POLL_INTERVAL` 200µs は scheduler 粒度に依存 —
  crash 検出の latency bound を構成するものではない（doc
  明記 — 「pace a poll, not a deadline」）。CI 上の遅い host で
  `NOTIFY_TIMEOUT` 10 s の headroom を持たせている。
- `l1-test-worker` と実 runner（Task 2）の子側意味論の drift —
  worker は `Lifecycle` で legality を実判定し `Activate` の
  config bound も本番順序で模倣しているが、hook 失敗/panic/
  shm slot の実処理は Task 4 で実 runner との proxy E2E を組む
  段階で真に担保される（fixture は制御面の oracle に留まる）。
- `command_timeout` 超過の converge は child を kill するが、
  timeout 判定が false positive（slow-but-alive child）の場合
  健全な instance を `Failed` にする — defined contract
  （spec: timeout は `Ipc` + `Failed`）どおりであり、worker
  `--hang-after-msgs` で検証済み。

## Follow-ups

- Task 4 `docs/tasks/l1-shm-data-plane-and-rt-legs.md`（actions
  12–14）— shm slot 所有（`L1Instance.segment` の consumer）・
  doorbell `BlockIn`/`BlockDone` 配線・seq-space partition・
  `ruviform-rt` SPSC の RT try-legs・`process` 等価・
  `compiler_fence` の正式 cross-process ordering 契約（export
  側の最小 fence と対で確定）。
- Task 5 `docs/tasks/plugin-host-composite-and-l1-e2e.md`（actions
  15–17）— `PluginHost` composite + unified handle space +
  `register_out_of_process` 公開 API・`Destroy` legality gate
  （本 task の `destroy` は無条件 — L0 規則の gate は
  composite 層）・`L1Instance` の `Send`/`Sync` 公開面決定・
  `HostNotification::InstanceFailed` 公開型（`ChildExit` +
  queue を包む）。
- frame helper dedup — `read_message`/`write_message` が
  `l1/mod.rs`・`l1-test-worker.rs`・`l1_proxy_e2e`・export
  `read_frame`/`send` と 4 箇所に同形状で散在（Task 2 IR に
  同 follow-up 記録 — wire 層への共通 helper 格上げ or
  test-only dedup を Task 4 以降で評価）。
- `WireError` → `HostError::Ipc` の読み取り経路 — reader の
  malformed frame は `Failed` 収束だが、driver write 側の
  encode failure も同じ `Failed` — protocol loss と transport
  loss の区別は現状 `Ipc` に畳まれている（design どおり —
  通知 payload に原因詳細を持たせるかは Task 5 の通知型で
  再評価）。
- `stderr` piping — `Stdio::inherit` のまま（deferred design
  concern — spawn 経路に明記）。plugin 診断の配管は別途設計。
