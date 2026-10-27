---
id: "DISC-034NiVjJSeI0gfYX87KMjH"
type: "discovery"
status: "active"
title: "L1 bundle 子プロセス（1 子プロセス N インスタンス）の探索（Issue #117）"
created: "2026-10-28"
updated: "2026-10-28"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/117"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/119"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/plugin-isolation-strategy.md"
    - "../designs/plugin-isolation.md"
    - "../specs/plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "../specs/issue-119-l1-wire-version-policy.md"
    - "../designs/issue-122-shared-platform-process-ipc-eval.md"
    - "../impl/ir/issue-76-l1-core-wire-task-1-implementation.md"
    - "../test-specs/plugin-l1-wire-codec-tests.md"
    - "../test-specs/plugin-l1-proxy-contract-tests.md"
    - "../test-specs/plugin-l1-runner-e2e-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "plugin-l1-out-of-process.md"
    - "../specs/plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "../specs/issue-119-l1-wire-version-policy.md"
    - "../impl/ir/issue-76-l1-core-wire-task-1-implementation.md"
  derived-by:
    - "../specs/issue-117-bundle-child-process.md"
    - "../designs/issue-117-bundle-child-process.md"
    - "../adr/issue-117-bundle-child-failed-granularity.md"
    - "../plans/implement-issue-117-bundle-child-process.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/115"
    - "https://github.com/scarecrow173/ruviform/issues/118"
    - "https://github.com/scarecrow173/ruviform/issues/120"
    - "https://github.com/scarecrow173/ruviform/issues/122"
    - "https://github.com/scarecrow173/ruviform/issues/123"
  defers: []
  deferred-by: []
---

# L1 bundle 子プロセス（1 子プロセス N インスタンス）の探索（Issue #117）

## Exploration Goal

Issue [#117](https://github.com/scarecrow173/ruviform/issues/117) は、現行 L1 が per-instance spawn（singleton runner）を前提とすることを指摘し、メモリ/spawn コスト削減のため **1 子プロセスが複数プラグインインスタンスを持つ bundle 形態の検討・実装**を要求する。

Issue スコープ（原文）:

- `wire::Command` の handle 修飾追加（現行は singleton 前提で handle を drop）・子側インスタンス集合管理・seq/correlation 空間分離
- `Failed` 粒度の契約確定（子プロセス単位 vs インスタンス単位）— per-instance spawn が `Failed` 粒度に整合した現行設計の見直し
- wire 変更を伴うため wire バージョニング Issue（#119）との順序付け

トリガ:

- `docs/designs/plugin-l1-out-of-process.md` L294 deferred 行 — 「bundle 子（1 子 N インスタンス）| per-instance spawn の方が `Failed` 粒度に整合・単純 | メモリ/spawn コスト要求 | インスタンス数スケールに弱い」
- 同設計 L174 — 「残り 10 動詞は `wire::Command` に 1:1 で写す（handle は子が singleton のため落とす — 将来の bundle 子は `Instantiate`/handle 修飾を wire に追加するプロトコル改訂）」— **bundle 化がプロトコル改訂であることは design 時点で既に予告されている**
- `docs/impl/ir/issue-76-l1-core-wire-task-1-implementation.md` L409-415 Non-Goals — 「wire バージョン進化機構・capability bit 定義・bundle 子（handle 修飾）— plan non-goal」

本探索は **issue framing・コード観測事実（wire 形状・spawn/共有状態・シーケンス分割・runner singleton 前提・公開 API 面・検証ハーネスの現状）・選択肢比較・設計フェーズへ渡す推奨方向の記録**を行う。採否・最終契約の確定権限は design/ADR フェーズにあり、本書は最終判断を含まない。

## Key Issues and Assumptions

### 上流で確定済み（confirmed — design 論点ではない）

- **ワイヤバージョン方針（Issue #119・implemented）**: サポートする wire プロトコルは常に最新の単一バージョンのみ — `Hello.version` 厳密一致・不一致は handshake 失敗、交渉機構・旧版 decode/変換/共存は実装しない。`WIRE_VERSION` は非互換変更時のみ bump（`docs/specs/issue-119-l1-wire-version-policy.md`・`crates/ruviform-plugin-core/src/wire.rs` L29-49）。**bundle 化の handle 修飾・新動詞・ペイロード変更は `WIRE_VERSION` 1→2 bump を伴う非互換変更**として扱う前提が確定済み — 順序付けの懸念は「#119 の方針確定が #117 に先行した」という形で既に解消している。
- **`Instantiate`/`Destroy` の語彙写像**: `Instantiate` = spawn + `Hello` handshake、`Destroy` = `Shutdown` + reap — `wire::Command` は両動詞を持たない（`wire.rs` L63-69・IR L338-343）。bundle では「子の spawn/Shutdown」（子スコープ）と「インスタンスの create/destroy」（インスタンススコープ）が分離するため、この写像の見直し自体が設計対象になる。
- **RT 契約**: RT スレッドは pipe/shm/mutex を触らず、`RtLeg` の SPSC try-leg + 事前確保 staging のみ（`l1/data.rs` L20-34・`composite.rs` L210-219）。bundle 化でもこの不変条件は維持必須。
- **自動再起動なし・`Failed`+通知+pass-through**: L1 契約（design 確定・実装済み）。bundle で変わるのは `Failed` の**粒度**のみ。
- **公開 enum `#[non_exhaustive]` 方針**: `Message`/`Command`/`Reply`/`WireError`/`HostNotification`/`HostError` は全て `#[non_exhaustive]`（`wire.rs` L257/314/356/464・`composite.rs` L144）— wire 側の新バリアントはバージョン bump 前提で追加可能、公開 notification enum の拡張は後方互換のまま行える。

### コード観測事実（confirmed — singleton 前提が最初に崩れる地点の特定）

#### wire 層（`crates/ruviform-plugin-core/src/wire.rs`）

- フレーム形式: `[u32 LE payload_len][u8 kind][payload]`（L13-16）。kind バイトは L831-836 に固定列挙 — `Hello=0`/`Command=1`/`Reply=2`/`BlockIn=3`/`BlockDone=4`/`Shutdown=5`。
- `Message`（L259-301）: `Command { seq: u64, cmd }`（L272-277）・`Reply { seq, result }`（L280-285）・`BlockIn { seq }`（L287-290）・`BlockDone { seq, result }`（L292-297）・`Shutdown`（L298-300）。**いずれの variant もインスタンス識別子を持たない** — `Command` のドキュメントが明示する（L303-308: "no variant carries a `PluginHandle` — a runner child is a singleton instance"）。
- `Command`（L316-348）: `Scan`/`Initialize`/`Activate{config}`/`StartProcessing`/`StopProcessing`/`Deactivate`/`SaveState`/`RestoreState{bytes}`/`OpenGui{parent}`/`CloseGui` の 10 動詞 — codec tag は L892-935 の `push_u8(0..9)` で密に連番。**handle を入れる位置の候補**: (i) `Message::Command`/`BlockIn`/`BlockDone` のペイロード先頭に `instance` フィールド（envelope 化）、(ii) 各 `Command` variant への per-verb フィールド、（iii) kind バイト側での子/インスタンススコープ分割。`Scan` は子スコープ（インスタンス非依存）なので scope 表現の設計が必要。
- `Reply` の相関は `seq` のみ（L280-285）— handle なし。`BlockIn`/`BlockDone` の doorbell も `seq` のみ（L286-297）。
- decode は untrusted 入力の検証境界 — 宣言長・reserved フィールド・count 上限を `WIRE_MAX_*` で拒否（L51-61・L780-827 の trailing-bytes 検査を含む）。`decode_message` は任意の `u16` version を受理し、等値判定は proxy 側（L29-37）。
- `WIRE_VERSION: u16 = 1`（L88）。`WIRE_MAX_*` 定数は L90-114（`WIRE_MAX_FRAMES=8192`・`WIRE_MAX_CHANNELS=16`・`WIRE_MAX_EVENTS=1024`・`WIRE_MAX_PAYLOAD=16MiB`・`WIRE_MAX_STRING_BYTES=4096`・`WIRE_MAX_ELEMENTS=1024`・`WIRE_MAX_STATE_BYTES=16MiB`）— **全てメッセージ/フレーム単位の上限であり、子あたり・インスタンスあたりの上限語彙は存在しない**。

#### seq/correlation 分割（`crates/ruviform-plugin-host/src/l1/data.rs`・`l1/mod.rs`）

- コマンド seq と block seq は `u64` 空間を**フラグビットで分割**: コマンド seq は `L1Instance::next_seq`（`AtomicU64`・0 起算、`l1/mod.rs` L1062・L1127 以降の `execute` で採番）、block seq は `BLOCK_SEQ_FLAG = 1 << 63`（`data.rs` L83）を立てる。reader はフラグビットのみで demux（`data.rs` L6-18 モジュール文書・`l1/mod.rs` L33）。
- `Inner.waiters: HashMap<u64, Waiter>`（`l1/mod.rs` L214）は seq キーのみ — **インスタンス成分を含まない**（1 子 = 1 インスタンスなので不要だった）。`Inner.block_in_flight: HashSet<u64>`（L226）も同様。
- `RtLeg::next_block` は per-instance の単調カウンタ（`data.rs` L430・L474 初期値 0・L630-631 採番）— インスタンス毎に独立しており、`next_block` の値域は `seq % D` の slot 選択と instance 自身の segment にのみ意味を持つ。
- **帰結**: 現行 demux は「seq 空間が子プロセス内で一意」であることを暗黙に利用している。bundle で (a) 子ワイド seq 空間を維持するか (b) インスタンス毎 seq 空間＋`instance` echo で demux するかが設計分岐（下記軸 5）。

#### host 側 L1 プロキシ（`crates/ruviform-plugin-host/src/l1/`）

- モジュール文書が singleton 前提を明示: L1-4 "one child process per instance"・L28-37 のスレッド表 "per instance = 1 child + 2 threads"。
- `SpawnSpec`（L138-172）— `descriptor`/`program`/`args`/`lookahead_blocks`/`command_timeout`。**pool キー・隔離レベル・bundle 指示のフィールドは存在しない**。
- `Shared`（L250-284）: `inner: Mutex<Inner>`・`rt_state: Arc<AtomicU8>`（lifecycle mirror・RT leg の lock-free 読み取り）・`child: OnceLock<Mutex<Child>>`・`notify_tx`・`result_tx: Mutex<ruviform_rt::Producer<u64>>`。**child ハンドルとインスタンス状態（waiters/mirror/staging/terminal/notify）が 1 構造体に同居** — bundle では「子共有部」（child・stdin/stdout・I/O スレッド・waiters 基底）と「インスタンス部」（mirror・staging・notify・seq 簿記）の分離が必要。
- `converge`（L375-426）: 初回 caller が terminal を確定 → `Failed` なら mirror を `LifecycleState::Failed` へ・waiters を `Err(Ipc)` で wake・通知を一度だけ queue・`Shared.child` 経由で子を force-kill。**convergence は child-スコープの副作用（kill/reap）と instance-スコープの状態遷移を一体で行う** — bundle で「子死亡 → 全インスタンス Failed」「インスタンス起因プロトコル違反 → そのインスタンスのみ？」の分解再設計が要る地点。
- `L1Instance::spawn`（L976-1088）: 1 回の呼出しが shm segment create（L983）→ SPSC ch pair（L987-990）→ `ProcCommand` に `--ruviform-plugin-id` + `--ruviform-shm-key` を付けて spawn（L991-1002）→ Shared 構築（L1005-1020）→ driver spawn（L1023-1031）→ reader spawn（L1040-1053）→ handshake（L1079-1087）まで一気通貫。**`L1Instance` が子・SHM・スレッド・RT leg 全てを所有する一体型**。
- handshake（L1092-1113）: `Hello.version == WIRE_VERSION` 厳密一致 → `Command::Scan` → `Scanned` が `spec.descriptor` と field-equality（`Descriptor::matches`）。**scan 検証は spawn 毎に行われ、descriptor は「この子が提供する plugin_id の正当性」を担保している** — bundle で複数 plugin_id を 1 子が serve する場合、この照合の意味付けを再確認する必要（spec 単位照合は初回 spawn 時に一度で済むか、インスタンス毎の plugin_id 正当性を `CreateInstance` で再確認するか）。
- `execute`（L1127-）: `Failed` instance → `Err(HostError::Failed)`、`OpenGui`/`CloseGui` → `Err(Unsupported)`、waiter 登録 → driver_tx 経由で送信 → `command_timeout` 内に `Reply` 待ち。`destroy`（L1296-）は `Shutdown` + grace + kill watchdog + reap。
- `prepare_endpoint`（L1469-1559）: **インスタンス単位で `RtLeg` を detach** — 取り出すのは per-instance の RT leg + mirror atomic + arm/return チャネルのみで、子・SHM・mutex・notify receiver は残る（`endpoint.rs` L456-623: "`L1Endpoint` owns no `L1Instance`, child process, SHM segment, mutex state, I/O thread, or notification receiver"）。**この分離が「1 子 N インスタンスでも RT leg はインスタンス毎」という推奨構造の実証的裏付け**。
- `try_recv_notification`（L1741-）: instance 単位の `ChildExit` を一度だけ返す。

#### host 側 composite/公開面（`composite.rs`・`command.rs`・`endpoint.rs`）

- `OutOfProcessSpec`（L68-104）: `descriptor`/`program`/`args`/`lookahead_blocks`/`command_timeout` — **公開 spawn 宣言に bundle/pool/isolation の語彙なし**。`Route::OutOfProcess(SpawnSpec)`（L160-167）のドキュメントも "every `Instantiate` spawns one child process per the spec"。
- `PluginHost::instantiate`（L822-842）: route 分岐の L1 側は `Slot::OutOfProcess(Box::new(L1Instance::spawn(spec)?))` — **Instantiate 1 回 = spawn 1 回の写像がここにある**。bundle ではこの写像が「子を pool から引く or 新規 spawn → 子に `CreateInstance`」へ変わる地点。
- `register_out_of_process`（L311-）: route を `plugin_id` で claim — registration-time 選択は変わらないため、**bundle グルーピングの自然な単位は「登録 route（`plugin_id`）」or「`OutOfProcessSpec` の (program, args) 同一性」**。
- `HostNotification::InstanceFailed { handle, exit }`（L144-156）: 通知語彙は既にインスタンス単位で、observer には `PluginHandle` が返る。`try_recv_notification`（L771-796）は全 slot を走査して per-instance `ChildExit` を拾う — **bundle crash でも「影響を受けた各インスタンスに 1 通知ずつ」ならこの機構のまま拡張できる**。ただし「N インスタンスが同一 `ChildExit` で同時 Failed」という bundle 相関の情報は現語彙では表現できない（新 variant が要るかは設計論点）。
- `command.rs` L35-101: `HostCommand` の dispatch 動詞は全て `PluginHandle` 修飾済み、`Instantiate` は `plugin_id` のみ、`HostReply::Instantiated { handle }` が mint。`to_wire`（composite.rs L992-1000）で handle を drop するのが現行の singleton 反映 — **bundle で wire handle を入れるならこの翻訳層が「`PluginHandle` → wire `instance`」の写像を担う**。
- 公開 handle は `PluginHandle { slot: u32, generation: u16 }` — composite の unified slot table（L243-248）が mint。wire handle と同一物にするか、子ローカル mint にするかは設計論点（公開 handle の slot は host プロセス内で一意・世代つき・0 起算密配置、wire handle は子プロセス内で一意で足りる）。

#### shm データ面（`l1/shm.rs`・`export/shm.rs`）

- `l1/shm.rs` L1-11: 「segment is created and owned by the proxy; the child only attaches (`open`) by the `--ruviform-shm-key` `os_id` and never creates or unlinks」。**segment は per-`L1Instance`**（`spawn` L983 でインスタンス毎に create）。
- ヘッダ（L26-47）: `magic`・`wire_version`（厳密一致）・`in_depth`/`out_depth`・`slot_bytes`・`in_off`/`out_off` — 固定レイアウトは `wire::shm` が正本。**segment 内にインスタンス識別情報はない**（不要だった）。
- slot 選択は `seq % depth`・strict alternation・片側 single-writer/single-reader — SAFETY 論証は「この segment のこの slot を触るのは 1 側のみ」に依存。**per-instance segment 維持ならこの論証は無変更**（推奨方向・軸 6）。
- runner は `--ruviform-shm-key` を **argv で 1 個だけ**受け取り attach（export `lib.rs` L24-40・`RunnerArgs` L311-381: `--ruviform-shm-key` 必須・`--ruviform-plugin-id` は factory 複数時に必須）。**argv は「1 子 1 segment」前提 — bundle では per-instance segment キーを wire 経由で渡す新動詞が必要**（argv に N 個のキーを列挙することはできない: インスタンスは実行中に動的に増減する）。

#### runner 側（`crates/ruviform-plugin-export/src/`）

- モジュール文書 L14-15: "One child process hosts exactly one plugin instance (per-`Instantiate` spawn)"。
- `run(factories)`（L124-188）: argv parse → `Hello` 送出（L145-156）→ **1 segment attach**（L158-164）→ **1 factory.create()**（L167-171）→ `Runner` 構築（L173-183）→ `serve` ループ（L187）。`Runner` フィールド（L384-400）: `instance: &'a mut dyn ErasedPlugin`・`lifecycle: Lifecycle`・`activated_max_frames: Option<u32>`・`segment: &'a mut shm::Segment`・staging — **全てシングルトン**。
- `serve`（L196-285）: `Command{seq,cmd}` → `runner.dispatch(cmd)` → `Reply{seq,result}`; `BlockIn{seq}` → `runner.process_block(seq)` → `BlockDone` か flagged `Reply{seq,Err}`。**dispatch に instance 解決の段がない** — bundle では「`instance` で registry lookup → 対象 entry の instance/lifecycle/segment に dispatch」が挿入点。
- BlockIn の seq echo 契約（L234-241 コメント）: 子は doorbell seq を verbatim echo — 子側分割作業は不要と明記。bundle では「`instance` も verbatim echo」に拡張される見込み。
- `Scan`（L412-419）は `self.factories` を列挙 — **子スコープのまま**。
- 終了規約（L42-56）: `Shutdown` または stdin EOF で normalize → exit 0。bundle で「最後のインスタンス破棄時に子が自発 exit するか、host が `Shutdown` を送るか」は設計論点（軸 7）。

#### 検証ハーネス（tests + fixture）

- `l1-test-worker`（`src/bin/l1-test-worker.rs` L1-31）: singleton descriptor 1 個を serve する wire-conformant fixture。argv knob は**全て子プロセス単位**（`--crash-after-msgs`/`--hang-after-msgs`/`--reject-blocks-after-msgs`/`--stop-reading-after-msgs`/`--leave-heir-after-msgs`/`--hello-version`/`--fail-stop-processing`）— **「子が死ぬ/応答しない」シナリオはそのまま bundle の子単位故障注入に転用可能**だが、「インスタンス A は生きてインスタンス B だけ失敗」の注入語彙はない。
- `l1_proxy_e2e.rs`（L16-18）: `#[path]` で `src/l1/` をインクルード — `pub(crate)` のまま内部 E2E を書く慣行。`spec()`/`fast_timeout_spec()`（L59-74）で `SpawnSpec` を構成。**同一 worker を 2 回 spawn する検証は現形でも書ける**（複数 `L1Instance`）が、「1 子 N インスタンス」の wire 形は v2 codec が無いと書けない。
- `l1_runner_e2e.rs` `MiniHost`（L60-120）: segment create → argv 2 引数で spawn → stdio wire を話す最小 host。**多インスタンス化するには「複数 segment create + CreateInstance wire 動詞」が必要**で、fixture 構造そのものは流用可能。
- runner fixture 側は `GainFactory`/`PanicFactory`/`UnderwriteFactory` の複数 factory を `run(vec![...])` に渡す構成済み（`l1_runner_e2e.rs` L33-35 の 3 ID）— **1 子が複数 factory を持ち `Scan` が複数 descriptor を返す形は既に存在する**。bundle で `CreateInstance { plugin_id }` が factory 選択を担えば、異種 plugin_id の混在 bundle も fixture で表現可能。

#### 隣接 Issue との関係（#118/#120/#122/#123）

- `docs/designs/issue-122-shared-platform-process-ipc-eval.md` L154/L170-172/L187: 「#117（bundle 子）・#118（video/NONRT データ面: shm レイアウト見直し）・#120（共有リーパー/io 多重化: `try_wait` poll → poll/epoll/IOCP イベント駆動）が抽出候補面を再設計する」と明記し、platform/process/ipc 共通クレート抽出を延期継続と判定。**#117 は spawn モデル自体を変え得る Issue として位置付け済み**。逆に言えば、bundle で I/O スレッドが「子単位」に集約されるなら #120 の poll/epoll 多重化の恩恵は bundle 側のコスト構造にも効く — 設計は #120 と競合しない形（子単位 I/O スレッド pair を残す or 後続で多重化）を選ぶ余地がある。
- #123（OPEN）の残件（`runner_program()` 解決・frame-I/O dedup）と bundle の wire 変更は別々に進められるが、**frame-I/O dedup は bundle で「host/driver/reader/runner の 4 系統が instance 付き frame を読み書きする」変更量を直に増減させる** — 先行実施の旨味あり。

## Alternatives and Comparison

### 軸 1: `Failed` の粒度（子プロセス単位 vs インスタンス単位）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) 子スコープ terminal — 子死亡・pipe/shm プロトコル違反は **bundle 内全インスタンスを `Failed`** へ収束。インスタンス単位の `Failed` は存在しない | pipe/stdio は子単位の共有資源 — frame が壊れるとストリーム全体が信用不能（どのインスタンス宛かすら分からない）。converge/kill/reap の現行構造をほぼ維持できる。`Failed` の意味が「この論理インスタンスは二度と応答しない」で一貫 | 1 インスタンスの plugin defect が全インスタンスを巻き込む — per-instance spawn と比較して隔離粒度が**退化**（issue が「見直し」対象に挙げる点）。ただし plugin defect は現行でも `Err(PluginFailed)` であって `Failed` ではない（panic は catch 済み）ため、実害は「子プロセス自体が死ぬ/壊れる」ケースに限定される | **yes** |
| (b) ハイブリッド — transport/プロトコル破壊は子スコープ `Failed`、検出可能なインスタンス起因違反（例: 特定 instance 宛 reply の型不整合）はそのインスタンスだけ `Failed` | 「他インスタンスは生き残れる」境界を最大化 | 判定不能なケースが残る（reply の instance フィールド自体が壊れていたら？）。`Failed` が 2 種類の意味を持ち observer 契約が複雑化。**pipe が無事という仮定の上にしか instance-scope 判定は載らない**ため、(a) で十分な範囲を過剰設計するリスク | maybe — (a) を土台に限定範囲だけ |
| (c) 完全インスタンススコープ — 子死亡でも他インスタンスは `Failed` にならない | — | **物理的に不可能**: 子が死ねば全インスタンスのプラグインコードも死ぬ。却下 | no |

推奨方向: **(a) 子スコープ `Failed`（terminal は子単位・bundle 全インスタンスを `Failed` 収束）**。`converge` の構造は「子の terminal 条件」と「各インスタンスの mirror/waiter/notify 収束」の分離に再編するだけでほぼ写せる。`Err(HostError)` 系の per-command/per-instance 拒否（`PluginFailed`/`Lifecycle`/`Failed` 後の後続コマンド）は現行どおりインスタンス単位を維持 — 「`Failed`」という terminal 状態だけが子単位になる。

### 軸 2: bundle グルーピング境界（どの単位で子を共有するか）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) **登録 route（`plugin_id`/`OutOfProcessSpec`）単位の pool** — 同一 route の `Instantiate` が同一子に入る | `OutOfProcessSpec` は既に (program, args, descriptor) の同一性を持つ自然なグルーピングキー。公開 API 追加最小（spec への `isolation`/pool フラグ追加のみで既定挙動を変えられる）。異種 plugin_id 混在を防げば handshake の descriptor 照合も現行意味のまま | route 跨ぎの bundle（同一 runner binary で複数 plugin_id）が表現できない | **yes**（既定形） |
| (b) 明示 pool キー — `OutOfProcessSpec` に `bundle_key`/`pool` を追加し、キー一致 route 同士で子共有 | 「同一 runner binary が複数 factory を内蔵」する場合（現行 `run(vec![Gain, Panic, Underwrite])` 型）に異種 plugin_id の混在 bundle が可能。柔軟性最大 | `CreateInstance` が `plugin_id` を payload に持つ前提が必須。key の同一性規則（program+args 一致を要求するか）の設計が要る。API 面が増える | **yes** — (a) の拡張として採用候補（両立可: キー未指定 = route 単位 pool） |
| (c) `PluginHost` 単位の大域 pool — host 内 L1 インスタンスを全部 1 子へ | 実装が最も単純 | **物理的に不可能**: 異なる `program`（異種 runner binary）は同じ子に入れられない。(program,args) で自然に分割されるので「大域」は実質 (b) の退化形 | no |
| (d) `HostCommand::Instantiate` への isolation hint 追加 | インスタンス毎に隔離強度を選べる | 公開コマンド面の変更（`plugin_id` のみの現形を壊す）。登録時選択の設計原則（route claim）とずれる — 隔離は route 属性として spec に置く方が整合 | maybe — spec 側 field で代替可 |

推奨方向: **`OutOfProcessSpec` に bundle/isolation 制御を追加** — 既定値次第だが「route 単位 pool = bundle が自然な既定」「明示キーで route 跨ぎも可能に」「dedicated/per-instance も選択可」の 3 形を 1 フィールド（例: `isolation: Isolation::{Dedicated, Bundled(key?)}` あるいは `bundle: Option<BundleKey>`）で表す案が整合的。確定は design/ADR。

### 軸 3: wire handle の型・allocator

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) `u32`、host が mint — `CreateInstance{instance, plugin_id, shm_key, …}` で送り、子は registry key として受理 | per-child 一意で十分（child が死ねば空間ごと消える — 世代は不要）。`PluginHandle.slot: u32` と同幅で `to_wire` 翻訳が自然。子側 registry は `HashMap<u32, _>` or 密 Vec で bounded | host 側が「子内 handle 空間」を別途管理（`PluginHandle` とは別名空間）— とはいえ slot table に wire handle を記録するだけで済む | **yes** |
| (b) `u64`、host mint | 枯渇不可能 | per-child で u32 以上のインスタンス数は非現実（`WIRE_MAX_INSTANCES` 上限を置くなら尚更）。8 バイトは frame 毎の無駄 | no |
| (c) 公開 `PluginHandle{slot,generation}` をそのまま wire に（48bit or 64bit packing） | 写像不要 | 公開 handle の内部表現を wire 契約に固定してしまう — 層間結合の悪化。generation は子内では無意味。`PluginHandle` は公開型のまま変えず、wire handle は別物として写像する方が境界として正しい | no |
| (d) 子が mint（`CreateInstance` の reply で handle 返却） | host が空間管理しなくて済む | reply 相関が「instance 未確定のコマンド」になる — CreateInstance 以外の全コマンドが instance 修飾を要求するプロトコルで、唯一 instance を持たないコマンドが混ざる。host 側で「plugin_id → 期待 handle」の帳簿を持つ以上、host mint の方が直線的 | maybe |

推奨方向: **`u32`・host mint**（wire `instance` は子ローカル名空間。公開 `PluginHandle` とは `to_wire` 層で写像）。

### 軸 4: handle の wire 上の置き場所

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) 各フレームペイロード先頭に `instance: u32`（`Command`/`BlockIn`/`BlockDone`/`Reply` 全部） — envelope 化 | 子側が instance を持たない状態でも echo できる（`Reply`/`BlockDone` は受けた値をそのまま返す）。demux は (instance, seq) で完全に対称。`Scan` 等の子スコープ動詞は `instance = u32::MAX` 等の sentinel or variant 側で区別可能 | 全 kind のペイロード変更 — 非互換変更（bump 前提で問題ない）。`Reply` にも載せるかは軸 5 依存 | **yes**（Command/BlockIn/BlockDone は必須・Reply は要検討だが対称性で載せる案が自然） |
| (b) `Command` variant 内部に per-verb フィールド（`Initialize{instance}` …） | payload 構造が動詞毎に自立 | `Scan`/`CreateInstance` は instance を持たないので例外規則が要る。codec の per-variant 分岐が増え decode/encode の対称が崩れる。`BlockIn`/`BlockDone` の handle 表現が別途必要 | maybe |
| (c) kind バイトで scope 分割（instance 宛 kind と子宛 kind を別番号） | kind 空間はまだ余裕あり | 動詞数×スコープで kind 数が倍化 — `Command` tag との二重管理。envelope 1 フィールドの方が明快 | no |

推奨方向: **envelope（ペイロード先頭 `instance: u32`）**。`BlockIn`/`BlockDone`/`Reply` も同位置に置き、子は verbatim echo。子スコープ動詞の表現方法（sentinel `u32::MAX` or `Scan` を引き続き instance 必須扱いせず別扱い or 新 kind）は設計で確定。

### 軸 5: seq/correlation 空間の分離

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) **コマンド seq は子ワイド単一空間**（proxy 側 1 カウンタ・waiters は seq キーのみ）+ **block seq は per-instance**（各 `RtLeg::next_block` が独立・`BlockIn`/`BlockDone` に instance 付きで demux） | `Reply` に instance を載せなくても seq で一意に waiter 引ける（load 最小・wire 変更最小）。block 側は `seq % D` が instance 自身の segment を指すので per-instance 空間維持が自然 — `RtLeg` の既存採番をそのまま使える | `Reply` demux で「この seq はどの instance のコマンドか」を waiters 側の記録から引く必要（map は (seq→waiter) のままで良い — waiter が自分の instance を知っている）。flagged `Reply{seq,Err}`（BlockIn 拒否）は instance 情報が要る — `Reply` に instance を載せる or flagged reply を BlockDone エラー表現に変える等の細部設計が要る | **yes** |
| (b) per-instance seq 空間 — `(instance, seq)` が真の相関キー。`Reply`/`BlockDone`/`Reply` に instance echo 必須 | 概念対称・子側デバッグ容易 | `Reply` に instance 必須（ペイロード拡大）。proxy の waiters キーが `(instance, seq)` 複合に — 変更面は小さいが wire 対称性のために全フレームに instance を載せる必要 | maybe |
| (c) seq に instance 上位ビットを埋め込む（`u64` の上位 n bit を instance index に） | wire 追加分ゼロ | seq の意味論が混ざり `BLOCK_SEQ_FLAG` との直交性が壊れる。hack — 明示フィールドの方が正直 | no |

推奨方向: **(a) コマンド seq 子ワイド + block seq per-instance**。現行 `Inner.waiters`/`next_seq` は per-child 共有部へ、`RtLeg::next_block` は per-instance に残る自然な分割。`BlockIn` 拒否の flagged `Reply` 経路だけ instance echo の要否を設計で確定（`Reply` に instance を載せる案が最も対称）。

### 軸 6: RT/SHM 面（インスタンス毎 segment か共有 segment か）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) **per-instance segment + per-instance `RtLeg`** — bundle は制御面（子・pipe・I/O スレッド）のみ共有し、データ面はインスタンス毎 | `wire::shm` 固定レイアウト・strict alternation・単一 writer/reader の SAFETY 論証・`prepare_endpoint` の leg detach・`Staging` — **RT 面の全不変条件が無変更**。depth/slot_size をインスタンス毎に変えられる（lookahead 差も許容）。`l1-test-worker`/`MiniHost` の shm コードが流用可 | 子が attach する segment 数が増える — attach 失敗・管理簿記が増える。子が attach すべき key を wire で渡す新動詞が要（軸 7） | **yes** |
| (b) 1 共有 segment を instance 分割（slot range を handle 毎に割当） | segment 数が 1 | **構造的に困難**: レイアウトが instance 数で変わり、子 spawn 時に N が未決（動的増減と矛盾）。strict alternation の slot 計算・depth 差異・SAFETY 論証（同 segment 内の複数 writer 領域分離）を全部引き受ける。`wire::shm` レイアウト自体の改訂（#118 と同種の大改訂） | no（本 Issue 範囲外級） |
| (c) 共有 segment + instance を slot 内フィールドで多重化 | — | (b) と同じ問題 + 同時書込みの論証が破綻 | no |

runner 側の segment 発見経路（(a) 採用時の派生論点）:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `CreateInstance` コマンドの payload に `shm_key`（+ depth 情報 or spec 固定）を載せる | 動的生成と整合・argv 規約を変えずに済むかは要検討（argv から shm-key を外すなら v2 argv は `<runner>` 単体 or `--ruviform-plugin-id` のみ。plugin-id も per-instance へ移るので argv からも外れる見込み） | argv 規約変更（v2 bump と同時なので許容範囲） | **yes** |
| `CreateInstance` reply が key を持つ（子が create） | — | 所有権規則（host が create/own・child は attach のみ）を反転させる — 現行設計の ownership 原則と矛盾 | no |
| argv に N key 列挙 | — | 動的増減と両立しない | no |

推奨方向: **(a) per-instance segment + `CreateInstance` が `shm_key` を運ぶ**。

### 軸 7: 新 wire 動詞（CreateInstance/DestroyInstance の要否と形）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) `CreateInstance{instance, plugin_id, shm_key, in_depth, out_depth?}` + `DestroyInstance{instance}` を `Command` に追加 | `Instantiate`/`Destroy` の写像を「spawn+`CreateInstance`」「`DestroyInstance`（+全滅時 `Shutdown`+reap）」に書き換えるだけで L0 語彙との対応が維持される。`plugin_id` を持たせれば runner 内蔵複数 factory からの選択も可能（現行 `--ruviform-plugin-id` の機能を wire に移す形で自然）。`shm_key` + depth で segment attach に必要な情報を完結 | 新 tag 追加は非互換変更 — bump 前提なので問題なし。`in_depth`/`out_depth` は spec 共有値で省略可（子が固定値を期待するなら）か payload 化かは設計細部 | **yes** |
| (b) 既存動詞の再利用（例: `Initialize{instance,…}` が「無ければ作る」） | 動詞数を増やさない | 語彙写像が曖昧化・`Instantiate`/`Destroy` の意味が lifecycle 動詞に潜む — design の明示性を損なう | no |
| (c) argv 駆動（起動時に N 固定でキーと plugin_id を列挙） | wire 変更最小 | 動的インスタンス増減を排除 — `Instantiate`/`Destroy` が動的な以上この形は要求を満たさない | no |

`Shutdown` の意味確定: 「子が空（全 instance destroy 済み）になった時、子を自発 exit させるか、host が明示 `Shutdown` を送るか」— 現行は host 送信（`destroy` L1296-）。**host が「最後の instance を destroy した後に `Shutdown`」を送る形が現行语义の自然な延長**で推奨方向（子の自律終了は新語彙を要し不要と見積もる）。

### 軸 8: `WIRE_VERSION` 取り扱い

- 確定事項: 上記全ての wire 変更（envelope `instance`・新 tag・argv 規約変更）は**非互換変更**であり、Issue #119 方針により `WIRE_VERSION` **1→2 bump**。交渉機構・旧版 decode は追加しない。host/runner は同一リビジョンで更新。`Hello.capabilities` は予約のまま（bundle 化を capability bit で表現する選択肢もあるが、「常時有効なプロトコル形状」なので capability 経由ではなく bump で行う方が #119 方針と整合 — version は「通信契約の世代」であり opt-in 機構ではない）。

### 軸 9: 上限・bounds

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) `WIRE_MAX_*` は per-message 維持 + **新 `WIRE_MAX_INSTANCES`**（子あたり handle 空間上限。defined rejection: 超過 `CreateInstance` は `Err(HostError)`） | 現行上限語彙は全てメッセージ単位で「untrusted 入力の bound」— instance 数は wire でなく子側 registry 容量の問題だが、子が untrusted host からの連続 `CreateInstance` で無制限に instance を持たないよう wire 側上限 or 子側固定上限が要る（proxy/runner は同一ビルド前提なので実質「契約としての上限」） | 上限値の選定が必要（実用上数十〜数百程度か） | **yes** |
| (b) 上限なし（`u32` 枯渇のみ拒否） | 実装最小 | 子が instance registry を無制限に増やす — untrusted peer 防御の原則と不整合 | no |

推奨方向: **`WIRE_MAX_INSTANCES`（仮名）相当の wire/子側 registry 上限 + 超過は defined `Err`**。`u32` handle 空間の reuse 規則（destroyed handle の再利用を許すか・単調のみか）も設計で確定 — per-child 空間なので単調増加で十分と見積もる（child 死亡で空間ごと消えるため generation 不要）。

### 軸 10: 移行形（per-instance spawn を隔離オプションとして残すか）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) **dedicated（現行形）を隔離オプションとして残す** — `OutOfProcessSpec` の isolation 指定で per-instance spawn / bundle を選択 | 「クラッシュ隔離を取るかコストを取るか」の利用者選択肢を保持（bundle は隔離粒度の退化を伴うため、残さないと単なる機能退化）。`SpawnSpec`/`L1Instance` の dedicated 経路をそのまま温存できる — bundle 経路の初期実装リスク時の fallback にもなる | 2 経路の維持コスト（ただし dedicated は既存コードそのまま） | **yes** |
| (b) bundle のみに統一 | コード経路単純化 | 隔離粒度の退化を全利用者に強制 — L1 の存在意義（隔離）を損なう | no |

推奨方向: **dedicated を残す**（軸 2 の isolation 指定と一体の論点）。

### 軸 11: `HostNotification::InstanceFailed` の scope と observer 契約

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) **bundle crash → 影響を受けた各インスタンスに `InstanceFailed{handle, exit}` を 1 つずつ** — `ChildExit` は全て同値 | 公開通知契約（per-instance `handle`・一度だけ・`exit` は子の終了状態）を不変に保てる — observer は「自分の handle が Failed になった」事実だけ見ればよい。`#[non_exhaustive]` なので後で bundle 相関 variant を足しても後方互換 | 「N 個の通知が同一 `exit` で連続する」ことの意味（同一子の死）は推測に任せる形になる — doc で明記すれば許容範囲 | **yes** |
| (b) 新 `ChildFailed`/`BundleFailed` variant（bundle id/影響 handle 集合） + per-instance mirror は別途 Failed | bundle 相関を明示 — observer が「1 子の死」を 1 イベントとして処理可能 | 通知意味が増え observer の分岐が増える。(a) で実用上足りるはず — `#[non_exhaustive]` により後から追加は可能 | maybe（後続拡張として） |

推奨方向: **(a) per-instance 通知を維持** — `try_recv_notification` の slot 走査は「bundle 内全 live instance が terminal=Failed に収束 → 各々 1 通知」という形で自然に拡張可能（`converge` が子死亡を受けて全インスタンスの notify を queue する形）。

### 軸 12: host 側構造（`L1Instance` の分解）

- `Shared`/`Inner` は「子共有部」（`child`・stdin/stdout I/O スレッド・`waiters`（子ワイド seq の場合）・`notify_tx`・`result_tx`）と「インスタンス部」（`mirror`/`rt_state`・`staging`・`block_in_flight`・`notify`・terminal/インスタンス毎 seq 簿記）に分離する再編が要る — **観測上、現行構造は 1:1 対応なので分離線は明確に引ける**（child 関連フィールドは `Shared` の内側でも `child: OnceLock`・`result_tx` として既に物理分離されており、分離は構造変更として自然）。
- `driver`/`reader` スレッドは子単位で 1 pair に集約（spawn コスト削減の主旨と一致）。`submit`/`result` SPSC は per-instance（`RtLeg` は per-instance）— driver は「(instance, seq) 付き submit」を受けて「instance → segment view」で書き分ける形。`result_tx` が 1 つの `Producer` に集約されるか、instance 毎 `Producer` を持つかは設計細部（`Shared.result_tx` は現在 1 つ — bundle では per-instance `Producer` が `RtLeg` 帰属として自然）。

## Tentative Conclusions and Hypotheses

- **confirmed**: 現行コードの singleton 前提は「`Command` variant が handle を持たない」ことを含め**意図的設計**（`wire.rs` L303-308 明記・IR L338-343・design L174 で将来のプロトコル改訂として予告済み）— bundle 化は「発見した設計破綻」ではなく「予告されたプロトコル改訂の実行」として位置付けられる
- **confirmed**: `Failed` は現行「1 子死亡 → 1 インスタンス Failed」の 1:1 写像を持ち、bundle では **子死亡/プロトコル破壊 = 子スコープ terminal → bundle 全インスタンス `Failed`** が物理的に唯一正直な粒度（pipe/プロセスは共有資源 — 部分 Failed は表現不能）。per-instance `Err(HostError)` 拒否は現行のまま維持可能
- **confirmed**: wire 変更は全て非互換 — **Issue #119 方針どおり `WIRE_VERSION` 1→2 bump**（交渉なし・旧版共存なし）。順序付けの懸念は #119 確定により解消済み
- **confirmed**: RT/データ面は **per-instance segment + per-instance `RtLeg` を維持**するのが SAFETY・strict-alternation・endpoint detach の不変条件を一切変えない唯一の形 — bundle の共有対象は「子プロセス・制御 pipe・I/O スレッド pair」に限定される。runner への per-instance segment 通知は `CreateInstance` payload の `shm_key` が自然
- **confirmed**: グルーピングの自然な境界は **`OutOfProcessSpec`（登録 route）** — program/args 同一性が pool キーとして機能し、公開面への追加は spec の isolation/bundle フィールド 1 個で済む。明示 pool キーによる route 跨ぎ bundle は同じ枠組みの拡張として表現可能
- **confirmed**: seq は **コマンド = 子ワイド単一空間・block = per-instance** の分割が現行構造（waiters/seq-map は子共有部へ、`RtLeg::next_block` は per-instance 残留）に最も整合
- **confirmed**: wire handle は **`u32`・host mint** が `PluginHandle` 写像・bounded registry・子ローカル性の三点で最適。公開 `PluginHandle`（slot+generation）とは別名空間 — `to_wire` 層で写像
- **confirmed**: 新動詞 **`CreateInstance`/`DestroyInstance`** が `Instantiate`/`Destroy` の語彙写像を保つ最小の wire 追加 — `CreateInstance` は `{instance, plugin_id, shm_key, (depth)}` を運び、`Scan`（子スコープ）と instance 宛動詞の scope 表現（envelope sentinel or 別扱い）は設計で確定
- **confirmed**: per-instance spawn（dedicated）は **隔離オプションとして残す** — bundle は隔離粒度の退化を伴うので、残さないと機能退化になる
- **hypothesis**: `HostNotification::InstanceFailed` は per-instance × bundle 内全 instance への fan-out で現契約のまま拡張可能 — 新 `BundleFailed` variant は `#[non_exhaustive]` により後から追加可能なので初期実装では不要と見積もる
- **hypothesis**: runner 側は `Runner` → `HashMap<u32, InstanceEntry>` registry 化（instance/lifecycle/segment/staging を entry 化）+ `serve` に instance 解決段の挿入で構造変更が閉じる見込み — 複数 factory の `run(vec![...])` は既に存在するため `CreateInstance{plugin_id}` で factory 選択も自然に載る
- **hypothesis**: `l1-test-worker` の argv knob は子スコープ故障注入として転用可能、`MiniHost`/`l1_proxy_e2e` の harness 構造も流用可能 — 新規に要るのは「bundle 子 fixture（複数インスタンス serve）」と「instance-scoped 故障注入（特定 instance だけ hang/reject）」の語彙

## Open Questions

> 全て design/ADR フェーズが確定する論点 — architecture 契約レベル（ADR 候補）と実装設計レベルを分記。

### ADR 候補（architecture 契約 — 変更コストが大きい語彙）

- **`Failed` 粒度の確定**: 子スコープ terminal（推奨 (a)）を契約として確定するか、限定範囲の instance-scoped `Failed`（(b)）を認めるか — **`HostNotification`/`LifecycleState`/`Failed` の公開意味に関わるため ADR 記録が要る**
- **bundle/dedicated の選択面 API 形**: `OutOfProcessSpec` への isolation フィールド形（`Isolation::{Dedicated, Bundled}` enum vs `bundle_key: Option<..>`）と既定値 — 公開 API 追加なので ADR or 少なくとも design 確定記録が要る
- **`InstanceFailed` 通知 semantics**: per-instance fan-out で確定するか、bundle 相関 variant を初期から入れるか

### design 確定対象（実装設計 — ADR ほどの重さは不要）

- `instance` envelope の sentinel/scope 表現（`Scan`/`CreateInstance`/`Shutdown` 等の子スコープ動詞を `u32::MAX` or variant 側で区別するか）
- `Reply`/`BlockDone` に instance echo を載せるか（(a) seq 子ワイドなら Reply 不要・ただし flagged `Reply{seq,Err}` = BlockIn 拒否経路の demux 要確認 — BlockIn reject reply に instance が要るか、BlockDone 系エラーを別表現にするか）
- `CreateInstance` payload の確定形（`plugin_id` を String で運ぶか・depth を payload 化 or spec 共有前提か・`shm_key` の String 表現幅上限 — `WIRE_MAX_STRING_BYTES` 既存上限で足りるか）
- 子の空時挙動（最後の `DestroyInstance` で host が `Shutdown` を送る vs 子が自発 exit — 推奨は host 送信の現行语义延長）
- `WIRE_MAX_INSTANCES`（仮）の値・handle reuse 規則（単調のみ vs 再利用）
- pool key の同一性規則（program+args 一致必須か）・pool の寿命（最後の instance destroy で子を畳むか keep-alive するか）
- `spawn` 時 handshake の再設計 — `Scan` は子 spawn 時 1 回か、`CreateInstance` 毎の plugin_id 正当性を別途照合するか（`Scan` は child-scope のままで、`CreateInstance{plugin_id}` が `Err(UnknownPluginId)` で reject される形が自然と見込む）
- dedicated 経路と bundle 経路で `l1/` 内部をどう共通化するか（`Shared` 分解後の両経路の共通部品化 — 無理に共通化せず dedicated を現行のまま残し bundle を別型にする案も）
- wire v2 でも dedicated route は wire v2 を話すか（host/runner 同一リビジョン配布なら v2 一択 — v1 互換は #119 により不要）
- GUI（`OpenGui`）: `GuiHandle` 空間は現状子単位 — bundle で instance 紐付けが要るか（runner registry が instance→gui を記録する形で十分か）
- `kill_child`（`#[doc(hidden)]` 検証 API）の bundle 版形 — handle 指定で所属子を kill（現行语义そのまま・子に紐づく全 instance が死ぬ）

## Promotion Candidates

- [x] spec-doc: [issue-117-bundle-child-process.md](../specs/issue-117-bundle-child-process.md)（**created — draft**。Issue 要求・軸別の選択肢・推奨方向・open 論点を DRAFT として記録。受入基準は「wire v2 の codec 契約・bundle 制御面・`Failed` 粒度・通知・上限・E2E」に分解）
- [ ] adr-doc: **`Failed` 粒度（子スコープ terminal）・bundle/dedicated 選択面の公開 API 形の 2 論点は ADR 記録を推奨** — 公開 notification/lifecycle 契約と公開 spec フィールドに関わるため、design 確定時に `docs/adr/issue-117-*` 系で記録するのが筋（#119 spec と同型の「方針確定を記録」パターン）
- [ ] design-doc: bundle 化の実装設計（`Shared` 分解・`L1Instance`/child 分離・runner registry・wire v2 codec 詳細）は design フェーズで `docs/designs/issue-117-*.md` として作成

## Sources and Relations

- Issue [#117](https://github.com/scarecrow173/ruviform/issues/117)（本探索対象 — bundle 子 1 子 N インスタンス）・Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)（L1 実装・CLOSED — bundle を non-goal として明示）・Issue [#119](https://github.com/scarecrow173/ruviform/issues/119)（wire バージョン方針・CLOSED — bump 前提を確定）・Issue [#118](https://github.com/scarecrow173/ruviform/issues/118)（video/NONRT データ面・shm レイアウト別改訂候補）・Issue [#120](https://github.com/scarecrow173/ruviform/issues/120)（共有リーパー/io 多重化 — bundle 後の子単位 I/O スレッドと相性）・Issue [#122](https://github.com/scarecrow173/ruviform/issues/122)（platform/process/ipc 共通クレート評価 — #117 を再設計前提として延期判定）・Issue [#123](https://github.com/scarecrow173/ruviform/issues/123)（frame-I/O dedup 等残件）・Issue [#115](https://github.com/scarecrow173/ruviform/issues/115)（L1 GUI 越境 parenting — bundle で GUI handle が instance 紐付けになるかは関連論点）
- 上流文書: [plugin-l1-out-of-process discovery](plugin-l1-out-of-process.md)・[spec](../specs/plugin-l1-out-of-process.md)・[design](../designs/plugin-l1-out-of-process.md)（#76 系 — singleton 前提の原点と deferred 行）・[issue-119 spec](../specs/issue-119-l1-wire-version-policy.md)（bump 前提）・[issue-76 task-1 IR](../impl/ir/issue-76-l1-core-wire-task-1-implementation.md)（bundle non-goal 記録）・[issue-122 eval](../designs/issue-122-shared-platform-process-ipc-eval.md)（#117 を再設計前提とする判定）
- 下流文書: [issue-117-bundle-child-process spec](../specs/issue-117-bundle-child-process.md)（`derived-by` — DRAFT）
- コード地図: `crates/ruviform-plugin-core/src/wire.rs`（wire codec・versioning・`WIRE_MAX_*`）・`crates/ruviform-plugin-host/src/l1/{mod,data,shm}.rs`（proxy spawn/Shared/converge/seq 分割/shm）・`crates/ruviform-plugin-host/src/{composite,command,endpoint}.rs`（公開面・route/slot・notification・endpoint detach）・`crates/ruviform-plugin-export/src/{lib,shm}.rs`（runner singleton・argv・serve ループ）・`crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` + `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` + `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs`（検証ハーネス）
