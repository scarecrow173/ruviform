---
id: "DESIGN-034Uc9jupOZFWHdV6h98Xu"
type: "design"
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
        path: "crates/ruviform-ui-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/fake.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/scenario_contract.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/ipc.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-390-driver-contract-lab.md"
    - "docs/designs/issue-390-driver-contract-lab.md"
    - "docs/specs/issue-429-consumer-observation-seam.md"
    - "docs/designs/issue-429-consumer-observation-seam.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #464 — Consumer-neutral GUI Scenario Contract

## Context

`ruviform-ui` が driver-neutral な View/Command 契約（`ControlInput`、
`CommandIntent`/`CommandReceipt`/`CommandPhase`、`ComponentSnapshot`、
`AttachEpoch`/`generation`、`WindowId`/`ViewInstanceId`/`ComponentKey`）を
提供し、#390 が `ruviform-ui-test` に Fake driver・driver-lab・
boundary guard を、#429/#436/#442 が `ruviform-ui-gpui` に
`GpuiObserver`（`dispatch_input`/`observe`/`drawn`/`intents`/`settle`/
`select_row`/`select_tab`/`press_tab_key`/`invoke`）を公開した。

残る欠落は「consumer が driver 差を吸収して同じ scenario を書くための
薄い契約層」である。`GpuiObserver` は `Window` を抱え、Fake 側には
`ViewHost` map があるが、両者を束ねる target 解決・capability 照合・
receipt drain・verdict/report の共通語彙が無い。

## Scope

### In Scope

- `ruviform-ui-test/src/scenario.rs`: Scenario ID・Target・Capability・
  Action・Result/Report・`SemanticGuiTest` trait・共有 parity scenario
  helper の定義。
- `ruviform-ui-test/src/scenario/fake.rs`: `ViewHost` map 上の Fake
  adapter。
- `ruviform-ui-gpui/src/gpui_driver/scenario.rs`: `GpuiObserver` 上の
  GPUI adapter（`gpui-test` feature 限定）。
- 両側の `tests/scenario_contract.rs` と mise lane。
- `ruviform-ui` の最小追加: `ViewHost::attach_epoch()` getter、
  `ViewHost::deliver_event(&Event)` による wire→view decode の一本化、
  `AttachEpoch` の serde derive。

### Out of Scope

- spec の Out of Scope と同一。JSON DSL・独自 executor・第二の
  gesture/state authority・GPUI 型の共通 API 露出・dependency 逆転。

## Components and Boundaries

- **Scenario Contract (`ruviform-ui-test::scenario`)**: driver-neutral
  な型と trait のみ。`ruviform-ui` の公開型のみを参照し、
  `ruviform-ui-gpui`・`gpui` 型を一切 import しない。公開 enum は
  `#[non_exhaustive]`。
- **Fake adapter (`ruviform-ui-test::scenario::fake`)**: `WindowId ->
  HashMap<ViewInstanceId, ViewHost>` 相当の薄い map を保持し、
  `ViewHost::{dispatch_input, select_row, select_tab, invoke, observe,
  spec, mounted_keys}` をそのまま呼ぶ。`ViewHost` のイベントは
  `ViewHost::deliver_event(&Event)`（新設）経由で wire `Event` を
  decode して既存の event 処理へ流す。
- **GPUI adapter (`ruviform-ui-gpui::gpui_driver::scenario`)**:
  `GpuiObserver` を包み、`dispatch_input`/`observe`/`intents`/`settle`/
  `select_row`/`select_tab`/`invoke`/`drawn` を `SemanticGuiTest` に
  写像する。`gpui::TestAppContext`・`Entity`・`Window`・`Context` は
  このモジュール内に閉じる。`ViewObservation` に view identity
  （`view_id`/`attach_epoch`/`generation`）を追加して target 解決の
  照合に使う。
- **`ruviform-ui` の最小追加 (`view.rs`/`ipc.rs`)**: `attach_epoch()`
  getter、`deliver_event`、`AttachEpoch` の serde derive。

## Data and Control Flow

1. consumer の `#[test]` が adapter（Fake or GPUI）を構築し、
   `SemanticGuiTest` trait の共有 scenario 関数を呼ぶ。
2. scenario は `TargetQuery`（`WindowId` + `ViewInstanceId` または
   `ComponentKey` + 任意の pinned `AttachEpoch`/`generation`）を
   `resolve_target` に渡し、adapter が `ScenarioTarget`
   （解決済み triple + `attach_epoch` + `generation` + mounted key
   集合）を返す。
3. `dispatch(ControlInput)` / `select_row` / `select_tab` / `invoke` /
   `observe(ComponentKey)` は操作前に target の
   `attach_epoch`/`generation`/mounted を照合し、stale なら
   `ScenarioError::Stale{..}` を返す。
4. `settle` は bounded な queue drain（Fake は `ViewHost` event、
   GPUI は `GpuiObserver::settle`）を行い、`drain_intents` が
   `MintedIntent`（`ViewEvent::Intent` → `CommandIntent` + receipt/
   projection）を返す。
5. `capabilities()` は adapter が実際にサポートする
   `ScenarioCapability` 集合を返し、scenario の required 集合と
   pre-flight 照合して不足なら `ScenarioVerdict::Unsupported`。
6. scenario は `ScenarioReport`（verdict + run/target/expected/
   observed/receipt/capability/evidence）を返す。

## Non-Functional Constraints

- API は synchronous・non-RT。GPUI `!Send` context を thread 越境
  させない。
- `TestDriver` の process/timeout/cleanup を再利用し、独自 runner を
  作らない。
- 公開 API の doctest を `cargo test` で検証する。
- 既存 `proto::Capabilities`（TestAgent wire 用）とは別物として
  `ScenarioCapability` を定義する。

## Risks and Trade-offs

- **`deliver_event` の decode 二重化リスク**: Fake adapter が
  `Event::Result`/`Event::Notify` を自前 decode すると GPUI 側と
  ずれる。`ViewHost::deliver_event(&Event)` を正本にして wire→view
  decode を一本化する。
- **GPUI adapter の `!Send` 制約**: `GpuiObserver` は `Rc<RefCell>`
  ベースで `!Send`。adapter も同 thread 内でのみ使い、`Send` を
  主張しない。
- **target 解決の曖昧さ**: `ComponentKey` だけでは view を特定でき
  ない場合があるため、`TargetQuery` は `WindowId` +
  `ViewInstanceId` を必須とし、`ComponentKey` はその view 内の
  component 指定に限定する。

## Deferred Design Concerns

- keyboard/IME・pointer 座標・pixel capture の拡張 action は
  `GuiAction` を `#[non_exhaustive]` にして後続 Issue で追加する。
- consumer 側 fixture/oracle の Ruviform への持ち込みは各 Consumer
  Issue の範囲。

## Verification Notes

- [ ] `mise run check-ui-driver-boundary` が pass（GPUI 型非露出）。
- [ ] `mise run test-ui-driver-contract` / `test-ui-parity` /
  `test-ui-consumer-observation` / `test-ui-component-view` /
  `test-ui-gpui` / `check-ui-gpui` / `lint-ui-gpui` が pass。
- [ ] `mise run test-ui-test-msrv` が pass。
- [ ] Fake/GPUI 両 adapter で同一 scenario 関数が pass するテストが
  ある。
- [ ] stale key/epoch/generation が明示 `ScenarioError` になる
  テストがある。
