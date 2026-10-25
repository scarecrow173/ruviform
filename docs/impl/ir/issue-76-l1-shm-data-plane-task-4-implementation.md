---
id: "IMPL-034LVAYFtNda72goeeCUwD"
type: "impl"
status: "completed"
title: "Issue #76 L1 shm data plane + RT legs — Task 4 implementation"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/data.rs"
      - type: "file"
        path: "docs/impl/ir/issue-76-l1-shm-data-plane-task-4-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/shm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_runner_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/tasks/l1-shm-data-plane-and-rt-legs.md"
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
    - "docs/tasks/l1-shm-data-plane-and-rt-legs.md"
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

# Issue #76 L1 shm data plane + RT legs — Task 4 implementation

## Summary

Issue #76 Task 4 完了 — actions 12–14（shm データ面 + RT 脚 +
`process` 等価フロー + pass-through）を全範囲実施。Task 3 の
`pub(crate)` L1 プロキシ制御面に、新規 `l1::data` モジュール
（seq 分割 doorbell・atomic staging・`ruviform-rt` SPSC の
try-only RT 脚・preallocated scratch）・`l1::shm::View`（I/O
スレッド固有の非所有 attach）・driver の submit-drain +
`armed_for` generation ゲート・reader の flagged-`Reply`/
`BlockDone` demux・`L1Instance::process` RT 入口を追加し、
`l1-test-worker` を shm attach + ×0.5 gain block path 持ちの
決定的 fixture に拡張した。`tests/l1_proxy_e2e.rs` は 34 tests
（新規 14・既存 20 無退行）。独立レビュー修正（P1:
`read_out_slot` の hostile dims 範囲外アクセスを publish
withhold で fail-closed・P2 群 — `stage_in` stale plane
zero-fill・re-activation stale submit の generation ゲート・
`lookahead_blocks == 0` の `Err(Ipc)`・全 cross-process エッジ
の `compiler_fence` → `std::sync::atomic::fence` 強化・doc
群）を全て適用済み。全 gate green 実 log。

## Context

approved plan `docs/plans/implement-issue-76-plugin-l1-out-of-process.md`
の Task 4（`docs/tasks/l1-shm-data-plane-and-rt-legs.md`・
actions 12–14）。upstream: approved spec/design
（`docs/specs/plugin-l1-out-of-process.md`・
`docs/designs/plugin-l1-out-of-process.md`）・Task 1 IR
（wire codec・共有語彙）・Task 2 IR（子側 `run`・`wire::shm`
正本・`process_block` の対抗側実装）・Task 3 IR
（`issue-76-l1-proxy-control-plane-task-3-implementation.md` —
spawn/handshake・driver/reader 2 スレッド・exactly-once
`Failed` 収束・`l1::shm` segment + header・本 task が拡張した
制御面全体）。TSpec `docs/test-specs/plugin-l1-proxy-contract-tests.md`。
本 task で `l1` は制御面 + データ面の両 plane を持つ — 公開
`PluginHost` composite・`register_out_of_process`・`Destroy`
legality gate・`Send`/`Sync` 決定は引き続き Task 5。

## Implementation

### Changes

- `crates/ruviform-plugin-host/src/l1/data.rs`（新規 832 行・
  `pub(crate)`）— データ面全体:
  - **seq-space partition** — `BLOCK_SEQ_FLAG = 1 << 63`:
    command seq（`next_seq`・`0..`）と doorbell seq を `u64` の
    high bit で分割。child は `BlockIn` seq を verbatim echo
    するため、reader は flag bit のみで demux できる —
    `Reply{seq, Err}` の flagged 形は block rejection（slot
    parse 失敗・lifecycle 拒否・plugin panic）、unflagged は
    command waiter。flag のない `BlockDone` は存在しない
    doorbell の応答 = protocol violation（converge）。Task 3 IR
    の non-goal（doorbell/command seq 空間の disjoint 化）を
    ここで解消。
  - `RESULT_FAILED = 1 << 62` + `SEQ_MASK` + `mark_failed` —
    result channel 上で「reject された doorbell」を表す marker。
    block counter は low 62 bit のみ使用（`debug_assert!(block
    < RESULT_FAILED)` で invariant を pin — review nit 修正）。
    reject も alternation は完了する（slot 再利用可）一方、RT
    leg は failed 結果を pass-through に解決する。
  - `InStage`/`OutStage` — slot 当たりの cross-thread staging:
    全フィールドが plain atomic（samples は `f32` bit pattern の
    `AtomicU32`・wire byte 領域は `AtomicU8`・scalars は
    `AtomicU32`/`AtomicU64`）。`Arc` で caller（ins 書）・
    driver（ins 読 → shm 転記）・reader（shm → outs 書）間で
    共有。element アクセスは全て `Relaxed` — ordering は SPSC
    channel の release/acquire エッジ + pipe doorbell fence
    が担い、RT パス上の lock/alloc/shm アクセスはゼロ
    （`unsafe` もゼロ）。
  - `Staging` — activation 毎の staging ring: `depth =
    lookahead + 1`・`frame_capacity = activated max_frames`・
    `generation`（成功 `Activate` 毎に bump）。`max_frames ×
    WIRE_MAX_CHANNELS` を slot 当たり事前確保（channel 数は
    `process` 時点まで不明のため wire 上限で reserve）— RT
    パスに alloc を持ち込まない。`Submit{seq, generation}` +
    `armed_for` が driver 側の stale-submit ゲート: re-
    activation で `Inner::staging` が差し替わる前に queue
    された submit は新 ring の zeroed stage を実 seq で
    doorbell してしまうため drop — seq は消費されるが
    doorbell も `block_in_flight` 登録もされず、結果未到着は
    RT leg が定義済み pass-through に解決する。
  - `RtLeg` — caller スレッド端の RT 脚: `submit:
    Producer<Submit>` / `result: Consumer<u64>`（容量 D）、
    staging `Arc`、strict alternation の bookkeeping
    （`last_pushed`/`cleared`）、早期到着 stash `held`（容量
    上限 = channel 容量）、`submit_dims`（done seq が応答する
    block の authoritative dims）、`last_output` pass-through
    履歴、`scratch` encode バッファ（`SCRATCH_CAP =
    max(TRANSPORT_LEN, EVENT_WIRE_SIZE)` で `new` 時に
    reserve — transport ≤ 91 B・event record = `EVENT_WIRE_SIZE`
    固定なので全 encode が capacity 内に収まり、`clear` 済み
    `Vec` の `push`/`extend` が再確保しないことを構造で証明 —
    RT パスの alloc-free は観測ではなく構成による）。
  - `RtLeg::process` — `process` 等価フロー（design の 5 段
    どおり）: (1) `mirror.is_failed()` → silence +
    `Ok(CONTINUE)`・非 `Processing` → `Err(Lifecycle)`（L0 と
    同じ caller contract 違反・pass-through ではない）→
    (2) strict alternation の producer-side ゲート
    （`cleared[i] >= last_pushed[i]` の時のみ stage + submit —
    満杯/slot busy は投入放棄 = pass-through）→ (3)
    `expected = block - lookahead`（underflow = 定義済み無音
    — drain は `cleared` 更新のため実行）→ (4) `drain_to` で
    result ch を bounded drain（`< expected` は stale 破棄・
    `>` は `held` stash・回数は channel 容量で上限）→ (5)
    matched 非 failed なら out-stage の `seq` tag（`Acquire`）
    を検査して `copy_out` + `last_output` 更新 +
    `unpack_result`、不一致/未達/failed は `pass_through`
    （`last_output` または無音）。全非成功経路は `Ok(CONTINUE)`
    + 代替出力 — ハングしない。
  - `write_in_slot` — staged in-slot を shm slot record に
    serialize（driver 側・非 RT）。offset は `wire::shm` const
    のみ使用（`SLOT_OFF_*`/`TRANSPORT_*`/`RESULT_*`/`EVENTS_*`/
    `SAMPLES_OFF`/`SAMPLE_STRIDE`/`FLAG_TRANSPORT`）— 未使用
    領域（in-slot の `result`・transport tail）は子側の
    `write_out_slot` と同じく zero-fill。
  - `read_out_slot` — shm out-slot を `OutStage` に deserialize
    （reader 側・非 RT）。slot `seq` echo 不一致は publish
    なし `false`。**child が publish した `frames > frame_
    capacity` または `channels > WIRE_MAX_CHANNELS` も同じく
    `false`** — `stage.samples` は `frame_capacity ×
    WIRE_MAX_CHANNELS` しか持たず、over-contract dims は範囲
    外 index になるため publish withhold で fail-closed（RT
    leg の seq-tag 検査が pass-through に落とす — P1 レビュー
    修正）。成功時は `seq` tag を最後に `Release` store —
    読側は常に完全に publish された stage のみ観測する。
  - `mirror_encode`/`mirror_decode` — `LifecycleState` ↔ `u8`
    （`Shared::rt_state` atomic 用）。`#[non_exhaustive]` 対応で
    `Failed` と未知 state は保守的に terminal（fail closed —
    decode 側の out-of-range も `Failed`）。
  - `pack_result`/`unpack_result` — `ProcessResult`（status tag
    + `tail_frames`）を 1 `u64` に packing（`OutStage.result`
    用）。
- `crates/ruviform-plugin-host/src/l1/mod.rs`（982 → 1305 行）:
  - `Shared::rt_state: AtomicU8` — RT leg が読む const-cheap
    lifecycle snapshot。`Shared::set_mirror` を全 mirror 変異の
    単一経路化（`converge`・`execute` の post-command 更新・
    `destroy`）— `Release` store が `process` の `Acquire`
    load と対になり、RT 経路は mutex を一切取らない。
  - `Inner::block_in_flight: HashSet<u64>` — driver が
    `BlockIn` 書込み前（reader の `BlockDone`/flagged-`Reply`
    が必ず見つけるよう write 前に insert）に登録する doorbell
    legality set。reader は未知/unflagged seq の block 応答を
    protocol violation として converge。seq 分割により
    `waiters` key と alias しない。
  - `Inner::staging: Option<Arc<data::Staging>>` — 成功
    `Activate` で arm・re-activation で差替え。`execute` は
    reply 成功後に `staging_generation += 1` → `Staging::new`
    → shared publish + `rt.arm` を 1 step で行う（converge 済み
    なら arm は観測されず無害）。
  - driver — poll loop で data-plane `submit` channel を
    drain（capacity D で bounded・非 blocking）: `terminal`/
    `shutdown_initiated` なら新 doorbell を取らず seq 消費のみ、
    `armed_for` で stale-generation を drop、生存なら
    `block_in_flight.insert` → `write_in_slot`（`View::in_
    slot_mut`）→ `fence(Ordering::Release)` → `BlockIn`
    doorbell 書込み。RT 側が行うのは `try_push` のみ — 全ての
    blocking pipe/shm 作業はこのスレッドに留まる。
  - reader — 3 系統 demux: flagged `Reply{seq, Err}` →
    `block_in_flight.remove`（未知 → converge）+ `mark_failed`
    `try_push`（容量 D と in-flight 上限 D で溢れない）・
    `BlockDone` → flag + in-flight legality →
    `fence(Ordering::Acquire)` → `read_out_slot`（`View::out_
    slot`・publish 可否は内部判定 — done はいずれにせよ
    alternation を完了）→ result `try_push`。それ以外の kind・
    2 度目の `Hello`・逆方向 kind は従来どおり converge。
  - `L1Instance::process` — RT 入口: `rt_state` を `Acquire`
    load → `mirror_decode` → `rt.process`。RT-suitable —
    atomic staging write と `try_push`/`try_pop` のみ・lock/
    pipe/shm/alloc なし（`rt_l1_*` 契約テストが主張）。
  - spawn 分割 — `spawn_driver`/`spawn_reader` に分離:
    `Shmem` は `!Send`（`MapData` が raw mapping pointer を
    持つ）ため owner `Segment` をスレッドへ渡せず、各 I/O
    スレッドが `shm::View::attach(key)` で自前の mapping を
    開く。driver は child spawn → `Shared.child` install →
    自身の `View` attach → stdout 受け渡しの順（attach 失敗
    は kill + reap + `Err` で bounded）。reader は `Hello` 前に
    `View` attach — 壊れたデータ面を最初の `BlockDone` では
    なく spawn 失敗として報告する。
  - `lookahead_blocks == 0` → `Err(Ipc)`（spawn 時 defined
    rejection）— `D = 1` では `expected = block` が同一
    `process` call が submit した seq 自身となり、結果は原理
    的に先着しない永久 pass-through（永久無音）になる。
    使えない spawn 構成として他の失敗と同じく拒否（P2
    レビュー修正・`lookahead_zero_is_rejected` で pin）。
  - SPSC `submit`/`result` channel は capacity `D` — doorbell/
    result が in-flight に存在するのは最大 1 ring 分。
  - module doc 更新 — scope を両 plane に拡張・thread table
    に driver の submit-drain / reader の demux を追記・
    `lookahead_blocks` の意味（shm depth・SPSC 容量・result
    horizon）と `0` 拒否を明記。
- `crates/ruviform-plugin-host/src/l1/shm.rs`（283 → 356 行）:
  - `View` — proxy 自身の segment への非所有 local attach:
    `open` は `is_owner` を立てないため `View` drop は自
    mapping を unmap するだけで segment を unlink しない
    （unlink は `L1Instance` の owner `Segment` の責務）。
    thread 割当は thread table で固定 — driver は
    `View::in_slot_mut`（in-slot 唯一の writer）・reader は
    `View::out_slot`（out-slot 唯一の reader）。**`View` は
    header を再検証しない** — header フィールドは
    `wire::shm` const と spawn-time depth で既知（documented
    decision・follow-up に記録）。
  - `in_slot_mut`/`out_slot` — doorbell `seq % depth` で slot
    選択（`wire::shm` offset のみ）。
  - SAFETY コメントを実プロトコルに更新 — header は spawn 前
    に owner が一度だけ書く・各 slot は strict alternation の
    下で一度に片側のみが触る（`out_slot` は `BlockDone` の
    Acquire エッジ後に reader でのみ呼ぶ）。
  - `get_u64`/`put_u64` 追加 + `get_u32`/`put_u32` を
    `pub(crate)` 化（`l1::data` の slot codec が使用）。
- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`
  （272 → 610 行）— wire 適合 fixture にデータ面を追加:
  - `Segment` — proxy の `--ruviform-shm-key` を `Hello` 後に
    attach・実 runner の `shm::Segment` と同じ header 全項目
    検証（magic/version/depth/slot_bytes/offsets/長さ — 全て
    `wire::shm` const）。attach 失敗は非ゼロ exit（実 runner
    と同じ）。
  - `process_block` — in-slot 読取り（`seq` echo・wire bound・
    `activated_max_frames` — 実 runner の wire bounds →
    negotiated config の enforcement 順を再現）→ `WORKER_GAIN
    = 0.5` の gain 適用 → out-slot 書込み。degenerate block
    （`frames == 0 || channels == 0`）は `ZERO_DIM_RESULT =
    ProcessResult{Sleep, 0xDEAD}` marker — legitimately staged
    block は必ず実 dims を持つため、zero-dim は「staging せず
    doorbell された seq」（re-activation stale submit）の検出
    用 observability 仕掛け。
  - `block_in`/`block_in_dispatch` — lifecycle 拒否は実 runner
    同様の flagged `Reply{seq, Err}`、`hung` は応答なし
    （wedged data plane）、`--reject-blocks-after-msgs=N` 以降
    の doorbell は `Reply{seq, Err(PluginFailed)}`（実
    rejection shape — doorbell-only counter）。shm プロトコル
    は `fence(Acquire)` → slot read → gain → slot write →
    `fence(Release)` → `BlockDone`。
  - scoped `#[expect(unsafe_code)]` ×2（`bytes`/`bytes_mut` —
    `shared_memory` 0.12.4 accessor deviation・`l1::shm`/
    export `shm` と同一記録 — Cargo.toml の unsafe site note
    を 2 ファイル化に更新済み）。
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`
  （546 → 1406 行・**34 tests** — 新規 14 + 既存 20 無退行）:
  - helpers — `processing_instance`（Processing まで駆動・
    512-frame cap）・`run_block`/`run_leg`/`run_leg_full`
    （`HostInput`/`HostOutput` erased view 経由 — composite
    が使う面）・`sample_eq`/`is_defined_sample`（bit-exact —
    `to_bits`/`from_bits` round-trip + ×0.5 は厳密表現可能
    なので epsilon 不使用）・`ALLOC_DISABLER`（`rt_l1_*`
    レーン用）。
  - process 経路 — `process_roundtrip_gain_and_lookahead_delay`
    （×0.5 gain の正当性・定義済み 2 block 遅延・全出力が
    defined value・全 call `Ok(CONTINUE)`）・
    `process_requires_processing_state`（非 `Processing` は L0
    と同じ `Err(Lifecycle)`）・`process_over_capacity_block_
    is_ipc_error`（activated `max_frames` 超は `Err(Ipc)` —
    定義済み拒否）。
  - pass-through — `process_underrun_passes_through_defined_
    output`（健全 underrun — rapid call は `last_output`/無音
    のみ・pace を戻せば pipeline が満ち ×0.5 が正確に到着）・
    `process_after_crash_passes_through_then_silences`（crash
    後も `Ok(CONTINUE)`・converge 後は定義済み無音 — RT
    error 化もハングもしない）・`process_hung_child_passes_
    through_without_blocking`（`--hang-after-msgs` wedged
    data plane — 永久 pass-through・wall-clock wait なし・
    teardown は `Shutdown` を受理）。
  - RT レーン — `rt_l1_process_path_no_alloc`（`assert_no_alloc`
    窓で lifecycle gate → stage_in → `try_push` → bounded
    drain → `copy_out`/`pass_through` の全経路が 0 alloc —
    fixture/staging/channel/fabricated slot/warm-up は窓外・
    窓内で `read_out_slot` と実 result channel 経由で
    `copy_out`/`held`/event+transport encode/scratch 経路も
    実到達 — review nit で拡張）・`rt_l1_result_leg_try_ops`
    （reader `try_push` → leg の bounded `try_pop` drain —
    matched copy_out・`mark_failed`/stale は pass-through・
    submit seq が `BLOCK_SEQ_FLAG` を持つ）。
  - review 修正の直接検証 — `lookahead_zero_is_rejected`・
    `submit_carries_staging_generation_and_stale_is_dropped`
    （generation gate の unit 検証 — stale drop・現行保持・
    unarmed drop）・`reactivation_cycle_never_publishes_stale_
    blocks`（32 cycle: re-activation 直前に queue した submit
    が `ZERO_DIM_RESULT` を一度も publish しない — stale
    doorbell は marker で検出可能）・`read_out_slot_rejects_
    dims_over_activated_contract`（hostile dims が OOB せず
    withhold）・`stage_in_zero_fills_planes_without_a_source`
    （outputs-only block が前 occupant の samples を leak
    しない）・`rejected_block_replies_resolve_pass_through`
    （実 flagged-`Reply` rejection — reject 済み block が
    processed output を publish しない・定義済み pass-
    through・ハングなし）。
  - `ruviform_media_core::{MediaTime, TransportStatus}` —
    `run_leg_full` の `TransportInfo::new` が必要とするため
    dev-dep 追加（下記 Cargo.toml）。
- `crates/ruviform-plugin-export/src/lib.rs` — `BlockIn` arm の
  `compiler_fence(Acquire)`/`compiler_fence(Release)` を
  `std::sync::atomic::fence` に強化（同一 protocol edge —
  declared file set 外だが fence 一貫性のため同 commit で変更
  — deviation に記録）+ Task 4 seq 分割が確定したため TODO
  comment を `BLOCK_SEQ_FLAG` 契約の記述に置換。
- `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` —
  e2e 内の test 側 fence ×4 を同じく `atomic::fence` に。
- `crates/ruviform-plugin-host/Cargo.toml` — dev-dep に
  `ruviform-media-core`（e2e の `TransportInfo` 構築が
  `MediaTime`/`TransportStatus` を要する）+ `shared_memory`
  の unsafe site note を「`l1::shm` と `l1-test-worker` bin」
  の実情に更新。
- `crates/ruviform-plugin-core/src/wire.rs` — rustfmt の
  const-block 整形のみ（semantic 変化なし — `mise run fmt` が
  適用した既存コードの normalize）。
- `Cargo.lock` — `ruviform-media-core` を `ruviform-plugin-host`
  dev-deps に記録。

### Review findings and resolutions（独立レビュー P1 ×1 + P2 群 + nit 群 — 修正済み）

- **P1（fixed）— `read_out_slot` の hostile dims OOB**: child
  publish の `frames`/`channels` を無検査で信頼していた —
  `stage.samples` は `frame_capacity × WIRE_MAX_CHANNELS`
  しか持たないため、over-contract dims を報告する child は
  staging buffer を範囲外 index し得た。dims を contract と
  照合し、違反は seq mismatch と同じ経路（publish withhold →
  RT leg の seq-tag 検査が fail-closed → defined pass-
  through）に落とすよう修正 — protocol violation を「切詰めて
  破損 publish」するのではなく「公表しない」。`read_out_
  slot_rejects_dims_over_activated_contract` で直接検証。
- **P2（fixed）— `stage_in` の stale plane 残留**: 出力が
  `block.channels` を claim する全 plane が defined samples
  を持つべきところ、source の無い channel（outputs-only
  block・bus が `channel` slice を不足して露出）は前
  occupant の samples が残り `write_in_slot` 経由で child に
  serialize され得た。`src.and_then(|s| s.get(f)).map_or(0,
  |s| s.to_bits())` で zero-fill に修正 —
  `stage_in_zero_fills_planes_without_a_source`（出力のみ
  block が slot を再利用する経路）で直接検証。
- **P2（fixed）— re-activation stale submit**: `Activate` が
  `Inner::staging` を差し替える前に queue された submit は、
  新 ring の zeroed stage を実 seq で doorbell し得た（child
  には「staging されていない block」として届く）。
  `Submit{seq, generation}` + `armed_for` driver ゲートで
  stale-generation を drop — `block_in_flight` 未登録・
  doorbell 未送信・seq 消費のみで、未到着結果は RT leg の
  defined pass-through に解決。`submit_carries_staging_
  generation_and_stale_is_dropped`（unit）+
  `reactivation_cycle_never_publishes_stale_blocks`（32
  cycle・worker の `ZERO_DIM_RESULT` marker で stale
  doorbell を検出可能化した E2E）で検証。
- **P2（fixed）— `lookahead_blocks == 0` の永久無音**: `D =
  1` では `expected = block - 0 = block` が同一 call の
  submit seq 自身 — 結果は原理的に先着せず leg は永久に
  pass-through（永久無音）する。`spawn` で `Err(Ipc)` の
  defined rejection に（`lookahead_zero_is_rejected` で pin・
  `SpawnSpec::lookahead_blocks` doc 明記）。
- **P2（fixed・deviation）— `compiler_fence` → `std::sync::
  atomic::fence` 全面強化**: design/task は doorbell エッジ
  を `compiler_fence` と記述したが、compiler fence は同一
  スレッド内の命令順序規制であり、process 境界を越える
  ordering をハードウェア面で保証しない。全 6 本の
  cross-process エッジ（host driver の `BlockIn` 前
  `Release`・host reader の `BlockDone` 後 `Acquire`・
  worker の slot 読み前 `Acquire` + `BlockDone` 前
  `Release`・export runner の同 2 エッジ）+ export e2e の
  対応 test fence を `std::sync::atomic::fence` に強化 —
  ordering は「hardware fence + pipe read/write の syscall
  境界」の対で構成される。design の literal 記述を超える
  deliberate strengthening — deviation として記録（詳細は
  Decisions）。
- **nit（fixed）— `rt_l1_process_path_no_alloc` の未到達経路**:
  旧版は `copy_out`/`held` stash/`pass_through` 更新・
  `stage_in` の event+transport encode（scratch `Vec` 経路）
  を窓内で一度も通っていなかった。`read_out_slot` で結果を
  fabricate し実 result channel に `try_push` する形に拡張
  — 全 RT 経路が窓内で実到達し、scratch の alloc-free は
  `SCRATCH_CAP` の capacity 証明に裏付けられた「構成による
  ゼロ再確保」として検証される。
- **nit（fixed）— 実 rejection 経路の欠如**: `mark_failed`
  経路は fabricated な検証のみだった — worker に
  `--reject-blocks-after-msgs`（doorbell-only counter・実
  runner の rejection shape `Reply{seq, Err(PluginFailed)}`）
  を追加し、`rejected_block_replies_resolve_pass_through`
  で実 flagged-`Reply` → `mark_failed` → pass-through の
  全経路を E2E 検証。
- **nit（fixed）— seq invariant の pin**: `debug_assert!(block
  < RESULT_FAILED)` — block counter が `RESULT_FAILED` bit
  に触れないこと（`mark_failed` の packing と seq 分割が
  依存する前提）を構造で主張。
- **doc 修正群（fixed）**: mod.rs scope（両 plane）・thread
  table（submit-drain/demux）・`lookahead_blocks` 意味・
  `Inner`/`Shared::rt_state` ドキュメント・shm.rs `View`/
  SAFETY 記述・Cargo.toml unsafe site note（`l1::shm` +
  worker の 2 ファイル）・export `BlockIn` arm の
  TODO → `BLOCK_SEQ_FLAG` 契約・`stage_in` transport encode
  失敗の defined-drop 仕様 — 全て明記。

### Decisions During Implementation

- **staging は「RT が shm に触れない」構造で解く** — design
  の「RT スレッドは shm に触れない」を、caller が plain
  atomic の `InStage` を書き driver が shm へ memcpy する
  2 段構造で実現した。element は全て `Relaxed` — publish
  は SPSC channel の release/acquire エッジと doorbell
  fence が担い、per-element の強い ordering は不要（lock
  ・alloc・shm・`unsafe` が RT パスに存在しないことを構造
  で担保）。
- **strict alternation は producer-side で強制** — slot
  再利用を `cleared[i] >= last_pushed[i]`（done `seq - D`
  受領後のみ）で caller 側に閉じ込めた。結果が doorbell
  順に到着し `expected = seq - lookahead`・`D = lookahead
  + 1` なので、この gate は shm in-slot の空きと
  `staging.outs[i]` のデータ保持を同時に保証する —
  out-stage の `seq` tag は misbehaving child に対する
  belt-and-suspenders 検査。
- **rejected doorbell は `RESULT_FAILED` で result 完結**
  — child が flagged `Reply{seq, Err}` で応答した block は
  alternation 上は完了扱い（slot 再利用を詰まらせない）
  だが、RT leg は `RESULT_FAILED` bit で publish せず
  pass-through に解決する。「done だが失敗」を seq 空間に
  載せることで、result channel 型を `u64` のまま保てた。
- **`next_block` は activation を跨いで monotone** — 再
  arm 後も block counter を reset しないため、stale
  in-flight reply は新世代の seq と alias し得ない。
  `generation` は ring 差替えの gate（`armed_for`）として
  別の次元を担う。
- **cross-process ordering = `atomic::fence` + pipe syscall
  境界**（deviation）— design の字面 `compiler_fence` を
  `std::sync::atomic::fence` に強化した。compiler fence は
  コンパイラの並べ替え規制に留まり、別 process の CPU が
  観測する shm への可視性を何も保証しない。実 fence は
  ハードウェア ordering を発行し、doorbell の pipe read/
  write syscall が片側の publicaton boundary を構成する —
  fence 無しには原理的に成立しない契約なので、弱い方を
  選ぶ余地はなかった。export 側（declared file set 外）の
  2 エッジ + その e2e の test fence も同じ protocol edge
  なので一貫性のため同 commit で変更した（deviation 記録
  済み — design doc 側の `compiler_fence` 文言との正本
  整合は closeout task の doc 同期で扱う）。
- **reader は `Hello` 前に `View` attach** — `Shmem` が
  `!Send` のため owner `Segment` を渡せず各スレッドが key
  で開く設計の帰結として、reader の attach を handshake
  前に置いた — 壊れたデータ面を「最初の `BlockDone` で
  気付く」ではなく spawn 失敗（`Err(Ipc)` + converge）に
  する。driver も `driver_main` 前に attach し、失敗は
  kill + reap で bounded。
- **`read_out_slot` は 2 種の違反を同じ `false` に畳む**
  — slot `seq` echo 不一致と over-contract dims はどちらも
  protocol violation で、応答は同一（publish withhold —
  done 自体は result channel に流し alternation を完了
  させる）。切詰めは「破損データの公表」なので採らない。
- **`stage_in` の transport encode 失敗は defined drop**
  — `Err` でなく `FLAG_TRANSPORT` を立てないだけ（子は
  transport 無しの block を見る）。encode 失敗は wire
  改訂で表現不能な plugin 供給 side-channel レコードの
  問題であり、block 自体を fail させる話ではない（doc
  明記）。
- **`lib.rs` は変更不要だった** — task doc の Affected
  Files は `process`/`lifecycle_state`/`kill_child` の L1
  経路配線を `lib.rs` に見込んでいたが、`process` は
  `L1Instance` の `pub(crate)` メソッドとして `l1` 内に
  閉じ、composite への公開配線は Task 5 の責務。`mod l1`
  宣言も Task 3 で済んでおり、本 task の lib.rs 差分は
  ゼロ（実情を IR に記録）。
- **`wire.rs` の差分は rustfmt normalize のみ** —
  `mise run fmt` が既存の const-block アサーション整形を
  適用した非 semantic 変更。commit 時にワーキングツリー
  を汚さないため取り込む。
- **e2e は「defined value のみ」を主張** — `is_defined_
  sample`（silence or `0.5 ×` 完了し得た block の入力）で
  全出力を検査し、torn mix・garbage・stale block を構造的
  に排除する。`sample_eq` は bit-exact（`to_bits` round-
  trip + ×0.5 は厳密）— epsilon は不要で望ましくもない。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装 + レビュー修正のみ
（実験ログなし）。

### Rejected

なし — seq 分割・atomic staging・strict alternation・
pass-through 規則は design 確定事項。`compiler_fence` →
`atomic::fence` は design を超える修正であり「棄却した
候補」ではなく deviation として記録。

## Validation

- gate（実 log・全 exit 0）:
  - `mise run fmt-check` — exit 0
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets`）
  - `mise run lint` — exit 0（clippy `--all-targets -D
    warnings` clean — `l1` 内 `#[expect]`/`#[allow]` は
    `l1::shm` の 2 scoped unsafe site・worker の同 2 site・
    lib.rs の module-level `#[allow(dead_code)]` のみ）
  - `mise run test` — exit 0（`cargo test --workspace` —
    全 suites 0 fail・`l1_proxy_e2e` は **34 tests 全
    pass**（新規 14 + 既存 20 無退行 — 本 IR 作成時に
    `cargo test -p ruviform-plugin-host --test l1_proxy_e2e`
    で再実測: 34 passed / 0 failed））
  - `mise run test-rt` — exit 0（`rt_` filter が
    **`rt_l1_process_path_no_alloc` と `rt_l1_result_leg_
    try_ops` の 2 件を実選択** — 0 件スキップの fake
    success ではない。本 IR 作成時に `cargo test -p
    ruviform-plugin-host rt_l1` で再実測: 2 passed /
    32 filtered out）
- 構造確認（実コマンド）:
  - `rg -n "compiler_fence" crates/ruviform-plugin-host/src/l1
    crates/ruviform-plugin-export` — **0 件**（全 6 本の
    cross-process エッジ + e2e test fence は
    `std::sync::atomic::fence` — task doc の `compiler_fence`
    文字面検査は deviation 反映で実コマンドを更新済み）
  - `rg -n "fence\(Ordering" crates/ruviform-plugin-host/src`
    — `l1/mod.rs` Release/Acquire + worker Acquire/Release
    の doorbell エッジ存在
  - `rg -n "atomic\|Mutex\|head\|tail" crates/ruviform-plugin-
    host/src/l1/shm.rs` — shm 内に共有制御状態なし
    （`View`/`Segment` の byte アクセスと accessor のみ —
    slot 選択は doorbell `seq % D`）
  - `rg -n "unsafe" crates/ruviform-plugin-host/src/l1
    crates/ruviform-plugin-host/src/bin` — `l1::shm` の 2
    scoped `#[expect(unsafe_code)]` + `l1-test-worker` の
    2 scoped（同 deviation・SAFETY コメント付き）のみ —
    `l1/data.rs`・`l1/mod.rs` の RT 経路に自前 `unsafe`
    ゼロ
- レビュー修正の検証（実施済み）:
  - hostile dims の publish withhold —
    `read_out_slot_rejects_dims_over_activated_contract`
  - stale plane zero-fill — `stage_in_zero_fills_planes_
    without_a_source`
  - generation gate — `submit_carries_staging_generation_
    and_stale_is_dropped` + `reactivation_cycle_never_
    publishes_stale_blocks`（32 cycle）
  - `lookahead == 0` — `lookahead_zero_is_rejected`
  - 実 rejection — `rejected_block_replies_resolve_pass_
    through`

## Non-Goals

- `PluginHost` composite 公開面・`register_out_of_process`
  公開 API・統一 handle 空間・`Destroy` legality gate（L0
  規則は composite 層）・`HostNotification::InstanceFailed`
  公開型・`L1Instance` の `Send`/`Sync` 決定 — 全て Task
  5（`l1` は引き続き `pub(crate)`）。
- `process` 経路のブロック処理自体の RT 化 — RT は try
  脚のみ（pipe/shm/blocking read は driver/reader 側）。
- video/NONRT plugin の L1 経路・`WIRE_MAX_*` 上限引上げ・
  可変 slot・shm 内共有制御状態（head/tail・ロック —
  design 棄却どおり非採用）。
- underrun/pass-through の計測公開 — design が指標を定義
  しない（follow-up に記録）。
- `View::attach` の header 再検証 — owner 側の作成時
  `verify_header` に依る documented decision（follow-up
  に記録）。

## Risks

- cross-process ordering は「`atomic::fence` + pipe syscall
  境界」の対に依存 — C++ メモリモデルがプロセス間 shm に
  及ばないのは design 前提どおりだが、万一の misbehaving
  child（`BlockDone` 後も out-slot を書き続ける等）への防御
  は out-stage `seq` tag と `read_out_slot` の dims 検査が
  担い、破損は publish withhold → pass-through に
  fail-closed する構造。fence の HW 保証を超える領域は
  残存リスクとして記録（design 棄却済みの共有制御状態は
  採らない）。
- driver の submit drain は `DRIVER_POLL_INTERVAL`（200µs）
  周期 — doorbell 投入から wire 送出までの latency は poll
  cadence にboundされる（定義遅延 `lookahead` の中では
  吸収されるが、scheduling 粒度依存である点は Task 3 と
  同じ）。
- stale-generation submit の drop は seq を消費する —
  result は決して到着せず pass-through に解決するため、
  re-activation 直前の submit は定義上「失われた block」
  になる（`reactivation_cycle_*` が 32 cycle で publish
  されないことを検証）。underrun として観測可能にする
  かは underrun metrics の follow-up 範囲。
- reader の result `try_push` は容量 D と in-flight 上限
  D で溢れない設計 — 万一の full でも seq は drained 済み
  扱いとなり pass-through に落ちるだけ（harmless・doc
  明記）。
- worker fixture と実 runner（Task 2 export）の意味論
  drift — worker は wire bound + `activated_max_frames` の
  enforcement 順と rejection shape を実 runner に合わせ
  模倣したが、真の runner との proxy E2E 結合は Task 5
  （composite E2E が runner bin を使う）で担保される。

## Follow-ups

- Task 5 `docs/tasks/plugin-host-composite-and-l1-e2e.md`
  （actions 15–17）— `PluginHost` composite + unified
  handle space + `register_out_of_process` 公開 API・
  `Destroy` legality gate・`L1Instance` の `Send`/`Sync`
  公開面決定・`HostNotification::InstanceFailed` 公開型・
  実 runner（`ruviform-plugin-l1-runner`）との composite
  E2E。`L1Instance::process` が composite の `process`
  配線で消費される。
- underrun metrics — `last_output`/silence pass-through・
  dropped submit・`RESULT_FAILED` の発生回数は現状
  観測手段なし（design が指標を定義しない）。可観測化が
  必要なら composite/notification 面で再評価。
- `View::attach` の header 再検証 — `View` は owner の
  `Segment::create` が `verify_header` 済みである前提で
  header を再検証しない documented decision。第三者
  attach 経路が現れた場合に再評価。
- design doc の fence 文言同期 — `docs/designs/plugin-
  l1-out-of-process.md` は `compiler_fence` のまま（本
  task の deviation は実装側を強化）。正本 doc の更新は
  closeout task（Task 7）の canonical doc 同期で扱う。
- frame-I/O helper dedup — `read_message`/`write_message`
  が `l1/mod.rs`・`l1-test-worker.rs`・`l1_proxy_e2e`・
  export `read_frame`/`send` の 4 箇所に同形状で散在
  （Task 2/3 IR から継続 — wire 層への共通 helper 格上げ
  or test-only dedup を Task 5 以降で評価）。
