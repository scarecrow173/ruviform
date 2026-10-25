---
id: "IMPL-034LX97oTeWiGsXQQ4DYcH"
type: "impl"
status: "completed"
title: "Issue #76 PluginHost composite + L1 E2E — Task 5 implementation"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "docs/impl/ir/issue-76-plugin-host-composite-task-5-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/data.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/shm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "docs/tasks/plugin-host-composite-and-l1-e2e.md"
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
    - "docs/tasks/plugin-host-composite-and-l1-e2e.md"
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

# Issue #76 PluginHost composite + L1 E2E — Task 5 implementation

## Summary

Issue #76 Task 5 完了 — actions 15–17（`PluginHost` コンポジット
公開面 + crate/リポジトリ doc 整合 + 公開面 E2E）を全範囲実施。
新規 `src/composite.rs`（688 行）が `PluginHost` を公開:
`InProcessHost`（L0）と `pub(crate) l1`（L1）を単一統一
`PluginHandle` 空間の後ろに統合し、`plugin_id` → route 表の
登録時ルーティング（`register_factory` = L0 /
`register_out_of_process(OutOfProcessSpec)` = L1）で
`Instantiate{plugin_id}` の呼出し形を不変に保つ。統一スロット表
（`instances`/`generations`/`free_slots`）は Task 1 で修正済みの
`InProcessHost` generation 規則をそのまま踏襲 — stale handle は
generation 検査で `Err(InvalidHandle)` となり cross-route しない。
`l1` は引き続き `pub(crate)` — `composite` が唯一の in-crate
consumer。`tests/l1_proxy_e2e.rs` は **42 tests**（新規
`mod composite_e2e` 8 tests・既存 34 無退行）。独立レビュー修正
（P2 ×2 — reply-kind pairing・advisory `stop_processing` の
mirror lag・doc/test 群）を全て適用済み。全 gate green 実 log。

## Context

approved plan `docs/plans/implement-issue-76-plugin-l1-out-of-process.md`
の Task 5（`docs/tasks/plugin-host-composite-and-l1-e2e.md`・
actions 15–17）。upstream: approved spec/design
（`docs/specs/plugin-l1-out-of-process.md`・
`docs/designs/plugin-l1-out-of-process.md`）・Task 1 IR
（契約移設 — `Instantiate{plugin_id}`・`HostError::
DuplicatePluginId`・`PluginHandle` mint・Task 1 で修正済みの
generation 規則）・Task 2 IR（子側 `run` — 実 runner
`ruviform-plugin-l1-runner` を composite E2E が `program` に
使う）・Task 3 IR（`pub(crate) l1` 制御面 — spawn/handshake・
driver/reader・`Failed` 収束・notification queue・
`ChildExit`/`SpawnSpec` pub(crate) 型）・Task 4 IR
（`l1::data`/`l1::shm::View` データ面・`L1Instance::process` RT
脚）。TSpec `docs/test-specs/plugin-l1-proxy-contract-tests.md`・
`docs/test-specs/plugin-l1-runner-e2e-tests.md`。本 task で L1
は公開面を持つ — Task 4 までの「later-task」としていた全ての
口（`OutOfProcessSpec`/`HostNotification`/`ChildExit`・`Destroy`
legality gate・`Send`/`Sync` 決定・composite consumer）をここで
確定させた。

## Implementation

### Changes

- `crates/ruviform-plugin-host/src/composite.rs`（新規 688 行・
  `pub`）— コンポジット全体:
  - `OutOfProcessSpec` — L1 route の登録宣言: `descriptor`
    （`'static`・`Scan`/`Instantiate` 公開面 + handshake の
    field-equality oracle）・`program`/`args`（runner 実行体 —
    `ruviform_plugin_export::run` 上に構築された bin を想定）・
    `lookahead_blocks`（既定 **2** — L1 遅延契約
    `lookahead_blocks × block_frames / sample_rate` を決める・
    `0` は spawn 時 `Err(Ipc)` — Task 4 の defined rejection
    を継承）・`command_timeout`（既定 **10 s** — handshake と
    全 `Reply` wait の defined timeout・timeout は `Failed`
    converge）。`OutOfProcessSpec::new(descriptor, program)` で
    既定値構成。
  - `ChildExit { code: Option<i32>, signal: Option<i32> }` —
    公開の exit payload（`signal` は unix のみ・内部
    `l1::ChildExit` から `From` 変換）。
  - `HostNotification::InstanceFailed { handle, exit }`
    （`#[non_exhaustive]`）— `try_recv_notification` が返す
    非同期イベント。L0 は通知を発しない。`Failed` 遷移は
    「live 中に exactly once」（exit status 判明後・`Destroy`
    で slot が解放されると queue 残留分は slot と共に drop）。
  - `Route` — `plugin_id` の serving route（登録時確定・id の
    存続期間中固定）: `InProcess`（factory 経路）/
    `OutOfProcess(SpawnSpec)`。
  - `Slot` — 統一スロット表の live entry:
    `InProcess(PluginHandle)`（`InProcessHost` が mint した
    *inner* handle — コマンドは retarget で転送）/
    `OutOfProcess(Box<L1Instance>)`（`L1Instance` は ~500 B の
    ため Box 化して slot table を compact に — deviation
    記録）。
  - `PluginHost` — 統一スロット表（`instances:
    Vec<Option<Slot>>`・`generations: Vec<u16>`・`free_slots:
    Vec<u32>`）で全公開 handle を自前で mint:
    `InProcessHost` の slot/generation 規則をそのまま踏襲
    （Task 1 修正済み — slot 再利用で generation を bump・
    `wrapping_add(1)` は L0 と同じ `u16` 幅・stale handle は
    必ず拒否され alias しない）。
  - `register_factory` — L0 route claim。**infallible を維持**:
    id が既 claim（L0/L1 不問）なら first claimant が route を
    保持し、shadowed factory は `InProcessHost` の repeated-id
    first-match 意味論どおり登録は残る（`Scan` には列挙・
    `Instantiate` では first claimant のみ到達 — deviation
    記録）。`descriptor()` は route claim + `Scan` cache のため
    eager に問合せる — 非純粋 `descriptor()` は plugin 側
    defect として明記。
  - `register_out_of_process` — L1 route claim。既 claim の id
    は `Err(DuplicatePluginId)` の defined rejection（順序・
    route 不問）。`OutOfProcessSpec` → `l1::SpawnSpec` へ変換
    して route 表に格納。
  - `execute` — `Scan`（全登録 descriptor を登録順に）/
    `Instantiate`（route で分岐後、unified slot から handle
    mint）/`Destroy`（route 各自の teardown — L0 は
    `InProcessHost` 委譲・L1 は composite が legality gate を
    掛けてから `Shutdown`+reap）/ 残りは `dispatch`。
  - `dispatch` — handle 保持コマンドを unified slot 表経由で:
    L0 は `retarget`（inner handle 書換え）→ `l0.execute`、L1
    は `to_wire` 翻訳 → `instance.execute` →
    `host_reply(expected, reply)`（reply-kind pairing —
    review 修正、後述）。
  - `process` — RT 入口: L0 は直接呼出し、L1 は try 脚
    データ面（stage → doorbell `try_push` → `seq - lookahead`
    までの bounded drain → copy_out / defined pass-through）。
  - `lifecycle_state` — L0 は `InProcessHost` 委譲、L1 は
    proxy の local mirror（child 側 `Lifecycle` が authority —
    mutex 取得のため **L1 では non-RT 扱い**・crate doc に
    caveat 明記）。
  - `plugin_mut` — L0 スロットのみ `Some(&mut dyn
    ErasedPlugin)`、L1 スロットは常に `None`（out-of-process
    に貸せる in-process プラグインは存在しない）。
  - `try_recv_notification` — live L1 スロットを走査して
    instance の notification queue を `try_recv` — `Failed`
    を exactly once で `InstanceFailed` として公開 handle
    付きで報告。
  - `kill_child`（`#[doc(hidden)]`）— 検証支援: L1 child を
    force-kill（L0 スロットは `Err(Unsupported)`）。
  - `destroy` — L0 legality rule を L1 にも適用:
    `lifecycle_state().transition(LifecycleEvent::Destroy)` で
    `Activated`/`Processing` を `Err(Lifecycle)` 拒否（子には
    触れない）— `L1Instance::destroy` が無条件なため L0 規則は
    composite の責務。成功後は slot 解放 + `free_slots` へ。
  - `command_handle`/`retarget`/`to_wire` — handle 抽出・L0
    転送・wire 翻訳の private table。
  - `expected_reply_kind`/`ReplyKind`/`host_reply` — reply 側
    の pairing oracle（review 修正、後述）。
  - `mint` — `InProcessHost` と同一の handle 生成（free slot
    再利用 + generation bump、else table 拡張）。
  - **Send/Sync 決定** — `PluginHost` は `!Send`/`!Sync`:
    L0 は `Box<dyn ErasedPlugin>`（erased object に auto
    trait なし）、L1 slot は `shared_memory::Shmem`（raw
    pointer）を内部に持つ — 手動 impl は付けず、crate doc で
    明記（`L1Instance` 側の Send/Sync 記述も同決定に更新）。
- `crates/ruviform-plugin-host/src/lib.rs`:
  - `mod composite` + `pub use composite::{ChildExit,
    HostNotification, OutOfProcessSpec, PluginHost}`。
  - crate doc — L0/L1 両 backend・統一 handle 空間と登録時
    routing・**遅延契約（L1）**（`lookahead_blocks ×
    block_frames / sample_rate` — 既定 2 × 512f @ 48 kHz ≈
    21.3 ms・underrun は defined pass-through）・**RT 契約**
    （`PluginHost::process`/`plugin_mut` は両 route で
    RT-suitable・`lifecycle_state` は L0 const-cheap / L1 は
    shared mutex 取得のため non-RT 扱い・`execute`/
    `register_*`/`try_recv_notification` は non-RT 専用）・
    `Failed` 収束と notification の公開面。
  - `l1` mod コメント — consumer は `crate::composite::
    PluginHost` と明記・blanket `#[allow(dead_code)]` を撤去し
    test-support 項目に scoped `#[allow(dead_code)]` へ。
- `crates/ruviform-plugin-host/src/l1/mod.rs`:
  - **advisory `stop_processing` の mirror lag 修正**（review
    P2、後述）— `execute` が `self.request(cmd)?` で early
    return せず、`Err(Lifecycle)`（pre-commit legality 拒否 —
    子は遷移していないため mirror も動かさない）とそれ以外の
    `Err` を分け、後者は `err_committed_state`（`StopProcessing`
    → `Activated` のみ — 全 verb audit を doc 明記）で mirror
    を更新。terminal guard は converge 済み `Failed` の resurrect
    を防ぐ（timeout した `StopProcessing` は `Failed` のまま）。
  - `err_committed_state(cmd) -> Option<LifecycleState>` 追加。
  - doc 更新 — `l1` コメントの "later-task" 文言を composite
    確定名に置換・`ChildExit`/`SpawnSpec`/`L1Instance` の
    Send/Sync 記述を決定版（`!Send`/`!Sync` — `Shmem` raw
    pointer 由来・`PluginHost` も継承）に更新・`segment`
    フィールドに RAII-only 注記 + `#[allow(dead_code)]`。
  - `INSTANCE_COUNTER` doc — composite の公開 slot 番号とは
    別系統（key uniquing 用途のみ）と明記。
- `crates/ruviform-plugin-host/src/l1/data.rs` — `Staging::
  depth` フィールド削除（未使用 — dead_code trim・
  `frame_capacity`/`generation`/`ins`/`outs` のみ残存）。
- `crates/ruviform-plugin-host/src/l1/shm.rs` — `Segment::
  declared_len`/`verify_header` に scoped `#[allow(dead_code)]`
  + 「`tests/l1_proxy_e2e.rs` の `#[path]` include 経由の
  test-support surface」と注記（module-level blanket allow
  撤去に伴う移行 — Task 3 IR の方針どおり）。
- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`:
  - `--fail-stop-processing` flag 追加 — `StopProcessing` を
    commit（`Processing → Activated`）してから
    `Err(HostError::PluginFailed)` を返す、実 runner
    （`Runner::stop_processing` — 遷移適用後に hook を走らせる
    commit-then-advisory shape）と同型の fixture。advisory
    mirror lag 修正の実証用。
  - `dispatch` シグネチャに `fail_stop_processing: bool` を
    追加・flag 表 doc 更新。
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（1406 →
  2187 行・**42 tests** — 新規 `mod composite_e2e` 8 tests +
  既存 34 無退行）:
  - fixtures — `L0Gain`/`L0GainFactory`（L0 route 用 in-process
    gain plugin・調整可能 normalized gain + state blob・worker
    とは別 id）・`RUNNER_GAIN_DESCRIPTOR`（実 runner の
    `GainPlugin` が報告する descriptor — `OutOfProcessSpec`
    oracle）・`worker_program()`（`env!("CARGO_BIN_EXE_l1-
    test-worker")`）・`runner_program()`（**workspace test
    entry のみ**: `current_exe` → `target/<profile>/deps` の
    親 dir に `ruviform-plugin-l1-runner` を解決 —
    `CARGO_BIN_EXE_*` は cross-package bin を指せないため・
    deviation 記録）・`mixed_host`/`instantiate`/`host_block`/
    `drive_to_processing`/`wait_host_failed`/`wait_host_
    notification` helpers。
  - tests — `mixed_routes_full_series`（同一 `Instantiate` 形で
    L0+L1 混在 scan→instantiate→lifecycle→process→state→
    destroy・L0 `plugin_mut` `Some` / L1 `None`・両 route の
    `OpenGui`/`CloseGui` `Unsupported`・teardown 後
    `InvalidHandle` + notification 空）・
    `duplicate_plugin_id_is_a_defined_rejection`（L0→L1・L1→
    L0・L1→L1 の全順序で `Err(DuplicatePluginId)`）・
    `shadowed_factory_registration_keeps_first_claimant`
    （`register_factory` infallible 維持の帰結 — shadowed
    factory は `Scan` には列挙・`Instantiate` は first
    claimant のみ）・`l1_kill_reports_failed_once`（kill →
    `Failed` + `InstanceFailed` exactly once・exit status
    presence・以後 `Destroy` 以外 `Err(Failed)`・`process` は
    `Ok(CONTINUE)` + 無音・`Destroy` で escape）・
    `l1_destroy_honours_lifecycle_legality`（`Activated`/
    `Processing` からの `Destroy` は `Err(Lifecycle)` — 子に
    届かない・`Created`/`Initialized`/`Failed` からは合法）・
    `l1_stop_processing_advisory_err_still_reports_activated`
    （`--fail-stop-processing` — commit-then-advisory なので
    mirror は `Activated`・RT 脚も `Err(Lifecycle)` で一致・
    `StartProcessing` で復帰・2 回目も同 shape）・
    `stale_handles_never_cross_route`（slot 再利用で死んだ
    handle は相手 route に alias しない — L0 死 handle が L1
    instance を指すことも逆もない）・`runner_bin_serves_real_
    gain_plugin`（実 runner bin を `program` に持つ composite
    E2E — handshake field-equality・gain 正当性・遅延契約・
    `Destroy` まで）。
- `crates/ruviform-plugin-host/README.md` — 全面書換え: L0+L1
  2 backend・`PluginHost` 責務（統一 handle 空間・登録時
  routing・`DuplicatePluginId`）・L1 経路説明（1 instance =
  1 子プロセス・stdio 制御面 + shm データ面 + RT try 脚・
  `Failed`/`InstanceFailed`/defined pass-through）・遅延契約・
  許可依存に `ruviform-rt` + `shared_memory`（`l1::shm` 閉じ・
  公開 API/feature 名に非露出）・`l1` は `pub(crate)`・RT 契約
  （`lifecycle_state` L1 caveat 含む）。
- `crates/README.md` — Host 層行を「L0 インプロセス + L1
  アウトプロセス — `PluginHost` コンポジットが登録時
  ルーティング」に更新。
- `Cargo.toml`（root）— `ruviform-plugin-host` members
  コメントに Issue #76 L1 + `PluginHost` コンポジットを追記。
- `crates/ruviform-plugin-export/src/lib.rs`・`README.md` —
  parent 側の消費者記述を最終一致: runner bin は
  `ruviform_plugin_host::OutOfProcessSpec::program` として
  登録され、`PluginHost` が `Instantiate` ごとに spawn する。

### Review findings and resolutions（独立レビュー P2 ×2 + doc/test 群 — 修正済み）

- **P2（fixed）— wire reply の kind pairing 未検証**:
  `dispatch` は L1 `execute` の成功 `wire::Reply` を構造的に
  `HostReply` へマップしていたが、verb と reply kind の対応を
  検査していなかった — `Ok` で応える `SaveState` は state を
  黙って捨て、`State` で応える `Initialize` は state を捏造
  し得た。`dispatch` が `expected_reply_kind`（`SaveState`→
  `State`・`OpenGui`→`Opened`・その他全て `Ok`・
  `#[non_exhaustive]` wildcard は fail-closed で `Ok` 既定 —
  将来 `WIRE_VERSION` の新 verb は新 reply kind と pair せず
  anomaly）を計算し、`host_reply` が期待 kind と実 reply を
  pair 検査 — 不一致・`Scanned`・未知 kind は全て protocol
  anomaly `Err(Ipc)`。`Scanned` は handshake で消費済みで
  dispatchable verb が応答し得ないため常に anomaly。
- **P2（fixed）— advisory `stop_processing` の mirror lag**:
  `execute` が `self.request(cmd)?` で早期 `Err` していた
  ため、commit-then-advisory の verb（実 runner の
  `stop_processing` — `Processing → Activated` 遷移を適用
  *してから* hook が decline 可能・L0 `InProcessHost::
  stop_processing` と同一 ordering）では子が既に `Activated`
  なのに proxy mirror が `Processing` のまま —
  `lifecycle_state()` が嘘をつき RT 脚が doorbell し続ける
  lag（L0 にはない）。`Err(Lifecycle)`（常に pre-commit
  legality 拒否 — 子は遷移せず mirror も動かさない）と他の
  `Err` を分離し、後者は `err_committed_state`（全 verb
  audit: `StopProcessing` のみ `Some(Activated)` —
  `Initialize`/`Activate` は `InitFailed`/`ActivateFailed`
  ループで pre-command へ、`StartProcessing` は hook 失敗を
  `Activated` へ roll back、`Deactivate` の遷移は dispatch
  自体で Err は全て pre-commit、state/GUI verb は遷移なし）
  で mirror を更新。terminal guard が converge 済み
  `Failed` の resurrect を防ぐ。worker `--fail-stop-
  processing` fixture + `l1_stop_processing_advisory_err_
  still_reports_activated` で実 runner と同型の shape を
  E2E 検証。
- **doc 修正群（fixed）**: lib.rs `lifecycle_state` の L1
  mutex caveat・`l1` mod コメントの "later-task" 文言全撤去
  （`ChildExit`/`SpawnSpec`/`INSTANCE_COUNTER`/`destroy`/
  `L1Instance` Send-Sync を確定名・確定決定に）・
  notification の「live 中に exactly once」明記・
  `register_factory` の eager `descriptor()` purity 前提
  コメント・`Staging.depth`/`Segment::declared_len`/
  `verify_header`/`segment` の dead_code 整理。
- **test 修正群（fixed）**: `mixed_routes_full_series` で
  `CloseGui` も両 route 検査（`OpenGui` のみだった）・
  `try_recv_notification().is_none()` を drain と `Destroy`
  間に assert 追加（notification exactly-once の pin）・
  composite_e2e に `shadowed_factory_registration_keeps_
  first_claimant` + `l1_stop_processing_advisory_err_still_
  reports_activated` の 2 tests 追加。

### Decisions During Implementation

- **統一 handle 空間は composite 自身が mint** —
  `InProcessHost` の inner handle を公開 handle として透過
  せず、composite が全 handle を `InProcessHost` と同一規則
  （slot/generation・`u16` wrapping・free slot 再利用）で mint
  する。slot 表は handle → `Slot::InProcess(inner)` |
  `Slot::OutOfProcess` の対応だけを持つ — dispatch は
  generation 検査を通過した live handle のみ到達し、stale
  handle が cross-route する構造的可能性がない
  （`stale_handles_never_cross_route` で pin）。
- **`register_factory` は infallible のまま（deviation 記録
  済み）** — `InProcessHost` からの引継ぎ。既 claim id への
  L0 登録は `Err` ではなく first-claimant shadowing（`Scan`
  は全登録を列挙・`Instantiate` は first claimant のみ）—
  L0 の repeated-id first-match 意味論を composite に
  持ち越した。defined rejection は唯一の fallible 登録面
  `register_out_of_process` の `DuplicatePluginId` に集約。
- **L1 `Destroy` legality は composite の責務** —
  `L1Instance::destroy` は teardown primitive として無条件
  （Task 3 決定）なので、L0 規則（`Created`/`Initialized`/
  `Failed` からのみ合法）は composite が mirror に対して
  `transition(LifecycleEvent::Destroy)` で強制する。
- **reply pairing は「verb が唯一合法の kind」を持つ表で
  fail-closed** — `host_reply` を `reply → HostReply` の自由
  マッピングではなく `(expected, reply)` の対で検査する形に
  した。未知/将来 kind ・`Scanned` は pair を構成できず常に
  `Err(Ipc)` — `#[non_exhaustive]` な wire 拡張が黙って破損
  した公開 reply を作らない。
- **advisory-Err mirror は verb audit に基づく 1 項目表** —
  「全 Err を post-commit 扱い」でも「全 Err を pre-commit
  扱い」でもなく、runner dispatch の commit ordering を verb
  ごとに audit して `StopProcessing` のみ `Some(Activated)`。
  表自体に audit 根拠を doc として残し、`Err(Lifecycle)` は
  呼出し側で常に pre-commit として filter。
- **`PluginHost`/`L1Instance` は `!Send`/`!Sync` で確定** —
  手動 `unsafe impl` は付けない。根拠は `Box<dyn
  ErasedPlugin>`（auto trait 非対応）と `Shmem`（raw
  pointer）— `InProcessHost` と同じく single-threaded 前提を
  crate doc で明記。
- **`runner_program()` は `current_exe` から `target/
  <profile>` を解決（deviation・workspace test entry のみ）** —
  `CARGO_BIN_EXE_*` は同一 package の bin しか指せず、
  `ruviform-plugin-export` の bin を host crate の test から
  参照できないため。`cargo test --workspace` が全 member bin
  を profile dir に build することに依存・`assert!(path.
  is_file())` で不在時は実行手順を明示して fail（fragility
  は follow-up に記録）。
- **`Slot::OutOfProcess` は `Box<L1Instance>`（deviation・
  `large_enum_variant`）** — `L1Instance` は ~500 B で
  `Slot::InProcess`（8 B handle）と大きさが乖離するため、
  slot table の compact 性を優先して Box 化。clippy
  `large_enum_variant` は構造選択で回避。
- **blanket `dead_code` allow を撤去し項目別 scoped allow に
  移行** — `l1` に in-crate consumer（composite）ができた
  ため Task 3 の一時的 `#[allow(dead_code)]` を撤去。
  `#[path]` include 経由で test が触る面
  （`Segment::declared_len`/`verify_header`・`segment`
  フィールド）に test-support surface として局所許可を
  残す。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装 + レビュー修正のみ
（実験ログなし）。

### Rejected

なし — 統一 handle 空間・登録時ルーティング・`DuplicatePluginId`
・`Destroy` legality gate は design/task 確定事項。
`register_factory` infallible 維持と `runner_program()` の
`current_exe` 解決は deviation として記録（「棄却した候補」
ではない）。

## Validation

- gate（実 log・全 exit 0）:
  - `mise run fmt-check` — exit 0
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets`）
  - `mise run lint` — exit 0（clippy `--all-targets -D
    warnings` clean — `#[expect]`/`#[allow]` は `l1::shm` の
    2 scoped unsafe site・worker の同 2 site・`l1` の
    test-support `dead_code` site のみ）
  - `mise run test` — exit 0（`cargo test --workspace` — 全
    suites 0 fail・`l1_proxy_e2e` は **42 tests 全 pass**（新規
    `composite_e2e` 8 + 既存 34 無退行 — 本 IR 作成時に
    `cargo test -p ruviform-plugin-host --test l1_proxy_e2e`
    で再実測: 42 passed / 0 failed））
  - `mise run test-rt` — exit 0（`rt_` filter が
    **`rt_l1_process_path_no_alloc` と `rt_l1_result_leg_
    try_ops` の 2 件を実選択** — Task 4 から継続・0 件
    スキップの fake success ではない）
- 構造確認（実コマンド）:
  - `rg -n "pub struct PluginHost|pub struct OutOfProcessSpec|
    pub enum HostNotification|pub struct ChildExit"
    crates/ruviform-plugin-host/src/composite.rs` — 全公開型
    存在（`OutOfProcessSpec` L65・`ChildExit` L109・
    `HostNotification` L143・`PluginHost` L210）
  - `rg -n "lookahead_blocks.*block_frames.*sample_rate|遅延"
    crates/ruviform-plugin-host/src/lib.rs` — 遅延契約明記
    （L46）
  - `rg -n "expected_reply_kind|host_reply|err_committed_state"
    crates/ruviform-plugin-host/src` — review 修正の実在
- レビュー修正の検証（実施済み）:
  - reply-kind pairing — `host_reply` 不一致経路・`Scanned`
    anomaly（`mixed_routes_full_series`・`runner_bin_serves_
    real_gain_plugin` が実 runner の正規 pairing で通過）
  - advisory `stop_processing` mirror — `--fail-stop-
    processing` 経由 `l1_stop_processing_advisory_err_
    still_reports_activated`（mirror `Activated`・RT 脚
    `Err(Lifecycle)`・2 回目再現）
  - composite E2E 全体 — 8 tests 全 pass・既存 34 tests 無退行

## Non-Goals

- エンジン/利用側への L1 選択配線（consumer 側責務 —
  plugin-host の engine 消費自体が未実装）。
- `InProcessHost` の挙動変更（L0 は無変更 — `PluginHost` が
  composite）。自動再起動・L1 GUI（`Unsupported` 拒否のまま）。
- 実プラグインの L1 E2E — `l1-test-worker`/
  `ruviform-plugin-l1-runner` fixture で検証。
- `l1` の `pub` 化 — `pub(crate)` のまま（公開面は
  `composite` のみ）。

## Risks

- `runner_program()` の `current_exe` → `target/<profile>`
  解決は `cargo test --workspace` が全 member bin を同一
  profile dir に build する挙動に依存 — workspace 外実行や
  package 単独 `cargo test -p ruviform-plugin-host` では
  runner bin が存在しない可能性（不在時は手順明示の
  assert で fail — 実行入口は `mise run test` に限定・
  fragility は follow-up に記録）。
- `generations` は `u16` — `InProcessHost` と同一幅・同一
  wrapping 意味論（parity by design）。極端な slot 再利用
  回転数（2^16 周期）で stale handle が理論上 alias し得る
  のは L0 と同じ既知限界 — 本 task は L0 に厳しく揃える
  ことを選んだ（follow-up に記録）。
- advisory-Err mirror は `err_committed_state` の verb audit
  が根拠 — `ruviform_plugin_export::Runner` の commit
  ordering が将来変わると表が stale になる（表に audit
  根拠を doc 記録済み・worker fixture が commit-then-
  advisory shape を pin）。
- `View::attach` の header 再検証（Task 4 carry-over）・
  underrun metrics 未観測化・frame helper dedup — 引続き
  follow-up として残存（下記）。

## Follow-ups

- Task 6 `docs/tasks/plugin-l1-final-verification.md`
  （action 18 — Full Gate 1 回・`docs/impl/exp/data/
  issue-76-final-gate-*-*.log` 保存）・Task 7
  `docs/tasks/plugin-l1-closeout.md`（action 19 — canonical
  closeout・coverage・task/plan status・test-spec confirmed
  更新・design doc の `compiler_fence` 文言同期を含む）。
- u16 generation wrap — `PluginHost::mint` は L0 規則の
  parity 実装であり wrap 限界も L0 と同一（parity by
  design）。実害が観測された場合に L0 と一緒に評価。
- `runner_program()` path fragility — `current_exe` からの
  `target/<profile>` 解決は workspace test entry 依存。
  `CARGO_BIN_EXE_*` の cross-package 代替 or cargo metadata
  ベースの解決を評価。
- `View::attach` の header 再検証 — Task 4 documented
  decision 継続（第三者 attach 経路が現れた場合に再評価）。
- underrun metrics — pass-through/dropped submit/
  `RESULT_FAILED` の観測手段なし（Task 4 から継続 —
  `HostNotification` 面ができたため metrics surface の置き場
  候補として composite を再評価可）。
- frame-I/O helper dedup — `read_message`/`write_message`
  が `l1/mod.rs`・`l1-test-worker.rs`・`l1_proxy_e2e`・export
  `read_frame`/`send` の 4 箇所に同形状で散在（Task 2/3/4
  から継続 — wire 層への共通 helper 格上げを Task 6/7 で
  評価）。
