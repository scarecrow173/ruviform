---
id: "DESIGN-034Qa7Kf2NpXw9RtYbHc5d"
type: "design"
status: "approved"
title: "L1 bundle 子プロセス（1 子プロセス N インスタンス）設計（Issue #117）"
created: "2026-10-28"
updated: "2026-10-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/117"
    - "https://github.com/scarecrow173/ruviform/issues/76"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-117-bundle-child-process.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/issue-122-shared-platform-process-ipc-eval.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-117-bundle-child-process.md"
    - "docs/discovery/issue-117-bundle-child-process-discovery.md"
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  derived-by:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/tasks/l1-wire-v2-codec-cutover.md"
    - "docs/tasks/l1-child-instance-split.md"
    - "docs/tasks/l1-route-pool-and-isolation-surface.md"
    - "docs/tasks/l1-bundle-e2e-and-granularity-verification.md"
    - "docs/tasks/l1-wire-v2-docs-sweep.md"
    - "docs/tasks/l1-bundle-child-final-verification.md"
    - "docs/test-specs/issue-117-wire-v2-codec-tests.md"
    - "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
    - "docs/test-specs/issue-117-bundle-e2e-tests.md"
    - "docs/discovery/issue-116-supervisor.md"
    - "docs/specs/issue-116-supervisor.md"
    - "docs/designs/issue-116-supervisor.md"
    - "docs/discovery/issue-118-video-l1.md"
    - "docs/specs/issue-118-video-l1.md"
    - "docs/designs/issue-118-video-l1.md"
    - "docs/adr/issue-118-video-l1-host-surface.md"
    - "docs/adr/issue-118-video-l1-create-path-segment.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-117-wire-v2-codec-tests.md"
    - "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
    - "docs/test-specs/issue-117-bundle-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/115"
    - "https://github.com/scarecrow173/ruviform/issues/118"
    - "https://github.com/scarecrow173/ruviform/issues/119"
    - "https://github.com/scarecrow173/ruviform/issues/120"
    - "https://github.com/scarecrow173/ruviform/issues/122"
    - "https://github.com/scarecrow173/ruviform/issues/123"
  defers: []
  deferred-by:
    - "docs/designs/plugin-l1-out-of-process.md"
---

# L1 bundle 子プロセス（1 子プロセス N インスタンス）設計（Issue #117）

## Context

Issue #117 — 現行 L1 は **1 インスタンス = 1 子プロセス**（per-`Instantiate` spawn）を前提とする（`l1/mod.rs` `L1Instance::spawn` L976-1088・export `lib.rs` L14-15・`composite.rs` `Route::OutOfProcess` L160-167）。メモリ/spawn コスト削減のため、**1 子プロセスが N 個のプラグインインスタンスを host する bundle 形態**を L1 wire プロトコル・host プロキシ・runner・`PluginHost` 公開面に導入する。

bundle 化は親設計が当初から予告したプロトコル改訂である: `docs/designs/plugin-l1-out-of-process.md` L176「handle は子が singleton のため落とす — 将来の bundle 子は `Instantiate`/handle 修飾を wire に追加するプロトコル改訂」・同 L296 deferred 行「bundle 子（1 子 N インスタンス）」・`wire.rs` L303-308「no variant carries a `PluginHandle` — a runner child is a singleton instance」。Issue #119（implemented）が「単一最新バージョン・交渉機構なし・非互換変更は `WIRE_VERSION` bump」を確定済みであり、本書の全 wire 変更は **`WIRE_VERSION` 1→2 bump** として扱う（順序付けの懸念は解消済み）。

確定の入力となる現行実装の事実（discovery `docs/discovery/issue-117-bundle-child-process-discovery.md` の観測表に一致）:

- **wire**: `Message` は `Hello`/`Command{seq,cmd}`/`Reply{seq,result}`/`BlockIn{seq}`/`BlockDone{seq,result}`/`Shutdown` の 6 kind — **いずれもインスタンス識別子を持たない**（`wire.rs` L259-301）。`Command` 10 動詞（`Scan`/`Initialize`/`Activate`/`StartProcessing`/`StopProcessing`/`Deactivate`/`SaveState`/`RestoreState`/`OpenGui`/`CloseGui`、tag 0-9）も handle を持たない。`WIRE_VERSION: u16 = 1`（L88）。`WIRE_MAX_*` は全てメッセージ単位の上限で、子あたり・インスタンスあたりの語彙は存在しない
- **seq 分割**: コマンド seq（`L1Instance::next_seq`・`l1/mod.rs` L901/L1247）と block seq（`RtLeg::next_block`・`data.rs` L430）が `u64` 空間を共有し `BLOCK_SEQ_FLAG = 1<<63`（`data.rs` L83）で分割。`Inner.waiters: HashMap<u64, Waiter>`（L214）と `Inner.block_in_flight: HashSet<u64>`（L226）は seq のみをキーとし、demux が「子内で seq 一意」に依存している（reader demux: `l1/mod.rs` L489-513）。BlockIn 拒否は **flagged `Reply{seq,Err}`** — `data.rs`/`export/lib.rs` L234-260 が pin する契約
- **host**: `Shared`/`Inner`/`L1Instance` が子ハンドル・stdin/stdout・I/O スレッド・waiters・mirror・staging・notify・RT leg を 1 構造体に同居させる一体型。`converge`（L375-426）は「子の terminal 確定・kill/reap」と「インスタンスの mirror/waiter/notify 収束」を一体で行う。1 子 = driver + reader の 2 スレッド（モジュール文書 L28-37 のスレッド表）
- **runner**: `run(factories)` は argv `--ruviform-shm-key`/`--ruviform-plugin-id` で **1 segment attach + 1 factory.create()** の singleton（`export/lib.rs` L124-188・`Runner` L387-400・`RunnerArgs` L311-381）。`serve` の dispatch に instance 解決段はないが、`run(vec![..])` で複数 factory を内蔵する形は既に存在する
- **公開面**: `OutOfProcessSpec`（`composite.rs` L68-87）は `descriptor`/`program`/`args`/`lookahead_blocks`/`command_timeout` のみ — bundle/pool/isolation の語彙なし。`HostNotification::InstanceFailed{handle, exit}`（L146-156）は per-instance の通知語彙で `try_recv_notification`（L771-796）が slot 走査で fan-out する。`PluginHandle{slot,generation}` は公開 opaque handle（`boundary.rs` L25-59）
- **GUI**: `gui_open` field は Issue #123 で除去済み（`crates/` 内 0 件 — grep で再確認）。wire 語彙 `OpenGui`/`CloseGui`/`Reply::Opened` は存置だが L1 host は `execute` が `Err(HostError::Unsupported)` で host-side short-circuit（wire 非送信 — #115 設計・#114 E2E pin 済み）
- **RT/SHM**: segment は per-`L1Instance`（`spawn` で 1 本 create・`l1/shm.rs`）— header・slot レイアウト・strict alternation・fence 規則は `wire::shm` が正本。`prepare_endpoint`（`l1/mod.rs` L1469-）は per-instance `RtLeg` の detach/return を既に実装しており、「RT leg はインスタンス毎」の構造が実証済み

上流確定事項（再決定しない）: RT 契約（RT スレッドは pipe/shm/mutex を触らない）・`Failed`+通知+pass-through・自動再起動なし・`#[non_exhaustive]` 公開 enum 方針・`PluginHost` の `!Send`/`!Sync` 所有権・公開 `PluginHandle`/`HostCommand`/`HostReply`/`HostError` 面の不変・`ruviform-audio` が plugin 系クレートに依存しない層方向・unsafe は局所 `#[expect(unsafe_code)]` + SAFETY コメントのみ。

## 決定サマリ（spec OPEN 論点 → 本書の結論）

| # | 論点 | 決定 |
| --- | --- | --- |
| 1 | `Failed` 粒度 | **子スコープ terminal 基礎 + 帰属可能な per-instance 収束の限定併用** — 子死亡・EOF・帰属不能なプロトコル違反は bundle 内全 live インスタンスを `Failed` へ収束。well-formed で `(instance, block-seq)` 帰属が確立する違反のみ対象インスタンス単独を `Failed` にできる（下記軸 1 の表）。registered-but-not-live 宛 frame（`Failed` 未破棄・`Destroy`ing）・post-removal 完了・post-`Shutdown` frame は **inert — 違反でなく静かに drop**。ADR `docs/adr/issue-117-bundle-child-failed-granularity.md` が公開契約として記録 |
| 2 | bundle グルーピング | **登録 route（`plugin_id`/`OutOfProcessSpec`）単位の pool** — route 内部状態として `pool: Vec<Arc<L1Child>>`。`PluginHost` 大域 pool は却下（異 program は同居不可能）。明示 pool キーによる route 跨ぎ共有は deferred |
| 3 | wire handle | **`u32`・host mint** — `to_wire` 層で公開 `PluginHandle` と写像・子ローカル名空間・generation 不要。値域 `[0, WIRE_MAX_INSTANCES)` + sentinel `u32::MAX` |
| 4 | handle の置き場所 | **envelope 化** — `Command`/`Reply`/`BlockIn`/`BlockDone` の全ペイロードで `seq` 直後に `instance: u32`。`Reply` も instance echo を持つ（flagged `Reply` による BlockIn 拒否 demux が (instance, seq) を要求するため — 軸 4/5） |
| 5 | seq/correlation | **コマンド seq = 子ワイド単一空間・block seq = per-instance**（`RtLeg::next_block` 維持・`BLOCK_SEQ_FLAG` 分割維持）。`waiters` は seq キー + instance 記録、`block_in_flight` は `(u32, u64)` キー |
| 6 | RT/SHM 面 | **per-instance segment + per-instance `RtLeg` を維持** — bundle が共有するのは制御面（子・pipe・I/O スレッド pair）のみ。`wire::shm` レイアウト・alternation・SAFETY 論証・endpoint detach は無変更。共有 segment 化は却下（`wire::shm` 大改訂級・#118 領域） |
| 7 | 新動詞・語彙写像 | **`Command::CreateInstance`/`DestroyInstance` 追加** — `Instantiate` = 「pool から子を引く or spawn」+ `CreateInstance`、`Destroy` = `DestroyInstance`（+ 子が空なら `Shutdown`+reap・shutdown-on-empty）。`CreateInstance` は `{plugin_id, shm_key, in_depth, out_depth}` を運び、envelope の `instance` が新規 handle |
| 8 | wire バージョン・上限 | **`WIRE_VERSION` 1→2**・新上限 **`WIRE_MAX_INSTANCES: u32 = 64`**・sentinel **`WIRE_INSTANCE_NONE: u32 = u32::MAX`**。上限超過（host 側 handle 枯渇・子 registry 満杯）は defined `Err(HostError::InstanceLimit)` — 新 variant・wire tag 9。duplicate handle 指定は `Err(Ipc)`（host 契約違反）。negotiation・v1 共存なし（#119 方針） |
| 9 | 公開選択面 | **`OutOfProcessSpec.isolation: OutOfProcessIsolation`** — `#[non_exhaustive]` enum `{ Dedicated, Bundled }`・既定 `Bundled`。dedicated は隔離強度の opt-in として残す（登録時選択の原則維持・caller は層を意識しない） |
| 10 | handshake・子スコープ動詞 | `Scan` は子スコープ・spawn 時 1 回（factory 集合は spawn 固定）・`CreateInstance{plugin_id}` が per-instance 正当性を担う（`Err(UnknownPluginId)`）。子スコープ表現は **envelope sentinel `WIRE_INSTANCE_NONE`** — `Scan` のみが使い、codec が動詞↔scope 不整合を `Malformed` 拒否。1 子 1 descriptor/plugin_id（pool = route 単位のため）・多 plugin 混在 bundle は deferred。v2 argv は `<runner>` + spec.args のみ（`--ruviform-shm-key`/`--ruviform-plugin-id` は `CreateInstance` へ移転） |
| 11 | 通知 | **per-instance `InstanceFailed` fan-out 維持** — bundle crash は影響を受けた各 live インスタンスへ `handle`+同一 `ChildExit` を 1 つずつ。新 bundle 相関 variant は初期不要（`#[non_exhaustive]` で後追加可）。per-instance 収束（子は生存）は `exit = {None, None}` で通知 |
| 12 | host 内部構造 | `Shared`/`Inner` を「子共有部」（child・I/O スレッド・waiters・`block_in_flight`・terminal・`instances` map）と「インスタンス部」（`InstShared` = mirror・`rt_state`・staging・notify・result `Producer`・`RtLeg`・segment）へ分解し、**dedicated も同じ v2 経路**（pool 非投入の 1 子 1 instance）として共通化する |
| 13 | GUI | `OpenGui`/`CloseGui` の host-side `Err(Unsupported)` short-circuit を **不変維持** — wire 語彙は instance-scoped 動詞として残置（#115 の解除条件待ち）。`GuiHandle` の再スコープは不要（waiter instance 相関 + entry 所有で足りる） |
| 14 | `kill_child` | **シグネチャ不変** `kill_child(handle)` — handle が属する子を kill → 子スコープ収束で同居全インスタンス `Failed`。bundle crash 注入語彙はこれで足りる |

## Architecture

### wire v2 プロトコル（`ruviform-plugin-core::wire`）

**フレーム**: `[u32 LE payload_len][u8 kind][payload]` 不変。kind 表も不変（`Hello=0`/`Command=1`/`Reply=2`/`BlockIn=3`/`BlockDone=4`/`Shutdown=5`）。

**envelope**: `Command`/`Reply`/`BlockIn`/`BlockDone` の payload は `[u64 seq][u32 instance][残ペイロード]` — `seq` は従来どおり先頭、`instance` が直後に入る統一位置。`Hello`・`Shutdown` は `instance` を持たない（message kind 自体が子スコープ）。

| メッセージ | 方向 | payload |
| --- | --- | --- |
| `Hello { version: u16, capabilities: u32 }` | 子→親 | 不変（`version` = `WIRE_VERSION` = **2**） |
| `Command { seq: u64, instance: u32, cmd: Command }` | 親→子 | `instance` = 宛先 registry key・`Scan` のみ `WIRE_INSTANCE_NONE` |
| `Reply { seq: u64, instance: u32, result }` | 子→親 | `instance` = 受信 `Command`/`BlockIn` の verbatim echo |
| `BlockIn { seq: u64, instance: u32 }` | 親→子 | `instance` = 対象インスタンスの segment 宛 doorbell |
| `BlockDone { seq: u64, instance: u32, result }` | 子→親 | `instance` = 受信 `BlockIn` の verbatim echo |
| `Shutdown` | 親→子 | 不変（空 payload・子終了） |

**新定数**:

```rust
pub const WIRE_VERSION: u16 = 2;
/// 1 子が host できるインスタンス数の上限（runner registry 容量・
/// wire `instance` 値域の上界）。超過 CreateInstance は Err で拒否。
pub const WIRE_MAX_INSTANCES: u32 = 64;
/// 子スコープを表す `instance` sentinel — `Scan` のみに許容。
pub const WIRE_INSTANCE_NONE: u32 = u32::MAX;
```

**`Command` への 2 動詞追加**（tag 10・11 — 既存 0-9 は不変）:

```rust
/// 子スコープ唯一の動詞 — envelope instance = WIRE_INSTANCE_NONE。
Scan,
/// envelope instance = 新規 handle。子側 registry へ entry 挿入:
/// factories から plugin_id で選択 → shm_key の named segment を
/// attach（header 検証 + 宣言 depth との照合）→ lifecycle Created。
CreateInstance {
    plugin_id: String,   // ≤ WIRE_MAX_STRING_BYTES
    shm_key: String,     // ≤ WIRE_MAX_STRING_BYTES
    in_depth: u32,       // 1..=WIRE_MAX_ELEMENTS
    out_depth: u32,      // 1..=WIRE_MAX_ELEMENTS
},
/// envelope instance = 対象。entry を除去（その entry の lifecycle
/// を runner 側で最善尽正規化 — deactivate → destroy の合法範囲）
/// → segment detach → reply Ok。未登録は Err(InvalidHandle)。
DestroyInstance,
```

**scope 規則（decode 境界で強制）**:

- 子スコープ動詞: `Scan` のみ — `instance` は `WIRE_INSTANCE_NONE` 必須
- インスタンススコープ動詞: `CreateInstance`/`DestroyInstance`/既存 9 動詞（`Initialize`/`Activate`/`StartProcessing`/`StopProcessing`/`Deactivate`/`SaveState`/`RestoreState`/`OpenGui`/`CloseGui`）— `instance` は `WIRE_INSTANCE_NONE` 以外必須
- 全 `instance` フィールドは `< WIRE_MAX_INSTANCES` または `== WIRE_INSTANCE_NONE` のみ受理 — 範囲外・scope 不整合は `WireError::Malformed`（codec が動詞→scope 表を持つ。untrusted 入力の bounded-read 原則の延長）
- `BlockIn`/`BlockDone` の `instance` も同じ bounds（`NONE` は `Malformed`）
- `Reply.instance` は echo なので同 bounds を受理し、scope 整合（`NONE` は `Scan` 回答のみ）は reader 側の waiter 照合で担保

**`HostError` 追加**: `InstanceLimit`（wire tag 9 — `#[non_exhaustive]` + v2 bump 内の additive 拡張）。意味: 子 registry が `WIRE_MAX_INSTANCES` 上限（runner 側拒否）、または host 側が pool の free handle を枯渇させた状態で `Instantiate` が呼ばれた（host 側拒否）。`EndpointUnavailable` と同じ「曖昧な `Ipc`/`Unsupported` に潰さない」原則 — 容量という分離された事実を caller が識別できる。なお `CreateInstance` が**既存の** `instance` 値を指定した場合は容量問題ではなく host の契約違反であり `Err(Ipc)` で拒否する（duplicate mint は host 側簿記の破綻 — runner は untrusted 入力として検証境界で拒否）。

**codec wildcard の罠**: `encode_host_error` は `_ => Err(WireError::UnsupportedVariant)` の wildcard arm を持つ（`wire.rs` L1152 頃 — `EndpointUnavailable` 等の wire 表現を持たない variant を fail-closed で弾く仕組み）。`InstanceLimit` の encode arm を書き忘れると、runner 側の defined `Err(InstanceLimit)` 拒否を encode する段で `UnsupportedVariant` に化け、reply が作れず子死亡相当の収束へ転落する — **「定義済み拒否が子死亡に化ける」退行**。codec round-trip test で `encode_host_error(InstanceLimit) → tag 9` と `decode(9) → InstanceLimit` の両向きを pin する（Verification Notes 参照）。

**語彙写像（v2）**:

| 公開面 | wire v2 意味 |
| --- | --- |
| `Instantiate{plugin_id}` | pool から非フル子を引く or spawn → handshake（`Hello`→`Scan`→`Scanned` 照合）→ `CreateInstance{plugin_id=spec.descriptor.id, …}` → `Instantiated{handle}`。`CreateInstance` reply が `Err(UnknownPluginId)` を返すのは Scan 照合済み id に対する不整合であり、host は **`Err(Ipc)` に畳む**（`Err(UnknownPluginId)` を透過すると route 未登録の defined 拒否と区別不能になり誤帰属する — handshake 不変条件違反として `Ipc` に統一） |
| `Destroy{handle}` | `DestroyInstance{instance}` → reply → 子の live 集合が空なら `Shutdown`+grace+reap。`Failed`（`converge_instance` 済み）だが子が live の instance でも `DestroyInstance{i}` は通常送信される — runner registry slot を回収しないと `live_count`・子容量・runner 簿記が drift するため（`Failed` は runner 側でなく host 側の収束状態）。子が既に terminal の場合は送信自体が不可能 — ローカル撤去のみで完了し `Destroy` はいずれにせよ `Ok`（公開 slot は解放される — 現行意味維持） |
| `Scan` | 不変 — route lookup が登録 `descriptor` を返す（子 `Scan` は handshake の照合用途のみ） |
| 残り 9 動詞 | `Command` に 1:1 写像 + envelope `instance`（`to_wire` は変わらず動詞ペイロードのみ翻訳し、`L1Instance::execute` が envelope を付ける） |

**handle minting 規則**: wire `instance` は `[0, WIRE_MAX_INSTANCES)` の子ローカル名空間。host が free 値を選択（実装は lowest-free 等 — 単調採番は不要）・`DestroyInstance` 完了で解放・**reuse 可**。世代は不要: handle→entry 写像は `L1Instance` オブジェクトが所有し、公開 `PluginHandle` の stale 検出は slot generation が既に担う（proxy 層より上で弾かれる）。

reuse の安全性を担保する不変条件チェーンは **(a) 子 `serve` の単一スレッド serial dispatch + (b) stdout FIFO 順序 + (c) `Inner.instances[i]` の removal が `DestroyInstance` reply 粒度 + (d) コマンド seq が子内一意** の 4 点であり、「子が死ねば handle 空間ごと消える」や「`PluginHandle` の generation」に依るのではない（後者は別名空間の別機構 — proxy 層より上の stale 検出を担うだけで wire handle の reuse 安全性とは無関係）。(a)+(b) により `DestroyInstance{i}` より前に送られた全 `BlockIn{i}` の `BlockDone` は reply より先に stdout に並び、新 `CreateInstance{i}` の reply より前に新規宛の frame は存在しない — 適合する子では reuse 競合は閉じる。(c) により reply 到着前の遅延 frame は「`i ∉ instances`」ではなく「registered-but-not-live」へ解決され inert drop に落ち、(d) により古い `Reply` の seq が新規コマンドの waiter に衝突することはない。

**非適合子への degraded 保証**: block seq は per-instance 空間（`RtLeg::next_block`）なので、serial dispatch を破る子（遅延・複製 `BlockDone` を出す）は古い `BlockDone{i,s}` を reuse 済み handle `i` の新しい in-flight doorbell `(i,s)` に alias させ**得る**。この場合 reader は正当な完了として処理するが、被害は defined per-instance 失敗に留まる — out-slot の seq-tag 不一致は `read_out_slot` が fail-closed で抑止し（`data.rs` L357-395 — `slot_seq != seq` または宣言 `frames`/`channels` 超過なら publish しない）、仮に seq tag まで捏造されても「自身の out-slot を壊す」能力は子が元々持つ範囲内であり、最悪でも対象 instance のデータ破壊 = per-instance `Failed`/pass-through へ収束する。兄弟インスタンス・waiter 空間・`block_in_flight` 簿記は影響を受けない。

**seq/correlation 規則**:

- コマンド seq: 子共有の単一 `u64` カウンタ（`Shared.next_cmd_seq` — `L1Instance::next_seq` から移設）。`Reply` は seq で `waiters` を引き、`waiter.instance` と `Reply.instance` の一致を照合（不一致 = 帰属不能 → 子スコープ収束）
- block seq: per-instance `RtLeg::next_block` + `BLOCK_SEQ_FLAG` — **インスタンス間で seq は衝突し得る**ため、demux キーは `(instance, seq)`。`block_in_flight: HashSet<(u32, u64)>`
- BlockIn 拒否: 子は `Reply{instance, seq, Err}`（flagged seq）で応答 — reader は `(instance, seq)` を `block_in_flight` から引き、そのインスタンスの result ch へ `mark_failed` 合成完了（error payload は現行どおり細目を問わず mark_failed — `l1/mod.rs` L498 契約の維持）
- waiter 記録: `{instance: u32, tx}`（`Scan` は `instance = WIRE_INSTANCE_NONE`）— per-instance 収束が「そのインスタンスの pending コマンドのみ」を drain するための帰属情報

**`DestroyInstance` doorbell window（確定）**: `DestroyInstance{i}` が driver により stdin へ直列化された時点で、host 側の `Inner.instances[i]` entry は `Destroying` に遷移する。以後の `Submit{i}` は新規 `BlockIn` を出さず既存の `mark_failed` 合成経路（stale/非適格 submit が leg の alternation gate を wedge させない現行契約 — `l1/mod.rs` L686-723）へ流れる。entry 自体は reply 到着まで保持される — in-flight doorbell `(i,·)` は引き続き `block_in_flight` に残り、届く `BlockDone{i,·}` は通常経路で完了する（serial dispatch + FIFO で、適合する子の全 `BlockDone{i}` は destroy reply に先行する）。reply 処理で entry を除去する際、残存する `(i,·)` の `block_in_flight` があれば `mark_failed` 合成で leg を解放し、以後の `(i,seq)` 完了は「`i ∉ instances` ⇒ inert drop」規則に落ちる — **決して protocol violation にしない**（除去後の遅延完了は dishonest 子による単調でない再現か、removal と並行した通常の後追いであり、どちらも追跡対象が既に存在しない）。

**runner 側対称契約（確定）**: 子が受けた `BlockIn{i}`/instance-scoped 動詞で `i` が runner registry に無い場合、runner は flagged `Reply{i, seq, Err(InvalidHandle)}` の **defined 拒否**を返す — runner 側 protocol violation（normalize + exit 1）には決してならない。handle は host が mint する側であり、runner は untrusted 入力を defined に捌く検証境界として振る舞う（現行 `serve` の「inbound `Hello`/`Reply`/`BlockDone` → protocol violation」は方向違反 frame 限定のまま — registry key 不明は dispatch 結果として `Err` を返す層）。

### host プロキシ再構成（`crates/ruviform-plugin-host/src/l1/`）

**分解**: 現行 `Shared`/`Inner`/`L1Instance` の一体構造を 2 層に分ける。bundle/dedicated の両経路がこの構造を共有する（dedicated = pool に入れない 1 子 1 instance — プロトコル・スレッド・収束の別実装を持たない）。

```rust
// 子共有部（1 子 = 1 個）— 現行 Shared を継承
struct Shared {
    inner: Mutex<Inner>,
    child: OnceLock<Mutex<Child>>,
    next_cmd_seq: AtomicU64,      // 子ワイド・非 flagged seq 採番（Relaxed）
}
struct Inner {
    waiters: HashMap<u64, Waiter>,          // Waiter { instance, tx }
    instances: HashMap<u32, InstShared>,    // wire handle → インスタンス部
    block_in_flight: HashSet<(u32, u64)>,
    terminal: Option<Terminal>,
    shutdown_initiated: bool,
    seen_eof: bool,
}
// インスタンス部（1 インスタンス = 1 個 — Inner.instances の値）
struct InstShared {
    rt_state: Arc<AtomicU8>,      // RT leg が lock-free で読む mirror
    mirror: LifecycleState,
    notify_tx: mpsc::Sender<ChildExit>,
    notify: Notify,               // Sent/Pending の一度切り状態
    staging: Option<Arc<Staging>>,
    shm: SegmentInfo,             // { key, in_depth, out_depth } — I/O スレッドの attach 情報
    result_tx: Mutex<ruviform_rt::Producer<u64>>, // leaf lock（inner 下で bounded try_push）
}
// host 側インスタンスオブジェクト（公開 slot が所有）
pub(crate) struct L1Instance {
    shared: Arc<Shared>,           // 所属子の共有部
    child: Arc<L1Child>,           // 所属子参照（pool backref・teardown 判定用）
    instance: u32,                 // wire handle
    rt: Option<RtLeg>,
    segment: shm::Segment,         // 所有権アンカー（create/unlink は host 側 — 不変）
    notify_rx: mpsc::Receiver<ChildExit>,
    // command_timeout・endpoint attach 簿記は現行どおり
}
// 子共有リソース（pool entry が保持・L1Instance が Arc で参照）
struct L1Child { /* Child・driver/reader JoinHandle・driver_tx・spawn 簿記・live set */ }
```

- **スレッド構成**: 1 子 = driver + reader の **2 スレッドを維持**（インスタンス毎に増えない — spawn コスト削減の主旨）。driver は `submits: HashMap<u32, Consumer<Submit>>` と `views: HashMap<u32, shm::View>` をローカル所有し、poll サイクル毎に全インスタンスの submit consumer を drain: `armed_for(inner.instances[i].staging, submit)` → in-slot 書込 → `BlockIn{instance, seq}` → `block_in_flight.insert((i, seq))`。stale 投入は現行どおり `mark_failed` 合成で対象 instance の result ch へ（alternation gate を wedges させない既存契約の維持）
- **reader**: `views: HashMap<u32, shm::View>` をローカル所有 — 初回 `BlockDone{i,·}` で `inner.instances[i].shm` の情報から attach（attach 不可 = host 側 segment 喪失 → 子スコープ収束）。demux は instance 解決を最初に行う（判定表は軸 1 — 本書「`Failed` 粒度・通知の確定契約」の表が単一の正本）: 通常 `Reply` → `waiters.remove(seq)` + `waiter.instance` 照合 / flagged `Reply` → `(i,seq)` を `block_in_flight` から引き mark_failed / `BlockDone` → `(i,seq)` 確認 → out-slot 読取 → staging → result push。`i` が registered-but-not-live（`Failed` 未破棄・`Destroying`）または `i ∉ instances` の frame は **inert drop** — 帰属可能判定を codec でなく reader に置く責務分離の延長で、「live entry への帰属不能違反」のみが収束を起こす
- **install/uninstall**: インスタンス生成時に host は `DriverCmd::AddInstance{instance, submit_rx}` で consumer を登録し、`Inner.instances` に `InstShared`（segment info・notify・result Producer・`rt_state`）を `CreateInstance` 送信前に登録（waiter `{instance: i}` の一貫性のため）。**mid-`CreateInstance` で子が死んだ場合はローカル収束のみ** — 公開 handle はまだ mint されていないので `InstanceFailed` 通知は出さず、`Instantiate` caller へ `Err(Ipc)` を返し、registry entry は除去する（`live_count` リークなし）。`CreateInstance` の `Err` reply・handshake 失敗も同様に `RemoveInstance` で巻き戻す。`DestroyInstance` reply 後に同様の撤去
- **converge の分解**:
  - `converge_child(exit)` — 子スコープ: terminal を一度だけ確定 → kill/reap → **全 live インスタンスに対し**: mirror=`Failed`（`rt_state` atomic へ publish）・その instance の pending waiters を `Err(Ipc)` で drain・in-flight doorbell に `mark_failed` 合成・`notify_tx` へ `ChildExit` を一度だけ queue。**未完了の child-scope waiter（`Scan` — `instance = WIRE_INSTANCE_NONE`）も同じく `Err(Ipc)` で drain** する（現行は waiter 全 drain — `l1/mod.rs` L407）。mid-`DestroyInstance` の entry もここでは registered として扱われ通常の fan-out を受ける（notification は emit される — observer が見る前に slot が解放される従来 race は現行意味どおり）。mid-`CreateInstance` は上記のとおり通知なしで巻き戻す
  - `converge_instance(i)` — インスタンススコープ（軸 1 の帰属可能違反のみ）: 子は殺さず、対象 instance の mirror=`Failed`・その waiters を `Err(Ipc)` drain・その doorbell に `mark_failed` 合成・`notify_tx` へ `ChildExit{code:None, signal:None}`（子 exit 未観測を正直に表す — 子は生きている）。収束後も entry は `DestroyInstance` まで registered に留まる（runner registry slot の占有が続く）— drain 済み waiter 宛・合成済み doorbell 宛の後続 frame は registered-but-not-live として inert drop で吸収する（「waiter-less `Reply` ⇒ 子スコープ」の厳格規則は live instance にのみ適用 — 現行 `l1/mod.rs` L500-508 の waiter-less drop 契約と同じ挙動）
  - 既存の冪等性（先着者が terminal/Notify を確定）はそのまま維持
- **pool**: `Route::OutOfProcess` が struct variant `{ spec: SpawnSpec, isolation: OutOfProcessIsolation, pool: Vec<Arc<L1Child>> }` へ形状変更（現行 tuple variant `OutOfProcess(SpawnSpec)` — `composite.rs` L160-167）。`instantiate` は `routes.get` → `routes.get_mut` に変わる（pool を可変に触る — 現行 L823）。pool entry は `{ L1Child, live set }`（live set = `Inner.instances` の登録キー集合 — `live_count` は registered 全件、`Failed` 未破棄・`Destroying` 含む）。**選択述語（確定）**: `live_count < WIRE_MAX_INSTANCES && !terminal && !shutdown_initiated` の最初の子を選び、該当なしなら `L1Child::spawn` で追加 → `CreateInstance`。pool は複数子へ自然に拡張する（route 容量は事実上無制限・爆風半径は子単位 64 に bounded）。`converge_child` は terminal 子を pool から eviction する（dead entry が選択対象に残らない）。spawn/handshake/`CreateInstance` 失敗は pool entry を巻き戻す — **fresh-spawn 子の失敗のみ**子ごと teardown + 除去する（`live_count` も戻る）。**既存の共有子への `CreateInstance` 失敗は instance-entry の巻き戻しのみ**（runner は成功時のみ entry を挿入するため runner 側に残滓なし）— live 兄弟を持つ健全な子を teardown してはならない。**last-reference teardown 規則**: `L1Instance` drop または `PluginHost`/route teardown で最後の参照が外れた子は、最後の referrer が bounded teardown（live なら `Shutdown`+grace、terminal/wedged なら kill → reap → driver join）を実行する — 現行 `Drop for L1Instance` の「driver_tx 切断 → driver `Disconnected` → kill/reap → `join_threads`」（`l1/mod.rs` L1770-1801）を子単位へ一般化した形。live=0 の子は `Destroy` 経路で `Shutdown` → pool 除去 → reap（shutdown-on-empty）。**1 インスタンスの drop が live 兄弟を持つ子を殺してはならない** — drop は自身の registry entry 除去（best-effort `DestroyInstance`）と自身の `Arc<L1Child>` 参照解放のみを行い、driver_tx は子共有リソースのため最後の参照が外れるまで切断しない
- **dedicated**: `isolation: Dedicated` では route が pool を持たず、各 `Instantiate` が新規 `L1Child::spawn` → `CreateInstance`（v2 handshake）→ `L1Instance` がその子を排他所有。同じ v2 codec・同じ `L1Child`/demux/converge を通る — 差異は pooling のみ。検証 seam は `PluginHost` + `isolation: Dedicated` の route 駆動が正本で、`l1_proxy_e2e.rs` が現行 `#[path]` 経由で `L1Instance::spawn` を直接駆動しているのと同等の `pub(crate)` spawn+`CreateInstance` helper を残す（`#[path]` harness が dedicated 子を 1 発で立てられる形）

### runner 再構成（`crates/ruviform-plugin-export`）

- `Runner` singleton → **`instances: HashMap<u32, InstanceEntry>`** registry 化。`InstanceEntry = { instance: Box<dyn ErasedPlugin>, lifecycle: Lifecycle, context, activated_max_frames, segment: shm::Segment, staging_in, staging_out }`（現行 `Runner` フィールドの per-instance 化）
- `serve` の dispatch に instance 解決段を挿入: `Command{seq, instance, cmd}` → `Scan` → factories 列挙 / `CreateInstance` → registry 挿入 / 他は `instances[instance]` lookup → `Err(InvalidHandle)` / `BlockIn{instance, seq}` → entry の segment で `process_block` → `BlockDone{instance, seq}` or flagged `Reply{instance, seq, Err}`。**echo は受信値 verbatim**。未登録 registry key 宛の `BlockIn`/動詞は全て flagged `Reply{i, seq, Err(InvalidHandle)}` の defined 拒否 — runner 側 protocol violation（normalize + exit 1）にはならない（上記「runner 側対称契約」）
- `CreateInstance` の検証: `plugin_id` ∈ factories（`Err(UnknownPluginId)`）・`registry.len() < WIRE_MAX_INSTANCES`（`Err(InstanceLimit)`）・`instance` が既登録でないこと（重複指定は `Err(Ipc)` — host 契約違反）・segment attach + header 検証 + depth 照合（`Err(Ipc)`）。capacity は duplicate より先に判定する — 満杯の registry は全合法 handle を既登録として保持するため、dup 先行では `InstanceLimit` が到達不能になる（実装の順序が正本）
- plugin hook panic → `Err(PluginFailed)` 正規化は per-command で不変 — **dispatch 内の `factory.descriptor()`（plugin_id 照合）/`factory.create()` を含む全 plugin code を `catch_unwind` で guard し、panic は `Reply{i, seq, Err(PluginFailed)}` で返して process-exit させない**（mid-serve の exit は同居 sibling instance を全て殺す — `Scan` guard と同形を `CreateInstance` 経路へ維持する）。instance-scoped の定義済み拒否として runner/他 instance は生き続ける
- **argv 規約（v2）**: `<runner>` + spec.args のみ — `--ruviform-shm-key`/`--ruviform-plugin-id` は `CreateInstance` payload へ移転し argv から除去。未知の `--ruviform-*` フラグは usage 拒否（exit 2）を維持
- 終了規約: `Shutdown` 受信 or stdin EOF で **registry 内の全 entry を正規化して exit 0**（子の自律 exit 機構は入れない — host 送信 `Shutdown` のみ。`DestroyInstance` 全滅後に子が残るのは host が `Shutdown` を送るまでの正常状態）

### 公開面（plugin-host 追加）

```rust
pub struct OutOfProcessSpec {
    pub descriptor: PluginDescriptor,
    pub program: OsString,
    pub args: Vec<OsString>,
    pub lookahead_blocks: u32,
    pub command_timeout: Duration,
    /// この route のインスタンスを共有子プロセスに束ねるか（Bundled —
    /// 既定）、インスタンス毎に専用子プロセスを立てるか（Dedicated —
    /// crash 隔離強度の opt-in）。
    pub isolation: OutOfProcessIsolation,
}

/// L1 route の子プロセス共有方針 — `#[non_exhaustive]`（将来の
/// 明示 pool キー・強化隔離 variant を additive に足せる余地）。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutOfProcessIsolation {
    /// 1 インスタンス = 1 子プロセス — `Failed` 粒度がインスタンス
    /// 単位になる強隔離形（プロセス・メモリコストは per-instance）。
    Dedicated,
    /// 同一 route のインスタンスを子プロセス共有 — spawn/メモリコスト
    /// を削減するが、子死亡は同居インスタンスを同時に `Failed` にする
    /// （ADR `docs/adr/issue-117-bundle-child-failed-granularity.md`）。
    /// crash 隔離だけでなく **liveness 隔離も trade する**: 1 インスタンス
    /// の plugin hang（`process`/hook が返らない）は子の単一スレッド
    /// `serve` ループを wedge させ、`command_timeout` 経由で同居全
    /// インスタンスが `Failed` 収束する。共有 stdin/stdout は大きな
    /// frame（例: multi-MB `RestoreState` payload）を serial 化するため
    /// 他インスタンスの doorbell 応答を遅らせ得る。分離は state/attribution
    /// 単位でありアドレス空間単位ではない — 破損した plugin が同居 slot
    /// を書き換え得るが、`read_out_slot` の fail-closed 検査が defined
    /// per-instance 失敗へ封じる。
    Bundled,
}
```

`OutOfProcessSpec::new` は `isolation: Bundled` を既定とする（公開 enum・struct とも doc で `Failed` 粒度の意味を明示）。`register_out_of_process`/`Instantiate`/`Destroy`/`HostNotification`/`PluginHandle` の公開面・`PluginHost` の `!Send`/`!Sync` は不変 — caller は bundle/dedicated をコマンド系列から意識しない（登録時選択の原則）。

`#[doc(hidden)] kill_child(handle)` はシグネチャ不変で「handle が属する子プロセスを kill する」に doc を確定 — bundle では同居全インスタンスが `Failed` 収束し per-instance `InstanceFailed` が fan-out する。bundle crash 注入の検証語彙はこれで足り、instance-scoped 故障注入（特定 instance のみ hang/reject）は fixture 側の拡張として deferred。

## `Failed` 粒度・通知の確定契約（軸 1 — ADR と一体）

**用語の定義** — `i` が `Inner.instances` に登録済みで未除去のものを **registered** と呼ぶ（runner registry slot を占有し `live_count` が数える集合 — `Failed` 未 `Destroy`・`Destroying` を含む）。registered のうち converge 未収束・`Destroying` でないものを **live** と呼び、残り（`converge_instance` 済みの `Failed` 未破棄・`Destroying`）を **registered-but-not-live** と呼ぶ。`i ∉ Inner.instances` は post-removal（`DestroyInstance` reply 処理後）または未登録値のいずれか — reader は両者を区別しない（区別の必要もない — 後述の inert 規則で同一に扱う）。

| 違反・事象 | 収束粒度 | 理由 |
| --- | --- | --- |
| 子死亡・kill・`try_wait` exit 検出 | 子スコープ — 同居全 live インスタンス `Failed` | プラグインコードは子と共に死ぬ — 物理的事実 |
| stdout EOF・stdin write 失敗・デコード不能 frame・未知 kind・2 度目の `Hello` | 子スコープ | ストリーム共有資源の破壊 — どのインスタンス宛か分からず**帰属不能**。部分 `Failed` は「この frame は instance X 宛だった」と嘘をつく |
| `instance` が **live** entry に解決する frame で: `Reply` の seq が waiter に無い・`Reply.instance` と waiter 記録の不一致・`instance` 範囲外/scope 不整合（decode `Malformed`） | 子スコープ | 相関空間（子ワイド seq）または instance 値自体が信用不能 — 帰属不能。**「waiter-less `Reply` ⇒ 子スコープ」の厳格規則は live instance にのみ適用する**（registered-but-not-live・`i ∉ instances` は下行の inert 規則） |
| `BlockDone`/flagged `Reply` で `(instance, seq)` が「live instance だが seq が `block_in_flight` に無い」 | **その instance のみ `Failed`** | frame は well-formed で demux キーが一意帰属 — そのインスタンスの doorbell 簿記破壊と確定できる。兄弟を巻き込む正当化がない（runner registry 上も per-instance state は分離済み） |
| `instance` が **registered-but-not-live** に解決する frame — waiter 不在の `Reply`・`block_in_flight` に無い `BlockDone`/flagged `Reply` | **inert — 静かに drop（違反ではない）** | 記録は `converge_instance`/`DestroyInstance` 経路が**正当に drain 済み** — 遅れて届く frame を違反とみなすと、正常な収束経路の後追いで子・兄弟を殺してしまう。典型シナリオ: (a) `converge_instance(i)` が in-flight doorbell に `mark_failed` を合成した後に届く `BlockDone{i,s}`、(b) 同じく waiter を drain した後に届くコマンド `Reply{i}`。現行 singleton 実装も waiter-less `Reply` を静かに drop している（`l1/mod.rs` L500-508） |
| `instance ∉ Inner.instances`（post-removal or 未登録値）宛の `Reply`/`BlockDone` 完了 | **inert — 静かに drop（決して protocol violation にしない）** | `DestroyInstance` reply で entry が除去された後の遅延完了・未使用値の捏造 frame — 追跡対象が存在せず、well-formed であっても兄弟を殺す根拠がない（捏造 handle による DoS を拒む）。runner 側対称契約「未登録 key ⇒ flagged `Err(InvalidHandle)`」と対になる host 側規則 |
| `Shutdown` 送信済み（`shutdown_initiated`）の子から届く frame・EOF | **non-event — clean teardown を変えない** | 現行 `failed = !inner.shutdown_initiated` 意味の維持（`l1/mod.rs` L390）— shutdown-on-empty（live=0 で送信）と last-ref teardown（live 残でも graceful `Shutdown`+grace）のどちらでも、送信後に届く遅延 frame・exit・EOF は clean 収束の範囲に留まり `Failed` 判定に寄与しない |
| コマンド応答 timeout（`command_timeout`） | 子スコープ | 未応答の原因を局所化できない — 現行と同じ扱い |
| per-command `Err(HostError)`（`PluginFailed`/`Lifecycle`/`InvalidHandle`/`InstanceLimit`/`Unsupported`） | instance 単位の定義済み拒否（`Failed` ではない — 現行不変） | well-formed reply は常に帰属可能 |

**通知**: bundle crash は `converge_child` が影響を受けた各 live instance の `notify_tx` へ `ChildExit`（全て同値）を一度ずつ queue → `try_recv_notification` の slot 走査が per-instance `InstanceFailed{handle, exit}` を返す（現行機構の自然な拡張・新 variant なし）。per-instance 収束は `ChildExit{None, None}` で 1 通知（`Failed`+通知+pass-through の契約維持 — exit は「子の終了状態」であり、子未観測を正直に表す）。

## RT / safety invariants（維持を要求 — bundle で変えない）

- `PluginHost` は `!Send`/`!Sync` のまま — route/slot 表・pool は host スレッド局所の所有物
- 制御側所有権と RT 側所有権の分離 — `prepare_endpoint` が per-instance `RtLeg` を detach し、endpoint が一時所有して channel 経由で返す（`l1/mod.rs` L1469-・`endpoint.rs` の所有権表）。endpoint は子・segment・mutex・I/O スレッドを所有しない
- `ruviform-audio` は plugin 系クレートに依存しない — wire 型は `ruviform-plugin-core` が所有し host/export が共有する依存方向不変
- 各インスタンスの `RtLeg` は strict alternation・`armed_for` generation・arm/detach/return lifecycle を維持 — `next_block` は per-instance のまま（衝突は `(instance, seq)` demux で吸収）
- per-instance segment: producer/consumer の single-writer 規則・fence(Release/Acquire)・`seq % depth` slot 選択・`wire::shm` 固定レイアウトは無変更 — unsafe は `l1/shm.rs`・`export/shm.rs` の既存 2 関数 + SAFETY コメントのまま
- per-instance segment は **state/attribution 分離**であってアドレス空間隔離ではない — 全 segment は同一の子プロセス空間に map されるため、悪意ある・破損した plugin は同居インスタンスの slot へ書き込める（bundle はプロセス単位隔離 — spec の L2+ 強化範囲外）。ホスト側の防衛線は `read_out_slot` の fail-closed 検査（`data.rs` L357-395 — seq-tag 不一致・宣言 `frames`/`channels` 超過は publish しない）で、被害は defined per-instance 失敗/pass-through に封じる
- 公開 enum は `#[non_exhaustive]` 維持 — v2 で追加される `Command`/`Reply` variant・`HostError::InstanceLimit` は同方針。未知 variant は `UnsupportedVariant`/defined 拒否
- 1 子 = 1 reader スレッド（インスタンス毎に増えない）— reader が全 child frame を `(instance, seq)` で demux
- `converge` の冪等性: 先着者が terminal を確定・二重収束なし。子スコープ収束は**影響を受けた全 live インスタンス**の mirror/waiter/notify を完了させる
- RT 経路（`RtLeg::push`/`pop` 系）は allocation-free・lock-free・nonblocking を維持 — `rt_` 命名テストが bundle 経路でも測定対象
- decode は untrusted 入力の検証境界のまま — `instance` 値域・scope 整合・新 payload の宣言長/count を `Malformed` で拒否し panic・無制限 alloc しない

## `WIRE_VERSION` 1→2 bump の棚卸し（Issue #119 方針により全更新）

| 対象 | 変更 |
| --- | --- |
| `crates/ruviform-plugin-core/src/wire.rs` | `WIRE_VERSION` L88 = 2・`Message`/`Command`/`Reply` enum + codec（envelope・新 tag・scope 検査）・新定数 `WIRE_MAX_INSTANCES`/`WIRE_INSTANCE_NONE`・「`WIRE_VERSION` `1`」を名指しする doc 記述（L192・L202・L213・L266・L1352 頃 — shm レイアウト const は値不変だが版名指しを更新）・`encode_host_error`/`decode_host_error` tag 9（L1111-1189 — wildcard arm 追加忘れは defined 拒否が `UnsupportedVariant` に化ける罠。round-trip test で pin） |
| `crates/ruviform-plugin-host/src/l1/mod.rs` | spawn argv から `--ruviform-*` bootstrap 引数除去（spec.args のみ）・handshake（Hello → `Scan` child-scope → `CreateInstance`）・`Shared`/`Inner`/`L1Instance` 分解・demux・converge・`L1Instance::execute` の envelope/scope 写像（`Scan`→`WIRE_INSTANCE_NONE`・他動詞→`self.instance` — `request`/`execute` が付ける） |
| `crates/ruviform-plugin-host/src/l1/shm.rs`・`crates/ruviform-plugin-export/src/shm.rs` | header `wire_version` 検証は `WIRE_VERSION` const 参照のまま自動追従 — doc 記述の版名指しのみ要確認。`--ruviform-shm-key` 言及の doc は `CreateInstance` 受け渡しへ更新（`l1/shm.rs` L8・L181・`export/shm.rs` L8） |
| `crates/ruviform-plugin-export/src/lib.rs` | `RunnerArgs`/`run`/`serve`/`dispatch`/`Runner` の registry 化・argv 規約・USAGE・doc（crate doc L14-15 "one child process hosts exactly one plugin instance"・L24-40 argv 契約・L42-56 exit semantics の registry 対応記述） |
| `crates/ruviform-plugin-host/src/composite.rs` | `OutOfProcessSpec.isolation`・`OutOfProcessIsolation`・`Route::OutOfProcess` は tuple variant → struct variant `{ spec, isolation, pool }` へ（L160-167）・`instantiate` は `routes.get` → `get_mut`（L823 — pool 可変）・`instantiate`/`destroy` 写像・`to_wire`（動詞のみ翻訳 — instance は `execute` が envelope で付す）・`expected_reply_kind`/`host_reply` の v2 対応（wildcard が新動詞→`Reply::Ok` に既定で写るが、`CreateInstance`/`DestroyInstance` → `Reply::Ok` の pairing を明示 pin） |
| `crates/ruviform-plugin-host/src/lib.rs` | crate doc L25-26「L1 (opt-in) — one child process per instance」の記述を bundle/dedicated 形態へ更新・L146 `pub use composite::{…}` に `OutOfProcessIsolation` を追加（公開面の一部として再エクスポート — 忘れると `OutOfProcessSpec::isolation` の型が公開経路から辿れない） |
| `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` | v2 codec + registry 化（`CreateInstance` で shm attach）— argv の shm-key/plugin-id は fixture 側でも除去、crash/hang/reject の子スコープ注入 knob は維持 |
| `crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-runner.rs` | bin doc L28-29「`--ruviform-plugin-id <id>` is required on spawn; `--ruviform-shm-key <key>` is always required」を v2 argv（`<runner>` + spec.args・id/key は `CreateInstance` payload）へ更新 |
| `crates/ruviform-plugin-vst3/src/bin/ruviform-plugin-vst3-l1-runner.rs`・`crates/ruviform-plugin-clap/src/bin/ruviform-plugin-clap-l1-runner.rs` | bin doc の argv 契約記述（vst3 L12・clap L11 — 「`run` argv contract (`--ruviform-shm-key`, `--ruviform-plugin-id`, …) is unchanged」）を v2 argv へ更新 |
| `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` | `MiniHost` argv/segment create → CreateInstance 駆動化・v2 コマンド系列 |
| `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` | 既存検証の v2 更新 + bundle シナリオ追加（`#[path]` harness が駆動する spawn seam は dedicated 子の spawn+`CreateInstance` helper へ移行） |
| `crates/ruviform-plugin-export/README.md` | L7（`Instantiate` ごとに spawn の記述）・L13-14（argv 契約表）を v2 へ更新 |
| `crates/ruviform-plugin-vst3/README.md`・`crates/ruviform-plugin-clap/README.md` | `run` の argv 規約記述（vst3 L72・clap L64）を v2 へ更新 |
| `crates/ruviform-plugin-export/Cargo.toml` | L13 依存コメントの `--ruviform-shm-key` attach 記述を `CreateInstance` 経由へ更新 |
| `docs/test-specs/plugin-l1-wire-codec-tests.md` | L52（`WIRE_VERSION = 1` 名指し）・L62（「子は singleton」・`Instantiate`/`Destroy` 非 wire 動詞の記述 — v2 では `CreateInstance`/`DestroyInstance` が wire 動詞）・L93（non-goals「bundle 子 …将来プロトコル改訂」行 — v2 で改訂済みとなる旨の更新）を v2 契約へ更新 |
| `docs/test-specs/plugin-l1-proxy-contract-tests.md` | L52（「per-instance L1 プロキシ（`pub(crate)` — 1 子プロセス + `driver`/`reader` 2 スレッド）」— v2 では 1 子 = 2 スレッドを維持するが「per-instance」→「per-child」）・L80「`Hello.version != 1` → handshake 失敗」等の version 名指し・per-instance spawn 前提の記述を v2 へ更新 |
| `docs/test-specs/plugin-l1-runner-e2e-tests.md` | L57（argv 規約 `--ruviform-plugin-id`/`--ruviform-shm-key` — v2 では `<runner>` + spec.args・id/key は `CreateInstance` payload）・L60（dispatch 行 — `instance` dispatch 段と registry）・L70-71（argv 前提の examples）・L80（guarantees の argv→attach 経路）・L91（non-goals「runner は singleton」行）を v2 へ更新 |
| `docs/test-specs/real-adapter-l1-e2e-tests.md` | L55（runner argv 規約の「不変」記述 — v2 では argv は spec.args のみ）・L56（spawn→handshake 系列 — `Instantiate` が spawn+Hello+Scan+`CreateInstance` 経由に変わる）・L61/L70-71（`InstanceFailed` once・kill 系列 — per-handle once は維持するが bundled route の kill は影響 instance 数分の通知列になる但書）を v2 へ更新 |
| `docs/specs/issue-114-real-adapter-l1-e2e.md` | L99・L123 の argv 規約「不変/維持」記述 — #114 の変更対象としての不変性記録は正しいが、#117 bump で argv 規約自体が v2 へ変わる旨の但書（実装 bump 時） |
| `docs/designs/issue-114-real-adapter-l1-e2e.md` | L76「上流確定事項（再決定しない）」の v1 前提列（「1 子 = 1 インスタンス（per-`Instantiate` spawn）」・argv 規約）・L354 verification 行 — v2 前提への更新 |
| `docs/specs/engine-plugin-host-integration.md` | L135 AC5「L1 子強制終了 → `Failed` + `InstanceFailed` 通知 once」— per-handle once semantics は維持するが、bundled route の子 kill は影響を受けた **各** live instance へ 1 つずつ（N 通知列）になる Failed-granularity 但書（粒度正本は本 ADR） |
| `docs/designs/plugin-l1-out-of-process.md` | 既 reconcile 済みの L179 語彙写像・deferred 行に加え、実装 bump 時に本書確定へ更新: L113（「1 子プロセス = 1 プラグインインスタンス（per-`Instantiate` spawn）」singleton モデル記述）・L115（スレッド構成表「per L1 インスタンス = 子プロセス 1 + スレッド 2」）・L142（`WIRE_VERSION = 1`）・L147-151（wire 表 — `Command{seq}` 等に `instance` なし）・L155（`Command` enum コメント「子は singleton」）・L181（handshake argv `--ruviform-plugin-id`/`--ruviform-shm-key`）・L196（argv shm-key 引き渡し記述）・L204-210（plugin-export argv 規約・責務記述） |
| `docs/specs/plugin-l1-out-of-process.md` | L120 クラッシュ契約（「子プロセスの wait/exit status でクラッシュを検出し、対象インスタンスを `Failed` 状態へ遷移」）へ Failed-granularity の但書 — bundle 形態では子死亡は同居全 live インスタンスの `Failed` を意味し、粒度の正本は本 ADR（`docs/adr/issue-117-bundle-child-failed-granularity.md`）である旨を追記 |
| 歴史記録（更新対象外 — 時点記録として残す） | `docs/discovery/issue-117-bundle-child-process-discovery.md`・`docs/discovery/issue-114-real-adapter-l1-e2e.md`・`docs/impl/ir/*`・`docs/plans/*`・`docs/tasks/*` の v1/singleton 前提記述は各 Issue の記録時点の事実 — v2 bump で書き換えない（「`WIRE_VERSION` `1`」字句検索・第 2 grep の対象から除外する運用を明記） |
| バージョン言及するテスト名 | `handshake_version_mismatch`・`e2e_version_mismatch_is_a_parent_side_handshake_failure`・`e2e_shm_header_wire_version_mismatch` 等は `WIRE_VERSION` const 参照で追従（fixture の `+1` 破損注入は semantics 不変）— 実装時に「`WIRE_VERSION` `1`」の字句検索で残存を全拾い。第 2 grep パターン（singleton 前提の言及残り）: `--ruviform-`・`one child`・`per-instance spawn`・`1 子`・`singleton` |

## Risks and Trade-offs

| リスク / トレードオフ | 評価と緩和 |
| --- | --- |
| `Failed` 粒度の退化 — 1 インスタンスの子破壊で bundle 内全員が `Failed` | 物理的事実（子死亡 = 全プラグインコード死亡）への正直な写像。`Dedicated` isolation が強隔離 opt-in として残る。per-command `Err` 拒否は instance 単位を維持し、帰属可能違反のみ instance 単独 `Failed` で爆風を最小化（ADR で公開契約として記録） |
| 帰属可能違反でのみ instance 単独 `Failed` は「他を信じすぎ」との反論 | runner registry は per-instance state（lifecycle・segment・seq 空間）を分離済み — `(i, seq)` が帰属確立する違反は i の channel 破壊と確定できる。逆に無差別に子を殺す方が `Dedicated` と `Bundled` の差を意味なくする。どちらにせよ honest でない中間（「多分 X のせい」）は設けない。なお「分離」は state/attribution 単位であってアドレス空間単位ではない — 破損した plugin が同居 slot を汚しても `read_out_slot` の fail-closed 検査が defined per-instance 失敗へ封じる |
| `Bundled` 既定が既存挙動を静かに変える（crash 爆風半径の拡大） | ワークスペースは `publish = false`・0.1.0 の未公開 API — 互換契約の破壊ではなく仕様確定期の既定値選択。enum field は pub なので struct literal 利用者はコンパイル時に選択を強制される。spec・enum doc・ADR で `Failed` 粒度を明示（Issue 要求の主目的であるコスト削減を既定に据える） |
| liveness 隔離の退化 — plugin hang（`process`/hook が返らない）が同居全員を `Failed` にする | `Bundled` が trade するのは crash 隔離だけでなく **liveness 隔離** — 1 インスタンスの hang は子の単一スレッド `serve` を wedge させ、wedge は `command_timeout` 経由で子スコープ `Failed` になる（hang 検出は timeout しかない — 現行 singleton でも同じ機構・爆風半径が子に拡大しただけ）。共有 stdin/stdout は大きな frame（multi-MB `RestoreState` payload 等）を serialize し、他インスタンスの doorbell 応答を遅らせ得る — block 経路自体は RT leg の pass-through で継続するため音は切れないが、`command_timeout` 系コマンドの遅延は実害になる。enum doc と ADR で公開契約として明示し、強隔離が要る route は `Dedicated` を選ぶ |
| reader 1 本が N インスタンスの frame を捌くスループット集中 | block 完処理は staging memcpy + try_push で浅い。N ≤ 64・ブロックは RT 周期駆動なので実負荷は穏当。#120（共有リーパー/io 多重化）が後続で最適化余地を持つ — 本設計はその前提と競合しない（子単位 I/O pair を維持） |
| per-instance segment 増加で子の attach 数・マップ量が増える | 1 インスタンス = `segment_len(3,3)` = **3,349,276 B**（`SLOT_BYTES` 558,208 × 6 slots + header 28 — `wire.rs` L214/L232）× `WIRE_MAX_INSTANCES` 64 = **214,353,664 B ≈ 204.4 MiB** の named mapping 上限/子 — 許容範囲だが `WIRE_MAX_INSTANCES=64` の根拠の一部（1024 等への拡大は segment 総量と爆風半径の双方で不適切）。`segment_len(3,3) * WIRE_MAX_INSTANCES` の const test を layout-chain テスト（`wire.rs` L1716-1737 系）に並べて pin すると、上限根拠がレイアウト変更に追随する |
| `CreateInstance` の handle が reuse されると旧 handle 宛の遅延 frame と衝突するのでは | 適合する子では serial dispatch + FIFO + reply 粒度 removal + 子一意コマンド seq の不変条件チェーンで閉じる（`DestroyInstance{i}` 以前の全 `BlockIn{i}` の `BlockDone` は reply より先、新 `CreateInstance{i}` reply より後に新宛 frame は来ない）。`Inner.instances` の登録/撤去は reply 単位で行い中間状態を作らない。serial dispatch を破る非適合子では古い `BlockDone` が reuse handle の新 doorbell に alias し得るが、被害は seq-tag fail-closed 経由で defined per-instance 失敗に留まる（子が元々持つ自己 out-slot 破壊能力の範囲 — 詳細は「handle minting 規則」） |
| driver の N-consumer drain がコマンド応答を遅延させる | 現行と同じ優先度（`DriverCmd` 先処理 → submit drain）を維持。poll 間隔 `DRIVER_POLL_INTERVAL` も維持 — per-cycle の drain 量は per-instance 1 Producer で bounded |
| v2 化で fixture・MiniHost・worker の更新面が広い | `#123` の frame-I/O dedup が先行すれば 4 系統（host/driver/reader/runner）の instance 付き frame 変更量が縮む — 実装 Issue では #123 残件の先行着地を推奨（必須ではない） |
| 帰属可能判定を codec でなく reader に置く設計の複雑さ | codec は形式検証（bounds・scope 整合）までで帰属判定は registry/live 状態を見る reader の責務 — 検証境界の責務分離として正しい。判定表（軸 1）を reader demux の単一場所に集約する |

## Deferred Design Concerns

| 論点 | 本書での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| 明示 pool キー・route 跨ぎ bundle（同一 runner binary が複数 plugin_id を内蔵） | **deferred** — pool は route 単位に閉じる。`OutOfProcessIsolation` は `#[non_exhaustive]` で `Bundled` variant 拡張（キー付き）や新 variant の余地を残す | 混在 bundle の需要が顕在化した時点の follow-up issue | 同一 binary の複数 plugin_id が子を共有できず bundle 効果が route 単位に限定される |
| pool keep-alive / warm pool（live=0 後も子を残す） | **deferred** — shutdown-on-empty を既定。keep-alive は spawn コストと引き換えの運用ポリシーで別論点 | warm-start レイテンシ要求 | 空の子プロセスが残留する or spawn コストが Instantiate 毎に戻る |
| `BundleFailed` 等の bundle 相関通知 variant | **deferred** — per-instance fan-out で現契約維持（同一 `ChildExit` の連続通知で相関は推測可能）。`#[non_exhaustive]` で後追加可 | observer が相関情報を明示要求した時点 | observer が「同一子の死」を 1 イベントとして処理できない |
| instance-scoped 故障注入語彙（特定 instance のみ hang/reject/crash の fixture knob） | **deferred** — `kill_child` の子スコープ注入で確定契約は検証可能 | bundle per-instance 収束経路の精密検証が必要な時点 | `converge_instance` 経路の実シナリオ検証が fixture 拡張まで限定的 |
| 複数 reader/demux スレッド・io 多重化 | **deferred** — 1 子 2 スレッド維持 | Issue #120（共有リーパー/io 多重化 — 子単位集約後の方が相性が良い） | 高インスタンス密度での単一 reader スループット上限 |
| 共有 shm segment / `wire::shm` 第 2 レイアウト | **out of scope 確定** — per-instance segment 維持 | Issue #118 系 | shm 再設計に沈み本 Issue が発散 |
| `GuiHandle` と GUI surface の子内所有モデル | **deferred** — L1 GUI は `Unsupported` 維持（#115 の解除条件待ち）。実装時は registry entry 所有 + destroy auto-close（#123 IR の hand-off 指針）で足りる見込み | Issue #115 解除条件の顕在化 | — |
| `runner_program()` 解決・frame-I/O dedup | **別 Issue 継続**（#123 残件 — 先行着地で本実装の変更量が縮むが必須前提ではない） | Issue #123 | instance 付き frame を 4 系統で読み書きする変更量が増えたまま実装へ |

## Verification Notes

- [ ] spec AC1: `WIRE_VERSION = 2` codec round-trip（envelope・新動詞・scope 検査）・v1 Hello の handshake 失敗・不正 `instance`（範囲外・未登録・`NONE` 不整合）の defined 拒否・非 panic/非無制限 alloc。`encode_host_error(InstanceLimit) → tag 9`・`decode(9) → InstanceLimit` の両向き pin（wildcard arm 罠 — 「定義済み拒否が子死亡に化ける」退行防止）
- [ ] spec AC2: 同一 route の 2 回目 `Instantiate` が子追加 spawn せず `CreateInstance` で成立・各 instance が独立 `PluginHandle`/lifecycle/process 経路を持つ。pool 選択述語 `live_count < WIRE_MAX_INSTANCES && !terminal && !shutdown_initiated` に従う
- [ ] spec AC3: 2 インスタンス並行時に `Reply`/`BlockDone` が `(instance, seq)` で正しく相関・混線なし（waiter 照合・`block_in_flight` の複合キー）
- [ ] spec AC4: 確定粒度の全分岐 — 子死亡/帰属不能違反 → bundle 内全 live インスタンス `Failed`・waiters `Err(Ipc)`・pass-through・以後 `Err(Failed)`。帰属可能違反（live instance の未知 block seq）は対象のみ `Failed` で兄弟生存。registered-but-not-live 宛 frame・`i ∉ instances` 宛完了・post-`Shutdown` frame は inert drop/収束不変（違反・通知なし）。`DestroyInstance` doorbell window: `Destroying` 化で新規 submit は `mark_failed` 合成・in-flight doorbell は通常完了・reply で entry 除去（残存 seq は `mark_failed` 解放）
- [ ] spec AC5: bundle crash → 影響各インスタンスへ `InstanceFailed` が 1 つずつ・`exit` 同値。per-instance 収束は `ChildExit{None,None}` 1 通知。mid-`CreateInstance` 死亡は通知なし（handle 未 mint）・mid-`DestroyInstance` 死亡は通知が出る
- [ ] spec AC6: runner registry の dispatch・panic→`PluginFailed` 正規化・`CreateInstance` 上限/duplicate の defined `Err` で runner 生存。未登録 registry key 宛 `BlockIn`/動詞 → flagged `Reply{i,seq,Err(InvalidHandle)}`（runner 側 protocol violation にしない）・Scan 照合済み id の `Err(UnknownPluginId)` reply は host が `Err(Ipc)` に畳む
- [ ] spec AC7: dedicated 経路（`PluginHost` + `isolation: Dedicated` route、または `#[path]` harness が駆動する `pub(crate)` spawn+`CreateInstance` helper）で既存 E2E 相当が無退行 — v2 handshake（spawn→Hello→Scan→CreateInstance）経由でも同契約
- [ ] spec AC8: `rt_` 命名テストが bundle 経路で `mise run test-rt` グリーン — per-instance `RtLeg` 不変条件維持
- [ ] spec AC9: `WIRE_MAX_INSTANCES` 超過 `CreateInstance`/`Instantiate` が `Err(HostError::InstanceLimit)` で拒否・destroy→create で handle 空間が再利用規則どおり動く。`live_count` は `Failed` 未 `Destroy` entry を含む registered 全件を数える。wire-ID reuse（destroy→create で同一 `i`）時に driver が前世代の queued submit を持ち得る場合、`Destroying` 化が新規 doorbell を止め `mark_failed` で解放し、新インスタンスの ring に stale doorbell が届かない。`Failed` + live 子への `Destroy` は `DestroyInstance` を送信して slot 回収・terminal 子ならローカル撤去のみで `Ok`
- [ ] spec AC10: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス・doc audit findings 0
- [ ] spec AC11（pool/子 lifecycle）: 最後の `Destroy` で live=0 の子は `Shutdown`+grace+reap+pool eviction（shutdown-on-empty）。`converge_child` は terminal 子を pool から除去。spawn/handshake/`CreateInstance` 失敗は pool entry 巻き戻し — fresh-spawn 子は teardown+除去、既存共有子への `CreateInstance` 失敗は instance-entry のみ巻き戻し（健全な兄弟同居子を殺さない）。`L1Instance` drop は live 兄弟を持つ子を殺さず、最後の参照が外れた子は最後の referrer が bounded teardown（`Shutdown` or kill + reap + join）
- [ ] spec AC12（in-flight lifecycle crash）: `CreateInstance` 送信中の子死亡 → ローカル収束のみ・`InstanceFailed` 通知なし・`Instantiate` caller へ `Err(Ipc)`・registry entry 除去（`live_count` リークなし）。`DestroyInstance` 送信中の子死亡 → entry は registered のまま `Failed` 収束 + per-instance 通知 emit + `Destroy` は `Ok` を返す
- [ ] 「`WIRE_VERSION` `1`」字句の全言及が v2 へ更新 or 歴史記述として明確化されている

## Sources and Relations

- Issue [#117](https://github.com/scarecrow173/ruviform/issues/117)（本設計の対象 Issue — `relations.source`）・Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)（L1 実装 — bundle を non-goal とした singleton 起点・`relations.source`）
- [issue-117-bundle-child-process spec](../specs/issue-117-bundle-child-process.md) — 本書が `implements` する draft spec（OPEN 論点を本書で確定）・[issue-117 discovery](../discovery/issue-117-bundle-child-process-discovery.md) — 軸別選択肢・推奨方向の一次記録（`derives-from`）
- [plugin-l1-out-of-process spec](../specs/plugin-l1-out-of-process.md)・[design](plugin-l1-out-of-process.md) — singleton 前提の親契約（`derives-from`・deferred 行を本書が引き継ぐ `deferred-by`）
- [issue-119-l1-wire-version-policy](../specs/issue-119-l1-wire-version-policy.md) — 単一最新バージョン方針（`derives-from` — bump 前提）
- [plugin-isolation-strategy ADR](../adr/plugin-isolation-strategy.md)・[plugin-rt-ownership-split ADR](../adr/plugin-rt-ownership-split.md) — 隔離戦略・RT 所有権分離（`derives-from` — dedicated/bundle 選択面・per-instance `RtLeg` の前提）
- [issue-117-bundle-child-failed-granularity ADR](../adr/issue-117-bundle-child-failed-granularity.md) — 本書の `Failed` 粒度・通知・隔離選択面の公開契約記録（`related`）
- [issue-115-l1-gui-cross-process-parenting](issue-115-l1-gui-cross-process-parenting.md)（L1 GUI `Unsupported` 維持の確定）・[issue-114-real-adapter-l1-e2e](issue-114-real-adapter-l1-e2e.md)（実アダプタ E2E）・[issue-122-shared-platform-process-ipc-eval](issue-122-shared-platform-process-ipc-eval.md)（#117 を再設計前提とした抽出延期判定）
- Refs: Issue [#113](https://github.com/scarecrow173/ruviform/issues/113)（RT 所有権分離・endpoint detach）・[#118](https://github.com/scarecrow173/ruviform/issues/118)（shm レイアウト別改訂）・[#119](https://github.com/scarecrow173/ruviform/issues/119)・[#120](https://github.com/scarecrow173/ruviform/issues/120)（共有リーパー/io 多重化）・[#122](https://github.com/scarecrow173/ruviform/issues/122)・[#123](https://github.com/scarecrow173/ruviform/issues/123)（frame-I/O cleanup — `gui_open` 除去済みの確認根拠）
