---
id: "TASK-034MrsUVP3NUi1Nrap9gyd"
type: "task"
status: "done"
title: "audio シーム（AudioNode FnMut impl + plan.rs 退役 — Retirer/ptr_eq/Drop for RtPlan/drain）"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-audio/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio/src/graph.rs"
      - type: "file"
        path: "crates/ruviform-audio/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio/src/plan.rs"
      - type: "file"
        path: "crates/ruviform-audio/src/session.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  implemented-by:
    - "docs/impl/ir/issue-113-audio-fnmut-node-and-plan-retirement-task-5-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-rt-retirement-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# audio シーム（AudioNode FnMut impl + plan.rs 退役 — Retirer/ptr_eq/Drop for RtPlan/drain）

## Work

Plan actions 14–16。`ruviform-audio` 側の接着面を実装する — `AudioNode` の `FnMut` blanket impl（1 本のみ・plugin 型を名指さない）と `plan.rs` の退役シーム（`RtPlan.retirer`・`active()` の `ptr_eq` retire・`Drop for RtPlan`・`PlanSwap` collector + 全 non-RT entry point drain + `drain_retired`/`retired_overflows`/`with_retire_capacity`）。**`ruviform-audio` へ plugin 依存・plugin 型の露出はゼロ**（AC3 — `ruviform-rt` は既存依存の再利用・新規外部依存なし）。

- **`graph.rs` `FnMut` blanket impl**: `impl<F> AudioNode for F where F: FnMut(&mut AudioBufferViewMut<'_, f32>, &RtContext) + Send { fn pull(&mut self, out: &mut AudioBufferViewMut<'_, f32>, ctx: &RtContext) { self(out, ctx) } }` — consumer 配線の `Send` closure が `Box<dyn AudioNode>` へ coerce する。named newtype・plugin 型 trait ラップは不採（design 確定）。単体テスト: `Send` capture の closure が `graph.push(Box::new(closure))` で push でき pull で呼ばれる（`Graph` の既存公開メソッドは `push` — `push_node` という名前は存在しない）。**Send 証拠の方法は `static_assertions` を採用**（`assert_impl_all!` — 通常の `expect_send` 型関数は generic `F: FnMut` に対して `FnMut` impl の有無を compile 時に証明できないため不適）— `static_assertions` を `ruviform-audio` の **dev-dependency として追加**（`assert_impl_all!(GraphPlan: Send)`・`assert_impl_all!(RtPlan: Send)`・`assert_not_impl_any` 相当の負側も同 crate — trybuild なし）。`#[cfg(test)]` inline の `expect_send::<T>()` 型関数は具象 `L0Endpoint`/`L1Endpoint`/`GraphPlan`/`RtPlan`/callback capture に併用可だが、**enum variant 自体は型ではない**（variant 名を `expect_send` の引数に書かない）・crate-private 具象型の assert は外部 `tests/` に置かない（`static_assertions` の assert は `src/` 内 `#[cfg(test)]` または lib に配置）
- **`plan.rs` `RtPlan` 退役（`RtPlan.cached` が退役対象 — 現行 shape に正確に合わせる）**: 現行 `PlanSwap { current: Arc<RwLock<Arc<GraphPlan>>> }`（共有 slot）と `RtPlan { current: Arc<RwLock<Arc<GraphPlan>>>（slot の clone）, cached: Option<Arc<GraphPlan>> }` の構造 — `RtPlan.active(&mut self)` は `self.current.try_read()` 成功で `self.cached = Some(Arc::clone(&plan))` を行い `self.cached.clone()` を返す。**退役対象は displaced `cached`**（`active()` が新 plan を読んで `self.cached` を差し替えるときの旧 `cached`）— `current`（共有 slot）は `PlanSwap` 側の publish 先であり `RtPlan` が所有・retire する対象ではない。`RtPlan` に `retirer: Retirer<Arc<GraphPlan>>` を追加し、`active()` が `try_read` 成功時に `Some(old) = self.cached` を取り `!Arc::ptr_eq(&old, &new_plan)` なら `self.retirer.retire(old)`（同一 `Arc` の再読で retire しない — `try_read` 失敗時は `cached` 不変で displaced なし）。`impl Drop for RtPlan` — `self.retirer.retire(self.cached.take())`（`cached` が `Option` — `take` で取り、`current` slot 側は touch しない。`take` 後の `None` で二重 retire しない — RT callback 上での `GraphPlan` drop を回避）
- **`PlanSwap` collector**: `PlanSwap` に `retired: Arc<Mutex<DropCollector<Arc<GraphPlan>>>>` を追加（`Clone`/公開形を維持 — `PlanSwap` を clone した handle が同じ collector を共有）。`retire_channel::<Arc<GraphPlan>>(capacity)` で `(Retirer, DropCollector)` を mint し `RtPlan` へ `Retirer` を渡す。`PlanSwap::swap(&self)`（現行シグネチャは `&self` — non-RT &self entry）冒頭で `collector.drain()`。公開 API（全て `&self`）: `drain_retired(&self) -> usize`（drain した要素数）・`retired_overflows(&self) -> usize`（`Full` で `mem::forget` した leak-on-full 飽和の観測点）・`with_retire_capacity(usize)`（既定容量の調整口 — swap 頻度比例の小容量が既定）
- **drain-on-every-non-RT-entry**: `swap(&self)`・`drain_retired(&self)`・`Session` 終了/teardown で `collector.drain()` が走る — swap を走らせない session でも teardown で退役物が回収される。`RtPlan`/`PlanSwap` の collector を共有する `Arc<Mutex<DropCollector>>` は clone 全てが同じ `DropCollector` を指す
- **`Session` field 順序 + `Session` API**: `session.rs` の field 宣言順を確認 — `stream`（callback 所有）→ `swap`/`plan`（`RtPlan`・retirer 所有）→ `swap` の collector（`DropCollector` 所有）の順で **collector が retirer より後に drop** される安全順を doc で pin（`RtPlan::drop` で `retirer.retire` した退役物を `DropCollector` が回収するには、collector が生存している必要がある — field 順序で保証・doc で固定）。既存順序が適合するか確認のみ — 不適合なら field 宣言順を修正。**`Session` に `retired_overflows`/`drain_retired` の passthrough を budget する** — `Session` は現行 `swap(&self)` 公開のみで `PlanSwap` の退役観測点を露出していないため、`Session::retired_overflows(&self) -> usize`・`Session::drain_retired(&self) -> usize`（内部で `self.swap.*` を呼ぶ）を追加し、graph e2e テストから退役観測を可能にする（test-spec の観測点が `Session` 経由で到達できるように）
- **退役対象の構造**: `GraphPlan` 内の node（`Box<dyn AudioNode>` — endpoint closure 含む）→ `PluginEndpoint` → L0 `Arc<Mutex<SendCore>>`+`attached` CAS・L1 `RtLeg`+`return_tx` — plan drop 連鎖でこれらが collector スレッド上で drop される（RT callback では `retirer.retire` の push のみ）
- **`rt_` テスト（本 task 所有 — `plugin-rt-retirement-tests.md` の audio 面）**: `rt_plan_active_no_alloc`（同一 `Arc` の再読で retire しない・0 alloc）・`rt_plan_swap_retire_no_alloc`（差替 plan を `active` で読むと旧 `Arc` が `retire` — DropFlag probe は drain まで立たない）・`rt_fn_node_pull_no_alloc`（`FnMut` closure ノードの pull 経路が 0 alloc — fixture 確保は窓外）。`Drop for RtPlan` の `retire` も窓内で検証
- **非 RT-drop 保証の unit test（`rt_` 外の通常 test）**: DropFlag probe を `GraphPlan`/node に仕込み plan swap で RT 経路（`RtPlan::active`・`RtPlan::drop`）では立たず `drain_retired()`/teardown で立つ・`retired_overflows()==0`・swap なし session の teardown でも回収される

## Classification

- `normal-plan-task`（audio 側接着 + 退役シーム — plugin 側に依存しない独立 task・ただし単一レーン直列）

## Upstream Documents

- `docs/plans/implement-issue-113-engine-plugin-integration.md`（actions 14–16・Files/Ownership・依存の確定）
- `docs/designs/engine-plugin-host-integration.md`（graph seam・`FnMut` 接続の確定・`plan.rs` 退役シーム・`ptr_eq`・`Drop for RtPlan`・drain-on-every-entry・`retired_overflows`）
- `docs/designs/audio-engine.md`（Issue #20/#30 上流 — `AudioNode`/`Graph`/`GraphPlan`/`PlanSwap`/`RtPlan`/`Session` の既存構造）
- `docs/adr/audio-engine-snapshot-swap.md`（snapshot slot swap の確定 — `ptr_eq` はその補完）
- `docs/specs/engine-plugin-host-integration.md`（AC1・AC3・AC7・AC8 — 実グラフ消費・非露出・RT no-alloc・退役）

## Dependencies

- `docs/plans/implement-issue-113-engine-plugin-integration.md`（plugin 側 task への依存なし — 独立 task。単一レーンでは Task 4 後に実行）

## Affected Files

- `crates/ruviform-audio/src/graph.rs` — `FnMut` blanket impl 1 本 + `push` を使う単体テスト
- `crates/ruviform-audio/src/plan.rs` — `RtPlan.retirer` + `active()` の displaced `cached` `ptr_eq` retire + `Drop for RtPlan`（`self.cached.take()`）・`PlanSwap` collector（`Arc<Mutex<DropCollector<Arc<GraphPlan>>>>`）+ `swap(&self)` 冒頭 drain + `drain_retired(&self)`/`retired_overflows(&self)`/`with_retire_capacity` + `rt_`/unit テスト + `static_assertions` Send assert（`src/` 内）
- `crates/ruviform-audio/src/session.rs` — field 順序の確認 + doc pin（必要なら宣言順修正）+ **`Session::retired_overflows(&self)`/`Session::drain_retired(&self)` passthrough 追加**
- `crates/ruviform-audio/src/lib.rs` — `rt_` テストの配置（既存 `rt_` パターンと同じ `#[cfg(test)]` 内）+ 公開 API への `drain_retired`/`retired_overflows` 露出確認
- `crates/ruviform-audio/Cargo.toml` — `static_assertions` を **dev-dependency** として追加（新規外部依存は dev のみ・`[dependencies]` には追加しない — AC3 の `ruviform-audio` 非依存は維持）

## Done When

- [x] `impl<F: FnMut(&mut AudioBufferViewMut<'_, f32>, &RtContext) + Send> AudioNode for F` が存在し、`Send` capture の closure が `graph.push(Box::new(closure))` で push され `Box<dyn AudioNode>` へ coerce して pull で呼ばれる（`push` という既存メソッド名 — `push_node` は存在しない）
- [x] `static_assertions`（dev-dependency）で `assert_impl_all!(GraphPlan: Send)`/`assert_impl_all!(RtPlan: Send)` 等の Send 証拠 — `unsafe impl Send` なし・enum variant を型名として assert しない・crate-private 具象型の assert は `src/` 内配置
- [x] `RtPlan.retirer: Retirer<Arc<GraphPlan>>` が存在し、`active()` が `try_read` 成功で displaced `cached` を `!Arc::ptr_eq` なら retire（同一 `Arc` 再読・`try_read` 失敗で retire しない）・`Drop for RtPlan` が `self.cached.take()` を retire（`current` slot 側を touch せず `take` で二重 retire なし）— **deviation（review 確定修正）: `active()` は task doc の `self.cached.clone()`（`Option<Arc<GraphPlan>>`）ではなく `Option<&GraphPlan>`（cached slot の借用）を返す — 返却 `Arc` が RT 側で retain され最後の refcount で `GraphPlan` destructor が RT スレッド上で走り得る hole を静的に閉鎖。`session.rs` の call site は `drive(&self)` 経由で変更なく compile**
- [x] `PlanSwap` が `Arc<Mutex<DropCollector<Arc<GraphPlan>>>>` を持ち `Clone`/公開形を維持・`swap(&self)`/`drain_retired(&self)`/teardown で drain・`drain_retired(&self) -> usize`/`retired_overflows(&self) -> usize`/`with_retire_capacity` が公開される
- [x] `Session` field 順序が collector > retirer の安全順で doc pin される + `Session::retired_overflows(&self)`/`Session::drain_retired(&self)` passthrough が公開される
- [x] 退役 `GraphPlan`（node→endpoint→`SendCore`/`RtLeg`/`Arc<Staging>`/SPSC endpoint）の destructor が RT callback（`RtPlan::active`/pull/`RtPlan::drop`）で走らず collector drain/teardown で回収される（DropFlag probe で検証）
- [x] `ruviform-audio` の `[dependencies]`・公開 API に plugin 型・plugin-host/plugin-core 依存がない（`cargo tree` で出現しない — AC3。`static_assertions` は dev-dependency のみ）
- [x] narrow gate（下記 Verification）が exit 0 — `rt_plan_*`/`rt_fn_node_*` が実一致件数で通る

## Verification

- [x] `mise run check` — exit 0
- [x] `mise run lint` — exit 0（clippy `--all-targets -D warnings` clean — `#[must_use]`/`must_use_candidate` の pin 込み）
- [x] `mise run test` — exit 0（`cargo test --workspace` — **55 test binaries 全 pass**・既存 audio テスト無退行 + 新規 unit 4 件（`fn_mut_closure_node_pushes_and_pulls`/`plan_retirement_drops_via_collector_drain`/`plan_teardown_recovers_retired_without_swap`/`session_retire_passthrough_and_teardown`）+ `plan.rs` doctest 1 件）
- [x] `mise run test-rt` — exit 0・実一致件数: ruviform-audio で **6 件**（新規 4 件 = `rt_plan_active_no_alloc`/`rt_plan_swap_retire_no_alloc`/`rt_plan_drop_retire_no_alloc`/`rt_fn_node_pull_no_alloc` + 既存 `rt_callback_path_no_alloc`/`rt_denormal_flush_no_alloc`）— 0 件スキップなし
- [x] `mise exec -- cargo tree -p ruviform-audio` — DAG は `ruviform-audio-core`/`ruviform-media-core`/`ruviform-rt` のみで `ruviform-plugin-*` が出現しない（AC3 — Full Gate でも再監査）・dev-deps は `assert_no_alloc` + `static_assertions` のみ
- [x] 対象 test-spec: `docs/test-specs/plugin-rt-retirement-tests.md` — 規則「`rt_plan_*`/`rt_fn_node_*`」「退役の非 RT drop 保証」「drain-on-every-non-RT-entry」「`retired_overflows` 観測点」を担保

補足（task doc の Verification 範囲外・IR に記録）: `mise run fmt-check`/`mise run build` exit 0・`mise exec -- cargo test -p ruviform-audio` exit 0（lib **16** + doctest **1**）。**review-driven deviation**: `RtPlan::active()` が `Option<Arc<GraphPlan>>` ではなく `Option<&GraphPlan>`（cached slot の借用）を返す — 返却 `Arc` の retain による RT-drop hole を静的に閉鎖（B=adversarial review の ranked fix・`session.rs` は unchanged-compatible）。レビュー（A=APPROVE-WITH-FIXES・B=adversarial）の確定修正適用済み — 詳細は `docs/impl/ir/issue-113-audio-fnmut-node-and-plan-retirement-task-5-implementation.md`。
