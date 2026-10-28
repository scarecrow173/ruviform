---
id: "DESIGN-034NNxNMrrk6OoIGfX18rk"
type: "design"
status: "draft"
title: "L1 supervisor（自動再起動 util 層）設計（Issue #116）"
created: "2026-10-28"
updated: "2026-10-28"
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
  derived-by: []
  verifies: []
  verified-by: []
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
| 4 | backoff 形状 | **`RestartPolicy{max_attempts, base_delay, max_delay, stable_window, max_attempts_per_poll}`** — 失敗通知受信で即時 attempt・attempt 失敗後に `min(base_delay · 2^(attempts-1), max_delay)` 遅延。attempts は episode カウンタ（restart 成功で即 reset せず、`stable_window` 生存で reset — crash-loop bounded・散発 crash は無制限回復） |
| 5 | `RestoreState` `Err(Unsupported)` 分類 | **skip（attempt 消費しない）** — state 非対応 plugin でも復旧が動く。`Err(PluginFailed)`/`Err(Ipc)`/`Err(Failed)`/その他 `Err` は attempt 失敗 |
| 6 | attempt 内失敗の後片付け | minted handle が live なら lifecycle 逆順 unwind（`Processing`→`StopProcessing`→`Activated`→`Deactivate`→`Initialized`）→ `Destroy`。unwind 自体が `Err` なら live handle を `lingering` として entry に保持・`current_handle` で露出（リークしない・放置もしない） |
| 7 | bundle burst | 1 poll = 通知全 drain + entry 毎独立回復。`max_attempts_per_poll`（既定 8）で attempt 発行を cap — 残りは次 poll へ繰越し。子プロセス数は pool 判定に委ねる（supervisor は「N 通知 = N 子」と解釈しない） |
| 8 | 通知 drain 責任 | **`poll` が唯一の drain 点** — 未監視 handle の通知は `SupervisionEvent::Unsupervised` で pass-through（caller の手動復旧責務は維持） |
| 9 | `GaveUp` 後の扱い | entry は terminal 保持・**自動 unwatch しない**（caller が `unwatch` or re-watch を判断）。`GaveUp` は `HostError::Failed` に畳まない別語彙 — `Failed` は「子死亡で live インスタンスが terminal」意味を維持 |
| 10 | `watch` 検証 | `watch(host, handle, recipe)` — `lifecycle_state` で live 検証（`Err(InvalidHandle)`）・`Scan` で plugin_id 登録検証（`Err(UnknownPluginId)`）・同一 current_handle の重複 watch は `Err(AlreadyWatched)` |
| 11 | `watch` 後の handle 変化 | caller が supervisor 経路外で `Destroy`→`Instantiate` した場合は `unwatch`→`watch` で追従する契約（`rebind` API は YAGNI） |
| 12 | mid-`DestroyInstance` 死の通知消失 | **検出しない** — `poll` が `lifecycle_state` を走査する fallback は YAGNI（頻度が低く、caller 側 `unwatch` で対処可能） |
| 13 | route 単位 watch | **deferred** — per-instance `watch` が最小構成（同一 plugin_id 全インスタンスを supervise する route 宣言は追加機能として残す） |
| 14 | give up 語彙 | `SupervisionEvent::GaveUp` — circuit breaker 的 `Err(Failed)` fold は却下（spec matrix と同じ結論） |
| 15 | endpoint 再配線 | `RestartRecipe.endpoint_bounds` を recipe に保持し `Restarted` event へ echo — **`prepare_endpoint` の呼出しは caller 責務**（`PluginEndpoint` は RT owner スレッドへの move が前提・supervisor が届ける経路がない構造的必然） |

## Architecture

### 配置と境界（ADR 一体）

- `crates/ruviform-plugin-host/src/supervisor.rs` — `mod supervisor;` を lib.rs に追加し、`Supervisor`/`SupervisedId`/`RestartPolicy`/`RestartRecipe`/`SupervisionEvent`/`SupervisionError` を `pub use` で再エクスポートする。
- **公開 API のみ使用**: `execute`・`try_recv_notification`・`lifecycle_state`・`Scan`（`execute` 経由）に限定して `PluginHost` を駆動する。`pub(crate)` L1 machinery（`l1::` モジュール・`composite` の `Route`/`Slot`/pool 内部型）には一切触れない。**これは型システムではなく設計契約として強制する** — 同 crate 内のためコンパイラは private 面を遮らないが、supervisor の crate テストを全て公開 API 経由で書くことで実証的に担保する（`l1-test-worker` + `#[path]` harness をそのまま利用可能）。
- **core 無変更**: `execute`/`try_recv_notification`/`instantiate`/`destroy`/`mint`/`slot_mut`/`routes`/`instances`/`generations`/`pool` の型・意味・フィールドは一切変更しない。`PluginHost` に handle→plugin_id 逆引き面・recipe 簿記・restart 動詞を追加しない — AC4「自動再起動なし」の境界を型・意味レベルで維持する。
- **`Supervisor` は `Send`/`Sync` 可能なデータのみ保持**（`PluginHost` を所有しない — `poll` の引数で `&mut` 借用）。poll は host を所有するスレッド＝caller の非 RT 制御ループで呼ぶ。内部スレッド・タイマー・async runtime は持たない。
- **依存追加なし** — `std::time::{Instant, Duration}`・`BTreeMap`・既存公開型（`PluginHost`/`PluginHandle`/`HostCommand`/`HostReply`/`HostError`/`HostNotification`/`EndpointBounds`・`ruviform_plugin_core::ProcessConfig`）のみ。
- **1 `Supervisor` = 1 `PluginHost` の drain 点**: 複数 `Supervisor` で同一 host を poll すると先に drain した側が全通知を消費する（後述の単一 drain 点契約）— 複数 host は `Supervisor` 毎に 1 対 1 で対応させる。

### 公開 API 面（スケッチ — 命名・型は実装 Issue で最終確定）

```rust
/// `InstanceFailed` 後の機械的復旧を担う opt-in util —
/// `PluginHost` を `&mut` で借りて caller の非 RT ループで poll 駆動する。
pub struct Supervisor { /* entries: BTreeMap<SupervisedId, Entry>, next_id, policy */ }

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
    /// restart 後この時間生存したら attempts を 0 に reset する episode 境界。
    pub stable_window: Duration,     // 既定 60s
    /// 1 poll あたりの attempt 発行上限（bundle burst の spawn 集中を bound）。
    pub max_attempts_per_poll: u32,  // 既定 8
}

#[derive(Clone, Debug)]
pub struct RestartRecipe {
    /// 同 route 再 instantiate 対象 — `Instantiate{plugin_id}` に渡す登録時 claim。
    pub plugin_id: String,
    /// `Some` なら replay で `Activate{config}`。`None` は `Initialized` で止める。
    pub config: Option<ProcessConfig>,
    /// `config` 成功後に `StartProcessing` まで進めるか（`config: None` では無視）。
    pub start_processing: bool,
    /// `Restarted` へ echo する呼出側再 attach 用 bounds —
    /// supervisor は `prepare_endpoint` を呼ばない（caller 責務）。
    pub endpoint_bounds: Option<EndpointBounds>,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum SupervisionEvent {
    /// 監視対象が復旧した — caller は handle 写像・endpoint・param 簿記を
    /// `new` へ付け替える（`old` は stale — `Err(InvalidHandle)` になる）。
    Restarted { id: SupervisedId, old: PluginHandle, new: PluginHandle },
    /// 1 回の復旧 attempt が失敗した（backoff またはこの poll で `GaveUp` が続く）。
    AttemptFailed { id: SupervisedId, attempt: u32, error: HostError },
    /// episode の attempt 上限に達した — mint 済みインスタンスが残っていれば
    /// unwind + `Destroy` 済み（`current_handle` は残存 handle のみ返す）。
    GaveUp { id: SupervisedId, attempts: u32 },
    /// `watch` されていない handle の通知 — pass-through（caller が自分で捌く）。
    Unsupervised { notification: HostNotification },
}

#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SupervisionError {
    /// `lifecycle_state` が `None` — stale/未知 handle。
    InvalidHandle,
    /// 同一 current_handle の entry が既に存在する。
    AlreadyWatched,
    /// `recipe.plugin_id` が `Scan` の登録 descriptor に無い。
    UnknownPluginId,
    /// 未知の `SupervisedId`。
    NotWatched,
}
```

```rust
impl Supervisor {
    pub fn new(policy: RestartPolicy) -> Self;
    /// 監視登録 — live 検証（`InvalidHandle`）・plugin_id 登録検証
    /// （`Scan` で `UnknownPluginId`）・重複検出（`AlreadyWatched`）。
    /// handle の route（L0/L1）は問わない — L0 は通知しないので inert。
    pub fn watch(
        &mut self,
        host: &mut PluginHost,
        handle: PluginHandle,
        recipe: RestartRecipe,
    ) -> Result<SupervisedId, SupervisionError>;
    /// entry を除去 — Live なら instance は live のまま caller へ返る
    /// （監視を外すだけで instance を殺さない）。未知 id は `false`。
    pub fn unwatch(&mut self, id: SupervisedId) -> bool;
    /// recipe 全置換（plugin_id の `Scan` 検証を含む）。
    pub fn set_recipe(
        &mut self,
        host: &mut PluginHost,
        id: SupervisedId,
        recipe: RestartRecipe,
    ) -> Result<(), SupervisionError>;
    /// pre-failure checkpoint の登録 — caller が `SaveState` で取った blob。
    /// `id` キーなので restart 後の remint handle に対しても無条件に効く。
    pub fn set_checkpoint(&mut self, id: SupervisedId, bytes: Vec<u8>) -> bool;
    /// checkpoint 消去 — 以後の復旧は `RestoreState` を発行しない。
    pub fn clear_checkpoint(&mut self, id: SupervisedId) -> bool;
    /// 現在の live minted handle — `Live` は `Some(current)`、
    /// `Pending`（未 mint）は `None`、`GaveUp` は lingering が残れば `Some`。
    pub fn current_handle(&self, id: SupervisedId) -> Option<PluginHandle>;
    /// 通知 drain → window reset → attempt 発行 → 結果 event 列を返す。
    /// caller の非 RT 制御ループで `Instant::now()` 等を注入して呼ぶ。
    /// **この呼出しが `try_recv_notification` の唯一の drain 点になる**
    /// （supervised 外の通知は `Unsupervised` で pass-through）。
    pub fn poll(
        &mut self,
        host: &mut PluginHost,
        now: Instant,
    ) -> Vec<SupervisionEvent>;
}
```

補助コンストラクタ: `RestartPolicy::default()`（上記既定値）・`RestartRecipe::new(plugin_id)`（`config: None`・`start_processing: false`・`endpoint_bounds: None` — `Initialized` までの最小復旧。Processing 復旧は `config`+`start_processing: true` を caller が設定）。`Supervisor` 自体は `Send`/`Sync` 可能なデータのみ保持する（`PluginHost` を所有しない — `poll` で借りる）。`SupervisionEvent`/`SupervisionError` は `#[non_exhaustive]`（公開 enum 方針 — additive 拡張の余地）。

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
```

- **`Live { handle, failures, healthy_since }`** — 監視中・instance live。`failures` は episode 内で消費した attempt 数（`stable_window` 経過で 0 に reset）。`healthy_since` は watch 時点または直近 `Restarted` 成功時点。
- **`Pending { next_attempt_at, failed: PluginHandle, lingering: Option<PluginHandle> }`** — 失敗済み・retry window 待ち。`failed` は通知で届いた旧 handle（`Restarted{old}` の echo 元）。`lingering` は attempt 失敗の cleanup が残した live handle（通常 `None`）。
- **`GaveUp { lingering: Option<PluginHandle> }`** — terminal・caller `unwatch` まで保持。自動除去しない（spec deferred 行の結論 — caller 責務として残す方が最小）。

遷移規則:

| 事象 | 現在 state | 動作 → 次 state |
| --- | --- | --- |
| `InstanceFailed{h}` drain（watched） | `Live` | `Destroy{h}`（`Err` は無視 — 既に解放済みの外部 `Destroy` 競合を吸収）→ `failures >= max_attempts` なら `GaveUp` emit、さもなくば `Pending{next: now, failed: h}` |
| `InstanceFailed{h}` drain（watched・lingering handle の後発死） | `Pending`/`GaveUp` | `Destroy{h}` → `Pending` は backoff 継続（state 不変・`lingering` クリア）、`GaveUp` は `GaveUp` event を再 emit（lingering の死を caller が `current_handle` 変化なしに見逃さないため） |
| `poll` attempt（`now >= next_attempt_at`） | `Pending` | 下記 attempt シーケンス → 成功で `Live` + `Restarted`、失敗で `failures+=1` → `failures >= max_attempts` なら cleanup + `GaveUp` emit、さもなくば `Pending{next: now + delay(failures)}` + `AttemptFailed` emit |
| `poll` window reset | `Live` で `failures > 0` かつ `now - healthy_since >= stable_window` | `failures = 0`（silent — event なし） |
| `poll` window reset | `Pending`/`GaveUp` | 不適用（`Live` のみ reset — episode は「live だった連続期間」で区切る） |
| `unwatch` | 任意 | entry 除去（instance は殺さない — Live は caller へ返る） |

`max_attempts` 意味: episode 内で発行する attempt の**総数**上限。`failures`（消費済み attempt 数）が `max_attempts` に達した後の新規 `InstanceFailed` は attempt を発行せず即 `GaveUp`（drain 時の `Destroy` は常に実行 — Failed instance は terminal で slot を返す責務）。`max_attempts = 0` は「一切 attempt しない」— 監視して give up だけを報告する defined 動作。

### attempt シーケンス（`Pending` → replay）

```
Instantiate{recipe.plugin_id}                    // handle 未 mint — Err で即 attempt 失敗
  → HostReply::Instantiated{new}
Initialize{new}                                   // Created → Initialized
if entry.checkpoint.is_some():
    RestoreState{new, bytes}                      // Err(Unsupported) → skip（attempt 消費なし）
if recipe.config.is_some():
    Activate{new, config}                         // Initialized → Activated
if recipe.start_processing:
    StartProcessing{new}                          // Activated → Processing
→ Restarted{id, old: entry.failed, new}
```

**replay 順序の根拠**: `RestoreState` を `Activate` **前**に置く — plugin の `prepare`（Activate hook）が restore 済み状態を見て resource 確保できる順序（plugin 側で「config が state 依存」の一般的ケースに対応）。`RestoreState` は runner dispatch に lifecycle gate が無く `Initialized` で合法（`l1` `post_command_state` が `Some(_)` を返さない = 状態不変 — discovery 観測）。`Initialize` は常に必須（`Created` → `Initialized` の合法遷移のみ存在）。

### 失敗分類表（通知/コマンド結果 → 動作 — spec R-S6 の確定形）

| 段階・事象 | 結果 | 分類・動作 |
| --- | --- | --- |
| `InstanceFailed{h}`（watched） | — | 回復トリガ — drain 時点で `Destroy{h}` → `Pending`（`failures>=max` なら `GaveUp`） |
| `InstanceFailed{h}`（unwatched） | — | `Unsupervised{n}` event で pass-through — caller が自分で `Destroy`/復旧する責務 |
| `Instantiate` | `Err(UnknownPluginId/Ipc/InstanceLimit)` | attempt 失敗（handle 未 mint — cleanup 不要） |
| `Instantiate` mid-`CreateInstance` 死亡 | `Err(Ipc)` + 同居 sibling へ fan-out 通知が次 poll に到着 | attempt 失敗（兄弟の通知は通常経路で別 entry として処理） |
| `Initialize` | `Err(PluginFailed/Ipc/Failed/Unsupported)` | attempt 失敗 → minted（`Created` — `Destroy` 合法）を cleanup |
| `RestoreState` | `Err(Unsupported)` | **skip — state 非対応として replay 継続**（attempt 消費なし・checkpoint は entry に保持 → 次 attempt で再適用を試みる） |
| `RestoreState` | `Err(PluginFailed/Ipc/Failed/Lifecycle/Prepare/InvalidHandle)` | attempt 失敗 → cleanup |
| `Activate` | `Err(Prepare/Lifecycle/PluginFailed/Ipc/Failed)` | attempt 失敗 → cleanup（`Initialized` から `Destroy` 合法） |
| `StartProcessing` | `Err(PluginFailed/Ipc/Failed/Lifecycle)` | attempt 失敗 → instance は `Activated` — unwind `Deactivate` → `Initialized` → `Destroy` |
| mid-replay 子死亡 | 途中 command が `Err(Ipc)`/`Err(Failed)` で返る | attempt 失敗 → mirror は `Failed` → `Destroy` 合法（unwind 不要） |
| cleanup unwind（`StopProcessing`/`Deactivate`/`Destroy`） | `Err` | best-effort — live handle を `lingering` として entry に保持・`current_handle` で露出（リークしない・放置もしない） |
| drain 時 `Destroy` | `Err` | 無視して続行（外部 `Destroy` 競合で既に解放済みの場合を吸収 — 回復対象の判定は通知で既に完了） |

cleanup 規則（attempt 失敗時・minted handle 存在時）: `lifecycle_state(new)` で mirror を読み、`Processing` → `StopProcessing`→`Deactivate`→`Destroy`、`Activated` → `Deactivate`→`Destroy`、`Created`/`Initialized`/`Failed` → `Destroy`。unwind の各 step が `Err` ならそれ以降を打ち切り `lingering` に残す（次 poll・次 attempt で再 cleanup するか、`GaveUp` 時に再度試行）。

### `poll` アルゴリズム（確定）

```
poll(host, now) -> Vec<SupervisionEvent>:
    events = []
    // Phase 1 — 通知 drain（poll が唯一の drain 点）
    while let Some(n) = host.try_recv_notification():        // drain_endpoint_channels も同時に動く
        let InstanceFailed{handle, ..} = n                    // 将来 variant 追加時は wildcard で Unsupervised へ
        if let Some((id, e)) = find_watched(handle):
            destroy_failed(host, handle)                      // Destroy{h} — Err は無視
            e.lingering = None
            if e is Live:
                if e.failures >= max_attempts: e → GaveUp; emit GaveUp{id, e.failures}
                else:                        e → Pending{next: now, failed: h}
            elif e is GaveUp:              emit GaveUp{id, e.failures}   // lingering の後発死を報告
            // Pending 中の通知（lingering handle の後発死）は backoff 継続 — state 不変
        else:
            emit Unsupervised{n}
    // Phase 2 — stable window reset（Live のみ）
    for e in Live where e.failures > 0 && now >= e.healthy_since + stable_window:
        e.failures = 0
    // Phase 3 — attempt 発行（ready 順 + SupervisedId 昇順で determinism・cap まで）
    budget = policy.max_attempts_per_poll
    for (id, e) in entries where Pending && now >= e.next_attempt_at, sorted by (next_attempt_at, id):
        if budget == 0: break                                // 残りは次 poll へ繰越し
        budget -= 1
        match attempt(host, id, e):
            Ok(new) => { e → Live{new, healthy_since: now}; emit Restarted{id, old: e.failed, new} }
            Err(error) => {
                e.failures += 1
                cleanup(host, e)                              // unwind→Destroy best-effort・残ったら lingering
                if e.failures >= max_attempts: e → GaveUp; emit GaveUp{id, e.failures}
                else: { e.next_attempt_at = now + delay(e.failures); emit AttemptFailed{id, e.failures, error} }
            }
    events
```

backoff 式: 通知側は `next_attempt_at = now`（**初回 attempt は即時** — crash 後の dead time を最小化。backoff は attempt 失敗の繰返しにのみ効く）。attempt 失敗後の delay: `delay(k) = min(base_delay · 2^(k-1), max_delay)`（`k = failures` — attempt 1 失敗 → `base`、2 失敗 → `2·base`、…。指数部は 31 で飽和・`Duration::checked_mul` が `None` なら `max_delay` に clamp）。`Instant` は `checked_add` で進める — `poll` 引数注入なので backoff 検証は `now` を進めるだけで済む。

### checkpoint 所有（push 型 — spec 軸 4 (a) 確定）

```
caller 側:   host.execute(SaveState{handle}) → HostReply::State{bytes}
             → supervisor.set_checkpoint(id, bytes)     // id キー — remint 後も有効
supervisor:  entry.checkpoint = Some(bytes)
recovery:    Initialize → RestoreState{new, bytes}（失敗分類は上表）
```

- `SaveState` は non-RT・allocate 可・plugin hook 呼出 — **supervisor は `poll` 内で `SaveState` を駆動しない**（頻度・タイミングは caller のドメイン知識: param 変更 settle・定期・保存要求時）。pull 型は spec matrix で却下済み。
- `Failed` 後は `SaveState` が `Err(Failed)` — 構造的に pre-failure push しか取れない（不変条件）。
- checkpoint 未登録の entry は `RestoreState` を発行せず `Initialize` → `Activate` で復旧する（plugin default state）。
- `RestoreState` `Err(Unsupported)` は skip（attempt 消費なし — 決定 5）— state 非対応 plugin で max_attempts 消費して give up する退化を防ぐ。

### bundle fan-out・pool 連携（spec R-S10 確定）

- **1 子死亡 = N 個の独立回復イベント** — phase 1 で同一 `ChildExit` の N 通知を全件 drain → 各 supervised handle が独立 entry で `Pending` 化 → phase 3 で cap（`max_attempts_per_poll`）まで attempt を発行し、残りは次 poll に繰越す（`next_attempt_at` は既に elapsed のまま）。
- **子プロセス数は pool 判定に委ねる** — `Bundled` route の fresh 子に N インスタンスが順次載る（`Instantiate` の scan が terminal 子を evict → 最初の attempt が fresh spawn + `CreateInstance`、後続は同子に `pool_eligible` で載る）。supervisor が「N 通知 = N 子 spawn」と解釈・強制してはならない（`Bundled` の意味を壊す — spec matrix 確定）。
- **`ChildExit` 値で復可否を分岐しない** — `None` 許容は「帰属違反（子生存の per-instance 収束）」と「status 未到着（`Pending` flush）」の両義であり分岐材料にならない。supervisor は全 supervised 失敗を一律に回復対象とする（「exit で route 選択しない」spec 決定の延長）。
- mid-`CreateInstance` 死亡: attempt 中の `Instantiate` が `Err(Ipc)` で返る（通知は出ない — handle 未 mint）+ 同居兄弟への fan-out 通知は次 poll の drain で別 entry として処理される（supervisor が他の回復を進行中でも競合しない — entry 単位の独立 state machine）。

### endpoint/RT 再配線（caller 責務 — spec 軸 8 (a) 確定）

- `Restarted{id, old, new}` 発行時点で新インスタンスは **endpoint 未 attach**。caller は event を受けて `host.prepare_endpoint(new, recipe.endpoint_bounds.unwrap_or(...))` を呼び、返された `PluginEndpoint` を RT owner（graph node 等）へ差し替える。
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

## Risks and Trade-offs

| リスク / トレードオフ | 評価と緩和 |
| --- | --- |
| **caller が `poll` を呼ばないと復旧が走らない** | `try_recv_notification` 自体が caller drain 型の既存パターンと同じ責務分担 — supervisor を導入する consumer は control loop に `poll` を置くだけ。`Unsupervised` pass-through で「drain し忘れ」の責務を移譲しない（supervised/unsupervised の区別なく通知は必ず誰かに届く） |
| **crash-loop が `max_attempts` + `stable_window` を潜る** | `stable_window` より早い crash で attempts が蓄積し bounded に give up。window より遅い crash は「stable であった」と解釈して reset — 意図したトレード（毎月 1 回落ちる plugin を永久禁錮しない）。`stable_window` を 0 にすると「成功毎に reset = crash-loop 無制限」になる点は doc で pin |
| **`poll` 内の復旧コスト集中（bundle crash + N attempt）** | `max_attempts_per_poll` で 1 呼出しの spawn/reply-wait 回数を bound。残りは `next_attempt_at` elapsed のまま次 poll で処理 — spawn は pool が 1 子へ収束させるため 2 回目以降は `CreateInstance` round-trip のみ |
| **checkpoint blob の staleness** | `set_checkpoint` は push 型 — caller が最後に save した時点の blob をそのまま適用する。restore は best-effort（`Err(Unsupported)` skip・その他 `Err` は attempt 失敗で retry — checkpoint を捨てて default state で動かす選択は caller の `clear_checkpoint`） |
| **recipe 陳腐化（`Activate` config 変更を `set_recipe` し忘れ）** | caller 責務として明記 — wrap 追跡は deferred。失敗は `Err(Prepare)` → attempt 失敗として surface するので静かな退化ではない |
| **同 crate 内 private 面への手出し** | 型システムでは遮れないため設計契約 + 「supervisor テストは公開 API 経由のみ」で担保 — 実装 Issue の review 観点に明記 |
| **`Unsupervised` 通知の取りこぼし** | `poll` が全通知を drain し未監視分は pass-through — supervisor 採用 consumer は `try_recv_notification` を直接呼ばない契約を doc で pin（`poll` が唯一の drain 点） |

## Verification Notes

> spec `docs/specs/issue-116-supervisor.md` の AC/R-V への対応。supervisor は `PluginHost` 公開 API のみ使用するため検証は全て公開面経由で書ける（`l1-test-worker` + `#[path]` harness は `#[doc(hidden)] kill_child` 経由で crash 注入可能）。

- [ ] **spec AC1（opt-in util 面・core 無変更）**: `src/supervisor.rs` が公開 API（`execute`/`try_recv_notification`/`lifecycle_state`/`Scan`）のみで動作し、`pub(crate)` L1 machinery へ触れないこと（review + test が全て公開面経由で書かれていることで実証）。`composite.rs`/`l1/` の diff が無いこと
- [ ] **spec AC2（同 route 限定再 spawn）**: `Bundled`/`Dedicated` 両 route で `InstanceFailed` → `Destroy` → `Instantiate{plugin_id}` → `Restarted{old,new}`（新 handle mint・generation bump・旧 handle が `Err(InvalidHandle)`）を検証。他 route の `plugin_id` への instantiate が発生しないこと
- [ ] **spec AC3 / R-V1（bundle fan-out）**: `Bundled` route で `kill_child`（1 子死亡）→ N supervised handle の `InstanceFailed` 同一 `ChildExit` 到着 → `poll` で各 supervised が `Destroy` → `Instantiate` され全員新 handle で復旧（子数は pool 判定 — N 子 spawn を強制しない）。supervised 外 handle は `Unsupervised` で pass-through。`max_attempts_per_poll` による cap 繰越しを `Instant` 注入で検証
- [ ] **spec AC4 / R-V3（checkpoint/restore）**: pre-failure に `set_checkpoint` した blob が `Restarted` 前に `RestoreState{bytes}` で適用される。`Err(Unsupported)` は skip（attempt 消費なし — state 非対応 fixture）。checkpoint 未登録は `RestoreState` を発行しない
- [ ] **spec AC5 / R-V4（bounded retry）**: `Instant` 注入で backoff deadline を跨いだ再試行・`max_attempts` 超過で `GaveUp`・attempts が respawn 越しに継承・`stable_window` 経過で `failures` reset を検証。`max_attempts = 0` が即 `GaveUp` になること
- [ ] **spec AC6（give up 意味）**: `GaveUp` 時に mint 済み instance があれば unwind + `Destroy` 済み・entry は `unwatch` まで保持・`HostError::Failed` 語彙を `GaveUp` に使わないこと（`GaveUp` event は別 enum variant）
- [ ] **spec AC7 / R-V2（attempt 中の失敗・mid-create 死）**: `Instantiate` `Err(Ipc)`/`Err(InstanceLimit)` で attempt 失敗・handle 未 mint（cleanup 不要）・同居 sibling への fan-out が次 poll で別 entry として処理される。lifecycle replay 中の `Err` → unwind + `Destroy` で残留リソースなし
- [ ] **spec AC8（driver 契約）**: `poll(&mut host, now)` で駆動・内部スレッド/タイマー/async なし（型・フィールド検査で `Send`/`Sync` 可能なデータのみ）。`!Send`/`!Sync` の `PluginHost` と同一スレッドで動くこと
- [ ] **spec AC9（event 報告）**: `Restarted{id,old,new}` が返り、`new` で `prepare_endpoint`/`latency_frames`/`lifecycle_state` が呼べること。`#[non_exhaustive]` enum。`Unsupervised` が未監視通知を pass-through すること
- [ ] **spec R-V5（AC4 無退行）**: supervisor 未使用の `l1_proxy_e2e` 系全テストが変わらず通ること（`PluginHost` core 無 diff の証明）
- [ ] **spec R-V6（endpoint 再配線）**: `Restarted` の `new` で `prepare_endpoint`（`endpoint_bounds` echo）が動くこと
- [ ] **spec AC10（CI/監査）**: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス・`doc_status.js audit` blocking 0・findings なし

## Sources and Relations

- Issue [#116](https://github.com/scarecrow173/ruviform/issues/116)（本書の対象 Issue — supervisor 運用面の確定）・Issue [#31](https://github.com/scarecrow173/ruviform/issues/31)（プラグイン分離設計・CLOSED — 再起動判断/状態復元責務帰属の起点）・Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)（L1 実装・CLOSED — `Failed`+通知+AC4 基底契約）・Issue [#117](https://github.com/scarecrow173/ruviform/issues/117)（bundle 子プロセス・implemented — `Failed` 粒度・fan-out・pool・`WIRE_MAX_INSTANCES`・depends-on）・Issue [#113](https://github.com/scarecrow173/ruviform/issues/113)（engine プラグイン統合 — `PluginEndpoint` 所有権・復旧後の再配線関連）・Issue [#114](https://github.com/scarecrow173/ruviform/issues/114)（実規格プラグイン L1 E2E — 実 adapter fixture 活用の候補）・Issue [#115](https://github.com/scarecrow173/ruviform/issues/115)（L1 GUI — out of scope）・Issue [#119](https://github.com/scarecrow173/ruviform/issues/119)（wire バージョン方針 — wire 変更なし）・Issue [#120](https://github.com/scarecrow173/ruviform/issues/120)（io 多重化 — poll 型 supervisor は競合なし）
- [issue-116-supervisor discovery](../discovery/issue-116-supervisor.md) — 本書の discovery（コード観測事実・選択肢比較・推奨方向）
- [issue-116-supervisor spec](../specs/issue-116-supervisor.md) — 本書が確定する spec（Requirements・decision matrix・AC — `implements`）
- [issue-116-supervisor-placement ADR](../adr/issue-116-supervisor-placement.md) — 配置確定の ADR（util 層・core 無変更 — `related`）
- [plugin-isolation design](plugin-isolation.md)・[plugin-isolation-strategy ADR](../adr/plugin-isolation-strategy.md) — #31 確定判断の正本
- [plugin-l1-out-of-process spec](../specs/plugin-l1-out-of-process.md)・[design](plugin-l1-out-of-process.md) — #76 系上流（`Failed`/通知/pass-through/非 RT/AC4・deferred 行 L301 の起点 — `deferred-by` 先）
- [issue-117-bundle-child-process spec](../specs/issue-117-bundle-child-process.md)・[design](issue-117-bundle-child-process.md)・[ADR](../adr/issue-117-bundle-child-failed-granularity.md) — bundle 後の `Failed` 粒度・fan-out・pool・`WIRE_MAX_INSTANCES`（本書の失敗粒度前提 — `depends-on`）
- [engine-plugin-host-integration design](engine-plugin-host-integration.md) — `PluginEndpoint` の detach/attach 所有権（復旧後の再配線が caller 責務である根拠 — `related`）
