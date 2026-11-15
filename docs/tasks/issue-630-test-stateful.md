---
id: "TASK-034Y4fNMwyna1imxqrp1rF"
type: "task"
status: "done"
title: "Issue #630 test_stateful slice — task"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-630-test-stateful.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-630-test-stateful.md"
  implements:
    - "docs/plans/issue-630-test-stateful.md"
  implemented-by:
    - "docs/impl/ir/issue-630-test-stateful.md"
  depends-on:
    - "docs/plans/issue-630-test-stateful.md"
    - "docs/tasks/issue-630-test-fault.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-test-stateful.md"
    - "evidence/rf630/test-stateful-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-630-test-stateful.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-630-test-stateful.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_stateful slice — task

## Work

両 fixture cdylib に state を持つ第 3 plugin class `test_stateful`
を追加し、実 ABI の save → mutate → restore → save round-trip を
in-process host-path test と L1 out-of-process runner test の両方で
検証する。

### VST3

- `stateful.rs` 新規: `SharedGain`（atomic f64 bits）、
  `TestStatefulProcessor`、blob codec（component=plain /
  controller=normalized）、`TestStatefulComponent`、COM-level
  `IBStream` tests。
- `component.rs`: `ComponentStateSurface` trait + optional surface
  で stateful class のみ `setState`/`getState` を実装。
- `controller.rs`: `new_stateful` + `setComponentState`（plain →
  normalized 変換）+ `setState`/`getState`。
- `factory.rs`: `countClasses` 8、index 6/7 = stateful pair。
- `gain.rs`: `apply_gain` を `pub(crate)` 化。
- `host_path.rs`: descriptor/factory 4、round-trip/malformed/
  stateless-decline tests。
- `l1_stateful_test_plugin_e2e.rs`: delivered-audio ordering gate
  付き round-trip + malformed。

### CLAP

- `stateful.rs` 新規: `TestStatefulShared`、`MainThread` +
  `PluginStateImpl`（`clap.ext.state` save/load）、persistent
  activation law、codec tests、RT test。
- `lib.rs`: factory 3 plugin dispatch。
- `Cargo.toml`: `clack-extensions` features に `"state"`。
- `host_path.rs`: descriptor/factory 3、round-trip/malformed/
  stateless-decline tests。
- `l1_stateful_test_plugin_e2e.rs`: round-trip + malformed。

### 共通

- `mise.toml`: `test-l1-real-plugin` に stateful e2e 行追加。
- 両 README 更新。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-630-test-stateful.md`
- `docs/specs/issue-630-test-stateful.md`
- `docs/designs/issue-630-test-stateful.md`
- `docs/designs/issue-630-adapter-state-bridge.md`
- `docs/test-specs/issue-630-test-stateful.md`

## Dependencies

- `issue-630-adapter-state-bridge`（production state bridge）
- `issue-630-test-fault`（fixture base / L1 runner pattern）
- `issue-630-test-headless`（scenario console pattern）

## Affected Files

- `crates/ruviform-plugin-test-vst3/src/{stateful,component,controller,factory,gain,lib}.rs`
- `crates/ruviform-plugin-test-vst3/tests/{host_path,l1_stateful_test_plugin_e2e}.rs`
- `crates/ruviform-plugin-test-vst3/README.md`
- `crates/ruviform-plugin-test-clap/src/{stateful,lib}.rs`
- `crates/ruviform-plugin-test-clap/Cargo.toml`
- `crates/ruviform-plugin-test-clap/tests/{host_path,l1_stateful_test_plugin_e2e}.rs`
- `crates/ruviform-plugin-test-clap/README.md`
- `mise.toml`
- `docs/specs|designs|plans|tasks|test-specs|impl/ir|impl/exp|evidence/rf630/`

## Done When

- VST3 factory が 8 class（gain/fault/editor/stateful × comp/ctrl）を
  expose し、stateful pair のみ state verb が実装される。
- CLAP custom factory が 3 plugin を expose し、`test_stateful` が
  `clap.ext.state` を宣言・実装する。
- production `Vst3Library`/`ClapLibrary` ABI 経由の in-process
  save → mutate → restore → save が byte-exact。
- L1 runner 経由の out-of-process round-trip + malformed
  rejection が実走・pass。
- stateless class は VST3 `kNotImplemented` / CLAP
  `HostError::Unsupported` を維持。
- malformed blob は `HostError::PluginFailed` で reject され、
  instance は生存する。
- `fmt-check`/`check`/`lint`/`test`/`check-l1`/`lint-l1`/
  `test-l1`/`test-rt`/`prek` が全て pass。
- doc chain（spec→design→plan→task→test-spec→impl/ir→impl/exp）
  と evidence が揃い、index が再生成されている。
