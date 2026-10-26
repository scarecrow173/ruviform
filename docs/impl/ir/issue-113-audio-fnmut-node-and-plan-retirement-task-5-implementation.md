---
id: "IMPL-034c5ADLlSmjfCuxmyJJsQ"
type: "impl"
status: "completed"
title: "Issue #113 audio FnMut node + plan retirement — Task 5 implementation"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-113-audio-fnmut-node-and-plan-retirement-task-5-implementation.md"
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
      - type: "file"
        path: "docs/tasks/audio-fnmut-node-and-plan-retirement.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/audio-fnmut-node-and-plan-retirement.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #113 audio FnMut node + plan retirement — Task 5 implementation

## Summary

Issue #113 Task 5 完了 — plan actions 14–16（`ruviform-audio`
側の接着 + 退役シーム — plugin 側依存ゼロの独立 task）を全
範囲実施。(a) `AudioNode` の `FnMut` blanket impl 1 本（
`impl<F> AudioNode for F where F: FnMut(&mut
AudioBufferViewMut<'_, f32>, &RtContext) + Send` — `pull` は
`self(out, ctx)` に forward）、(b) `plan.rs` 退役シーム —
`RtPlan.retirer: Retirer<Arc<GraphPlan>>` + `active()` の
displaced `cached` `ptr_eq` retire + `impl Drop for RtPlan`
（`self.cached.take()`）+ `PlanSwap` の
`Arc<Mutex<DropCollector<Arc<GraphPlan>>>>` collector +
`swap`/`drain_retired`/teardown の drain-on-every-non-RT-entry +
`drain_retired`/`retired_overflows`/`with_retire_capacity` 公開、
(c) `Session` field 順序の doc pin + `retired_overflows`/
`drain_retired` passthrough。**review-driven deviation**:
`active()` は task doc の `self.cached.clone()`（`Option<Arc<
GraphPlan>>`）ではなく **`Option<&GraphPlan>`**（cached slot の
借用）を返す — 返却 `Arc` が RT 側で retain され最後の
refcount で `GraphPlan` destructor が callback thread 上で
走り得る hole を静的に閉鎖。新規テスト 8 件（rt_ 4 + unit 4）
+ `static_assertions` Send 証拠（dev-dep）。独立レビュー 2 件:
A=APPROVE-WITH-FIXES・B=adversarial（returned-Arc hole を ranked
fix として適用）。全 gate green 実 log。

## Context

approved plan `docs/plans/implement-issue-113-engine-plugin-integration.md`
の Task 5（`docs/tasks/audio-fnmut-node-and-plan-retirement.md`・
actions 14–16 — plugin 側 task への依存なしの独立 task、単一
レーンでは Task 4 後に実行）。upstream: approved design
`docs/designs/engine-plugin-host-integration.md`（graph seam・
`FnMut` 接続の確定・`plan.rs` 退役シーム・`ptr_eq`・`Drop for
RtPlan`・drain-on-every-entry・`retired_overflows`）・
`docs/designs/audio-engine.md`（Issue #20/#30 上流 —
`AudioNode`/`Graph`/`GraphPlan`/`PlanSwap`/`RtPlan`/`Session`
の既存構造）・ADR `docs/adr/audio-engine-snapshot-swap.md`
（snapshot slot swap の確定 — `ptr_eq` はその補完）・spec
`docs/specs/engine-plugin-host-integration.md`（AC1・AC3・
AC7・AC8 — 実グラフ消費・非露出・RT no-alloc・退役）・TSpec
`docs/test-specs/plugin-rt-retirement-tests.md`（proposed —
「`rt_plan_*`/`rt_fn_node_*`」「退役の非 RT drop 保証」
「drain-on-every-non-RT-entry」「`retired_overflows` 観測点」）。
消費側: Task 6 の graph e2e（endpoint closure が `FnMut` impl
経由で `Box<dyn AudioNode>` へ coerce し、退役観測を
`Session` passthrough 経由で到達）。

## Implementation

### Changes

- `crates/ruviform-audio/src/graph.rs`（+18 行）:
  - `impl<F> AudioNode for F where F: FnMut(&mut
    AudioBufferViewMut<'_, f32>, &RtContext) + Send` — blanket
    impl 1 本のみ・`fn pull(&mut self, out, ctx) { self(out, ctx)
    }` で forward。consumer 配線の `Send` closure が
    `Graph::push(Box::new(closure))` で `Box<dyn AudioNode>` へ
    coerce する seam。`Send` bound が契約 — RT thread へ渡れ
    ない capture は compile error（runtime でなく）。named
    newtype・plugin 型 trait ラップ不採（design 確定）。
- `crates/ruviform-audio/src/plan.rs`（+169/-26 行）:
  - `RtPlan.retirer: Retirer<Arc<GraphPlan>>` — RT 側は退役を
    wait-free push のみ行い、destructor は collector 側の
    drain で走る（RT callback 上での `GraphPlan` drop を構造
    排除）。`cached: Option<Arc<GraphPlan>>` は `pub(crate)` —
    in-crate test が `active()` の返却借用を slot 自身の `Arc`
    と `Arc::as_ptr` で照合するため。
  - **`RtPlan::active(&mut self) -> Option<&GraphPlan>`**
    （**review-driven deviation** — task doc は `self.cached.
    clone()`（`Option<Arc<GraphPlan>>`）を指定）: `try_read`
    成功で `self.cached.replace(Arc::clone(&plan))` の旧値を
    取り、`!Arc::ptr_eq(&old, &plan)` なら `retirer.retire(old)`
    （同一 `Arc` の再読で retire しない — `try_read` 失敗時は
    `cached` 不変で displaced なし）。返却は両経路とも
    `self.cached.as_deref()` — **借用であり `Arc` clone では
    ない**ため、呼び出し側が `Arc<GraphPlan>` を retain して
    RT スレッド上で最後の refcount drop（＝ `GraphPlan`
    destructor）を引き起こす経路を静的に閉鎖する（B=adversarial
    review の ranked fix — 後述）。`session.rs` の call site
    （`let Some(plan) = rt_plan.active() else { return };
    plan.drive(&mut out, &ctx);` — `drive(&self)` 経由）は
    signature 変更に対して **unchanged-compatible**。
  - `impl Drop for RtPlan` — `self.cached.take()` を
    `retirer.retire`（`cached` が `Option` — `take` 後の `None`
    で二重 retire せず、`current` 共有 slot は touch しない）。
    callback 上で `RtPlan` が drop されても `GraphPlan`
    destructor は走らない。drop order の契約（`RtPlan`/
    callback は最後の `PlanSwap` clone より先に drop —
    collector が retirer を生き延びる必要）を doc で pin・
    `Session` field 順序が in-crate 経路を保証。
  - `PlanSwap.retired: Arc<Mutex<DropCollector<Arc<GraphPlan>>>>
    ` — `retire_channel::<Arc<GraphPlan>>(capacity)` で
    `(Retirer, DropCollector)` を mint し `RtPlan` へ `Retirer`
    を渡す。`#[derive(Clone)]`・公開形維持（clone した handle
    全てが同じ collector を共有）。
  - `PlanSwap::new(plan)` = `with_retire_capacity(plan,
    DEFAULT_RETIRE_CAPACITY)`（**`DEFAULT_RETIRE_CAPACITY = 4`**
    — `RtPlan` の退役は `active()` 1 回あたり displaced 1 個 +
    drop 1 個、全 non-RT entry が drain する通常 cadence では
    pending は容量を大きく下回る）。`with_retire_capacity(plan,
    capacity)` — 調整口・`capacity == 0` は panic（always-full
    channel は全退役を leak する degenerate）。
  - `swap(&self)` — 冒頭で `self.drain_retired()`（
    **drain-at-entry** — publish 前に退役物を回収）。既存
    `&self` signature・`write()` の `PoisonError::into_inner`
    経路は不変。
  - `drain_retired(&self) -> usize` — 明示 drain 点（swap を
    走らせない運用の回収口）。`#[allow(clippy::
    must_use_candidate)]` — drain 自体が契約で count は
    advisory。`retired_overflows(&self) -> usize` — leak-on-full
    （`Full` で `mem::forget`）飽和の観測点。両者とも
    `retired.lock().unwrap_or_else(PoisonError::into_inner)` で
    collector に到達（lock poisoned でも回収は継続）。
  - **drain-under-lock の再入注意を doc で pin**（レビュー修正
    — 後述）: 退役 plan の destructor は collector mutex 保持
    中に走る — plan node の `Drop` impl が `PlanSwap` method を
    呼ぶと非再入 `Mutex` で deadlock。node `Drop` から
    `PlanSwap` へ再入しない契約を `PlanSwap` doc に明記。
  - `GraphPlan::drive` doc — `try_lock` の `Err` 全て（
    `WouldBlock` *および* `Poisoned`）で silent no-op である旨
    へ doc 拡張 + **poison 由来の永久無音を documented**
    （panicking node が guard 越し unwind で mutex を poison
    すると以後の全 `drive` が静黙 no-op — poisoned graph の
    再 enter を避ける意図的 degraded behavior。observed
    silence は transitive で `drive` 自体は何も書かず、
    device port の interleave 経路（`ruviform-audio-device`
    `convert.rs`）が未書き込みを zero-fill — follow-up 記録の
    裏付け）。
  - `PlanSwap::drain_retired` doctest（`rt_plan.active()` →
    `drain_retired()==0`・`retired_overflows()==0` —
    publish 中は退役・leak なしの pin）。
- `crates/ruviform-audio/src/session.rs`（+34 行）:
  - **field 順序の doc pin** — `stream`（callback 所有 →
    `RtPlan`・`Retirer` 所有）が `swap`（`DropCollector` 所有）
    より先に drop される宣言順を load-bearing として `Session`
    doc に明記（`RtPlan::drop` の retire が必ず生きた
    collector に着弾し、collector drop 時に残っていた plan
    の destructor は non-RT teardown スレッドで走る）。
    既存宣言順が既に適合 — 宣言順の変更なし・pin のみ。
  - `Session::drain_retired(&self) -> usize`・
    `Session::retired_overflows(&self) -> usize` passthrough
    （`self.swap.*` を呼ぶ — `Session` 経由の退役観測点を公開
    し graph e2e test-spec の観測点に到達可能に）。
- `crates/ruviform-audio/src/lib.rs`（+225/-3 行）: crate doc
  整合（retired plan は `ruviform-rt` retire channel 経由で
  non-RT 側へ遅延 — `PlanSwap::drain_retired`/
  `retired_overflows`・`RtContext` の qualified link）+
  `#[cfg(test)]` 新規 8 件 + `static_assertions`:
  - Send 証拠（compile-time・`src/` 内配置）:
    `assert_impl_all!(GraphPlan: Send, Sync)`（`DropCollector`
    が `Arc<GraphPlan>` を thread 越し受領するため `Sync` も
    必要）・`assert_impl_all!(RtPlan: Send)`・
    `assert_impl_all!(PlanSwap: Send, Sync)`・
    `assert_impl_all!(Session: Send)`・`assert_impl_all!(
    OutputDataCallback: Send)` + 負側 pin
    `assert_not_impl_any!(Rc<u8>: Send, Sync)`・
    `assert_not_impl_any!(RefCell<u8>: Sync)` — `expect_send`
    型関数では負側を表現不能なため `static_assertions` を採用
    （task doc 確定）。`unsafe impl Send` なし・全て構造型。
  - `DropFlagNode`/`flagged_plan` fixture — plan destructor の
    **実行地点**を probe する `Arc<AtomicBool>` flag。
  - unit 4 件: `fn_mut_closure_node_pushes_and_pulls`（`Send`
    closure が `graph.push(Box::new(..))` で coerce し pull で
    呼ばれ plane に書く）・`plan_retirement_drops_via_collector_
    drain`（swap で displaced `cached` が retire queue へ —
    flag は drain まで立たず、`swap` 自体の drain-at-entry で
    回収 + clone した `PlanSwap` handle が同一 collector を
    共有）・`plan_teardown_recovers_retired_without_swap`（swap
    なしでも `RtPlan::drop` が queue し collector drop で
    teardown 回収 — field 順序の実証）・`session_retire_
    passthrough_and_teardown`（`Session::swap` → callback 経由
    retire → `Session::drain_retired`/`retired_overflows`
    passthrough + swap なしでも session teardown で回収）。
  - rt_ 4 件（`assert_no_alloc` 窓内・fixture/warm-up は窓外 —
    `rt_plan_drop_retire_no_alloc` は `drop(rt)` 自体を窓内で
    検証）: `rt_plan_active_no_alloc`（同一 `Arc` 再読で
    retire しない + 返却 borrow が cached slot の `Arc`
    pointee と `Arc::as_ptr` で一致 — borrow 保持は次の
    `&mut self` call を阻むため raw pointer のみ保持する形で
    pin）・`rt_plan_swap_retire_no_alloc`（差替 plan で旧
    `cached` が retire — destructor は `drain_retired` まで
    立たない）・`rt_plan_drop_retire_no_alloc`（`Drop` の
    retire が 0 alloc・`current` slot 不 touch）・
    `rt_fn_node_pull_no_alloc`（`active` → `drive` → FnMut
    closure の pull 経路が 0 alloc）。
- `crates/ruviform-audio/Cargo.toml`: `static_assertions = "1.1"`
  を **dev-dependency** に追加（`[dependencies]` には追加
  しない — AC3 の `ruviform-audio` 非依存を維持。comment で
  `expect_send` 型関数が負側を表現不能な理由を記録）。
- `Cargo.lock` — `ruviform-audio` の `static_assertions` entry。
  **lockfile 自体は T2 commit（`dca7b82`）に既着** — 単一
  file で分割不能のため deviation として記録（T2 IR 側に同
  note — 本 commit の diff には Cargo.lock が含まれない）。
- `docs/tasks/audio-fnmut-node-and-plan-retirement.md` — status
  `done`・`implemented-by` 本 IR・changes 記録・checkbox 全
  tick + 補足 line。
- `docs/tasks/README.md`・`docs/impl/ir/README.md` — status/索引
  同期。

### Review findings and resolutions（独立レビュー 2 件 — A=APPROVE-WITH-FIXES・B=adversarial — 確定修正を全て適用）

- **`active()` の返却 `Arc` retain hole（fixed — B adversarial
  ranked fix・deviation）**: task doc どおりの `self.cached.
  clone()` 返却（`Option<Arc<GraphPlan>>`）では、呼出し側が
  返却 `Arc` を次の `active()` call を跨いで retain できて
  しまう — その `Arc` が最後の refcount で RT callback 上に
  drop されると `GraphPlan` destructor（node→endpoint→
  `SendCore`/`RtLeg` の連鎖）が RT スレッド上で走る退役契約
  違反の hole。返却を `self.cached.as_deref()`（`Option<&
  GraphPlan>` — cached slot の借用）へ変更し、borrow が次の
  `&mut self` call で終わる lifetime rule により hole を
  **静的に閉鎖**。`session.rs` の callback call site（
  `plan.drive(&mut out, &ctx)` — `drive(&self)` 経由）は変更
  なしで compile（unchanged-compatible）。`cached` field は
  in-crate test の `Arc::as_ptr` 照合用に `pub(crate)` 化。
- **drain-under-lock の再入注意（fixed — B 指摘）**: 退役 plan
  の destructor は collector `Mutex` 保持中に走る — plan
  node の `Drop` が `PlanSwap` method（`swap`/`drain_retired`/
  `retired_overflows`）を呼ぶと deadlock。node `Drop` から
  `PlanSwap` へ再入しない契約を `PlanSwap` doc に明記。
- **`#[must_use]`/`must_use_candidate` lint pin（fixed — A
  probable-lint 指摘）**: `drain_retired`（`PlanSwap`・
  `Session` 双方）は drain 自体が契約で戻り値は advisory の
  ため `#[allow(clippy::must_use_candidate)]` を明示付与。
  `retired_overflows`・`bounds`/`latency_frames` 相当の accessor
  側は `#[must_use]`。attribute の意図を comment で pin。
- **doc pin 一式（fixed — A 指摘）**: `PlanSwap`/`RtPlan`/
  `Session` の退役契約 doc（retirer→collector の所有分割・
  drop order 契約・leak-on-full 語義・field 順序の
  load-bearing 明記）+ `GraphPlan::drive` の `Poisoned` 挙動
  doc 拡張（永久無音 — follow-up 化）+ crate doc 整合。

### Decisions During Implementation

- **`active()` は `Option<&GraphPlan>` を返す**（deviation・
  review 確定修正 — 上記）。task doc の `self.cached.clone()`
  形を変更した唯一の semantic deviation — signature のみで
  `session.rs` 等の call site は無修正。
- **`DEFAULT_RETIRE_CAPACITY = 4`**（task doc「既定容量は
  swap 頻度比例の小容量」に従った具体値）: `active()` 1 回の
  退役は displaced `cached` 最大 1 個で drain は全 non-RT
  entry にあるため、pending が 4 を超えるのは swap を全く
  行わない長期運用の drain 間隔のみ — `with_retire_capacity`
  で調整可能。`capacity == 0` は panic（degenerate）。
- **`drain_retired`/`retired_overflows` は `&self`** — 現行
  `PlanSwap::swap(&self)`/`Session::swap(&self)` signature に
  整合（plan の確定形）。collector を `Arc<Mutex<…>>` に置く
  のは `Clone` handle 全てが同一 collector を共有するため。
- **`swap` の drain は冒頭（publish 前）** — drain-at-entry:
  退役物の回収が publish 成功に依存しない（drain で走る
  destructor が slot に触れても publish は未開始 — 順序の
  単純さを優先）。
- **lock poisoned でも回収継続** — `drain_retired`/
  `retired_overflows`/`swap` は collector の
  `PoisonError::into_inner` 経路（退役 queue の中身は
  destructor で汚れない構造 — poison は観測不能な副作用）。
- **`Session` field 宣言順は変更不要だった** — 既存順
  （`stream` → `swap`）が既に「collector が retirer より後に
  死ぬ」安全順に適合。順序を pin する doc のみ追加（
  declaration order = drop order の依存関係を明文化）。
- **`plan_retirement_drops_via_collector_drain` は非 `rt_`
  命名**（deliberate）— drain で destructor が走る経路は
  no-alloc 契約の対象外（destructor は alloc/free し得る）。
  `rt_` prefix を付けると `test-rt` レーンで `assert_no_alloc`
  窓の意味を持たないため、unit lane 側に配置する命名規則を
  遵守。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — `FnMut` blanket impl の代替（named newtype・plugin 型
trait ラップ）は design 段階で不採決定済み・本 task で新規
評価なし。

## Validation

- gate（実 log・全 exit 0）:
  - `mise run fmt-check` — exit 0
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets`）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean — `#[must_use]`/
    `must_use_candidate` pin 込み）
  - `mise run test` — exit 0（`cargo test --workspace` —
    **55 test binaries 全 pass**）
  - `mise run test-rt` — exit 0・**実一致件数**: ruviform-audio
    で **6 件**（新規 4 件 = `rt_plan_active_no_alloc`/
    `rt_plan_swap_retire_no_alloc`/`rt_plan_drop_retire_no_alloc`/
    `rt_fn_node_pull_no_alloc` + 既存 `rt_callback_path_no_alloc`/
    `rt_denormal_flush_no_alloc`）— 0 件スキップ・0 件バイナリ
    なし
  - `mise run build` — exit 0（`cargo build --workspace`）
  - `mise exec -- cargo test -p ruviform-audio` — exit 0（lib
    **16** + doctest **1** — 新規 8 件を含む全 pass）
  - `mise exec -- cargo tree -p ruviform-audio` — deps は
    `ruviform-audio-core`/`ruviform-media-core`/`ruviform-rt`
    のみで `ruviform-plugin-*` 出現なし（AC3 実証。dev-deps
    は `assert_no_alloc` + `static_assertions` のみ）
- 構造確認（実コマンド）:
  - `rg -n "fn rt_" crates/ruviform-audio/src` — 新規 `rt_` 4
    件の実在（`test-rt` の実一致件数と整合）
  - `rg -n "static_assertions" crates/ruviform-audio` — dev-dep
    宣言 + `src/lib.rs` 内 `assert_impl_all!`/`assert_not_impl_any!`
    の実在（`[dependencies]` への混入なし — `cargo tree` で
    AC3 実証済み）

## Non-Goals

- `PluginHost::prepare_endpoint` から mint される endpoint の
  `FnMut` closure 化（graph 接続）— Task 6 の scope。本 task
  は closure が `Box<dyn AudioNode>` へ coerce する seam と
  退役シームのみ供給。
- `process_audio` shell・bound clip・source-position 慣行・
  graph e2e — Task 6 の scope。
- `Option<RtPlan>` wrap（`session.rs` `let mut rt_plan =
  Some(rt_plan)`）の整理 — 現行形は dead wrap 気味（callback
  内の `as_mut()` 経由のみ使用）だが挙動上無害 — follow-up
  へ記録（本 task の scope 外）。
- `DropCollector::take_queued` 型 API（queue 中身の検査口）—
  `ruviform-rt` 側の API 候補として follow-up 記録。現行の
  `drain()`/`overflows()` で本 task の観測点は充足。
- `GraphPlan::drive` の poison 永久無音の根治 — documented
  degraded behavior（poisoned graph 再 enter 回避は意図的）。
  無音を「transitive に観測する仕組み」（e.g. underrun 統計へ
  の接続）は follow-up。
- test-spec `plugin-rt-retirement-tests.md` の `proposed →
  confirmed` 更新 — final verification（Task 7）で実施。

## Risks

- **`active()` signature の公開変更** — `Option<Arc<GraphPlan>>`
  → `Option<&GraphPlan>`: pre-1.0 許容・plan で API 未確定の
  範囲。workspace 内の唯一の call site（`session.rs`）は
  unchanged-compatible で compile 確認済み — downstream 外部
  crate（存在する場合）は compile error で検出される silent
  change ではない。
- **退役 channel 飽和は leak（silent capacity miss）** —
  `retire` が `Full` なら `mem::forget` で leak し
  `retired_overflows` で観測 — drain cadence が不足する運用
  （swap 連発・drain 無し）では容量設計が効く。観測点は公開
  済みだが監視自体は consumer の責務。
- **drain-under-lock の再入 deadlock** — node `Drop` から
  `PlanSwap` method 呼出しは非再入 `Mutex` で deadlock（doc
  pin 済み）。T6 で endpoint closure node を持つ plan が退役
  される際、`PluginEndpoint::drop`（CAS のみ）が `PlanSwap`
  を呼ばない契約が効く — 構造上安全だが暗黙依存として記録。
- **`assert_no_alloc` 窓は `catch_unwind` 初回登録に注意** —
  `rt_endpoint_*`（host 側）で warm-up が必要だったのと同じ
  く、本 task の rt_ テストも fixture/warm-up を窓外に配置
  （測定窓内の初回実行で panic-state 登録が alloc として誤検
  されないための規約 — 全 rt_ テストが遵守）。
- **`Cargo.lock` の T5 entry は T2 commit（`dca7b82`）に着済**
  — 本 commit の diff には lockfile が含まれないが、T2 commit
  が本 task の `static_assertions` entry を既に運ぶ（単一 file
  分割不能 — deviation として両 IR に記録）。

## Follow-ups

- Task 6 `docs/tasks/plugin-graph-e2e-and-rt-lane.md` —
  endpoint closure が `FnMut` impl 経由で `Box<dyn AudioNode>`
  へ coerce し、`Session::drain_retired`/`retired_overflows`
  passthrough 経由で退役観測（graph e2e test-spec の観測点到達
  を可能にするために本 task が供給した面を消費）。
- `GraphPlan::drive` poison 永久無音 — documented degraded
  behavior として `drive` doc に記録済み（panicking node の
  guard 越し unwind → 以後全 `drive` が静黙 no-op）。無音の
  transitive 観測（underrun 統計・device port 側の検出）への
  接続は別途検討。
- `session.rs` の `Option<RtPlan>` wrap — `let mut rt_plan =
  Some(rt_plan)` + `as_mut()` は dead wrap 気味（挙動上無害）
  — callback signature 整理の機会に簡素化候補。
- `DropCollector::take_queued` 型 API — `ruviform-rt` 側で
  queue 中身を inspect する観測口の候補（現行 `drain()`/
  `overflows()` で本 task は充足 — test-spec の今後の観測点
  要求で再評価）。
- Task 7 `docs/tasks/engine-plugin-final-verification.md` —
  test-spec `plugin-rt-retirement-tests.md` の `proposed →
  confirmed` 更新 + Full Gate での `cargo tree` AC3 再監査。
