---
id: "TASK-034MrsUoYlicb5uG8XpODT"
type: "task"
status: "todo"
title: "PluginHost endpoint 公開面（prepare_endpoint・latency_frames・統一拒否表・crate doc）"
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
    - "docs/tasks/l0-shared-slot-and-endpoint-inner.md"
    - "docs/tasks/l1-endpoint-arm-and-leg-return.md"
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
  references: []
  defers: []
  deferred-by: []
---

# PluginHost endpoint 公開面（prepare_endpoint・latency_frames・統一拒否表・crate doc）

## Work

Plan actions 12–13。`PluginHost` コンポジットに endpoint の**公開面**を集約する — `prepare_endpoint`/`latency_frames` の public API・L0/L1 統一 defined-rejection 表・`EndpointUnavailable` の wiring・crate doc（endpoint 面・遅延契約・video NONRT 境界 — AC10）・`composite.rs` の旧遅延式 doc 修正。機構自体は T2/T3 で実装済み — 本 task は公開 API 形状・統一 rejection・doc のみを所有する。

- **`PluginHost::prepare_endpoint(&mut self, handle: PluginHandle, bounds: EndpointBounds) -> Result<PluginEndpoint, HostError>`**: `Slot::InProcess` → `InProcessHost::attach_endpoint`（T2 の `pub(crate)` 内側を呼ぶ）・`Slot::OutOfProcess` → `L1Instance::prepare_endpoint`（T3 の `pub(crate)` 内側を呼ぶ）。**video は endpoint 経路の対象外** — `PluginHost` の slot 表は `Slot::InProcess`/`Slot::OutOfProcess` のみで video instance を routing しないため、`prepare_endpoint` に video 分岐は存在しない（`Slot::Video` 相当の行は dead row — 表には「video handle は `PluginHost` から発行されない＝到達不能」を記すのみ。`Err(Unsupported)` 分岐を設けない）。**本 task が L0/L1 共通の bounds-validation prelude を所有する** — `bounds` の uniform 検査（`max_frames`/`channels` 非 0・`max_frames ≤ WIRE_MAX_FRAMES`(8192)・`channels ≤ WIRE_MAX_CHANNELS`(16)・`max_frames ≥ activated max_frames`）を 1 回行い、違反は `Err(Prepare(UnsupportedConfig))`（`u32::MAX` も同じ defined rejection — allocation abort にしない）。**prelude の実行位置は下記拒否順序の (3) bounds 位置 — 「全検査より先」ではない**: composite は handle → slot を resolve し、既存の per-slot routing・`lifecycle_state`/mirror 読みで (1) handle/state 系・(2) slot-shape/attach-liveness を先に判定してから (3) の uniform bounds 検査を行い、最後に slot 内側 attach を呼ぶ（composite 側の state 読みに新 accessor は不要 — `PluginHost` は既に per-slot に route し lifecycle を読む）。T2/T3 内側の bounds 検査は activated 層の defense-in-depth 再検査として残る（authoritative gate は本 prelude — L1 では leg 移出前に内側でも再検査済み）。state 検証（`Activated`/`Processing` 要求）は両層内側に既存 — 本 task は **統一 defined-rejection 表**を公開 API レベルで pin する。**拒否順序: (1) state 系 → (2) slot-shape/presence → (3) bounds**:
  - (1) `state < Activated`（`Created`/`Initialized`）→ `Err(HostError::Lifecycle(_))`・`Failed` → `Err(HostError::Failed)`・`Destroyed`/stale handle → `Err(InvalidHandle)`
  - (2) L0 `Plain` slot（非 Send factory・`create_sendable() == None`）→ `Err(EndpointUnavailable)`・L0 2 本目 attach（`attached != 0` epoch CAS 失敗）→ `Err(EndpointUnavailable)`・L1 leg 未返却（`rt.is_none()`）→ `Err(EndpointUnavailable)`
  - (3) bounds 各種違反（`max_frames==0`/`channels==0`/`max_frames < activated`/`> WIRE_MAX_FRAMES`(8192)/`> WIRE_MAX_CHANNELS`(16)）→ `Err(Prepare(UnsupportedConfig))` — `channels` は v1 では activated main bus との等値を要求しない（design 修正 §bounds.channels — channel query 未存在のため non-0・wire 上限・per-block `out.channels() == bound.channels` 契約）
  - `Err(Ipc)` は L1 内部の wire 表現不能のみに留め、公開面の拒否として露出しない（`EndpointInner::L1::process`/`process_audio` の `Err` 経路は全て `Prepare(UnsupportedConfig)` 系 — `armed_frame_capacity` pre-gate による dims 拒否含む）
- **`PluginHost::latency_frames(&self, handle: PluginHandle) -> Option<u32>`**: `Slot::InProcess` → `Some(0)`（activated 済のとき — `lifecycle_state` が `Activated`/`Processing` または L0 `activated.is_some()` を根拠。`&self` で mirror/lifecycle 読み — **L0 `core` mutex の blocking lock は不許可**（RT-lane に持ち込まれる照会のため `try_lock` 相当の観測のみ。L1 側の `shared.lock()` は non-RT control スレッドの mutex でありこの制約の対象外））・`Slot::OutOfProcess` → `L1Instance::latency_frames`（T3 の内側 — `shared.lock().staging` で `Some(lookahead_blocks × activated max_frames)`）・未 `Activate`/無効 handle → `None`。**修正版式** `lookahead_blocks × activated max_frames`（frames 整数）— 旧式（`× block_frames / sample_rate`）の残留を正す
- **`EndpointUnavailable` の wiring**: 公開面で `EndpointUnavailable` は「endpoint 不在」の意味で統一 — `Plain` スロット（非 Send）・epoch CAS 失敗・`rt.is_none()`（leg 未返却）の 3 分岐。`Ipc`/`Unsupported` へ潰さず local-only の topology 事実として返す
- **crate doc（`lib.rs` — T2 の re-export 骨格を doc 面で完成）**: endpoint 面の存在（`PluginHost` のみ公開 — standalone `InProcessHost` は direct-call のみ）・遅延契約（`frames / sample_rate` 秒換算・re-activation で変わり得る旨の明記）・**video NONRT 境界**（`VideoPluginInstance` は全操作 NONRT・本 issue の endpoint/RT 経路対象外 — AC10）・RT 契約（`process`/`process_audio` は `Send`・try-only・no alloc — `rt_` テストで担保）を crate-level doc に明記
- **遅延式 doc の修正（`composite.rs` + `lib.rs` — stale parity）**: 既存の遅延式 doc（`lookahead × block_frames / sample_rate` 形）を**修正版式** `lookahead_blocks × activated max_frames`（frames 整数・秒換算 `frames / sample_rate`）へ同期 — `composite.rs` の該当 doc 行に加え **`lib.rs` L44-49 付近の crate doc（`Latency contract (L1)` — `lookahead_blocks × block_frames / sample_rate`・`2 × 512 frames @ 48 kHz ≈ 21.3 ms` の stale 記述）も併せて修正する**（stale parity — `sample_rate` 除算ではなく frames 整数が正本）。実装相違があれば正本へ反映（design の確定式が正）
- **`plugin_mut` の公開面 parity**: `PluginHost::plugin_mut` が `Shared` L0 slot で `None` を返す（T2 で `InProcessHost::plugin_mut` が `Shared`→`None` — composite 側は透過）。L1 と同じ defined 挙動として doc に統一記述
- **composite 級単体テスト**: L0/L1 両スロットで `prepare_endpoint` → `Ok`・各 rejection 分岐（`Created`/`Failed`/`Destroyed`/stale/`Plain`/2本目/leg 未返却/bounds 各種 — `0`・`> WIRE_MAX_*`・`< activated`・`u32::MAX` の defined rejection 含む）が統一表どおりの `Err` 型を返す・**拒否順序**（state 系 → slot-shape/presence → bounds — state 違反 + bad bounds でも state 系 `Err` が勝つ）・L1 bad bounds で leg が移出されず再 attach 可能・`latency_frames` 値（L0 `Some(0)`・L1 activated `Some(lookahead×cap)`・未 `Activate`/`None`）・`plugin_mut` L0-Shared `None` parity・`EndpointUnavailable` が wire 型（`Ipc`/`Unsupported`）に潰れないこと

## Classification

- `normal-plan-task`（公開 API 面・統一表・doc — 機構は T2/T3 依存）

## Upstream Documents

- `docs/plans/implement-issue-113-engine-plugin-integration.md`（actions 12–13・Files/Ownership）
- `docs/designs/engine-plugin-host-integration.md`（「エラー・拒否面」統一表・`PluginHost` のみ公開面の確定・`latency_frames` 修正版式・video NONRT 境界）
- `docs/specs/engine-plugin-host-integration.md`（AC4・AC6・AC10 — bounds/error・latency・video 境界明文化）
- `docs/adr/plugin-rt-ownership-split.md`（`PluginHost` コンポジットのみが endpoint 面を持つ確定）

## Dependencies

- `docs/plans/implement-issue-113-engine-plugin-integration.md`
- `docs/tasks/l0-shared-slot-and-endpoint-inner.md`（`InProcessHost::attach_endpoint`・L0 inner・epoch が前提）
- `docs/tasks/l1-endpoint-arm-and-leg-return.md`（`L1Instance::prepare_endpoint`・leg 移出・`latency_frames` 内側が前提）

## Affected Files

- `crates/ruviform-plugin-host/src/composite.rs` — `PluginHost::prepare_endpoint`/`latency_frames` 公開 API・**bounds-validation prelude 所有**（拒否順序 (3) bounds 位置での uniform 検査 — state 系・slot-shape 判定の後・内側 attach 呼出し前）・統一 defined-rejection 表（state 系 → slot-shape/presence → bounds の順序 pin）・`plugin_mut` parity・旧遅延式 doc 修正
- `crates/ruviform-plugin-host/src/lib.rs` — crate doc 完成（endpoint 面・遅延契約・video NONRT 境界・RT 契約）+ **L44-49 付近の stale 遅延式 doc 修正**（`lookahead_blocks × block_frames / sample_rate` → frames 整数式）
- `crates/ruviform-plugin-host/src/endpoint.rs` — 公開 API の doc（`PluginEndpoint::latency_frames`/`bounds`/`max_frames` 等の getter 公開面）
- `crates/ruviform-plugin-host` 内 inline `#[cfg(test)] mod tests` + `tests/` — composite 級 rejection/latency テスト

## Done When

- [ ] `PluginHost::prepare_endpoint` が `Slot` で dispatch し、統一 defined-rejection 表（`Lifecycle`/`Failed`/`InvalidHandle`/`EndpointUnavailable`/`Prepare(UnsupportedConfig)`）どおりの `Err` 型を全分岐で返す — video 分岐は存在しない（`PluginHost` が video handle を発行しない dead row — `Err(Unsupported)` 分岐を設けない）
- [ ] **bounds-validation prelude を本 task が所有** — 拒否順序の (3) bounds 位置（(1) handle/state 系・(2) slot-shape/attach-liveness 判定の後・slot 内側 attach 呼出し前 — composite は既存の per-slot routing・lifecycle 読みで先行判定し新 accessor は不要）に `bounds` の uniform 検査（非 0・`≤ WIRE_MAX_FRAMES`(8192)/`≤ WIRE_MAX_CHANNELS`(16)・`≥ activated max_frames`）を行い、`u32::MAX` を含む違反を `Err(Prepare(UnsupportedConfig))` で返す — T2/T3 内側は activated 層の defense-in-depth 再検査として残る
- [ ] **拒否順序が pin される** — (1) state 系（`Lifecycle`/`Failed`/`InvalidHandle`）→ (2) slot-shape/presence（`EndpointUnavailable` — `Plain`/2本目/`rt.is_none()`）→ (3) bounds（`Prepare(UnsupportedConfig)`）。state 違反と bad bounds が同時でも state 系 `Err` が返る
- [ ] `PluginHost::latency_frames` が L0=`Some(0)`・L1=`Some(lookahead_blocks × activated max_frames)`・未 `Activate`/無効=`None` を返す（`&self`・L0 `core` mutex の blocking lock なし — L1 `shared.lock()` は non-RT 側 mutex で対象外）
- [ ] `EndpointUnavailable` が local-only topology 事実として公開され、`Ipc`/`Unsupported` へ潰れない・`Err(Ipc)` が `process`/`process_audio` の公開面から露出しない
- [ ] crate doc に endpoint 面（`PluginHost` のみ）・遅延契約・video NONRT 境界・RT 契約が明記される — `composite.rs` と `lib.rs` L44-49 付近の stale 遅延式 doc が修正版式へ同期される
- [ ] `plugin_mut` が L0-Shared slot で `None`（L1 と parity）を返す
- [ ] narrow gate（下記 Verification）が exit 0 — composite 級テスト一式が通る

## Verification

- [ ] `mise run check` — exit 0
- [ ] `mise run lint` — exit 0
- [ ] `mise run test` — exit 0（既存 composite テスト無退行 + 新規 composite 級テスト）
- [ ] `mise exec -- cargo test -p ruviform-plugin-host` — composite 級一式（統一 rejection・`latency_frames` 値・`plugin_mut` parity・`EndpointUnavailable` 型）
- [ ] 対象 test-spec: `docs/test-specs/plugin-endpoint-contract-tests.md` — 規則「`prepare_endpoint` 拒否面」「`latency_frames`」「単一 endpoint 規則」「`EndpointUnavailable` local-only」を担保
