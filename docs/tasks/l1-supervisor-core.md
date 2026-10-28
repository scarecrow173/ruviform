---
id: "TASK-034NOXIiAgzm2Ur1Ud7nLt"
type: "task"
status: "todo"
title: "L1 supervisor core（src/supervisor.rs — watch/poll state machine・attempt replay・checkpoint bound・host binding）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/116"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/supervisor.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-116-supervisor.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-116-supervisor.md"
  blocks:
    - "docs/tasks/l1-supervisor-e2e.md"
    - "docs/tasks/l1-supervisor-final-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/l1-test-worker-fault-flags.md"
    - "docs/tasks/l1-route-pool-and-isolation-surface.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-116-supervisor.md"
    - "docs/designs/issue-116-supervisor.md"
    - "docs/adr/issue-116-supervisor-placement.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-116-supervisor-core-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# L1 supervisor core（src/supervisor.rs — watch/poll state machine・attempt replay・checkpoint bound・host binding）

## Work

`crates/ruviform-plugin-host/src/supervisor.rs` を新設し、design 確定の公開 API・entry state machine・poll Phase 0-3・attempt/replay/cleanup・backoff・checkpoint bound・host binding を実装する。**`PluginHost` 公開 API（`execute`・`try_recv_notification`・`lifecycle_state`・`Scan` via `execute`）のみ使用 — `pub(crate)` L1 machinery（`l1::` モジュール・`Route`/`Slot`/pool 内部型）には一切触れない**（ADR の配置契約 — 同 crate 内のため型システムは遮らず、設計契約 + review 観点 + T3 の別 test target で担保）。core `PluginHost`/`composite.rs`/`l1/`/`host.rs` は無 diff（AC1/AC4 確定契約）。

1. **公開面** — design の API sketch（`docs/designs/issue-116-supervisor.md` の公開 API 面・補助コンストラクタ）を正本として最終確定:
   - `Supervisor{entries: BTreeMap<SupervisedId, Entry>, next_id, policy, bound_host: Option<usize>}` — `PluginHost` 非所有・sendable データのみ
   - `SupervisedId(u64)` — opaque・`Copy`+`Ord`（`(next_attempt_at, id)` 昇順ソート用）
   - `RestartPolicy{max_attempts: u32 = 3, base_delay: Duration = 250ms, max_delay: Duration = 30s, stable_window: Duration = 60s, max_attempts_per_poll: u32 = 8}` + `Default`
   - `RestartRecipe{plugin_id: String, config: Option<ProcessConfig>, start_processing: bool, endpoint_bounds: Option<EndpointBounds>}` + `new(plugin_id)`（`config:None`・`start_processing:false`・`endpoint_bounds:None` — `Initialized` までの最小復旧）
   - `RestoreOutcome{NotRequested, Applied, Skipped}`・`SupervisionEvent{Restarted{id,old,new,bounds,restored}, AttemptFailed{id,attempt,error}, GaveUp{id,attempts,last_error}, Unsupervised{notification}, WrongHost}`・`SupervisionError{InvalidHandle, AlreadyWatched, UnknownPluginId, NotWatched, WrongHost, CheckpointTooLarge, InertPolicy}`（`Copy` 維持 — non-`Copy` payload は別 Issue）
   - `#[non_exhaustive]`: 3 enum 全て enum レベル + `SupervisionEvent` struct variant も variant レベル（additive field 拡張余地 — design L325）
2. **entry 簿記** — `Entry{recipe, checkpoint, failures, last_error, lingering, state}`・`Live{handle, healthy_since: Option<Instant>}`（watch 登録時 `None`・`Restarted` 成功時 `Some(now)`）/`Pending{next_attempt_at: Option<Instant>, failed: PluginHandle}`/`GaveUp`・`find_watched` match-set（`Live.handle` current・`Pending.failed`・任意 state の `lingering`）
3. **API** — `new`（`max_attempts_per_poll == 0` → `Err(InertPolicy)` 拒否・clamp しない）・`watch`（`lifecycle_state → None` → `InvalidHandle`・`Scan` 検証 `UnknownPluginId`・match-set 一致 `AlreadyWatched`・`Some(Failed)` adopt — `Destroy{h}`（`Err(InvalidHandle)` 継続・キュー残通知は slot 解放で落ちる）→ `Pending{next:None, failed:h}`・初回 host 受取で `bound_host` 束縛）・`unwatch`（`Option<PluginHandle>` — `Live.current` または `lingering`・殺さない）・`set_recipe`（全置換・`Scan` 検証・`WrongHost`）・`set_checkpoint`（`bytes.len() + 17 > wire::WIRE_MAX_PAYLOAD` → `CheckpointTooLarge` — envelope = seq8+instance4+verb1+len4）・`clear_checkpoint`（`bool`）・`current_handle`（`Live` → `Some(current)`・`Pending`/`GaveUp`+lingering → `Some(lingering)`・else `None`）
4. **`poll` Phase 0-3** — Phase 0: `host as *const PluginHost as usize` 束縛/比較（不一致 → `WrongHost` emit のみ・drain/reset/attempt 一切しない）・Phase 1: `try_recv_notification` 全 drain — watched `InstanceFailed` は `Destroy{h}`（`Err` 無視）→ `Live`+current 一致で window-reset pre-check（`healthy_since+stable_window` 越し → `failures=0`/`last_error=None`）→ `failures>=max_attempts` で `GaveUp` else `Pending{next:Some(now)}`・後発死（`failed`/`lingering` 一致）は `Destroy`+lingering のみクリア・`GaveUp` 再 emit（`attempts`/`last_error` 不変）・unwatched は `Unsupervised`・Phase 2: `Live` window reset 掃引 + lingering cleanup retry（unwind→`Destroy` best-effort・`Err(InvalidHandle)` で clear・他 `Err` は残して次 poll）・Phase 3: ready（`next_attempt_at:None` or `now>=next`）を `(next_attempt_at, id)` 昇順（`None` 最小値）で `max_attempts_per_poll` cap まで attempt 発行・残り繰越（event なし）
5. **attempt/cleanup** — `Instantiate{plugin_id}` → `Initialize` → checkpoint あれば `RestoreState`（`Ok`→`Applied`・`Err(Unsupported)`→`Skipped` skip・他 `Err`→attempt 失敗）→ `config:Some` で `Activate` → `config:Some && start_processing` で `StartProcessing`（`config:None` では `Activate` が走らず `Initialized` 止まり — `start_processing` 単独では `StartProcessing` を発行しない）→ `Live{new, healthy_since:Some(now)}` + `Restarted{id, old:failed, new, bounds:endpoint_bounds, restored}`。失敗時 `failures+=1`・`last_error`・minted handle の `lifecycle_state` mirror を読み unwind（`Processing`→`StopProcessing`・`Activated`→`Deactivate`・→`Destroy`・`Failed`→`Destroy` 合法）・unwind `Err` 打ち切りで `lingering`・`failures>=max` で `GaveUp` else `next_attempt_at=Some(now+delay(failures))`+`AttemptFailed`
6. **backoff** — `delay(k)=min(base_delay·2^(k-1), max_delay)`（k=failures・指数 31 飽和・`Duration::checked_mul` が `None` → `max_delay` clamp・`Instant::checked_add` で deadline 進行 — `now` non-decreasing は caller 契約・検査しない）
7. **`lib.rs`** — `mod supervisor;` + `pub use supervisor::{Supervisor, SupervisedId, RestartPolicy, RestartRecipe, SupervisionEvent, SupervisionError, RestoreOutcome};` + crate doc に opt-in util としての一行（`poll` が唯一の drain 点 — sole-drain 契約への pointer）
8. **unit test（module 内 `#[cfg(test)]`）** — `issue-116-supervisor-core-tests` の oracle: policy 検証・`RestartPolicy::default()` 値・`RestartRecipe::new`・watch validation（live `InProcessHost` handle / stale / `Scan` 未登録 id / match-set 重複）・host binding `WrongHost`（`watch`/`set_recipe` `Err`・`poll` event・他 host 通知未消費）・`unwatch` 戻り値・`set_recipe` 全置換+検証・`set_checkpoint` bound（`len+17 == WIRE_MAX_PAYLOAD` → `Ok` / `+1` → `Err` 両側）・`clear_checkpoint`・`current_handle` 各分岐・`delay` 数学（指数飽和・clamp）・`find_watched` match-set（current/failed/lingering 区別）・L0 host `poll` が空 events・`assert_impl_all!(Supervisor: Send, Sync)`・`#[non_exhaustive]` compile witness（`_` arm 要求）。**L0 `Failed` adopt は fragile のため必須ではない** — adopt 検証は T3 の L1 実子経路（`kill_child` → `lifecycle_state → Some(Failed)`）で pin（plan Open Questions）

注意: `host as *const PluginHost as usize` は pointer→usize cast（`unsafe` ではない・deref しない — workspace lint `unsafe_code` 抵触なし）。依存追加なし（`static_assertions` は dev-dep として既存 — Cargo.toml 変更不要）。`Restarted` 時点で新 instance は endpoint 未 attach — `prepare_endpoint` は caller 責務（doc に明記）。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-116-supervisor.md](../plans/implement-issue-116-supervisor.md)（T2 — supervisor 本体）
- [../designs/issue-116-supervisor.md](../designs/issue-116-supervisor.md) — 公開 API sketch・状態機械遷移表・attempt シーケンス・失敗分類表・poll アルゴリズム・checkpoint 所有・`usize` 束縛（**正本 — naming・field・規則は本書確定形を写す**）
- [../adr/issue-116-supervisor-placement.md](../adr/issue-116-supervisor-placement.md) — util 層配置・公開 API のみ使用・core 無変更境界
- [../specs/issue-116-supervisor.md](../specs/issue-116-supervisor.md) R-S1–R-S9・AC1/AC5/AC8/AC9
- [issue-116-supervisor-core-tests.md](../test-specs/issue-116-supervisor-core-tests.md) — unit test oracle

## Dependencies

- 先行タスク: なし（plan のみ — T1 `l1-test-worker-fault-flags` とは独立・並行可。unit test 経路は fixture flag を使わない）
- 後続タスク: [l1-supervisor-e2e.md](l1-supervisor-e2e.md)（T3 — 本 task の公開 API を検証）・[l1-supervisor-final-verification.md](l1-supervisor-final-verification.md)（T4）

## Affected Files

- `crates/ruviform-plugin-host/src/supervisor.rs` — 新規（公開 API・entry state machine・poll Phase 0-3・attempt/cleanup・unit test module）
- `crates/ruviform-plugin-host/src/lib.rs` — `mod supervisor;` + `pub use` + crate doc 一行

## Done When

- [ ] 全公開型・全 API が design sketch 確定形（`#[non_exhaustive]` enum + struct variant・`usize` bound_host・`Copy` `SupervisionError`）で `lib.rs` から公開される
- [ ] `watch`（live/stale/未登録/dup/`Failed` adopt）・`unwatch`・`set_recipe`・`set_checkpoint` bound・`clear_checkpoint`・`current_handle`・`poll` Phase 0-3・attempt/replay/cleanup/lingering・`delay` 数学が遷移表どおり
- [ ] `composite.rs`/`l1/`/`host.rs`/`wire.rs` 無 diff・`supervisor.rs` が `crate::l1`/`pub(crate)` 内部型を参照しない（AC1/AC4 — review で確認）
- [ ] unit test が `issue-116-supervisor-core-tests` の検証行を網羅して pass — Phase-1 drain の通知駆動行（watched `InstanceFailed` → `Destroy`/`Pending`・後発死 match・`GaveUp` 再 emit・`Unsupervised` pass-through）は L0 host が `InstanceFailed` を emit しない（`composite.rs` L200-201 — queue は L1 `Failed` 遷移のみを運ぶ）ため `poll` 経由では unit 到達不能で、実子経路の到達は T3 がカバー。unit test は synthesized `Entry`/`PluginHandle::new`（同 crate 内なので `#[non_exhaustive]` `HostNotification`/`ChildExit` も構築可）で内部 drain ハンドラを直接駆動して drain ロジック行を網羅する

## Verification

- [ ] `cargo test -p ruviform-plugin-host --lib` — supervisor unit test が全 pass
- [ ] `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test`・`mise run test-rt` が全 pass
- [ ] `mise exec -- cargo doc -p ruviform-plugin-host --no-deps` が warning 0（新規公開 API doc — sole-drain WARNING 含む）
