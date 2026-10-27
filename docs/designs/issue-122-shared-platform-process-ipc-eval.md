---
id: "DESIGN-034N3EYKKosV8OzCP7Y9tY"
type: "design"
status: "approved"
title: "共通 platform/process/ipc 基盤の抽出評価（Issue #122）"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/122"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/101"
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
    - "shared-foundations-and-modularization-policy.md"
    - "plugin-l1-out-of-process.md"
    - "ruviform-architecture-design-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "plugin-isolation.md"
    - "issue-114-real-adapter-l1-e2e.md"
    - "../adr/select-audio-device-backend.md"
    - "../adr/plugin-isolation-strategy.md"
    - "../adr/plugin-architecture-clap-vst3.md"
    - "../adr/rt-foundation-primitives-implementation.md"
  refines: []
  refined-by: []
  derives-from:
    - "shared-foundations-and-modularization-policy.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/117"
    - "https://github.com/scarecrow173/ruviform/issues/118"
    - "https://github.com/scarecrow173/ruviform/issues/120"
    - "https://github.com/scarecrow173/ruviform/issues/123"
  defers: []
  deferred-by: []
---

# 共通 platform/process/ipc 基盤の抽出評価（Issue #122）

## Context

Issue #122 — [共通基盤・Observability・クレート分割の設計方針](shared-foundations-and-modularization-policy.md)（Issue #101・approved）は「共通 `platform` / `process` / `ipc` の独立」を**要 PoC** とし、「Issue #76 L1 の着手と 2 番目の OS サービス利用者の出現」を再評価トリガとして延期していた（同書「状態一覧」および「後続 Issue と再着手条件」）。Issue #76（[プラグイン L1 アウトプロセス分離](plugin-l1-out-of-process.md)）が完了し、host 側 `l1::shm` + export 側 `shm` という同一 wire プロトコルの 2 実装、および child spawn/reap・stdio pipe framing がワークスペースに実在した現在、トリガは名目上充足した。本書はその充足を受けた**抽出要否の評価結果を記録する**。

さらに Issue #114（[実規格アダプタ L1 E2E](issue-114-real-adapter-l1-e2e.md)）が `libloading`/`clack-host` による dlopen 経路を adapter `host` feature と runner bin に追加しており、process/dlopen プリミティブの post-#76 利用者として併せて評価対象に含める。

上流確定事項（再決定しない）: `shared_memory` 0.12 採用と `l1::shm`/export `shm` モジュールへの内部封じ込め（採用条件 — 公開 API・feature 名に露出しない）・wire/shm レイアウトと codec の正本は `ruviform-plugin-core::wire`・L1 スレッドモデル（per-instance child + driver/reader）・`Failed` 収束意味・方針書の分割判断の軸（行数ではなく独立利用/依存・可搬性/契約/開発・互換性/安全性/ビルド）・「OS のコードだから platform に集約しない」原則（方針書「依存方向と既存契約との整合」）。

本書は**評価と判断の記録**であり、新クレートの新設・コード移動・公開 API 変更は行わない。抽出が将来必要になった場合の実装は後続 Issue に委ねる。

## 評価スコープと方法

Issue の指示に従い、platform/process/ipc プリミティブの**実在する重複**をコード棚卸し（`f33ea31` 時点）し、方針書の分割判断の軸と依存 DAG 帰結に照合する。対象は `shared_memory` ラップ・spawn/reap・pipe framing・wait/timeout・dlopen の 5 系統。`raw_sync` は #76 で不採用確定（同期はパイプ doorbell で代替）のため対象外。

## 棚卸し: 現行の platform/process/ipc プリミティブと重複実況

### named shared memory（`shared_memory` ラップ）

| 箇所 | 役割 | 範囲 |
| --- | --- | --- |
| `crates/ruviform-plugin-host/src/l1/shm.rs`（362 行） | **所有者側** — セグメント生成・キー一意化・ヘッダ書込・drop 時 unlink・I/O スレッド用ローカル attach | `process_salt` L74-81・`unique_key` L97-104・`Segment::create`/`create_keyed`/`create_named` L135-178・`key`/`declared_len`/`verify_header` L182-215・`write_header` L220-250・`View::attach`/`in_slot_mut`/`out_slot` L280-306・`bytes`/`bytes_mut`（scoped `unsafe` ×2）L310-338・`get_u32`/`get_u64`/`put_u32`/`put_u64` L340-362 |
| `crates/ruviform-plugin-export/src/shm.rs`（265 行） | **接続側** — 名前付きセグメント attach・ヘッダ検証・スロット parse/write（create/unlink は持たない） | `Segment::attach`+ヘッダ検証 L85-130・`in_slot` L136-171・`write_out_slot` L178-210・`bytes`/`bytes_mut`（scoped `unsafe` ×2）L214-241・`get_u32`/`get_u64`/`put_u32`/`put_u64` L243-265 |
| `crates/ruviform-plugin-core/src/wire.rs` `pub mod shm` L131-222 | **共有契約の正本（既に集約済み）** — `MAGIC`・`HEADER_LEN`・ヘッダ/スロット offset・`SLOT_BYTES`・`SAMPLE_STRIDE`・`out_slots_off`・`segment_len` | wire 層の一部として両側が共通参照 |
| `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`（テスト fixture・runner 役割ミラー） | export `shm` の接続側を模倣 — `Segment::attach`+全ヘッダ検証 L255-304・`process_block`（in-slot parse・out-slot write・degenerate 次元処理）L306-371・`bytes`/`bytes_mut`（scoped `unsafe` ×2）L374-398・`get_u32`/`get_u64`/`put_u32`/`put_u64` L400-422 | 全 offset は `wire::shm` consts 経由（レイアウトは複写せず契約を参照）。`shared_memory` 直接利用（L42 import・L256 attach） |
| `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs`（E2E テスト・owner 役割ミラー `MiniHost`） | host `l1::shm` の所有者側を模倣 — `unique_key` L207-210・`create_segment_with`（`ShmemConf::create`+ヘッダ書込）L212-243・`write_in_slot` L249-298・`read_out_slot` L300-339・`segment_bytes`/`segment_bytes_mut`（scoped `unsafe` ×2）L341-353・`get_u32`/`get_u64`/`put_u32`/`put_u64` L354-368 | 同様に `wire::shm` consts 経由。`shared_memory` 直接利用（L30 import・L217 create）。`MiniHost` 本体 L60-185 は process 節を参照 |

#76 レビューが確認した「byte-identical layout」は **`wire::shm` に単一正本として既に集約済み**であり、レイアウト自体は重複していない。残る実重複は薄い OS 接地面の糊のみ:

- `ShmemConf` によるセグメント create/attach — host `shm.rs`（`create` 系 L135-178・ローカル `open` :281-285）/ export `shm.rs`（`open` :86-89）に加え、テスト側でも `l1-test-worker.rs:256`（`open`）・`l1_runner_e2e.rs:217`（`create`）が同一 API を直接呼ぶ（~5 行/箇所）
- `bytes`/`bytes_mut` の scoped `unsafe` slice accessor — **計 8 サイト / 4 ファイル**: プロダクション 4 サイト（host `shm.rs` `#[expect(unsafe_code)]` L320-323・L334-337、export `shm.rs` 同 L224-227・L237-240）+ テストミラー 4 サイト（`l1-test-worker.rs` L382-385・L394-397、`l1_runner_e2e.rs` `segment_bytes`/`segment_bytes_mut` L341-344・L349-352）。形状は同型だが SAFETY 論証は側ごとに異なる（所有者書込み済みヘッダ+strict alternation vs attach 検証済み+単一スレッド runner — テスト側は各役割の論証を引き継ぐ）
- `get_u32`/`get_u64`/`put_u32`/`put_u64` — 同一実装の 4 ヘルパが **4 箇所**（host `shm.rs:340-362` / export `shm.rs:243-265` / `l1-test-worker.rs:400-422` / `l1_runner_e2e.rs:354-368`、~23 行/箇所）
- ヘッダ検証・スロット parse/write 手続き — export `shm.rs`（attach 時の全ヘッダ検証・`in_slot`/`write_out_slot`）を `l1-test-worker.rs` `Segment`/`process_block`（L243-422）が、host `shm.rs`（`write_header`・`in_slot_mut`/`out_slot`）を `l1_runner_e2e.rs` `create_segment_with`/`write_in_slot`/`read_out_slot`（L212-339）が、同一 `wire::shm` 契約に対して手続きレベルで複写

役割分割であって重複ではない部分: create/key 一意化/unlink/ヘッダ書込は host のみ、ヘッダ検証/slot parse/write は export のみ。両側は「2 つの独立利用者」ではなく **1 つの L1 wire プロトコルの 2 半**である。テスト側の 2 ミラーはこの役割分割をそのまま検証用に複写したもの（`l1-test-worker` が runner 役割、`MiniHost` が owner 役割）で、新たな独立利用者ではない — ただし手続きレベルの重複量としては正直に計上する（後述「実重複量」）。

### process spawn / reap / kill / wait

| 箇所 | 内容 |
| --- | --- |
| `crates/ruviform-plugin-host/src/l1/mod.rs` | `ChildExit::from_status`（unix `ExitStatusExt::signal` 含む）L124-133・`kill_child` L338-342・`converge` L376-427・`spawn_child` L464-472・`try_wait_child` L588-593・`reap` L604-624・`driver_main`/`reader_main` L637-784/L490-583・`spawn_driver`/`spawn_reader` L803-932・`L1Instance::spawn` L990-・`abort` L1762・`join_threads` L1784 |
| `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` | テスト fixture の heir spawn（`--leave-heir-after-msgs` — 検証用途のみ） |
| `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` | `MiniHost` L60-185 — `spawn`/`spawn_with_env`（`Command::spawn`+stdin/stdout 取得+reader スレッド demux）L69-112・`wait_exit` L172-184（`try_wait` poll + `Instant` デッドライン + `kill` — **2 つ目の reap ループ**） |

ワークスペース内で `std::process::Command`/`Child` を扱うプロダクションコードは `l1/mod.rs` の 1 系統のみ（export 側は被 spawn 側で spawn/reap を持たない）。ただしテスト側には `MiniHost`（`l1_runner_e2e.rs` L60-185）が存在し、その `wait_exit` は `try_wait` poll + デッドライン + `kill` という **2 つ目の reap ループ**である（`l1-test-worker.rs` の heir spawn も fixture 用途で `Command` を使う）。spawn/reap/kill は `Shared`/`converge`/waiter 起床/通知という **L1 固有の冪等収束機構と深く癒着**しており、汎用化可能な残滓は `Command::spawn` + `stdin/stdout` 取得 + `try_wait`/`wait`/`kill` の薄い呼出し（`MiniHost` 側も同形、実質数十行）に留まる。

### pipe framing（stdio 上の長さプレフィクスフレーム I/O）

| 箇所 | 内容 |
| --- | --- |
| `crates/ruviform-plugin-host/src/l1/mod.rs` | `read_message` L433-449・`write_message` L452-457 — `[u32 LE payload_len][u8 kind][payload]`・`WIRE_MAX_PAYLOAD` 上限・`io::Result` |
| `crates/ruviform-plugin-export/src/lib.rs` | `read_frame` L741-761 + `fill` L771-788 + `send` L792-797 — 同フレーム形・`ReadFail`（Truncated/LimitExceeded/Io）分類とフレーム境界 clean EOF の区別 |
| テスト側コピー | `src/bin/l1-test-worker.rs` L140-163（`read_message`/`write_message` — 第 3 コピー）・`export/tests/l1_runner_e2e.rs` の最小 host 実装 `MiniHost`（`read_message` L187-204 + `send` の inline frame 書込 L122-129 — **第 4 コピー**） |
| （参考: 複写でない再利用） | `host/tests/l1_proxy_e2e.rs` は最小 host を持たず、`#[path]` で実体の `l1` モジュールを取り込む（L16-18）— ソース再利用でありフレーム I/O の別実装ではない |

codec（`wire::encode`/`decode`・`WIRE_FRAME_HEADER_LEN`・`WIRE_MAX_PAYLOAD`）は既に plugin-core 集約済み。重複しているのはストリーム上の読み書き糊 ~50-60 行/側であり、エラー分類は各側の契約（`io::Result` vs `ReadFail`+EOF 区別 vs テスト側の `Result<_, String>`/`expect`）で意図的に異なる。「×4」の内訳は `l1/mod.rs`・export `lib.rs`・`l1-test-worker.rs`・`l1_runner_e2e.rs` の 4 実装（`l1_proxy_e2e.rs` は `#[path]` 取り込みで別実装を持たない）。**この dedup は既に Issue #123 が「frame-I/O dedup ×4」として追跡**しており、予定されている着地は `wire` 層共通 helper（plugin-core）または test-only dedup — 新 foundation クレートではない。

### wait / timeout

`DRIVER_POLL_INTERVAL`（mod.rs:93）・`DRIVER_JOIN_POLL`（:97）・`DRIVER_JOIN_MARGIN`（:102）・`recv_timeout` による有界待機（`spawn_rx` :1046・`hello_rx` :1107・`reply_rx` :1283・driver poll :648）・`reap`/`join_threads` の `Instant` デッドライン（:604-624・:1784-1797）。全て `std` の `mpsc::recv_timeout`/`Instant`/`thread::sleep` の直接使用であり、プロダクションでは共有抽象の対象となる重複は存在しない。ただしテスト側 `MiniHost::wait_exit`（`l1_runner_e2e.rs` L172-184）が `try_wait` poll + `Instant` デッドライン + `kill` という **2 つ目の wait/reap ループ**（~13 行・テスト限定）を持つ — `std` プリミティブの直接使用で共有抽象を要する規模ではないが、「wait/timeout の重複ゼロ」はプロダクション範囲の記述として読むべきである。

### dlopen / dynamic loading（#114 追加分を含む）

| 箇所 | 内容 |
| --- | --- |
| `crates/ruviform-plugin-ofx/src/ffi/loader.rs:103` | `libloading::Library::new`（#114 以前から存在・scoped `unsafe` 封じ込め） |
| `crates/ruviform-plugin-vst3/src/host.rs:696` | `libloading::Library::new`（feature `host`・#114） |
| `crates/ruviform-plugin-clap/src/host.rs` | `clack-host` の `PluginEntry::load` 経由の dlopen（`clack-host` が内部で libloading をラップ。#114） |
| `crates/ruviform-plugin-{clap,vst3}/src/bin/*-l1-runner.rs` | 各 bridge の `*Library::load` を呼ぶのみ — 新プリミティブではなく bridge API の消費者 |

`libloading`/`clack-host` **自体が既に共有実装**であり、3 サイトとも規格固有の ABI 知識（`GetPluginFactory`/`clap_entry`/COM 駆動）は adapter 責務として scoped `unsafe` + 内部モジュールに封じ込め済み（cpal → `ruviform-audio-device`、libloading → OFX loader の adapter confinement 先例）。独自ラッパを設けると「外部共有抽象の上の独自抽象」となり、方針書の「外部ライブラリを再利用し、独自抽象は必要な保証に限定する」に反する。

### `ruviform-rt` / `ruviform-exec` の現状 OS 抽象

- `ruviform-rt`: `channel`（rtrb）・`retire_channel`・`slot`（arc-swap）・`alloc_counter`（`#[cfg(test)]` 限定）・`MasterClock`・`RtContext`・`Processor`/`RtProcessor` — 全てプロセス内プリミティブ。プロセス/IPC/共有メモリの抽象は存在せず、方針書の棚卸し（L246-278）から増分なし。
- `ruviform-exec`: `thread_priority`（`audio_thread_priority` の薄ラップ — Windows MMCSS/macOS time-constraint/Linux `SCHED_FIFO`）+ `OfflineDriver` — 単一利用者の OS サービス。方針書「当面維持」のまま。

### 消費者カウント（正直な評価）

OS サービス利用「箇所」は増えたが、**独立した利用「ケース」は依然 1 系統**: L1 wire（shm+spawn+pipe）は host/export に分かれた同一プロトコルの両半であり、wire 契約は既に plugin-core で共有済み。テスト側の `l1-test-worker`（runner 役割ミラー）と `MiniHost`（owner 役割ミラー）はこの「1 プロトコル・2 役割」を検証用に各々複写するものであり、利用ケースを増やすものではない — ただし複写の存在は実重複量に正直に計上する。dlopen は 3 サイトあるが外部クレートが共有抽象を担う。thread-priority は単一利用者のまま。「2 番目の OS サービス利用者」の出現というトリガ文はコード上の 2 実装を指したが、意味のある重複（同一責務を担う知識の分散）としては、レイアウトは正本化済み・残る糊は役割非対称かつ薄い（テストミラーを含めても責務は同一プロトコルの検証に閉じる）。

## 決定サマリ

| # | 論点 | 決定 |
| --- | --- | --- |
| a | 共通 `platform`/`process`/`ipc` クレートの新設 | **延期継続（抽出しない・keep local）** — 方針書の分割判断の軸を現状満たさず、再トリガ条件を具体化して保留に戻す |
| b | frame-I/O dedup | **新クレートでは解決しない** — Issue #123 の既定方針どおり plugin-core `wire` 層の共通 helper（または test-only dedup）に委ねる。`wire` は host/export 双方が既に依存する正本位置 |
| c | dlopen 抽象 | **共通化しない** — `libloading`/`clack-host` が既に共有実装。規格固有 ABI 知識は adapter 責務（OFX/cpal 先例） |
| d | 暫定封じ込め | **現行維持** — `shared_memory` は公開 API・feature 名に露出しない（採用条件の **public-API 封じ込めは成立**。直接利用はプロダクション 2 モジュール `l1::shm`/export `shm` + テスト 2 ターゲット `l1-test-worker.rs`/`l1_runner_e2e.rs` の計 4 ファイル）・`unsafe` は scoped サイト計 8 箇所（プロダクション 4 + テストミラー 4）に限定・spawn/reap/wait は `l1/` モジュール + `MiniHost` テストミラー内 |

## 判定根拠

### 方針書「クレート分割の判断基準」への照合

| 分割判断の軸 | 評価 |
| --- | --- |
| 独立利用 | **不成立** — 抽出候補の API 単体では利用者非依存の意味を持たない。shm 両半は L1 役割非対称、spawn/reap は `converge`/waiter 意味に癒着 |
| 依存・可搬性 | **既達成** — OS/`unsafe`/外部依存の分離は封じ込めで済んでいる（`shared_memory` の直接利用はプロダクション 2 モジュール + テスト 2 ターゲット内に留まり、libloading/clack-host は adapter feature 内） |
| 契約 | **抽出不能な部分が核心** — `bytes`/`bytes_mut` の SAFETY 不変条件は strict-alternation プロトコル依存であり、汎用クレートが所有できない。呼出し側がプロトコル知識を供給する限り、汎用側が所有するのは `Shmem` の薄い所有権のみ |
| 開発・互換性 | **時期尚早** — #117（bundle 子: 1 子 N インスタンス）・#118（video/NONRT データ面: shm レイアウト見直し）・#120（共有リーパー/io 多重化: `try_wait` poll → poll/epoll/IOCP イベント駆動）がまさに抽出候補面を再設計する。今 API 化すると再設計前の形を固定する |
| 安全性 | **現状で局所化済み** — 監査対象の `unsafe` は計 8 箇所の `#[expect(unsafe_code)]` scoped サイトに限定済み（プロダクション: `l1/shm.rs`・export `shm.rs` 各 2 箇所 / テストミラー: `l1-test-worker.rs`・`l1_runner_e2e.rs` 各 2 箇所）。抽出しても SAFETY 論証自体は利用側に残るため監査範囲は実質縮小しない |
| ビルド | **根拠なし** — 実測なし |

「分割しない条件」（方針書 L307）のうち「実装詳細のためだけに `pub` を増やす」「依存 DAG が複雑化するだけ」「利用者も保証契約も未定」に該当する。

### 実重複量

正本化済み（`wire::shm`・codec）を除く残存重複は、プロダクション 2 側で attach/create 糊 ~5 行/側・`unsafe` accessor 同型 ~30 行/側・LE get/put ~23 行/側・frame I/O ~50-60 行/側の **~100-200 行の糊**。テスト側を含めた全量では `unsafe` accessor 計 8 サイト・LE get/put 計 4 コピー・frame I/O 計 4 実装に加え、`l1-test-worker` の `Segment`/`process_block` ミラー（L243-422）と `MiniHost` ミラー（`l1_runner_e2e.rs` L60-368）という検証目的の手続き複写が存在し、数百行規模になる。ただしミラー分は同一 wire 契約を両役割から検証する意図的複写であり、抽出可能な新しい共有知識ではない。糊の大半は `shared_memory`/`std` の再ラップ（wrapper over wrapper）になる。行数は補助指標だが、契約・所有権の違い（上表）が伴わないため分割理由にならない。

### 依存 DAG 帰結

- 新クレートを Primitives/foundations に置くと、host/export からの下向き依存は向きとして合法。ただし foundations は plugin 概念を所有できない（方針書: `ruviform-platform`/`process`/`ipc` 候補は「プラグイン固有プロトコル」を所有しない）→ `wire::shm` レイアウト・ヘッダ schema・`seq % D`・`Failed` 収束・doorbell・argv 規約は全て plugin 層残留となり、移せるのは前述の薄い糊のみ。
- frame I/O を foundations に置く選択肢もあるが、**正しい着地は plugin-core `wire`**（codec と同じ正本位置・host/export 双方が既に依存・`Read`/`Write` 上の薄い helper）— #123 が既にその方針で追跡中。wire 隣接知識を foundations に下ろすと plugin 意味を foundations 側へ漏らすか、無味な byte helper になるかの二者択一になる。
- `ruviform-rt → foundation` 系の非既定依存は本評価では発生しない（rt のプロセス内プリミティブは対象外）。

### premature 性（#117/#118/#120 との関係）

抽出候補となる spawn/reap/wait 面は #120 が poll/epoll/IOCP への置換を予定する、最も再設計される予定の領域である。#117 は spawn モデル自体（1 子 = 1 インスタンス前提）を変え得る。#118 は shm レイアウトの第 2 形状を導入し得る — **真に汎化可能な知識はこれらの着地後に初めて輪郭が見える**。現時点での抽出は、変わることが確定している API 面の早期固定であり、extract-then-redesign は redesign-then-evaluate より厳しくコストがかかる。

## 将来抽出する場合の境界スケッチ（非確定 — 再トリガ時の参照用）

方針書の延期項目「L1 の汎用プロセス基盤とプラグイン固有境界の切り分け（Issue #76 着手時に解決する）」への回答として、切り分けのみ記録する（クレート新設はしない）。

- **汎用候補（抽出可能な残滓）**: 名前付きセグメント create/open/unlink + キー retry seam + mapping 長カバレッジ検査 + scoped `unsafe` slice accessor（SAFETY 論証は消費者がプロトコル毎に供給）・`Child` spawn/kill/`try_wait`/reap + `ExitStatus` の `(code, signal)` 正規化・`[u32 LE len][payload]` frame reader/writer over `Read`/`Write`。
- **plugin 固有（plugin 層に残留）**: `wire::shm` レイアウト/ヘッダ/スロット形状・`seq % D` スロット選択・strict-alternation の SAFETY 論証・`Failed` converge/waiter/notification 意味・doorbell `BlockIn`/`BlockDone`/`BLOCK_SEQ_FLAG`・argv 規約・wire codec 全般。
- **配置候補**: `ruviform-platform` または `ruviform-process`/`ruviform-ipc`（方針書の構成候補名）。拡張層より下に置き、plugin 概念の import を禁止し、host/export が下向きに依存する。adapter→adapter 依存は作らない（dlopen は adapter 各所のまま）。
- **発動時の移行順序（参考）**: 現行コードは全て `pub(crate)` のため公開 API 互換の問題は生じない。(1) byte accessor + get/put helper → `platform::shm::access`（呼出し側が SAFETY 論証を維持）、(2) Segment create/attach → `platform::shm`、(3) spawn/reap/kill → `platform::process`、(4) frame I/O → `platform::ipc::framed`（#123 の wire 層 dedup が不十分と判明した場合のみ）、(5) 検証は `mise run check`/`lint`/`test`/`test-rt` + 3 OS `verify` CI。

## 再トリガ条件（pinned）

方針書の「2 番目の OS サービス利用者」トリガを、本評価を経て以下に具体化する。**いずれか 1 つ**の充足で再評価する:

1. L1 wire プロトコル外の**独立した** OS サービス利用者が出現する（例: #117 bundle の汎用 worker pool 管理、crash-report/supervisor プロセス、sandboxed decoder、GPU process 分離）。
2. #120 の event-driven reaper/io 多重化が実装された上で、spawn/reap/wait 系コードが依然 ≥2 クレートに重複している。
3. #118 等で**第 2 の shm レイアウト**が導入され、そのレイアウト用の `unsafe` slice accessor + validation パターンが **第 2 レイアウト側の 3 箇所目**に出現する（現行レイアウトの accessor/validation ミラーは既に計 4 ファイル — プロダクション 2 + テスト 2 — に存在するため、既存レイアウトの複写増加ではトリガしない）。
4. #123 の wire 層 dedup が不十分と判明し、`platform::ipc::framed` のような共置が必要になる。

それまでは現行の封じ込めを維持する: `shared_memory` の公開 API・feature 名への露出禁止（直接利用はプロダクション 2 モジュール + テスト 2 ターゲットの現状のまま）・wire 由来の dedup は plugin-core `wire` 経由（#123）・dlopen は adapter 内 scoped `unsafe` に限定。

## 既存設計・ADR との整合

- [共通基盤方針書](shared-foundations-and-modularization-policy.md)「状態一覧」の platform/process/ipc 行（要 PoC・トリガ #76 着手 + 2 番目利用者）— 本書はトリガ充足後の評価を記録し、結論を「延期継続 + 再トリガ具体化」とする。方針書の方針・構成候補・その他の保留区分は変更しない（同書の `derived-by` に本書を追加するのみ）。
- [プラグイン L1 設計](plugin-l1-out-of-process.md) 決定 d（`shared_memory` 採用・内部隔離条件）・決定 e（制御=stdio パイプ）と整合 — 本評価はその隔離を維持しつつ抽出不要と判断する。
- [audio-device backend ADR](../adr/select-audio-device-backend.md)・OFX `ffi/loader.rs` の adapter confinement 先例と整合 — 外部 OS 依存は adapter/feature 内部に閉じ込める方式を維持。
- 本書はコード・`Cargo.toml`・`members`・baseline の層構造を変更しない。baseline・accepted ADR の改訂は不要。

## Deferred Design Concerns

- `ruviform-platform`/`ruviform-process`/`ruviform-ipc` の新設 | 上記判定根拠（独立利用なし・契約所有不可・premature） | 再トリガ条件 1-4 のいずれか充足 | 放置リスクは小さい — 現行の重複は ~100-200 行の糊で、誤進化しても局所的
- frame-I/O dedup ×4 | Issue #123 に権限委譲済み | #123 実施時 | 現状でも codec 正本は共有済みで、糊の非対称はエラー分類の意図的差分
- dlopen 共通化 | 外部共有抽象（`libloading`/`clack-host`）で十分 | libloading 非互換の OS ローダが必要になった場合 | adapter 各所の小さな scoped `unsafe` が残る（現行も同等）

## Verification Notes

- [x] 棚卸し対象 5 系統（shm ラップ・spawn/reap・pipe framing・wait/timeout・dlopen）を `f33ea31` 時点のコードで file:line 特定済み
- [x] 方針書トリガ（#76 着手 + 2 番目利用者）の充足を確認し、評価を実施・記録
- [x] #114 で追加された dlopen 利用者（adapter `host` feature + runner bins）を評価対象に含めた
- [x] #117/#118/#120 の OPEN 状態と各スコープを確認し、premature 判定の根拠に使用
- [x] frame-I/O dedup の既存追跡（#123）を確認し、新クレートで解決しない方針と整合させた
- [x] レビュー指摘の census 訂正（`unsafe` サイト計数 8/4 ファイル・`shared_memory` 直接利用範囲・テスト側ミラー 2 件の計上・`l1_proxy_e2e` 帰属訂正・`wire::shm` 範囲・再トリガ条件 3 の明確化）を反映し、status を `draft` → `approved` へ

## Sources

- [共通基盤・Observability・クレート分割の設計方針](shared-foundations-and-modularization-policy.md)（Issue #101・approved — トリガ句と分割判断の軸の正本）
- [プラグイン L1 アウトプロセス分離設計](plugin-l1-out-of-process.md)（Issue #76・`shared_memory` 採用条件とスレッドモデルの正本）
- [実規格アダプタ L1 E2E 設計](issue-114-real-adapter-l1-e2e.md)（Issue #114 — dlopen 利用者の追加分）
- [プラグイン分離設計](plugin-isolation.md)・[プラグイン分離戦略 ADR](../adr/plugin-isolation-strategy.md)
- [Ruviform architecture design baseline](ruviform-architecture-design-baseline.md)（層構造・依存方向の正本）
- [audio-device backend 選定 ADR](../adr/select-audio-device-backend.md)（adapter confinement 先例）
- コード棚卸し対象: `crates/ruviform-plugin-host/src/l1/{mod.rs,shm.rs,data.rs}`、`crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`、`crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`、`crates/ruviform-plugin-export/src/{lib.rs,shm.rs}`、`crates/ruviform-plugin-export/tests/l1_runner_e2e.rs`、`crates/ruviform-plugin-core/src/wire.rs`、`crates/ruviform-plugin-ofx/src/ffi/loader.rs`、`crates/ruviform-plugin-{clap,vst3}/src/host.rs`、`crates/ruviform-{rt,exec}/src/`
- Issue [#122](https://github.com/scarecrow173/ruviform/issues/122)、[#76](https://github.com/scarecrow173/ruviform/issues/76)、[#101](https://github.com/scarecrow173/ruviform/issues/101)、[#114](https://github.com/scarecrow173/ruviform/issues/114)、[#117](https://github.com/scarecrow173/ruviform/issues/117)、[#118](https://github.com/scarecrow173/ruviform/issues/118)、[#120](https://github.com/scarecrow173/ruviform/issues/120)、[#123](https://github.com/scarecrow173/ruviform/issues/123)
