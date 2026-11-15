---
id: "IMPL-034XTg5OfewvDOstfoNzy3"
type: "impl"
status: "completed"
title: "Issue #629 MIDI output path"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-core/src/midi_output.rs"
      - type: "directory"
        path: "crates/ruviform-audio-core/src/midi_output/"
      - type: "file"
        path: "crates/ruviform-midi-device/src/os/windows_output.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/tests/output_ports.rs"
      - type: "file"
        path: "docs/impl/ir/issue-629-midi-output.md"
      - type: "file"
        path: "docs/impl/exp/issue-629-midi-output.md"
      - type: "file"
        path: "docs/impl/exp/issue-629-midi-output-exp.jsonl"
      - type: "directory"
        path: "evidence/rf629/"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/src/midi.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/src/os/mod.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/src/os/unsupported.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/src/system.rs"
      - type: "file"
        path: "docs/specs/issue-629-midi-output.md"
      - type: "file"
        path: "docs/designs/issue-629-midi-output.md"
      - type: "file"
        path: "docs/plans/issue-629-midi-output.md"
      - type: "file"
        path: "docs/tasks/issue-629-midi-output.md"
      - type: "file"
        path: "docs/test-specs/issue-629-midi-output.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-629-midi-output.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/exp/issue-629-midi-output-exp.jsonl"
    - "docs/impl/exp/issue-629-midi-output.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-629-midi-output.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-629-midi-output-exp.jsonl"
    rejected: []
---

# Issue #629 MIDI output path

## Summary

`ruviform-audio-core` に `midi_output` port（`MidiOutputEnumerator`/
`MidiOutputPort`/`MidiOutput`/`MidiOutputInfo`）と deterministic mock
（`midi_output::mock`）を追加し、`ruviform-midi-device` に
`encode_message` codec・WinMM `midiOut*` backend
（`os/windows_output.rs`）・`SystemBackend::output_ports` 配線を実装
した。discovery gap #1「`midiOut*` 送信経路不在」を解消 — Issue #629
の 2nd slice。loopback round-trip・probe 拡張・reconnect/xrun・
WASAPI・Test Runtime 結線・CI lane・consumer 照合は後続 slice。

## Context

`docs/tasks/issue-629-midi-output.md`（plan
`docs/plans/issue-629-midi-output.md`、spec
`docs/specs/issue-629-midi-output.md`、design
`docs/designs/issue-629-midi-output.md`、test-spec
`docs/test-specs/issue-629-midi-output.md`）に従う。鏡像の正本は
`midi_input.rs`/`midi_input/mock.rs`、codec の相手は
`midi.rs::decode_message`、FFI 先例は `os/windows.rs`。

## Implementation

### Changes

- `crates/ruviform-audio-core/src/midi_output.rs`（新設）:
  `MidiOutputEnumerator::output_ports`、`MidiOutputPort::{info,open}`、
  `MidiOutput::{send,send_sysex,close}`。`MidiPortOrigin`/
  `MidiPortCapabilities` は `midi_input` から re-export（鏡像であって
  複製ではない）。`send_sysex` は default で
  `Err(DeviceError::Unsupported)` — long-message 非対応 backend の
  honest refusal。新規公開 enum なし（`#[non_exhaustive]` 要件は
  `MidiOutputInfo` struct に適用）。
- `crates/ruviform-audio-core/src/midi_output/mock.rs`（新設）:
  `MockBackend`/`MockPort` — `Arc<Mutex<PortState>>` で送信
  `MidiEvent`・SysEx bytes を順序記録（`sent_events`/`sent_sysex`/
  `sent_count`/`clear_sent`）、`fail_next_open`/`fail_next_send` の
  one-shot 故障注入（消費型・失敗 send は未記録）、close 後 send は
  `StreamInvalidated`。
- `crates/ruviform-audio-core/src/lib.rs`: `pub mod midi_output;`。
- `crates/ruviform-midi-device/src/midi.rs`: `pub(crate)
  encode_message(MidiEvent) -> Option<(u8,u8,u8)>` — `decode_message`
  の逆写像。`group != ZERO` と未知 `MidiKind`（`#[non_exhaustive]`
  wildcard）は `None`。値は `MidiValue::as_u7`/`bend_as_u14` の
  canonical narrowing。pitch bend は `u8::try_from` で 7-bit byte を
  取り出し（cast lint を suppress せず）。
- `crates/ruviform-midi-device/src/os/windows_output.rs`（新設）:
  `midiOutGetNumDevs`/`midiOutGetDevCapsW` 列挙（消失 device は skip、
  infallible `Vec`）、index+name fallback のみの port identity
  （`midiOutMessage`+`DRV_QUERYDEVICEINTERFACE` は **呼ばない** —
  1st slice E2 の crash 既知）、`midiOutOpen`+
  `CALLBACK_FUNCTION|MIDI_IO_STATUS`、`midiOutShortMsg` 送信、
  `MIDIHDR`+`midiOutPrepareHeader`/`midiOutLongMsg`/
  `MM_MOM_DONE` callback flag/`midiOutUnprepareHeader` の同期 SysEx
  送信、`midiOutReset`+`midiOutClose` teardown。module-level
  `#![expect(unsafe_code)]` + 各 SAFETY コメント。
- `crates/ruviform-midi-device/src/os/mod.rs`:
  `enumerate_output_ports` の platform dispatch 追加。
- `crates/ruviform-midi-device/src/os/unsupported.rs`:
  `enumerate_output_ports -> Err(DeviceError::Unsupported)`。
- `crates/ruviform-midi-device/src/system.rs`:
  `impl MidiOutputEnumerator for SystemBackend`（input と対称形）。
- `crates/ruviform-midi-device/src/lib.rs`: bidirectional 記述に更新、
  doctest に output enumeration 追加。
- `crates/ruviform-midi-device/tests/output_ports.rs`（新設）:
  `Box<dyn …>` trait-object 経路、honest capability
  （`Unknown`/`supports_ump=false`/`provides_timestamps=false`/
  `clock_domain=None`）、非 Windows `Unsupported`、`#[ignore]`
  実デバイス send smoke。

### Decisions During Implementation

- **SysEx を同期 `MM_MOM_DONE` wait で送る**: callback は allocation
  不可のため、完了通知は `Mutex<bool>`+`Condvar` の flag のみ。
  buffer（`MIDIHDR`+data copy）は send_sysex の stack frame で
  prepare→longmsg→done→unprepare まで生存させ、pool/lifetime 管理を
  持たない（KISS）。`&mut self` で send は直列化されるため flag の
  世代管理は不要 — 前 send の DONE 消費後にしか次の longmsg を出さ
  ない。device stall 時は control path が block することを doc に明記。
- **SysEx framing 違反・非対応 event の error 写像**: どちらも
  「この port で wire 表現できない」ため `DeviceError::Unsupported`
  に統一（`encode_message` の `None` → `send` も同じ）。
- **port identity は index+name のみ**: output 側に
  `DRV_QUERYDEVICEINTERFACE` 相当の安全な代替がなく、probe で
  `midiOutMessage` query が crash した実績があるため、spec どおり
  name 再検証 fallback に限定。
- **`encode_message` は `MidiEvent` を by-value で取る**:
  `Copy` 型で `decode_message` の triple-by-value と対称。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-629-midi-output-exp.jsonl` /
  `docs/impl/exp/issue-629-midi-output.md` — 本 slice の試行記録
  （codec round-trip・SysEx 同期 wait・error 写像の判断根拠）。

### Rejected

- なし（`midiOutMessage`+`DRV_QUERYDEVICEINTERFACE` は 1st slice E2 で
  crash 実績があり、そもそも試行していない — discovery gap の既知
  記録として継承）。

## Validation

Windows host（`windows-local`）、rust toolchain は mise 管理
（`mise.toml` pin）。canonical = mise task、cargo 直接実行なし。

| コマンド | 結果 |
| --- | --- |
| `mise run fmt-check` | Pass（exit 0） |
| `mise run check ruviform-audio-core` | Pass（exit 0） |
| `mise run check ruviform-midi-device` | Pass（exit 0） |
| `mise run lint ruviform-audio-core` | Pass（exit 0、`-D warnings` clippy） |
| `mise run lint ruviform-midi-device` | Pass（exit 0） |
| `mise run test ruviform-audio-core` | Pass（64 unit + 37 doc + 2 compile-fail） |
| `mise run test ruviform-midi-device` | Pass（unit 24（2 ignored）+ `output_ports` 2（1 ignored）+ `ports` 2（1 ignored）+ doctest 1） |
| `mise run test-rt` | Pass（workspace `rt_` filter、exit 0 — 新規 rt_ test なし、影響なし確認） |
| `mise exec -- prek run --all-files` | Pass（exit 0） |

NotRun（指定どおり）: MSRV lane（daily CI 管轄）、
`test-midi-real-device` lane（実デバイス前提 — `#[ignore]` smoke は
収集対象として存在、未実行）。`environment_kind` 証拠は OS-L1 のみ
— mock/codec/WinMM enumerate の Pass を `virtual-os-endpoint` や
実 OS L3 の Pass と主張しない。

証拠ファイル: `evidence/rf629/`（本 slice 分）。

## Risks

- `send_sysex` の `MM_MOM_DONE` wait は device stall で無期限 block
  し得る（control path の既知リスクとして doc 明記）。
- WinMM の `midiOutShortMsg` は queue acceptance のみ返し、device
  側消費・順序保証は driver 依存 — port contract が
  fire-and-forget として正直に宣言済み。
- index+name fallback は index 再編や同名 device で誤 device を開く
  可能性が残る（WinMM に安定 ID がない制約の正直な帰結）。

## Follow-ups

- loopMIDI/Windows MIDI Services provisioning 上の loopback
  send→receive round-trip（`virtual-os-endpoint` evidence）。
- `examples/midi_endpoint_probe.rs` の output enumeration 拡張
  （本 slice の lease 外）。
- 仮想 endpoint 経由の reconnect/xrun/permission 再現、WASAPI 側
  仮想 endpoint、Test Runtime（#628）結線、CI lane 統合、
  Synvelis/AudioLens consumer 照合 — いずれも後続 slice。
