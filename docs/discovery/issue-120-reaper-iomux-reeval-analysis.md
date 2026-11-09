---
id: "DISC-fqJPHPj2JQlpssrbP4erm6"
type: "discovery"
status: "resolved"
title: "Issue #120 re-evaluation analysis — trigger #3 fired (per-child 3rd thread)"
created: "2026-10-30"
updated: "2026-11-09"
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

## Post-#147 追記（EINTR 修正後のトリガエッジ評価）

refined 再トリガ条件の一つ「stderr drain の EINTR latent bug 修正時に drain 構造を触る場合 — C への統合を同時評価」のエッジケースを #147 修正（PR #148・`a14f0db`）後に明示評価した。

- **トリガ前提は不成立**: #147 の修正は `poll_stderr` の戻り値写像を `classify_poll` に抽出したのみで、drain の構造（per-child スレッドが `ChildStderr` を所有・`DRAIN_IDLE_WAIT` ポーリング・exit-observation deadline・自己終了）は不変。条件文の「drain 構造を触る場合」という前提に該当しない
- **メリット側の再評価（実施）**: 仮に同時評価した場合でも結論は不変と判断 —
  - `classify_poll` は純粋関数として C（共有 drain）でも再利用可能だが、シグネチャが call 単位結果（`rc`/`errno`）と fd 単位結果（`revents`）を結合しており、fd 配列 `poll` では分割が必要 — 再利用の価値は限定的
  - C の支配的コスト（crate 初グローバル worker・fd→状態 registry・fairness・lifecycle・EOF/deadline 契約複製・テスト隔離）は #147 で一切低下していない
  - 利益側ゲート（実測 N>1・2N+H < 3N が効く領域）も依然未発火 — N=1 では 2N+H=3N で削減効果ゼロ
- **結論**: re-defer 継続。副次発見の latent bug は #147 で修正・merged 済み、stale doc は本コミット群で修正済み — 副次発見 2 件ともクローズ

## 実測記録（2026-11-09・Windows — Issue #120 受入証跡）

Issue #120 受入条件 (1)「per-child 構成・reader EOF 検出・idle CPU・process 数の現状測定と再トリガ条件の成立有無を記録」への応答。計測 fixture は `crates/ruviform-plugin-host/tests/l1_scale_probe.rs`（全テスト `#[ignore]`・手動実行専用・CI 非対象）。環境: **Windows 11 x64・debug build（`cargo test` dev profile）・単一マシン**。macOS/Linux 実機は **NotRun**（対象外）。以下の `PROBE` 行が生証跡であり、外部サンプラ（`Get-Process`）が process/thread/CPU を採取した。

### per-child 構成・process/thread 数

`probe_scale_idle`（`L1_PROBE_CHILDREN`/`L1_PROBE_INSTANCES`/`L1_PROBE_IDLE_SECS`）が spawn → idle → teardown の位相を `PROBE phase=` 行で出力し、その間に外部サンプラが CPU/thread を採取。instance は mint のみで `Activate`/`StartProcessing` は行わない — 測定対象は driver/reader/stderr-drain の idle コストであり、これらのスレッドは instance の processing 状態に依らず子ごとに存在するため、この形状で代表値となる。

| 形状 | 子プロセス | host スレッド（実測） | spawn/mint コスト |
|---|---|---|---|
| Bundled 4 子 × 8 instance（計 32） | 4 + host | 14–17（≒ 3×子 + main/harness） | spawn ≈8.1ms/子・mint ≈0.68–1.23ms/instance |
| Dedicated 形状 8 子 × 1 instance | 8 + host | 26–29（≒ 3×子 + main/harness） | 同上 |

worker プロセス（`l1-test-worker`）は各 1 スレッド。3 スレッド/子（driver・reader・stderr-drain）が host 側に存在する構造は #121 以降の記述どおり実測で確認。

### idle CPU（15 秒窓・`Get-Process` CPU 差分）

| 形状 | host CPU Δ | worker CPU Δ 合計 | 換算 |
|---|---|---|---|
| Bundled 4 子・32 instance | 0.172 s | ≈0.047 s | host ≈1.1% of one core・子あたり ≈0.3% |
| Dedicated 8 子・8 instance | 0.422 s | 0 s | host ≈2.8% of one core・子あたり ≈0.35% |

`recv_timeout(200µs)` の実効 cadence（`probe_recv_timeout_cadence`・300 reps）: avg **15634.8µs**・min 8886.3µs・med 15828.8µs・max 28265.4µs — Windows のタイマ量子化で設計上の 5000 wake/s は実際 ~64 wake/s。idle CPU は子あたり ~0.3% of one core に留まり、電力上の問題を示す実測値は得られなかった。

### reader EOF 検出・収束レイテンシ（`probe_crash_latency`・10 reps）

| 経路 | `Failed` mirror（med） | `FailedNotice`（med） |
|---|---|---|
| `eof_kill`（健全子を host `kill()`） | 2335.1µs | 13181.0µs |
| `noeof_kill`（`--spawn-heir` で EOF 不到達 — `try_wait` backstop のみ） | 12776.9µs | 12780.4µs |
| `eof_crash`（子が `exit(3)` — reader EOF 即時検出） | 1758.9µs | —（exec_err med 1755.7µs） |

EOF 経路は ~2ms で収束。EOF 不到達時は ~12.8ms ≈ 実効タイマ tick 1 回分に律速され、設計どおり `try_wait` backstop が機能。reader EOF 即時検出の記述どおりの挙動を確認。

### command/data plane スループット

- `probe_command_rtt`（500 reps・`SaveState` 往復）: **14313.8 ops/s**・avg 69.6µs・med 66.4µs・max 272.9µs
- `probe_block_rate`（単一 instance・512 frame）:
  - lookahead=2（slot depth=3）: offered 100/s → dropped 3/1000（定常ほぼ無損失）。offered 500/s → dropped 1229/2000 → 持続受理 ≈**190 blocks/s**（slot alternation × driver tick の往復律速 — child/reader ではない）
  - lookahead=16（depth=17）: offered 2000/s → 受理 ≈**1085 blocks/s** — per-instance 上限は slot depth に比例して伸び、child+reader は ~1000 blocks/s/instance を余裕でさばく
- `probe_bundled_block_rate`（**1 子 × 64 instance・各 ~100 blocks/s offer = aggregate ~6400 blocks/s**・30 rounds）: **dropped_submits=0・passthrough_submits=0**（1920 calls）— Bundled 最大占有率で audio レート（512 frame @48kHz ≈ 93.75 blocks/s）の 64 instance が無損失で持続

### 再トリガ条件の成立判定（refined 条件との照合）

| refined 条件 | 実測結果 | 判定 |
|---|---|---|
| 実測 >16–32 子 | Bundled は `ceil(N/64)` 子 — 64 instance 最大 bundle が無損失で成立。>16 子は dedicated opt-in か >1024 instance を要し、実測対象の需要は存在しない（dedicated 8 子形状はコスト測定のみ） | **不発火** |
| 200µs wake 起因の idle-CPU/電力問題 | 実効 cadence ~15.6ms・idle ≈0.3% of one core/子 — 問題を示す実測なし | **不発火** |
| crate 級グローバル worker 基盤の出現 | 出現していない | **不発火** |
| RT 契約改定 / FFI wake primitive 採用 | 出現していない | **不発火** |
| drain 構造を触る機会 / レイテンシ契約厳格化 | 出現していない（#147 は構造不変と評価済み） | **不発火** |
| sub-poll 検出・doorbell レイテンシ契約・単一 reader スループット上限 | reader+child は aggregate 6400 blocks/s・command 14.3k ops/s を無損失で処理 — 上限未観測。per-instance ~190/s 上限は slot depth 由来の設計特性（lookahead で伸びる）で reader/reaper の問題ではない | **不発火** |

### 結論

**re-defer 維持 — 共有 reaper / full io-mux の実装なしが正しい結果。** 全 refined 再トリガ条件が実測で不発火と確認された。trigger #3（構造変化）は引き続き「再評価の発火」であって実施命令ではなく、実測は全 option のコストが利益を上回ることを裏付けた。

**留保（正直な記録）**: 計測は Windows 11 x64・debug build・単一マシンの値であり、macOS/Linux 実機・release build・実ワークロード（実 plugin の block 内容・RT スレッド競合）は **NotRun**。idle CPU は外部サンプラの 15 秒窓 1 回の値であり、長期・低負荷でのドリフトは未測定。per-instance スループット上限 ~190/s（lookahead=2）は 512 frame @48kHz の ~2 倍のマージンがあり、lookahead を下げる設計変更が入った場合は再測定要。
