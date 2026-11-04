---
id: "TSPEC-XQ6Q9VkNnBlPRcMf4cqLFu"
type: "test-spec"
status: "approved"
title: "Issue #307 attach 時 frame_base 検証 context — テスト仕様"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-307-attach-frame-base-verified-context.md"
    - "docs/designs/issue-307-attach-frame-base-verified-context.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-307-attach-frame-base-verified-context.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-307-attach-frame-base-verified-context.md"
    - "docs/designs/issue-307-attach-frame-base-verified-context.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #307 attach 時 frame_base 検証 context — テスト仕様

> status: approved — `mise run test` で実行される `tests/frame_base.rs` の oracle 正本。

## Purpose

spec AC-1..5（design「不変条件」1..4 を含む）を、`ruviform-audio-plugin-bridge`
の実 API（`attach` → `PluginNodeControl`/`PluginNode` → typed graph
process）を通じて検証する。

## Feature

`PluginNodeControl::{frame_base, check_frame_base, plan_builder,
install_plan}`、`PluginNode::frame_base`、`FrameBaseMismatch`、
`InstallPlanError` の公開契約。RT callback 経路は変更なし
（`rt_param_dispatch_no_alloc` 既存 lane が非回帰を兼ねる）。

## Rules

- `control.frame_base() == node.frame_base() == AttachConfig.frame_base`
  （attach 時 1/48000 で往復一致。単一起源）。
- `check_frame_base` は base 一致で `Ok`、異値で
  `Err(FrameBaseMismatch{expected,actual})`（expected は attach 値）。
- `plan_builder(g)` で作った plan の `frame_base` は attach 値であり、
  `install_plan` の照合を必ず通る。
- 異値 base で手組した `TransportPlan` の `install_plan` は
  `Err(InstallPlanError::FrameBaseMismatch)`。**非破壊**:
  queue 未使用・report なし・`control_overflows` 非加算・後続の正常
  install は通常適用。
- `device_restart`/`set_clock`/seek/loop/plan generation 更新で
  `frame_base()` は不変（Clock Epoch と混同しない）。
- 検証済み base で組んだ TimedParamEvent plan は RT process で
  絶対 frame に一致して届く（既存 AC-5 系の非回帰 + 本 AC-1）。

## Examples

- `attach(frame_base=1/48000)` → `control.frame_base() ==
  TimeBase::new(1,48000)`、`node.frame_base()` 同値。
- `check_frame_base(TimeBase::new(1,44100))` →
  `Err(FrameBaseMismatch{expected:1/48000, actual:1/44100})`。
- `TransportPlanBuilder::new(TimeBase::new(1,96000), gen)` の plan を
  `install_plan` → `Err(InstallPlanError::FrameBaseMismatch{..})`。
  直後に正しい plan を install → 適用・receipt 正常・process で
  event 配送。
- `control.device_restart(0)` 適用後も `control.frame_base()` は
  `1/48000` のまま。
- `plan_builder(PlanGeneration::new(1))` + `event(MediaTime::new(64,base))`
  → install → process → 絶対 frame 64 の call 相対 offset で配送。

## Guarantees

- `InstallPlanError`/`FrameBaseMismatch` は `Copy` で `&'static` メッセージ
  （`Display`）— RT 経路には乗らないが型として allocation-free。
- `InstallPlanError` は `#[non_exhaustive]`（外部 match は `_` 必須）。
- 新規テストは MSRV 1.89 でコンパイル（特別な unsafe/依存なし）。

## Non-goals

- scheduler 内部照合・`RejectReason` variant（scope 外・design 参照）。
- `AttachConfig.frame_base` vs `sample_rate` 整合（別 Issue）。
- Synvelis 側 `ParamDispatch` 導入テスト（synvelis#101）。
- 実 Plugin/OS 実機計測（本 Issue は Not Run 明記）。

## Risk

- `install_plan` 異値拒否テストが「非破壊」を証明するには、report queue
  空・後続 install 正常の両立を oracle に含める必要がある
  （単に Err を見るだけでは既存 state 破壊を検出できない）。
