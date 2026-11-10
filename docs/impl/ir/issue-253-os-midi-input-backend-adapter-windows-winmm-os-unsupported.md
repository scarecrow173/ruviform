---
id: "IMPL-034Vi78tpdb4wvJ1oTJpLb"
type: "impl"
status: "completed"
title: "Issue #253 OS MIDI input backend adapter（Windows WinMM + 非対応 OS Unsupported 境界）"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-midi-device/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-midi-device/README.md"
      - type: "file"
        path: "crates/ruviform-midi-device/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/src/midi.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/src/queue.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/src/system.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/src/os/mod.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/src/os/windows.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/src/os/unsupported.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/tests/ports.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/examples/list_midi_ports.rs"
      - type: "file"
        path: "docs/specs/issue-253-os-midi-backend.md"
      - type: "file"
        path: "docs/designs/issue-253-os-midi-backend.md"
      - type: "file"
        path: "docs/adr/select-midi-input-backend.md"
      - type: "file"
        path: "docs/plans/implement-issue-253-os-midi-backend.md"
      - type: "file"
        path: "docs/test-specs/issue-253-os-midi-backend.md"
      - type: "file"
        path: "docs/tasks/issue-253-os-midi-backend.md"
      - type: "file"
        path: "docs/impl/ir/issue-253-os-midi-input-backend-adapter-windows-winmm-os-unsupported.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "crates/ruviform-audio-core/src/midi_input.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/adr/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-253-os-midi-backend.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-253-os-midi-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-253-os-midi-backend.md"
    - "docs/designs/issue-253-os-midi-backend.md"
    - "docs/adr/select-midi-input-backend.md"
    - "docs/test-specs/issue-253-os-midi-backend.md"
    - "docs/plans/implement-issue-253-os-midi-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-253-os-midi-backend.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-253-os-midi-backend.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #253 OS MIDI input backend adapter（Windows WinMM + 非対応 OS Unsupported 境界）

## Summary

Issue #253 slice (a)。`ruviform_audio_core::midi_input` の 3 trait
（`MidiInputEnumerator` / `MidiInputPort` / `MidiInput`）を実装する
OS MIDI input adapter を新規 crate `ruviform-midi-device` として追加
した。Windows は WinMM（`windows-sys` 経由）で実 port 列挙・open・
`CALLBACK_FUNCTION | MIDI_IO_STATUS` subscription・channel-voice
decode・driver timestamp 保持・bounded queue drain までを実装し、
macOS/Linux は `DeviceError::Unsupported` を返す明示境界のみを
用意した。audio-core には additive 変更のみ
（`MidiPortOrigin::Unknown`、`MidiReadStats.unsupported`）。

## Context

Issue #253。spec → design → ADR（`docs/adr/select-midi-input-backend.md`）
→ plan → test-spec → task の文書チェーン
（`docs/{specs,designs,plans,test-specs,tasks}/issue-253-os-midi-backend.md`）
に従う。Issue #253 全体は open のまま — 本 slice は Windows backend +
非対応 OS 境界のみで、macOS/Linux backend・opt-in 実機 lane・実 DAW
往復・host-graph consumer 配線・MIDI2/UMP/MPE は後続 slice。

## Implementation

### Changes

- `crates/ruviform-audio-core/src/midi_input.rs`
  - `MidiPortOrigin` に `Unknown` を追加（WinMM は external/virtual を
    区別できないため、実 backend は `Unknown` を返す）。
  - `MidiReadStats` に `unsupported: u64` と
    `with_unsupported(dropped, unsupported)` を追加。
    `new(dropped)` は `unsupported = 0` を設定（後方互換の既定値）。
- `crates/ruviform-midi-device/`（新規 crate、workspace member 追加）
  - `src/midi.rs`: MIDI 1.0 packed-byte → `MidiEvent` の pure decoder。
    channel-voice（0x8n–0xEn）のみ受理、status<0x80・system
    common/realtime・SysEx status・data byte>0x7F を拒否。
    NoteOn velocity 0 は `NoteOn` のまま（`NoteOff` へ読み替えない）。
  - `src/queue.rs`: `Mutex` + 事前確保 `VecDeque` の bounded queue
    （capacity 1024）。callback 側は alloc なし。overflow で
    `dropped++`、翻訳不能 traffic で `unsupported++`。
    counter は read drain 時に回収・reset。
  - `src/system.rs`: `SystemBackend` が `MidiInputEnumerator` を実装。
    OS 別 dispatch は `src/os/` に cfg 分離。
  - `src/os/windows.rs`: WinMM backend。
    `midiInGetNumDevs`/`midiInGetDevCaps` で列挙、capabilities は
    `MidiPortOrigin::Unknown`・`supports_ump=false`・
    `provides_timestamps=true`・per-port `ClockDomainId`・
    `TimeBase` 1/1000s・`TimestampProvenance::Carried`。
    open は `DeviceBusy`（one-open-per-handle 契約）、driver 異常は
    `DeviceError` へ正規化。`MM_MIM_DATA` の `dwParam1` packed-byte
    decode・`dwParam2` raw timestamp 保持。`MM_MIM_MOREDATA`→
    `dropped`、`MM_MIM_ERROR`/`MM_MIM_LONGERROR`/`MM_MIM_LONGDATA`/
    decode 不能 `MIM_DATA`→`unsupported`、`MM_MIM_CLOSE`→dead。
    SysEx header は callback 内で OS call せず read 側で requeue。
    `read` は dead 後も queued event と最終 counter を drain してから
    `StreamInvalidated`。`close` は冪等・Drop 安全。
  - `src/os/unsupported.rs`: 非 Windows で `DeviceError::Unsupported`
    を返す stub（module は `#[cfg(not(windows))]`、列挙は空 Vec ではなく
    port open/info で Unsupported を返す構造）。
  - `tests/ports.rs`: `Box<dyn MidiInputEnumerator>`/
    `Box<dyn MidiInputPort>`/`Box<dyn MidiInput>` 経路、列挙契約、
    open 失敗、Unsupported 境界、device 不在環境で Pass。
    実機依存は `#[ignore]`。
  - `examples/list_midi_ports.rs`: 手動列挙 helper（CI 対象外）。
- `Cargo.toml` / `Cargo.lock`: workspace member + `windows-sys 0.61.2`
  （`Win32_Media_Audio` + `Win32_Media_Multimedia`、Windows target のみ）。
- `crates/README.md`: crate 数 38→39、索引追加。
- docs index（`docs/{specs,designs,adr,plans,tasks,test-specs}/README.md`）
  を doc_maintenance で再生成。

### Decisions During Implementation

- **adapter 配置**: 新規 `ruviform-midi-device` crate を選択。
  OS MIDI は audio-device adapter（cpal）とは別ドメインで、依存方向
  （adapter→foundations のみ）を保つため。`midir` ではなく raw WinMM
  を `windows-sys` で直接呼ぶ — `dwParam2` timestamp・`MIM_MOREDATA`/
  `MIM_LONGERROR` 等の message semantics・SysEx header 管理を wrapper が
  握りつぶすと defined-drop 契約を満たせないため（ADR 記録）。
- **timestamp**: `dwParam2`（`midiInStart` からの ms）を raw のまま
  `InputStamp` に載せ、`TimeBase` 1/1000s・`Carried` provenance・
  per-port `ClockDomainId`（atomic 採番、well-known ID より上）を宣言。
  sample clock への変換は engine（`ruviform_media_core::timed_input`）
  の責務で adapter では行わない。
- **clock domain 採番点**: `MidiInputPort::info()` 時に per-port で採番。
  one-open-per-handle 契約上 1 port = 最大 1 subscription であり、
  再列挙は新 handle=新 domain となるため subscription 単位ではない。
- **message 分類**: `MIM_MOREDATA` のみ `dropped`（driver 側 queue
  overflow = overload loss）。`MIM_ERROR`/`MIM_LONGERROR`/`MIM_LONGDATA`/
  decode 不能 `MIM_DATA` は「受信したが翻訳不能」なので `unsupported`。
  review で `MIM_ERROR` の silent ignore と `MIM_LONGERROR` の
  dropped 誤分類を検出・修正済み。
- **SysEx buffer**: 4×1024B の `MIDIHDR` を open 時に prepare+queue、
  callback では `dwParam1` の header pointer を queue に渡すだけにし
  requeue は `read` 側で実施（callback 内 OS call 禁止の RT 契約）。
- **`MidiPortOrigin::Unknown`**: WinMM capabilities は port が external
  か virtual かを区別しないため `Unknown` を返す（`External` と偽らない）。
- **`MidiReadStats.unsupported`**: 既存 `dropped` のみでは「受信したが
  翻訳不能」と「queue 溢れ」を区別できないため additive に追加。
  `new(dropped)` は `unsupported=0` で既存呼び出しを壊さない。
- **非 Windows module cfg**: `midi.rs`/`queue.rs` は現状 Windows backend
  のみが消費するため `#[cfg(windows)]` で gate。macOS/Linux backend
  実装時に ungate する。

## Related Experiments

### Adopted

なし（backend 選定は ADR で確定。`midir` vs raw WinMM の比較は
design/ADR の代替案評価として記録）。

### Rejected

なし。

## Validation

canonical = `mise run` 経由。実施済み:

- `mise run fmt-check`: Pass（`cargo fmt --all -- --check`）。
- `mise run check`: Pass（workspace 全体）。
- `mise run lint`: Pass（`cargo clippy --workspace --all-targets -- -D
  warnings`、workspace 全体）。
- `mise run test`: Pass（workspace 全体、exit 0）。
- `mise run test-rt`: Pass（`rt_` filter、exit 0）。
- `cargo check -p ruviform-midi-device --target x86_64-unknown-linux-gnu`:
  Pass（非 Windows cfg 経路の compile 確認 — `mise run check-linux` は
  io-file/io-archive のみ対象のため crate 個別に実施）。
- `cargo test -p ruviform-midi-device`: unit 17 pass + 1 ignored、
  integration 2 pass + 1 ignored、doctest 1 pass。
- review-protocol self-review 1 回実施。blocking finding
  （`MM_MIM_ERROR` silent ignore / `MM_MIM_LONGERROR` 分類誤り）を
  修正済み、残 blocker なし。

NotRun / Unsupported（正直な記録）:

- 実 MIDI デバイスを接続した実機検証: NotRun（本環境に実デバイスなし。
  `examples/list_midi_ports` と `#[ignore]` smoke は手動 opt-in）。
- 実 DAW 往復・`InputStage` 実消費: 本 slice 対象外（adapter 提供まで）。
- macOS CoreMIDI / Linux ALSA・JACK・PipeWire backend: Unsupported
  境界のみ（後続 slice）。
- MIDI2/UMP/MPE: `supports_ump=false` で非対応を宣言（Issue #317）。
- device-loss 再作成・recovery: `StreamInvalidated` contract のみ。

## Risks

- WinMM timestamp `dwParam2` の timebase は driver 依存の解釈余地が
  ある（ms since `midiInStart` は仕様だが driver 実装差は実機でのみ
  検証可能）。`TimeBase` 1/1000s 宣言は仕様準拠。
- callback は Windows message thread から呼ばれ、`Mutex` 競合時の
  blocking は理論上あるが queue は bounded で push は alloc なし。
  RT thread ではなく OS callback thread のため許容範囲と判断。
- `MM_MIM_MOREDATA` の発生条件・頻度は driver 依存で、実機挙動は
  NotRun。

## Follow-ups

- macOS CoreMIDI backend（別 slice）。
- Linux ALSA/JACK/PipeWire backend（別 slice）。
- opt-in 実機検証 lane・実 DAW 往復（Issue #253 残件）。
- host-graph consumer 配線（`InputTx::pump` 経路との結線確認は
  別 slice で実施）。
- `midi.rs`/`queue.rs` の `#[cfg(windows)]` gate 解除（macOS/Linux
  backend 実装時）。
- Issue #253 は open のまま（本 slice で Close しない）。
