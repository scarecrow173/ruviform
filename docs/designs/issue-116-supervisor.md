---
id: "DESIGN-034NNxNMrrk6OoIGfX18rk"
type: "design"
status: "approved"
title: "L1 supervisor（自動再起動 util 層）設計（Issue #116）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/116"
    - "https://github.com/scarecrow173/ruviform/issues/31"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/117"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-116-supervisor.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-117-bundle-child-process.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-116-supervisor-placement.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/designs/engine-plugin-host-integration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-116-supervisor.md"
    - "docs/discovery/issue-116-supervisor.md"
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/specs/issue-117-bundle-child-process.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
    - "docs/adr/issue-116-supervisor-placement.md"
  derived-by:
    - "docs/plans/implement-issue-116-supervisor.md"
    - "docs/tasks/l1-test-worker-fault-flags.md"
    - "docs/tasks/l1-supervisor-core.md"
    - "docs/tasks/l1-supervisor-e2e.md"
    - "docs/tasks/l1-supervisor-final-verification.md"
    - "docs/test-specs/issue-116-supervisor-core-tests.md"
    - "docs/test-specs/issue-116-supervisor-e2e-tests.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-116-supervisor-core-tests.md"
    - "docs/test-specs/issue-116-supervisor-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/115"
    - "https://github.com/scarecrow173/ruviform/issues/119"
    - "https://github.com/scarecrow173/ruviform/issues/120"
  defers: []
  deferred-by:
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
---

# L1 supervisor（自動再起動 util 層）設計（Issue #116）

## Context

Issue #116 — Issue [#31](https://github.com/scarecrow173/ruviform/issues/31) の確定判断「クラッシュ時は `Failed` 遷移 + 通知・**再起動は呼出し側判断（自動再起動はしない — 状態復元はプラグイン責務）**」（`docs/designs/plugin-isolation.md` L66・`docs/adr/plugin-isolation-strategy.md`）を維持したまま、L1 `HostNotification::InstanceFailed` 後の機械的復旧を担う **supervisor 的運用面**を確定する。`docs/designs/plugin-l1-out-of-process.md` L301 deferred 行（「自動再起動・supervisor ポリシー」）の re-engagement であり、spec `docs/specs/issue-116-supervisor.md` の OPEN・推奨論点と deferred 表全項目を本書で確定する。

上流確定事項（再決定しない — discovery `docs/discovery/issue-116-supervisor.md` の観測表に一致）:

- **AC4（「自動再起動なし」契約）**: core `PluginHost` は自律的に再起動しない — `l1_proxy_e2e.rs` `kill_transitions_failed_and_notifies` で pin 済み。supervisor は opt-in でなければならない。
- **`Failed` は terminal**: `Failed` インスタンスは `Destroy` 以外の全コマンドを `Err(HostError::Failed)` で拒否し、`Failed → Destroyed` のみ進行可能（`composite.rs` の mirror legality — `Destroy` は `Created`/`Initialized`/`Failed` からのみ合法、`Activated`/`Processing` は `Err(Lifecycle)`）。
- **通知契約**: `HostNotification::InstanceFailed{handle, exit}` — per-instance・live 中 exactly once・`Destroy` で slot 解放されるとキュー残通知は slot と共に落ちる（`composite.rs` L200-204・L859-897）。`handle` と `exit` のみを運び `plugin_id` は持たない。`PluginHost` に handle→plugin_id 逆引き面は存在しない。
- **bundle fan-out（#117 implemented）**: 子死亡は子スコープ terminal で bundle 内全 live インスタンスを `Failed` 収束させ、各 minted インスタンスへ同一 `ChildExit` を持つ `InstanceFailed` を fan-out。per-instance 収束（子生存の帰属違反）は `ChildExit{None,None}` の単発通知（ADR `docs/adr/issue-117-bundle-child-failed-granularity.md` が粒度正本）。
- **`PluginHost` は `!Send`/`!Sync`**: `composite.rs` L304-308 — supervisor スレッドを spawn して host を渡す設計は型レベルで不可能。単一スレッドの非 RT 制御ループに乗る形しか取れない。
- **RT 契約**: supervisor 関連の全操作（通知 drain・`Destroy`・`Instantiate`・lifecycle replay・`SaveState`/`RestoreState`）は非 RT。RT 経路（`PluginHost::process`/`PluginEndpoint`）に supervisor 語彙は入らない — `Failed` 時の RT 側は pass-through/無音のまま。
- **死後の state 取得は不可能**: `Failed` 以後 `SaveState` は `Err(Failed)`（`l1` execute 入口の Failed gate）— checkpoint は故障前に取るしかない。`RestoreState` は lifecycle gate なし（runner dispatch — `Created`/`Initialized`/`Activated`/`Processing` いずれでも適用可能）。
- **`Instantiate` 自体の失敗は通知を出さない**: spawn/handshake/`CreateInstance` 中途死亡は `Err(Ipc)` で同期返し（公開 handle 未 mint — `InstanceFailed` は出ない）。`Creating` 中の死亡も同様。
- **handle は毎回新規 mint**: `mint`（`composite.rs` L1065-1078）が slot 再利用時に generation++ — 再起動インスタンスは別 `PluginHandle`。stale handle は `Err(InvalidHandle)` で安全に拒否される。
- **pool は terminal 子を自然 evict**: `instantiate_bundled` の scan が `pool_eligible` 外の terminal 子を除去し fresh spawn に進む（`composite.rs` L1081-1139）— supervisor が明示的に pool を掃除する面は不要。
- **`mid-`DestroyInstance` 死亡でも `Destroy` は `Ok`**: slot は解放・そのインスタンスの通知は slot と共に落ちる。
- **`try_recv_notification` は `drain_endpoint_channels` を兼ねる**（`composite.rs` L877-881）— supervisor の poll 中に detach 済み endpoint の leg 返還も処理される。

## 決定サマリ（spec OPEN・deferred → 本書の結論）

| # | 論点 | 決定 |
| --- | --- | --- |
| 1 | supervisor 面の置き場所 | **`ruviform-plugin-host` 内 `supervisor` モジュール**（`src/supervisor.rs`・`pub use` 再エクスポート）— `PluginHost` の公開 API のみ使用・core 型・フィールド・意味は一切無変更（AC4 維持）。配置確定の ADR は `docs/adr/issue-116-supervisor-placement.md` |
| 2 | 監視キー（`supervised_key` 型） | **`SupervisedId`**（u64・supervisor mint・opaque）— handle remint に不変な論理キー。spec 推奨の `PluginHandle` は「restart 毎にキーが変わる」ため採らない（checkpoint/set_recipe/unwatch の引数が安定しない・`old→new` alias を supervisor が内部保持する必要が出る） |
| 3 | handle→plugin_id 逆引き | **util 側帳簿** — `watch` 時に caller が `RestartRecipe.plugin_id` で宣言。core に逆引き面を追加しない（`PluginHost` 無変更・AC4 境界を汚さない） |
| 4 | backoff 形状 | **`RestartPolicy{max_attempts, base_delay, max_delay, stable_window, max_attempts_per_poll}`** — 失敗通知受信で即時 attempt・attempt 失敗後に `min(base_delay · 2^(attempts-1), max_delay)` 遅延。attempts は episode カウンタ（restart 成功で即 reset せず、`stable_window` 生存で reset — crash-loop bounded・散発 crash は無制限回復）。**reset は `InstanceFailed` drain 時にも評価する** — `now >= healthy_since + stable_window` を満たす crash は reset 後の `failures` で give-up 判定する（`Live` 限定の poll 掃引だけでは window 越しの散発 crash が古い episode の `failures` を背負って永久蓄積する） |
| 5 | `RestoreState` `Err(Unsupported)` 分類 | **skip（attempt 消費しない）** — state 非対応 plugin でも復旧が動く。`Err(PluginFailed)`/`Err(Ipc)`/`Err(Failed)`/その他 `Err` は attempt 失敗。skip は `Restarted.restored`（`RestoreOutcome::Skipped`）で caller から観測可能 — `Err(PluginFailed)` 系を attempt 失敗とするのは保守的決定（caller の checkpoint を黙って捨てない — `AttemptFailed`/`GaveUp` の `error`/`last_error` を見た caller が `clear_checkpoint` で degrade できる） |
| 6 | attempt 内失敗の後片付け | minted handle が live なら lifecycle 逆順 unwind（`Processing`→`StopProcessing`→`Activated`→`Deactivate`→`Initialized`）→ `Destroy`。unwind 自体が `Err` なら live handle を `lingering` として entry に保持・`current_handle` で露出（リークしない・放置もしない） |
| 7 | bundle burst | 1 poll = 通知全 drain + entry 毎独立回復。`max_attempts_per_poll`（既定 8）で attempt 発行を cap — 残りは次 poll へ繰越し。子プロセス数は pool 判定に委ねる（supervisor は「N 通知 = N 子」と解釈しない） |
| 8 | 通知 drain 責任 | **`poll` が唯一の drain 点** — 未監視 handle の通知は `SupervisionEvent::Unsupervised` で pass-through（caller の手動復旧責務は維持）。**caller が `poll` 外で `try_recv_notification` を drain するのは禁止** — supervised handle の `InstanceFailed` を先に消費すると entry は `Failed` instance を指したまま `Live` に滞留し回復不能になる（deferred の `lifecycle_state` 走査 fallback がまさにこの検出手段） |
| 9 | `GaveUp` 後の扱い | entry は terminal 保持・**自動 unwatch しない**（caller が `unwatch` or re-watch を判断）。`GaveUp` は `HostError::Failed` に畳まない別語彙 — `Failed` は「子死亡で live インスタンスが terminal」意味を維持。`GaveUp` event は `attempts` + `last_error: Option<HostError>` を payload に持つ（`max_attempts = 0` 等 attempt 未発行の give up は `None`） |
| 10 | `watch` 検証 | `watch(host, handle, recipe)` — `lifecycle_state` で live 検証（`Err(InvalidHandle)`）・`Scan` で plugin_id 登録検証（`Err(UnknownPluginId)` — 登録の証明のみで handle 所有権の検証ではない。misdeclare は cross-plugin resurrection になる caller 責務領域）・current/failed/lingering いずれかの handle が一致する重複 watch は `Err(AlreadyWatched)`。**`Failed` handle は adopt** — `Destroy{handle}` で slot を解放（キュー残の `InstanceFailed` は slot と共に落ちる = 二重復旧しない）して `Pending` で登録し、次 poll の attempt で復旧する（caller が通知を消費済みでも復旧できる） |
| 11 | `watch` 後の handle 変化 | caller が supervisor 経路外で `Destroy`→`Instantiate` した場合は `unwatch`→`watch` で追従する契約（`rebind` API は YAGNI） |
| 12 | mid-`DestroyInstance` 死の通知消失 | **検出しない** — `poll` が `lifecycle_state` を走査する fallback は YAGNI（頻度が低く、caller 側 `unwatch` で対処可能） |
| 13 | route 単位 watch | **deferred** — per-instance `watch` が最小構成（同一 plugin_id 全インスタンスを supervise する route 宣言は追加機能として残す） |
| 14 | give up 語彙 | `SupervisionEvent::GaveUp` — circuit breaker 的 `Err(Failed)` fold は却下（spec matrix と同じ結論） |
| 15 | endpoint 再配線 | `RestartRecipe.endpoint_bounds` を recipe に保持し `Restarted.bounds` field へ echo — **`prepare_endpoint` の呼出しは caller 責務**（`PluginEndpoint` は RT owner スレッドへの move が前提・supervisor が届ける経路がない構造的必然）。`endpoint_bounds` は recipe が `Activated` に到達する場合（`config: Some`）にのみ意味を持つ — `config: None` では `prepare_endpoint` が `Err(Lifecycle)` で拒否するため bounds echo は dead data |
| 16 | 1 `Supervisor` = 1 `PluginHost` の強制 | 初回の host 受取 call（`watch`/`set_recipe`/`poll`）で `host as *const PluginHost as usize` を束縛 — `PluginHandle{slot,generation}` は host 跨ぎで衝突し得るため束縛なしでは通知の誤帰属・誤 host への `Destroy`/`Instantiate` が起きる。以後の異 host call は defined 拒否（`watch`/`set_recipe` → `Err(SupervisionError::WrongHost)`、`poll` → `SupervisionEvent::WrongHost` emit で drain/attempt を行わない）。束縛は `usize` 保持 — 生 pointer は `!Send`/`!Sync` で `Supervisor` の型制約を壊す |

## Architecture

### 配置と境界（ADR 一体）

- `crates/ruviform-plugin-host/src/supervisor.rs` — `mod supervisor;` を lib.rs に追加し、`Supervisor`/`SupervisedId`/`RestartPolicy`/`RestartRecipe`/`SupervisionEvent`/`SupervisionError`/`RestoreOutcome` を `pub use` で再エクスポートする。
- **公開 API のみ使用**: `execute`・`try_recv_notification`・`lifecycle_state`・`Scan`（`execute` 経由）に限定して `PluginHost` を駆動する。`pub(crate)` L1 machinery（`l1::` モジュール・`composite` の `Route`/`Slot`/pool 内部型）には一切触れない。**これは型システムではなく設計契約として強制する** — 同 crate 内のためコンパイラは private 面を遮らないが、supervisor の crate テストを全て公開 API 経由で書くことで実証的に担保する（`l1-test-worker` + `#[path]` harness をそのまま利用可能）。
- **core 無変更**: `execute`/`try_recv_notification`/`instantiate`/`destroy`/`mint`/`slot_mut`/`routes`/`instances`/`generations`/`pool` の型・意味・フィールドは一切変更しない。`PluginHost` に handle→plugin_id 逆引き面・recipe 簿記・restart 動詞を追加しない — AC4「自動再起動なし」の境界を型・意味レベルで維持する。
- **`Supervisor` は `Send`/`Sync` 可能なデータのみ保持**（`PluginHost` を所有しない — `poll` の引数で `&mut` 借用）。poll は host を所有するスレッド＝caller の非 RT 制御ループで呼ぶ。内部スレッド・タイマー・async runtime は持たない。host 束縛は `usize`（`host as *const PluginHost as usize`）で保持する — 生 pointer は `!Send`/`!Sync` なので `Supervisor` の型制約を壊す。
- **依存追加なし** — `std::time::{Instant, Duration}`・`BTreeMap`・既存公開型（`PluginHost`/`PluginHandle`/`HostCommand`/`HostReply`/`HostError`/`HostNotification`/`EndpointBounds`・`ruviform_plugin_core::ProcessConfig`/`ruviform_plugin_core::wire` の `WIRE_MAX_*` const）のみ。
- **1 `Supervisor` = 1 `PluginHost` の drain 点**: 複数 `Supervisor` で同一 host を poll すると先に drain した側が全通知を消費する（後述の単一 drain 点契約）— 複数 host は `Supervisor` 毎に 1 対 1 で対応させる。`PluginHandle{slot,generation}` は host 跨ぎで衝突し得る（別 host の通知を自 entry に誤帰属し、誤った host へ `Destroy`/`Instantiate` を発行する）ため、**host 束縛を型ではなく defined 拒否で強制する** — `Supervisor` は初回の host 受取 call（`watch`/`set_recipe`/`poll`）で `host as *const PluginHost as usize` を記録し、以後の異なる host に対して `watch`/`set_recipe` は `Err(SupervisionError::WrongHost)`、`poll` は `SupervisionEvent::WrongHost` を emit して drain/attempt を一切行わない（他 supervisor の drain 権・通知を奪わない）。束縛は address identity — caller が host 値を move すると別 host と見なされ `WrongHost` を返す（誤帰属より厳しすぎる側に倒す安全側の拒否）。**caller が `poll` 外で `try_recv_notification` を drain するのは禁止** — supervised handle の `InstanceFailed` を先に消費すると entry は `Failed` instance を指したまま `Live` に滞留し、二度と復旧も `GaveUp` もしない（silent unrecoverable loss — deferred の `lifecycle_state` 走査 fallback はこの検出手段）。

### 公開 API 面（スケッチ — 命名・型は実装 Issue で最終確定）

```rust
/// `InstanceFailed` 後の機械的復旧を担う opt-in util —
/// `PluginHost` を `&mut` で借りて caller の非 RT ループで poll 駆動する。
/// `bound_host` は初回 host 受取 call で束縛される `usize`（`*const PluginHost` の
/// address identity）— 以後の異 host call は defined 拒否。
pub struct Supervisor { /* entries: BTreeMap<SupervisedId, Entry>, next_id, policy,
                           bound_host: Option<usize> */ }

/// 監視対象論理インスタンスの opaque キー — handle remint に不変。
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SupervisedId(u64);

#[derive(Clone, Debug)]
pub struct RestartPolicy {
    /// episode 内の復旧試行上限 — 0 は「失敗を観測したら即 give up」。
    pub max_attempts: u32,           // 既定 3
    /// attempt 失敗後の初回遅延 — delay_k = min(base·2^(k-1), max_delay)。
    pub base_delay: Duration,        // 既定 250ms
    /// backoff 上限。
    pub max_delay: Duration,         // 既定 30s
    /// restart 後この時間生存したら `failures`（+ `last_error`）を 0 に reset する
    /// episode 境界 — `InstanceFailed` drain 時の give-up 判定前と poll 掃引で評価。
    pub stable_window: Duration,     // 既定 60s
    /// 1 poll あたりの attempt 発行上限（bundle burst の spawn 集中を bound）。
    /// 0 は inert（attempt が永久に発行されない）— `Supervisor::new` が
    /// `Err(InertPolicy)` で拒否する。
    pub max_attempts_per_poll: u32,  // 既定 8
}

#[derive(Clone, Debug)]
pub struct RestartRecipe {
    /// 同 route 再 instantiate 対象 — `Instantiate{plugin_id}` に渡す登録時 claim。
    /// `watch`/`set_recipe` の `Scan` 検証は「id が登録済み」の証明のみ —
    /// handle の所有 plugin と一致しているかは検証不可能な caller 責務
    /// （misdeclare は別 plugin の復活 = cross-plugin resurrection）。
    pub plugin_id: String,
    /// `Some` なら replay で `Activate{config}`。`None` は `Initialized` で止める。
    pub config: Option<ProcessConfig>,
    /// `Activate{config}` 成功後に `StartProcessing` まで進めるか — 実効 guard は
    /// `config.is_some() && start_processing`（`config: None` では `Activate`
    /// 自体が走らず `Initialized` 止まりのため無視 — `Initialized` への
    /// `StartProcessing` は `Err(Lifecycle)` の defined rejection）。
    pub start_processing: bool,
    /// `Restarted.bounds` へ echo する呼出側再 attach 用 bounds —
    /// supervisor は `prepare_endpoint` を呼ばない（caller 責務）。
    /// `prepare_endpoint` は `Activated` 以上を要求するため `config: None`
    /// （`Initialized` 止まり）では dead data — `config: Some` と対で意味を持つ。
    pub endpoint_bounds: Option<EndpointBounds>,
}

/// `Restarted.restored` — checkpoint の適用結果（attempt 成功時のみ emit）。
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RestoreOutcome {
    /// checkpoint 未登録 — `RestoreState` は発行していない。
    NotRequested,
    /// `RestoreState` が `Ok` — checkpoint を適用済み。
    Applied,
    /// `RestoreState` が `Err(Unsupported)` — state 非対応として skip
    /// （決定 5 — skip が起きたことを caller が観測できる）。
    Skipped,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum SupervisionEvent {
    /// 監視対象が復旧した — caller は handle 写像・endpoint・param 簿記を
    /// `new` へ付け替える（`old` は stale — `Err(InvalidHandle)` になる）。
    /// `bounds` は recipe の `endpoint_bounds` echo（caller の `prepare_endpoint`
    /// 入力）・`restored` は checkpoint 適用結果。
    #[non_exhaustive]
    Restarted {
        id: SupervisedId,
        old: PluginHandle,
        new: PluginHandle,
        bounds: Option<EndpointBounds>,
        restored: RestoreOutcome,
    },
    /// 1 回の復旧 attempt が失敗した（backoff またはこの poll で `GaveUp` が続く）。
    #[non_exhaustive]
    AttemptFailed { id: SupervisedId, attempt: u32, error: HostError },
    /// episode の attempt 上限に達した — mint 済みインスタンスが残っていれば
    /// unwind + `Destroy` 済み（`current_handle` は残存 handle のみ返す）。
    /// `last_error` は episode 直近の attempt 失敗（`max_attempts = 0` 等
    /// attempt 未発行の give up は `None`）。lingering 後発死による再 emit は
    /// `attempts`/`last_error` が初回 emit と同一 — 初回 give-up と「lingering が
    /// 死んだ報告」の区別が要る consumer は `(id, attempts)` で dedupe する。
    #[non_exhaustive]
    GaveUp { id: SupervisedId, attempts: u32, last_error: Option<HostError> },
    /// live minted handle が cleanup 追跡から外れた — lingering 単一 slot が
    /// 既存 residue で占有され inline retry も失敗した二重 unwind 失敗角のみ。
    /// instance は殺さず caller 簿記へ委ねる（`Destroy`/salvage は caller 判断・
    /// 後発死は `Unsupervised` で届く）— 追跡不能を event で観測可能にする。
    #[non_exhaustive]
    Orphaned { id: SupervisedId, handle: PluginHandle },
    /// `watch` されていない handle の通知 — pass-through（caller が自分で捌く）。
    /// watched 失敗の `exit` は各 event に echo しない（`ChildExit` で復可否を
    /// 分岐しない決定の延長 — 生の通知は `Unsupervised` のみが保持する非対称）。
    #[non_exhaustive]
    Unsupervised { notification: HostNotification },
    /// `poll` が束縛済みと異なる `PluginHost` で呼ばれた — drain も attempt も
    /// 行わない（他 supervisor の drain 権・通知を奪わない defined 拒否。
    /// spec が `poll -> Vec<SupervisionEvent>` を pin するため rejection は
    /// `Result` ではなく event 経路）。
    WrongHost,
}

#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SupervisionError {
    /// `lifecycle_state` が `None` — stale/未知 handle。
    InvalidHandle,
    /// current/failed/lingering のいずれかの handle が一致する entry が既に存在する。
    AlreadyWatched,
    /// `recipe.plugin_id` が `Scan` の登録 descriptor に無い。
    UnknownPluginId,
    /// 未知の `SupervisedId`。
    NotWatched,
    /// 束縛済みと異なる `PluginHost` — 1 Supervisor = 1 PluginHost。
    WrongHost,
    /// `set_checkpoint` の blob が `RestoreState` 1 frame の wire 上限を超える
    /// （`bytes.len() + WIRE_COMMAND_ENVELOPE > wire::WIRE_MAX_PAYLOAD`）。
    /// oversized blob は runner の `read_frame`/`decode` を落として child-scope
    /// `Failed`（同居全インスタンス converge + fan-out）を引くため、wire に
    /// 乗る前に拒否する。
    CheckpointTooLarge,
    /// `max_attempts_per_poll == 0` の inert policy — attempt が永久に発行
    /// されない supervisor は意味を持たない（clamp せず拒否 — 0 を既定値に
    /// 読み替えるのは caller の意図を変える）。
    InertPolicy,
}
```

```rust
impl Supervisor {
    /// policy 検証 — `max_attempts_per_poll == 0` は `Err(InertPolicy)`
    /// （attempt が永久に発行されない inert supervisor を作らせない）。
    pub fn new(policy: RestartPolicy) -> Result<Self, SupervisionError>;
    /// 監視登録 — live 検証（`lifecycle_state` が `None` → `InvalidHandle`）・
    /// plugin_id 登録検証（`Scan` で `UnknownPluginId` — 登録の証明のみ。
    /// handle の所有 plugin かは caller 責務）・重複検出（current/failed/
    /// lingering のいずれかが一致する entry 既存 → `AlreadyWatched`）。
    /// **`Failed` handle は adopt** — `Destroy{handle}` で slot を解放し
    /// （キュー残の `InstanceFailed` は slot と共に落ちる — 後の poll で
    /// 二重復旧しない）、`Pending{next: None, failed: handle}` で登録して
    /// 次 poll の attempt で復旧する（caller が通知を消費済みでも復旧できる）。
    /// adopt の `Destroy` が `Err(InvalidHandle)`（既解放）でも adopt は継続 —
    /// caller の意図は plugin の復旧であり slot 解放は best-effort。
    /// handle の route（L0/L1）は問わない — L0 は通知しないため live `watch` は
    /// inert だが、`Failed` adopt 経路（L0 `Shared` slot の poisoned `Failed`
    /// 等）では通知なしでも復旧が走る。
    /// 初回 host 受取で `bound_host` を束縛 — 以後異なる host は `Err(WrongHost)`。
    pub fn watch(
        &mut self,
        host: &mut PluginHost,
        handle: PluginHandle,
        recipe: RestartRecipe,
    ) -> Result<SupervisedId, SupervisionError>;
    /// entry を除去 — `Live` なら `Some(current)`、`Pending`/`GaveUp` で
    /// `lingering` が残れば `Some(lingering)` を返す。返った handle の instance
    /// は live のまま caller へ返る（監視を外すだけで instance を殺さない —
    /// `Destroy`/salvage は caller 判断。lingering を返さないと orphan する）。
    /// `None` は未知 id または live handle を持たない entry — 区別が要る
    /// 場合は先に `current_handle` で問い合わせる。
    pub fn unwatch(&mut self, id: SupervisedId) -> Option<PluginHandle>;
    /// recipe 全置換（plugin_id の `Scan` 検証を含む）。
    /// 異なる host への呼出しは `Err(WrongHost)`。
    pub fn set_recipe(
        &mut self,
        host: &mut PluginHost,
        id: SupervisedId,
        recipe: RestartRecipe,
    ) -> Result<(), SupervisionError>;
    /// pre-failure checkpoint の登録 — caller が `SaveState` で取った blob。
    /// `id` キーなので restart 後の remint handle に対しても無条件に効く。
    /// `bytes.len() + WIRE_COMMAND_ENVELOPE > wire::WIRE_MAX_PAYLOAD`
    /// （envelope = seq 8 + instance 4 + verb 1 + len 4 = 17B、実効上限は
    /// `WIRE_MAX_STATE_BYTES`(16MiB) 未満）は `Err(CheckpointTooLarge)` —
    /// oversized blob は runner の `read_frame`/`decode` を失敗させて
    /// 子プロセスごと落とす（child-scope `Failed` + fan-out → attempt 毎の
    /// 繰返し kill）ため、wire に乗る前に拒否する。未知 id は `Err(NotWatched)`。
    pub fn set_checkpoint(
        &mut self,
        id: SupervisedId,
        bytes: Vec<u8>,
    ) -> Result<(), SupervisionError>;
    /// checkpoint 消去 — 以後の復旧は `RestoreState` を発行しない。未知 id は `false`。
    pub fn clear_checkpoint(&mut self, id: SupervisedId) -> bool;
    /// 現在の live minted handle — `Live` は `Some(current)`（`lingering`
    /// 残留時も current が優先 — lingering は内部 cleanup 対象）。
    /// `Pending`/`GaveUp` で `lingering` が残れば `Some(lingering)` —
    /// caller が `Destroy`/salvage できるよう露出する（orphan させない）。
    /// `Pending` で lingering なし・未知 id は `None`。
    pub fn current_handle(&self, id: SupervisedId) -> Option<PluginHandle>;
    /// 通知 drain → window reset → attempt 発行 → 結果 event 列を返す。
    /// caller の非 RT 制御ループで `Instant::now()` 等を注入して呼ぶ — `now`
    /// は non-decreasing が caller 契約（逆行は backoff deadline の早期発火・
    /// window reset 判定の歪みを招く — supervisor は検査しない）。
    /// attempt/deadline は poll 呼出し粒度で解決する — `next_attempt_at` 到達は
    /// 実時刻ではなく次回 `poll` まで遅れ、cap で繰越された `Pending` entry は
    /// attempt まで event を emit しない（`current_handle → None`・lingering
    /// residue が残れば `Some(lingering)` で観測）。
    /// **この呼出しが `try_recv_notification` の唯一の drain 点になる**
    /// （supervised 外の通知は `Unsupervised` で pass-through）。
    /// WARNING — caller が `poll` 外で `try_recv_notification` を drain すると
    /// supervised handle の `InstanceFailed` が失われ、entry は `Failed`
    /// instance を指したまま `Live` に滞留する（silent unrecoverable loss）。
    /// 束縛済みと異なる host では `WrongHost` event を emit して drain・
    /// window reset・attempt を一切行わない。
    pub fn poll(
        &mut self,
        host: &mut PluginHost,
        now: Instant,
    ) -> Vec<SupervisionEvent>;
}
```

補助コンストラクタ: `RestartPolicy::default()`（上記既定値）・`RestartRecipe::new(plugin_id)`（`config: None`・`start_processing: false`・`endpoint_bounds: None` — `Initialized` までの最小復旧。Processing 復旧は `config`+`start_processing: true` を caller が設定）。`Supervisor` 自体は `Send`/`Sync` 可能なデータのみ保持する（`PluginHost` を所有しない — `poll` で借りる。`bound_host` は `usize` — 生 pointer は `!Send`/`!Sync`）。`SupervisionEvent`/`SupervisionError`/`RestoreOutcome` は `#[non_exhaustive]`（公開 enum 方針 — additive 拡張の余地）— `SupervisionEvent` の struct variant は variant レベルでも `#[non_exhaustive]` で、後からの field 追加（`bounds`/`restored`/`last_error` 等）を additive にする。`RestartPolicy` は `Supervisor` 単位で 1 つ — per-entry の policy 上書きは deferred（YAGNI — Deferred 表）。`SupervisionError` は `Copy` を維持する — 将来 variant の payload は `Copy` 型（`HostError`・`SupervisedId`）に限定し、non-`Copy` payload が必要になった場合の `Copy` 除去は breaking change として別 Issue で扱う。

戻り値の `bool`/`Option` vs `Result` 非対称の根拠: host 対話（`lifecycle_state`・`Scan`）または入力検証（policy・checkpoint bound）が defined 拒否を持つ操作のみ `Result` を返す — `unwatch`/`clear_checkpoint` は host を触らない純簿記なので「entry が存在したか（+ caller へ返る handle）」だけを返す。

### 状態機械（per-supervised entry）

```
                watch                InstanceFailed(drain-Destroy)
  (none) ──────────────► Live ──────────────────────────► Pending ─┐
                         ▲   │                                      │ attempt
                         │   │ InstanceFailed                       │ 成功
            Restarted    │   │ (failures >= max_attempts)           │
            (new minted) │   ▼                                      ▼
                         └───┴──◄────────── GaveUp ◄──── attempt 失敗
                                terminal     │          (failures>=max)
                                             │ InstanceFailed(lingering)
                                             └─ Destroy → GaveUp 再 emit

  ※ 図の `watch` は live handle 経路。`Failed` handle の `watch` は adopt —
    (none) → Destroy → Pending で直接 `Pending{next: None, failed: h}` に入る
    （遷移表の adopt 行 — キュー残通知は `Destroy` の slot 解放で落ちる）。
```

- **Entry 共通簿記**（全 state で保持）: `recipe: RestartRecipe`・`checkpoint: Option<Vec<u8>>`・`failures`（episode 内で消費した attempt 数 — `stable_window` 経過で `last_error` と共に 0/`None` に reset）・`last_error: Option<HostError>`（直近 attempt 失敗 — `AttemptFailed.error`/`GaveUp.last_error` の元）・`lingering: Option<PluginHandle>`（cleanup が残した live handle — `Pending→Live` 遷移でも `Live` へ繰越し、retry は次の非 RT entry point（poll Phase 2）で再試行）。
- **`Live { handle, healthy_since: Option<Instant> }`** — 監視中・instance live。`healthy_since` は直近 `Restarted` 成功時点の `Some(now)` — watch 登録時は `None`（`watch` は `Instant` を取らない。`failures` は `Restarted` を経ないと 0 より大きくならないため、`None` の entry で reset predicate は評価不要 — `stable_window` の episode 境界は「観測された healthy 期間」を測る）。
- **`Pending { next_attempt_at: Option<Instant>, failed: PluginHandle }`** — 失敗済み・retry window 待ち。`next_attempt_at` が `None` は次 poll で即 attempt 可能（`watch` adopt 経路 — `watch` は `Instant` を取らないため deadline を注入できない）。`failed` は通知で届いた（または adopt した）旧 handle（`Restarted{old}` の echo 元）。
- **`GaveUp`** — terminal・caller `unwatch` まで保持。自動除去しない（spec deferred 行の結論 — caller 責務として残す方が最小）。共通簿記の `failures`/`last_error`/`lingering` は `GaveUp` 再 emit・`current_handle` 露出のために保持する。

遷移規則:

| 事象 | 現在 state | 動作 → 次 state |
| --- | --- | --- |
| `InstanceFailed{h}` drain（watched・current handle 一致） | `Live` | **`healthy_since = Some(t)` かつ `now >= t + stable_window` なら先に `failures = 0`・`last_error = None`**（window 越しの散発 crash が古い episode の failures を背負わない — reset は give-up 判定の前に評価）→ `Destroy{h}`（`Err` は無視 — 既に解放済みの外部 `Destroy` 競合を吸収）→ `failures >= max_attempts` なら `GaveUp` emit、さもなくば `Pending{next: Some(now), failed: h}` |
| `InstanceFailed{h}` drain（watched・`Pending.failed` または任意 state の `lingering` 一致 — 後発死） | `Pending`/`GaveUp`/`Live` | `Destroy{h}` → **一致したのが `lingering` の場合のみ `lingering = None`**（`failed`/current 一致で lingering を消すと残存 live handle が orphan する）。`Pending`/`Live` は state 不変（backoff/監視継続）、`GaveUp` は `GaveUp` event を再 emit（`attempts`/`last_error` 不変 — lingering の死を caller が `current_handle` 変化なしに見逃さないため。初回 emit との区別は `(id, attempts)` dedupe）。`Pending.failed` 一致は exactly-once で理論上到達しない（crash 起因の通知は Pending 遷移時に消費済み・adopt 起因は `Destroy` で slot と共に落ちる）が、一致しても idempotent `Destroy` で無害 |
| `poll` attempt（`next_attempt_at: None` または `now >= next_attempt_at`） | `Pending` | 下記 attempt シーケンス → 成功で `Live` + `Restarted`、失敗で `failures+=1`・`last_error = Some(error)` → `failures >= max_attempts` なら cleanup + `GaveUp` emit、さもなくば `Pending{next: Some(now + delay(failures))}` + `AttemptFailed` emit |
| `poll` window reset | `Live` で `failures > 0` かつ `healthy_since = Some(t)` かつ `now >= t + stable_window` | `failures = 0`・`last_error = None`（silent — event なし）。同一 predicate は Phase-1 drain の `failures >= max_attempts` 判定前にも評価される（上の drain 行 — episode は「live だった連続期間」で区切る） |
| `poll` window reset | `Pending`/`GaveUp` | 不適用（`Live` のみ reset） |
| `poll` lingering retry | `lingering: Some` の任意 entry | best-effort cleanup（unwind→`Destroy`）— `Ok` または `Err(InvalidHandle)`（外部 `Destroy` で既解放）で `lingering = None`、その他 `Err` は残して次 poll で再試行 |
| `watch`（`Failed` handle adopt） | （none） | `lifecycle_state(h) == Some(Failed)` → `Destroy{h}`（キュー残の `InstanceFailed` は slot 解放で落ちる — 二重復旧しない。`Err` は吸収 — `InvalidHandle` = 既解放でも adopt 継続）→ `Pending{next: None, failed: h}` で登録（failures/last_error/lingering は初期値 — `max_attempts = 0` なら次 poll で attempt せず Phase 3 の発行前判定で `GaveUp{0,None}` に収束） |
| `unwatch` | 任意 | entry 除去 → `Live.handle` または `lingering` が残れば `Some(handle)` で caller へ返す（instance は殺さない — lingering を返さないと orphan する） |

`max_attempts` 意味: episode 内で発行する attempt の**総数**上限。`failures`（消費済み attempt 数）が `max_attempts` に達した後の新規 `InstanceFailed` は attempt を発行せず即 `GaveUp`（drain 時の `Destroy` は常に実行 — Failed instance は terminal で slot を返す責務）。`max_attempts = 0` は「一切 attempt しない」— 監視して give up だけを報告する defined 動作（adopt 経路も同じ — `Pending{next:None}` で登録されても Phase 3 が発行前に `failures >= max_attempts` を検査し `GaveUp{0,None}` に収束する）。`max_attempts_per_poll = 0` は対称にならない — attempt が永久に発行されない inert 構成のため `Supervisor::new` が `Err(InertPolicy)` で拒否する（clamp しない — 0 を既定値へ読み替えるのは caller の意図を変える）。

### attempt シーケンス（`Pending` → replay）

```
Instantiate{recipe.plugin_id}                    // handle 未 mint — Err で即 attempt 失敗
  → HostReply::Instantiated{new}
Initialize{new}                                   // Created → Initialized
let mut restored = RestoreOutcome::NotRequested
if let Some(bytes) = &entry.checkpoint:           // bytes は set_checkpoint の bound で
                                                  // 必ず 1 wire frame に収まる
    match RestoreState{new, bytes}:
        Ok(())             → restored = Applied
        Err(Unsupported)   → restored = Skipped   // skip — replay 継続（attempt 消費なし）
        Err(e)             → attempt 失敗へ（失敗分類表）
if recipe.config.is_some():
    Activate{new, config}                         // Initialized → Activated
if recipe.config.is_some() && recipe.start_processing:
    StartProcessing{new}                          // Activated → Processing —
                                                  // config: None では Activate が走らず
                                                  // Initialized 止まり（start_processing
                                                  // 単独では StartProcessing を発行しない）
→ Restarted{id, old: entry.failed, new,
            bounds: recipe.endpoint_bounds, restored}
```

**replay 順序の根拠**: `RestoreState` を `Activate` **前**に置く — plugin の `prepare`（Activate hook）が restore 済み状態を見て resource 確保できる順序（plugin 側で「config が state 依存」の一般的ケースに対応）。`RestoreState` は runner dispatch に lifecycle gate が無く `Initialized` で合法（`l1` `post_command_state` が `Some(_)` を返さない = 状態不変 — discovery 観測）。`Initialize` は常に必須（`Created` → `Initialized` の合法遷移のみ存在）。

### 失敗分類表（通知/コマンド結果 → 動作 — spec R-S6 の確定形）

| 段階・事象 | 結果 | 分類・動作 |
| --- | --- | --- |
| `InstanceFailed{h}`（watched） | — | 回復トリガ — drain 時点で `Destroy{h}` → `Pending`（`failures>=max` なら `GaveUp`） |
| `InstanceFailed{h}`（unwatched） | — | `Unsupervised{n}` event で pass-through — caller が自分で `Destroy`/復旧する責務 |
| `Instantiate` | `Err(UnknownPluginId/Ipc/InstanceLimit)` | attempt 失敗（handle 未 mint — cleanup 不要。`InstanceLimit` は `pool_eligible` が満杯の子を再利用対象から除外するため公開面から到達不能 — `composite.rs` L1092-1095・`Err(Ipc)` と同じ分類 arm で review-covered） |
| `Instantiate` mid-`CreateInstance` 死亡 | `Err(Ipc)` + 同居 sibling へ fan-out 通知が次 poll に到着 | attempt 失敗（兄弟の通知は通常経路で別 entry として処理） |
| `Initialize` | `Err(PluginFailed/Ipc/Failed/Unsupported)` | attempt 失敗 → minted（`Created` — `Destroy` 合法）を cleanup |
| `RestoreState` | `Err(Unsupported)` | **skip — state 非対応として replay 継続**（attempt 消費なし・checkpoint は entry に保持 → 次 attempt で再適用を試みる）。`Restarted.restored = Skipped` で caller が観測できる |
| `RestoreState` | `Err(PluginFailed/Ipc/Failed/Lifecycle/Prepare/InvalidHandle)` | attempt 失敗 → cleanup（保守的決定 — 下記注） |
| `Activate` | `Err(Prepare/Lifecycle/PluginFailed/Ipc/Failed)` | attempt 失敗 → cleanup（`Initialized` から `Destroy` 合法） |
| `StartProcessing` | `Err(PluginFailed/Ipc/Failed/Lifecycle)` | attempt 失敗 → instance は `Activated` — unwind `Deactivate` → `Initialized` → `Destroy` |
| mid-replay 子死亡 | 途中 command が `Err(Ipc)`/`Err(Failed)` で返る | attempt 失敗 → mirror は `Failed` → `Destroy` 合法（unwind 不要） |
| cleanup unwind（`StopProcessing`/`Deactivate`/`Destroy`） | `Err` | best-effort — live handle を `lingering` として entry に保持・`current_handle` で露出（リークしない・放置もしない）。retry で `Err(InvalidHandle)` が返れば外部 `Destroy` で既解放として `lingering` をクリアする。`lingering` は単一 slot — 占有 residue の inline retry が再失敗した二重失敗角のみ、新 residue は追跡せず `Orphaned{id, handle}` emit で caller 簿記へ委ねる（観測可能な accepted corner — instance は殺さず、後発死は `Unsupervised` で届く） |
| drain 時 `Destroy` | `Err` | 無視して続行（外部 `Destroy` 競合で既に解放済みの場合を吸収 — 回復対象の判定は通知で既に完了） |
| `set_checkpoint` | `bytes.len() + 17 > wire::WIRE_MAX_PAYLOAD` | **`Err(CheckpointTooLarge)` — attempt に到達させない**。`wire::encode` は `WIRE_MAX_*` bound を encode 側で検査しない設計（over-bound field は verbatim encode → peer の defined rejection — `wire.rs` L729-733・L990-992）であり、oversized `RestoreState` frame は runner の `read_frame`（`payload_len > WIRE_MAX_PAYLOAD` → io error → `ExitCode::FAILURE`）または `decode`（`read_bytes(WIRE_MAX_STATE_BYTES)` → `LimitExceeded` → `FAILURE`）で子プロセスを落とす — `converge_child` で同居全 minted インスタンスが `Failed` + `InstanceFailed` fan-out し、supervisor restart は attempt 毎に繰返される kill になる |

cleanup 規則（attempt 失敗時・minted handle 存在時）: `lifecycle_state(new)` で mirror を読み、`Processing` → `StopProcessing`→`Deactivate`→`Destroy`、`Activated` → `Deactivate`→`Destroy`、`Created`/`Initialized`/`Failed` → `Destroy`。unwind の各 step が `Err` ならそれ以降を打ち切り `lingering` に残す — **retry は poll Phase 2（次の非 RT entry point）で再試行する**（`Err(InvalidHandle)` は既解放としてクリア、`GaveUp` 時の cleanup でも同じ unwind を試行）。`Pending→Live` 成功遷移で `lingering` は `Live` entry に繰越される（`current_handle` は current が優先 — lingering は内部 cleanup 対象）。

**`RestoreState` `Err(PluginFailed)` 系が attempt budget を burn するのは保守的決定** — restore を拒否し続ける plugin は `max_attempts` 分の budget を消費して `GaveUp` するが、supervisor が caller の checkpoint を黙って捨てて default state で動かすことはしない（`AttemptFailed{error}`/`GaveUp{last_error}` を見た caller が `clear_checkpoint` を呼べば次の episode で checkpoint なし復旧に degrade できる）。代替案（restore 専用 budget の分離・`Err(PluginFailed)` 時の自動 `clear_checkpoint`・`Err(Unsupported)` 以外も skip 扱い）は初版では却下 — caller の state 所有権と「復旧できたが state が戻らない」の区別を supervisor が越権しないため。

### `poll` アルゴリズム（確定）

```
poll(host, now) -> Vec<SupervisionEvent>:
    events = []
    // Phase 0 — host 束縛（初回 host 受取で bound_host = key を確定。
    // 不一致は drain/attempt を一切行わず defined 拒否 — 他 supervisor の
    // drain 権・通知を奪わない。`poll -> Vec` は spec pin のため rejection は
    // event 経路）
    let key = host as *const PluginHost as usize
    match bound_host:
        Some(b) if b != key => { emit WrongHost; return events }
        None => bound_host = Some(key)
        _ => {}
    // Phase 1 — 通知 drain（poll が唯一の drain 点）
    while let Some(n) = host.try_recv_notification():        // drain_endpoint_channels も同時に動く
        let InstanceFailed{handle, ..} = n                    // 将来 variant 追加時は wildcard で Unsupervised へ
        if let Some((id, e)) = find_watched(handle):            // match-set:
            //   Live.handle（current）・Pending.failed（その entry の現行 handle）・
            //   任意 state の lingering — `failed` 一致は exactly-once で理論上
            //   到達しないが idempotent Destroy で無害
            destroy_failed(host, handle)                      // Destroy{h} — Err は無視
            if e.lingering == Some(handle): e.lingering = None
            //   ↑ 一致が lingering の場合のみクリア — failed/current 一致で
            //   消すと残存 live handle が orphan する
            if e is Live && handle == e.handle:               // current handle の死
                if e.healthy_since.is_some_and(|t| now >= t + stable_window):
                    e.failures = 0; e.last_error = None       // give-up 判定前に episode reset —
                                                              // window 越しの散発 crash
                if e.failures >= max_attempts: e → GaveUp; emit GaveUp{id, e.failures, e.last_error}
                else:                        e → Pending{next: Some(now), failed: handle}
            elif e is GaveUp:              emit GaveUp{id, e.failures, e.last_error}
            //   ↑ lingering の後発死を報告（attempts/last_error 不変 —
            //   初回 emit との区別は `(id, attempts)` dedupe）
            // Pending/Live 中の lingering 一致通知は state 不変 — backoff/監視継続
        else:
            emit Unsupervised{n}
    // Phase 2 — stable window reset（Live・Phase 1 で通知を受けなかったもの）
    //         + lingering cleanup retry（次の非 RT entry point）
    for e in Live where e.failures > 0
        && e.healthy_since.is_some_and(|t| now >= t + stable_window):
        e.failures = 0; e.last_error = None
    for e in entries where e.lingering.is_some():
        cleanup(host, e)                                      // unwind→Destroy best-effort —
        // Ok / Err(InvalidHandle)（外部解放済み）で e.lingering = None、
        // その他 Err は残して次 poll で再試行
    // Phase 3 — attempt 発行（ready 順 + SupervisedId 昇順で determinism・cap まで）
    budget = policy.max_attempts_per_poll
    for (id, e) in entries where Pending && (e.next_attempt_at.is_none() || now >= e.next_attempt_at),
        sorted by (next_attempt_at, id):                      // None（即時）は最小値として先頭
        if e.failures >= max_attempts:
            e → GaveUp; emit GaveUp{id, e.failures, e.last_error}; continue
            // ↑ adopt 経路は drain の give-up 判定を通らないため Phase 3 側で
            //   検査 — `max_attempts = 0` の adopt Pending{failures:0} が
            //   attempt を発行しない。attempt 消費なしのため budget 外
            //   （cap に関わらず全件即時報告 — drain の `GaveUp{0,None}` と同じ意味）
        if budget == 0: break                                 // 残りは次 poll へ繰越し —
            // cap 繰越しの Pending は attempt まで event を emit しない
            // （caller は `current_handle → None`・lingering が残れば
            //   `Some(lingering)` で観測）
        budget -= 1
        match attempt(host, id, e):
            Ok((new, restored)) => {
                let old = e.failed                            // Pending.failed — Restarted.old の echo 元
                // lingering は共通簿記なので Live へ自然に繰越 —
                // cleanup は次 poll の Phase 2 で retry
                e → Live{handle: new, healthy_since: now}
                emit Restarted{id, old, new,
                               bounds: e.recipe.endpoint_bounds, restored}
            }
            Err(error) => {
                e.failures += 1; e.last_error = Some(error)
                cleanup(host, e)                              // unwind→Destroy best-effort・残ったら lingering
                if e.failures >= max_attempts: e → GaveUp; emit GaveUp{id, e.failures, e.last_error}
                else: { e.next_attempt_at = Some(now + delay(e.failures)); emit AttemptFailed{id, e.failures, error} }
            }
    events
```

backoff 式: 通知側は `next_attempt_at = Some(now)`（**初回 attempt は即時** — crash 後の dead time を最小化。backoff は attempt 失敗の繰返しにのみ効く）。`watch` adopt 経路は `next_attempt_at = None`（次 poll で即 attempt — `watch` は `Instant` を取らないため deadline を注入できない）。attempt 失敗後の delay: `delay(k) = min(base_delay · 2^(k-1), max_delay)`（`k = failures` — attempt 1 失敗 → `base`、2 失敗 → `2·base`、…。指数部は 31 で飽和・`Duration::checked_mul` が `None` なら `max_delay` に clamp）。`Instant` は `checked_add` で進める — `poll` 引数注入なので backoff 検証は `now` を進めるだけで済む。**deadline は poll 呼出し粒度で解決する** — `now >= next_attempt_at` を満たすのは実時刻ではなく次の `poll` 呼出しであり、`now` は caller が non-decreasing を保証する（`Instant::now()` で自然に成立 — 逆行は deadline の早期発火・window reset 判定の歪みを招くが、supervisor は検査しない caller 契約）。

### checkpoint 所有（push 型 — spec 軸 4 (a) 確定）

```
caller 側:   host.execute(SaveState{handle}) → HostReply::State{bytes}
             → supervisor.set_checkpoint(id, bytes)     // id キー — remint 後も有効。
                                                       // bound 超過は Err(CheckpointTooLarge)
supervisor:  entry.checkpoint = Some(bytes)
recovery:    Initialize → RestoreState{new, bytes}（失敗分類は上表）
```

- `SaveState` は non-RT・allocate 可・plugin hook 呼出 — **supervisor は `poll` 内で `SaveState` を駆動しない**（頻度・タイミングは caller のドメイン知識: param 変更 settle・定期・保存要求時）。pull 型は spec matrix で却下済み。
- **`set_checkpoint` は blob に bound を課す** — `bytes.len() + WIRE_COMMAND_ENVELOPE（= 17B: seq 8 + instance 4 + verb 1 + len prefix 4）> wire::WIRE_MAX_PAYLOAD`（16MiB — `WIRE_MAX_STATE_BYTES` と同値のため実効上限は `WIRE_MAX_STATE_BYTES − 17`）を `Err(CheckpointTooLarge)` で拒否する。`wire::encode` は `WIRE_MAX_*` bound を encode 側で検査しない（over-bound field は verbatim encode → peer の defined rejection — `wire.rs` L729-733・L990-992）ため、チェックしないまま attempt が `RestoreState` を送信すると runner の `read_frame`/`decode` が失敗して子プロセスごと落ちる（`serve` loop L190-208 → `ExitCode::FAILURE` → `converge_child` で同居全 minted インスタンス `Failed` + fan-out — attempt 毎に繰返される kill）。caller 直接の `HostCommand::RestoreState` と plugin 側 `SaveState` reply（child→parent 方向も同じ codec bound — host `reader_main` が同様に converge する、l1/mod.rs L1104-1110）には同種の穴が残る — **pre-existing core gap で別 Issue 級の follow-up 候補**（Deferred 表）。
- `Failed` 後は `SaveState` が `Err(Failed)` — 構造的に pre-failure push しか取れない（不変条件）。
- checkpoint 未登録の entry は `RestoreState` を発行せず `Initialize` → `Activate` で復旧する（plugin default state — `Restarted.restored = NotRequested`）。
- `RestoreState` `Err(Unsupported)` は skip（attempt 消費なし — 決定 5）— state 非対応 plugin で max_attempts 消費して give up する退化を防ぐ（`Restarted.restored = Skipped` で観測可能）。

### bundle fan-out・pool 連携（spec R-S10 確定）

- **1 子死亡 = N 個の独立回復イベント** — phase 1 で同一 `ChildExit` の N 通知を全件 drain → 各 supervised handle が独立 entry で `Pending` 化 → phase 3 で cap（`max_attempts_per_poll`）まで attempt を発行し、残りは次 poll に繰越す（`next_attempt_at` は既に elapsed のまま）。
- **子プロセス数は pool 判定に委ねる** — `Bundled` route の fresh 子に N インスタンスが順次載る（`Instantiate` の scan が terminal 子を evict → 最初の attempt が fresh spawn + `CreateInstance`、後続は同子に `pool_eligible` で載る）。supervisor が「N 通知 = N 子 spawn」と解釈・強制してはならない（`Bundled` の意味を壊す — spec matrix 確定）。
- **`ChildExit` 値で復可否を分岐しない** — `None` 許容は「帰属違反（子生存の per-instance 収束）」と「status 未到着（`Pending` flush）」の両義であり分岐材料にならない。supervisor は全 supervised 失敗を一律に回復対象とする（「exit で route 選択しない」spec 決定の延長）。
- mid-`CreateInstance` 死亡: attempt 中の `Instantiate` が `Err(Ipc)` で返る（通知は出ない — handle 未 mint）+ 同居兄弟への fan-out 通知は次 poll の drain で別 entry として処理される（supervisor が他の回復を進行中でも競合しない — entry 単位の独立 state machine）。

### endpoint/RT 再配線（caller 責務 — spec 軸 8 (a) 確定）

- `Restarted{id, old, new, bounds, restored}` 発行時点で新インスタンスは **endpoint 未 attach**。caller は event を受けて `host.prepare_endpoint(new, bounds.unwrap_or(...))` を呼び、返された `PluginEndpoint` を RT owner（graph node 等）へ差し替える。`bounds` は recipe の `endpoint_bounds` echo — recipe が `config: None`（`Initialized` 止まり）では `prepare_endpoint` が `Err(Lifecycle)` で拒否するため、bounds echo は `config: Some` で `Activated` 以上に到達する recipe でのみ意味を持つ（recipe 整合性は caller 責務）。
- supervisor が attach しない構造的根拠: `PluginEndpoint` は `Send` として RT owner スレッドへ **move** する所有物であり、supervisor（control スレッド）が保持しても届ける経路（mpsc 等）を別途要求することになる + `EndpointBounds` は caller のドメイン知識 — #113 の non-RT 制御/RT 処理所有分離契約（`docs/designs/engine-plugin-host-integration.md`）の延長で、復旧後の再配線は caller が握る。
- 旧 handle は `mint` の generation bump で `Err(InvalidHandle)` に stale — caller は handle 写像・endpoint・param bookkeeping を `new` へ付け替える責務を持つ。`Restarted{old,new}` はその rebind を駆動するトリガ。

### 基底契約の不変性（変更しない面）

- `PluginHost` core（型・`execute`/`try_recv_notification`/`instantiate`/`destroy`/`mint`/`slot_mut`/`routes`/`instances`/`generations`/`pool`）・`HostNotification`/`HostError`/`ChildExit`/wire/`PluginHandle` — 一切無変更（AC4）。
- `Failed`/`Failed → Destroyed` の唯一脱出・通知 per-instance exactly once・`ChildExit{None,None}` 許容・`Creating` 中の死亡は通知なし — 全て現行どおり。
- `OpenGui`/`CloseGui` の L1 `Err(Unsupported)` 不変 — supervisor は GUI を扱わない。
- 復旧中の RT 側は現行どおり pass-through/無音（caller が endpoint 再 attach するまで `process` の意味は変わらない）。
- supervisor 関連の全操作は非 RT — `poll` 呼出し自体が非 RT 前提（`Instantiate`/`Destroy`/`RestoreState` は reply-wait を伴う）。

## Deferred Design Concerns

| 論点 | 本書での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| route 単位 watch（同一 plugin_id 全インスタンスを一括 supervise） | deferred — per-instance `watch` が最小構成 | 実装 Issue または後続 Issue（API 追加は additive） | 複数インスタンスを個別 `watch` する手間 — per-instance で十分の可能性が高い |
| pull 型 checkpoint（supervisor が `poll` で定期 `SaveState`） | 却下（spec matrix 確定） | caller が `SaveState` を駆動し `set_checkpoint` — 自動化は caller 側パターン | save 頻度のドメイン最適化が効かなくなる |
| `execute` wrap による recipe 自動追跡（command 傍受） | deferred — `PluginHost` 全面 wrapper が必要・`&mut` borrow で複雑化 | 後続拡張（`watch`/`set_recipe` の明示宣言が初版） | recipe 陳腐化リスクは caller 責任のまま |
| `rebind` API（外部 `Destroy`→`Instantiate` 後の handle 追従） | deferred — `unwatch`→`watch` で表現可能（新 `SupervisedId` が mint される点だけ異なる） | 必要が実証された段階で additive 追加 | caller が新 id を受け取る手間（checkpoint は再 `set_checkpoint`） |
| mid-`DestroyInstance` 死の通知消失検出（`lifecycle_state` 走査 fallback） | 却下（YAGNI）— 通知が落ちても `Failed` 自体は `lifecycle_state` で観測可能だが、supervisor は走査しない | caller 側 `unwatch` または手動復旧で対処 | 通知が `Destroy` 前に落ちたインスタンスを supervisor が見逃す（頻度は低い） |
| `GaveUp` 後の自動 unwatch | 却下 — terminal 保持で caller 判断を残す | `unwatch`（caller 責務） | entry が残る（caller が `unwatch` を忘れると supervise 簿記が残る） |
| `BundleFailed` 相関通知 variant | 却下（#117 ADR と同じ結論 — `#[non_exhaustive]` で後追加可能） | 必要が実証された段階で `HostNotification` に additive 追加（core 側・別 Issue 級） | bundle crash を「1 イベント」として相関表示したい consumer は `ChildExit` 同値から推測する |
| health-check/heartbeat による予防的失敗検出 | 却下（non-goal）— L1 は crash 後回復の supervisor であり予防監視ではない | — | ハングは `command_timeout` → 子スコープ `Failed` として既に検出される |
| keep-alive / warm pool（常駐待機子） | 却下（non-goal）— spawn は `Instantiate` 時にのみ行う現行契約 | — | 初回 attempt の spawn コストは残る（数 ms〜十数 ms 級） |
| per-entry `RestartPolicy` 上書き（entry 毎の attempts/backoff 個別設定） | deferred — `RestartPolicy` は `Supervisor` 単位で 1 つ（初版の最小構成） | 実装 Issue または後続 Issue（additive — `watch` 引数 or `set_policy(id, …)`） | 同一 supervisor 内の全 entry が同じ budget/backoff を共有 — 個別チューニング不可 |
| caller 直接の `HostCommand::RestoreState` / `SaveState` reply の oversized blob（wire encode 側に `WIRE_MAX_*` 検査なし → 子 `read_frame`/`decode` 失敗で child-scope `Failed`） | pre-existing core gap — `set_checkpoint` は bound 拒否で防御済みだが、supervisor 非経由の `RestoreState` と plugin 側 `SaveState` reply（host `reader_main` も同じ codec bound で converge — l1/mod.rs L1104-1110）には同種の穴が残る | follow-up 候補 — core の encode-side bound チェック or host-side validate（`ruviform-plugin-core` wire/`ruviform-plugin-host` 層の別 Issue 級） | caller が bound 超過 blob を持つ plugin を使うと supervisor 非経由でも子を殺せる（attempt 毎の繰返し kill と同型の blast radius） |

## Risks and Trade-offs

| リスク / トレードオフ | 評価と緩和 |
| --- | --- |
| **caller が `poll` を呼ばないと復旧が走らない** | `try_recv_notification` 自体が caller drain 型の既存パターンと同じ責務分担 — supervisor を導入する consumer は control loop に `poll` を置くだけ。`Unsupervised` pass-through で「drain し忘れ」の責務を移譲しない（supervised/unsupervised の区別なく通知は必ず誰かに届く） |
| **crash-loop が `max_attempts` + `stable_window` を潜る** | `stable_window` より早い crash で attempts が蓄積し bounded に give up。window より遅い crash は「stable であった」と解釈して reset — 意図したトレード（毎月 1 回落ちる plugin を永久禁錮しない）。`stable_window` を 0 にすると「成功毎に reset = crash-loop 無制限」になる点は doc で pin |
| **`poll` 内の復旧コスト集中（bundle crash + N attempt）** | `max_attempts_per_poll` で 1 呼出しの spawn/reply-wait 回数を bound。残りは `next_attempt_at` elapsed のまま次 poll で処理 — spawn は pool が 1 子へ収束させるため 2 回目以降は `CreateInstance` round-trip のみ |
| **checkpoint blob の staleness** | `set_checkpoint` は push 型 — caller が最後に save した時点の blob をそのまま適用する。restore は best-effort（`Err(Unsupported)` skip・その他 `Err` は attempt 失敗で retry — checkpoint を捨てて default state で動かす選択は caller の `clear_checkpoint`） |
| **checkpoint blob の oversized → 子 kill** | `set_checkpoint` が `bytes.len() + envelope > WIRE_MAX_PAYLOAD` を `Err(CheckpointTooLarge)` で拒否 — wire に乗る前に止める（encode は `WIRE_MAX_*` を検査しない設計）。同種の穴は caller 直接の `RestoreState`・`SaveState` reply 側に pre-existing core gap として残る — Deferred 表で follow-up 候補として記録 |
| **restore 拒否 plugin が budget を burn する** | `Err(PluginFailed)` 系は保守的に attempt 失敗 — `max_attempts` 分消費して `GaveUp`。supervisor は checkpoint を黙って捨てない（caller の state 所有権）— `AttemptFailed`/`GaveUp` の `error`/`last_error` を見た caller が `clear_checkpoint` で degrade できる |
| **recipe 陳腐化（`Activate` config 変更を `set_recipe` し忘れ）** | caller 責務として明記 — wrap 追跡は deferred。失敗は `Err(Prepare)` → attempt 失敗として surface するので静かな退化ではない |
| **`recipe.plugin_id` の misdeclare → cross-plugin resurrection** | `watch`/`set_recipe` の `Scan` 検証は「id が登録済み」の証明のみ — handle がその plugin のインスタンスかは `PluginHost` に逆引き面がないため検証不可能（AC4 境界）。declared id が誤っていても `Instantiate` は成功し別 plugin が live handle として帰属する — caller 責務として `watch` doc・決定 10・本表で明記 |
| **Bundled 共有子上の crash-loop が同居 supervised entry の budget を削る（blast radius）** | A の crash-loop（共有子の反復死亡）は `converge_child` で同居 B/C の `Failed` + `InstanceFailed` fan-out を引き、B/C の `failures` が A 起因で蓄積して各 entry が give up し得る — 同居インスタンスを被害者にしない運用は `Dedicated` route での opt-out（isolation は登録時指定で recipe 側からは変えられない） |
| **caller が `poll` 外で `try_recv_notification` を drain → supervised 通知の silent unrecoverable loss** | sole-drain 契約を `poll` doc の WARNING・決定 8・本表で pin — supervised handle の `InstanceFailed` を caller が先に消費すると entry は `Failed` instance を指したまま `Live` に滞留し、二度と復旧も `GaveUp` もしない。deferred の `lifecycle_state` 走査 fallback はまさにこの検出手段（現行は caller 側 `unwatch` + 手動復旧で対処） |
| **同 crate 内 private 面への手出し** | 型システムでは遮れないため設計契約 + 「supervisor テストは公開 API 経由のみ」で担保 — 実装 Issue の review 観点に明記 |
| **`Unsupervised` 通知の取りこぼし** | `poll` が全通知を drain し未監視分は pass-through — supervisor 採用 consumer は `try_recv_notification` を直接呼ばない契約を doc で pin（`poll` が唯一の drain 点 — 違反時の挙動は上の drain 損失行） |

## Verification Notes

> spec `docs/specs/issue-116-supervisor.md` の AC/R-V への対応。supervisor は `PluginHost` 公開 API のみ使用するため検証は全て公開面経由で書ける（`l1-test-worker` + `#[path]` harness は `#[doc(hidden)] kill_child` 経由で crash 注入可能）。

- [x] **spec AC1（opt-in util 面・core 無変更）**: `src/supervisor.rs` が公開 API（`execute`/`try_recv_notification`/`lifecycle_state`/`Scan`）のみで動作し、`pub(crate)` L1 machinery へ触れないこと（review + test が全て公開面経由で書かれていることで実証）。`composite.rs`/`l1/` の diff が無いこと（実績: merge-base `fc02103` 対比 `composite.rs`/`l1/`/`host.rs`/`wire.rs` byte-level 無 diff・`supervisor.rs` の `use` は `lib.rs` 公開面のみ・`l1_supervisor_e2e.rs` が `#[path]` 非使用 — IR IMPL-034NU3jXjckCIx8sPNr1Wz coverage 表）
- [x] **spec AC2（同 route 限定再 spawn）**: `Bundled`/`Dedicated` 両 route で `InstanceFailed` → `Destroy` → `Instantiate{plugin_id}` → `Restarted{old,new,…}`（新 handle mint・generation bump・旧 handle が `Err(InvalidHandle)`）を検証。他 route の `plugin_id` への instantiate が発生しないこと（実績: `restarted_dedicated_replays_recipe_and_rewires`/`restarted_bundled_reaches_each_recipe_state`（`tests/l1_supervisor_e2e.rs:347,404`）全 pass）
- [x] **spec AC3 / R-V1（bundle fan-out）**: `Bundled` route で `kill_child`（1 子死亡）→ N supervised handle の `InstanceFailed` 同一 `ChildExit` 到着 → `poll` で各 supervised が `Destroy` → `Instantiate` され全員新 handle で復旧（子数は pool 判定 — N 子 spawn を強制しない）。supervised 外 handle は `Unsupervised` で pass-through。`max_attempts_per_poll` による cap 繰越しを `Instant` 注入で検証（実績: `bundle_burst_cap_defers_and_unsupervised`（`l1_supervisor_e2e.rs:1142`）+ `poll_caps_attempts_and_orders_ready`（`supervisor.rs:2108`）pass）
- [x] **spec AC4 / R-V3（checkpoint/restore）**: pre-failure に `set_checkpoint` した blob が `Restarted` 前に `RestoreState{bytes}` で適用される。`Err(Unsupported)` は skip（attempt 消費なし — state 非対応 fixture）で `Restarted.restored = Skipped`、未登録は `RestoreState` を発行せず `restored = NotRequested`。**fixture gap — `l1-test-worker` の `RestoreState` は常に `Ok`（seed blob `0x11,0x22,0x33,0x44` を clone するだけ — l1-test-worker.rs L235-237）で `Err(Unsupported)`/`Err(PluginFailed)` を生起できない**: 実装 task に `l1-test-worker` への flag 追加（`--fail-restore`/`--state-unsupported` 相当 — `RestoreState` を `Err(PluginFailed)`/`Err(Unsupported)` で返す）を含める（AC4/AC7/R-V3 の前提）。bound 超過 blob の `Err(CheckpointTooLarge)` 拒否も検証（子 kill に到達しないこと）（実績: fixture gap は T1 `db80ead` で解消・`checkpoint_applied_roundtrips_state`/`checkpoint_unsupported_skips_without_attempt`/`checkpoint_pluginfailed_burns_attempts_to_gaveup`（`l1_supervisor_e2e.rs:467,521,558`）+ `set_checkpoint_wire_bound`（`supervisor.rs:1580`）全 pass）
- [x] **spec AC5 / R-V4（bounded retry）**: `Instant` 注入で backoff deadline を跨いだ再試行・`max_attempts` 超過で `GaveUp`・attempts が respawn 越しに継承・`stable_window` 経過で `failures` reset を検証。`max_attempts = 0` が即 `GaveUp` になること（drain 経路・`watch` adopt 経路の双方 — adopt の `Pending{next:None}` は drain の give-up 判定を通らないため Phase 3 発行前検査で `GaveUp{0,None}` に収束する unit pin あり）。**`stable_window` 経過後の crash が drain 時点の pre-check で `failures` reset を先に評価すること**（旧 `failures` のまま即 `GaveUp` しない回帰 pin — `healthy_since + stable_window` を超えた `InstanceFailed` で `failures = 0` → `Pending` へ進む）・`GaveUp.last_error` が episode 直近の attempt 失敗を運ぶこと（attempt 未発行の give up は `None`）・`max_attempts_per_poll = 0` が `Supervisor::new` で `Err(InertPolicy)` になること（実績: `poll_backoff_gates_attempt_timing`/`delay_grows_exponentially_and_clamps`/`drain_resets_stable_window_before_give_up`/`adopt_zero_max_attempts_gives_up`/`drain_zero_max_attempts_gives_up`（`supervisor.rs:2089,1658,1750,1821,1801`）+ `crash_loop_bounded_gaveup`/`stable_window_crash_starts_fresh_episode`/`zero_max_attempts_adopt_gives_up_immediately`/`inert_policy_rejected`（`l1_supervisor_e2e.rs:804,1064,1038,1316`）全 pass）
- [x] **spec AC6（give up 意味）**: `GaveUp` 時に mint 済み instance があれば unwind + `Destroy` 済み・entry は `unwatch` まで保持・`HostError::Failed` 語彙を `GaveUp` に使わないこと（`GaveUp` event は別 enum variant）。lingering 後発死の `GaveUp` 再 emit が `attempts`/`last_error` 不変で届くこと（`(id, attempts)` dedupe で初回 emit と区別できる）（実績: `crash_loop_bounded_gaveup`/`gaveup_lingering_late_death_reemits`/`lingering_residue_unwatch_returns_handle`（`l1_supervisor_e2e.rs:804,948,917`）+ `drain_gave_up_lingering_reemits`/`cleanup_unwinds_by_mirror_state`（`supervisor.rs:1899,2209`）全 pass）
- [x] **spec AC7 / R-V2（attempt 中の失敗・mid-create 死）**: `Instantiate` `Err(Ipc)` で attempt 失敗・handle 未 mint（cleanup 不要）・同居 sibling への fan-out が次 poll で別 entry として処理される。`Err(InstanceLimit)` は公開面から到達不能（`pool_eligible` が満杯の子を再利用対象から除外 — `composite.rs` L1092-1095・single-threaded scan で sibling mint が割り込まない）ため注入検証せず、`Err(Ipc)` と同じ「`Instantiate` `Err` → attempt 失敗・handle 未 mint」分類 arm で review-covered とする。lifecycle replay 中の `Err` → unwind + `Destroy` で残留リソースなし。unwind 失敗で残った live handle が `lingering` として `current_handle` に露出され、次 poll Phase 2 で retry されること（`Err(InvalidHandle)` で clear）— 注入は `--fail-start-processing`（`StartProcessing` 不 commit `Err` → minted `Activated` 残留・unwind は `Deactivate` 起点）+ `--crash-after-msgs`/`--hang-after-msgs` を unwind 段の frame に合わせた子死亡/応答停止（Dedicated fresh 子・checkpoint 未登録 — Scan=1・CreateInstance=2・Initialize=3・Activate=4・StartProcessing=5・unwind `Deactivate`=6 で死亡 → `Err(Ipc)`・minted `Failed` 収束 → retry `Destroy` は `Failed` から合法。`RestoreState` が入れば以降 +1）。`--fail-stop-processing` 類の注入は到達不能 — unwind が `StopProcessing` を発行するには minted mirror `Processing` が必要で、それは `StartProcessing` `Ok` = attempt 成功（cleanup なし）時のみ成立する（実績: `mid_create_death_attempt_failed_without_mint`/`fail_initialize_attempt_cleans_created`/`fail_start_processing_attempt_unwinds_activated`/`activate_over_limit_attempt_fails_prepare`/`lingering_residue_retry_destroy_clears`/`lingering_residue_external_destroy_clears`/`lingering_residue_unwatch_returns_handle`（`l1_supervisor_e2e.rs:725,615,647,682,849,886,917`）全 pass — crash 変種で注入し `--hang-after-msgs` は未使用・`--fail-stop-processing` は設計上の到達不能どおり非経路）
- [x] **spec AC8（driver 契約）**: `poll(&mut host, now)` で駆動・内部スレッド/タイマー/async なし（型・フィールド検査で `Send`/`Sync` 可能なデータのみ — `bound_host` は `usize`）。`!Send`/`!Sync` の `PluginHost` と同一スレッドで動くこと。**host 束縛の検証**: `watch` 後に異なる host で `watch`/`set_recipe` が `Err(WrongHost)`・`poll` が `WrongHost` event を emit して drain/attempt しないこと（他 host の通知を消費しない）（実績: `assert_impl_all!(Supervisor: Send, Sync)`/`assert_not_impl_any!(PluginHost: Send, Sync)`（`supervisor.rs:1169-1171`）+ `host_binding_rejects_foreign_host`/`poll_binds_host_on_first_call`（1468,1513）+ `wrong_host_rejected_without_drain`（`l1_supervisor_e2e.rs:1238`）全 pass）
- [x] **spec AC9（event 報告）**: `Restarted{id,old,new,bounds,restored}` が返り、`new` で `prepare_endpoint`/`latency_frames`/`lifecycle_state` が呼べること。`#[non_exhaustive]` enum（struct variant も `#[non_exhaustive]`）。`Unsupervised` が未監視通知を pass-through すること。**`watch` の `Failed` handle adopt**: `lifecycle_state → Some(Failed)` の handle を `watch` → `Destroy` で slot 解放（キュー残通知は落ちる = 二重復旧しない）→ `Pending` で次 poll attempt → `Restarted` で復旧（caller 消費済み通知の系でも同じ復旧を確認）。`unwatch` が `Option<PluginHandle>`（`Live.current` または `lingering`）を返して handle を orphan させないこと（実績: `restarted_dedicated_replays_recipe_and_rewires`/`adopt_failed_handle_recovers`/`bundle_burst_cap_defers_and_unsupervised`/`lingering_residue_unwatch_returns_handle`（`l1_supervisor_e2e.rs:347,986,1142,917`）+ `watch_*`/`unwatch_returns_handle_by_state`/`drain_unwatched_passes_through`（`supervisor.rs:1392-1434,1526,1922`）全 pass）
- [x] **spec R-V5（AC4 無退行）**: supervisor 未使用の `l1_proxy_e2e` 系全テストが変わらず通ること（`PluginHost` core 無 diff の証明）（実績: `l1_proxy_e2e` 100/100 無 diff green・`plain_host_reports_failed_no_autorestart`（`l1_supervisor_e2e.rs:1285`）pass — IR IMPL-034NU3jXjckCIx8sPNr1Wz）
- [x] **spec R-V6（endpoint 再配線）**: `Restarted` の `new` で `prepare_endpoint`（`Restarted.bounds` の echo — `config: Some` で `Activated` 到達する recipe）が動くこと（実績: `restarted_dedicated_replays_recipe_and_rewires`（`l1_supervisor_e2e.rs:347`）+ `rt_l1_supervised_process_no_alloc`（1334）pass）
- [x] **spec AC10（CI/監査）**: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス・`doc_status.js audit` blocking 0・findings なし（実績: T4 Full Gate 全コマンド初回 exit 0 — `test` 60 binaries・1294 passed・`test-rt` 137 一致 0 failed・doc audit `blocking:0`・`findings:[]`）

## Sources and Relations

- Issue [#116](https://github.com/scarecrow173/ruviform/issues/116)（本書の対象 Issue — supervisor 運用面の確定）・Issue [#31](https://github.com/scarecrow173/ruviform/issues/31)（プラグイン分離設計・CLOSED — 再起動判断/状態復元責務帰属の起点）・Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)（L1 実装・CLOSED — `Failed`+通知+AC4 基底契約）・Issue [#117](https://github.com/scarecrow173/ruviform/issues/117)（bundle 子プロセス・implemented — `Failed` 粒度・fan-out・pool・`WIRE_MAX_INSTANCES`・depends-on）・Issue [#113](https://github.com/scarecrow173/ruviform/issues/113)（engine プラグイン統合 — `PluginEndpoint` 所有権・復旧後の再配線関連）・Issue [#114](https://github.com/scarecrow173/ruviform/issues/114)（実規格プラグイン L1 E2E — 実 adapter fixture 活用の候補）・Issue [#115](https://github.com/scarecrow173/ruviform/issues/115)（L1 GUI — out of scope）・Issue [#119](https://github.com/scarecrow173/ruviform/issues/119)（wire バージョン方針 — wire 変更なし）・Issue [#120](https://github.com/scarecrow173/ruviform/issues/120)（io 多重化 — poll 型 supervisor は競合なし）
- [issue-116-supervisor discovery](../discovery/issue-116-supervisor.md) — 本書の discovery（コード観測事実・選択肢比較・推奨方向）
- [issue-116-supervisor spec](../specs/issue-116-supervisor.md) — 本書が確定する spec（Requirements・decision matrix・AC — `implements`）
- [issue-116-supervisor-placement ADR](../adr/issue-116-supervisor-placement.md) — 配置確定の ADR（util 層・core 無変更 — `related`）
- [plugin-isolation design](plugin-isolation.md)・[plugin-isolation-strategy ADR](../adr/plugin-isolation-strategy.md) — #31 確定判断の正本
- [plugin-l1-out-of-process spec](../specs/plugin-l1-out-of-process.md)・[design](plugin-l1-out-of-process.md) — #76 系上流（`Failed`/通知/pass-through/非 RT/AC4・deferred 行 L301 の起点 — `deferred-by` 先）
- [issue-117-bundle-child-process spec](../specs/issue-117-bundle-child-process.md)・[design](issue-117-bundle-child-process.md)・[ADR](../adr/issue-117-bundle-child-failed-granularity.md) — bundle 後の `Failed` 粒度・fan-out・pool・`WIRE_MAX_INSTANCES`（本書の失敗粒度前提 — `depends-on`）
- [engine-plugin-host-integration design](engine-plugin-host-integration.md) — `PluginEndpoint` の detach/attach 所有権（復旧後の再配線が caller 責務である根拠 — `related`）
