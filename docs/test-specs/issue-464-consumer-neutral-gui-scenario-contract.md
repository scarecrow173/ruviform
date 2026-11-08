---
id: "TSPEC-034UcapDuO3SYzYw2r2tHz"
type: "test-spec"
status: "approved"
title: "Issue #464 — Consumer-neutral GUI Scenario Contract"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/464"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/scenario_contract.rs"
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
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #464 — Consumer-neutral GUI Scenario Contract

## Purpose

Issue #464 の Scenario Contract が「consumer-neutral・薄い・証拠レベルを
混同しない」ことを、Fake と GPUI simulated の両 adapter で同一
scenario 関数を実行して検証する。JSON DSL・独自 executor を導入せず、
`ruviform-ui-test` の `SemanticGuiTest` trait が両 adapter で同じ
意味を持つことを保証する。

## Feature

Consumer-neutral GUI Scenario Contract の Fake/GPUI 共通実行

## Rules

- **R1**: `ruviform-ui-test` の `scenario` module は `ruviform-ui`
  の公開型のみを参照し、GPUI 型・`gpui` crate を import しない。
- **R2**: `SemanticGuiTest` trait は `resolve_target`・`dispatch`・
  `observe`・`settle`・`drain_intents`・`capabilities` を提供し、
  `ControlInput`/`ComponentSnapshot`/`CommandIntent`/`CommandReceipt`/
  `StateProjection` をそのまま使う。
- **R3**: `TargetQuery` は `WindowId` + `ViewInstanceId` を必須とし、
  `ComponentKey` は view 内の component 指定に限定する。
  表示文言・絶対 pixel 座標を stable identity に使わない。
- **R4**: target 解決時に `AttachEpoch`・`generation`・component
  mounted を pin し、操作/観測のたびに照合する。stale key/epoch/
  generation は明示的な `ScenarioError` を返す。
- **R5**: `ScenarioCapability` は `SemanticAction`/
  `SimulatedPointerOrKeyboard`/`RealOsPointerOrKeyboard`/`Ime`/
  `VisualCapture`/`MultiWindow`/`NativePluginDock` を区別し、
  pre-flight で required ⊆ supported を照合する。不足なら
  `ScenarioVerdict::Unsupported`。
- **R6**: `ScenarioVerdict` は `pass`/`fail`/`not_run`/`unsupported`/
  `blocked` を維持し、`ScenarioReport` は run ID・scenario/case ID・
  layer/driver/capability・target/generation・expected/observed/
  receipt・evidence refs を記録する。
- **R7**: Fake adapter と GPUI adapter は同一の `SemanticGuiTest`
  契約を実装し、consumer の `#[test]` が同じ scenario 関数を両者に
  対して呼べる。
- **R8**: 観測（`observe`/`drain_intents`）は read-only で、製品
  service への書き込み・認可迂回を行わない。
- **R9**: `ruviform-ui-test -> ruviform-ui-gpui` の依存逆転は禁止。
  GPUI adapter は `ruviform-ui-gpui` 内に置き、`gpui-test` feature
  でのみ有効。

## Examples

- **E1**: Fake adapter で `resolve_target` → `dispatch(ControlInput::
  Begin)` → `dispatch(ControlInput::Commit)` → `observe` が snapshot
  を返し、`drain_intents` が `CommandIntent` を返す scenario が pass。
- **E2**: GPUI adapter（`gpui-test` feature）で同一 scenario 関数が
  pass し、Fake と同じ `ScenarioReport` 構造を返す。
- **E3**: pinned `AttachEpoch`/`generation` が stale な場合、
  `dispatch`/`observe` が `ScenarioError::Stale{..}` を返す。
- **E4**: `SimulatedPointerOrKeyboard` 等 `SemanticAction` 以外を
  required とする scenario が Fake adapter で `Unsupported` verdict
  を返す。
- **E5**: `ComponentKey` が mounted でない target への `observe` が
  明示的な `ScenarioError` を返す。

## Guarantees

- `SemanticGuiTest` の public signature に GPUI 型が現れない。
- Fake pass が simulated-input・real-OS・draw-plan・pixel-Golden の
  証拠へ昇格しない（capability 区別が report に残る）。
- `begin`/`preview`/`commit` helper は `ControlInput` の wrapper で
  あり、第二の gesture/state authority を作らない。
- `TestDriver` の process/timeout/cleanup を再利用し、独自 runner を
  作らない。

## Non-goals

- 実 OS 入力・Focus・IME・window・DPI・a11y（#313/#243）。
- pixel screenshot・Golden normalization/diff/approval（#314）。
- driver-lab Case/Story・2-theme×2-scale binding（#426）。
- consumer 側 fixture/oracle の Ruviform への持ち込み（Synvelis #114・
  AudioLens #166）。
- JSON Scenario DSL・独自 Test Executor・新しい GUI framework・
  二重 Runner。

## Risk

- target 解決が曖昧だと stale 検出が不安定になる → `WindowId` +
  `ViewInstanceId` を必須にし、`AttachEpoch`/`generation`/mounted を
  pin して照合する。
- GPUI adapter が `ruviform-ui-test` の trait を実装する際に
  `gpui-test` feature 外で GPUI 型が漏れる → feature gate と
  boundary guard で検出する。
- `deliver_event` の decode が Fake/GPUI でずれる → `ViewHost::
  deliver_event(&Event)` を正本にして一本化する。
