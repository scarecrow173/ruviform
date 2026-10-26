---
id: "TSPEC-034MrshMb3JSLSRiyAbWcK"
type: "test-spec"
status: "proposed"
title: "plugin グラフ E2E（L0/L1 endpoint → FnMut AudioNode → pull グラフ → mock device）テスト仕様"
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
    - "docs/test-specs/plugin-endpoint-contract-tests.md"
    - "docs/test-specs/plugin-rt-retirement-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/designs/audio-engine.md"
  derived-by: []
  verifies:
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/designs/engine-plugin-host-integration.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# plugin グラフ E2E（L0/L1 endpoint → FnMut AudioNode → pull グラフ → mock device）テスト仕様

> status: proposed — Issue #113 の spec AC1/AC3/AC9（実グラフ消費・構成面・L0/L1 graph E2E）と AC5/AC6 のグラフ経路部分を固定する。endpoint 面の単独契約は `plugin-endpoint-contract-tests.md`、`rt_` no-alloc 実測と退役観測は `plugin-rt-retirement-tests.md` が担当する。run 可能コマンドは task 側に定義する。

## Purpose

spec `docs/specs/engine-plugin-host-integration.md`（proposed・11 AC）と approved design `docs/designs/engine-plugin-host-integration.md` に基づき、**endpoint → graph 接着の end-to-end 受入契約**を固定する — (a) L0/L1 `PluginEndpoint` を capture した `Send` `FnMut` closure が `Box<dyn AudioNode>` としてグラフに push でき pull 駆動で per-block 音声が流れること、(b) `ruviform-audio` が plugin 型・依存を知らず構成面が closure 境界のみであること（AC3）、(c) `Session::open` → mock `OutputStream` callback 駆動の device-independent E2E（実デバイス依存なし）、(d) `Failed`/遅延未着/`Err` の defined 出力（pass-through or 無音）と source-position `fill(0.0)` 慣行、(e) node removal/plan-swap での退役と leg 再 attach の E2E 往復。テストは `crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs`（dev-dep `ruviform-audio`・prod 依存は非循環）に置く。グラフ接着の形が置換・再設計された場合に限り本 spec を retire する。

## Feature

`ruviform-plugin-host` + `ruviform-audio` の結合面: Send-capable mock plugin → `PluginHost`（`register_factory`/`register_out_of_process` + `execute(HostCommand::Instantiate{plugin_id})`）→ `prepare_endpoint` → `PluginEndpoint` を capture する `FnMut(&mut AudioBufferViewMut<'_, f32>, &RtContext)` + `Send` closure → `Graph::push(Box<dyn AudioNode>)`（既存メソッド名 `push` — `push_node` という名は存在しない）→ `PlanSwap`/`RtPlan` → `Session::open`（mock `DeviceEnumerator`/`Device`/`OutputStream`）→ callback 手動駆動の per-block `process`。L1 脚は `CARGO_BIN_EXE_l1-test-worker` で実子プロセスを spawn する（crate 内 `env!` — `l1_proxy_e2e` と同じ方式）。

## Rules

- **接着は `FnMut` blanket impl（AC1/AC3）**: `impl<F: FnMut(&mut AudioBufferViewMut<'_, f32>, &RtContext) + Send> AudioNode for F` — `move |out: &mut AudioBufferViewMut<'_, f32>, ctx: &RtContext| { out.data_mut().fill(0.0); let _ = endpoint.process_audio(out, EventList::empty(out.frames() as u32), None, ctx); }` 形の closure が `graph.push(Box::new(closure))`（既存 `Graph::push` — `push_node` という名は存在しない）で `Box<dyn AudioNode>` へ coerce する。`process_audio` は `Result<ProcessResult, HostError>` を返し `events: EventList<'_>`/`transport: Option<TransportInfo>`（両 `Copy`・by-value）+ `ctx: &RtContext` を受ける — v1 は `EventList::empty(block_frames)`（`block_frames: u32` — `event.rs` L192）/`None` transport。named newtype・plugin 型の trait ラップは不要 — consumer 配線の closure のみが接着面
- **engine 側の plugin 非露出（AC3）**: `ruviform-audio` の公開 API・`Cargo.toml` `[dependencies]` に plugin 型・plugin-host/plugin-core 依存がない — `cargo tree -p ruviform-audio` の DAG に plugin クレートが出現しないことを gate で監査（`mise run test` 成功だけでは検出しないため plan action 20 で明示監査）。テスト側は `ruviform-plugin-host` の dev-dep `ruviform-audio` のみ — prod 依存は非循環
- **L0 graph E2E（AC9・実 API 経路）**: Send-capable gain mock factory → `PluginHost::new()` → `host.register_factory(Box<dyn PluginFactory>)`（`plugin_id` を L0 に claim — `Route::InProcess` は private で `PluginHost::instantiate(InProcess)` という API は存在しない）→ `host.execute(HostCommand::Instantiate{plugin_id})` → `Initialize`→`Activate`→`StartProcessing`（`execute(HostCommand::*)` 経由）→ `prepare_endpoint` → closure を `graph.push` → `Session::open`（mock device・callback 手動駆動）→ 固定入力ブロックに対して出力が mock の定義済み gain で変換されることを sample 値で検証（流れただけでなくデータ正当性）・`latency_frames(h) == Some(0)`・endpoint `latency_frames() == 0`
- **L1 graph E2E（AC9・実 API 経路）**: `l1-test-worker` を `CARGO_BIN_EXE_l1-test-worker` で spawn（crate 内 `env!`）→ `host.register_out_of_process(OutOfProcessSpec{..})`（`plugin_id` を L1 に claim — `Result<(), HostError>`）→ `host.execute(HostCommand::Instantiate{plugin_id})` → lifecycle（`execute(HostCommand::*)` 経由）→ `prepare_endpoint` → closure を `graph.push` → `Session::open` → callback 駆動で per-block `process`（child 側 mock plugin の定義済み変換）→ `Destroy` — device 非依存（mock `OutputStream`）
- **source-position `fill(0.0)` 慣行（AC5）**: source 位置の closure は `endpoint.process_audio` の前に `out.data_mut().fill(0.0)` を呼ぶ adapter 契約 — `Err`（bound clip・非 `Processing`・busy bypass）時に `out` が上流 stale でなく無音になることを E2E で pin（transform 位置は `Err` 不変更 = 上流 bypass 経路のまま）
- **failure/フォールバックの E2E（AC5）**: L1 `kill_child` → `Failed` — `InstanceFailed` 通知は exactly once・以後の callback 出力は無音（pass-through 規則に従い `Ok(CONTINUE)`）・`process` 以後のコマンド（`Destroy` 除く）は `Err(Failed)`。遅延未着ブロック（control 側 `process` 未駆動）は pass-through（直前受信 else 無音）— エラーでなく defined 出力
- **latency の観測（AC6）**: L1 `latency_frames(h) == Some(lookahead_blocks × activated max_frames)`（`OutOfProcessSpec{lookahead_blocks:2}` × `Activate{max_frames:512}` = `Some(1024)`）・endpoint 側も同一値・未 `Activate` は `None` — `frames` 整数・秒換算 `frames / sample_rate` の doc 明記
- **plan-swap/removal の退役 E2E（AC8 結合面）**: endpoint ノードを含む `GraphPlan` の `Session::swap(&self)`・node removal で旧 `Arc<GraphPlan>` が `Retirer` 経由で退役 — DropFlag probe を `GraphPlan`/node/`SendCore` 側に仕込み、RT pull 経路（`RtPlan::active`/`RtPlan::drop`）では drop せず `Session::drain_retired()`/`Session` 終了（teardown）後に立つこと・`Session::retired_overflows() == 0`（`Session` 経由の観測点 — T5 で `Session` passthrough を budget）。endpoint drop（plan drop 連鎖）→ L1 leg は `return_tx` 経由で host に還り再 attach 可能であることを E2E で往復検証
- **lifecycle 全体の E2E**: attach 中の `StopProcessing`/`Deactivate`/`Activate`（bound 内再 arm — arm channel 経由）/`Destroy` がグラフ駆動を止めず defined 挙動で動く — `Deactivate` 後の callback は `Err` bypass または定義済み出力、再 `Activate`+`StartProcessing` で処理再開
- **無 audio-device 依存**: 全 E2E は mock `Device`/`OutputStream`（callback をテストスレッドから手動駆動 — `ruviform-audio` 側で公開される callback 取得口か test seam を使う）— cpal/実デバイスに依存しない。CI `verify` ジョブ（`cargo test --workspace`）で 3 OS 実行可能

## Examples

- **L0 往復**: gain=2.0 の Send mock → endpoint attach → graph へ closure → `Session::open` → mock callback に 512-frame×2ch 入力ブロック（1.0 値）を与える → 出力が 2.0 — `latency_frames`/`bounds()`/`max_frames()` が期待値
- **L1 往復 + クラッシュ**: l1-test-worker → `prepare_endpoint` → graph → 数ブロック正常駆動 → `host.kill_child(h)` → 次ブロックから無音 + `try_recv_notification` が `InstanceFailed` を 1 回だけ返す → `destroy(h)` → `Ok`
- **graph-attached verb**: endpoint attach 中に `set_param`/`SaveState`/`StopProcessing` → 全て `Ok`（L0 3 段 lock・L1 wire verb）— callback は verb 実行中に busy bypass し得るがハングしない
- **source-position 慣行**: bound clip を故意に破る closure（`channels != bound.channels` の `out`）で `Err(Prepare(UnsupportedConfig))` → `fill(0.0)` 済みの `out` は無音のまま（上流 stale を残さない）
- **swap 退役**: endpoint ノードを含む plan を `Session::swap` で差替 → 旧 plan の DropFlag が drain 前に立たないこと → `Session::drain_retired()` → DropFlag 立つ・`Session::retired_overflows()==0` → node removal plan でも同様
- **leg 還流**: endpoint を drop（plan drop 連鎖を含む）→ 何もせず `prepare_endpoint` → `Err(EndpointUnavailable)`（leg 未復元）→ `try_recv_notification` 等の `&mut` entry → `prepare_endpoint` → `Ok`（leg が `return_rx` drain で還流）
- **re-activation bound gate**: attach 中に `Activate{max_frames: bound.max_frames+1}` → `Err(Prepare(UnsupportedConfig))`（L1 は control 側 gate・arm 生成せず）

## Guarantees

- L0/L1 双方で `prepare_endpoint` → closure → `Graph`/`Session` の同じ接着面が動く — engine 側は plugin 差を知らない（`EndpointInner` dispatch のみ）
- `ruviform-audio` の公開面・依存木に plugin 由来の型・クレートが出ない（AC3 — DAG 監査で機械検証）
- mock device 駆動で実デバイス・cpal なしに per-block 音声正当性を検証できる
- `Failed`/遅延未着/ bound clip 違反は panic でもハングでも stale 出力でもなく defined 出力（無音 or pass-through）になる — `Err` 時 `out` 不変更（transform）・source 位置は `fill(0.0)` で無音
- plan swap/removal で endpoint・`SendCore`・`RtLeg`・`Arc<Staging>` が RT callback スレッドで drop しない — `Session::drain_retired()`/teardown で回収・`Session::retired_overflows()==0`
- L1 leg は endpoint drop 後に `&mut` entry で host へ還流し再 attach できる（swap を走らせなくても）

## Non-goals

- `rt_` 命名の `assert_no_alloc` 実測（`rt_endpoint_*`/`rt_plan_*`/`rt_fn_node_*`）・collector drain の RT-drop 非発生証明 — `plugin-rt-retirement-tests.md` が担当
- endpoint 公開面の単独契約（拒否表・3 段 lock・arm 規則・encode）— `plugin-endpoint-contract-tests.md` が担当
- wire codec・子プロセス制御面の低水準検証 — issue-76 test-spec の既存契約が担保
- 実規格アダプタ経由のプラグイン・プラグイン chain/routing・多 bus・GUI 配線 — Issue #114 / 範囲外
- イベント・transport のグラフ配線（v1 は `EventList::empty`/`None` transport）・music-time 統合
- `process_audio` 複写コスト・role メタデータ・`FnMut` 以外のノード形 — design deferred
- macOS/Linux ローカル実証 — 3 OS matrix は CI `verify` ジョブ（`cargo test --workspace` に E2E を含む）

## Risk

- `FnMut` impl なしに closure が `Box<dyn AudioNode>` へ coerce しない（接地面の根本）→ compile-time + E2E で固定
- 実デバイス依存のテスト形を採ると CI/他 OS で実行不能になる → mock `OutputStream` の手動 callback 駆動を pin
- L1 E2E が host crate 外から `l1-test-worker` を見つけられない → `CARGO_BIN_EXE_l1-test-worker` を host crate 内に閉じ込める
- `fill(0.0)` 慣行が closure 側の暗黙ルールに留まると transform 位置と取り違える → doc + E2E で pin
- 退役の RT-drop が E2E レベルで検出できない → DropFlag probe + `retired_overflows` の gate を rt-retirement spec 側と分担
