---
id: "PLAN-034Y4f3N8mEveC8nUJLKFw"
type: "plan"
status: "approved"
title: "Issue #630 test_stateful slice — 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/designs/issue-630-test-stateful.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-630-test-stateful.md"
  implements:
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/designs/issue-630-test-stateful.md"
  implemented-by:
    - "docs/tasks/issue-630-test-stateful.md"
  depends-on:
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/designs/issue-630-test-stateful.md"
    - "docs/plans/issue-630-adapter-state-bridge.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-630-test-stateful.md"
    - "docs/test-specs/issue-630-test-stateful.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/designs/issue-630-test-stateful.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-630-test-stateful.md"
  verified-by:
    - "docs/test-specs/issue-630-test-stateful.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_stateful slice — 実装計画

## File map

| file | 変更 |
| ---- | ---- |
| `crates/ruviform-plugin-test-vst3/src/stateful.rs` | 新規: `SharedGain`/`TestStatefulProcessor`/codec/`IBStream` helper/`TestStatefulComponent` + unit/COM-level tests |
| `crates/ruviform-plugin-test-vst3/src/component.rs` | `ComponentStateSurface` trait + `state_surface` field + `with_state` + 実 `setState`/`getState` dispatch（stateless は `kNotImplemented` 維持） |
| `crates/ruviform-plugin-test-vst3/src/controller.rs` | `new_stateful` + stateful state verbs（`setComponentState` plain→normalized 変換） |
| `crates/ruviform-plugin-test-vst3/src/factory.rs` | `countClasses` 6→8、index 6/7 = stateful comp/ctrl |
| `crates/ruviform-plugin-test-vst3/src/gain.rs` | `apply_gain` を `pub(crate)` 化 |
| `crates/ruviform-plugin-test-vst3/src/lib.rs` | `mod stateful` + docs 更新 |
| `crates/ruviform-plugin-test-vst3/tests/host_path.rs` | 4 descriptor/factory、stateful round-trip/malformed/stateless-decline |
| `crates/ruviform-plugin-test-vst3/tests/l1_stateful_test_plugin_e2e.rs` | 新規: round-trip + malformed（delivered-audio ordering gate） |
| `crates/ruviform-plugin-test-vst3/README.md` | 8 class 表 + `test_stateful` semantics + scope 更新 |
| `crates/ruviform-plugin-test-clap/src/stateful.rs` | 新規: `TestStatefulShared`/`MainThread`(+`PluginStateImpl`)/`AudioProcessor`/`TestStatefulClapPlugin` + codec + tests |
| `crates/ruviform-plugin-test-clap/src/lib.rs` | `mod stateful` + factory 3 plugin dispatch + docs 更新 |
| `crates/ruviform-plugin-test-clap/Cargo.toml` | `clack-extensions` features に `"state"` |
| `crates/ruviform-plugin-test-clap/tests/host_path.rs` | 3 factory/descriptor、stateful round-trip/malformed/stateless-decline |
| `crates/ruviform-plugin-test-clap/tests/l1_stateful_test_plugin_e2e.rs` | 新規: round-trip + malformed |
| `crates/ruviform-plugin-test-clap/README.md` | 3 plugin 表 + `test_fault`/`test_stateful` semantics + scope 更新 |
| `mise.toml` | `test-l1-real-plugin` に stateful e2e 行追加 |

## Dependency graph

fixture 実装（codec + state surface）→ host_path（in-process
oracle: production bridge 経由の round-trip）→ l1 e2e
（out-of-process oracle: delivered-audio ordering gate）→ mise lane
→ docs/evidence。production crate への依存なし（test crate →
production host bridge のみ、逆流なし）。VST3/CLAP 両 slice は
独立に実装可能だが同一 codec law を共有する設計判断のため同一
slice で実施。

## Risk register

- **L1 ordering race**: `process` submit は fire-and-forget で
  state verb が mutation block を追い越し得る → delivered-audio
  gate（3 つの distinct gain output で処理済み証明）で対処。
- **VST3 component/controller domain ずれ**: component=plain /
  controller=normalized の二 domain — `setComponentState` の
  変換を in-process COM-level test + host-path round-trip で検証。
- **CLAP `load` の trailing bytes**: fixture は exactly-16-byte
  stream を要求 — trailing は defined rejection（host bridge は
  raw bytes をそのまま渡すため plugin 側で検証）。
- **activation reset の混同**: `test_gain` の per-activation reset
  を `test_stateful` に誤って継承すると restore→activate で state
  が消える → `activation_preserves_loaded_gain` unit test +
  `prepare_preserves_cell_state` で固定。

## Verification matrix

`mise run fmt-check` / `check` / `lint` / `test`（両 crate）/
`check-l1-real-plugin` / `lint-l1-real-plugin` /
`test-l1-real-plugin`（stateful e2e 4 case 実走）/ `test-rt` /
`prek run --all-files` / `git diff` 禁止パス空。
