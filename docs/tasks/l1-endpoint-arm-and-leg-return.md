---
id: "TASK-034MrsLWkKNHnu7jX1ghp8"
type: "task"
status: "todo"
title: "L1 endpoint（leg 移出・arm channel・pending_arm・leg 返却・drain-before-early-return）"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
    - "docs/tasks/plugin-core-endpoint-primitives.md"
    - "docs/tasks/l0-shared-slot-and-endpoint-inner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-endpoint-contract-tests.md"
    - "docs/test-specs/plugin-rt-retirement-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# L1 endpoint（leg 移出・arm channel・pending_arm・leg 返却・drain-before-early-return）

## Work

Plan actions 8–11。L1（out-of-process）経路の endpoint 所有分離 — `L1Instance.rt: Option<RtLeg>` の `take` で RT leg を non-RT host から移出し、arm channel・leg 返却 channel・arm `Retirer` の per-attach 経路を導入する。`Shared::rt_state` を `Arc<AtomicU8>` 化して endpoint が mirror を直接観測できるようにする。endpoint は `RtLeg` のみを保持し、child process・shared memory・mutex 状態を持たない。

- **`l1/mod.rs` 構造変更（crate-free discipline 遵守）**: `src/l1/` 木は `crate::`/`ruviform_plugin_host::` パス禁止（`l1/mod.rs` L19-26 — `tests/l1_proxy_e2e.rs` が `#[path]` include）。新規型・import は `ruviform_plugin_core::`/`ruviform_rt::`/`std::`/`self::`/`super::`/`data::` のみ — **`EndpointBounds`（`src/endpoint.rs` の公開型）・`L1Endpoint` を l1 から名指さない**（parts API 分離 — 下記）。`L1Instance.rt: Option<RtLeg>`（現行 `rt: data::RtLeg`（L888）を Option 化 — `process`/`destroy` 等の呼出しを `rt` presence に追従）・`Shared::rt_state: AtomicU8`（L256）を `Arc<AtomicU8>` 化（`set_mirror`/更新点は不変 — endpoint が同じ `Arc` を clone 共有）・新規 field: `arm_tx: Option<Producer<Arc<Staging>>>`・`pending_arm: Option<Arc<Staging>>`（latest-wins 退避）・`return_rx: Option<Consumer<RtLeg>>`・`arm_collector: Option<DropCollector<Arc<Staging>>>`・`endpoint_bound: Option<(u32, u32)>`（attach 時の bound 簿記 `(max_frames, channels)` — 再 Activate gate 用・公開型 `EndpointBounds` を名指さないプリミティブ対）
- **`l1/data.rs` `RtLeg::arm_apply`**: `pub(crate) fn arm_apply(&mut self, arm: Arc<Staging>, retirer: &mut Retirer<Arc<Staging>>)` — **簿記分割**: ring 束縛のみ reset（`staging` 差替・`frame_capacity = arm.frame_capacity`・`last_output` 中身を clear + `resize`（事前 `reserve` 済み capacity 内 — realloc なし）・`last_frames`/`last_channels`/`have_output`）+ 旧 ring（`self.staging.take()` の旧値）を `retirer.retire`。**seq 空間簿記は持続**（`last_pushed`/`cleared`/`held`/`submit_dims`/`next_block`/`depth`/`lookahead` は変更しない — strict-alternation 不変条件 `cleared[i] ≥ last_pushed[i] ⇒ slot i 空` の維持・stale done が再 arm 後 seq と alias しない）。現行 `RtLeg::arm`（ホスト側 `Activate` 用）は不変 — 別メソッド
- **`endpoint.rs` `L1Endpoint`**（T2 の `EndpointInner` に `L1` variant を充填）: `struct L1Endpoint { leg: Option<RtLeg>, mirror: Arc<AtomicU8>, arm_rx: Consumer<Arc<Staging>>, arm_retirer: Retirer<Arc<Staging>>, return_tx: Producer<RtLeg> }` — child process・shm segment・`L1Instance` 参照を持たない（`Send` を型構造のみで担保・`unsafe impl` なし）。**組立ては `endpoint.rs` 側** — `L1Endpoint`/`EndpointBounds` は crate 直下モジュールの型で l1 から名指せないため、`L1Instance::prepare_endpoint` は **`EndpointLegParts` を返す parts API** とし、`endpoint.rs` が `crate::l1::data::EndpointLegParts` から `L1Endpoint` を組立てる（`endpoint.rs` は `crate::l1::data` を利用可 — l1 木の外）
- **`l1/data.rs` `EndpointLegParts`（`pub(crate)` struct・crate-free）**: `{ leg: RtLeg, mirror: Arc<AtomicU8>, arm_rx: Consumer<Arc<Staging>>, arm_retirer: Retirer<Arc<Staging>>, return_tx: Producer<RtLeg> }` — l1 が返せる名指し可能なプリミティブ束（`data::RtLeg`/`ruviform_rt` 型/`std` 型のみ — `EndpointBounds`/`L1Endpoint` を含まない）。併せて `RtLeg` に **`pub(crate) fn armed_frame_capacity(&self) -> usize`**（現装着 `staging` の `frame_capacity`・未装着は 0）を追加 — `EndpointInner::L1::process` の per-block dims pre-gate が `staging` field（private）を読むために必要
- **`L1Instance::prepare_endpoint(&mut self, bound_max_frames: u32, bound_channels: u32) -> Result<data::EndpointLegParts, HostError>`（`pub(crate)`）**: **順序を pin** — (1) state check（`Activated`/`Processing` 要求 — `rt.is_none()` の detached もここで `Err(EndpointUnavailable)` — 単一 endpoint 規則。詳細な state 写像は T4 統一表・拒否順序は state 系 → slot-shape/leg presence → bounds）→ (2) bounds validation（`bound_max_frames`/`bound_channels` 非 0・`≤ WIRE_MAX_FRAMES`(8192)/`≤ WIRE_MAX_CHANNELS`(16)・`bound_max_frames ≥ armed staging.frame_capacity`（activated 相当 — `shared.lock().staging`）— 違反は `Err(Prepare(UnsupportedConfig))`。uniform prelude は T4 所有・本層は leg 移出前の activated 層再検査 — **take より前に行い bad bounds は leg を動かさず返す**）→ (3) `rt.take()` で leg 移出 → (4) `channel::<Arc<Staging>>(2)`/`retire_channel::<Arc<Staging>>(4)`/`channel::<RtLeg>(1)` を mint → (5) `Inner::staging` clone を **seed arm** として `arm_tx.try_push` → (6) leg の `last_output` を `bound_max_frames as usize * WIRE_MAX_CHANNELS as usize` へ事前 `reserve` → (7) `endpoint_bound = Some((bound_max_frames, bound_channels))` 簿記 + `arm_tx`/`pending_arm`/`return_rx`/`arm_collector` field 配線 → (8) `EndpointLegParts` を返す。**(3) 以降のあらゆる `Err` 経路は `self.rt = Some(leg)` で leg を復元してから返す** — post-take `Err` が leg を orphan 化しない（契約: 「`prepare_endpoint` `Err` 後もインスタンスは attachable/processable のまま」— test-spec で pin）
- **seed arm の apply 条件（generation 規則の正確化）**: seed arm は **`seed.generation > leg の現装着 staging.generation` のときのみ apply される — generation が一致する場合に限り正規化 no-op（apply されず unapplied のまま retire）**。通常経路（`Activate`→`prepare_endpoint`）では `Inner::staging` clone = leg 装着 ring と同一 `Arc`・同 generation で no-op だが、**detached-Activate**（attach 生存中の `Deactivate`→`Activate` で `Inner::staging` が新 generation へ進み、endpoint 側に未着のまま leg が `return_rx` drain で還流した場合）では `Inner::staging` が leg の装着 generation を上回り得る — その場合 seed は **legitimately apply される**（偽の no-op 前提を書かない）
- **Activate 時の arm 供給**: `L1Instance::execute(Activate)` 成功時（新 `Arc<Staging>` 生成後）に `arm_tx` が存在すれば `try_push` — `Full` なら `pending_arm` へ latest-wins 退避（前の退避を drop して最新のみ保持 — `DropCollector` 行き不要・`pending_arm` は host 側変数なので drop は non-RT）
- **再 Activate bound gate**: **attach 生存の authoritative 判定は `self.rt.is_none()`（leg 移出中）** — `endpoint_bound.is_some()` ではない（`endpoint_bound` は簿記であり、leg 返却後に clear され得るため liveness 判定として使わない）。`self.rt.is_none()` かつ `new_config.max_frames > endpoint_bound.0`（簿記した bound の `max_frames`）→ `Err(HostError::Prepare(UnsupportedConfig))` — arm 生成・`try_push` の前に拒否（比較対象は `endpoint_bound` の簿記値 — 現行 activated config ではない）
- **endpoint 側 arm drain**（`L1Endpoint` の process 冒頭 — `EndpointInner::L1` arm）: `arm_rx` を bounded drain（1 pass の pop ≤ `arm_capacity + 1` = 3）— 各 popped `Arc<Staging>` について `generation > 現装着 staging.generation` なら `arm_apply`（適用）・それ以外は unapplied — **drain した arm は適用・不適用・seed を問わず全て `arm_retirer.retire`**。`frame_capacity > bound.max_frames` の arm は apply せず retire + `Err(Ipc)` バックストップ（本線 gate は control 側 `Activate` — この `Err(Ipc)` は endpoint 内部処理で「unrepresentable block」として pass-through 収束し、公開面の `Err` として露出しない）
- **`EndpointInner::L1::process` の per-block dims pre-gate（`Err(Ipc)` 非露出の pin）**: leg `process` 呼出し前に `frames ≤ leg.armed_frame_capacity()`（現装着 staging の capacity — `RtLeg` 側 accessor で取得）を検査し、over-activated dims は **`Err(Prepare(UnsupportedConfig))`** で返す — 現行 `block_data` が同条件を `Err(Ipc)` で返すのに先行する pre-gate。**`Err(Ipc)` は L1 wire 内部の「wire が表現不能なブロック」意味に留め、`EndpointInner::L1::process`/`process_audio` の公開面から一切露出しない**（`prepare_endpoint` 系の defined rejection は全て `Err(Prepare(UnsupportedConfig))` 系 — T4 統一表と整合）
- **`drain_endpoint_channels(&mut self)`**（`L1Instance` — 全 `&mut` non-RT entry point 冒頭）: `pending_arm` resend（`arm_tx.try_push` — 成功なら `pending_arm=None`・`Full` なら残す）+ `return_rx.try_pop` → `self.rt = Some(leg)`（leg 復元）+ `arm_collector.drain()`（destructor 実行 — non-RT）。**leg 復元時は per-attach field を全て clear する**: `return_rx` pop で `self.rt = Some(leg)` としたら `arm_tx = None`・`pending_arm = None`・`return_rx = None`・`arm_collector = None`・`endpoint_bound = None` — 旧 attach の channel endpoint は endpoint drop と共に死んでおり、stale producer/consumer・stale bound 簿記を残さない（次 attach は `prepare_endpoint` が新規 mint で配線）。呼出し点: `L1Instance::execute`・`L1Instance::prepare_endpoint`・`L1Instance::destroy`/teardown・`PluginHost::try_recv_notification(&mut)`（composite 側 — `&self` 版は drain しない defined non-drain）
- **host `process` の wait-free 部分**: `L1Instance::process` は `arm_collector.drain()` を除く wait-free 部分のみ（`pending_arm` resend + `return_rx` pop → `self.rt` 復元）を `rt.is_none()` early-return **より前**に実行 — detached 中は drain 後に `Err(EndpointUnavailable)`。destructor 実行（`arm_collector.drain()`）を RT-suitable 経路に入れない
- **`&self` query は defined non-drain**: `L1Instance::lifecycle_state`/`latency_frames`/`L1Instance::try_recv_notification`（`&self`）は drain しない観測点（`&self` では `pending_arm`/`self.rt` に触れられない — tier 境界を test で pin）
- **endpoint `Drop` → leg 返却**: `L1Endpoint::drop` で `leg.take()` を `return_tx.try_push` — `TryPushError::Full`（cap 1 超過）のみ失敗形で `into_inner()` 回収した leg は endpoint を drop したスレッド上でそのまま drop（unit test で pin — RT スレッド上の endpoint drop は leg 自体の drop を RT に持ち込むため、本來 RT callback 内での endpoint drop は発生させない — plan drop 連鎖を介す）。leg 喪失（host 側 channel endpoint 既 drop）は再 attach 不能の defined 終端
- **`latency_frames` 内側**: `L1Instance::latency_frames`（`&self`）— `shared.lock().staging` が `Some`（activated 済）なら `Some((depth-1) × staging.frame_capacity)`・`None` なら `None` — T4 の公開面がこれを呼ぶ
- **単体テスト**: `rt: Option<RtLeg>` take・`Shared::rt_state` の Arc 化（endpoint mirror 観測が動く）・**`prepare_endpoint` Err 契約** — post-take Err で `self.rt` が復元されインスタンスが attachable/processable のまま・bad bounds は take 前に `Err(Prepare(UnsupportedConfig))`（leg 不動）・seed arm の generation 規則（同 generation は no-op・上回る generation は legitimately apply — detached-Activate 経路）・generation 単調 apply（stale arm 拒否）・全 drained arm → `Retirer`（DropFlag probe — `arm_collector.drain()` で立つ）・`pending_arm` latest-wins（Full 時の中間世代喪失）・`return_rx` drain → `self.rt` 復元 **+ per-attach field 全 clear** → 再 attach・endpoint `Drop` の `try_push` + `Full` → `into_inner` drop・detached `process` → `Err(EndpointUnavailable)`（wait-free drain は済み）・再 Activate bound gate（`rt.is_none()` が authoritative）・`capacity > bound` arm の `Err(Ipc)` バックストップ（内部処理 — 公開面非露出）・`armed_frame_capacity` pre-gate で over-activated dims が `Err(Prepare(UnsupportedConfig))`（`Err(Ipc)` が `process`/`process_audio` から漏れない）・`latency_frames` 値（activated → `Some(lookahead × cap)`・未 → `None`）

## Classification

- `normal-plan-task`（L1 endpoint 機構の中核 — design の決定サマリ i–k 実装・l1 構造変更を含む）

## Upstream Documents

- `docs/plans/implement-issue-113-engine-plugin-integration.md`（actions 8–11・Files/Ownership）
- `docs/designs/engine-plugin-host-integration.md`（L1 leg 移出・arm channel 5 点固定・`RtLeg::arm_apply` 簿記分割・`pending_arm`/tier 境界・leg 返却 + drain-before-early-return・bounded 性）
- `docs/specs/engine-plugin-host-integration.md`（AC2・AC4・AC5・AC8 — Send 証拠・bounds/error・退役）
- `docs/designs/plugin-l1-out-of-process.md`（Issue #76 上流 — `L1Instance`/`RtLeg`/`Staging`/`arm`/`process` の既存構造・pass-through 規則）
- `docs/adr/plugin-rt-ownership-split.md`（leg を graph node 所有へ移す確定分岐）

## Dependencies

- `docs/plans/implement-issue-113-engine-plugin-integration.md`
- `docs/tasks/plugin-core-endpoint-primitives.md`（`EndpointUnavailable`・view impl が前提）
- `docs/tasks/l0-shared-slot-and-endpoint-inner.md`（T2 — `src/endpoint.rs` の `EndpointInner` 骨格・`EndpointBounds` 公開型を T2 が先に作る。`L1Endpoint` は `endpoint.rs` で `crate::l1::data::EndpointLegParts` から組立てるため T2 の骨格が前提）

## Affected Files

- `crates/ruviform-plugin-host/src/l1/mod.rs` — `rt: Option<RtLeg>`・`Shared::rt_state: Arc<AtomicU8>`・arm/return/collector/`endpoint_bound` field・`pub(crate) prepare_endpoint`（`EndpointLegParts` 返却）・Activate 時 `try_push`/`pending_arm`・再 Activate bound gate（`rt.is_none()` authoritative）・`drain_endpoint_channels`（leg 復元 + per-attach field 全 clear）+ `process` wait-free 部分 + `rt.is_none()` early-return 順序・`latency_frames` 内側
- `crates/ruviform-plugin-host/src/l1/data.rs` — `RtLeg::arm_apply` 新設（簿記分割）+ `EndpointLegParts`（`pub(crate)` struct）+ `RtLeg::armed_frame_capacity` accessor + 単体テスト
- `crates/ruviform-plugin-host/src/endpoint.rs` — `EndpointInner::L1` variant + `L1Endpoint` struct（`EndpointLegParts` から組立て）+ arm drain + `armed_frame_capacity` pre-gate + endpoint `Drop` → `return_tx.try_push`（T2 の骨格を拡張）
- `crates/ruviform-plugin-host/src/composite.rs` — `PluginHost::try_recv_notification(&mut)` が `drain_endpoint_channels` を呼ぶ配線（`&self` 版は non-drain 維持）
- `crates/ruviform-plugin-host` 内 inline `#[cfg(test)] mod tests` — 新規単体テスト
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` — `#[path]` include の制約証人（crate-free discipline 維持の確認 — 変更不要のはずだが `src/l1` 構造変更で `mod` 宣言が増える場合に整合確認）

## Done When

- [ ] `L1Instance.rt` が `Option<RtLeg>` で、`prepare_endpoint` が `take` で leg 移出する — `rt.is_none()` の間の再 `prepare_endpoint` は `Err(EndpointUnavailable)`
- [ ] `src/l1/` 木の crate-free discipline が保持される（`crate::`/`ruviform_plugin_host::` パスを導入しない — `tests/l1_proxy_e2e.rs` `#[path]` include が継続して機能）。`L1Instance::prepare_endpoint` は `EndpointLegParts`（`pub(crate)`・l1 名指し可能なプリミティブ束）を返し、`L1Endpoint`/`EndpointBounds` を l1 から名指さない
- [ ] `prepare_endpoint` の順序が pin される: (1) state check（`Activated`/`Processing`）→ (2) bounds validation（非 0・`≤ WIRE_MAX_*`・`bound ≥ armed staging.frame_capacity` — take 前）→ (3) `rt.take()` → (4+) channel mint/seed/reserve/簿記 → (8) `EndpointLegParts` 返却。**(3) 以降の全 `Err` 経路は `self.rt = Some(leg)` で leg を復元**し、bad bounds は take 前に `Err(Prepare(UnsupportedConfig))`（leg 不動）— Err 後もインスタンスは attachable/processable
- [ ] `Shared::rt_state` が `Arc<AtomicU8>` で endpoint が同じ `Arc` を持ち、`set_mirror`/更新点が不変
- [ ] `prepare_endpoint` が arm channel（cap 2）・`retire_channel`（cap 4）・leg 返却 channel（cap 1）を mint し、`Inner::staging` clone の seed arm を `try_push` し、leg `last_output` を `bound.max_frames × WIRE_MAX_CHANNELS` へ事前 `reserve` する
- [ ] seed arm は `seed.generation > leg の現装着 staging.generation` のときのみ apply — 同 generation のときのみ正規化 no-op（detached-Activate で `Inner::staging` が新世代の場合は legitimately apply される）
- [ ] `Activate` 成功時に `arm_tx.try_push`（`Full` → `pending_arm` latest-wins）・再 Activate gate は **`self.rt.is_none()` が authoritative 判定**（`endpoint_bound` 簿記は比較値のみ）で bound 超過時 `Err(Prepare(UnsupportedConfig))`
- [ ] endpoint が `process` 冒頭で arm drain（bounded ≤3 pop・`generation > 現装着` のみ `arm_apply`・全 drained arm → `Retirer`・`frame_capacity > bound` は `Err(Ipc)` バックストップ — 公開面非露出）+ leg 呼出し前に `frames ≤ leg.armed_frame_capacity()` pre-gate で over-activated dims を `Err(Prepare(UnsupportedConfig))`（`Err(Ipc)` が `process`/`process_audio` 公開面から露出しない）
- [ ] `drain_endpoint_channels` が全 `&mut` entry（`execute`/`prepare_endpoint`/`destroy`/teardown/composite `try_recv_notification(&mut)`）の冒頭で走り、`process` が wait-free 部分のみで `rt.is_none()` early-return より前に drain する — `&self` query は defined non-drain
- [ ] `return_rx` pop で leg 復元時に per-attach field を全て clear（`arm_tx`/`pending_arm`/`return_rx`/`arm_collector`/`endpoint_bound` = None）— stale producer/consumer・stale bound 簿記を残さない
- [ ] `RtLeg::arm_apply` が ring 束縛のみ reset（seq 空間簿記持続）し旧 ring を `Retirer` へ — 現行 `RtLeg::arm` は不変
- [ ] endpoint `Drop` が `leg.take()` → `return_tx.try_push`（`Full` → `into_inner` drop）— leg が `return_rx` drain で `self.rt` 復元し再 attach 可能
- [ ] endpoint が child process/shm segment/mutex/`L1Instance` 参照を持たない（`Send` を型構造のみで担保）
- [ ] narrow gate（下記 Verification）が exit 0 — L1 単体テスト一式が通る

## Verification

- [ ] `mise run check` — exit 0
- [ ] `mise run lint` — exit 0
- [ ] `mise run test` — exit 0（既存 L1 テスト無退行 + 新規 L1 単体テスト）
- [ ] `mise exec -- cargo test -p ruviform-plugin-host` — L1 単体一式（leg take・seed arm・generation apply・drained arm retire・`pending_arm`・leg 返却・detached `process`・再 Activate gate・tier 境界）
- [ ] 対象 test-spec: `docs/test-specs/plugin-endpoint-contract-tests.md` — 規則「L1 leg 移出と arm 経路」「`RtLeg::arm_apply` 簿記分割」「leg 返却 + drain-before-early-return」「再 Activate bound gate」「単一 endpoint 規則（L1 側）」を担保
- [ ] 対象 test-spec: `docs/test-specs/plugin-rt-retirement-tests.md` — 規則「bounded 性」「tier 境界」の host 側（`drain_endpoint_channels`・`process` wait-free 部分）を担保
