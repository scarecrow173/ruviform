---
id: "DISC-034NMZseZM94vtNrYx24rM"
type: "discovery"
status: "active"
title: "L1 自動再起動・supervisor ポリシーの探索（Issue #116）"
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
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/plugin-isolation-strategy.md"
    - "../designs/plugin-isolation.md"
    - "../specs/plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "issue-117-bundle-child-process-discovery.md"
    - "../specs/issue-117-bundle-child-process.md"
    - "../designs/issue-117-bundle-child-process.md"
    - "../adr/issue-117-bundle-child-failed-granularity.md"
    - "../designs/engine-plugin-host-integration.md"
    - "../impl/ir/issue-117-bundle-child-implementation.md"
    - "../test-specs/issue-117-bundle-e2e-tests.md"
    - "../test-specs/plugin-l1-proxy-contract-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/plugin-isolation.md"
    - "../adr/plugin-isolation-strategy.md"
    - "plugin-l1-out-of-process.md"
    - "../specs/plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "../designs/issue-117-bundle-child-process.md"
    - "../adr/issue-117-bundle-child-failed-granularity.md"
    - "../specs/issue-117-bundle-child-process.md"
  derived-by:
    - "../specs/issue-116-supervisor.md"
    - "../designs/issue-116-supervisor.md"
    - "../adr/issue-116-supervisor-placement.md"
    - "../plans/implement-issue-116-supervisor.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/115"
    - "https://github.com/scarecrow173/ruviform/issues/119"
    - "https://github.com/scarecrow173/ruviform/issues/120"
  defers: []
  deferred-by: []
---

# L1 自動再起動・supervisor ポリシーの探索（Issue #116）

## Exploration Goal

Issue [#116](https://github.com/scarecrow173/ruviform/issues/116) は Issue [#31](https://github.com/scarecrow173/ruviform/issues/31)（プラグイン分離設計 — L0 既定 / L1 opt-in）の確定判断を維持したまま、**supervisor 的運用を host 側 util 層 or 利用側パターンとして提供する面を確定する**ことを要求する。Issue スコープ（原文）:

- `HostNotification::InstanceFailed` 後の再 spawn 経路設計
- save/restore・バックオフ・リトライ上限の方針確定
- 現行「自動再起動なし」契約（AC4）との両立

トリガ:

- `docs/designs/plugin-l1-out-of-process.md` L301 deferred 行 — 「自動再起動・supervisor ポリシー | 状態復元はプラグイン責務の #31 確定判断 | supervisor 型運用要求 | 手動復旧の運用負荷」
- `docs/specs/plugin-l1-out-of-process.md` deferred 表の同項目 — 本 Issue はこの deferred 行の re-engagement
- `docs/designs/plugin-isolation.md`（#31 確定判断）— 「再起動は呼び出し側判断（自動再起動はしない — 状態復元はプラグイン責務）」が本 Issue の**動かせない前提**

本探索は **#31/AC4 の正確な契約境界・通知機構とハンドル意味論・現行再 spawn 経路・bundle（#117）後の fan-out 挙動・state フック契約・配置選択肢（host util 層 / composite メソッド / consumer パターン）・最小バックオフ形状**を記録し、spec/design フェーズへ渡す推奨方向を確定する。最終の公開 API 形状確定権限は design/ADR フェーズにある。

## Key Issues and Assumptions

### 上流で確定済み（confirmed — design 論点ではない）

- **#31 確定判断（再 restart 判断と状態復元の責務）**: `docs/designs/plugin-isolation.md` の境界設計 — 「クラッシュ時はインスタンスを `Failed` 状態へ遷移・ホストへ通知。**再起動は呼び出し側判断（自動再起動はしない — 状態復元はプラグイン責務）**」。supervisor は opt-in でなければならず、host core が自律的に再起動してはならない。
- **AC4（「自動再起動なし」契約）**: `docs/designs/plugin-l1-out-of-process.md` L315-319 — クラッシュ検出 → `Failed` 遷移 + 通知 + 以後定義済み拒否・自動再起動なし。実装側の pin は `l1_proxy_e2e.rs` `kill_transitions_failed_and_notifies`（L289-307 —「No automatic restart — repeated commands keep failing」`Err(HostError::Failed)`）。
- **`Failed` は terminal**: `docs/designs/plugin-l1-out-of-process.md` L257-267 — `Failed` インスタンスは `Destroy` 以外の全コマンドを `Err(HostError::Failed)` で拒否し、`Failed → Destroyed` のみ進行可能。`Destroy` は `Ok` でスロット解放。
- **通知契約**: `HostNotification::InstanceFailed { handle, exit }`（`#[non_exhaustive]`、composite.rs L212-224）— L0 インスタンスは通知しない、live 中は exactly once、`Destroy` で slot 解放されるとキュー残通知は slot と共に落ちる（composite.rs L200-204・L859-897）。
- **bundle fan-out（#117 implemented）**: 1 子プロセス = N インスタンス（最大 `wire::WIRE_MAX_INSTANCES` = 64）。子死亡は**子スコープ terminal**で bundle 内全 live インスタンスを `Failed` 収束させ、各 minted インスタンスへ同一 `ChildExit` を持つ `InstanceFailed` を 1 つずつ fan-out（`converge_child` l1/mod.rs L633-706・ADR `issue-117-bundle-child-failed-granularity`）。
- **`PluginHost` は `!Send`/`!Sync`**: composite.rs L304-308 — L0 の `Box<dyn ErasedPlugin>`（auto-traits なし）と L1 slot の `shared_memory::Shmem`（raw pointer）が理由。**supervisor スレッドを spawn して `PluginHost` を渡す設計は型レベルで不可能** — 単一スレッドの非 RT 制御ループに乗る形しか取れない。
- **RT 契約**: supervisor 関連の全操作（通知 drain・`Destroy`・`Instantiate`・lifecycle・`SaveState`/`RestoreState`）は非 RT（composite.rs L301-302・lib.rs L123-124）。RT 処理経路（`PluginHost::process` / `PluginEndpoint`）に supervisor 語彙は入らない — `Failed` 時の RT 側は pass-through/無音のまま。

### コード観測事実（confirmed — 再 spawn 経路の正確な形）

#### 通知配送機構（`composite.rs` + `l1/mod.rs`）

- `PluginHost::try_recv_notification`（composite.rs L872-897）: L1 slot 表を走査し、endpoint チャネル drain（`drain_endpoint_channels` — 返還された leg 復帰と arm チャネル再送/破棄）後に `L1Instance::try_recv_notification`（`notify_rx.try_recv`、l1/mod.rs L2889-2895）を 1 件ずつ返す。スロット index + 現行 generation から `PluginHandle` を再構成する（L886-889 —「The slot is still live … the current generation is the minted handle's」）。
- **通知は mint 済みインスタンス単位・exactly once**: `converge_child`（l1/mod.rs L657-684）が minted 判定した各 entry に `Notify::Idle → Sent/Pending` で 1 通知。exit 不明（EOF・プロトコル破壊・kill 前の timeout）時は `Pending` にし、driver reap 後に実 status で補完（L617-632・L639-648）。`remove_instance`（L583-608）は `DestroyInstance` reply 粒度の entry 除去時に `Pending` を `ChildExit{None,None}` で flush — **exactly-once は「実 status が届かなくても通知は出る」方向に保たれる**。
- **`ChildExit { code, signal }` の `None` 許容**: (a) per-instance 帰属可能違反（`converge_instance`、L720-769 — 子生存で exit status が存在しない）→ `ChildExit{None,None}`；(b) `Pending` flush（L599-606）→ `ChildExit{None,None}`；(c) `code`/`signal` の片側 `None`（unix signal kill で `code: None`、Windows で `signal: None` — composite.rs L180-188・E2E L2816/2919「`exit.code.is_some() || exit.signal.is_some()`」）。**supervisor は `ChildExit` 値で再起動可否を分岐できない** — `None` は「子が生きているが帰属違反」の場合も「status 未到着」の場合もあり得る。
- **`Creating` 中の死亡は通知なし**: `converge_child` L658-663 — `minted()` 前 entry は silent 収束、`Instantiate` caller は waiter drain 経由で `Err(Ipc)`。公開 handle が無いので `InstanceFailed` も出ない（E2E `bundled_mid_create_death_notifies_only_minted` L2797-2833）。**supervisor が見る失敗は「mint 済み handle を持つ失敗」のみ** — `Instantiate` 自体の失敗（spawn/handshake/`CreateInstance` 死）は `Err(Ipc)` で同期返し、別扱いが要る。
- **mid-`DestroyInstance` 死亡でも `Destroy` は `Ok`**: E2E `bundled_mid_destroy_death_ok_and_sibling_failed` L2901- — 死亡が `Destroy` リクエスト中に起きても slot は解放・`Ok` 返却、そのインスタンス自身の通知は slot と共に落ちる。

#### 再 spawn 経路（`composite.rs` + `l1/mod.rs` + `command.rs`）

- 公開面は「`Destroy{handle}` → `Instantiate{plugin_id}`」の 2 コマンドでしかない — `Instantiate` は `plugin_id` のみを取り route 分岐（composite.rs L927-960）。**`InstanceFailed` 通知は `plugin_id` を運ばない** — `handle` と `exit` のみ（composite.rs L217-223）。supervisor 側で「失敗した handle がどの `plugin_id` だったか」の対応表を持つ必要がある（`PluginHost` 側に handle→plugin_id の逆引き面は存在しない）。
- `Destroy`（composite.rs L962-1004）: L0 相当の lifecycle 合法性を composite 側で mirror — `Failed` は `Destroy` 可能（`(Failed, Destroy) → Destroyed`、design L267）。`L1Instance::destroy` は無条件・冪等（l1/mod.rs L2402-2418 付近 — live 子なら `DestroyInstance` 送信で runner registry slot 回収、terminal 子なら local 撤去のみ）。
- `instantiate_bundled`（composite.rs L1081-1139）: pool を走査して `pool_eligible`（`live_count < WIRE_MAX_INSTANCES && !terminal && !shutdown_initiated`、l1/mod.rs L1999-2015）な子を再利用、無ければ fresh spawn（`create_on_fresh` L1141-1164）。terminal 子は scan 経過時に evict（L1116-1124）。**crash 済み pooled 子が残っていても instantiate scan が自然に evict し、次の `Instantiate` は fresh 子に載る** — supervisor が明示的に pool を掃除する面は不要。
- **handle は毎回新規 mint**: `mint`（composite.rs L1065-1078）が slot 再利用時に generation++。再起動後のインスタンスは **別 `PluginHandle`**（同 slot なら generation が上がる）— supervisor は旧 handle を残してはならず、論理インスタンス → handle の写像を更新しなければならない。`HostNotification` が返す handle は通知発行時の generation で構成されるため、slot 解放前に drain すれば正しい handle が得られるが、**通知 drain → Destroy → Instantiate の順序中に新 handle が mint されると、その slot に別の generation が乗る**（stale handle は `Err(InvalidHandle)` で安全に拒否される — slot_mut L1049-1059）。
- **同 route 再 instantiate は pool に委ねられる**: Bundled route なら再起動インスタンスは同 route の残存 pool 子 or fresh 子に載る — **他 route（別 plugin_id）への migrate は `Instantiate{plugin_id}` の形が route ごとの claim を前提にしているため supervisor の範囲では表現しない**（route 選択は登録時判断・composite.rs L393-431）。
- **`SaveState`/`RestoreState` は plugin-defined**: core `plugin.rs` L160-174 — 既定 `Err(Unsupported)`、restore は bytes 拒否で `Err(PluginFailed)`。`Failed` 後の `SaveState` は `Err(Failed)`（execute 入口の Failed gate、l1/mod.rs L2252/L1903）— **死後の state 取得は不可能。checkpoint は故障前に取るしかない**。
- **`RestoreState` は lifecycle 無関係**: runner 側 dispatch（export lib.rs L466-471）に lifecycle gate なし — `Created`/`Initialized`/`Activated` いずれでも適用可能（`post_command_state` l1/mod.rs L2918-2927 で `Some(_)` を返さない = 状態不変）。supervisor の再現シーケンスは `Initialize → RestoreState → Activate → StartProcessing` の順で recipe を再生できる。
- **`InstanceFailed` の endpoint 側**: `drain_endpoint_channels`（composite.rs L881）が detach 済み `PluginEndpoint` の leg 返還を拾う — supervisor の poll 中にこの drain が走る点は通知 drain と同一。再起動後は **新しい `PluginEndpoint` の再 attach（`prepare_endpoint`）と RT 側 owner への差し替えが caller の責務** — supervisor は handle 写像の更新を報告するが、endpoint 再配線自体は util の中ではできない（RT 所有者が別スレッドにいるため）。

#### 通知の「N 個」解釈

- **bundle crash = N 件の `InstanceFailed`（同一 `ChildExit`）**: E2E `bundled_route_shares_one_child_across_instances` L2709-2725 — 1 回の `kill_child` で mint 済み handle ごとに 1 通知、全て同一 exit。**N 通知は「N 回の独立した再起動トリガ」ではなく「1 子プロセスの死が N インスタンスに fan-out した」事実** — supervisor は通知を per-instance の回復イベントとして処理し、各影響インスタンスに対して `Destroy` → `Instantiate` を 1 回ずつ行う。pool が fresh 子を再利用するため、子プロセス自体は 1 個 spawn されれば後続 instantiate はそれに載る（burst re-instantiate の子数は pool 判定任せ — supervisor が「N 通知 = N 子」と解釈して N 個 spawn を強制してはならない）。
- **`converge_instance`（per-instance 帰属違反）は単独の `InstanceFailed`**: 子生存なので兄弟は生き続ける — この場合は `ChildExit{None,None}` を伴う単発通知で、対象インスタンスのみ `Failed`。同じ子上の他インスタンスは巻き込まれない（ADR `issue-117-bundle-child-failed-granularity` AC4(b)）。

#### `Instantiate` 失敗（再 spawn 試行自体の失敗）

- `Err(Ipc)` — spawn/handshake/`CreateInstance` の途中死亡・`Err(InstanceLimit)`（wire cap 超過・実際は pool_eligible 除外で到達困難）。**再 spawn の失敗は「その時点では handle を持たない」ため通知ではなく `Instantiate` コマンドの `Err` で返る** — supervisor の attempt カウントはこの `Err` でも進めるべき（無限 retry しないため）。mid-`CreateInstance` の死亡は子の残り live インスタンスを `Failed` にする fan-out を起こす点に注意（supervisor が既に他の回復を進行中なら新たな `InstanceFailed` 群が届き得る）。

## Alternatives and Comparison

### 軸 1: supervisor 面の置き場所（host util 層 vs composite メソッド vs consumer パターン）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) `ruviform-plugin-host` 内の util 層（`supervisor` モジュール or `SupervisedPluginHost` ラッパ）— `PluginHost` を `&mut` で借りて通知 drain → Destroy → Instantiate → lifecycle replay → RestoreState を駆動 | 再起動の**機械的な手順**（通知 → 破棄 → 再 instantiate → recipe 再生）を 1 箇所に集約 — 全 consumer が再実装する boilerplate を消せる。`PluginHost` の公開 API だけで完結（`try_recv_notification`/`execute`/`lifecycle_state`/`latency_frames`/`prepare_endpoint` は公開済み）。core `PluginHost` 本体は無変更 — AC4 完全維持。crate テストで `l1-test-worker` fixture がそのまま使える | 新規公開 API（`Supervisor`/`SupervisedPluginHost` + `Policy`/`Event` 型）の設計確定が要る。「wrap する crate 側」が handle→plugin_id 逆引き・recipe 記録・attempt 簿記を持つ — その状態は host core ではなく util 側に置く | **yes** |
| (b) `PluginHost` composite メソッド（例: `restart_failed(handle) -> Result<PluginHandle>` + `poll_supervision()`） | API が `PluginHost` に 1 メソッドで収まる・crate 外依存ゼロ | **core に再起動語彙を持ち込む** — AC4「core は自動再起動しない」の境界が「opt-in メソッドがある」と読み取りにくくなる。`PluginHost` が handle→plugin_id 逆引きと recipe 簿記を新たに内部保有する必要（現状は route/slot しか持たない）。`PluginHost::execute` 経由の全コマンドに recipe 追跡が混入する | no — core 境界を汚す |
| (c) consumer パターン文書のみ（docs/designs か test-specs で標準手順を記述し、各 consumer が自前で実装） | コード追加ゼロ — 最小 diff | 全 consumer が同一の（通知 drain → Destroy → Instantiate → replay）を再実装 — **構造的問題の兆候そのもの**（同じドメインルールが複数 caller に分散）。attempt/backoff/exactly-once の落とし穴（通知 drain 前の Destroy で通知喪失・mid-create 死・`ChildExit{None,None}`）を各 caller が踏む | no — 機械手順が DRY 化できない |

推奨方向: **(a) host util 層** — `supervisor` 相当の opt-in モジュールを `ruviform-plugin-host` に置く（拡張層ルール内・`PluginHost` の公開面のみ利用・依存追加なし）。core `PluginHost` の型・フィールド・`execute`/`try_recv_notification` 语义は一切変更しない（AC4 維持）。`PluginHost` を `&mut` で借りる形なら `!Send`/`!Sync` 制約も満たす（supervisor 自体も `!Send` で良い — 同一スレッドの非 RT ループ）。

### 軸 2: 再起動スコープ（同 route のみ vs 他 route への migrate）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) 同一 `plugin_id`（同 route）のみ再 instantiate | `Instantiate{plugin_id}` の意味を変えずに済む。失敗 route の isolation 指定（Bundled/Dedicated）がそのまま引き継がれる — supervisor が route 選択を新設しない | 失敗した plugin_id が恒久的に spawn できない場合（runner binary 消失等）は give up まで繰り返すだけ — ただしそれは「その plugin_id は instantiate できない」という正しい帰結 | **yes** |
| (b) 別 `plugin_id` への fallback/migrate | 障害時に代替 plugin で復帰できる | 「どの plugin_id に落ちるか」を supervisor が知らなければならず route 選択語彙を新設 — `Instantiate` の登録時 claim モデルと矛盾。YAGNI 級 | no |
| (c) `InstanceFailed` 通知の `exit` 値で route 選択 | — | `ChildExit` は spawn 失敗でなく「live 中の子死亡」の status — route 属性情報を持たないので分岐材料にならない | no |

推奨方向: **(a) 同 route のみ**。`plugin_id` は supervisor 側で「監視登録時に caller が指定」または「通知 handle から supervisor 内部 map で逆引き」のどちらか — 後者が機械的に正確（`PluginHost` に逆引き面がないため util 側の帳簿が必要）。**OPEN — design 確定対象**（監視登録 API の形）。

### 軸 3: 再起動の回数・バックオフ形状

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) リトライなし — 通知を消費するだけで復旧は caller 手動 | 最小 | supervisor の存在意義が消える | no |
| (b) 1 回だけ即時リトライ | 最小限の「自動」で十分なケースをカバー（一過性 spawn 失敗・子死亡） | 連続 crash 時に毎 poll で spawn を試みる可能性 — backoff がないと CPU/プロセスコストを浪費。attempt 数を数えないので crash loop と正常運用を区別できない | maybe |
| (c) 固定最大回数 + 固定間隔 backoff（例: 3 attempts・500ms 等間隔） | 実装最简で理解容易 | 一時的な spawn 圧力に即応しすぎる側面 | maybe |
| (d) 最大 attempt 数 + exponential backoff（base delay・上限 cap） | crash loop を早く諦められる・一過性負荷には間を置く — 「最小 useful」の形としてバランス | backoff 計時に `Instant`/`Duration` だけで済む（`PluginHost` は `!Send`、新スレッド不要 — supervisor 側が呼ばれた poll 時点で deadline 判定するだけ） | **yes** |
| (e) 無限リトライ（caller が止めるまで） | — | 契約違反 — 起動不能な plugin を無限に spawn し続け制御ループを占有する | no |
| (f) circuit breaker + `Err(Failed)` への fold | 明確な終端状態 | 「supervisor の give up」を `HostError::Failed` に畳むと**「子死亡で `Failed` になったが Destroy 可能」という現行语义と別物**（supervisor が諦めた = インスタンス自体は既に Destroy 済みで slot も返却済み）— 同じ variant で表現すると語彙が曖昧化 | no — 別語彙で |

推奨方向: **(d) 最大 attempt 数 + exponential backoff（cap 付き）** — attempt カウンタは「監視対象論理インスタンス」単位（respawn で generation が変わっても継承）。backoff の経過判定は supervisor の `poll(&mut host, now: Instant)` 形で caller が渡す — スレッド・タイマー・イベント駆動は入れない（`!Send` 制約と non-RT ループ両立）。**OPEN — 確定値は design**（`max_attempts`・base/max delay・attempt リセット条件「成功してから一定期間生存でカウンタクリア」の有無）。

### 軸 4: チェックポイント所有（誰が state blob を持つか）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) caller が pre-failure に `SaveState` 実行・blob を supervisor に渡す（push 型） | supervisor は store としてのみ機能 — `SaveState` のタイミング・頻度（param 変更 settle・定期・手動 save）を caller のドメイン知識に委ねられる | consumer は「いつ save するか」を自前で管理する必要 | **yes** |
| (b) supervisor が `poll` 内で定期 `SaveState` を駆動（pull 型・周期駆動） | 手動復旧経路が完全自動化 | **`SaveState` は非 RT・allocate 可・plugin hook を呼ぶ** — poll 周期は caller の制御ループ周期であり、supervisor の internal 駆動は save 頻度を暗黙化させる。「param 変更後だけ save」等のドメイン最適化が効かない | no（初版では） |
| (c) `InstanceFailed` 受信時に `SaveState` を試行 | — | **`Failed` 以後 `SaveState` は `Err(Failed)`** — 不可能 | no |

推奨方向: **(a) caller 主導の pre-failure checkpoint** — supervisor は `checkpoint(handle, bytes)` or `save_checkpoint(&mut host, handle)` 経由で最新 blob を保持し、復旧時に `RestoreState` を発行する。**「失敗したインスタンスの state は取れない」は不変条件** — supervisor の「状態復元」の実質は「事前 checkpoint の適用」であり、**restore は best-effort**（checkpoint 未保存なら restore 無しで復旧 = デフォルト状態）。

### 軸 5: 復旧シーケンスの lifecycle 再生範囲（recipe 追跡）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) `watch(handle, recipe)` 形式 — caller が「この handle を supervise する際の recipe」（plugin_id・Activate 用 `ProcessConfig`・`StartProcessing` 到達目標・checkpoint blob）を宣言 | supervisor は機械的な再生だけを行う — 内部でコマンドを傍受して状態を追う必要がない。`PluginHost` 側の変更ゼロ | caller が `Activate{config}` や `RestoreState` した時に `set_recipe`/`update_recipe` で recipe を更新する必要 — recipe 陳腐化リスクは caller 責任 | **yes** |
| (b) supervisor が `PluginHost::execute` を wrap して全コマンドを傍受し recipe を自動追跡 | caller は既存コードを変えずに supervisor を差し込める | `PluginHost` 全面の薄い wrapper が要る — `process`/`lifecycle_state`/`prepare_endpoint`/`plugin_mut` も全て delegate しなければ使えない。`&mut` borrow 規則と `prepare_endpoint`（`&mut self`）の併用で wrapper の複雑度が増す | maybe — (a) の後続拡張として |
| (c) `PluginHost` が `HandleMetadata`（最後に成功した Activate config・plugin_id・checkpoint）を内部保有 | 公開面の追加最小 | core に再起動関連の簿記を持ち込む — AC4 境界を侵す（core が「再起動用の情報」を持つ設計になる） | no |

推奨方向: **(a) recipe 宣言型** — supervisor の API 形は `watch(handle, recipe)` / `set_recipe` / `unwatch` + `poll`。**OPEN — design 確定対象**（`watch` 時点の handle がその後 `Destroy`→`Instantiate` で変わる点・`Activate` 中に `Deactivate`→別 `config` で再 activate した時の recipe 更新規則）。

### 軸 6: 復旧失敗時の挙動（attempt 中の途中失敗・restore 拒否）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) どの段階の失敗（`Instantiate` `Err`・lifecycle `Err`・`RestoreState` `Err(PluginFailed)`/`Err(Unsupported)`）も同一の「attempt 失敗」としてカウント・新 mint 済み handle があれば `Destroy` して後片付け・backoff を経て再試行 | 語彙最小・失敗分類が要らない | `RestoreState` の `Err(Unsupported)`（plugin が state 機能を持たない）は「復旧不能」なのか「state 無しで動くべき」かが曖昧 — `Err(Unsupported)` を attempt 失敗として数えると、state 非対応 plugin が max_attempts 後に give up して動かなくなる | maybe |
| (b) `RestoreState` の `Err(Unsupported)` は「復旧成功（state 非対応）」として扱い、`Err(PluginFailed)`/`Err(Ipc)`/`Err(Failed)` のみ attempt 失敗 | state 非対応 plugin でも復旧が動く — 実用性 | 「state は持っていたが新規インスタンスでは入れられない」を無視する — checkpoint blob の意味を保証できない | maybe |
| (c) `RestoreState` 失敗（`Unsupported` 含む）は全て attempt 失敗 | 契約厳密 | (a) と同じ問題 | no（初版では厳しすぎる） |
| (d) attempt 内の失敗で give up 時に新 mint インスタンスを `Destroy` せず残す | 失敗の跡が調査可能 | **`Failed` でなく live のまま残った中途半端なインスタンスが残る** — リソースリーク・次回 poll で再度 destroy 処理が要る | no |

推奨方向: **(b) `Err(Unsupported)` は state 非対応としてスキップ、他の `Err` は attempt 失敗** — 途中で mint 済み handle が存在する場合は `Destroy` で後片付け（残留リソースなし）。**OPEN — design 確定対象**（`RestoreState` 失敗を「attempt 失敗」とみなすか「checkpoint 無効で state 無し継続」に倒すかは UX 論点）。

### 軸 7: supervisor の駆動形（スレッド vs poll）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) caller の非 RT 制御ループ内で `supervisor.poll(&mut host, now)` を呼ぶ | `PluginHost` `!Send`/`!Sync` に自然適合 — 追加スレッド不要。backoff の時計は引数の `Instant` — テストで時間制御も容易（`Instant::now` 依存を消せる） | caller が poll を忘れると復旧が走らない — ただし `try_recv_notification` 自体も caller が drain する設計なので既存パターンと同じ責務分担 | **yes** |
| (b) supervisor が内部スレッドを持ち `PluginHost` を別スレッドで共有 | — | **`PluginHost` は `!Send`/`!Sync`** — スレッドを跨げない。型レベルで不可能 | no（型制約） |
| (c) `try_recv_notification` の内部に監視を組み込み（core が drain 中に再起動を判断） | — | core 語彙への混入（軸 1 (b) 同様）かつ自動再起動そのもの — AC4 違反 | no |

推奨方向: **(a) caller 駆動 poll** — supervisor は「通知 drain → attempt 実行 → 結果 event を返す」だけの非同期なし state machine。**全ての復旧作業は caller の非 RT スレッド上で同期的に実行**（`Instantiate`/`Destroy`/`RestoreState` は reply-wait を伴う非 RT コマンド — poll 呼出し自体が non-RT 前提）。

### 軸 8: 復旧結果の報告形（handle 差し替え・endpoint 再配線）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) supervisor が `SupervisionEvent`（`Restarted { old_handle, new_handle }` / `GaveUp { handle, attempts }` / `Pending` 等の `#[non_exhaustive]` enum）を `poll` の戻り値で返し、caller が新 handle を受けて `prepare_endpoint` 再 attach・RT 側 owner 差し替えを行う | supervisor は機械的な復旧に専念、endpoint/RT 配線は caller の責務を維持（RT 所有者は別スレッド — supervisor が直接触れない構造的必然） | consumer は新 handle を受け取って自分の管理表を更新する必要 — ただしそれは「通知で handle が来る」現行パターンと同型 | **yes** |
| (b) supervisor が内部で `prepare_endpoint` まで実行し `PluginEndpoint` を返す | 1 呼出しで完全復旧 | `PluginEndpoint` は RT 所有者への move が前提 — supervisor が endpoint を保持しても RT 側へ安全に渡す経路（mpsc 等）が別途要る。`prepare_endpoint` の bounds も caller ドメイン知識が要る | no — 責務越え |
| (c) 旧 handle を維持（slot reuse で同 slot 復活を試行） | — | generation 規則で不可能 — `mint` は世代を bump するので同 slot でも別 handle | no（型制約） |

推奨方向: **(a) `SupervisionEvent` 報告** — `Restarted` には `{supervised_handle（論理 ID or 旧 handle）, old: PluginHandle, new: PluginHandle}` を運び、caller が新 handle で endpoint/管理写像を更新する。`Restarted` を発行した時点で新インスタンスは **少なくとも `Initialized`（+ recipe があれば `Activated`/`Processing` 復帰・restore 済み）** — endpoint 未 attach 状態なので `PluginHost::process` は `Err(EndpointUnavailable)`…ではなく、`process` は detached leg が無い状態では host-side staging で処理できる（composite.rs L536-575 — `process` は endpoint 未 attach でも動く、ただし caller が RT 側を再配線するまでその path は「RT 所有者なし」）。

## Tentative Conclusions and Hypotheses

- **confirmed**: `PluginHost` core への自動再起動ロジック導入は **不可** — #31（再起動は呼出し側判断）と AC4（自動再起動なし）の双方を破る。core の語彙（`Failed`/`InstanceFailed`/`Err(Failed)`/通知语义）は不変
- **confirmed**: supervisor 面は `ruviform-plugin-host` 内の **opt-in util 層**（`supervisor` モジュール or `SupervisedPluginHost` ラッパ）が推奨 — `PluginHost` 公開 API のみで機械的復旧手順を集約でき、core 無変更で AC4 を維持。crate テストでは既存 `l1-test-worker` + `#[path]` harness がそのまま使える
- **confirmed**: 再起動スコープは **同 `plugin_id`（同 route）のみ** — route 横断 migrate は `Instantiate` の登録時 claim モデルと矛盾し YAGNI。supervisor 側に handle→plugin_id の帳簿が要る（core に逆引き面なし）
- **confirmed**: 再 spawn 経路は **`Destroy{failed}` → `Instantiate{plugin_id}`** の既存 2 コマンド — `Failed` は `Destroy` 可能、`Instantiate` は pool evict を自動処理して fresh 子に載る。handle は毎回新規 mint（generation++）— supervisor は新 handle を event で caller に渡し論理インスタンス→handle 写像を更新しなければならない
- **confirmed**: bundle crash は N 個の `InstanceFailed`（同一 `ChildExit`）— supervisor は per-instance 回復として処理し、**子数は pool 判定に委ねる**（N 通知 = N 子ではない）。mid-`CreateInstance` 死は通知なし・`Err(Ipc)` で返る（handle 未 mint — attempt 対象外ではなく attempt 失敗として扱う）
- **confirmed**: state 復元は **事前 checkpoint 前提** — `Failed` 後 `SaveState` は `Err(Failed)` で不可能。caller が pre-failure に `SaveState` 実行 → supervisor へ blob 引渡し → 復旧時 `RestoreState` 発行。`RestoreState` の `Err(Unsupported)` は state 非対応としてスキップ（attempt 失敗扱いしない）が実用的だが **OPEN**
- **confirmed**: supervisor は **caller 駆動 poll 型**（`poll(&mut host, now: Instant) -> Vec<SupervisionEvent>`）— `!Send`/`!Sync` 制約・non-RT ループ両立・スレッド不要・`Instant` inject 可能で backoff テスト容易。内部タイマー・新スレッドは作らない
- **hypothesis**: 最小 useful ポリシー形状は `Policy { max_attempts, base_delay, max_delay }` 相当 + attempt カウンタ（論理インスタンス単位・respawn で継承・成功後一定期間生存で reset 可）— 値は **OPEN**（YAGNI 観点では「1 回即時リトライ」でも成立するかが user 確認論点）
- **hypothesis**: endpoint/RT 側の再配線（`prepare_endpoint` 再 attach・RT 所有者への差し替え）は caller 責務 — supervisor は `Restarted{old,new}` を返すだけ。これは `prepare_endpoint` が `&mut` borrow を要し RT 所有者が別スレッドにいる構造的必然
- **hypothesis**: `Err(Failed)` 語彙は supervisor の give up 報告に再利用しない — give up は `SupervisionEvent::GaveUp` 相当で返し、core の `Failed` は「子死亡で live インスタンスが terminal」语义のまま残す（supervisor が諦めた時点でそのインスタンスの slot は既に解放済み — 「`Failed` だが `Destroy` 可能」语义とは別物）

## Open Questions

> 全て design/ADR フェーズが確定する論点 — architecture 契約レベル（ADR 候補）と実装設計レベルを分記。

### ADR 候補（architecture 契約 — 変更コストが大きい語彙）

- **supervisor util の crate 位置と公開形**: `ruviform-plugin-host` 内 `supervisor` モジュールの新設を決定するか（公開 API 追加 — 新規モジュール + `SupervisionEvent`/`Policy`/`Recipe` 型。公開 enum 追加は `#[non_exhaustive]` 方針で後方互換）。`docs/adr/` での確定記録が要る
- **「自動再起動なし」契約の解釈範囲**: supervisor util の存在自体が AC4 を破らないことの確認 — 「`PluginHost` core は自律的に再起動しない（caller が明示的に util を呼んだ時のみ）」と設計文書に明記するか
- **give up 後の语义**: supervisor が max_attempts 到達後の論理インスタンスを caller 側にどう報告するか（`SupervisionEvent::GaveUp` 相当）— `Err(Failed)` を再利用しないことを ADR/design で明確化するか

### design 確定対象（実装設計 — ADR ほどの重さは不要）

- 監視登録 API の形: `watch(handle, recipe)` で caller が recipe（plugin_id・Activate config・StartProcessing 目標・checkpoint blob）を宣言するか、`PluginHost` 側に handle→plugin_id 逆引き面を追加して supervisor を stateless に近づけるか（core への簿記追加は避けたい — util 側帳簿が妥当と見込むが確定論点）
- `watch` 後の handle 変化の追跡: caller が `Destroy`→`Instantiate`（非 supervisor 経路）で handle を変えた場合、`unwatch`/`set_recipe` で追従する規則
- recipe の更新規則: `Activate{config}` 再実行・`RestoreState` 手動適用時に caller が `set_recipe` を呼ぶ義務の明文化
- `RestoreState` 失敗の分類: `Err(Unsupported)`（state 非対応）を「復旧成功（state スキップ）」とするか、`Err(PluginFailed)`（bytes 拒否）を「attempt 失敗」とするか — UX 論点として確定
- backoff 確定値: `max_attempts`（候補 2-4）・base/max delay・成功後の attempt reset 条件・`Err(Unsupported)` を attempt 消費しない扱いか
- `SupervisionEvent` の variant 語彙: `Restarted`/`GaveUp`/`BackingOff`（再試行待ち）/`Unwatched` 等 — `#[non_exhaustive]` 前提の最小集合
- bundle fan-out の扱い: 同一 poll 内で N 通知が来た場合の処理順序（drain 順・通知 handle 昇順）・1 poll 内の attempt 上限
- `kill_child`（`#[doc(hidden)]` 検証面）との併用: supervisor 検証で bundle crash を注入する手順は既存 E2E パターンを流用できるが、supervisor 側が `kill_child` を呼ぶ必要性があるか（検証テスト構造の詳細）
- `PluginEndpoint` の再 attach タイミング: `Restarted` event 発行時点では新 instance は `Processing` 復帰済みか、endpoint attach は caller が行うが `prepare_endpoint` の bounds 再指定は `EndpointBounds` を recipe に含めるか
- `latency_frames` の recipe 記録: `Activate{config}` とは別に `ProcessConfig`（sample_rate・max_frames・channels）を recipe に含める必要があるか — `latency_frames`/`activated_sample_rate` からの逆算では config が完全復元できない点

## Promotion Candidates

- [x] spec-doc: [issue-116-supervisor.md](../specs/issue-116-supervisor.md)（**created — draft**。Issue 要求・選択肢比較・推奨方向・open 論点を DRAFT として記録。受入基準は「opt-in supervisor util 面・同 route 限定再 spawn・checkpoint/restore・backoff・give up 報告・AC4 維持」に分解）
- [ ] adr-doc: **supervisor util の crate 位置（`ruviform-plugin-host` 内 `supervisor` モジュール新設）と give up 語彙の独立**は公開 API・公開 enum 追加に関わるため design 確定時に `docs/adr/issue-116-*` で記録するのが筋（`#[non_exhaustive]` 公開 enum 追加は後方互換だが「追加するかどうか」の方針確定は ADR 級）
- [ ] design-doc: supervisor util の実装設計（`Supervisor`/`Policy`/`Recipe`/`SupervisionEvent` 型・poll 型 state machine・帳簿構造）は design フェーズで `docs/designs/issue-116-*.md` として作成

## Sources and Relations

- Issue [#116](https://github.com/scarecrow173/ruviform/issues/116)（本探索対象 — supervisor 運用面の確定）・Issue [#31](https://github.com/scarecrow173/ruviform/issues/31)（プラグイン分離設計・CLOSED — 再起動判断/状態復元の責務帰属を確定した起点）・Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)（L1 実装・CLOSED — `Failed`+通知+自動再起動なしの基底契約）・Issue [#117](https://github.com/scarecrow173/ruviform/issues/117)（bundle 子プロセス・implemented — 本 Issue の失敗粒度と fan-out 機構を定義）・Issue [#119](https://github.com/scarecrow173/ruviform/issues/119)（wire バージョン方針・supervisor は wire 変更を伴わない点で無関係）・Issue [#120](https://github.com/scarecrow173/ruviform/issues/120)（共有リーパー/io 多重化 — supervisor poll 型駆動は同じ caller ループ上に乗る形で競合しない）
- 上流文書: [plugin-isolation design](../designs/plugin-isolation.md)（#31 確定判断の正本）・[plugin-isolation-strategy ADR](../adr/plugin-isolation-strategy.md)（L0/L1 分離決定）・[plugin-l1-out-of-process discovery](plugin-l1-out-of-process.md)・[spec](../specs/plugin-l1-out-of-process.md)・[design](../designs/plugin-l1-out-of-process.md)（#76 系 — AC4・`Failed`/通知/pass-through/非 RT・deferred 行の起点）・[issue-117-bundle-child-process design](../designs/issue-117-bundle-child-process.md)・[ADR](../adr/issue-117-bundle-child-failed-granularity.md)・[spec](../specs/issue-117-bundle-child-process.md)（bundle 後の `Failed` 粒度・fan-out・pool・`WIRE_MAX_INSTANCES`）・[engine-plugin-host-integration design](../designs/engine-plugin-host-integration.md)（#113 — `PluginEndpoint` の detach/attach 所有権 — supervisor 復旧後の endpoint 再配線が関わる）
- 下流文書: [issue-116-supervisor spec](../specs/issue-116-supervisor.md)（`derived-by` — DRAFT）
- コード地図: `crates/ruviform-plugin-host/src/composite.rs`（`PluginHost` 統一 slot 表・`try_recv_notification`・`instantiate`/`instantiate_bundled`/`create_on_fresh`・`destroy`・`mint`/generation・`kill_child`・`prepare_endpoint`・`lifecycle_state`/`latency_frames`）・`crates/ruviform-plugin-host/src/l1/mod.rs`（`converge_child`/`converge_instance`/`remove_instance`/`Notify` 状態遷移・`pool_eligible`/`is_terminal`/`live_count`/`create_instance`/`spawn`/`destroy`・`L1Instance::try_recv_notification`）・`crates/ruviform-plugin-core/src/plugin.rs`（`save_state`/`restore_state` — `Err(Unsupported)`/`Err(PluginFailed)`）・`crates/ruviform-plugin-core/src/boundary.rs`（`HostError` 語彙 — `Failed`/`Ipc`/`InstanceLimit`/`InvalidHandle` 等）・`crates/ruviform-plugin-export/src/lib.rs`（runner `dispatch` — `SaveState`/`RestoreState` は lifecycle gate なし）・`crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（`l1-test-worker` fixture・bundle fan-out/復旧/中途死亡 E2E — supervisor 検証の流用元）・`crates/ruviform-plugin-host/src/lib.rs`（crate 公開面・RT contract・依存層ルール）
