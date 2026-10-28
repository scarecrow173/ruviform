---
id: "ADR-034NNxNMDjKssvme4VQnrv"
type: "adr"
status: "accepted"
title: "L1 supervisor 配置 — `ruviform-plugin-host` opt-in util 層と core `PluginHost` 無変更境界（Issue #116）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/116"
  changes:
    added: []
    modified: []
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
    - "docs/designs/issue-116-supervisor.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-116-supervisor.md"
    - "docs/discovery/issue-116-supervisor.md"
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  derived-by:
    - "docs/designs/issue-116-supervisor.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/31"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/117"
    - "https://github.com/scarecrow173/ruviform/issues/113"
  defers: []
  deferred-by: []
---

# L1 supervisor 配置 — `ruviform-plugin-host` opt-in util 層と core `PluginHost` 無変更境界（Issue #116）

## Context

Issue #116 は `HostNotification::InstanceFailed` 後の機械的復旧（Destroy → 同 route re-instantiate → lifecycle replay → checkpoint restore）を担う supervisor 的運用面を必要とする。配置・所有権の確定は以下の契約に影響するため ADR として記録する:

- **AC4（「自動再起動はしない」）**: Issue #31 の確定判断（`docs/designs/plugin-isolation.md` L66・`docs/adr/plugin-isolation-strategy.md`）— `Failed` + 通知を host が行い、再起動は caller 判断。「自動再起動なし」は `l1_proxy_e2e.rs` `kill_transitions_failed_and_notifies` で pin 済みの公開契約であり、**supervisor 導入がこの語彙を曖昧にしてはならない**
- **状態復元はプラグイン責務**: caller-pushed checkpoint（`SaveState` → `RestoreState`）の責務帰属 — supervisor は機械的復旧の orchestration を担うが、state の所有・save 頻度は caller のまま
- **`PluginHost` は `!Send`/`!Sync`**: `composite.rs` L304-308 — supervisor スレッドが host を所有・駆動する設計は型レベルで不可能。配置候補は「host と同じスレッドの非 RT 制御ループ上」に限定される
- **bundle 後の `Failed` 粒度（#117 ADR `docs/adr/issue-117-bundle-child-failed-granularity.md` が正本）**: 子死亡は同居 N インスタンスへ同一 `ChildExit` で fan-out — supervisor が「1 通知 = 1 独立回復」と扱う粒度前提
- **`PluginHost` に handle→plugin_id 逆引き面が無い**: 復旧には「失敗した handle がどの `plugin_id` を再 instantiate するか」の知識が要る — 逆引きを core に足すか util 側が recipe 簿記を持つかは配置決定の一部
- **`try_recv_notification` は caller drain 型**: 通知を複数 consumer で分け合う経路が無い — supervisor を使うと drain 点の所有権が問題になる

upstream の discovery（`docs/discovery/issue-116-supervisor.md`）・spec（`docs/specs/issue-116-supervisor.md`）が「supervisor 面は consumer 固有ではなく `ruviform-plugin-host` 側に opt-in util として置く」方向を推奨しているが、それは core 無変更・公開 API のみ使用の util 層としての推奨であり、実際の配置（crate 内モジュール vs 別クレート vs consumer 側パターンのみ）は本 ADR で確定する。

## Decision

### 1. 配置 — `ruviform-plugin-host` 内 `supervisor` モジュール（opt-in util 層）

`crates/ruviform-plugin-host/src/supervisor.rs` に `Supervisor`/`SupervisedId`/`RestartPolicy`/`RestartRecipe`/`SupervisionEvent`/`SupervisionError`/`RestoreOutcome` を実装し、lib.rs で `pub use` 再エクスポートする。**`PluginHost` を `&mut` で借用して `poll(&mut host, now: Instant)` で駆動する** — host を所有しない・内部スレッド/タイマー/async runtime を持たない。全操作は非 RT。

- **公開 API のみ使用**: `execute`・`try_recv_notification`・`lifecycle_state`・`Scan`（`execute` 経由）で完結する。`pub(crate)` L1 machinery・`composite` 内部型（`Route`/`Slot`/pool）は参照しない — 同 crate 内のためコンパイラは private 面を遮らないが、「supervisor のテストは全て公開 API 経由で書く」ことを設計契約として強制する（`l1-test-worker` + `#[path]` harness はそのまま利用可能）
- **依存追加なし** — `std::time::{Instant, Duration}`・既存公開型のみ
- **1 `Supervisor` = 1 `PluginHost`** — `poll` が `try_recv_notification` の唯一の drain 点になり、監視外の通知は `SupervisionEvent::Unsupervised` で pass-through（caller の手動復旧責務は維持）

### 2. core `PluginHost` は無変更 — 逆引き・recipe 簿記・restart 動詞を core に足さない

- `PluginHost` の型・`execute`/`try_recv_notification`/`instantiate`/`destroy`/`mint`/`routes`/`instances`/`generations`/`pool` の意味は一切変更しない
- **handle→plugin_id 逆引きを core に追加しない** — supervisor 側の `watch(host, handle, recipe)` で caller が `RestartRecipe.plugin_id` を宣言し、util 帳簿が `SupervisedId` キーで recipe・checkpoint・attempt 状態を保持する。core に逆引き面を足すと「どの route の spec で再 spawn するか」という復旧固有の知識が core の責務に混入する — AC4 境界（core は `Failed` + 通知まで、復旧知識は caller/復旧層側）を semantic レベルで維持する
- **restart は opt-in**: `PluginHost` を素で使う呼出し側は自動再起動されない — `Supervisor::new` + `watch` + `poll` を選んだ consumer のみが復旧を受ける。`InstanceFailed` の emit 意味・`Failed` の terminal 性・`Destroy` 合法遷移は現行どおり

### 3. 状態機械・policy の語彙

- per-supervised entry: `Live → Pending → (Live | GaveUp)` — `InstanceFailed` drain 時点で `Destroy` を発行し slot を返し、attempt は `RestartPolicy{max_attempts, base_delay, max_delay, stable_window, max_attempts_per_poll}` で bound する
- **`GaveUp` は `HostError::Failed` に畳まない** — `Failed` は「子死亡で live インスタンスが terminal」の意味を維持し、supervisor が「episode の試行上限に達した」は `SupervisionEvent::GaveUp` の別語彙で報告する
- **bundle fan-out は per-instance 独立回復** — N 通知を「N 子 spawn」と解釈・強制しない（pool が fresh 子へ収束させる — #117 ADR の粒度契約と一貫）

詳細の state machine・失敗分類・`poll` アルゴリズムは design `docs/designs/issue-116-supervisor.md` が正本。

## Considered Options

- **Option A — `ruviform-plugin-host` 内 opt-in util 層（採用）**: 上記 Decision。consumer 固有ではなく「L1 out-of-process の運用面」として host クレートに置く — `PluginHost` の公開契約（通知 drain・`Destroy`・`Instantiate`・lifecycle replay・`RestoreState`）を組合せた復旧は host の利用パターンであり、複数 consumer（engine・test・tooling）が再利用できる。`poll(&mut host, Instant)` は `!Send`/`!Sync` 制約に構造的に適合する
- **Option B — core `PluginHost` へ restart 語彙を組込む**（`Instantiate` 内での自動再 instantiate・`HostCommand::Restart` 追加等）: AC4 の「自動再起動なし」を core 意味で曖昧にし、`Failed` terminal 契約（`Destroy` 以外脱出なし）を壊す。handle→plugin_id 逆引き・route spec 簿記・attempt 状態が core の責務に混入 → 却下
- **Option C — consumer 側パターンのみ（reusable util を提供しない）**: engine が自分で `try_recv_notification` drain → `Destroy` → `Instantiate` → replay → `RestoreState` を組む。AC4 は最も純粋に守られるが、(a) 失敗分類・bundle fan-out・backoff・checkpoint 帳簿を全 consumer が重複実装する、(b) `plugin_id` recipe・`SupervisedId` 簿記が consumer 固有の複雑さとして残る、(c) 「supervisor を使う」という意図が API 面に現れないため AC4 の「自動再起動なし」契約が consumer コードに散り追跡しにくい → util 層として集約する Option A を採用
- **Option D — 別クレート（`ruviform-plugin-supervisor` 等）**: `PluginHost` の公開 API だけで実装可能なため分離は技術的に可能だが、(a) 1 host に密結合した util を別クレートにすると version/依存の境界が増えるだけで隔離利得が無い（`PluginHost` を所有しないため Send/Sync 境界の利点も無い）、(b) 0.1.0 未公開で API 面の確定途上 — 別クレート化は成熟後の additive 拡張として残す → 却下（今回は crate 内モジュール）
- **Option E — core に handle→plugin_id 逆引き面を追加して util が利用**: Option A の util 側 recipe 簿記で同等の知識が実現できるのに core の公開契約を広げる → 却下（AC4 境界の semantic 維持）

## Consequences

- **公開 API は additive**: `Supervisor`/`SupervisedId`/`RestartPolicy`/`RestartRecipe`/`SupervisionEvent`/`SupervisionError`/`RestoreOutcome` の `pub use` 追加のみ — 既存の `PluginHost`/`HostCommand`/`HostReply`/`HostError`/`HostNotification`/`ChildExit`/`OutOfProcessSpec`/`OutOfProcessIsolation` の語彙・意味は不変。`#[non_exhaustive]` enum 方針を継続
- **AC4 は opt-in で維持される**: `PluginHost` を supervisor なしで使う経路は自動再起動されない — 「自動再起動なし」契約は「core が自律的に再起動しない」という semantic のまま（supervisor は caller が明示的に `watch`+`poll` するまで動かない）。`l1_proxy_e2e` の既存 pin は supervisor 非導入経路を変わらず検証する
- **`poll` が唯一の drain 点**: supervisor を使う consumer は `try_recv_notification` を直接呼ばない — `Unsupervised` pass-through で責務は移譲されない。supervised/unsupervised を混在させたい consumer は supervisor に全 drain を委ね `Unsupervised` を自分で捌く
- **handle→plugin_id は caller 宣言**: `watch` 時に recipe を必須にすることで「`plugin_id` を知らないと復旧できない」ことを API で強制する — `watch` 検証（live handle・`Scan` 登録 descriptor・重複 handle）が `SupervisionError` で defined 拒否される
- **`GaveUp` 後の entry は caller `unwatch` まで保持** — 自動除去しない（caller が残存 instance の salvage/通知を判断）。`GaveUp` は `HostError::Failed` と別語彙で consumer が区別できる
- **bundle crash の burst は `max_attempts_per_poll` で bound** — N 通知を 1 poll で全 drain し attempt は cap を超えて繰越す（spawn 集中の bound — 回復は最終的に収束する）
- **依存追加なし・RT 語彙に侵入しない**: `std::time::{Instant, Duration}` のみ・supervisor 関連の全操作は非 RT（`process`/`PluginEndpoint` に supervisor 語彙を持ち込まない — RT leg の契約は `plugin-rt-ownership-split` ADR のまま）
- **host 束縛の強制（1 Supervisor = 1 PluginHost）**: `PluginHandle{slot,generation}` は host 跨ぎで衝突し得るため、`Supervisor` は初回の host 受取 call（`watch`/`set_recipe`/`poll`）で `host as *const PluginHost as usize` を束縛する — `usize` 保持で `Supervisor` の `Send`/`Sync` 適合を維持（生 pointer は `!Send`/`!Sync`）。以後の異 host call は defined 拒否 — `watch`/`set_recipe` は `Err(SupervisionError::WrongHost)`、`poll` は `SupervisionEvent::WrongHost` を emit して drain/attempt を行わない（spec pin の `poll -> Vec<SupervisionEvent>` により rejection は event 経路。`debug_assert` だけにしない defined 拒否を採用 — 誤帰属・誤 host への `Destroy`/`Instantiate` を防ぐ）。束縛は address identity — host 値の move は別 host と見なされる（安全側の拒否）
- **checkpoint blob は bound 拒否**: `set_checkpoint` は `bytes.len() + envelope > wire::WIRE_MAX_PAYLOAD` を `Err(CheckpointTooLarge)` で拒否する — `wire::encode` が `WIRE_MAX_*` を encode 側で検査しない設計上、oversized `RestoreState` は runner の `read_frame`/`decode` を落として child-scope `Failed`（同居全インスタンス + fan-out）を引き、attempt 毎に繰返される kill になる。caller 直接の `RestoreState`・`SaveState` reply 側には同型の pre-existing core gap が残る — 別 Issue 級 follow-up 候補として design の Deferred 表に記録
- **別クレート化・route 単位 watch・per-entry policy・`rebind` API 等の拡張は additive に残る** — design の Deferred 表に記録

## Verification

- [ ] `supervisor` モジュールが `PluginHost` 公開 API のみで動作し、`pub(crate)` L1 machinery/`composite` 内部型に触れないこと（実装 review + 全テストが公開面経由で書かれることで実証 — design R-V1/AC1）
- [ ] `PluginHost` を supervisor 非導入で使う経路で `InstanceFailed` → `Failed` terminal → `Destroy` の既存契約が無退行（`l1_proxy_e2e` `kill_transitions_failed_and_notifies` 等 — design R-V5）
- [ ] `Supervisor::new` + `watch` + `poll` を選んだ consumer のみ復旧が走ること — 素の `PluginHost` で通知を drain しても `Restarted` は発行されない
- [ ] `watch` 検証が defined 拒否を返すこと（stale handle → `InvalidHandle`・未登録 `plugin_id` → `UnknownPluginId`・重複 handle → `AlreadyWatched`・異 host → `WrongHost`）および `Failed` handle が adopt（`Destroy` → `Pending` → 次 poll で復旧）されること
- [ ] `poll` が `try_recv_notification` の唯一の drain 点として動き、unwatched handle の通知が `Unsupervised` で pass-through されること — 束縛済みと異なる host では `WrongHost` event を emit して drain/attempt を行わないこと
- [ ] `set_checkpoint` が bound 超過 blob を `Err(CheckpointTooLarge)` で拒否し、oversized `RestoreState` が wire に乗らないこと
- [ ] `GaveUp` event が `HostError::Failed` と別語彙で発行され、mint 済み instance があれば unwind + `Destroy` 済みであること
