---
id: "TSPEC-034Mrsijk0b3tEOMrMG0MN"
type: "test-spec"
status: "proposed"
title: "plugin/audio RT 契約 + 退役（assert_no_alloc・arm_apply・leg 返却・plan 退役 drain）テスト仕様"
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
    - "docs/test-specs/plugin-graph-e2e-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/designs/audio-engine.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
  derived-by: []
  verifies:
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/adr/plugin-rt-ownership-split.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# plugin/audio RT 契約 + 退役（assert_no_alloc・arm_apply・leg 返却・plan 退役 drain）テスト仕様

> status: proposed — Issue #113 の spec AC7（`rt_` no-alloc 実測）と AC8（退役の非 RT drop・`retired_overflows` 観測）を固定する。endpoint 公開面の契約は `plugin-endpoint-contract-tests.md`、グラフ E2E 正当性は `plugin-graph-e2e-tests.md` が担当する。run 可能コマンドは task 側に定義する。

## Purpose

spec `docs/specs/engine-plugin-host-integration.md`（proposed・11 AC）と approved design `docs/designs/engine-plugin-host-integration.md`・ADR `docs/adr/plugin-rt-ownership-split.md` に基づき、**RT 実行面と退役面の受入契約**を固定する — (a) `rt_` 命名の `assert_no_alloc` 測定窓内で endpoint `process`/`process_audio`・`RtLeg::arm_apply`・`RtPlan::active`/`swap`-退役参照経路・`FnMut` ノード pull を 0 alloc で駆動すること（fixture・warm-up は窓外）、(b) 退役 `GraphPlan`（node/endpoint/`Mutex<SendCore>`/`RtLeg` scratch/`Arc<Staging>`/SPSC endpoint）の dealloc が RT callback スレッドで発生せず `PlanSwap`/`Session` の全 non-RT entry point で drain されること、(c) `retired_overflows()` が leak-on-full 飽和の観測点として 0 を返すこと、(d) L1 arm channel/`Retirer`/`return_tx` の bounded 性と drain-before-early-return の tier 境界。RT 測定方式（`rt_`+`assert_no_alloc`）または退役方式（`Retirer`/`DropCollector` seam）が置換された場合に限り本 spec を retire する。

## Feature

2 系統の測定面 — **endpoint/L0-L1 RT 面**: `PluginEndpoint::{process, process_audio}`（L0 `try_lock`+`catch_unwind` 経路・L1 leg `process` + arm drain）・`RtLeg::arm_apply`・endpoint `Drop` の `return_tx.try_push`（cap 1）・host `process`/`&mut` entry の drain-before-early-return 順序。**audio 退役面**: `RtPlan::active`（`current` slot の `try_read` 成功で displaced `cached` を `Arc::ptr_eq` で判定し実差替のみ `retirer.retire`）・`impl Drop for RtPlan`（`self.cached.take()` — `current` slot 側を touch しない）・`PlanSwap` の `Arc<Mutex<DropCollector<Arc<GraphPlan>>>>`（`Clone`/公開形維持）+ `swap(&self)`/`drain_retired(&self)`/`retired_overflows(&self)`/`with_retire_capacity`・`Session::drain_retired(&self)`/`Session::retired_overflows(&self)` passthrough（T5 で budget — `Session` 経由の観測点）・`Session` teardown の field 順序（collector が retirer より後に死ぬ）。

## Rules

- **`rt_` 命名 + `assert_no_alloc` 測定窓**: 全 RT テストは `rt_endpoint_*`/`rt_plan_*`/`rt_fn_node_*` プレフィックス — fixture・warm-up（`prepare_endpoint`・初回 callback・`RtLeg` new・arm push）は**窓外**に置き、`assert_no_alloc(|| …)` 窓内で測定対象経路のみ駆動（Issue #22 規約 — `alloc_counter` API セマンティクスは変更せず併存）。`test-rt` は `rt_` 名フィルタ — 0 件一致はスキップ相当なので plan/task 側で実一致件数を記録する
- **`rt_endpoint_l0_process_no_alloc`**: Send mock で endpoint attach → 窓内で `endpoint.process`/`process_audio` を複数ブロック駆動 — `try_lock`（busy→`Err(EndpointUnavailable)` も alloc なし）・`catch_unwind`（パス登録のみ・alloc なし）・無音書込み込みで 0 alloc
- **`rt_endpoint_l1_process_no_alloc`**: `RtLeg`（child なし・`Staging` ring 直結 fixture）を持つ endpoint で `process` — arm drain（generation 単調 apply + `Retirer.retire`）・in-bus-first gate・`last_output` 書込み込みで 0 alloc（`arm_apply` の `resize` は事前 `reserve` 済み capacity 内 — これ自体が pin）
- **`rt_endpoint_arm_apply_no_alloc`**: 別 generation の `Arc<Staging>` を arm channel 経由で `arm_apply` — 窓内 0 alloc・旧 ring は `Retirer` 行き（`retire` 自体は push のみ — destructor は `DropCollector` 側）
- **`rt_endpoint_busy_bypass`**: L0 `try_lock` 失敗（別スレッドが guard 保持）・poisoned・非 `Processing` mirror で定義済み出力が panic/ハング/未定義動作なく返る — `Err` は alloc を伴わない
- **`rt_plan_active_no_alloc`/`rt_plan_swap_retire_no_alloc`**: `RtPlan::active` で `try_read` 成功しても `ptr_eq` 同一なら retire しない（`try_read` 失敗時は `cached` 不変・displaced なし）・差替 plan を `active` で読むと displaced `cached`（旧 `Arc<GraphPlan>`）が `retirer.retire` される（RT 側は push のみ）— DropFlag probe は `Session::drain_retired()`/`teardown` まで立たない・`Drop for RtPlan` も `self.cached.take()` を `retirer.retire` する（`current` slot 側を touch せず RT drop 回避）
- **`rt_fn_node_pull_no_alloc`**: `FnMut` closure ノードを持つ `GraphPlan` の `RtPlan` pull 経路を窓内駆動 — `pull` → closure 呼出し → `process_audio` が alloc なし
- **退役の非 RT drop 保証（AC8）**: plan-swap/node removal/removal-plan で退役した `GraphPlan` 内の node（=`PluginEndpoint`→L0 `Arc<Mutex<SendCore>>` + epoch CAS・L1 `RtLeg` + `return_tx.try_push`）・`Arc<Staging>` arm・SPSC endpoint は**全て** collector スレッドの `DropCollector` drain か collector drop で破棄 — RT callback 経路（`RtPlan::active`・pull・`RtPlan::drop`）で destructor が走らないことを DropFlag + `assert_no_alloc` で検証
- **drain-on-every-non-RT-entry**: `PlanSwap::swap(&self)`・`PlanSwap::drain_retired(&self)`・`Session::swap(&self)`・`Session::drain_retired(&self)`・`Session` 終了/teardown で collector が drain される（`&self` シグネチャ — non-RT entry）— drain 後に `PlanSwap::retired_overflows(&self)`/`Session::retired_overflows(&self)` が 0 を返す（leak-on-full `mem::forget` 飽和の観測点 — AC・`Session` 経由は T5 passthrough で budget）。swap を走らせない session でも teardown で退役物が回収される
- **tier 境界（`&self` defined non-drain / `&mut` drain / wait-free `process`）**: `lifecycle_state`/`latency_frames`/`L1Instance::try_recv_notification`（`&self`）は drain しない観測点 — `&mut` entry（`execute`/`prepare_endpoint`/`destroy`/composite `try_recv_notification`）でのみ `pending_arm` resend・`return_rx` pop・`arm_collector.drain()` が走る — host `process` は `arm_collector.drain()` を除く wait-free 部分のみで `rt.is_none()` early-return より前に drain（destructor 実行を RT 契約経路に入れない）
- **bounded 性**: arm channel cap 2・`Retirer` cap 4・`return_tx` cap 1 — endpoint drain は 1 pass ≤ arm_capacity+1（=3）pop・`Full` は `pending_arm`（latest-wins）/leg drop に倒す。capacity 超過の arm は適用せず retire + `Err(Ipc)` バックストップ（**`Err(Ipc)` は内部処理で収束し `process`/`process_audio` の公開 `Err` として露出しない** — 公開面の dims 拒否は `armed_frame_capacity` pre-gate の `Err(Prepare(UnsupportedConfig))`）

## Examples

- **L0 no-alloc 窓**: 窓外で `attach`+warm-up → 窓内で 512-frame `process_audio` ×N — `assert_no_alloc` が abort しない
- **arm apply no-alloc**: 窓外で leg+arm 準備 → 窓内で `arm_rx` pop → `arm_apply`（旧 ring `retire` push）→ 0 alloc・新 generation が装着される
- **busy/poisoned bypass**: 別スレッドで `core.lock()` guard を保持 → RT スレッドで `process` → `Err(EndpointUnavailable)` を窓内で返す（alloc なし・ハングなし）
- **plan 退役往復**: `Session::swap(&self)` で plan A→B に差替 → `RtPlan::active` が B を返す → 旧 A の DropFlag は未成立 → `Session::drain_retired()` → DropFlag 成立・`Session::retired_overflows()==0`
- **`Drop for RtPlan`**: `RtPlan` を drop → `self.cached.take()` の `Arc<GraphPlan>` が `retirer.retire` される（`current` slot 側は touch しない）— RT callback とみなした窓内で drop しても destructor は走らない
- **leg 還流の RT 非 drop**: endpoint 入り node を含む plan を退役 → `drain_retired` で endpoint drop → `RtLeg` が `return_tx` へ — host `&mut` entry で `self.rt` 復元 → 再 attach `Ok`

## Guarantees

- 測定窓内の endpoint `process`/`process_audio`・`arm_apply`・plan `active`/退役参照経路は 0 alloc（`assert_no_alloc` abort しない）— fixture/warm-up alloc は窓外
- 退役 `GraphPlan` 連鎖（node→endpoint→`SendCore`/`RtLeg`/`Arc<Staging>`/SPSC endpoint）の destructor は collector スレッドで走る — RT callback では `retire` push のみ
- `retired_overflows()`（`PlanSwap`/`Session` passthrough）は drain 後 0 — leak-on-full 飽和が起きても観測点が残る
- `&self` query は drain しない（defined non-drain）・host `process` は `arm_collector.drain()` を走らせない（destructor は RT 契約違反）— 境界が test で pin される
- 全て mock/`env!`-closed（`l1-test-worker` は crate 内）— cpal・実デバイス・OS 特有 API に依存せず CI 3 OS で実行可能

## Non-goals

- endpoint 公開面の拒否表・3 段 lock 形態・`encode`/`encode_host_error` の fallible 化 — `plugin-endpoint-contract-tests.md` が担当（本 spec は RT/退役面のみ）
- グラフ経由の音声正当性・mock device E2E — `plugin-graph-e2e-tests.md` が担当
- `ruviform-rt` の `alloc_counter` API セマンティクス変更 — `assert_no_alloc` は検証用に使い既存機構は併存
- `ruviform-rt::slot`（snapshot）採用可否・`PlanSwap` の snapshot 全面移行 — design deferred（初回 `read` の debt-node alloc が RT 初回適用に混入するため不採の判断をここで再議しない）
- 実規格アダプタ・video RT 経路・自動再起動 — 範囲外
- `rt_` フィルタ 0 件時の検出 — plan/task 側で実一致件数の記録を要求（本 spec 自体は契約のみ）

## Risk

- `arm_apply`/`process` の窓内 alloc（`resize` の realloc・`catch_unwind` の payload box・`Retirer`/`try_push` 内蔵 alloc）が見落とされる → `assert_no_alloc` で機械検出（fixture 確保は窓外）
- 退役 Drop が RT callback で走っても単体では検出しにくい → DropFlag probe + drain 後立証 + `retired_overflows()==0` の三組で固定
- `&self`/`&mut`/wait-free の tier 境界が実装で崩れる（`&self` で `&mut` プリミティブに触れる・`process` で destructor drain）→ 観測点テストで pin
- `rt_` 名フィルタが 0 件一致で成功に見える → plan/task の「実一致件数記録」規則で補完
