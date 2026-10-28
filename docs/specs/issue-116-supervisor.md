---
id: "SPEC-034NMZAKKGWufQ5vWwZNYe"
type: "spec"
status: "implemented"
title: "Issue #116 L1 自動再起動・supervisor ポリシー"
created: "2026-10-28"
updated: "2026-10-29"
owners: ["scarecrow173"]
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
  implements: []
  implemented-by:
    - "../designs/issue-116-supervisor.md"
    - "../plans/implement-issue-116-supervisor.md"
  depends-on:
    - "issue-117-bundle-child-process.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "plugin-l1-out-of-process.md"
    - "issue-117-bundle-child-process.md"
    - "issue-114-real-adapter-l1-e2e.md"
    - "../designs/plugin-isolation.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "../designs/issue-117-bundle-child-process.md"
    - "../designs/engine-plugin-host-integration.md"
    - "../discovery/issue-116-supervisor.md"
    - "../discovery/issue-117-bundle-child-process-discovery.md"
    - "../adr/plugin-isolation-strategy.md"
    - "../adr/issue-117-bundle-child-failed-granularity.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/issue-116-supervisor.md"
    - "plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "../designs/plugin-isolation.md"
    - "../adr/plugin-isolation-strategy.md"
    - "issue-117-bundle-child-process.md"
    - "../designs/issue-117-bundle-child-process.md"
    - "../adr/issue-117-bundle-child-failed-granularity.md"
  derived-by:
    - "../designs/issue-116-supervisor.md"
    - "../adr/issue-116-supervisor-placement.md"
    - "../plans/implement-issue-116-supervisor.md"
  verifies: []
  verified-by:
    - "../test-specs/issue-116-supervisor-core-tests.md"
    - "../test-specs/issue-116-supervisor-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/115"
    - "https://github.com/scarecrow173/ruviform/issues/119"
    - "https://github.com/scarecrow173/ruviform/issues/120"
  defers: []
  deferred-by:
    - "plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
---

# Issue #116 L1 自動再起動・supervisor ポリシー

> **IMPLEMENTED** — 本書は discovery（[issue-116-supervisor](../discovery/issue-116-supervisor.md)）を下流として記録した設計前仕様である。decision matrix の全 OPEN 行は design `../designs/issue-116-supervisor.md`（API sketch・遷移表・失敗分類表）と ADR `../adr/issue-116-supervisor-placement.md` で確定済み — 本書が確定させるのは Issue が要求する振る舞い・不変条件・受入基準であり、内部構造の確定ではない。実装記録: `docs/impl/ir/issue-116-supervisor-implementation.md`（AC1–AC10 + R-V1–R-V6 証拠写像を収録）。

## What Should Be Built

Issue #31（プラグイン分離設計）が確定した「再起動判断と状態復元は呼出し側/プラグイン責務（`docs/designs/plugin-isolation.md` — 自動再起動はしない）」を維持しつつ、**L1 の `HostNotification::InstanceFailed` 後にインスタンスを機械的に復旧する supervisor 的運用面**を提供する。Issue の選択肢 — 「host 側 util 層」or「利用側パターン」— のうち、**`ruviform-plugin-host` 内の opt-in util 層（`supervisor` モジュール）**を推奨する（decision matrix 軸 1）。

提供するもの:

1. **`InstanceFailed` 通知消費 → 機械的復旧シーケンス** — 通知 drain → `Destroy{failed}` → `Instantiate{plugin_id}` → lifecycle recipe 再生（`Initialize` → `RestoreState` → `Activate{config}` → `StartProcessing`）→ 新 `PluginHandle` の返却
2. **チェックポイント機構** — caller が pre-failure に `SaveState` で取った blob を supervisor に登録し、復旧時に `RestoreState` へ適用
3. **bounded retry policy** — `max_attempts` + exponential backoff（cap 付き）を論理インスタンス単位で管理、上限到達は give up（`Err(Failed)` には畳まない別語彙）
4. **event 報告** — 復旧結果（`Restarted{old,new}`/`GaveUp` 等、`#[non_exhaustive]`）を caller に返し、caller が新 handle で `prepare_endpoint` 再 attach・管理写像更新を行う

公開面・RT 契約・`Failed`+通知+pass-through・AC4「自動再起動なし」等の L1 基底契約は一切変更しない — `PluginHost` core は自律的に再起動せず、caller が util を明示的に呼んだ時のみ復旧が走る。

## Why Now

- Issue [#116](https://github.com/scarecrow173/ruviform/issues/116) — `docs/designs/plugin-l1-out-of-process.md` L301 deferred 行（「自動再起動・supervisor ポリシー | 状態復元はプラグイン責務の #31 確定判断 | supervisor 型運用要求 | 手動復旧の運用負荷」）の re-engagement。同 deferred 行が「#31 確定判断を前提に supervisor 型運用要求がある」と明示的に繋いでいる — 本 Issue はその deferred を解くフェーズ
- Issue #117（bundle 子プロセス）が implemented — `Bundled` 既定により 1 子死亡が N インスタンスに fan-out するようになった。supervisor 設計は **#117 後の失敗粒度**（子スコープ terminal → per-instance `Failed` + 同一 `ChildExit` fan-out）を前提にしなければならず、#117 前の「1 死 = 1 インスタンス」前提の設計は現行と不整合になるため本 Issue の確定が必要
- 手動復旧の運用負荷 — L1 を選ぶ利用者は「プロセスクラッシュからの回復」を求めるが、現行は caller が通知 drain・`Destroy`・`Instantiate`・lifecycle 再生・`RestoreState` を全て手書きしなければならない（`l1_proxy_e2e.rs` の E2E がこの手順を毎回明示している）

## Users and Value

- **ホストアプリケーション / エンジン実装者** — L1 インスタンスのクラッシュ復旧を util が機械的に実行し、caller は `Restarted{new}` イベントで endpoint 再配線と管理表更新だけを行えばよい。`Bundled` 既定の fan-out で複数インスタンスが同時に `Failed` になっても、util が per-instance 回復として扱い各影響インスタンスに 1 回ずつ復旧を適用する（pool の新規子に載せて一括で戻す）
- **状態復元を持つ plugin 利用者** — pre-failure checkpoint を supervisor に渡しておけば、復旧時に `RestoreState` で前の状態へ戻る（state 非対応 plugin は `Err(Unsupported)` をスキップして動く — OPEN 論点）
- **observer（`HostNotification` 消費者）** — supervisor を使わない consumer は現行どおり `InstanceFailed` + `Destroy` + 手動復旧を書ける（util は opt-in・core 不変）

## Scope

### In Scope

- **`ruviform-plugin-host` 内の supervisor util 層**（新規モジュール — `supervisor.rs` 相当、`PluginHost` の公開 API のみ使用、crate 内の `pub(crate)` L1 machinery には触れない）:
  - `Supervisor`（仮称）型 — `PluginHost` を `&mut` で借りて poll 駆動の非同期なし state machine。内部簿記は「監視対象論理インスタンス → plugin_id・recipe・checkpoint blob・attempt カウンタ」の map（`PluginHandle` ではなく論理キー — respawn で handle が変わるため）
  - `RestartPolicy`（仮称）— `max_attempts` + `base_delay` + `max_delay`（exponential backoff・attempt カウンタは respawn 越しに継承・成功後一定期間生存で reset 可）
  - `RestartRecipe`（仮称）— `plugin_id`（caller 指定 or handle 逆引き map）+ `Activate{config}` 用 `ProcessConfig`（Option — `Initialized` のみで止めてもよい）+ `StartProcessing` 目標フラグ + `EndpointBounds`（endpoint 再 attach 用・Option）
  - `SupervisionEvent`（`#[non_exhaustive]` enum）— `Restarted { supervised, old: PluginHandle, new: PluginHandle }` / `GaveUp { supervised, attempts: u32 }` / その他（`BackingOff`/`Unwatched`/`Pending` 等 — 語彙は design 確定）
  - `watch(handle, recipe)` / `unwatch(handle)` / `set_recipe`/`update_recipe` / `checkpoint(handle, bytes)` or `save_checkpoint(&mut host, handle)` / `poll(&mut host, now: Instant) -> Vec<SupervisionEvent>` — caller の非 RT 制御ループで呼ぶ
- **復旧シーケンスの semantic**:
  - 通知 drain 順に per-instance 処理 — bundle fan-out（同一 `ChildExit` の N 通知）は N 個の独立した回復イベントとして扱い、各 `Failed` handle に対して `Destroy` → `Instantiate` を 1 回ずつ実行（子数は pool 判定に委ねる — `Bundled` route なら N インスタンスが 1 fresh 子に載り得る）
  - `Destroy` 前に通知を drain 済みであること（通知 drain → 判定 → `Destroy` の順序 — `Destroy` が先だと slot 解放で通知が落ちる、composite.rs L200-204）
  - `Instantiate` の `Err`（`Ipc`/`InstanceLimit`）は attempt 失敗としてカウント（handle 未 mint — notification ではなく `Err` で返る現行契約に従う）
  - lifecycle 再生中の失敗（`Initialize`/`Activate`/`StartProcessing`/`RestoreState` の `Err`）は attempt 失敗 — mint 済み handle があれば `Destroy` で後片付け（残留リソースなし）
  - `RestoreState` の `Err(Unsupported)` は state 非対応としてスキップ（attempt 消費しない）が推奨 — `Err(PluginFailed)`/`Err(Ipc)`/`Err(Failed)` は attempt 失敗（OPEN — design 確定対象）
  - backoff 経過判定は `poll` 引数の `Instant` — スレッド・タイマーは作らない（`!Send`/`!Sync` 制約）
  - give up 時は supervised entry を caller 側へ `GaveUp` 報告し、既に mint 済みのインスタンスがあれば `Destroy` して slot を解放 — 「supervisor が諦めた」状態を `HostError::Failed` に畳まない（`Failed` は「子死亡で live インスタンスが terminal」语义を維持）
- **checkpoint 所有**: caller が `SaveState` を実行し blob を supervisor に渡す（push 型）— supervisor の `poll` 内で定期 `SaveState` を駆動しない（`SaveState` は non-RT・allocate 可・plugin hook 呼出 — poll 駆動は caller のドメイン最適化を奪う）
- **endpoint/RT 再配線**: `Restarted` event 発行時点で新インスタンスは endpoint 未 attach — caller が `prepare_endpoint`（`RestartRecipe` の `EndpointBounds` があればそれで）を呼び RT 所有者へ差し替える（supervisor 自体は `prepare_endpoint` しない — RT 所有者が別スレッドのため）
- **検証**: crate テスト（`l1-test-worker` + `#[path]` harness）で bundle crash fan-out・mid-create 死・restore 拒否・backoff・give up・`#[doc(hidden)] kill_child` 経由の crash 注入を検証 — supervisor は `PluginHost` の公開面だけを使うのでテストは public API 経由で書ける

### Out of Scope

- **`PluginHost` core の変更** — `execute`/`try_recv_notification`/`instantiate`/`destroy`/`mint`/`slot_mut`/`routes`/`instances`/`generations`/`pool` の语义・型・フィールドは一切変更しない（AC4 維持）。`PluginHost` 側に handle→plugin_id 逆引き面を追加する案は OPEN — 本 spec では「util 側帳簿」を推奨（core 汚染を避ける）
- **wire / `HostNotification` / `HostError` / `ChildExit` の変更** — supervisor は公開面のみで動き、protocol・通知语义・エラー語彙は不変（`#[non_exhaustive]` で後追加可能な `SupervisionEvent` は util 側の新規 enum）
- **自動再起動** — core は自律的に再起動しない。supervisor util は caller が明示的にインスタンスを `watch` した場合のみ復旧を試みる（opt-in 単位はインスタンス — route 単位でもよいかは design 確定対象だが最小は per-instance）
- **他 route / 別 `plugin_id` への migrate** — `Instantiate{plugin_id}` の登録時 claim モデルと矛盾するため範囲外。失敗 route の isolation（Bundled/Dedicated）は登録時指定のまま継承
- **GUI（`OpenGui`）** — L1 GUI は `Err(Unsupported)` のまま（design L262・L300）— supervisor は GUI を扱わない
- **RT 処理経路の変更** — `Failed` 時の pass-through/無音・`PluginHost::process`/`PluginEndpoint` 语义は不変。復旧中の RT 側は silence のまま（caller が endpoint 再 attach するまで `process` の意味は変わらない）
- **状態復元の自動化** — `RestoreState` の発行は supervisor が行うが、blob の取得は caller 主導（事前 `SaveState` → `checkpoint` 登録）— 「supervisor が poll で自動 `SaveState`」は入れない
- **give up 後の自動 unwatch** — `GaveUp` 報告後の supervised entry を caller が `unwatch` するか、supervisor が自動除去するかは design 確定対象（caller 責務として残す方が最小）
- **複数 `PluginHost` への同時 supervise** — 1 `Supervisor` = 1 `PluginHost`（`poll(&mut host)` で host を借りる形なので複数 host は複数 `Supervisor` か逐次呼出し）

## Requirements

### 基底契約（維持）

- `PluginHost` core（型・`execute`/`try_recv_notification`/`instantiate`/`destroy`/`mint`/`prepare_endpoint`/`lifecycle_state`/`latency_frames`/`kill_child`）は無変更 — AC4「自動再起動なし」不変（`l1_proxy_e2e.rs` `kill_transitions_failed_and_notifies` で pin 済み）
- `Failed` 语义不変 — `Failed` インスタンスは `Destroy` 以外 `Err(HostError::Failed)`、`Failed → Destroyed` のみ進行（`converge_child`/`converge_instance`/`remove_instance`/`Notify` 遷移・slot/generation 規則全て現行どおり）
- `HostNotification` 语义不変 — per-instance 通知・exactly once・`ChildExit{None,None}` 許容・`Destroy` で slot と共に通知消失・`Creating` 中の死亡は通知なし（`Err(Ipc)` で `Instantiate` 返却）
- `PluginHost` は `!Send`/`!Sync` のまま — supervisor は同一スレッドの非 RT 制御ループで駆動（新スレッド・タイマー・async runtime 不使用）
- RT 契約 — supervisor 関連の全操作は非 RT。`PluginHost::process`/`PluginEndpoint::process` 等の RT 経路に supervisor 語彙は入らない（復旧中の RT 側は現行どおり pass-through/無音）
- `#[non_exhaustive]` 公開 enum 方針 — 新規公開型（`SupervisionEvent` 等）は `#[non_exhaustive]`、既存 `HostNotification`/`HostError` は変更なし
- 依存追加なし — `std::time::Instant`/`Duration` と既存公開 API のみ使用（`ruviform-rt`/`shared_memory` 追加不要）

### supervisor util 面（OPEN を含む）

- **R-S1**: `ruviform-plugin-host` に supervisor util モジュール（`supervisor.rs` 相当）を新設 — `PluginHost` の公開 API（`execute`/`try_recv_notification`/`lifecycle_state`/`latency_frames`/`prepare_endpoint`/`register_*`）のみを使い、`PluginHost` を `&mut` で借用して駆動する。crate の層ルール（拡張層 → `ruviform-plugin-core`/`ruviform-rt`/`shared_memory`（内部 `shm` 限定））を破らない — 新規依存は追加しない
- **R-S2**: 監視登録 API — `watch(handle, recipe)` で論理インスタンス（`PluginHandle` ではない supervised キー）を登録し、`unwatch`/`set_recipe`/`checkpoint` で更新する形を推奨。`watch` に recipe（`plugin_id`・Activate 用 `ProcessConfig`・`StartProcessing` 目標・`EndpointBounds`・初期 checkpoint blob）を渡す形が最小。**OPEN — handle→plugin_id 逆引きを core に持たせるか util 帳簿に留めるか**（後者を推奨 — core 無変更）
- **R-S3**: 駆動形 — `poll(&mut host, now: Instant) -> Vec<SupervisionEvent>` で caller の非 RT ループから呼ぶ。内部タイマー・スレッド・async は入れない（`!Send`/`!Sync`・non-RT 制約）。backoff の経過は `Instant` 比較 — poll 呼出しごとに attempt deadline を評価する
- **R-S4**: 通知処理 — `poll` 内で `try_recv_notification` を drain し、`InstanceFailed{handle, exit}` ごとに supervised entry を引いて回復対象にする。drain 順・通知 handle 昇順等の順序規則は design 確定対象。supervised 外の handle は無視（caller が自分で `Destroy`/復旧する責務は残る — supervisor は「全失敗を拾う」のではなく「監視対象のみ復旧する」）
- **R-S5**: 復旧シーケンス — supervised handle の `InstanceFailed` に対して: (1) `Destroy{failed}`（`Ok` 保証・slot 解放）、(2) `Instantiate{plugin_id}`（同 route — route の Bundled/Dedicated を継承）、(3) `Initialize`、(4) checkpoint blob があれば `RestoreState{bytes}`、(5) recipe に `config` があれば `Activate{config}`、(6) recipe が `StartProcessing` 目標なら `StartProcessing`。成功で `Restarted{old,new}` event 発行 + attempt カウンタ進行（成功 reset 条件は design）
- **R-S6**: 失敗分類 — (a) `Instantiate` の `Err(Ipc)`/`Err(InstanceLimit)` → attempt 失敗（handle 未 mint — notification ではなく `Err` で返る）；(b) lifecycle 再生中の `Err(Lifecycle)`/`Err(Prepare)`/`Err(Ipc)` → attempt 失敗 + mint 済み handle があれば `Destroy` で後片付け；(c) `RestoreState` の `Err(Unsupported)` → state 非対応としてスキップ（attempt 消費しない・推奨）または attempt 失敗（OPEN — design 確定対象）；(d) `RestoreState` の `Err(PluginFailed)`/`Err(Ipc)`/`Err(Failed)` → attempt 失敗 + `Destroy` 後片付け
- **R-S7**: backoff — exponential backoff（base → cap）で `max_attempts` 超過時は attempt 停止 + `GaveUp` event。attempt カウンタは論理インスタンス単位（respawn で generation が変わっても継承）。成功後の reset 条件（一定期間生存でクリア）は design 確定対象。`Instant` ベースなのでテストで `now` を注入可能
- **R-S8**: give up 语义 — `max_attempts` 到達で `SupervisionEvent::GaveUp` を発行し、既に mint 済みのインスタンスが残っていれば `Destroy` して slot を解放。`HostError::Failed` に畳まない — 「supervisor が諦めた」は `GaveUp` 語彙で区別（`Failed` は「子死亡で live インスタンスが terminal」语义のまま）
- **R-S9**: event — `SupervisionEvent` は `#[non_exhaustive]` enum、`Restarted` は `{supervised_key, old: PluginHandle, new: PluginHandle}` を運ぶ — caller が管理写像の差し替えと `prepare_endpoint` 再 attach（`RestartRecipe` の `EndpointBounds`）を行うトリガ。`Restarted` 時点で新インスタンスは endpoint 未 attach（caller 責務）
- **R-S10**: bundle fan-out — 同一 `ChildExit` の N 通知は N 個の独立回復イベントとして扱い、各 supervised handle に対して `Destroy` → `Instantiate` を 1 回ずつ行う。子プロセス数は pool 判定に委ねる（`Bundled` route の fresh 子に N インスタンスが載る — supervisor は子数を制御しない）。同時に supervised 外の handle が混ざっても supervised のみ処理する

### 検証

- **R-V1**: `l1-test-worker` + `#[path]` harness（`l1_proxy_e2e.rs` の既存構造）で bundle crash fan-out（1 `kill_child` → N `InstanceFailed` → supervisor poll → N `Restarted`）を検証
- **R-V2**: mid-`CreateInstance` 死亡（`bundled_mid_create_death_notifies_only_minted` 相当 — supervisor の attempt 中に子死亡 → `Err(Ipc)` で attempt 失敗・通知なし）を検証
- **R-V3**: `RestoreState` 拒否（`Err(PluginFailed)` — 不正 bytes または `Err(Unsupported)` — state 非対応 plugin）を検証 — 確定した失敗分類ルールどおりの挙動
- **R-V4**: backoff — `Instant` 注入で attempt deadline を跨いだ再試行・`max_attempts` 超過で `GaveUp`・attempt カウンタ継承・成功 reset を検証
- **R-V5**: AC4 無退行 — supervisor 未使用時の `l1_proxy_e2e` 系全テストが変わらず通ること（core 無変更の証明）
- **R-V6**: endpoint 再配線 — `Restarted` で返った新 handle への `prepare_endpoint` が動くこと（`EndpointBounds` を recipe に含める場合の検証）

## Decision Matrix

> discovery の軸別比較を spec 側の確定/OPEN 表に写したもの。「確定」は本 spec が固定する方向、「OPEN」は design/ADR が確定する論点、「推奨」は design が覆し得る推奨方向。

| 論点 | 選択肢 | 本 spec での扱い | 理由・根拠 |
| --- | --- | --- | --- |
| supervisor 面の置き場所 | (a) `ruviform-plugin-host` 内 util 層（`supervisor.rs` 新規モジュール・`PluginHost` を `&mut` 借用） | **推奨 — 確定方向** | 機械的復旧手順を 1 箇所に集約・core 無変更で AC4 維持・公開 API のみ使用・crate テストで `l1-test-worker` fixture 流用可能。consumer パターン文書のみでは全 consumer が同一手順を再実装する構造的問題 |
| | (b) `PluginHost` composite メソッド（`restart_failed`/`poll_supervision`） | **却下** | core に再起動語彙・recipe 簿記を持ち込み AC4 境界を侵す。`PluginHost` が handle→plugin_id 逆引きを内部保有する必要が生じる |
| | (c) consumer パターン文書のみ | **却下** | 全 consumer が同じドメインルールを再実装 — 構造的問題の兆候そのもの。attempt/backoff/exactly-once の落とし穴を各 caller が踏む |
| 再起動スコープ | (a) 同一 `plugin_id`（同 route）のみ | **確定** | `Instantiate{plugin_id}` の登録時 claim モデルと整合。失敗 route の Bundled/Dedicated が継承される |
| | (b) 別 `plugin_id` への migrate | **却下** | route 選択語彙の新設・`Instantiate` モデルとの矛盾・YAGNI |
| | (c) `ChildExit` 値で route 分岐 | **却下** | `ChildExit` は spawn 失敗でなく live 中の子死亡 status — route 属性情報を持たない |
| バックオフ形状 | (a) リトライなし | **却下** | supervisor の存在意義が消える |
| | (b) 1 回即時リトライ | OPEN — 最小有用の検討対象 | crash loop を区別できない・backoff 無しで毎 poll spawn 試行の可能性 |
| | (c) 固定回数 + 固定間隔 | OPEN | 一時的負荷に即応しすぎる |
| | (d) 最大 attempt + exponential backoff（cap） | **推奨** | crash loop を早く諦められる・一過性負荷に間を置く・`Instant` ベースでスレッド不要 |
| | (e) 無限リトライ | **却下** | 契約違反 — 起動不能 plugin の無限 spawn |
| | (f) circuit breaker → `Err(Failed)` fold | **却下** | 「supervisor が諦めた」と「子死亡で `Failed`（Destroy 可能）」を同 variant で表現すると语义曖昧化 — `GaveUp` 別語彙で |
| checkpoint 所有 | (a) caller が pre-failure `SaveState` → supervisor へ blob 引渡し（push） | **推奨 — 確定方向** | `Failed` 後 `SaveState` は `Err(Failed)` で不可能 — 事前 checkpoint 前提が不変条件。save 頻度のドメイン最適化は caller に残る |
| | (b) supervisor が `poll` で定期 `SaveState` 駆動（pull） | **却下（初版）** | `SaveState` は non-RT・allocate 可・plugin hook — poll 駆動は caller の最適化を奪う |
| | (c) `InstanceFailed` 受信時に `SaveState` 試行 | **却下** | `Failed` 後は `Err(Failed)` — 不可能 |
| recipe 追跡 | (a) `watch(handle, recipe)` で caller が宣言 | **推奨** | supervisor は機械的再生のみ — コマンド傍受不要。`set_recipe` で更新する規則を設ける |
| | (b) supervisor が `execute` を wrap して傍受追跡 | OPEN — 後続拡張 | `PluginHost` 全面 wrapper が要る — `&mut` borrow/`prepare_endpoint` で複雑化 |
| | (c) `PluginHost` が recipe を内部保有 | **却下** | core に再起動関連簿記 — AC4 境界を侵す |
| 駆動形 | (a) caller が `poll(&mut host, now)` を非 RT ループで呼ぶ | **確定** | `PluginHost` `!Send`/`!Sync` に適合・スレッド不要・`Instant` 注入で backoff テスト容易 |
| | (b) supervisor が内部スレッドを持つ | **却下** | `!Send`/`!Sync` で型レベルで不可能 |
| | (c) `try_recv_notification` 内に監視を組み込み | **却下** | core 語彙への混入 + 自動再起動そのもの — AC4 違反 |
| 復旧結果報告 | (a) `SupervisionEvent` enum を `poll` 戻り値で返す | **確定** | supervisor は機械的復旧に専念、endpoint/RT 配線は caller 責務を維持（RT 所有者は別スレッド — supervisor が直接触れない構造的必然） |
| | (b) supervisor が `prepare_endpoint` まで実行 | **却下** | `PluginEndpoint` は RT 所有者への move が前提 — supervisor が保持しても渡す経路が別途要る・bounds は caller ドメイン知識 |
| | (c) 旧 handle を維持（slot reuse で同 handle 復活） | **却下** | generation 規則で不可能 — `mint` は世代を bump する |
| `Err(Failed)` の supervisor への再利用 | (a) `GaveUp` を `HostError::Failed` に畳む | **却下** | 「supervisor が諦めた」と「子死亡で `Failed`（Destroy 可能な live インスタンス）」は別语义 — 混同すると `Failed` の意味が曖昧化 |
| | (b) `SupervisionEvent::GaveUp` で報告 | **確定** | supervisor の give up は別語彙 — `Failed` は「子死亡で live インスタンスが terminal」语义のまま維持 |
| bundle fan-out の扱い | (a) N 通知 = N 個の独立回復イベント（per-instance `Destroy`→`Instantiate`・子数は pool 判定） | **確定** | #117 の `Failed` 粒度契約（子スコープ terminal → per-instance `Failed` + 同一 `ChildExit` fan-out）に整合。supervisor が「N 通知 = N 子」と解釈して N 子 spawn を強制すると `Bundled` の意味を壊す |
| | (b) bundle crash を 1 イベントとして処理 | **却下** | per-instance 通知语义を壊す — `Bundled` でも各インスタンスは独立した handle/recipe/state を持つ |
| mid-`CreateInstance` 死（`Instantiate` 自体の失敗） | (a) attempt 失敗としてカウント（`Err(Ipc)` で返る） | **確定** | handle 未 mint — notification は出ない（現行契約）が attempt は消費すべき（無限 retry しないため） |
| | (b) 通知のみを回復トリガとする | **却下** | `Instantiate` 自体の失敗を attempt に数えないと無限 spawn retry になる |

## Acceptance Criteria

> 各 AC は「確定した選択肢が採用された場合に検証されるべき振る舞い」を記す。OPEN と印した論点は design/ADR 確定後に対応する検証へ落とす。

- [ ] **AC1（opt-in supervisor util 面）**: `ruviform-plugin-host` に `supervisor` モジュール相当が存在し、`PluginHost` の公開 API のみで動作する（`pub(crate)` L1 machinery・`composite.rs`/`l1/` 内部型に一切触れない）。`PluginHost` core の型・フィールド・`execute`/`try_recv_notification` 语义が変更されていないこと（AC4 維持の証明 — 既存 E2E が全て通る）
- [ ] **AC2（同 route 限定再 spawn）**: supervised handle の `InstanceFailed` 受信 → `Destroy{failed}` → `Instantiate{plugin_id}` が同 route で実行され、新 `PluginHandle` が mint される。route の isolation 指定（Bundled/Dedicated）が登録時のまま継承されること。他 route の `plugin_id` への instantiate は発生しないこと
- [ ] **AC3（bundle fan-out）**: `Bundled` route で `kill_child`（1 子死亡）→ N supervised handle の `InstanceFailed` が同一 `ChildExit` で到着 → supervisor `poll` で各 supervised handle に対して `Destroy` → `Instantiate` が 1 回ずつ実行され、全 supervised インスタンスが新 handle で復旧する（pool が fresh 子を再利用する形で子数は pool 判定 — supervisor が N 子を強制しない）。supervised 外の handle が混ざっても supervised のみが処理される
- [ ] **AC4（checkpoint/restore）**: pre-failure に `checkpoint` で登録した blob が `Restarted` 発行前に `RestoreState{bytes}` で新インスタンスへ適用される。`RestoreState` の `Err(Unsupported)`（state 非対応 plugin）は復旧を妨げないこと（確定した分類ルール — OPEN が design で確定した場合はその規則どおり）。checkpoint 未登録インスタンスは `RestoreState` を発行せず `Initialize` → `Activate` で復旧する
- [ ] **AC5（bounded retry）**: `max_attempts` 超過で attempt が停止し `GaveUp` event が発行される。backoff 中は `poll` が新たな attempt を発行しない（`Instant` 注入で deadline 評価を検証）。attempt カウンタは respawn で generation が変わっても論理インスタンス単位で継承される
- [ ] **AC6（give up 语义）**: `GaveUp` 発行時に既に mint 済みのインスタンスが残っていれば `Destroy` して slot を解放する。`HostError::Failed` を `GaveUp` の報告に使わないこと（`Failed` は「子死亡で live インスタンスが terminal」语义のまま — supervisor の give up は別語彙）
- [ ] **AC7（attempt 中の失敗）**: `Instantiate` の `Err(Ipc)`/`Err(InstanceLimit)`・lifecycle 再生中の `Err` → attempt 失敗としてカウントされ、mint 済み handle があれば `Destroy` で後片付けされ残留リソースがない。`RestoreState` の `Err(PluginFailed)`/`Err(Ipc)`/`Err(Failed)` も同様に attempt 失敗 + 後片付け
- [ ] **AC8（driver 契約）**: supervisor は `poll(&mut host, now)` で駆動され、内部スレッド・タイマー・async runtime を持たない（`PluginHost` `!Send`/`!Sync` 制約を満たす）。全ての復旧作業は caller の非 RT スレッド上で同期的に実行される
- [ ] **AC9（event 報告）**: `Restarted` event が `{supervised_key, old: PluginHandle, new: PluginHandle}` を返し、caller が新 handle で `prepare_endpoint`（`RestartRecipe` の `EndpointBounds`）・`latency_frames`・`lifecycle_state` を呼べること。`GaveUp`/`BackingOff`/`Unwatched` 等の event 語彙が `#[non_exhaustive]` enum で定義される
- [ ] **AC10（CI/監査）**: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス・`doc_status.js audit` blocking 0・findings なし

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| `PluginHost` 側に handle→plugin_id 逆引き面を追加するか（util 帳簿 vs core 簿記） | OPEN — util 帳簿を推奨（core 無変更で完結・`watch` 時に caller が `plugin_id` を指定する形） | design（公開 API 追加の是非 — 追加するなら `PluginHost::plugin_id(handle) -> Option<&str>` 等の最小面） | core に再起動関連簿記が混入し AC4 境界が曖昧化する or util 側の帳簿が冗長になる |
| `RestoreState` の `Err(Unsupported)` の分類（attempt 消費しない vs attempt 失敗） | OPEN — 非対応スキップ推奨 | design（`Err(Unsupported)` の语义は「state 機能がない」なので失敗として数えるのは厳しすぎる） | state 非対応 plugin が max_attempts 後に give up して動かなくなる or 失敗分類が複雑化 |
| backoff 確定値（`max_attempts`・base/max delay・reset 条件） | OPEN — `max_attempts` + exponential backoff（cap 付き）を推奨 | design（`Policy` 型の確定・値の根拠記録） | 値が暗黙のまま実装されて後で変えると公開 API 変更 or デフォルト挙動の破壊になる |
| `watch` 後の handle 変化追跡（caller が非 supervisor 経路で `Destroy`→`Instantiate` した場合） | OPEN — `unwatch`/`set_recipe` で caller が追従する規則を推奨 | design（supervised キーと `PluginHandle` の寿命関係の明文化） | supervisor が stale handle を見て誤動作する or caller の handle 更新が supervisor と不整合になる |
| `PluginEndpoint` の再 attach を recipe に含める範囲（`EndpointBounds` のみか、caller が後から上書きするか） | OPEN — `EndpointBounds` を recipe に含めるを推奨 | design（`RestartRecipe` の確定形） | endpoint 未 attach のまま復旧完了となり RT 処理が戻らない |
| 1 poll 内の attempt 上限（bundle fan-out の N 件同時処理数） | OPEN — drain 順で全件処理を推奨 | design（poll の処理規則確定） | bundle crash で N 件一括処理すると `Instantiate` の spawn コストが集中する可能性（ただし pool 再利用で実害は限定的） |
| `watch` 対象を route 単位で指定する形（同一 plugin_id の全インスタンスを supervise） | 追加機能として記録（必須ではない） | design（API の広がり） | 複数インスタンスを個別 `watch` する手間 — ただし per-instance で十分の可能性が高い |
| `SupervisionEvent` の `supervised_key` 型（`PluginHandle` か独自論理 ID か） | OPEN — `PluginHandle` を推奨（event 発行時点で caller が handle 写像を更新しやすい） | design（公開型確定） | 独自 ID を導入すると caller が 2 つの写像を持つことになる |
| mid-`DestroyInstance` 死亡で supervised handle の通知が落ちた場合の supervisor 挙動 | OPEN — 「supervised のみ追跡」原則で自然解決（通知が落ちても `Failed` 状態を `lifecycle_state` で観測可能） | design（通知消失時のフォールバック — `poll` が `lifecycle_state` を走査するかは YAGNI 論点） | 通知が `Destroy` 前に落ちたインスタンスを supervisor が見逃す可能性（頻度は低い） |

## Sources and Relations

- Issue [#116](https://github.com/scarecrow173/ruviform/issues/116)（本 spec の対象 Issue — supervisor 運用面の確定）・Issue [#31](https://github.com/scarecrow173/ruviform/issues/31)（プラグイン分離設計・CLOSED — 再起動判断/状態復元責務帰属の起点）・Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)（L1 実装・CLOSED — `Failed`+通知+AC4「自動再起動なし」基底契約）・Issue [#117](https://github.com/scarecrow173/ruviform/issues/117)（bundle 子プロセス・implemented — 失敗粒度・fan-out・pool・`WIRE_MAX_INSTANCES` を定義。depends-on）・Issue [#113](https://github.com/scarecrow173/ruviform/issues/113)（engine プラグイン統合 — `PluginEndpoint` detach/attach 所有権・復旧後の endpoint 再配線関連）・Issue [#114](https://github.com/scarecrow173/ruviform/issues/114)（実規格プラグイン L1 E2E — supervisor 検証で実 adapter fixture 活用の候補）・Issue [#115](https://github.com/scarecrow173/ruviform/issues/115)（L1 GUI — out of scope）・Issue [#119](https://github.com/scarecrow173/ruviform/issues/119)（wire バージョン方針 — supervisor は wire 変更なし）・Issue [#120](https://github.com/scarecrow173/ruviform/issues/120)（共有リーパー/io 多重化 — poll 型 supervisor は競合なし）
- [issue-116-supervisor discovery](../discovery/issue-116-supervisor.md) — 本 spec の discovery（コード観測事実・選択肢比較・推奨方向）
- [plugin-isolation design](../designs/plugin-isolation.md)・[plugin-isolation-strategy ADR](../adr/plugin-isolation-strategy.md) — #31 確定判断の正本（再起動は呼出し側判断・自動再起動はしない — 状態復元はプラグイン責務）
- [plugin-l1-out-of-process spec](plugin-l1-out-of-process.md)・[design](../designs/plugin-l1-out-of-process.md) — #76 系上流（`Failed`/通知/pass-through/非 RT/AC4・deferred 行 L301 の起点）
- [issue-117-bundle-child-process spec](issue-117-bundle-child-process.md)・[design](../designs/issue-117-bundle-child-process.md)・[ADR](../adr/issue-117-bundle-child-failed-granularity.md) — bundle 後の `Failed` 粒度（子スコープ terminal）・per-instance fan-out・pool・`WIRE_MAX_INSTANCES` — 本 spec の失敗粒度前提
- [engine-plugin-host-integration design](../designs/engine-plugin-host-integration.md) — `PluginEndpoint` の detach/attach 所有権（復旧後の endpoint 再配線が caller 責務である根拠）
