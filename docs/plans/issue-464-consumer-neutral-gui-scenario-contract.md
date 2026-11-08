---
id: "PLAN-034UcaoYWDDGPoQ9CBmEdc"
type: "plan"
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
      - type: "file"
        path: "docs/tasks/issue-464-consumer-neutral-gui-scenario-contract.md"
      - type: "file"
        path: "docs/test-specs/issue-464-consumer-neutral-gui-scenario-contract.md"
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
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #464 — Consumer-neutral GUI Scenario Contract Implementation Plan

## Goal

`ruviform-ui-test` に driver-neutral な Scenario Contract（`SemanticGuiTest`
trait・Target・Capability・Result/Report）を追加し、Fake と GPUI
adapter が同一契約で同じ Scenario を実行できることを示す。

## Scope

単一実装ストリーム。`ruviform-ui` の最小追加（`attach_epoch()`・
`deliver_event`・`AttachEpoch` serde）と `ruviform-ui-test` の
contract/fake adapter、`ruviform-ui-gpui` の GPUI adapter、両側
tests・mise lane・docs chain を一括で扱う。`ui-webview`/`ui`/`poc`
には触れない。

## Upstream Documents

- `docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md`
- `docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md`
- `docs/specs/issue-390-driver-contract-lab.md`
- `docs/designs/issue-390-driver-contract-lab.md`
- `docs/specs/issue-429-consumer-observation-seam.md`
- `docs/designs/issue-429-consumer-observation-seam.md`

## Design Inputs

- `docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md`

## File Map

- `crates/ruviform-ui-test/src/scenario.rs` - Scenario ID/Target/
  Capability/Action/Result/Report/`SemanticGuiTest` trait/共有 parity
  scenario helper。
- `crates/ruviform-ui-test/src/scenario/fake.rs` - `ViewHost` map 上の
  Fake adapter。
- `crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs` - `GpuiObserver`
  上の GPUI adapter（`gpui-test` feature 限定）。
- `crates/ruviform-ui-test/tests/scenario_contract.rs` - Fake 側
  scenario contract test。
- `crates/ruviform-ui-gpui/tests/scenario_contract.rs` - GPUI 側
  scenario contract test。
- `crates/ruviform-ui-test/src/lib.rs` - `scenario` module 公開。
- `crates/ruviform-ui-gpui/src/lib.rs` - `gpui_driver::scenario` 公開
  （feature gate）。
- `crates/ruviform-ui-gpui/src/gpui_driver/observe.rs` - `ViewObservation`
  に view identity 追加。
- `crates/ruviform-ui-gpui/src/render.rs` - observe seam の view
  identity 公開。
- `crates/ruviform-ui-gpui/Cargo.toml` - `ruviform-ui-test` dev-dep
  確認・`gpui-test` feature 配線。
- `crates/ruviform-ui/src/view.rs` - `ViewHost::attach_epoch()`・
  `ViewHost::deliver_event(&Event)`。
- `crates/ruviform-ui/src/ipc.rs` - `AttachEpoch` serde derive。
- `mise.toml` - scenario contract lane 追加。
- `docs/development-environment.md` - lane 追記。

## Implementation Sequence

- [ ] `ruviform-ui`: `AttachEpoch` serde derive・`ViewHost::attach_epoch()`・
  `ViewHost::deliver_event(&Event)` を追加。
- [ ] `ruviform-ui-test`: `scenario.rs` に ID/Target/Capability/Action/
  Result/Report/`SemanticGuiTest` trait を定義。
- [ ] `ruviform-ui-test`: `scenario/fake.rs` に Fake adapter を実装。
- [ ] `ruviform-ui-test`: `tests/scenario_contract.rs` で Fake 側の
  contract/stale/capability テストを追加。
- [ ] `ruviform-ui-gpui`: `observe.rs`/`render.rs` に view identity を
  追加し `gpui_driver/scenario.rs` に GPUI adapter を実装。
- [ ] `ruviform-ui-gpui`: `tests/scenario_contract.rs` で GPUI 側の
  contract テストを追加。
- [ ] `mise.toml`・`docs/development-environment.md` に lane を追加。
- [ ] 全検証 lane を実行。

## Dependencies and Risks

- `ruviform-ui` の `deliver_event` は wire `Event` を `ViewHost` の
  既存 event 処理へ decode する正本。Fake/GPUI 両 adapter がこれを
  共有することで decode の二重化を防ぐ。
- GPUI adapter は `gpui-test` feature 限定で `ruviform-ui-test` の
  `SemanticGuiTest` を実装する。`ruviform-ui-test` は
  `ruviform-ui-gpui` に依存しない（逆転禁止）。
- `GpuiObserver` の `!Send` 制約を維持し、`Send` を主張しない。

## Task Breakdown

- T1: `ruviform-ui` 最小追加（serde・getter・deliver_event）。
- T2: `ruviform-ui-test` scenario contract 型・trait。
- T3: `ruviform-ui-test` Fake adapter + test。
- T4: `ruviform-ui-gpui` observe seam view identity + GPUI adapter +
  test。
- T5: mise lane・docs・全検証。

## Verification

- [ ] `mise run fmt-check` / `check` / `lint` / `test`
- [ ] `mise run check-ui-driver-boundary`
- [ ] `mise run test-ui-driver-contract` / `test-ui-parity` /
  `test-ui-consumer-observation` / `test-ui-component-view` /
  `test-ui-gpui` / `check-ui-gpui` / `lint-ui-gpui`
- [ ] `mise run test-ui-test-msrv`
- [ ] Fake/GPUI 両 adapter で同一 scenario 関数が pass。
- [ ] stale key/epoch/generation が明示 `ScenarioError`。

## Open Questions / Gaps

- `TargetQuery` の最終的な field 構成（`ComponentKey` の optional 性）
  は実装時に `ViewHost`/`GpuiObserver` の API に合わせて確定する。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: no（review-protocol で代替）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `ruviform-ui` への追加が本当に不可避か（`attach_epoch` getter・
  `deliver_event`・serde derive）を reviewer が確認する。
- GPUI adapter が `gpui-test` feature でのみ有効で、通常 build に
  影響しないことを確認する。
