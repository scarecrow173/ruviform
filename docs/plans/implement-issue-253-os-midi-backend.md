---
id: "PLAN-rf253osmidibackend001"
type: "plan"
status: "approved"
title: "Issue #253 OS MIDI input backend implementation plan"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-253-os-midi-backend.md"
    - "docs/specs/issue-253-os-midi-backend.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-253-os-midi-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/select-midi-input-backend.md"
    - "docs/test-specs/issue-253-os-midi-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-253-os-midi-backend.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-os-midi-backend.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Implementation Plan

正本: spec `docs/specs/issue-253-os-midi-backend.md`、design
`docs/designs/issue-253-os-midi-backend.md`、ADR
`docs/adr/select-midi-input-backend.md`、test-spec
`docs/test-specs/issue-253-os-midi-backend.md`。

## File map

### 変更（audio-core additive）

- `crates/ruviform-audio-core/src/midi_input.rs`:
  `MidiPortOrigin::Unknown` variant 追加、`MidiReadStats.unsupported`
  field + `with_unsupported` constructor 追加、同 crate 内 test の
  struct literal 追従。

### 新規（ruviform-midi-device）

- `crates/ruviform-midi-device/Cargo.toml`
- `crates/ruviform-midi-device/README.md`
- `crates/ruviform-midi-device/src/lib.rs`
- `crates/ruviform-midi-device/src/system.rs`
- `crates/ruviform-midi-device/src/midi.rs`（pure decode + tests）
- `crates/ruviform-midi-device/src/queue.rs`（pure queue + tests）
- `crates/ruviform-midi-device/src/os/mod.rs`
- `crates/ruviform-midi-device/src/os/windows.rs`（cfg(windows)、tests 含む）
- `crates/ruviform-midi-device/src/os/unsupported.rs`（cfg(not(windows))）
- `crates/ruviform-midi-device/tests/`（公開面・結線）
- `crates/ruviform-midi-device/examples/list_midi_ports.rs`（手動）

### workspace / docs

- `Cargo.toml`（members 追加）
- `crates/README.md`（索引追加）
- `Cargo.lock`（再生成）
- `docs/impl/ir/implement-issue-253-os-midi-backend.md`（実装後）

## Ordered tasks

1. audio-core additive 変更 + 追従修正
2. crate 骨格（Cargo.toml/README/lib.rs/system.rs/os dispatch）
3. pure module（midi.rs decode、queue.rs）+ 単体テスト
4. windows.rs WinMM 実装 + open 失敗テスト
5. unsupported.rs stub
6. tests/ 結線テスト + example
7. workspace member + crates/README + Cargo.lock
8. 検証 lane 実行・修正
9. IR doc + task doc 完了 + review-protocol

## Risk register

- WinMM timestamp が per-stream ms であることの誤解釈 → design に
  明記済み、per-subscription domain 採番で対処。
- callback 内 OS 呼出禁止の違反 → requeue を read 側へ移動済み。
- `MIM_MOREDATA` の意味解釈 → 喪失 signal として dropped 計数、
  実機検証は NotRun として記録。
- unsafe FFI → `#[expect(unsafe_code)]` 局所許可 + SAFETY コメント。
- mid-stream device-loss 検出不能 → 制限として design/IR/PR に明記。

## Verification matrix

| lane | command | 期待 |
|---|---|---|
| fmt | `mise run fmt-check` | Pass |
| check | `mise run check` | Pass |
| lint | `mise run lint` | Pass |
| test | `mise run test` | Pass |
| rt | `mise run test-rt` | Pass |
| cross | `cargo check -p ruviform-midi-device --target x86_64-unknown-linux-gnu` | Pass（Unsupported stub）— 補助証拠 |
