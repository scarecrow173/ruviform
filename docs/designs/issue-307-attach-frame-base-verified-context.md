---
id: "DESIGN-dHUQ40Hk50tvziqjx7Kewj"
type: "design"
status: "approved"
title: "Issue #307 attach 時 frame_base 検証 context — 実装設計"
created: "2026-11-04"
updated: "2026-11-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/307"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-307-attach-frame-base-verified-context.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-307-attach-frame-base-verified-context.md"
    - "docs/designs/issue-190-transport-span-scheduler.md"
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-186-param-control-events.md"
    - "docs/designs/issue-188-timed-midi.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-307-attach-frame-base-verified-context.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #307 attach 時 frame_base 検証 context — 実装設計

## Overview

`attach()` は `AttachConfig.frame_base` を `PluginNode::new`（→ 内蔵
`TransportScheduler::new`）へ渡すが、返却 `Attached.control` には base を
知る手段がなく、consumer は外部 `frame_base` を併記して「等しいはず」と
前提するしかなかった。本設計は control/node 両半分に同一 `AttachConfig`
起源の読み取り専用アクセサを置き、`install_plan` の stage 前に
`plan.frame_base` の照合を入れて、異値 plan が node へ届く全経路を
typed error で塞ぐ。変更は `ruviform-audio-plugin-bridge` に限定する。

## 現行コード照合（Issue 本文との突合結果）

- `attach.rs:44,168`: `cfg.frame_base` は `PluginNode::new` のみへ。
  `control.rs` の `PluginNodeControl::new(ctrl_tx, report_rx, status)` は
  base を受け取らない。
- `node.rs`: `PluginNode` は `frame_base` フィールドを持たず
  `TransportScheduler::new(frame_base, …)` へ委譲。getter 非公開。
- `transport_schedule.rs`: `TransportScheduler::frame_base()` getter あり。
  `install_plan` は `plan.frame_base` を scheduler base と比較しない
  （base 照合は render preflight の `PreflightError::Range` が唯一）。
- `TransportPlan.frame_base` は `pub` フィールド — bridge から直接比較可能。
- `TimeBase` は `new` で gcd 正規化されるため `PartialEq` が実質同値。
- `PluginNodeControl::install_plan` の呼出は本 repo では bridge tests のみ
  （`param_delivery.rs`×4, `midi_wiring.rs`×8, `e2e.rs`×1）。全て
  `.unwrap()` 経由で error 型への match なし。

## API 形態の確定（Issue 要件「互換検証後に確定」への回答）

Issue 候補: (a) `Attached` への `AttachedClockContext` 追加、(b)
`PluginNodeControl::frame_base() -> TimeBase` アクセサ。

**採用: (b) を両半分へ — `PluginNodeControl::frame_base()` +
`PluginNode::frame_base()`。`AttachedClockContext` は不採用。**

理由:

1. `Attached` は `node` を graph factory へ、`control` を engine 側へ即座に
   分解する中継 struct。context bundle をそこへ置くと、分解後の consumer
   は結局 `control`/`node` 側の取得経路を必要とし、参照点が二重化する。
2. bundle に含めうる第 2 要素は clock epoch だが、attach 時点の epoch は
   常に `ClockEpochId::new(0)` で情報価値がなく、`DeviceRestart` で変化する
   時刻世代を attach 不変の frame base と同じ context へ束ねることは
   Issue が禁じる「frame base と時刻世代の混同」を構造的に招く。
3. 「`PluginNode` と `PluginNodeControl` の公開 context が同一
   `AttachConfig` 起源」という保証は、両方が `cfg.frame_base` から
   供給されることで構成時に保てる（下記「単一起源」）。

追加の消費側 API（Issue「同一の検証済み Context として取得・消費」の
具体化）:

- `check_frame_base(base)` — consumer の Dispatcher/Plan 構築 ctor が
  `?` で伝播できる typed error 入口。
- `plan_builder(generation)` — 検証済み base に束縛された
  `TransportPlanBuilder` を返し、正しい構築を既定経路にする
  （異値拒否の `install_plan` と合わせ「構築時の正本化 + 境界の強制」）。

## 公開 API

```rust
// control.rs
impl PluginNodeControl {
    /// attach 時に確定した frame base（`AttachConfig::frame_base` と同一値）。
    /// `SetClock`/`DeviceRestart` 等の時刻世代変化では不変。
    /// **RT contract:** non-RT。
    #[must_use]
    pub const fn frame_base(&self) -> TimeBase;

    /// `base` が attach 時 base と等しいか検証（実質同値 = 正規化 rational
    /// の `PartialEq`）。consumer の Dispatcher 構築前検証用。
    /// **RT contract:** non-RT。
    pub fn check_frame_base(&self, base: TimeBase) -> Result<(), FrameBaseMismatch>;

    /// この node の frame base に束縛された `TransportPlanBuilder`。
    /// **RT contract:** non-RT。
    #[must_use]
    pub fn plan_builder(&self, generation: PlanGeneration)
        -> TransportPlanBuilder<EventKind>;

    /// 変更: `Result<(), QueueFull>` → `Result<(), InstallPlanError>`。
    /// `plan.frame_base` 不一致は stage 前に拒否（非破壊）。
    pub fn install_plan(&mut self, plan: Arc<TransportPlan<EventKind>>,
        due_frames: u64) -> Result<(), InstallPlanError>;
}

/// 供給された `TimeBase` が node の attach 時 frame base と異なる。
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FrameBaseMismatch {
    pub expected: TimeBase, // node の attach 時 base（検証済み context）
    pub actual: TimeBase,   // 供給された異値 base
}

#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InstallPlanError {
    /// bounded queue 満杯 — drop + `control_overflows` bump（従来意味）。
    QueueFull(QueueFull),
    /// `plan.frame_base` が attach 時 base と不一致 — stage 前拒否。
    FrameBaseMismatch(FrameBaseMismatch),
}

// node.rs
impl PluginNode {
    /// attach 時に確定した frame base（内蔵 scheduler と同一値）。
    /// **RT contract:** RT-suitable（read-only）。
    #[must_use]
    pub const fn frame_base(&self) -> TimeBase {
        self.scheduler.frame_base()
    }
}
```

`attach` は `PluginNodeControl::new(ctrl_tx, report_rx, status,
cfg.frame_base)` とし、`PluginNode::new` へ渡す値と同一変数を供給する
（**単一起源**: 両半分の公開 context は同一 `cfg.frame_base` 由来）。
`lib.rs` で `FrameBaseMismatch`/`InstallPlanError` を re-export。

## 境界の決定（どこで検証するか）

- **stage 前 control 側照合を採用**: `install_plan` は `Producer<NodeControl>`
  を持つ唯一の型であり、`NodeControl::InstallPlan` への全経路がこの
  メソッドを通る。ここでの照合で異値 plan の node 到達経路は全て閉じる。
  RT callback・node 内蔵 scheduler への追加検査は不要であり、
  「信頼された内部コードへの防御的チェック拡散」を避ける原則とも一致。
- **media-core `install_plan` 内照合は不採用**: Issue 境界は
  「plugin-bridge 内で完結」。scheduler 側への変種追加（`RejectReason`
  variant 等）は非破壊だが別 crate 変更であり、receipt 経由の非同期
  報告は「typed error を nonRT に返す」即時性を満たさない。
- **`NodeReport` による非同期報告も不採用**: 同上（成功/失敗の同期判定が
  consumer 側 ctor に必要）。
- **非破壊性**: 照合は `try_push` より先に行う。異値時は queue 未使用・
  `control_overflows` 非加算・Arc は drop のみで vault 未登録・旧 plan/
  graph/node 状態は一切変化しない。

## 互換性・移行

- `install_plan` 戻り値型の変更は `publish = false` 下の**明示移行**。
  repo 内全呼出は `.unwrap()` で影響なし。`QueueFull` 自体は
  `request`/`set_clock`/`device_restart`/`set_clock_anchor` の戻り値として
  維持（`InstallPlanError::QueueFull(QueueFull)` で包む）。
- `Attached`/`attach`/`AttachConfig`/`NodeControl`/`NodeReport` の形状は
  不変（`NodeControl::InstallPlan` payload も不変）。
- `PluginNodeControl::new` は `pub(crate)` — 外部影響なし。
- 破壊的 API 変更は `install_plan` 戻り値のみ（`Result<(), QueueFull>` →
  `Result<(), InstallPlanError>`）。PR 本文で consumer 移行手順を明記。

## RT 契約・時刻モデル遵守

- 追加するものは全て非 RT: `frame_base()`(const read), `check_frame_base`,
  `plan_builder`, `install_plan` の照合（`PartialEq` + early return。
  alloc/block/mutex なし）。
- `frame_base` は attach 以降不変。`DeviceRestart`/`SetClock`/Seek/Loop/
  `PlanGeneration` と独立 — `ClockEpochId` は別 identity として混同しない
  （doc コメントで明記）。
- 暗黙 rate conversion・時刻 scale 変更なし。異値は成功に変換しない。
- `TimedParamEvent` の `MediaTime` 位置は `plan_builder`/`builder.event` の
  `convert_exact` で検証済み base へ正確変換 — base 一致時は変換が自明に
  exact（AC-1 の絶対 frame 一致を構成上保証）。

## 不変条件

1. `control.frame_base() == node.frame_base() == AttachConfig::frame_base`
   （attach の単一供給源）。
2. `install_plan(Err(FrameBaseMismatch))` → 何も stage されない。
3. `plan_builder(g)` で `build()` した plan は `install_plan` の照合を
   必ず通る。
4. `check_frame_base(Ok)` ⟺ `base == frame_base()`。

## 検討したが棄却した選択肢

| 選択肢 | 棄却理由 |
| --- | --- |
| `Attached` へ `frame_base` フィールド/context struct 追加 | 分解後の取得経路が二重化。epoch を束ねると非混同原則に反する。`control.frame_base()` で充足 |
| `install_plan` 旧型維持 + `install_plan_checked` 追加 | 未検査経路が残り「authoritative な実装を一箇所に」に反する。`publish=false` 下では明示移行が正しい境界修正 |
| scheduler 側（media-core）での照合 | Issue の scope 外 + receipt 非同期報告では ctor 同期拒否を満たさない |
| `AttachConfig.frame_base` と `sample_rate` の attach 時整合検証 | engine tick 意味論は consumer 宣言の新規ポリシーであり別 Issue（render preflight が独立に照合済み） |
| `ParamDispatchLedger::record_plan` の base 照合 | install 境界で異値 plan の流入が閉じるため二重化。投影 helper は信頼入力前提の内部 layer |

## Test Plan

`crates/ruviform-audio-plugin-bridge/tests/frame_base.rs`（新規）に
test-spec の Feature/Rule/Example を実装する（詳細は test-spec 参照）。
検証入口は `mise run fmt-check` → `check` → `lint` → `test` → `test-rt`。
