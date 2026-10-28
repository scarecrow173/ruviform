---
id: "TSPEC-034NOXcdTo4m1tfmRleM0V"
type: "test-spec"
status: "proposed"
title: "L1 supervisor E2E tests（Issue #116 — 実子 restart・bundle burst・crash-loop GaveUp・restore 分類・lingering）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/116"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/117"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
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
    - "docs/test-specs/issue-116-supervisor-core-tests.md"
    - "docs/test-specs/issue-117-bundle-e2e-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/plans/implement-issue-116-supervisor.md"
    - "docs/tasks/l1-test-worker-fault-flags.md"
    - "docs/tasks/l1-supervisor-e2e.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-116-supervisor.md"
    - "docs/adr/issue-116-supervisor-placement.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-116-supervisor.md"
    - "docs/designs/issue-116-supervisor.md"
    - "docs/adr/issue-116-supervisor-placement.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 supervisor E2E tests（Issue #116 — 実子 restart・bundle burst・crash-loop GaveUp・restore 分類・lingering）

## Purpose

`l1-test-worker` 実子プロセス + `PluginHost` 公開面のみで、supervisor の drain-`Destroy` → `Instantiate` → lifecycle replay → `RestoreState` → `Restarted` 機械的復旧・失敗分類・bundle burst・give up・AC4 無退行を pin する E2E oracle — spec R-V1–R-V6 + AC2/AC3/AC4/AC6/AC7 の実装検証が主対象（API/state machine の詳細は `issue-116-supervisor-core-tests` が所有）。テストは `tests/l1_supervisor_e2e.rs` として **`#[path]` harness（`l1_proxy_e2e.rs` の `mod l1`）に乗らない別 test target** で書く — `pub(crate)` L1 machinery が resolve 不能であること自体が AC1「`PluginHost` 公開 API のみ使用」の compile-level 証明になり、`l1_proxy_e2e.rs` の無 diff が R-V5 の zero-diff 証明になる。

## Feature

supervised handle の `InstanceFailed`（`#[doc(hidden)] kill_child` で注入）を `poll` が drain → `Destroy` → `Instantiate{plugin_id}`（同 route — pool が fresh 子 or 同居子を選択・supervisor は子数を制御しない）→ `Initialize` → checkpoint があれば `RestoreState{bytes}` → `config: Some` なら `Activate{config}` → `start_processing` なら `StartProcessing` → `SupervisionEvent::Restarted{id,old,new,bounds,restored}`。失敗は attempt budget を消費して exponential backoff・`max_attempts` で `GaveUp`。`l1-test-worker` の `--fail-restore`・`--state-unsupported`・`--fail-initialize`・`--fail-start-processing`・`--fail-stop-processing`・`--crash-after-msgs` で失敗段階を注入する。

## Rules

- **happy path（AC2/R-V6）**: Dedicated/Bundled 両 route で `kill_child` → `InstanceFailed` drain → `Restarted{id,old,new,bounds,restored}` — `new` は generation bump された別 mint handle（旧 handle は `Err(InvalidHandle)` stale）・`lifecycle_state(new)` が recipe 到達点（`config:None`→`Initialized`・`config:Some`→`Activated`・`+start_processing`→`Processing`）・`bounds` は recipe の `endpoint_bounds` echo・`prepare_endpoint(new, bounds)` が動く（`config:Some` で `Activated` 到達 recipe — `Restarted` 時点で endpoint 未 attach・caller 再配線責務）
- **restore 分類（AC4/R-V3）**: pre-failure `SaveState` blob を `set_checkpoint` → 復旧で `RestoreState{bytes}`（worker が `state` へ反映 — 次の `SaveState` で round-trip 観測）→ `restored=Applied`・checkpoint 未登録 → `RestoreState` 未発行 `restored=NotRequested`・`--state-unsupported` → `Err(Unsupported)` は **skip（attempt 消費なし）** で `restored=Skipped` の `Restarted` 到達・`--fail-restore`（`Err(PluginFailed)`）→ **attempt 失敗で budget 消費** → `max_attempts` で `GaveUp`（保守的分類 — supervisor は caller の checkpoint を黙って捨てない）
- **backoff/episode（AC5/R-V4）**: `Instant` 注入で `next_attempt_at` 未達は attempt 発行なし・到達で attempt・attempt 失敗後 `delay(k)` 経過で再試行・`stable_window` 越し crash は **drain 時 pre-check で `failures` reset**（旧 failures で即 `GaveUp` しない回帰 pin — `healthy_since + stable_window` 越しの `InstanceFailed` が `Pending` へ進む）・`max_attempts=0` → `GaveUp{attempts:0,last_error:None}`・crash-loop（`--crash-after-msgs` で attempt 毎に子死亡）→ bounded `GaveUp{attempts:max, last_error:Some}`・`max_attempts_per_poll=0` → `Supervisor::new` `Err(InertPolicy)`
- **bundle burst（AC3/R-V1）**: Bundled 上 N supervised + 1 unwatched を `kill_child` → 同一 `ChildExit` の N `InstanceFailed` を全 drain → 各 entry 独立 `Pending` → `max_attempts_per_poll` cap まで attempt 発行・残りは次 poll 繰越（繰越 `Pending` は event なし — `current_handle → None`/`Restarted` 列で観測）→ 全 supervised が新 handle で復旧（N 子 spawn を強制しない — pool が fresh 子に収束）・unwatched は `Unsupervised{notification}` pass-through
- **attempt 中失敗（AC7/R-V2）**: mid-`CreateInstance` 死（`--crash-after-msgs=2` — Scan=1, CreateInstance=2 で死亡）→ `Err(Ipc)`・通知なし・handle 未 mint（cleanup 不要・同居 sibling fan-out は別 entry）・`--fail-initialize` → `Err(PluginFailed)`（minted `Created` を cleanup `Destroy`）・`Activate` `Err(Prepare)`（`max_frames > WIRE_MAX_FRAMES` config — 既存 worker 検証で注入・flag 不要）→ cleanup・`--fail-start-processing` → `Err(PluginFailed)`（`Activated` 残留 → unwind `Deactivate`→`Destroy`）・mid-replay 子死亡（attempt 中の子死）→ `Err(Ipc)`/`Err(Failed)`（mirror `Failed` → `Destroy` 合法・unwind 不要）
- **lingering（AC7）**: `--fail-stop-processing` で cleanup unwind の `StopProcessing` `Err`（commit-then-advisory — mirror は `Activated` に残る）→ live handle が `lingering` → `current_handle` で露出・次 poll Phase 2 retry で `Deactivate`→`Destroy` 成功して clear（外部 `Destroy` 済みなら `Err(InvalidHandle)` で clear）— リークしない・orphan させない
- **`Failed` adopt（AC9）**: `kill_child` 後に通知を drain せず `watch`（`lifecycle_state → Some(Failed)`）→ `Destroy{h}` で slot 解放（キュー残 `InstanceFailed` は落ちる = 二重復旧しない）→ `Pending{next:None}` → 次 poll `Restarted`。caller が通知を消費済みの系でも同じ復旧を確認
- **`GaveUp` 语义（AC6）**: mint 済み instance があれば unwind+`Destroy` 済み・entry は `unwatch` まで保持（自動除去なし）・`GaveUp` は `HostError::Failed` に畳まない別 event variant・`last_error` は episode 直近の attempt 失敗・lingering 後発死の再 emit は `attempts`/`last_error` 不変（`(id,attempts)` dedupe で初回 emit と区別可能）
- **drain 契約・opt-in（AC1/R-S4）**: `poll` が唯一の drain 点 — unwatched 通知は `Unsupervised` pass-through・supervisor を使わない素 `PluginHost` で通知 drain しても `Restarted` 相当は出ない（opt-in — core は自律再起動しない・caller が手動復旧する責務は維持）
- **無退行（R-V5/AC4 証明）**: supervisor 未使用の `l1_proxy_e2e` suite 全テストが変わらず pass・`l1_proxy_e2e.rs`/`composite.rs`/`l1/`/`host.rs`/`wire.rs` は無 diff
- **型 witness（AC8）**: `poll(&mut host, now: Instant)` のみで駆動・内部 thread/timer/async なし・`assert_impl_all!(Supervisor: Send, Sync)`（`bound_host: usize`）・`PluginHost` `!Send`/`!Sync` 維持

## Examples

- happy: `watch(h, recipe{plugin_id, config:Some(cfg), start_processing:true, endpoint_bounds:Some(b)})` → `kill_child` → `poll` → `Restarted{old:h, new:h2, bounds:Some(b), restored:NotRequested}`・`lifecycle_state(h2)=Processing`・`execute(h)` → `Err(InvalidHandle)`・`prepare_endpoint(h2, b)` `Ok`
- checkpoint: `SaveState{h}` → `set_checkpoint(id, bytes)` → kill → `Restarted.restored=Applied`・`SaveState{h2}` が checkpoint blob を返す（worker state round-trip — `RestoreState` が実際に新 instance へ届いた証明）
- skip vs burn: `--state-unsupported` worker → 1 attempt で `Restarted{restored:Skipped}`（budget 消費なし・`failures` 不増）vs `--fail-restore` → `AttemptFailed`×`max_attempts` → `GaveUp{attempts:3, last_error:Some(PluginFailed)}`
- burst: `max_attempts_per_poll: 2` で 4 supervised kill → 1 poll 目 `Restarted`×2・残 2 繰越（event なし — `current_handle → None` で観測）→ 次 poll で `Restarted`×2・unwatched は `Unsupervised`
- adopt: `kill_child` → caller が `try_recv_notification` で通知消費 → `watch(failed_h, recipe)`（adopt — `lifecycle_state→Some(Failed)`）→ `poll` → `Restarted`（通知なしでも復旧 — caller 消費済み系と同値）
- mid-create: `--crash-after-msgs=2`（handshake Scan=1・CreateInstance=2 で死亡）→ attempt が `Err(Ipc)` → `AttemptFailed`・handle 未 mint（cleanup 対象なし）
- lingering: `--fail-stop-processing` で attempt 失敗の unwind が `StopProcessing` で `Err` → `current_handle → Some(lingering)`・次 poll Phase 2 retry で `Deactivate`→`Destroy` 成功 → `current_handle → None`
- `GaveUp` 再 emit: lingering 残存の `GaveUp` entry が後発死通知を受ける → `GaveUp{attempts,last_error}` 不変で再 emit（`(id,attempts)` dedupe で初回と区別）
- opt-in: supervisor を介さない host で `kill_child` → `InstanceFailed`（従来契約 — caller が `Destroy`+手動復旧）・`Restarted` 相当は出ない

## Guarantees

- 全検証は `PluginHost` 公開面 + `#[doc(hidden)] kill_child` + `l1-test-worker` argv flags のみ — `pub(crate)` L1 machinery（`l1::`/`Route`/`Slot`/pool 内部型）を import しない（別 test target で compile-level 強制）
- 各 `Restarted` は新 mint handle を運び旧 handle は generation bump で `Err(InvalidHandle)` stale — `Restarted{old,new}` が caller の handle 写像・endpoint・param 簿記 rebind のトリガ
- bundle crash の復旧数は supervised entry 数 — `max_attempts_per_poll` が 1 poll の spawn/reply-wait 集中を bound・残りは `next_attempt_at` elapsed のまま次 poll 繰越（子数は pool 判定 — `Bundled` の fresh 子に収束）
- `GaveUp` 時点で minted instance は unwind+`Destroy` 済み、または unwind `Err` の live handle が `lingering` として `current_handle` に露出 — リークも orphan もしない
- `l1_proxy_e2e` は無 diff のまま green — AC4「core は自律再起動しない」の非退行証明（supervisor は additive opt-in 面）

## Non-goals

- API 契約・state machine 遷移表・backoff 数学・host binding・checkpoint bound・`unwatch`/`set_recipe`/`current_handle` の詳細 pin（`issue-116-supervisor-core-tests` が対象 — 本 spec は実子経路で観測される結果のみ）
- `l1-test-worker` flag の内部実装・argv 定義（`docs/tasks/l1-test-worker-fault-flags.md` が所有 — 本 spec は注入効果の oracle のみ）
- caller 直接 `RestoreState`/`SaveState` reply の wire bound gap・`BundleFailed` variant・route 単位 watch・pull checkpoint・per-entry policy・`lifecycle_state` 走査 fallback（design Deferred — 検証対象外）
- `EndpointBounds` 値の语义（`prepare_endpoint` が受理・`Restarted.bounds` echo が正しいことの検証のみ — bounds semantic 自体は endpoint contract 側の責務）
- GUI・RT 経路（supervisor は扱わない・復旧中の `process` は pass-through/無音のまま）
- L0 `Failed` adopt の deterministic pin（L0 `Shared` slot poisoned `Failed` は fragile — adopt は L1 実子経路で pin・core tests Non-goals と対）

## Risk

- worker flag の注入段階誤り（`--fail-initialize` が commit 後 `Err` を返す等 → cleanup 経路の誤検証）— flag は不 commit `Err` が既定形状（advisory commit-then-`Err` は `--fail-stop-processing` のみ）
- message counter が handshake を含む（Scan=1・CreateInstance=2）ため `--crash-after-msgs` の N 値を誤る → 注入段階のずれ（mid-create vs Initialize 中死の取り違え）
- pool が fresh 子へ収束するまでの spawn/reply-wait が実時刻依存 → backoff/episode の deadline は `Instant` 注入で進め、child 側の待ちは `NOTIFY_TIMEOUT` 流の bounded wait で bound
- cap 繰越 `Pending` が event なしで不可視 → 繰越を `current_handle`/`Restarted` 列数で pin（「繰越が起きた」ことを観測不能のまま終わらせない）
- `l1_proxy_e2e` 負荷依存 flake の混入 — 別 target で切り分けが自然（孤立 rerun で判断）
