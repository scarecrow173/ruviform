---
id: "DESIGN-034XTfmMpVaGOisECUyZkj"
type: "design"
status: "approved"
title: "Issue #629 MIDI output path design — midi_output port + WinMM midiOut* backend"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/specs/issue-629-midi-output.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-629-midi-output.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-midi-output.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-midi-output.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-629-midi-output.md"
    - "docs/test-specs/issue-629-midi-output.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-midi-output.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 MIDI output path design — midi_output port + WinMM midiOut* backend

## Context

Issue #629 の 2nd slice。1st slice（`docs/discovery/
issue-629-virtual-endpoint-survey.md`）が `ruviform-midi-device` を
input-only と確認し、discovery gap #1「`midiOut*` 送信経路不在」を
記録した。本設計はその gap を、input 側（`midi_input` port +
`os/windows.rs` WinMM backend）と対称の構造で埋める。

利用する既存契約（調査済み）:

- `ruviform_audio_core::midi_input` — `MidiInputEnumerator` /
  `MidiInputPort` / `MidiInput` の 3 object-safe trait、
  `MidiInputInfo` / `MidiPortOrigin` / `MidiPortCapabilities` /
  `MidiReadStats` / `TimedMidiInput`。`midi_input/mock.rs` の
  `MockBackend`/`MockPort` scripted fixture。
- `ruviform_media_core::midi` — `MidiEvent`/`MidiKind`（channel-voice
  のみ、`#[non_exhaustive]`）/`MidiChannel`/`MidiGroup`/`NoteKey`/
  `MidiController`/`MidiProgram`/`MidiValue`（canonical scale-to-max、
  `as_u7`/`as_u14`/`bend_as_u14` で規定の narrowing）。
- `ruviform_midi_device::midi::decode_message` — `pub(crate)` pure
  codec。`WinMM` `MIM_DATA` の packed status/data1/data2 → `MidiEvent`。
- `ruviform_midi_device::os::windows` — `#![expect(unsafe_code)]` +
  SAFETY コメントの WinMM FFI 先例、`PortKey`（Interface + Index
  fallback）、`caps_name`/`index_still_matches`、error 正規化
  （`MMSYSERR_*` → `DeviceError`）。
- `ruviform_midi_device::system::SystemBackend` — `input_ports()` が
  platform 選択（Windows: infallible `Vec` wrap、非 Windows:
  `DeviceError::Unsupported`）の先例。
- `windows-sys 0.61` `Win32::Media::Audio` — `midiOutGetNumDevs` /
  `midiOutGetDevCapsW` / `midiOutOpen` / `midiOutShortMsg` /
  `midiOutLongMsg` / `midiOutPrepareHeader` / `midiOutUnprepareHeader` /
  `midiOutReset` / `midiOutClose` / `HMIDIOUT` / `MIDIOUTCAPSW` /
  `MIDIHDR` / `MM_MOM_DONE` 系が既存 feature で利用可能（probe が
  `midiOutGetNumDevs`/`midiOutGetDevCapsW` を使用済み — Cargo.toml
  変更不要）。

重要な設計上の発見・制約:

- `midiOutMessage` + `DRV_QUERYDEVICEINTERFACE*` は本 host の driver で
  crash 済み（1st slice probe exp E2）。output 側の PortKey は
  **index+name fallback のみ**とし、interface-id 解決は実装しない。
- `MidiEvent`/`MidiKind` は channel-voice 語彙のみ（SysEx/System は
  意図的に不在 — input 側も `unsupported` 計数）。output の
  `send_sysex` は語彙拡張ではなく、長い MIDI message を byte 列のまま
  運ぶ経路として `midiOutLongMsg` に対応させる。
- WinMM の output timestamp は存在しない（`midiOutShortMsg` に timestamp
  引数なし）。output port の `MidiPortCapabilities` は
  `provides_timestamps=false` / `clock_domain=None` を正直に宣言する。
- `MidiPortOrigin` は WinMM が外部/仮想を区別できないため `Unknown`
  （input と同じ正直さ。`Virtual` 昇格はユーザー指定により対象外）。

## Scope

### In Scope

- `ruviform-audio-core`: `src/midi_output.rs`（port + info 型）、
  `src/midi_output/mock.rs`（scripted fixture）、`src/lib.rs` に
  `pub mod midi_output;`。
- `ruviform-midi-device`: `src/midi.rs` に `encode_message`、
  `src/os/windows_output.rs` 新設（WinMM output backend）、
  `src/os/mod.rs` に `enumerate_output_ports`、`src/os/unsupported.rs`
  に output 版 `Unsupported`、`src/system.rs` に
  `MidiOutputEnumerator` impl、`src/lib.rs` doc 更新、
  `tests/output_ports.rs`（contract/mock + `#[ignore]` smoke）。
- docs chain・`evidence/rf629/`。

### Out of Scope

- spec の Out of Scope と同一（loopback round-trip・probe 拡張・
  reconnect/xrun・WASAPI・Test Runtime 結線・CI 統合・origin 昇格・
  consumer 照合・macOS/Linux backend・`MidiKind` 語彙拡張）。

## Components and Boundaries

### `midi_output` port（`ruviform-audio-core`）

`midi_input` の鏡像。3 object-safe trait:

- `MidiOutputEnumerator::output_ports(&self) ->
  DeviceResult<Vec<Box<dyn MidiOutputPort>>>` — non-RT 列挙。
- `MidiOutputPort::info(&self) -> MidiOutputInfo` /
  `open(&self) -> DeviceResult<Box<dyn MidiOutput>>` — lightweight
  live handle、1 回だけ open。
- `MidiOutput::send(&mut self, event: MidiEvent) -> DeviceResult<()>` —
  channel-voice event の順序保持送信。`send_sysex(&mut self,
  bytes: &[u8]) -> DeviceResult<()>` — 語彙外の長い message（既定実装は
  `Err(DeviceError::Unsupported)`）。`close(&mut self)`。

`MidiOutputInfo { id, name, origin, capabilities }` は
`MidiInputInfo` と同形（`#[non_exhaustive]` + `new` コンストラクタ）。
`origin`/`capabilities` の型は `crate::midi_input::{MidiPortOrigin,
MidiPortCapabilities}` を再利用 — 同一概念の別型を作らない
（`pub use` で `midi_output::MidiPortOrigin` 経路も提供）。

thread model は input と同じ: 全て control-thread non-RT。`send` は
backend の OS API を呼ぶ（WinMM `midiOutShortMsg` は driver queue に
積むだけで RT thread を持たない）。

### `midi_output::mock`

`midi_input::mock` の鏡像。`MockBackend`（`MidiOutputEnumerator`）+
`MockPort`（`MidiOutputPort` + test handle、`Arc<Mutex<PortState>>`）。
`PortState` は `sent: Vec<MidiEvent>` / `sent_sysex: Vec<Vec<u8>>` /
`opened` / `closed` / `open_error` / `send_error` を保持。

- `push` 相当は存在しない（output は engine → device 方向）。代わりに
  `sent_events()` / `sent_sysex()` / `sent_count()` で Expected/Observed
  照合、`clear_sent()` で drain。
- `fail_next_open(error)` / `fail_next_send(error)` — one-shot 消費型の
  故障注入（input mock の `fail_next_open` と同じ semantics）。
- `open` 成功後の 2 回目は `DeviceError::DeviceBusy`、`close` 後の
  `send`/`send_sysex` は `DeviceError::StreamInvalidated`（input の
  read-after-close と対称）。

### `encode_message`（`ruviform-midi-device::midi`）

`decode_message(status, data1, data2) -> Option<MidiEvent>` の逆写像:

```rust
pub(crate) fn encode_message(event: &MidiEvent) -> Option<(u8, u8, u8)>
```

- `event.group != MidiGroup::ZERO` → `None`（MIDI 1.0 に group 概念なし）。
- `MidiKind` 各 variant を status byte（`0x80|kind|channel`）+ data1 +
  data2 に写像。`ProgramChange`/`ChannelPressure` は data2=0。
- 値は `MidiValue::as_u7` / `bend_as_u14` で canonical → MIDI 1.0 幅へ
  **規定の narrowing**（round-to-nearest）。port の
  `supports_ump=false` 宣言と対になる adapter 責務。
- `MidiKind` は `#[non_exhaustive]` — `_ => None` で将来 variant を
  語彙外として拒否（decode と同じ fail-closed）。
- pure・allocation-free・panic-free（全 field は検証済み newtype）。

### WinMM output backend（`src/os/windows_output.rs` 新設）

`os/windows.rs` は input 専用の 667 行であり、output を追記すると
責務が混在するため **専用ファイルを新設**（`os/mod.rs` が platform 選択）。
module-level `#![expect(unsafe_code)]` + SAFETY コメントは同じ。

- `enumerate_output_ports() -> Vec<Box<dyn MidiOutputPort>>` —
  `midiOutGetNumDevs` + `midiOutGetDevCapsW`（`MIDIOUTCAPSW`）。
  device 消失は skip（input と同じ infallible 列挙）。
- `WinMmOutputPort { info, index, name, opened: AtomicBool }` —
  PortKey は **index+name fallback のみ**（`midiOutMessage` 不使用）。
  `open` 時に `midiOutGetDevCapsW(index)` で name を再検証し、不一致・
  範囲外は `DeviceError::DeviceNotFound`。index reshuffle で別 device を
  open する既知の制約は input fallback と同じ（name が同じ別 driver
  は区別不能 — 記録のみ）。
- `info`: `id = "winmm-out:{index}:{name}"`、`origin = Unknown`、
  `capabilities = MidiPortCapabilities::midi1_unstamped()`
  （`supports_ump=false`, `provides_timestamps=false`,
  `clock_domain=None` — WinMM output に timestamp は存在しない）。
- `WinMmOutput { handle: HMIDIOUT, shared, pending_sysex, closed }`:
  - `open`: `midiOutOpen(&mut handle, index, 0, 0, CALLBACK_NULL)`。
    error 正規化は input と同じ対応表
    （`BADDEVICEID`/`NODRIVER`→`DeviceNotFound`、`ALLOCATED`→`DeviceBusy`、
    他→`Backend(&'static str)`）。
  - `send`: `encode_message(&event)` → `None` なら
    `DeviceError::Unsupported`（`group != ZERO` 等、port が宣言する
    `supports_ump=false` の範囲外）。`Some((s,d1,d2))` は
    `dwMsg = s | d1<<8 | d2<<16` に pack して `midiOutShortMsg`。
    `MMSYSERR_INVALHANDLE` → `StreamInvalidated`、他 error → `Backend`。
  - `send_sysex`: byte 列を検証（長さ ≥ 2、先頭 `0xF0`、末尾 `0xF7`、
    `u32` に収まる）→ 新規 `MIDIHDR`+data 確保 →
    `midiOutPrepareHeader` → `midiOutLongMsg` → `MHDR_DONE` を
    `CALLBACK_FUNCTION` の `midi_out_proc`（`MM_MOM_DONE`）で待つ
    のではなく、**input と同じ lazy requeue 型**: done 通知を
    `shared` の atomic flag で受け、次の `send_sysex`/`close` で
    `midiOutUnprepareHeader`。未 done の long message が残る間の
    `send_sysex` は `DeviceError::DeviceBusy`（1 本ずつ直列化）。
    `close`: `midiOutReset` → pending header unprepare →
    `midiOutClose`（idempotent、`Drop` からも）。
  - open は `CALLBACK_FUNCTION | MIDI_IO_STATUS` で行い、
    `midi_out_proc` は `MM_MOM_DONE` のみ処理（SysEx 完了通知）。
    short message 経路は callback を必要としない。

  **設計判断（SysEx の直列化）**: WinMM は long message buffer が
  `MHDR_DONE` になるまで解放できない。input の「requeue_if_done を
  read/close で lazy 実行」と対称に、output は「done 確認済みの
  header のみ unprepare」を `send_sysex`/`close` で行う。in-flight
  がある間の `send_sysex` は `DeviceBusy` — blocking wait は control
  path とはいえ無期限待機を避けるため採用しない。

### `SystemBackend` / platform 境界

- `os/mod.rs`: `enumerate_output_ports` を追加。Windows は
  `windows_output::enumerate_output_ports() -> Vec<…>`（infallible）、
  非 Windows は `unsupported::enumerate_output_ports() ->
  DeviceResult<…>` が `Err(DeviceError::Unsupported)`。
- `system.rs`: `impl MidiOutputEnumerator for SystemBackend` —
  `input_ports()` と同じ cfg wrap（Windows: `Ok(vec)`、他:
  `Err(Unsupported)`）。
- `lib.rs`: crate doc を双方向に更新。`mod midi` の cfg を
  `#[cfg(any(windows, test))]` に変更し、pure codec の unit test
  （encode/decode/round-trip）が全 OS lane で走るようにする
  （非 Windows 非 test build では module 非含有のため dead code なし）。

## Data and Control Flow

```
engine/consumer                      ruviform-midi-device                 WinMM
     │ output_ports() ──────────────► SystemBackend::output_ports()
     │                                └─ os::enumerate_output_ports()
     │                                     └─ windows_output::enumerate
     │                                          midiOutGetNumDevs/GetDevCapsW
     │◄────────── Vec<Box<dyn MidiOutputPort>> （info: id/name/origin/caps）
     │ port.open() ─────────────────► WinMmOutputPort::open
     │                                     resolve index+name → midiOutOpen
     │◄────────── Box<dyn MidiOutput>
     │ send(MidiEvent) ─────────────► encode_message → midiOutShortMsg
     │ send_sysex(&[u8]) ───────────► validate → prepare/LongMsg ──► driver
     │ close() ─────────────────────► midiOutReset/Unprepare/Close
```

mock 経路は同じ port object を `MockBackend`/`MockPort` で駆動し、
`sent_events()`/`sent_sysex()` が Expected/Observed の観測面になる
（独立 Oracle は test 側の期待列 — 自己参照にならない）。

## Non-Functional Constraints

- **RT contract**: port の全 method は non-RT（control thread、OS API
  呼出・alloc 可）。`encode_message`/`decode_message` は RT-suitable
  pure（`const`-compatible 整数 math、alloc なし、panic なし）。
  `send`/`send_sysex`/`close` は non-RT — WinMM driver queue に積む
  だけで audio thread には触れない。
- **unsafe**: Win32 FFI は `windows_output.rs` の module-level
  `#![expect(unsafe_code)]` + 各 SAFETY コメントに限定。core port/mock・
  codec に unsafe なし。
- **3 OS**: Win32 コードは全て `#[cfg(windows)]` 配下。非 Windows は
  `enumerate_output_ports` が `Unsupported` を返し compile/test が
  通る。
- **依存**: 新規依存・feature 追加なし（`windows-sys 0.61` 既存 feature
  で充足 — `Cargo.toml`/`Cargo.lock` 変更不要）。
- **証拠境界**: mock/codec/WinMM send path の Pass は `environment_kind`
  を `deterministic-backend` または OS-L1 の compile/link/contract
  範囲に留め、`virtual-os-endpoint` を主張しない。

## Risks and Trade-offs

- **index+name fallback の曖昧さ**: device index reshuffle・同名 device
  で誤 open の可能性。input fallback と同じ制約であり、`midiOutMessage`
  crash により interface-id が使えない以上の手段がない → name 再検証 +
  `DeviceNotFound` で fail-closed。長期的には Windows MIDI Services の
  endpoint id（別 API surface、feature 追加が必要）を後続で検討。
- **SysEx の `MHDR_DONE` 待ち**: driver が done を通知しない異常系で
  `send_sysex` が `DeviceBusy` を返し続ける → `close` の
  `midiOutReset` が pending buffer を done に戻す契約に依存
  （MSDN: reset は全 pending を done で返す）。`Drop` でも同じ teardown。
- **narrowing の情報損失**: UMP 幅の値（16-bit velocity 等）は
  `as_u7`/`bend_as_u14` で規定 narrowing される。`supports_ump=false`
  の正直な宣言と対で、silent truncation ではなく defined narrowing。
- **`#[cfg(any(windows, test))]` on `mod midi`**: codec を全 OS の
  unit test で走らせるための小さな cfg 変更。test build でのみ
  非 Windows にも module が現れ、dead code にはならない
  （`pub(crate)` fn は test 内で使用）。

## Deferred Design Concerns

- loopback send→receive round-trip（`virtual-os-endpoint`）—
  loopMIDI/WMS provisioning 前提。再開条件: provisioning 確立 +
  probe 拡張。無視時リスク: AC2 の実 OS L3 未達のまま。
- `midi_endpoint_probe` output enumeration 拡張 — lease 外。
  再開条件: 後続 slice の lease。無視時リスク: probe の production
  view が input のみのまま。
- 仮想 endpoint 経由の reconnect/xrun/permission（AC5）・WASAPI
  実走（AC1）— 後続 slice。
- `MidiPortOrigin::Virtual` 昇格・Windows MIDI Services endpoint id —
  API 判別手段の確立が前提。ユーザー指定により本 slice 対象外。
- Test Runtime（#628）結線・本番 pipeline 配線・consumer（Synvelis/
  AudioLens）照合 — 後続 slice / 別 Issue。
- macOS/Linux MIDI output backend — `Unsupported` 境界のまま、
  backend 選定 ADR が前提。
- `MidiKind` への SysEx/System variant 追加 — production vocabulary
  拡張の別 Issue。`send_sysex` の byte 経路はその前段として独立に
  有効。

## Verification Notes

- [x] spec の全 requirement が本設計の component に対応する
      （review-protocol で確認）。
- [ ] `encode_message`/`decode_message` round-trip・語彙外拒否の
      unit test が `mise run test ruviform-midi-device` で Pass。
- [ ] `midi_output` port/mock の contract test が
      `mise run test ruviform-audio-core` で Pass。
- [ ] WinMM backend の compile/link が Windows lane で Pass、
      非 Windows は `Unsupported` を返すことを code review +
      cfg 構造で確認（実機 non-Windows 実行は本 worktree では不可）。
- [ ] `#[ignore]` smoke が `test-midi-real-device` lane の収集対象
      として存在（実行は NotRun と記録）。
