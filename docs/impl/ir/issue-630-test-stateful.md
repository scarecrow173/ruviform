---
id: "IMPL-034Y4g5Cx7ToOHTcgE2U9x"
type: "impl"
status: "completed"
title: "Issue #630 test_stateful slice — 実装記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-630-test-stateful.md"
    - "docs/plans/issue-630-test-stateful.md"
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/designs/issue-630-test-stateful.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/stateful.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_stateful_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/stateful.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_stateful_test_plugin_e2e.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/component.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/controller.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/factory.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/gain.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/README.md"
      - type: "file"
        path: "mise.toml"
  implements:
    - "docs/tasks/issue-630-test-stateful.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-630-test-stateful.md"
    - "docs/plans/issue-630-test-stateful.md"
    - "docs/plans/issue-630-adapter-state-bridge.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-test-stateful.md"
    - "evidence/rf630/test-stateful-verification.md"
    - "docs/impl/exp/issue-630-test-stateful.jsonl"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-630-test-stateful.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-630-test-stateful.jsonl"
    rejected: []
---

# Issue #630 test_stateful slice — 実装記録

## Summary

両 fixture cdylib に state を持つ `test_stateful` class を追加。
VST3 は 8 class factory（stateful comp/ctrl が index 6/7）、
CLAP は 3 plugin factory（`test_gain`/`test_fault`/`test_stateful`）。
production host bridge（`Vst3Library`/`ClapLibrary`）経由の
in-process round-trip + malformed rejection + L1 out-of-process
E2E を実装・検証した。

## Context

`docs/tasks/issue-630-test-stateful.md`（status: completed）→
`docs/plans/issue-630-test-stateful.md` → `docs/specs/
issue-630-test-stateful.md` + `docs/designs/issue-630-test-stateful.md`。
`issue-630-adapter-state-bridge` が production state bridge を
提供済みであり、本 slice はその独立 oracle を fixture 側に実装する。

## Implementation

### Changes

**VST3** (`crates/ruviform-plugin-test-vst3/`):

- `src/stateful.rs`（新規）: `SharedGain`（`Arc<AtomicU64>` —
  plain gain f64 bits）、16-byte blob codec
  （component=plain / controller=normalized）、`IBStream`
  read/write helper、`TestStatefulProcessor`、
  `TestStatefulComponent`、COM-level state-verb tests。
- `src/component.rs`: `ComponentStateSurface` trait
  （`set_state`/`get_state` の `IBStream` 面）導入、
  `TestComponent` に `Option<Arc<dyn ComponentStateSurface>>`
  追加、`with_state(...)` constructor。stateless class は
  `state_surface=None` → `kNotImplemented` 維持。
- `src/controller.rs`: `ParamController::new_stateful` +
  stateful state verbs（`setComponentState` で plain→normalized
  変換、`setState`/`getState` は normalized blob 往復）。
- `src/factory.rs`: `countClasses` 6→8、index 6/7 = stateful
  component/controller（fresh `uid`、既存 CID と非衝突）。
- `src/gain.rs`: `apply_gain` を `pub(crate)` 化（DSP law
  再利用）。
- `src/lib.rs`: `mod stateful` + docs 更新。
- `tests/host_path.rs`: descriptor/factory 4、stateful
  round-trip（byte-exact）、malformed（truncated/trailing/
  bad-magic/NaN → `PluginFailed` + instance survival）、
  stateless-decline tests。
- `tests/l1_stateful_test_plugin_e2e.rs`（新規）:
  delivered-audio ordering gate 付き round-trip + malformed。
- `README.md`: 8 class 表 + `test_stateful` semantics + scope
  更新。

**CLAP** (`crates/ruviform-plugin-test-clap/`):

- `src/stateful.rs`（新規）: `TestStatefulShared`、
  `TestStatefulMainThread`（ports + params +
  `PluginStateImpl`）、`TestStatefulAudioProcessor`、
  `TestStatefulClapPlugin`（descriptor
  `org.ruviform-test.clap-test-stateful`、`declare_extensions`
  に `PluginState` 登録）、codec tests、activation-preserves-
  gain test、`rt_` allocation-free test。
- `src/lib.rs`: `mod stateful` + factory 3 plugin dispatch +
  docs 更新。
- `Cargo.toml`: `clack-extensions` features に `"state"`。
- `tests/host_path.rs`: descriptor/factory 3、stateful
  round-trip（byte-exact）、malformed、stateless-decline tests。
- `tests/l1_stateful_test_plugin_e2e.rs`（新規）: round-trip +
  malformed。
- `README.md`: 3 plugin 表 + `test_fault`/`test_stateful`
  semantics + scope 更新。

**共通**:

- `mise.toml`: `test-l1-real-plugin` に stateful e2e 行追加。

### Decisions During Implementation

- **`ComponentStateSurface` trait**: VST3 component の
  `setState`/`getState` を stateful class のみ実装するため、
  `TestComponent` に optional surface を追加し stateless class
  は `kNotImplemented` を維持（既存
  `state_stream_reports_not_implemented` test がそのまま通る）。
- **`apply_gain` `pub(crate)` 化**: DSP law の単一正本を維持する
  ため `gain.rs` の `apply_gain` を stateful processor から再利用。
- **CLAP `#[global_allocator]` 重複回避**: `stateful.rs` で
  `assert_no_alloc::AllocDisabler` を重複定義すると「cannot
  define multiple global allocators」→ `plugin.rs` が crate-wide
  allocator を所有するため stateful.rs からは除去。
- **`core::fmt::Write` import**: `ParamDisplayWriter` への
  `write!` で `core::fmt::Write` が必要 → `use
  core::fmt::Write as _;` で `std::io::Write` と共存。
- **L1 ordering gate = delivered audio**: L1 data plane は
  lookahead（2 block）+ bounded drain を持ち、`SaveState`/
  `RestoreState` は FIFO stdin loop の同期 command だが
  `process` submit は fire-and-forget で state verb が mutation
  event block を追い越し得る。fixture の 3 gain（1.0/0.75/1.75）
  は constant input `0.5/-0.25` に対し pairwise-distinct な
  output を生むため、期待 gain の delivered output 観測 = それ
  以前の全 block が child で処理済みの証明になる。各 state verb
  はこの gate の後にのみ送出する。
- **Persistent-state law**: `test_gain` の activation reset を
  `test_stateful` に継承すると restore→activate で state が
  消える → `activate_inner` は mirror を reset せず、`load`/
  restore で書き込まれた値を保持する。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-630-test-stateful.jsonl` — codec law、
  state surface design、L1 ordering gate の確定事項を記録。

### Rejected

- なし。

## Validation

実行済み検証（全て pass）:

- `mise run fmt-check` — pass
- `mise run check`（両 crate）— pass
- `mise run lint`（両 crate）— pass（`clippy -- -D warnings`）
- `mise run test`（両 crate）— VST3 unit 70 pass / CLAP unit 35
  pass
- `mise run test`（host_path）— VST3 15 pass / CLAP 10 pass
- `mise run check-l1-real-plugin` — pass
- `mise run lint-l1-real-plugin` — pass
- `mise run test-l1-real-plugin` — stateful e2e 実走 pass
- `mise run test-rt` — workspace-wide pass
- `mise exec -- prek run --all-files` — pass
- `git diff` 禁止パス（production crate）— 空

## Risks

- **L1 ordering race**: delivered-audio gate で排除済みだが、
  L1 runner の lookahead 実装が将来変更された場合は再検証が必要。
- **VST3 controller domain 変換**: `setComponentState` の
  plain→normalized 変換は `ParamInfo::normalize` に委譲 —
  production bridge の送信順序と fixture の decode domain が
  一致することを in-process test で検証済み。
- **CLAP `load` の trailing bytes**: fixture は exactly-16-byte
  stream を要求 — trailing は defined rejection。

## Follow-ups

- `test_instrument` / CLAP `test_editor` class — 後続 slice
  （Issue #630 の残 acceptance）。
- 外部 SDK validator（VST3 SDK validator / CLAP validator）—
  Deferred 継続。
- preset/bank・GUI state・foreign format migration — Non-Goal。
