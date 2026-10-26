---
id: "TASK-034MrsLWkKNHnu7jX1ghp8"
type: "task"
status: "done"
title: "L1 endpoint（leg 移出・arm channel・pending_arm・leg 返却・drain-before-early-return）"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-113-l1-endpoint-arm-and-leg-return-task-3-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/endpoint.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/data.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "docs/designs/engine-plugin-host-integration.md"
      - type: "file"
        path: "docs/plans/implement-issue-113-engine-plugin-integration.md"
      - type: "file"
        path: "docs/tasks/l1-endpoint-arm-and-leg-return.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/test-specs/plugin-endpoint-contract-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-rt-retirement-tests.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  implemented-by:
    - "docs/impl/ir/issue-113-l1-endpoint-arm-and-leg-return-task-3-implementation.md"
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
- **seed arm の apply 条件（generation 規則の正確化）**: seed arm は **`seed.generation > leg の現装着 staging.generation` のときのみ apply される — generation が一致する場合に限り正規化 no-op（apply されず unapplied のまま retire）**。通常経路（`Activate`→`prepare_endpoint`）では `Inner::staging` clone = leg 装着 ring と同一 `Arc`・同 generation で no-op。**detached-Activate**（attach 生存中の再 `Activate` で `Inner::staging` が新 generation へ進み、endpoint 側に未着のまま leg が `return_rx` drain で還流した場合）は review 修正で **host 側 drain が復元 leg を `Inner::staging` へ re-arm** するようになったため、還流後の再 attach の seed は常に同 generation の no-op — endpoint 側 drain の `generation > armed` apply が実際に新 ring を適用するのは **endpoint 生存中の detached-Activate で arm channel に届いた arm** のみ（偽の no-op 前提を書かない）
- **Activate 時の arm 供給**: `L1Instance::execute(Activate)` 成功時（新 `Arc<Staging>` 生成後）に `arm_tx` が存在すれば `try_push` — `Full` なら `pending_arm` へ latest-wins 退避（前の退避を drop して最新のみ保持 — `DropCollector` 行き不要・`pending_arm` は host 側変数なので drop は non-RT）
- **再 Activate bound gate**: **attach 生存の authoritative 判定は `self.rt.is_none()`（leg 移出中）** — `endpoint_bound.is_some()` ではない（`endpoint_bound` は簿記であり、leg 返却後に clear され得るため liveness 判定として使わない）。`self.rt.is_none()` かつ `new_config.max_frames > endpoint_bound.0`（簿記した bound の `max_frames`）→ `Err(HostError::Prepare(UnsupportedConfig))` — arm 生成・`try_push` の前に拒否（比較対象は `endpoint_bound` の簿記値 — 現行 activated config ではない）
- **endpoint 側 arm drain**（`L1Endpoint` の process 冒頭 — `EndpointInner::L1` arm）: `arm_rx` を bounded drain（1 pass の pop ≤ `arm_capacity + 1` = 3）— 各 popped `Arc<Staging>` について `generation > 現装着 staging.generation` なら `arm_apply`（適用）・それ以外は unapplied — **drain した arm は適用・不適用・seed を問わず全て `arm_retirer.retire`**。`frame_capacity > bound.max_frames` の arm は apply せず retire + pass-through 収束（`leg.pass_through(outputs)` + `Ok(CONTINUE)` — 本線 gate は control 側 `Activate` で in-protocol 不到達の内部バックストップ。`Err(Ipc)` は endpoint 公開面に一切露出しない — review 修正で `Err(Ipc)` return から pass-through へ収束）
- **`EndpointInner::L1::process` の per-block dims pre-gate（`Err(Ipc)` 非露出の pin）**: leg `process` 呼出し前に `frames ≤ leg.armed_frame_capacity()`（現装着 staging の capacity — `RtLeg` 側 accessor で取得）+ **`channels == bound.channels`**（L0 gate の mirror — review 修正で追加）を検査し、違反は **`Err(Prepare(UnsupportedConfig))`** で返す — 現行 `block_data` が同種の条件を `Err(Ipc)` で返すのに先行する pre-gate。**`min_frames` は child 側 plugin 契約に委譲**（armed `Staging` が保持しない — test-spec の defined-rejection 表が pin する deliberate divergence・`out.channels() == bound.channels`/`frames ≤ bound.max_frames` の bound clip は T6 の `process_audio` が所有）。**`Err(Ipc)` は L1 wire 内部の「wire が表現不能なブロック」意味に留め、`EndpointInner::L1::process`/`process_audio` の公開面から一切露出しない**（`prepare_endpoint` 系の defined rejection は全て `Err(Prepare(UnsupportedConfig))` 系 — T4 統一表と整合）
- **`drain_endpoint_channels(&mut self)`**（`L1Instance` — 全 `&mut` non-RT entry point 冒頭）: `pending_arm` resend（`arm_tx.try_push` — 成功なら `pending_arm=None`・`Full` なら残す）+ `return_rx.try_pop` → `self.rt = Some(leg)`（leg 復元）+ `arm_collector.drain()`（destructor 実行 — non-RT）+ **leg 復元の仕上げ（non-RT のみ — review 修正）**: 復元された leg が `Inner::staging` より古い generation を装着している場合（detached-Activate を endpoint が未消費）`rt.arm(inner.staging)` で re-arm（そのままだと以後の submit が全て driver の `armed_for` で drop され恒久的に pass-through + `latency_frames` 乖離 — wait-free restore から本 drain までの一過性 stale submit は pass-through して自然回復）+ **per-attach field を全て clear**: `arm_tx = None`・`pending_arm = None`・`return_rx = None`・`arm_collector = None`・`endpoint_bound = None` — 旧 attach の channel endpoint は endpoint drop と共に死んでおり、stale producer/consumer・stale bound 簿記を残さない（次 attach は `prepare_endpoint` が新規 mint で配線）。clear の検出子は `rt.is_some() && return_rx.is_some()`（attach 生存中は `rt` が `None`）。呼出し点: `L1Instance::execute`・`L1Instance::prepare_endpoint`・`L1Instance::destroy`/teardown・`PluginHost::try_recv_notification(&mut)`（composite 側 — `&self` 版は drain しない defined non-drain）
- **host `process` の wait-free 部分**: `L1Instance::process` は wait-free 部分のみ（`pending_arm` resend + `return_rx` pop → `self.rt` 復元 **のみ** — `arm_collector.drain()` も `clear_endpoint_fields` も除く — review 修正で leg 復元は destructor ゼロ・stale channel ends は non-RT drain が片付ける）を `rt.is_none()` early-return **より前**に実行 — detached 中は drain 後に `Err(EndpointUnavailable)`。**restore は非 terminal（mirror ≠ `Destroyed`）gate** — `Destroy` 後に返った leg は queued のまま channel ends と一緒に drop（bounded teardown）され、Destroyed インスタンスに復元しない。destructor 実行を RT-suitable 経路に入れない
- **`&self` query は defined non-drain**: `L1Instance::lifecycle_state`/`latency_frames`/`L1Instance::try_recv_notification`（`&self`）は drain しない観測点（`&self` では `pending_arm`/`self.rt` に触れられない — tier 境界を test で pin）
- **endpoint `Drop` → leg 返却**: `L1Endpoint::drop` で `leg.take()` を `return_tx.try_push` — `TryPushError::Full`（cap 1 超過）のみ失敗形で `into_inner()` 回収した leg は endpoint を drop したスレッド上でそのまま drop（unit test で pin — RT スレッド上の endpoint drop は leg 自体の drop を RT に持ち込むため、本來 RT callback 内での endpoint drop は発生させない — plan drop 連鎖を介す）。leg 喪失（host 側 channel endpoint 既 drop）は再 attach 不能の defined 終端
- **`latency_frames` 内側**: `L1Instance::latency_frames`（`&self`）— `shared.lock().staging` が `Some`（activated 済）なら `Some((depth-1) × staging.frame_capacity)`・`None` なら `None` — T4 の公開面がこれを呼ぶ
- **単体テスト**: `rt: Option<RtLeg>` take・`Shared::rt_state` の Arc 化（endpoint mirror 観測が動く）・**`prepare_endpoint` Err 契約** — post-take Err で `self.rt` が復元されインスタンスが attachable/processable のまま・bad bounds は take 前に `Err(Prepare(UnsupportedConfig))`（leg 不動）・seed arm の generation 規則（同 generation は no-op — review 修正後、還流 leg は host 側 drain が `Inner::staging` へ re-arm するため seed は常に同 generation）・generation 単調 apply（stale arm 拒否）・全 drained arm → `Retirer`（DropFlag probe — `arm_collector.drain()` で立つ）・`pending_arm` latest-wins（Full 時の中間世代喪失）・`return_rx` drain → `self.rt` 復元（**wait-free `process` 経路では per-attach field を clear しない** — non-RT drain が re-arm + 全 clear）→ 再 attach・**`Destroyed` mirror への leg 復元拒否**（queued のまま teardown で drop）・**driver drop seq の synthesized `mark_failed` completion が slot gate を解放**（`Shared::push_result` — 共有 result producer）・endpoint `Drop` の `try_push` + `Full` → `into_inner` drop・detached `process` → `Err(EndpointUnavailable)`（wait-free drain は済み）・再 Activate bound gate（`rt.is_none()` が authoritative）・`capacity > bound` arm の pass-through 収束バックストップ（内部処理 — 公開面非露出）・`armed_frame_capacity` + `channels == bound.channels` pre-gate で dims 違反が `Err(Prepare(UnsupportedConfig))`（`Err(Ipc)` が `process`/`process_audio` から漏れない）・`latency_frames` 値（activated → `Some(lookahead × cap)`・未 → `None`）

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

- [x] `L1Instance.rt` が `Option<RtLeg>` で、`prepare_endpoint` が `take` で leg 移出する — `rt.is_none()` の間の再 `prepare_endpoint` は `Err(EndpointUnavailable)`（`prepare_endpoint_takes_leg_and_blocks_reattach` で pin）
- [x] `src/l1/` 木の crate-free discipline が保持される（`crate::`/`ruviform_plugin_host::` パスを導入しない — `tests/l1_proxy_e2e.rs` `#[path]` include が継続して機能・e2e **54** tests pass）。`L1Instance::prepare_endpoint` は `EndpointLegParts`（`pub(crate)`・l1 名指し可能なプリミティブ束 — `data.rs` の `leg`/`mirror`/`arm_rx`/`arm_retirer`/`return_tx`）を返し、`L1Endpoint`/`EndpointBounds` を l1 から名指さない（`L1Endpoint` 組立ては `endpoint.rs` の `From<EndpointLegParts>`）
- [x] `prepare_endpoint` の順序が pin される: (1) state check（`Activated`/`Processing`）→ (2) bounds validation（非 0・`≤ WIRE_MAX_*`・`bound ≥ armed staging.frame_capacity` — take 前）→ (3) `rt.take()` → (4+) channel mint/seed/reserve/簿記 → (8) `EndpointLegParts` 返却。**(3) 以降の全 `Err` 経路は `self.rt = Some(leg)` で leg を復元**し、bad bounds は take 前に `Err(Prepare(UnsupportedConfig))`（leg 不動）— Err 後もインスタンスは attachable/processable（`prepare_endpoint_rejections_leave_leg_home` で pin — state 5 状態 + bounds 5 条件全て leg 不動）
- [x] `Shared::rt_state` が `Arc<AtomicU8>` で endpoint が同じ `Arc` を持ち、`set_mirror`/更新点が不変（`Arc::ptr_eq(&parts.mirror, &instance.shared.rt_state)` を `prepare_endpoint_takes_leg_and_blocks_reattach` で検証）
- [x] `prepare_endpoint` が arm channel（cap 2）・`retire_channel`（cap 4）・leg 返却 channel（cap 1）を mint し、`Inner::staging` clone の seed arm を `try_push` し、leg `last_output` を `bound.max_frames × WIRE_MAX_CHANNELS` へ事前 `reserve` する（`endpoint_channel_setup` — `RtLeg::reserve_last_output` 新設）
- [x] seed arm は `seed.generation > leg の現装着 staging.generation` のときのみ apply — 同 generation のときのみ正規化 no-op（review 修正: detached-Activate で還流した leg は host 側 drain が `Inner::staging` へ re-arm するため、再 attach の seed は常に同 generation no-op — endpoint 側の `generation > armed` apply は生存 endpoint の arm channel 配送のみが担う — `restored_leg_rearms_to_the_newer_staging` で pin）
- [x] `Activate` 成功時に `arm_tx.try_push`（`Full` → `pending_arm` latest-wins — `pending_arm_is_latest_wins_and_retries` で中間世代喪失を pin）・再 Activate gate は **`self.rt.is_none()` が authoritative 判定**（`endpoint_bound` 簿記は比較値のみ）で bound 超過時 `Err(Prepare(UnsupportedConfig))`（`activate_above_attach_bound_is_rejected_while_detached`）
- [x] endpoint が `process` 冒頭で arm drain（bounded ≤ `ARM_DRAIN_MAX`=3 pop・`generation > 現装着` のみ `arm_apply`・全 drained arm → `Retirer`・`frame_capacity > bound` は pass-through 収束バックストップ — `Err(Ipc)` 公開面非露出）+ leg 呼出し前に `frames ≤ leg.armed_frame_capacity()` + `channels == bound.channels` pre-gate で dims 違反を `Err(Prepare(UnsupportedConfig))`（`Err(Ipc)` が `process`/`process_audio` 公開面から露出しない — `l1_endpoint_dims_gate_never_surfaces_ipc`・`l1_endpoint_wire_unrepresentable_maps_to_prepare`・`l1_endpoint_over_bound_arm_passes_through` — `min_frames` は child 側 plugin 契約に委譲の deliberate divergence）
- [x] `drain_endpoint_channels` が全 `&mut` entry（`execute`/`prepare_endpoint`/`destroy`/teardown/composite `try_recv_notification(&mut)`）の冒頭で走り、`process` が wait-free 部分のみ（`drain_endpoint_channels_wait_free` — resend + restore のみ・destructor/clear を含まない）で `rt.is_none()` early-return より前に drain する — `&self` query は defined non-drain（`detached_process_returns_endpoint_unavailable_then_restores` で `lifecycle_state`/`try_recv_notification`/`latency_frames` 非 drain を pin）
- [x] non-RT `drain_endpoint_channels` が leg 復元の仕上げを行う（wait-free 復元は `self.rt = Some(leg)` のみ — stale ends 不動）: 復元 leg が `Inner::staging` より古い generation なら `rt.arm` で re-arm + per-attach field を全て clear（`arm_tx`/`pending_arm`/`return_rx`/`arm_collector`/`endpoint_bound` = None — rtrb ring free・`Arc<Staging>` destructor は non-RT のみ）— stale producer/consumer・stale bound 簿記を残さない。**mirror `Destroyed` では復元しない**（queued leg は teardown で drop — `leg_returned_after_destroy_is_not_restored`）・driver が doorbell せず drop した seq は `mark_failed` completion を synthesize し slot gate を wedged にしない（`Shared::result_tx: Mutex<Producer<u64>>` + `push_result` — `dropped_submit_result_frees_alternation_slot`）
- [x] `RtLeg::arm_apply` が ring 束縛のみ reset（seq 空間簿記持続）し旧 ring を `Retirer` へ — 現行 `RtLeg::arm` は不変（`arm_apply_swaps_ring_and_keeps_seq_bookkeeping` で両側 pin）
- [x] endpoint `Drop` が `leg.take()` → `return_tx.try_push`（`Full` → `into_inner` drop）— leg が `return_rx` drain で `self.rt` 復元し再 attach 可能（`l1_endpoint_drop_returns_leg`・`l1_endpoint_drop_full_recovers_on_thread`・composite 側 `attach_endpoint_routes_l1_slots_to_the_leg` で再 attach を pin）
- [x] endpoint が child process/shm segment/mutex/`L1Instance` 参照を持たない（`Send` を型構造のみで担保 — `expect_send::<L1Endpoint>()`/`expect_send::<EndpointLegParts>()` を `endpoint_types_are_send` に追加・`unsafe impl` なし）
- [x] narrow gate（下記 Verification）が exit 0 — L1 単体テスト一式が通る（lib **55** tests・新規 **25** 件全 pass）

## Verification

- [x] `mise run check` — exit 0
- [x] `mise run lint` — exit 0（clippy `--all-targets -D warnings` clean — `EndpointInner::L1` の `Box` 化は `large_enum_variant` 対応）
- [x] `mise run test` — exit 0（`cargo test --workspace` — **55 test binaries 全 pass・0 fail** — 既存 L1 テスト無退行 + 新規 L1 単体テスト **25** 件: l1/mod.rs **10**（leg take・pinned rejection・detached process・pending_arm・Activate gate・re-arm・Destroyed gate・synthesized completion・destroy bounded・rt_ drain）+ data.rs **2**（`arm_apply` 簿記分割 + `rt_l1_arm_apply_no_alloc`）+ endpoint.rs **11**（pass-through・Failed 収束・非 Processing 拒否・dims pre-gate・Ipc remap・arm drain generation gate・over-bound backstop・Drop leg 返却 + Full 回収・rt_ 2 件）+ composite.rs **2**（route-aware attach L0/L1））
- [x] `mise exec -- cargo test -p ruviform-plugin-host` — exit 0（lib **55** + `l1-test-worker` **0** + `l1_proxy_e2e` **54** + doctest **1** — 全 pass・新規 25 件を含む。load-sensitive 2 件（`process_roundtrip_gain_and_lookahead_delay`・`wedged_child_write_converges_and_teardown_is_bounded`）も本 run で pass）
- [x] 対象 test-spec: `docs/test-specs/plugin-endpoint-contract-tests.md` — 規則「L1 leg 移出と arm 経路」「`RtLeg::arm_apply` 簿記分割」「leg 返却 + drain-before-early-return」「再 Activate bound gate」「単一 endpoint 規則（L1 側）」を担保（本 task で review 修正込みの最新形へ in-place 更新済み）
- [x] 対象 test-spec: `docs/test-specs/plugin-rt-retirement-tests.md` — 規則「bounded 性」「tier 境界」の host 側（`drain_endpoint_channels`・`process` wait-free 部分）を担保（本 task で review 修正込みの最新形へ in-place 更新済み）

補足（task doc の Verification 範囲外・IR に記録）: `mise run fmt-check`/`mise run build` exit 0・`mise run test-rt` exit 0（`rt_` フィルタ実一致: lib **8** 件 — 新規 `rt_l1_endpoint_process_no_alloc`/`rt_l1_endpoint_arm_drain_no_alloc`/`rt_l1_detached_process_no_alloc`/`rt_l1_arm_apply_no_alloc` + 既存 3 件 + 副次一致 `shared_start_failure_rolls_back_to_activated`・e2e バイナリ **4** 件 — `rt_l1_process_path_no_alloc`/`rt_l1_result_leg_try_ops` + `#[path]` include 経由の `l1::` 新規 2 件）。**review-driven 修正 7 件適用済み**（wait-free drain 分割・non-RT drain re-arm・共有 `Mutex<Producer<u64>>` result channel・over-bound arm pass-through 収束・`channels == bound.channels` gate + `min_frames` child 委譲 pin・`Destroyed` restore gate・composite doc + `arm_retire_overflows` 観測点）— 詳細は `docs/impl/ir/issue-113-l1-endpoint-arm-and-leg-return-task-3-implementation.md`。
