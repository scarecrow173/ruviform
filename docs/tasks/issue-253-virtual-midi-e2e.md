---
id: "TASK-034Yo3y0TUFqzL4AFs8iwx"
type: "task"
status: "in-progress"
title: "Issue #253 virtual-first OS-endpoint MIDI e2e verifier implementation task"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-253-virtual-midi-e2e.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-253-virtual-midi-e2e.md"
  implemented-by:
    - "docs/impl/ir/issue-253-virtual-midi-e2e.md"
  depends-on:
    - "docs/plans/issue-253-virtual-midi-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-253-virtual-midi-e2e.md"
    - "docs/designs/issue-253-virtual-midi-e2e.md"
    - "docs/test-specs/issue-253-virtual-midi-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-253-virtual-midi-e2e.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-virtual-midi-e2e.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #253 virtual-first OS-endpoint MIDI e2e verifier implementation task

## Work

`crates/ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs`
を新設し、実 WinMM backend → name-match loopback-style pair →
`InputTx::pump` → `InputStage` → `attach` → `PluginNode` → typed
`GraphRuntime` → out-of-process VST3 `test_instrument` → L1 PCM
oracle の合成 e2e を `#[ignore]`-gated・fail-closed で実装する。
`Cargo.toml` に `ruviform-midi-device` path dev-dep を追加し、
docs chain と `evidence/rf253/` を整備する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-253-virtual-midi-e2e.md`
- `docs/specs/issue-253-virtual-midi-e2e.md`
- `docs/designs/issue-253-virtual-midi-e2e.md`
- `docs/test-specs/issue-253-virtual-midi-e2e.md`

## Dependencies

- `attach`/`AttachConfig::input`/`InputTx::pump`/
  `InputStageCounters`（`ruviform-audio-plugin-bridge`、merge 済み）
- `ruviform_midi_device::backend()` の input/output enumerator
  （merge 済み）
- L1 fixture cdylib + `l1-runner` runner（`l1_real_test_plugin_e2e.rs`
  先例、merge 済み）
- `docs/plans/issue-253-virtual-midi-e2e.md`

## Affected Files

- `crates/ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs`
  （新設）
- `crates/ruviform-plugin-test-vst3/Cargo.toml`（dev-dep 追加）
- `Cargo.lock`（再生成差分のみ）
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/
  issue-253-virtual-midi-e2e.md` + `issue-253-virtual-midi-e2e-
  exp.jsonl`
- `evidence/rf253/`

## Done When

- [x] `os_endpoint_midi_e2e.rs` が `l1-runner` feature で compile
  し clippy `-D warnings` を通る。
- [x] fail-closed `NotRun` 経路（backend 不在/Unsupported/列挙空/
  pair 0 件/timestamp 非対応/stamp base 非対応）が assert なしで
  実装されている。
- [x] PCM/counter/close の独立 oracle が test-spec R4–R7 を満たす。
- [x] `virtual-os-endpoint` 非主張が verdict/docs/evidence に
  固定されている。
- [x] canonical mise lane（`fmt-check`/`check`/`lint`/`test`
  `ruviform-plugin-test-vst3`、`check-l1-real-plugin`/
  `lint-l1-real-plugin`/`test-l1-real-plugin`、`prek`）が Pass
  — 全て exit 0。
- [x] 補助実走（`--ignored`）の verdict が `evidence/rf253/` に
  記録されている（`NotRun — NOT_PROVISIONED`）。
- [x] review-protocol review で blocker が残っていない
  （ACCEPT with non-blocking notes）。

## Verification

- [x] `cargo check -p ruviform-plugin-test-vst3 --features
  l1-runner --tests`（補助）— Pass。
- [x] `cargo clippy -p ruviform-plugin-test-vst3 --features
  l1-runner --tests -- -D warnings`（補助）— Pass。
- [x] `cargo check -p ruviform-plugin-test-vst3`（補助）— Pass。
- [x] `mise run fmt-check` / `mise run check
  ruviform-plugin-test-vst3` / `mise run lint
  ruviform-plugin-test-vst3` / `mise run test
  ruviform-plugin-test-vst3` / `mise run check-l1-real-plugin` /
  `mise run lint-l1-real-plugin` / `mise run test-l1-real-plugin`
  / `mise exec -- prek run --all-files` — 全て exit 0。
- [x] `mise exec -- cargo test -p ruviform-plugin-test-vst3
  --features l1-runner --test os_endpoint_midi_e2e -- --ignored`
  （補助証拠）— exit 0、`verdict: NotRun — NOT_PROVISIONED`。
