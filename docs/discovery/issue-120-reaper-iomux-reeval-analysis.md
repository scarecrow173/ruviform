---
id: "DISC-fqJPHPj2JQlpssrbP4erm6"
type: "discovery"
status: "resolved"
title: "Issue #120 re-evaluation analysis — trigger #3 fired (per-child 3rd thread)"
created: "2026-10-30"
updated: "2026-10-30"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/120"
    - "https://github.com/scarecrow173/ruviform/issues/121"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../designs/plugin-l1-out-of-process.md"
    - "../designs/issue-117-bundle-child-process.md"
    - "../designs/issue-121-l1-diagnostics.md"
    - "../designs/issue-122-shared-platform-process-ipc-eval.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/plugin-l1-out-of-process.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/117"
    - "https://github.com/scarecrow173/ruviform/issues/118"
    - "https://github.com/scarecrow173/ruviform/issues/122"
  defers: []
  deferred-by: []
---

# Issue #120 再評価分析（trigger #3 発火 — per-child 3 スレッド目）

## 背景

Issue #120「L1 ドライバの共有リーパー/io 多重化」は post-#117 で deferred 化され、
再トリガ条件が 5 つ定義されていた。そのうち条件 3「#121 が stderr 配管を導入
（3 スレッド目/子 → mux 価値が増す）」が Issue #121 の着地（PR #145 merge）
により**発火済み**。本書は発火を受けた再評価の分析対象（検証済み事実・選択肢・
推奨）を記録する。

## 検証済み事実（コード実測 — 全て file:line 引用付き）

### スレッドモデル（per-child = 1 process + 3 threads）

| スレッド | ブロッキング対象 | 終了条件 | 周期/コスト |
|---|---|---|---|
| `l1-driver` | `rx.recv_timeout(200µs)`（`mod.rs:1579`）+ `write_all`（wedge 点・無制限） | `try_wait`→Some・write error・`Disconnected`・watchdog | 5000 wake/s・`try_wait` 毎 tick（`mod.rs:1668`） |
| `l1-reader` | `wire::read_message` blocking read（`mod.rs:1397`） | EOF/read error → `converge_child`・demux `Verdict::Child`（`mod.rs:1416`） | idle コスト 0 |
| `l1-stderr-drain` | `poll_stderr` — POSIX `libc::poll`（`diag.rs:512`・wait=`DRAIN_IDLE_WAIT`=10ms・`:71,:592`）・Win `PeekNamedPipe`+`thread::sleep(wait)`（`diag.rs:546-570`） | pipe EOF・`terminal`観測+500ms grace・poll/read error | 100 wake/s・detached（JoinHandle は呼出側で破棄 — `mod.rs:2110`） |

- video 子は `L1Child::spawn_scanned`（`video.rs:158` → `Self::spawn` at `mod.rs:2222`）経由で同一機構を再利用 — 同じ 3 スレッド（`video.rs:1-8`）
- Bundled: 子 = route + overflow（≤64 instance/子）。Dedicated: per-instance → 3N は N=instance 数に線形退化
- `Shared.child: OnceLock<Mutex<Child>>` — `kill`/`try_wait`/`wait` が全て `&mut` のため Mutex。driver が `write_all` で wedge 中でも任意の収束スレッドが `kill_child` できる契約（`mod.rs:651-656,748-752`）— **load-bearing**

### submit path（RT producer → driver）

- `ruviform_rt::channel`（rtrb SPSC・`data.rs:694-700` で `try_push`）
- `try_push` は driver を **wake しない** — 発見遅延 ≤ `DRIVER_POLL_INTERVAL`（200µs）
- RtLeg 側に wake オブジェクトは存在しない（`data.rs:467-505`）
- std-only doorbell 素材: `Thread::unpark`/`park_timeout`、`SyncSender::try_send` — 実在

### reaper 実現可能性

- 現行は `Mutex<Child>` のみ所有 — 生 pid/handle 未使用（`Child::id()`・`as_raw_handle()` 未使用）
- POSIX: `pidfd_open`/`kqueue EVFILT_PROC` は **signal-only** 必須 — 外部 `waitpid` が status を消費すると `Child::try_wait` が `ECHILD` 化。status 収集は driver 側 `try_wait`/`wait` に残す
- Windows: `Child::as_raw_handle()`（std stable）→ `WaitFor*`/`RegisterWait*` が status に干渉しない
- 必要な新規 FFI（現在未使用）: `Win32_System_Threading`・`pidfd`/`epoll`・`kqueue` — `diag.rs` の `#[expect(unsafe_code)]+SAFETY` 規約を拡張する形

### 未発火の条件

- 条件 1（実測 >16–32 子）・条件 2（200µs wake 起因の実測 idle-CPU/電力問題）・条件 4（新契約のレイテンシ厳格化）・条件 5（単一 reader スループット上限実測）は**未発火** — 発火したのは構造変化の条件 3 のみ

## 選択肢（review-protocol で価格付け修正済み）

| ID | Option | 効果 | 実コスト | スレッド | 判定 |
|---|---|---|---|---|---|
| A | re-defer（ledger 精密化） | 構造変化を記録し refined trigger で再判定待ち | なし | 3N | **推奨（下記根拠）** |
| B | R1 submit doorbell | submit 発見遅延 ≤200µs → ~0（jitter 除去のみ） | **RT 契約抵触 or FFI** — `SyncSender::try_send`/`mpsc::Sender::send` は lock/alloc で `RtLeg` の wait-free 契約（`data.rs:465-466,501-504`）に抵触。`Thread::unpark` は atomic+futex で lock/alloc しないが std は wait-free を保証せず契約改定が要る。lock-free wake は eventfd/`SetEvent` 等の FFI が要り「std のみ」利点消滅。idle wake 削減は `try_wait`/watchdog tick にゲートされ独立実現不可。`park_timeout`+`try_recv` 化は `DriverCmd` の wake-on-send を失い全 send サイトの unpark 配線も必要 | 3N | **deferred** — 利益が未実測 jitter のみで対価は契約変更か FFI |
| C | 共有 stderr drain（1 drain が全子 stderr を多重化） | **3N → 2N+H**（H=PluginHost 数 — `DiagRegistry` は per-host・`composite.rs:447`。process-global 化は per-host 境界違反 + crate に process-lifetime thread） | **crate 初のグローバル worker** — fd registry・fairness policy（drain-to-idle は flood 子による独占・capped read は ~800KB/s/子 に劣化で verbose 子をブロック）・exit-on-empty handoff・dead-thread respawn・`mark_eof`/`mark_deadline_exceeded`/`mark_unread_observed` の E2E pin 契約を entry 単位で複製（`l1_proxy_e2e.rs:3823` 等）・DrainInit phase の join-latency 整備。`poll_stderr` は fd 単位ブロッキング構造でそのまま転用不可（per-entry 直列化すると cycle=N×10ms）。wedge 確率は service level で ~N×・被害は全子 | 2N+H | **deferred** — 利益が未発火 N>1 条件にゲート・コストは新規サブシステム級 |
| C′ | driver fold（drain を driver の 200µs tick に畳み込み） | 3N → 2N | **host 起因の相互デッドロックを導入** — driver が `write_all`（blocking stdin）で wedge 中に子が stderr `write(2)` でブロック → 子が stdin を読めず双方永久停止（子 serve loop は単一スレッド — `plugin-export/lib.rs:142,269`）。回復は: 後続 `Command` の `reply_rx.recv_timeout`→converge→kill（spurious `Timeout` kill）・manual `kill_child`・teardown `join_driver`（`command_timeout`+1s bound）— だが **keep-alive かつ command-free の窓では自動契機が存在しない**（`BlockIn` doorbell には reply/timeout なし）。doorbell-only で pipe を満たすには ΣDᵢ≳64KiB（非典型設定）が要るが、大型 `Command`（`RestoreState` ≤16MiB）経由では典型設定でも到達可能 — その場合は timeout-bound だが健全な子を誤 kill する。drain 分離は「stderr back-pressure が制御面に到達しない」liveness 契約の load-bearing | 2N | **排除** — liveness 契約を弱化 |
| C″ | reader fold（reader が stdout+stderr を `poll`/`PeekNamedPipe` で多重化） | 3N → 2N | `write_all` wedge は発生しない（reader は stdin を書かない）が、`wire::read_message` の frame 途中 blocking read を poll-gated 段階読みへ再構成する必要（mid-frame 停まると drain が飢える）+ drain flag 契約の複製 — 新規 FFI なしだが中程度の再構成 | 2N | **deferred** — 実現可能だが C と同じ N>1 ゲート・reader 再構成コスト |
| D | R2 共有 reaper（signal-only） | `try_wait` poll 除去・exit 検出イベント化 | tri-OS 新規 FFI（Linux **pidfd のみ** — poll 可能で epoll 不要・macOS `kqueue EVFILT_PROC`・Windows `RegisterWaitForSingleObject` — WFMO は `MAXIMUM_WAIT_OBJECTS=64` 上限で不適）+ POSIX waitpid 消費制約（signal-only 必須・status は driver `try_wait`/`wait` が保持） | 3N+H（per-host。process-global signaler も検討可能だが同じ crate-worker 異議） | **deferred** — crash 検出は既に reader EOF 即時 + try_wait backstop で、tri-OS FFI の対価に見合わず |
| E | C+D 統合 watcher | 2N+H + exit イベント化 | C の registry 機構全て + D の FFI incremental（~3 call・同一 `#[expect]+SAFETY` 規約）。pidfd は Linux 5.3+ 要 | 2N+H | **deferred** — C と同じ N>1 ゲート + FFI |
| F | フル io-mux | ~N | Windows named pipe 再構成・stdin 非ブロッキング状態機械・3 バックエンド | ~N | **deferred** — 不釣合い判定は post-#117 と不変 |

## 推奨（review 修正版）

**A — re-defer。ただし ledger を本分析で精密化し、再トリガ条件を refined する。**

根拠:

1. **全実装 option が未発火条件にゲートされる**: C/E の thread 削減は N>1 でのみ実質（2N+H=3N @N=1）— N>1 は条件 1（実測 >16–32 子）の未発火領域。B の利益は未実測 jitter のみ。D は利益 marginal。発火した条件 3 は構造変化の記録であり、実測問題の発生ではない
2. **trigger #3 は再評価の発火であって実施命令ではない** — 「mux 価値が増す」を正確に評価した結果、価値は依然コスト閾を超えないことが判明した。これは deferred 機構の仕様どおりの正当な帰結
3. **「安い削減経路」は存在しないことが証明された** — driver-fold は deadlock 契約違反で排除。reader-fold（C″: stdout+stderr を reader で poll 多重化）は実現可能だが `wire::read_message` の poll-gated 再構成 + flag 契約複製が要り、利益は C と同じ N>1 ゲート — 2N への安価な経路はなく、残る経路は全て再構成コストか新規機構を伴う
4. **C の真のコストはサブシステム級** — 「採用済み primitive の共有化」は不実。registry+fairness+lifecycle+契約複製は新規機構であり、その設計は実測要求なしに確定できない
5. **N>1 実例は現存するが未測定** — video（dedicated-only）複数 instance・multi-route・dedicated audio で N>1 は到達可能。ただし条件 1 が定める実測閾には達していない

## Refined re-trigger 条件（ledger 更新提案）

- 実測 >16–32 子（multi-route/dedicated/multi-video で実測 — 現行どおり）
- 200µs wake 起因の実測 idle-CPU/電力問題（現行どおり）
- **新規**: crate レベルでグローバル worker 基盤（他用途の shared thread pool）が別理由で出現した場合 — C/E の registry コストが大幅に下がる
- **新規**: RT 契約の改定（wait-free 緩和 or FFI wake primitive 採用決定）が別 issue で起きた場合 — B の実現可能性が変わる
- **新規**: stderr drain の EINTR latent bug（`diag.rs:513` `Poll::Error`→drain 死亡）修正時に drain 構造を触る場合 — C への統合を同時評価
- sub-poll 検出や doorbell レイテンシ厳格化の新契約・単一 reader スループット上限実測（現行どおり）

## 副次発見（別 issue 報告）

- **latent bug**: `libc::poll` の EINTR が `Poll::Error` → drain 即死（`diag.rs:513-514`→`drain_stderr` return :606）。`read` 側 EINTR は retry される非対称（`:599`）。caught signal（SIGPROF 等）を使うホストプロセスで **stderr capture が静かに停止**する。さらに drain 死亡で `ChildStderr` が drop され read end が閉じるため、子の以後の stderr write は **EPIPE/SIGPIPE** となりブロックではなく失敗する — Rust 子は `SIGPIPE=IGN` 継承で `eprintln!` が **panic**（`plugin-export/lib.rs` の serve loop 内 `eprintln!` サイト）→ 子死亡 → reader EOF → `Failed` 収束。capture 喪失 + 子の誤死という、記述より重い既存問題。merged #121 コードの問題 — #120 とは独立して報告・修正対象
- stale doc: `l1/mod.rs:5`・`:713` が依然 "two per-child I/O threads" / "its two I/O threads" と記述（実際は 3 — #121 で更新漏れ）
