---
id: "DESIGN-034JROdBjynllTNAVDZwaI"
type: "design"
status: "approved"
title: "音声エンジン設計"
created: "2026-10-21"
updated: "2026-10-21"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/audio-engine.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
  derived-by:
    - "docs/plans/implement-issue-25-audio-engine.md"
    - "docs/test-specs/audio-engine-tests.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 音声エンジン設計

## Context

Issue #25（spec `docs/specs/audio-engine.md`）。`ruviform-audio` エンジン新設 — pull グラフ・デバイスアダプタ統合・RT パス。

## Scope

### In Scope

- `crates/ruviform-audio` 新設（workspace メンバー・crate doc に責務/依存層/RT 制約）
- `pull_graph` モジュール: `Node` trait（`process(&mut self, io: &mut Io, ctx: &RtContext)` — `RtProcessor` の薄いラッパ）・`Graph`（`Vec<Box<dyn Node<Io=AudioBufferMut>>>`）
- `mixer`: planar f32 の和算ノード
- `session`: `OutputStreamConfig` 受けてコールバックを配線 — callback 内で `RtContext::mint` → `graph.process(out, &ctx)`（借用 `out` はデバイス提供 `&mut` 領域）
- `snapshot`: `RwLock<Arc<GraphPlan>>` + `try_read` + 直前キャッシュ（ADR `audio-engine-snapshot-swap`）

## Components and Boundaries

```text
ruviform-audio
├── graph: Graph { nodes: Vec<Box<dyn AudioNode>> }  // pull: destination が要求
├── mixer: Mixer { inputs, out }                      // planar f32 和算
├── plan:  GraphPlan (immutable, Send+Sync)          // prepare 済み・buffers 確保済
└── session: Session { stream: Box<dyn OutputStream>, plan: RwLock<Arc<GraphPlan>>, last: Mutex<Option<Arc>> }
```

## Data and Control Flow

`OutputStream` の `data_callback(out, info)` 内（RT）: `plan.try_read()` → 成功時 `Arc` を `last` にキャッシュ → `graph.process(out, ctx)`（`ctx` は mint）→ ミキサー和算。失敗時は `last` の `Arc` を使用（1 ブロック遅延）。差替 API は非 RT 側 `write()`。

## Non-Functional Constraints

- callback 内 no-alloc/no-block/no-OS-call — `assert_no_alloc` 窓で実測（#22 レーン）
- `try_read` 失敗時のフォールバックは「直前プラン継続」— 最大 1 ブロック遅延を契約として明記
- `unsafe` 不使用・audio-device 型の公開露出なし

## Risks and Trade-offs

- `RwLock` writer 占有 → RT 側は遅延 1 ブロックで吸収（許容）
- `Arc` clone/drop は参照カウントのみ・alloc-free — 実測で確認

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- <!-- concern | reason | re-engagement trigger | risk if ignored -->

## Verification Notes

- `rt_callback_path_no_alloc`（`assert_no_alloc`）
- `plan_swap_under_run`（実行中差替）
- `mixer_sums_planar`（和算正確性）
- 全量検証列

## Context

Issue #25（spec `docs/specs/audio-engine.md`）。`ruviform-audio` エンジン新設 — pull グラフ・デバイスアダプタ統合・RT パス。

## Scope

### In Scope

- `crates/ruviform-audio` 新設（workspace メンバー・crate doc に責務/依存層/RT 制約）
- `pull_graph` モジュール: `Node` trait（`process(&mut self, io: &mut Io, ctx: &RtContext)` — `RtProcessor` の薄いラッパ）・`Graph`（`Vec<Box<dyn Node<Io=AudioBufferMut>>>`）
- `mixer`: planar f32 の和算ノード
- `session`: `OutputStreamConfig` 受けてコールバックを配線 — callback 内で `RtContext::mint` → `graph.process(out, &ctx)`（借用 `out` はデバイス提供 `&mut` 領域）
- `snapshot`: `RwLock<Arc<GraphPlan>>` + `try_read` + 直前キャッシュ（ADR `audio-engine-snapshot-swap`）

## Components and Boundaries

- <!-- component and responsibility -->

## Data and Control Flow

- <!-- major data/control flow steps -->

## Non-Functional Constraints

- <!-- reliability, performance, security, operations -->

## Risks and Trade-offs

- <!-- risk and mitigation -->

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- <!-- concern | reason | re-engagement trigger | risk if ignored -->

## Verification Notes

- [x] plan の検証列（モック駆動・rt no-alloc・plan swap）が設計を実証
