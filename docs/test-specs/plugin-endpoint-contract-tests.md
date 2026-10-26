---
id: "TSPEC-034MrsawZsMkT7wzC5cbuK"
type: "test-spec"
status: "proposed"
title: "plugin endpoint 契約（prepare/bounds・Shared セル・arm 経路・leg 返却・Send 証拠）テスト仕様"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/plugin-graph-e2e-tests.md"
    - "docs/test-specs/plugin-rt-retirement-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/engine-plugin-host-integration.md"
  derived-by: []
  verifies:
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/designs/engine-plugin-host-integration.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# plugin endpoint 契約（prepare/bounds・Shared セル・arm 経路・leg 返却・Send 証拠）テスト仕様

> status: proposed — Issue #113 の endpoint 面（`PluginHost::prepare_endpoint`/`PluginEndpoint`/`EndpointBounds`・L0 Shared セル・L1 arm/返却経路・拒否表）の受入契約を固定する。run 可能コマンドは task 側に定義する（本書は契約と oracle の正本）。

## Purpose

spec `docs/specs/engine-plugin-host-integration.md`（proposed・11 AC）と approved design `docs/designs/engine-plugin-host-integration.md` に基づき、`ruviform-plugin-host` の endpoint 公開面とその内部機構の受入契約を固定する — (a) `prepare_endpoint` の state/bounds/単一 endpoint 拒否面、(b) L0 `SharedSlot`/`SendCore` 共有セルと全 verb の 3 段 lock 形態・commit 順序・`Failed` 収束、(c) L1 leg 移出・per-attach arm channel・`pending_arm` 再送・leg 返却の所有規則、(d) `HostError::EndpointUnavailable` の local-only 性と `encode`/`encode_host_error` の fallible 化・all-or-nothing append、(e) `Send` 証拠の compile-time 固定。グラフ経由の E2E・gain 正当性は `plugin-graph-e2e-tests.md`、`rt_` 命名の no-alloc 実測と plan 退役観測は `plugin-rt-retirement-tests.md` が担当する。endpoint 公開面または Shared/arm 機構が置換・再設計された場合に限り本 spec を retire する。

## Feature

`ruviform-plugin-host` の RT endpoint 面: `PluginHost::prepare_endpoint(handle, EndpointBounds) -> Result<PluginEndpoint, HostError>`（non-RT verb・`PluginHost` コンポジットのみ — standalone `InProcessHost` への endpoint API 配線は v1 非対象）・`PluginHost::latency_frames(handle) -> Option<u32>`・`PluginEndpoint`（`Send` — `inner: EndpointInner`/`bound`/`in_scratch`/`latency_frames`）・`PluginEndpoint::process(..., events: EventList<'_>, transport: Option<TransportInfo>) -> Result<ProcessResult, HostError>`・`PluginEndpoint::process_audio(..., events: EventList<'_>, transport: Option<TransportInfo>, ctx: &RtContext) -> Result<ProcessResult, HostError>`（`events`/`transport` は by-value — 両型とも `Copy`・`ProcessResult.tail_frames` を保持 — `ProcessStatus` ではない）・L0 `SharedSlot`/`SendCore`・L1 leg 移出 + arm channel + leg 返却。fixture は Send-capable mock plugin（`create_sendable` を実装する test factory — gain・panic 注入・DropFlag probe）と既存 `RtLeg`（child なしで unit 検証可能な構造）。

## Rules

- **`prepare_endpoint` 拒否面（spec AC4）**: **拒否順序 — (1) state 系 → (2) slot-shape/presence → (3) bounds**。(1) state が `Activated` 未満（`Created`/`Initialized`）→ `Err(HostError::Lifecycle(_))`・`Failed` → `Err(HostError::Failed)`・`Destroyed`/stale handle → `Err(InvalidHandle)`。(2) `Plain` slot（非 Send）・L0 epoch CAS 失敗・L1 `rt.is_none()` → `Err(EndpointUnavailable)`。(3) `bounds.max_frames == 0`/`bounds.channels == 0`・`bounds.max_frames < activated max_frames`・`bounds.channels > WIRE_MAX_CHANNELS`(16)/`bounds.max_frames > WIRE_MAX_FRAMES`(8192) → `Err(HostError::Prepare(PrepareError::UnsupportedConfig))`（両層共通 cap — caller-controlled scratch 確保の上限として L0 にも適用・`u32::MAX` も同じ defined rejection で allocation abort にしない）。**state 違反と bad bounds が同時でも state 系 `Err` が返る**（order pin）。`bounds.channels` は v1 で activated main bus との等値を要求しない（channel query 未存在 — non-0・wire 上限・per-block `out.channels()==bound.channels` 契約のみ）。**bounds-validation prelude は `PluginHost::prepare_endpoint`（T4）が所有** — 拒否順序 (3) bounds 位置での uniform 検査（(1) state 系・(2) slot-shape/presence 判定の後・slot 内側 attach 呼出し前 — composite は既存の per-slot routing・lifecycle 読みで先行判定し新 accessor は不要）・T2/T3 内側は defense-in-depth 再検査。`AudioBuffer::try_new` の shape `Err`（`InvalidValue`）は boundary で `Err(Prepare(UnsupportedConfig))` へ写像
- **単一 endpoint 規則 + `prepare_endpoint` Err 契約**: 1 インスタンスに attach できる endpoint は 1 本。L0 は `SharedSlot::next_epoch`（初期値 1・attach 毎に単調増）を採番し `attached.compare_exchange(0, epoch, AcqRel, Acquire)` — 非 0 なら `Err(EndpointUnavailable)`。endpoint `Drop` は `compare_exchange(epoch, 0)` で**自分の epoch のみ**返却（後続 attach の新 epoch を掴み消さない）。L1 は `Option::take` で型強制 — `rt.is_none()` の間は `Err(EndpointUnavailable)`、leg の `return_rx` drain 復元後のみ再 attach 可能。**`prepare_endpoint` が `Err` を返した後もインスタンスは attachable/processable のまま** — L1 の `prepare_endpoint` は (1) state check → (2) bounds validation（**take 前** — bad bounds は leg を動かさず `Err(Prepare(UnsupportedConfig))`）→ (3) `rt.take()` の順序で、(3) 以降の全 `Err` 経路は `self.rt = Some(leg)` で leg を復元する（post-take Err で leg が orphan 化しない — 再 `prepare_endpoint` が `Ok` に戻れる）
- **L0 非 Send 対応の defined rejection**: `create_sendable()` が `None` の factory のインスタンスは `Plain` スロットのまま — `prepare_endpoint` → `Err(EndpointUnavailable)`。L0 out-of-graph 利用（`plugin_mut`/`process` 直接呼出し）は損なわない
- **L0 Shared セルの verb 到達（design h）**: `Shared` スロットで `plugin`/`context` に触れる**全 verb**（lifecycle 動詞 + `SaveState`/`RestoreState`/`OpenGui`/`CloseGui`）は `core.lock()` を transition 合法性検査 + commit + `set_state`（mirror store）+ plugin hook の全体に保持 — `try_lock` 観測は常に完全 commit 済みのみ。**`mirror ∈ {Activated, Processing} ⟹ activated.is_some()`** が endpoint 不変条件（u8 順で `Failed`=4/`Destroyed`=5 ≥ `Activated`=2 — `≥` 比較は不変条件に使わず列挙で書く）
- **verb 3 段 lock 形態**: (1) entry pre-check — mirror Acquire 読みで `Failed` なら lock せず収束（`LifecycleEvent::Failed` + `Err(HostError::Failed)`）。(2) `core.lock()` を `match` で分岐 — `Err(PoisonError)` なら同じ収束。(3) lock 内で mirror Acquire 再読 — `Failed` なら hook/transition/`set_state` を一切実行せず解放して収束。`destroy` のみ `unwrap_or_else(PoisonError::into_inner)` で teardown（host 側 `Failed→Destroyed` transition + mirror=`Destroyed` store + core 内容物 drop — panic 済み plugin の destroy hook は呼ばない）
- **`&self` 照会は観測 union のみ**: `lifecycle_state(&self)`（`InProcessHost`/`PluginHost`）は `Shared` スロットで mirror atomic の Acquire 読みのみ — `core.lock()` は決して取らない・`lifecycle.state()` は変更しない（commit は `execute`/`process`/`prepare_endpoint`/`destroy` の `&mut` entry point で行う）。`plugin_mut` は `Shared` で `None`（`Mutex` 越しに `&mut` を貸せない — L1 と同じ defined 挙動）。host 側 `process` は `try_lock` — busy は `Err(EndpointUnavailable)`（blocking `lock()` しない — RT-suitable 維持）
- **L0 endpoint `process` 経路**: mirror Acquire fast-skip（`Failed`/非 `Processing` の早期 short-circuit — authoritative 判定は lock 内再読）→ `core.try_lock()`（busy → `Err(EndpointUnavailable)` = `out` 不変更の一時 bypass・ハングしない）→ lock 内 mirror Acquire 再読 → `activated` gate（`min_frames ≤ frames ≤ max_frames`・channels 一致）→ `plugin.process_host(...)` を `catch_unwind` で駆動 — panic 捕捉・poisoned `try_lock`・`Failed` 観測のいずれでも `mirror.fetch_max(Failed)`（endpoint は第 2 の書き手）+ `Ok(CONTINUE)` + 無音書込み（終端条件 — 以後 mirror 読みで short-circuit）
- **L1 leg 移出と arm 経路（5 点固定）**: `L1Instance.rt: Option<RtLeg>` — `prepare_endpoint` が `take` で移出。(1) `prepare_endpoint` が `channel::<Arc<Staging>>(capacity 2)` を鋳造し `Inner::staging` clone を **seed arm** として `try_push` — **seed は `seed.generation > leg 現装着 staging.generation` のときのみ apply・同 generation のときのみ正規化 no-op（apply されず unapplied のまま retire）**。通常経路では `Inner::staging` clone = leg 装着 ring と同 generation で no-op だが、detached-Activate で `Inner::staging` が新世代へ進んだ場合は seed が legitimately apply される（「常に no-op」前提を書かない — leg は装着 staging・`frame_capacity`・seq 簿記を保持して移動）。(2) `Activate` 成功時に control が `try_push` — `Full` は `pending_arm` 退避（**latest-wins**・中間世代の喪失は問題にならない）。(3) `pending_arm` resend・`return_rx` drain・arm-`DropCollector` drain は**全 `&mut` non-RT entry point** の冒頭（`L1Instance::execute`/`prepare_endpoint`/`destroy`/teardown + `PluginHost::try_recv_notification(&mut)`）— `&self` query（`lifecycle_state`/`latency_frames`/`L1Instance::try_recv_notification`）は defined non-drain observation point。(4) endpoint は `process` 冒頭で `arm_rx` を bounded drain（1 pass の pop ≤ `arm_capacity + 1` = 3）し `generation > 現装着` のみ `arm_apply` — drain した arm は適用・不適用を問わず全て `Retirer`(capacity 4) へ。(5) `frame_capacity > bound` の arm は apply せず retire + `Err(Ipc)` バックストップ（本線は control 側 `Activate` gate `Err(Prepare(UnsupportedConfig))` — `Err(Ipc)` は endpoint 内部処理で unrepresentable block として収束し公開面の `Err` として露出しない）
- **`RtLeg::arm_apply` 簿記分割**: ring 束縛のみ reset（`staging` 差替・`frame_capacity`・`last_output` 中身 clear + `resize`（capacity 内・realloc なし — `prepare_endpoint` で `bound.max_frames × WIRE_MAX_CHANNELS` 事前 `reserve`）・`last_frames`/`last_channels`/`have_output`）— `last_pushed`/`cleared`/`held`/`submit_dims`/`next_block`/`depth`/`lookahead` は arm を跨いで持続（`cleared[i] ≥ last_pushed[i] ⇒ slot i 空` の strict-alternation 不変条件を維持・stale done が再 arm 後 seq と alias しない）
- **leg 返却 + drain-before-early-return**: endpoint `Drop` で `leg.take()` を `return_tx.try_push`（capacity 1 — `TryPushError::Full` のみ・`into_inner()` 回収した leg は endpoint を drop したスレッド上でそのまま drop）。`L1Instance` の `&mut` entry point が `return_rx.try_pop` で `self.rt` 復元 — plan swap が一度も走らない session でも leg が還る。**leg 復元時は per-attach field を全て clear**（`arm_tx`/`pending_arm`/`return_rx`/`arm_collector`/`endpoint_bound` = None — stale producer/consumer・stale bound 簿記を残さず、次 attach は `prepare_endpoint` が新規 mint で配線）。`L1Instance::process` は `rt.is_none()` early-return **より前**に wait-free drain（`pending_arm` resend + `return_rx` pop — `arm_collector.drain()` は除く・destructor 実行は RT 契約違反）を行い、detached 中は `Err(EndpointUnavailable)`。leg 喪失（host 側 channel endpoint 既 drop）は再 attach 不能の defined 終端
- **再 Activate bound gate**: attach 生存中（L0 = `attached` epoch ≠ 0・L1 = **`self.rt.is_none()`（leg 移出中）が authoritative 判定** — `endpoint_bound` は簿記値であり liveness 判定に使わない）かつ `new_config.max_frames > bound.max_frames` → `Err(Prepare(UnsupportedConfig))`（比較対象は attach 時に `SharedSlot::bound`/`endpoint_bound` へ保持した bound）。bound 内の再 Activate は L1 で generation+1 の新 ring が arm 経路で届き、L0 は `SendCore.activated` 差替が同じ `core.lock()` 経由で RT 可視
- **`latency_frames`（spec AC6・修正版式）**: `PluginHost::latency_frames(handle) -> Option<u32>` — L0 = `Some(0)`・L1 = `Some(lookahead_blocks × activated max_frames)`（frames 整数 — wire/staging が dimension する上限量子）・未 `Activate`/無効 handle = `None`。`PluginEndpoint::latency_frames() -> u32` は prepare 時点値を保持（re-activation で変わり得ることを doc 明記）。秒換算は `frames / sample_rate` を doc に明記（f64 秒にはしない）
- **`EndpointUnavailable` は local-only variant**: wire 経路に乗らない — `encode_host_error` を `Result<(), WireError>` 化し wildcard arm で `Err(WireError::UnsupportedVariant)` を返す（`Ipc`/`Unsupported` へ潰さない — host-local の topology 事実を transport/plugin 起因と誤記させない）。`pub fn encode` も `Result<(), WireError>` 化 + 失敗時 `out.truncate(start)` の **all-or-nothing append**（partial frame を残さない）。`encode_reply` は `HostError` 不到達のため infallible のまま。`Display` arm 追加 + roundtrip 用手動 variant リスト（wire 表現可能 variant のみ）の更新 — `EndpointUnavailable` はリストに入れない
- **defined 拒否表（endpoint `process`/`process_audio` — `Result<ProcessResult, HostError>`・`events`/`transport` は by-value `Copy`）**: mirror `Failed`/poisoned/catch 済み panic → `Ok(CONTINUE)` + 無音。非 `Processing` → `Err(Lifecycle(_))` で `out` 不変更。L0 busy → `Err(EndpointUnavailable)` bypass。bound clip（`out.channels()==bound.channels`・`frames ≤ bound.max_frames`）/activated gate 違反（L1 は上界のみ `frames ≤ armed staging.frame_capacity` — `EndpointInner::L1::process` が leg `block_data` 呼出し前に `armed_frame_capacity` で pre-gate し `Err(Prepare(UnsupportedConfig))` — `min_frames` は child 側 plugin 契約に委譲）→ `Err(Prepare(UnsupportedConfig))` で `out` 不変更。**`Err(Ipc)` は L1 内部の wire 表現不能に留め、`process`/`process_audio` の公開 `Err` 経路として露出しない**（pre-gate が `block_data` の `Err(Ipc)` に先行・arm `frame_capacity > bound` の `Err(Ipc)` バックストップは内部処理で収束）。L1 結果未着/slot busy → pass-through（直前受信 else 無音・`Ok(CONTINUE)` — エラーでなくフォールバック）
- **`Send` 証拠（spec AC2 — `static_assertions` 採用）**: `expect_send::<PluginEndpoint>()`（`#[cfg(test)]` inline 型関数 — `PluginEndpoint` 具象型）・具象 `L0Endpoint`/`L1Endpoint`（crate-private — `src/` 内 `#[cfg(test)]` または lib 配置・外部 `tests/` には置かない。**enum variant 自体は型ではない**ため `EndpointInner::L0`/`L1` の variant 名を assert 引数に書かない）・endpoint を capture したノード closure（`FnMut(&mut AudioBufferViewMut<'_, f32>, &RtContext) + Send` の具体 closure 型）・`GraphPlan`/`RtPlan`/`OutputDataCallback` capture が `Send` であることを compile-time テストで静的に示す — `unsafe impl Send` は使わない。**負側（`!Send`）は `static_assertions` dev-dep の `assert_not_impl_any!`** で `Rc`/`RefCell`/`PluginHost`/`L1Instance`/`Box<dyn ErasedPlugin>`（無印）の capture を固定（`expect_not_send` 型関数は `!Send` を compile 時に表明できないため不適 — trybuild なし）。`PluginHost`/`L1Instance`/`Box<dyn ErasedPlugin>`（無印）の参照が RT 側に存在しないことを型で担保

## Examples

- **attach/detach 往復**: Send-capable factory → `Instantiate`→`Initialize`→`Activate{max_frames:512}`→`StartProcessing` → `prepare_endpoint(h, {max_frames:512, channels:2})` → `Ok(endpoint)` — 生存中の 2 本目 `prepare_endpoint` → `Err(EndpointUnavailable)`・endpoint drop（non-RT スレッド）→ `return_rx` drain を促す `&mut` entry（例: `try_recv_notification` または次 verb）→ 3 本目 `prepare_endpoint` → `Ok`
- **二重 attach 拒否（L0 epoch）**: L0 Shared slot で attach 中に 2 本目 `prepare_endpoint` → `Err(EndpointUnavailable)` — epoch CAS が `attached != 0` を検出
- **非 Send factory**: `create_sendable` を実装しない factory（default `None`）のインスタンスは `plugin_mut`/`process` で従来どおり駆動できるが `prepare_endpoint` → `Err(EndpointUnavailable)`
- **verb 到達（graph-attached）**: endpoint attach 中の L0 インスタンスへ `SaveState`/`RestoreState`/`StopProcessing`/`Deactivate` → 通常どおり `Ok`（lock 内 commit）— verb 実行中の endpoint `process` は busy → `Err(EndpointUnavailable)` bypass・ハングしない
- **endpoint panic 収束**: panic する Send mock plugin で endpoint `process` → `catch_unwind` 捕捉 → `Ok(CONTINUE)` + 無音・mirror=`Failed` store（`fetch_max`）— 以後の `process` は mirror 読みで short-circuit・verb 系は entry pre-check/lock 内再読で `Err(Failed)` 収束・`lifecycle_state(&self)` は `Some(Failed)` の観測 union のみ
- **poisoned lock**: verb 実行中に plugin hook が panic → lock poison — 後続 verb は `match lock()` の `Err` 分岐で `Err(Failed)` 収束・`destroy` のみ `into_inner` で `Ok` teardown
- **arm seed no-op**: `Activate` 済み → `prepare_endpoint` → seed arm push — endpoint 初回 `process` で drain されるが同 generation のため apply されず unapplied retire（`arm_retirer` 側 `DropCollector` が drain で回収）
- **再 Activate arm 供給**: attach 中に `Deactivate`→`Activate{max_frames:512}`（bound 内）→ control が新 `Arc<Staging>`（generation+1）を `try_push` → endpoint 次 `process` で apply — 旧 ring は `Retirer` 行き（RT drop なし）。bound 超過の再 `Activate` → `Err(Prepare(UnsupportedConfig))`
- **`pending_arm` latest-wins**: arm channel Full 状態で連続 `Activate` → 退避は最新のみ — 次 `&mut` entry で resend → endpoint は最新 generation を apply・中間 arm は unapplied retire
- **detached process**: L1 `prepare_endpoint` 後に host 側 `process(h,…)` → `Err(EndpointUnavailable)`（wait-free drain は実行済み — early-return より前）
- **`encode` の all-or-nothing**: 既存バイトを持つ `out` へ `Reply{Err(EndpointUnavailable)}` を encode → `Err(UnsupportedVariant)` かつ `out` の長さ・内容は呼出し前と不変（`REPLY_ERR` tag も残さない）。wire 表現可能な全 `HostError` variant の roundtrip は従来どおり成功
- **`latency_frames`**: L1 `OutOfProcessSpec{lookahead_blocks:2}` + `Activate{max_frames:512}` → `latency_frames(h) == Some(1024)`・endpoint 側も `== 1024`・未 `Activate` → `None`・L0 → `Some(0)`

## Guarantees

- `prepare_endpoint` の全拒否は defined `Err` — panic/UB/alloc 失敗 abort なし（`AudioBuffer::try_new` shape `Err` は `Prepare(UnsupportedConfig)` へ写像）
- `Shared` スロットで全 non-RT verb が到達可能のまま — `try_lock` が観測するのは完全 commit 済み状態のみ（half-committed mirror は構造上存在しない）
- endpoint `process`/`process_audio` はハングしない — busy/`Failed`/未着は `Err` bypass または `Ok(CONTINUE)`+定義済み出力（直前受信 or 無音）で、`out` は `Err` 時に不変更
- `arm_apply` は RT パスで alloc しない（`last_output` は事前 `reserve`・`resize` は常に capacity 内）— drain した arm は全て `Retirer` 経由で non-RT `DropCollector` へ
- leg は `return_rx` drain で復元 — swap が走らない session でも `&mut` entry 呼出し毎に再 attach 可能
- `EndpointUnavailable` は wire に乗らない local-only — encode 失敗は `out` 不変（all-or-nothing）
- `PluginEndpoint` は `unsafe impl` なしに `Send` — `!Send` 制御状態（`PluginHost`/`L1Instance`/裸 `ErasedPlugin`/shm segment）を含まない

## Non-goals

- グラフ経由の per-block 駆動・gain 正当性・mock device E2E — `plugin-graph-e2e-tests.md` が担当
- `rt_` 命名の `assert_no_alloc` 実測（`rt_endpoint_*`/`rt_plan_*`）と plan 退役の RT-drop 非発生 — `plugin-rt-retirement-tests.md` が担当
- L1 wire codec 自体・子プロセス制御面（spawn/handshake/クラッシュ検出の冪等性）— issue-76 test-spec（`plugin-l1-wire-codec-tests`/`plugin-l1-proxy-contract-tests`）の既存契約が担保
- `process_audio` の一律複写コスト最適化・多 bus/padding・role メタデータ — v1 範囲外（design deferred）
- 実規格アダプタ（CLAP/VST3/OFX）経由の実プラグイン — Issue #114
- video plugin の RT/アウトプロセス経路 — NONRT 境界の明文化まで（AC10）
- standalone `InProcessHost` への endpoint API 配線 — v1 は `PluginHost` のみ
- macOS/Linux でのローカル実証 — Windows ローカルのみ・3 OS matrix は CI `verify` ジョブに委譲（issue-76 と同じ platform notes）

## Risk

- verb 3 段 lock 形態の漏れ（pre-check/lock 内再読の欠落・`destroy` 例外の誤適用）→ `Failed→Activated` regression が構造的に可能になる — 本 spec の pin と unit test（poison fixture・verb-during-attach）で固定
- seed arm の誤実装（同 generation を apply してしまい装着簿記を壊す）→ 「unapplied retire」を明示
- `pending_arm` resend の `&mut`/`&self` tier 境界の誤り（`&self` query で drain しようとして `&mut` プリミティブに触れる・または `process` で destructor drain）→ tier 表を固定
- `EndpointUnavailable` が wildcard でなく個別 variant として wire に載る・partial frame を残す → encode 失敗の `out` 不変を negative test で固定
- `latency_frames` の旧式（`× block_frames / sample_rate`）残留 → 修正版式（`lookahead × activated max_frames`）を例で固定
