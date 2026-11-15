---
id: "PLAN-034XTg1js6FWqynnGrY6kw"
type: "plan"
status: "approved"
title: "Issue #629 MIDI output path plan"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/specs/issue-629-midi-output.md"
    - "docs/designs/issue-629-midi-output.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-629-midi-output.md"
      - type: "file"
        path: "docs/tasks/issue-629-midi-output.md"
      - type: "file"
        path: "docs/test-specs/issue-629-midi-output.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-midi-output.md"
  implemented-by:
    - "docs/tasks/issue-629-midi-output.md"
    - "docs/test-specs/issue-629-midi-output.md"
  depends-on:
    - "docs/specs/issue-629-midi-output.md"
    - "docs/designs/issue-629-midi-output.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-midi-output.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-midi-output.md"
    - "docs/designs/issue-629-midi-output.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-629-midi-output.md"
  verified-by:
    - "docs/test-specs/issue-629-midi-output.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 MIDI output path plan Implementation Plan

## Goal

`ruviform-audio-core` に `midi_output` port + scripted mock を、
`ruviform-midi-device` に `encode_message` codec と WinMM `midiOut*`
backend・`SystemBackend::output_ports` 配線を実装し、OS-L1 の
contract/mock 検証と docs chain・evidence を揃える。

## Scope

単一 implementation stream。Issue #629 の 2nd slice（`midiOut*`
送信経路の新設）のみ。loopback round-trip・probe 拡張・reconnect/
xrun・WASAPI・Test Runtime 結線・CI lane・consumer 照合は後続 slice
（spec Out of Scope 参照）。`mise.toml`・workflow・`Cargo.toml`・
`Cargo.lock`・`examples/`・`POLICIES.md`・`AGENTS.md` は変更しない
（依存追加不要を design で確認済み）。

## Upstream Documents

- `docs/specs/issue-629-midi-output.md`（approved）
- `docs/designs/issue-629-midi-output.md`（approved）
- `docs/discovery/issue-629-virtual-endpoint-survey.md`（gap #1・E2 crash）
- `docs/designs/issue-627-virtual-first-verification-contract-design.md`
- `POLICIES.md`（OS-L1/GUI-L2/EXT-L3・仮想優先・証拠要件）

## Design Inputs

- `docs/designs/issue-629-midi-output.md`（approved）

## File Map

- `crates/ruviform-audio-core/src/midi_output.rs` — `MidiOutputEnumerator`/
  `MidiOutputPort`/`MidiOutput`/`MidiOutputInfo` + 共有型の `pub use`。
- `crates/ruviform-audio-core/src/midi_output/mock.rs` — `MockBackend`/
  `MockPort` scripted fixture（sent 記録・fail_next_open/send）。
- `crates/ruviform-audio-core/src/lib.rs` — `pub mod midi_output;` 追加。
- `crates/ruviform-midi-device/src/midi.rs` — `encode_message` +
  round-trip/語彙外 unit test。
- `crates/ruviform-midi-device/src/os/windows_output.rs` — WinMM
  `midiOut*` backend（enumerate/open/send/send_sysex/close）。
- `crates/ruviform-midi-device/src/os/mod.rs` — `enumerate_output_ports`
  platform 選択。
- `crates/ruviform-midi-device/src/os/unsupported.rs` — output 版
  `DeviceError::Unsupported`。
- `crates/ruviform-midi-device/src/system.rs` — `MidiOutputEnumerator`
  impl + doc 更新。
- `crates/ruviform-midi-device/src/lib.rs` — crate doc を双方向に更新、
  `mod midi` の cfg を `any(windows, test)` に。
- `crates/ruviform-midi-device/tests/output_ports.rs` — contract/mock
  経路テスト + `#[ignore]` 実デバイス smoke。
- `docs/{specs,designs,plans,tasks,test-specs}/issue-629-midi-output.md`
  — docs chain。
- `docs/impl/{ir,exp}/issue-629-midi-output.md` + `*-exp.jsonl` — impl 記録。
- `evidence/rf629/` — 検証記録（slice 識別はファイル名）。
- `docs/*/README.md` — doc-maintenance / sync_impl_index 再生成。

## Implementation Sequence

- [x] T1: spec → design → plan → test-spec → task docs chain 作成・承認
- [ ] T2: `midi_output` port + `midi_output::mock`（audio-core）+ lib 公開
- [ ] T3: `encode_message` codec + unit/round-trip test（midi.rs）
- [ ] T4: `windows_output.rs` WinMM backend + `os/mod.rs`/`unsupported.rs`
- [ ] T5: `system.rs` `MidiOutputEnumerator` + `lib.rs` doc/cfg 更新
- [ ] T6: `tests/output_ports.rs` contract/mock + `#[ignore]` smoke
- [ ] T7: 検証 lane（fmt-check/check/lint/test/test-rt/prek）実行・
  evidence 記録
- [ ] T8: impl-doc（ir/exp）+ docs index 再生成 + review-protocol
- [ ] T9: commit/push/PR/worker_done

## Dependencies and Risks

- `midi_input` は read-only の鏡像参照元 — 変更せず、共有型は
  `crate::midi_input::{MidiPortOrigin, MidiPortCapabilities}` を再利用。
- `windows-sys 0.61` の `Win32_Media_Audio` feature で `midiOut*` API は
  充足（probe 使用実績あり）— `Cargo.toml`/`Cargo.lock` 変更不要。
- `midiOutMessage` + `DRV_QUERYDEVICEINTERFACE` は crash 既知 —
  呼ばない（PortKey は index+name のみ）。
- SysEx の `MHDR_DONE` は `CALLBACK_FUNCTION` + `MM_MOM_DONE` で受け、
  unprepare は done 確認後のみ（input の lazy requeue と対称）。
- `docs/*/README.md` index は生成物 — doc-maintenance /
  sync_impl_index で再生成。`Cargo.lock` 変更なしの見込み。

## Task Breakdown

- `docs/tasks/issue-629-midi-output.md`（T1〜T9 を内包）。

## Verification

- [ ] `mise run fmt-check` — Pass
- [ ] `mise run check ruviform-audio-core` — Pass
- [ ] `mise run check ruviform-midi-device` — Pass
- [ ] `mise run lint ruviform-audio-core` — Pass（`-D warnings`）
- [ ] `mise run lint ruviform-midi-device` — Pass
- [ ] `mise run test ruviform-audio-core` — Pass
- [ ] `mise run test ruviform-midi-device` — Pass
- [ ] `mise run test-rt` — 影響なし確認（新規 `rt_` 契約テストは
  codec が pure で既存 RT 契約を変えないため原則追加しない。追加した
  場合は収集・実行をテスト名で確認）
- [ ] `mise exec -- prek run --all-files` — Pass
- [ ] `test-midi-real-device` lane / MSRV lane — NotRun（指定どおり）

## Open Questions / Gaps

- `send_sysex` の `MHDR_DONE` 待ちを blocking wait にしない判断は
  design に記録（`DeviceBusy` 直列化）。異常系で done が来ない場合の
  `close`→`midiOutReset` 依存は MSDN 契約に基づく。
- index+name fallback の誤解決リスクは input と同じ制約として記録
  （interface-id は crash により不可）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker dispatch）
- Independent work streams: none
- User approval required before dispatch: review-protocol で代替
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `encode_message` の narrowing（`as_u7`/`bend_as_u14`）と
  `supports_ump=false` 宣言の整合。
- `send_sysex` の直列化・`MHDR_DONE` lazy unprepare の正しさ
  （use-after-free がないか）。
- `#[cfg(any(windows, test))]` on `mod midi` が 3 OS lane で
  dead code/lint を出さないか。
