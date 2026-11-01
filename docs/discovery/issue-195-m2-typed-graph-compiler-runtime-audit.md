---
id: "DISC-034QfjLwjBVqddpxivwOTg"
type: "discovery"
status: "active"
title: "Issue #195 M2 Typed Graph Audio Compiler・Buffer/PDC・Stateful RT Ownership・Plan Activation の現行棚卸し"
created: "2026-11-01"
updated: "2026-11-01"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/194"
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/pull/225"
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
    - "docs/discovery/issue-194-m0-baseline-audit.md"
    - "docs/discovery/issue-184-bus-port-graph-ir-crate.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-194-m0-baseline-audit.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/190"
  deferred-by: []
---

# Issue #195 M2 棚卸し: Typed Graph → Audio Runtime コンパイル経路の現状

## Purpose

Issue #195（Migration M2）着手前の実装棚卸し。#184（M1）が main に入った `ruviform-graph` IR を、既存 `ruviform-audio` の逐次 `Graph`/`GraphPlan`/`PlanSwap` モデルから `Typed Graph IR → 非 RT Compiler → immutable Prepared Schedule → 単一 owner Stateful Runtime` へ移行するために必要な既存資産・差分・制約を確定する。

## Baseline Facts（実測済み・diff なし）

- HEAD = `8ad88f5`（PR #225 merge、Issue #184 実装 `b238ea9` + 文書 `f69368`）。worktree clean、branch `scarecrow173/rf195-m2-audio-runtime`。
- `crates/ruviform-graph`（M1）: `graph.rs`(`NodeKind`/`EdgeDelay`/`NodeDef`/`PortRef`/`ConnDef`/`NodeSpec`/`PortSpec`/`Connection`/`GraphSpec`/`GraphBuilder`)、`port.rs`(`Direction`/`SignalDomain`/`PortRole`/`PortFormat`/`PortCapability`/`Availability`/`Activation`/`Multiplicity`/`PortDef`/`ChannelMap`)、`validate.rs`(`ValidationError`/`ValidationErrors`/`Validator`/`ValidatedGraph`+Tarjan SCC)、`mapping.rs`(`StableHandleMap`/`topo_order` Kahn+BTreeMap tie-break)、`project.rs`(`GraphProject`/`PublishedGraph`/`PrepareError`)。通常依存は `audio-core`+`media-core` のみ（engine/rt/host/device 非依存・`cargo tree` で TA-003 証明済み）。
- `crates/ruviform-audio`（現行 engine）: `graph.rs` の `AudioNode`/`Graph` は `Vec<Box<dyn AudioNode>>` の逐次 pull。`plan.rs` の `GraphPlan{Mutex<Graph>}`・`PlanSwap{RwLock<Arc<GraphPlan>>, Mutex<DropCollector>}`・`RtPlan{try_read+last-good cached+Retirer}`。`session.rs`/`mixer.rs`/`denormal.rs`。`tests/` dir は無く `lib.rs` inline `mod tests`（alloc_probe・swap/retire 系を含む）。`plugin-host` は audio へ **dev-dependency のみ**（`tests/plugin_graph_e2e.rs` が `PluginEndpoint` shell の golden path）。
- `crates/ruviform-rt`（再利用可能プリミティブ・実在確認済み）: `processor::{Processor, RtProcessor}`（`type Io` 関連型・`&RtContext` capability gating）、`snapshot::slot(initial, retire_capacity) -> (SnapshotWriter, SnapshotReader)`（ArcSwap・`read()` lock-free load+Arc clone・`SnapshotGuard` drop は Retirer 経由・`collect()` は非 RT drain・`retire_overflows()` 観測）、`retire::retire_channel(capacity) -> (Retirer, DropCollector)`（`retire` wait-free push・full は `mem::forget`+`overflows` 計数）、`spsc::channel`、`alloc_counter`（GlobalAlloc 計数・RT alloc 監査に使用可）、`context::RtContext`（`mint` は `#[doc(hidden)]`）。
- `ruviform_audio_core`: `BufferSpec`/`AudioBuffer`/`AudioBufferView`/`AudioBufferViewMut`（planar・`try_new(data,channels,frames)`・channel slice アクセス）、`AudioFormat{sample_rate,layout,sample_format}`、`ChannelLayout::channels()`。
- 制約（ADR/design 確定済み）: `audio-engine-snapshot-swap.md`（accepted）は `RwLock+try_read+last-good+非RT retire` を採択。`ruviform-rt::snapshot` はその機構の primitive 化であり、typed path で同機構（`slot`）を再利用するのは後継 ADR 不要の範囲。新しい同期方式の並存新設は禁止。RT は final destructor・OS cleanup を行わない（retire→非 RT drain）。`PDC: D_i = max(L_j) - L_i`（same-clock merge）。fan-out は最終 consumer まで slot 生存・上書き禁止。M1 ADR は Compiler/Buffer/PDC/Stateful Ownership を M2 へ明示 deferred。
- `EdgeDelay::Declared` は IR 表現可だが量を持たない（M1 で variant 形状のみ）。delayed cycle は M1 validator が `DelayedCycleUnsupported` で常時拒否 → M2 の対象グラフは全て zero-delay DAG。

## Gap（#195 G2 に対する欠落）

1. IR → 実行計画の compile 経路が存在しない（schedule・buffer 割当・PDC・binding 全て未実装）。
2. Bounded buffer（最大 slot 数・サイズ・alignment）を prepare 時に確保する仕組みがない。`Vec` の align は `align_of::<f32>`=4 のみ → 明示 alignment は over-allocate+offset の安全コードで実現可能。
3. RT callback で process する runtime（alias/uninit-read 拒否・可変 frame・no-alloc/no-lock/no-block/no-OS-I/O）がない。`Vec<RefCell<Slot>>` arena + disjoint index binding で unsafe なしに disjoint borrow を構成可能。
4. Stateful Ownership が未選定。`ruviform_rt::RtProcessor` は `type Io` が静的関連型のため borrowed multi-port IO を object-safe に表せない → crate 内 `GraphProcessor` trait（`&mut NodeIo<'_>` + `&RtContext` で同一 capability 契約を維持）が必要。
5. Activation/generation/fault の計装がない → `SnapshotWriter`+spsc command+atomic status で構成可能（既存 primitive 再利用）。
6. G2-06 の 64/128/256/可変 frame・zero/tail 規則・RT 監査（alloc_counter）の test がない。
7. 旧 API の golden fixture test がない（現行は lib.rs inline 単体 test のみ。plugin-host e2e は別 crate）。

## Options（棚卸し所見・決定は ADR）

- **crate 配置**: compiler/runtime は `ruviform-audio` 配下（engine が `ruviform-graph` を下向き参照=DAG 合法・`ruviform-rt`/`audio-core` 既存依存を再利用）。`ruviform-graph` 側に置くと rt/engine 依存を導入して TA-003 を壊すため不適。
- **Stateful Ownership**: (a) Plan-owned（plan が processor Box を保持・`Mutex` で `&mut`、legacy `GraphPlan` 型）vs (b) Stable Runtime Slot（processor registry は RT 側単一 owner `RtRuntime`、plan は immutable wiring のみ・NodeId→slot index 参照）vs (c) 第三案（plan が install 物を内包→Arc 不変性と矛盾し unsafe/`Mutex` 必須で実質 (a) 退化）。同一 Mock DSP で比較測定してから選定。
- **activation**: `snapshot::slot` + spsc install/remove command（generation 付き）+ atomic fault/status。旧世代 event は generation 不一致で遮断・計数。

## 依存・制約の確定値

- `ruviform-plugin-host` → `ruviform-audio` は dev-dependency のみ。M2 で生産依存に変更しない（TA-005）。
- 旧 `AudioNode`/`Graph`/`GraphPlan`/`PlanSwap`/`PluginEndpoint` shell は削除しない。golden fixture で維持（G2-07・TA-012）。
- `#[non_exhaustive]` public enum 方針・per-crate error enum・`unsafe` 禁止・`BTreeMap` 決定性・mise run 検証入口は #184 規約を継承。
- RT 監査は `ruviform_rt::alloc_counter` + `rt_` 命名 test（`mise run test-rt` lane）で実施。

## Open Questions → ADR/Design へ

- Q1: `GraphProcessor` trait の object-safe 形状（borrowed IO と `type Io` の両立不能問題の解消）。
- Q2: stable slot registry の RT 側配置と install/remove の generation 整合（spsc 部分適用時の rollback）。
- Q3: boundary（graph input/output）の決定規則 — `NodeKind::Source` 出力を graph 入力、`NodeKind::Sink` 入力を graph 出力とし、factory が processor を返さない node を boundary とする規約で良いか。
- Q4: fan-in merge semantics — 明示規約として sample-wise sum + per-edge ChannelMap/遅延を適用し scratch slot に materialize。implicit mix は禁止。
- Q5: `EdgeDelay::Declared`（DAG 上）の扱い — 量を持たないため M2 では `CompileError::UnsupportedEdgeDelay` で定義済み失敗とするか。
