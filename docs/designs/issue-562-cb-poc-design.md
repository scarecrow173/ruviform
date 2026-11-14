---
id: "DESIGN-034XeideNx6BpCgSGgEuQF"
type: "design"
status: "approved"
title: "Issue #562 — Generic Alternate Evaluation Context headless PoC 設計（RF-CB-01）"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/562"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-562-cb-poc.md"
  implemented-by:
    - "docs/impl/ir/issue-562-cb-poc.md"
  depends-on:
    - "docs/specs/issue-562-cb-poc.md"
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-562-cb-poc.md"
  derived-by:
    - "docs/test-specs/issue-562-cb-poc.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #562 — Generic Alternate Evaluation Context headless PoC 設計（RF-CB-01）

## Overview

Synvelis Creative Branch の「固定入力下での代替 Graph/Plugin 状態評価」
（2 候補 alternate evaluation）を、Ruviform 既存 API だけで
headless に構成できるかを検証する PoC 設計。新規 Generic API は
追加せず、既存能力の組み合わせで契約を満たすか、あるいは真の Gap
があるかを証拠で決裁する。

## Candidate Model（consumer 非依存の最小構成）

「候補（candidate）」の generic な実体は **独立した実行コンテキスト**
であり、Synvelis の VariantSet/Clip/Take 意味論を持ち込まない:

| 層 | 既存 API | 候補としての役割 |
| --- | --- | --- |
| Graph compile | `Compiler` + `MockFactory`/`NodeFactory` | 同一 `ValidatedGraph` を候補ごとに compile |
| RT 実行 | `GraphRuntime` + `RtRuntime` pair | 候補ごとに独立 pair（processor/slot/event channel 非共有） |
| Offline 描画 | `PreparedRender::compile` → `render` | instance ごとに固有 `GraphRuntime`+`RtRuntime`（independence by construction） |
| 観測 | `probe::pair` + `ProbeBroker`/`ProbeCollector` | 候補ごとに独立 pair（domain/runtime_instance が一意） |
| 比較 | `align_observation_points` | 3 値 Alignment（Aligned/Conditional/Incomparable） |

「固定入力」は consumer が供給する決定的 signal feeder / boundary
feed であり、Ruviform はその同一性を **consumer が宣言する**
（probe の `ClockAnchor` で各候補 cursor を共有 engine epoch に
写像する形）。これが Issue 本文の `EvaluationInputIdentity` に相当
する役割を既存型で担う。

## 決定事項

- **D1 候補 = 独立 runtime pair**: 2 候補は同一 spec・同一固定入力・
  異なる DSP 状態で構成し、mutable state 非共有を出力値と状態
  counter で証明する。共有があれば出力が変わる設計の fixture を使う。
- **D2 Preview = 並行評価 + 出力選択**: Preview A/B 切替は
  「どの runtime の出力を monitor/記録が読むか」の consumer 側選択
  であり、Ruviform 側に preview 専用 API は不要。Main/Recording
  副作用禁止は「候補評価が Main runtime の process 状態・出力・
  probe stream を変えない」ことで証明する。
- **D3 Offline = PreparedRender per candidate**: `PreparedRender::compile`
  が候補ごとに独立 `GraphRuntime`+`RtRuntime` を構築する既存契約を
  そのまま使い、transport（Play/Seek/SetLoop）・tail・PDC
  （`total_latency`）・preflight（`OfflineRenderSupport::Verified`
  ゲート）を検証する。
- **D4 Probe = 独立 pair + EngineFrame anchor**: 各候補の probe pair は
  一意の cursor domain を mint するため SharedCursor basis では
  align できない（`DomainMismatch`/`AnchorMissing` で fail-closed）。
  consumer が各 `ProbeTimeContext` に共有 engine epoch の
  `ClockAnchor` を配線すれば `EngineFrame` basis で `Aligned` になる
  ことを確認する。generation は runtime ごとの counter で両候補
  `single(1)` となり候補識別には使えない — 識別は
  `ProbeClockMark.domain` / `ProbeTimeContext.runtime_instance` /
  `ProbeId`+`ProbePoint` が担う。
- **D5 Failure = bounded rejection**: `OfflineRenderSupport` が
  `Verified` でない候補は `PreflightError::OfflineUnsupported` で
  出力生成前に reject。他候補・Main は影響を受けない。
  generation 不一致 event は `stale_events` counter で drop。

## Gap 判定方針

各 Case について Expected/Observed/使用 API を記録し、
Pass（既存 API で契約充足）/ Partial（条件付き充足・consumer 側
bookkeeping が必要）/ Gap（既存 API では表現・強制不能）で採点。
Gap が出た場合のみ `crates/ruviform-audio/src/` の最小変更を検討し、
変更の最小性・既存 API 非破壊・Owner・migration・後続 Issue を
PR 本文と gap report に記録する。

## Non-Goals

- Synvelis 意味論（Variant/Branch/Adopt/Diff）の Ruviform 移入。
- 実 Plugin/GUI/Device の検証（Windows L2/L3・EXT-L3 は別スライス）。
- 新規 crate・新規 public API の先行追加（Gap 実証が先）。

## Verification

- `mise run fmt-check` / `mise run check ruviform-audio` /
  `mise run lint ruviform-audio` / `mise run test ruviform-audio`。
- headless のみ — OS-L1 相当。実機 Pass は主張しない。
