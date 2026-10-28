---
id: "TSPEC-034NOX3x1mfWrROBAq4Wqu"
type: "test-spec"
status: "approved"
title: "L1 supervisor core tests（Issue #116 — watch/poll state machine・backoff・GaveUp・host binding・checkpoint bound）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/116"
    - "https://github.com/scarecrow173/ruviform/issues/31"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-116-supervisor-e2e-tests.md"
    - "docs/plans/implement-issue-116-supervisor.md"
    - "docs/tasks/l1-supervisor-core.md"
    - "docs/adr/issue-116-supervisor-placement.md"
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

# L1 supervisor core tests（Issue #116 — watch/poll state machine・backoff・GaveUp・host binding・checkpoint bound）

## Purpose

supervisor の **API 契約・entry state machine・policy/backoff 数学・host binding・checkpoint bound・drain 契約**の oracle — `PluginHost` 公開面のみで到達可能な挙動（純簿記・`InProcessHost`/L0 `register_factory` host・型レベル witness・`Instant` 注入による deterministic deadline）を pin する。実子プロセス経路の復旧・restore wire 分類・bundle burst・adopt の実観測は `issue-116-supervisor-e2e-tests` が所有する。本 spec は spec AC1/AC5/AC8/AC9 の機構面（+ AC6 の語彙）を主対象とし、T2（`l1-supervisor-core`）の検証正本となる。

## Feature

`Supervisor` は `PluginHost` を `&mut` で借りて caller の非 RT 制御ループから `poll(&mut host, now: Instant)` で駆動する opt-in util — `watch`/`unwatch`/`set_recipe`/`set_checkpoint`/`clear_checkpoint`/`current_handle`/`poll` の API 契約と、per-entry の `Live → Pending → (Live | GaveUp)` state machine・`RestartPolicy` の backoff 数学・「1 Supervisor = 1 PluginHost」の束縛・checkpoint wire bound を、`SupervisionEvent`/`SupervisionError`/`RestoreOutcome`（全て `#[non_exhaustive]`）の defined 語彙で振る舞う。実装先は `src/supervisor.rs`（`lib.rs` re-export）— `PluginHost` 公開 API（`execute`/`try_recv_notification`/`lifecycle_state`/`Scan`）のみを使い、`pub(crate)` L1 machinery・`composite` 内部型には一切触れない。

## Rules

- **policy 検証**: `Supervisor::new` は `max_attempts_per_poll == 0` を `Err(InertPolicy)` で拒否（clamp しない — 0 を既定値へ読み替えるのは caller の意図を変える）。`RestartPolicy::default()` = `max_attempts: 3`・`base_delay: 250ms`・`max_delay: 30s`・`stable_window: 60s`・`max_attempts_per_poll: 8`。`RestartRecipe::new(plugin_id)` = `config: None`・`start_processing: false`・`endpoint_bounds: None`
- **`watch` 検証**: `lifecycle_state(h)` が `Some(_)` で live（`None` → `Err(InvalidHandle)`）。`Scan` で `recipe.plugin_id` の登録を検証（未登録 → `Err(UnknownPluginId)` — 登録の証明のみで handle 所有 plugin との一致検証ではない・misdeclare は caller 責務）。current/failed/lingering のいずれかが一致する既存 entry → `Err(AlreadyWatched)`。**`lifecycle_state → Some(Failed)` は adopt** — `Destroy{h}`（`Err(InvalidHandle)` でも継続・キュー残の `InstanceFailed` は slot 解放で落ちる = 二重復旧しない）→ `Pending{next_attempt_at: None, failed: h}` で登録 → 次 poll で即 attempt（caller が通知を消費済みでも復旧できる）
- **host binding**: 初回の host 受取 call（`watch`/`set_recipe`/`poll`）で `host as *const PluginHost as usize` を束縛 — 以後の異 host は `watch`/`set_recipe` → `Err(WrongHost)`・`poll` → `SupervisionEvent::WrongHost` emit で drain・window reset・attempt を一切行わない（他 supervisor の通知を奪わない）。束縛は `usize` 保持（生 pointer は `Supervisor` の `Send`/`Sync` を壊す）— caller が host 値を move すると別 host と見なす安全側の拒否
- **`unwatch`**: `Live` → `Some(current)`、`Pending`/`GaveUp` で lingering 残存 → `Some(lingering)`、それ以外/未知 id → `None`。entry を除去し instance は殺さない（lingering を返さないと orphan する — caller が `Destroy`/salvage を判断）
- **`set_recipe`**: recipe 全置換・`plugin_id` を `Scan` 検証・未知 id `Err(NotWatched)`・異 host `Err(WrongHost)`
- **`set_checkpoint`**: `bytes.len() + WIRE_COMMAND_ENVELOPE(17B: seq 8 + instance 4 + verb 1 + len 4) > wire::WIRE_MAX_PAYLOAD` → `Err(CheckpointTooLarge)`・未知 id `Err(NotWatched)`。`id` キーなので remint 後も無条件に有効。`clear_checkpoint` → `bool`（存在したか・以後の復旧は `RestoreState` 未発行）
- **`current_handle`**: `Live` → `Some(current)`（lingering 残留時も current 優先 — lingering は内部 cleanup 対象）・`Pending`/`GaveUp`+lingering → `Some(lingering)`（caller が `Destroy`/salvage できるよう露出）・`Pending` なし・未知 id → `None`
- **drain 契約**: `poll` が唯一の `try_recv_notification` drain 点 — unwatched handle の通知は `Unsupervised{notification}` で pass-through（将来の `HostNotification` variant も `Unsupervised` へ）。caller が `poll` 外で drain するのは禁止 — supervised handle の `InstanceFailed` を先に消費すると entry は `Failed` instance を指したまま `Live` に滞留する silent loss（`poll` doc WARNING + 本 spec で pin）
- **state machine（drain）**: watched `InstanceFailed{h}` → `Destroy{h}`（`Err` 無視 — 外部 `Destroy` 競合を吸収）→ `Live`+current 一致なら **window-reset pre-check**（`healthy_since=Some(t)` かつ `now>=t+stable_window` → `failures=0`・`last_error=None`）→ `failures >= max_attempts` → `GaveUp` emit、else `Pending{next: Some(now), failed: h}`（初回 attempt は即時）。`Pending.failed`/任意 state の `lingering` 一致の後発死 → `Destroy` + **lingering 一致のみ `lingering=None`**（current/failed 一致で消すと残存 live handle が orphan）・`GaveUp` は `GaveUp` event 再 emit（`attempts`/`last_error` 不変 — `(id, attempts)` dedupe で初回 emit と区別可能）・`Pending`/`Live` は不変
- **backoff**: `delay(k) = min(base_delay · 2^(k-1), max_delay)`（k = `failures` — attempt 失敗後 `next_attempt_at = now + delay(failures)`。指数は 31 で飽和・`Duration::checked_mul` が `None` なら `max_delay` clamp・`Instant::checked_add` で進める）。`stable_window` reset は `Live`+`failures>0`+`healthy_since=Some(t)`+`now>=t+stable_window` で `failures=0`/`last_error=None` — **drain 時 pre-check と Phase 2 掃引の両方**で評価（`Pending`/`GaveUp` 不適用・`healthy_since=None`（watch 登録時）では不評価 — `failures` は `Restarted` を経ないと増えない）。`max_attempts=0` → 新規失敗で attempt 未発行・即 `GaveUp{attempts:0, last_error:None}`
- **attempt 発行**: ready（`next_attempt_at: None`（adopt 経路）または `now>=next_attempt_at`）を `(next_attempt_at, id)` 昇順（`None` は最小値・determinism）で `max_attempts_per_poll` cap まで発行 — 残りは次 poll へ繰越（繰越 `Pending` は attempt まで event を emit しない・`current_handle → None` で観測）。`now` の non-decreasing は caller 契約（supervisor は検査しない — deadline は実時刻ではなく次 `poll` 呼出しで解決）
- **型・語彙 witness**: `Supervisor` は sendable データのみ保持（`bound_host: usize`・`PluginHost` 非所有・内部 thread/timer/async なし）— `assert_impl_all!(Supervisor: Send, Sync)`。`PluginHost` は `!Send`/`!Sync` のまま（反対側 pin — 同一スレッドの非 RT ループ駆動）。`SupervisionEvent`/`SupervisionError`/`RestoreOutcome` は enum レベル `#[non_exhaustive]` + `SupervisionEvent` struct variant も variant レベル `#[non_exhaustive]`。`SupervisionError` は `Copy` 維持。`GaveUp` は `HostError::Failed` に畳まない別語彙（`Failed` は「子死亡で live インスタンスが terminal」语义を維持）

## Examples

- `Supervisor::new(RestartPolicy{max_attempts_per_poll: 0, ..})` → `Err(InertPolicy)` — clamp せず拒否
- `watch`: live handle + `Scan` 登録済 `plugin_id` → `Ok(SupervisedId)`・stale handle（`lifecycle_state → None`）→ `Err(InvalidHandle)`・未登録 id → `Err(UnknownPluginId)`・同一 handle の再 `watch` → `Err(AlreadyWatched)`（current/failed/lingering いずれの一致でも）
- host binding: hostA で `watch` → hostB で `watch`/`set_recipe` → `Err(WrongHost)`・`poll(hostB)` → `[WrongHost]`（hostB の通知は drain されず残る — 他 supervisor の drain 権を奪わない）
- checkpoint bound: `bytes.len() + 17 == WIRE_MAX_PAYLOAD` → `Ok`・`+ 18` → `Err(CheckpointTooLarge)`（boundary 両側を pin — oversized `RestoreState` frame は wire に乗る前に止まる）
- `delay` 表: `base=250ms, max=30s` で `failures=1 → 250ms`・`2 → 500ms`・`7 → 16s`・`8 → 32s → 30s clamp`・`31+ → 30s`（指数飽和）
- `unwatch`: `Live` → `Some(current)`・`GaveUp`+lingering → `Some(lingering)`・二度目/未知 id → `None`
- `Instant` 注入: `Pending{next: Some(now+250ms)}` で `poll(now)` → attempt なし・`poll(now+250ms)` → attempt 発行（wall-clock 非依存）
- L0 host（`register_factory`/`InProcessHost`）で live `watch` → `poll` → `[]`（通知源なし・非破壊 — L0 live `watch` は inert で合法）
- `#[non_exhaustive]` witness: `SupervisionEvent`/`SupervisionError`/`RestoreOutcome` の match が `_` arm を要求（compile-time pin）
- `Send`/`Sync` witness: `assert_impl_all!(Supervisor: Send, Sync)` が compile pass（`bound_host: usize` — 生 pointer 混入は `assert_not_impl_any` 相当で検出）

## Guarantees

- supervisor は `PluginHost` 公開面（`execute`/`try_recv_notification`/`lifecycle_state`/`Scan` via execute）のみで駆動 — `pub(crate)` L1 machinery（`l1::` モジュール・`Route`/`Slot`/pool 内部型）に一切触れない（design 契約 — テストも全て公開面経由）
- `poll` が唯一の drain 点 — unwatched 通知は必ず `Unsupervised` で誰かに届く（supervised/unsupervised の区別なく通知消失しない）
- `usize` bound_host で `Supervisor` の `Send`/`Sync` 適合を維持・`PluginHost` `!Send`/`!Sync` と同一スレッドで駆動・内部 thread/timer/async runtime なし（AC8）
- backoff deadline は `Instant` 注入で deterministic — wall-clock 非依存の unit test が可能
- checkpoint bound は `bytes.len() + envelope > WIRE_MAX_PAYLOAD` を wire に乗る前に `Err(CheckpointTooLarge)` で拒否 — oversized `RestoreState` frame が runner の `read_frame`/`decode` を落として child-scope `Failed`+fan-out を引く経路に到達しない
- `GaveUp` は `HostError::Failed` に畳まない別 event variant — `Failed` の「子死亡 terminal」语义は不変（AC6）

## Non-goals

- 実子プロセス経路の復旧挙動・`RestoreState` の wire 分類（`Applied`/`Skipped`/budget burn）・bundle burst・`Failed` adopt の L1 実観測・`prepare_endpoint`（`issue-116-supervisor-e2e-tests` が対象 — 本 spec は L1 経路でしか生起しない挙動を unit 化しない）
- `l1-test-worker` flag の内部実装（`docs/tasks/l1-test-worker-fault-flags.md` が所有）
- caller 直接 `HostCommand::RestoreState` / `SaveState` reply の wire bound gap（design Deferred — pre-existing core gap・follow-up 候補）
- route 単位 watch・`rebind`・pull 型 checkpoint・per-entry policy・`lifecycle_state` 走査 fallback・`GaveUp` 自動 unwatch・`BundleFailed` variant（design Deferred 表 — 初版では検証対象外）
- RT 契約 — supervisor の全操作は非 RT（`rt_` テスト対象なし・`process`/`PluginEndpoint` 语义不変）
- L0 `Failed` adopt の deterministic pin — L0 `Shared` slot poisoned `Failed` は fragile（adopt 検証は L1 経路に寄せる — plan Open Questions）

## Risk

- drain 時 window-reset pre-check の抜け → `stable_window` 越しの散発 crash が旧 episode `failures` を背負って早期 `GaveUp`（design 遷移表の drain 行 — give-up 判定前評価を pin）
- `find_watched` match-set（current/failed/lingering）の取り違え → lingering を `Destroy` で殺す・通知を誤帰属・`Pending.failed` 一致で lingering を消して残存 handle を orphan させる
- `poll` 外 drain の契約違反が silent unrecoverable loss（型で防げない）→ `poll` doc WARNING + review で pin
- `Instant` 注入なしの実時刻依存テスト → wall-clock flake（`now` は全て注入 — deadline は `poll` 呼出し粒度で解決する確定契約）
- `SupervisionEvent` struct variant の `#[non_exhaustive]` 抜け → 将来 field 追加が breaking change 化
