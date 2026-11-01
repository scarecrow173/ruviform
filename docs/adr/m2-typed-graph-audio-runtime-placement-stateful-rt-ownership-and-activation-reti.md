---
id: "ADR-034QfjWFfUXvgvaObuCUwS"
type: "adr"
status: "proposed"
title: "M2 typed graph audio runtime placement, stateful RT ownership, and activation/retirement for Issue #195"
created: "2026-11-01"
updated: "2026-11-01"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/195"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/typed-multi-bus-graph-ir-placement-and-port-connection-validation-model-issue-18.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines:
    - "docs/adr/audio-engine-snapshot-swap.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md"
    - "docs/discovery/issue-195-m2-typed-graph-compiler-runtime-audit.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  defers: []
  deferred-by: []
---

# M2 typed graph audio runtime placement, stateful RT ownership, and activation/retirement for Issue #195

## Context and Problem Statement

Issue #195 M2 は `ruviform-graph` の ValidatedGraph を実行可能な Prepared Plan へ compile し、RT callback で駆動する runtime を必要とする。決定が必要なのは (1) compiler/runtime の crate 境界、(2) stateful processor の所有形態（DSP 状態が plan 更新を跨いで継続する方法）、(3) activation・generation・retirement の機構である。既存 `audio-engine-snapshot-swap.md` は legacy 経路向けに採択済みであり、typed path は同じ publish→last-good→非 RT retire の機構を `ruviform-rt` の primitive 経由で再利用しなければならない（新同期方式の並存新設は禁止）。

## Decision Drivers

- RT 契約: callback 中の alloc/clone-heavy/blocking lock/block/OS-I/O 禁止、final destructor は非 RT。
- G2-04: 公開 plan 更新前後の DSP 状態継続・同一 instance の単一 RT invocation・旧 generation event 遮断。
- TA-006/TA-007/TA-008: 同一 stateful instance が新旧 plan から同時に process されない、RT に alloc/block/destructor なし、generation 不整合 event の誤配送なし。
- cargo DAG: `ruviform-graph` は rt/engine 非依存を維持（TA-003）。plugin-host は audio へ dev-dep のみ（TA-005）。
- `ruviform_rt::RtProcessor` は `type Io` が静的関連型であり、borrowed multi-port IO（`&mut NodeIo<'_>`）を `dyn` object 経由で表現できない。

## Considered Options

- **Option A: Plan-owned processors** — 各 `CompiledPlan` が `Vec<Box<dyn GraphProcessor>>` を所有し、plan 内 `Mutex`（legacy `GraphPlan` 型、`new` で materialize・`try_lock`）経由で RT が `&mut` を得る。processor 数は plan 単位で bounded、退役は旧 plan ごと retire channel。状態継続は plan 間で processor を構築し直す限り失われる（rebuild ごとに DSP 状態 reset）。単一 owner は Mutex が強制。
- **Option B: Stable Runtime Slot** — processor instance は RT 側 `RtRuntime` の `Vec<Option<Box<dyn GraphProcessor>>>` registry が所有し、`Arc<PreparedPlan>`（immutable wiring のみ）は NodeId→slot index を参照する。install/remove は generation 付き spsc command で RT 側に移送し、activation は snapshot publish + command drain のみ。同一 NodeId の processor は plan を跨いで同一 instance → DSP 状態継続。registry が単一 owner で Mutex 不要（`RefCell` のみ）。
- **Option C: Plan-embedded install payload** — plan が install 物を内包し RT が取り出す方式。`Arc<PreparedPlan>` の不変性と矛盾し、取り出しに `Mutex` か `unsafe` が必要で実質 Option A の退化版。さらに plan が processor 生成物を抱えて publish されるため activation 失敗時の install 物所在が不明瞭になる。

## Decision Outcome

- **配置**: compiler・runtime は `ruviform-audio` の新 module（`typed` 配下）に置き、`ruviform-audio` が `ruviform-graph` に path 依存する（engine→構造層の下向き参照・DAG 合法）。`ruviform-graph` 側配置は TA-003 を破壊するため棄却。
- **activation**: `ruviform_rt::snapshot::slot`（ArcSwap publish/last-good read/`SnapshotGuard` drop→Retirer）+ `retire_channel` + `spsc::channel`（install/remove/event、generation tag）+ 共有 atomic fault status を採用。`audio-engine-snapshot-swap.md` の機構の primitive 再利用であり、同 ADR を refines する（supersede ではない — legacy path も維持）。
- **stateful ownership**: **Option B（Stable Runtime Slot）を採用** — ただし本決定は PoC の実測前提であり、仮に Option B が RT safety・lifetime・API 整合のいずれかで不適格と判明した場合は Option A へ revert し、両者不適格なら測定付き Blocked を報告する。
- **理由（PoC 測定待ちの予備的根拠）**: Option A は plan swap のたび processor を再生成するため、同一 NodeId の DSP 状態継続（EX-008/EX-010 系）を snapshot/restore 契約なしには満たせない。Option B は processor の生存を plan から分離し、activation/deactivation を install/remove command で明示でき、同一 instance が registry 単一所有のもと新旧 plan から同時に process されない（schedule は callback 単位で単一 plan のみ実行）。Option C は不変性違反により棄却。
- **Processor trait**: `ruviform_rt::RtProcessor` の `type Io` 静的関連型は borrowed IO を object-safe に運べないため、`ruviform-audio` crate 内に `GraphProcessor` trait（`fn process(&mut self, io: &mut NodeIo<'_>, ctx: &RtContext)`・object-safe・`Send`）を定義し、同一の `&RtContext` capability gating を維持する。

### PoC 判定基準（同一 Mock stateful DSP で実測）

- 状態継続: plan 更新前後で同一 NodeId の内部カウンタ/履歴が保持されること。
- 移動: processor instance が install/remove command でのみ RT 側へ移送・除去されること。
- Activation/Deactivation: install→activate→process→deactivate→remove の順序が観測できること。
- 旧 generation event 遮断: 旧 generation の event が processor へ配送されず計数されること。
- Error rollback: activation/compile 失敗時に last-good plan と旧 registry 状態が維持されること。
- 単一 RT owner: 同一 instance が一度の callback で 1 回のみ process され、二重 process/drop がないこと。

## Consequences

- `ruviform-audio` が `ruviform-graph` に通常依存を追加（path・下向き・循環なし）。`Cargo.lock` 再生成。
- `PreparedPlan` は完全 immutable（wiring のみ）→ `Arc` 共有が型安全かつ processor の `&mut` 問題を registry に隔離。
- Option B 採用時、plan-owned 経路は PoC harness（test 専用）として残り、production API には露出しない。
- `EdgeDelay::Declared`（量なし）は M2 では `CompileError::UnsupportedEdgeDelay` の定義済み失敗 — delayed cycle は M1 validator が既に拒否しており実行意味を持たないため。
- Legacy `Graph`/`GraphPlan`/`PlanSwap`/`PluginEndpoint` は additive 維持。削除は別 Issue の承認待ち。

## Confirmation

- PoC 測定結果（同一 Mock DSP・判定基準の各項目）を impl record と test-spec に記録し、本 ADR を `accepted` に更新する。
- `cargo tree --locked -p ruviform-audio --edges normal` で `ruviform-graph`/`ruviform-rt`/`ruviform-audio-core` のみ・循環 0 を実測。
- `mise run test-rt`（rt_ lane）で RT 監査 test が緑。
