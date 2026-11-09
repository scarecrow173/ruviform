---
id: "IMPL-034UIypTlJgH6ElxJp3gHE"
type: "impl"
status: "completed"
title: "issue-253 midi backend fixture and vst3 translation"
created: "2026-11-09"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-core/src/midi_input/mock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/midi.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-core/src/midi_input.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/cap.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/tests/midi_wiring.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/component.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/lib.rs"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-188-midi-node-wiring-and-chase.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-188-timed-midi.md"
    - "docs/designs/issue-188-timed-midi-detailed.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# issue-253 midi backend fixture and vst3 translation

## Summary

Issue #253 の headless slice。VST3 の `IEventList` ↔ canonical `EventKind::Midi`
双方向翻訳レイヤを実装し、OS backend の列挙・open・timestamp・drop を fake 化する
`MockBackend` fixture を `ruviform-audio-core::midi_input::mock` に追加した。
Windows/macOS/Linux 実 backend・実デバイス検証 lane は本 slice の対象外であり、
全て `NotRun` として記録する。

## Context

#188（PR #252）で `InputStage` → `PluginNode`/`CallSink` の block 実経路配線、
録音済み event replay、loop/seek chase は headless 検証済み。一方で Issue #253 が
残した gate は 2 つ: (1) OS MIDI backend の実装（実デバイス列挙・open・timestamp
取得→`InputStage` 投入）、(2) VST3 MIDI 翻訳。VST3 host bridge (`host.rs`) は
`ProcessData.inputEvents` を常に null にしており canonical `EventKind::Midi` は
adapter に届かず、plugin 側 (`component.rs`) も `inputEvents` を読んでいなかった。
本 slice は (2) の headless 部分と、(1) の substitutable port 契約を検証する fake
fixture を実装する。実 OS backend・実デバイス検証は実 OS Gate として `NotRun`。

## Implementation

### Changes

- `crates/ruviform-audio-core/src/midi_input/mock.rs` (new): `MockBackend` —
  `MidiInputEnumerator`/`MidiInput`/`MidiInputPort` 契約を満たす deterministic fake。
  port 登録順の列挙、`MidiInputInfo`/`MidiPortCapabilities`/`MidiPortOrigin` 伝播、
  open 成功・scripted 失敗・二重 open の busy 拒否、timestamp 付き batch 注入、
  backend 側 drop 注入と `MidiReadStats` 計上、bounded read、close 後の
  `DeviceError::StreamInvalidated`。状態は `Arc<Mutex<PortState>>` で共有し、
  非 RT（audio callback 外）専用として lock/alloc を許容する。
- `crates/ruviform-plugin-vst3/src/midi.rs` (new): VST3 `Event` ↔ canonical
  `MidiEvent` 翻訳層。host→plugin 方向は NoteOn/NoteOff/PolyPressure →
  `kNoteOnEvent`/`kNoteOffEvent`/`kPolyPressureEvent`、ControlChange/
  ChannelPressure(`kAfterTouch`)/PitchBend(`kPitchBend`, 14bit LSB/MSB)/
  ProgramChange(`kCtrlProgramChange`) → `kLegacyMIDICCOutEvent`。`MidiGroup != 0`
  と将来の `MidiKind` variant は defined skip。plugin→host 方向は bus 0・
  block 内 sample offset・channel 0..=15・pitch 0..=127・finite normalized value・
  tuning==0 を検証し、SysEx/`kDataEvent`・note-expression・chord/scale・
  key 復元不能な `kCtrlPolyPressure` を defined skip とする。normalized `f32`↔
  `u32` 変換は `[0,1]` scale、非 finite 拒否、有限範囲外 clamp（7bit lattice 上は
  semantic round-trip exact）。`BridgeEventList`（有界 `IEventList` COM）と
  `fill_event_list`/`collect_incoming` を提供し、`MidiBridgeStats` が
  translated/skipped を計上する。host 側項目は `#[cfg(feature = "host")]` に閉じ、
  plugin 側 incoming 経路は default build でも常時コンパイルする。
- `component.rs`: `process` が param 収集と `data.inputEvents` の
  `collect_incoming` を両方行い、`event_scratch` を 1 回だけ sort する。
  `MidiBridgeStats` を累積保持。
- `host.rs`: `prepare` で `BridgeEventList` を reserve し、`process` で canonical
  `EventKind::Midi` を `fill_event_list` で VST3 event へ翻訳して
  `ProcessData.inputEvents` に渡す。`outputEvents`/`outputParameterChanges` は
  引き続き null（plugin→host 出力 event は未実装で正直に残す）。no-alloc 契約を維持。
- `cap.rs`: `Backend::Vst3.midi_channel_voice` を `Unsupported` → `Declared` に更新
  （headless 翻訳は証明済みだが real-plugin/実 OS 検証ではないことを notes に明記）。
- `tests/midi_wiring.rs`: `mock_backend_drives_input_stage_to_plugin` — 列挙→
  metadata/capability 伝播→open→timestamp 付き注入→`InputTx::pump`→backend drop
  計上→`InputStage` 経由で plugin event 記録までの headless 統合 test、
  close 後の `StreamInvalidated` も検証。
- `midi.rs` tests: note-on field mapping、pitch bend LSB/MSB、program/CC/channel
  pressure mapping、全 kind の semantic round-trip、invalid field・unsupported
  event・nonzero group・out-of-block offset の defined skip、PPQ 計算、
  `BridgeEventList` COM round-trip、`fill_event_list`/`collect_incoming` の
  stats・offset・payload 保持。host feature 無効時も incoming 側 fixture
  （`TestEventList`）で default build を維持。

### Decisions During Implementation

- `kLegacyMIDICCOutEvent` は VST3 SDK 上「output event（plugin→host）」専用と明記
  されているが、CC/pitch-bend/channel-pressure/program-change の input event 型は
  存在せず `IMidiMapping`→param queue 経路が正規のため、本 slice では de-facto
  host practice に従い input direction でも `kLegacyMIDICCOutEvent` を使う。
  `IMidiMapping` 経由の CC→param 配送は fixture controller が未実装のため別 gate。
- fake backend は `ruviform-audio-core::midi_input::mock` に配置（`ruviform-audio-midi`
  crate は未作成のため、既存 port 契約の所有 crate に置くのが最小の整合変更）。
- host/plugin の feature 境界: `host.rs` は `#[cfg(feature = "host")]` でしか
  コンパイルされないため、outgoing 翻訳・`BridgeEventList`・`fill_event_list` も
  host feature に閉じ、workspace 既定 lint（feature 無効）で dead code を出さない。
  incoming 翻訳と独立 fixture は default build で常時検証する。
- VST3 `MidiKind` は `#[non_exhaustive]` — wildcard arm で未知 variant を defined
  skip とし、推測で key を復元しない。

## Related Experiments

### Adopted

(None yet.)

### Rejected

(None yet.)

## Validation

- `mise run fmt-check` — pass
- `mise run check` — pass（workspace `cargo check --all-targets`）
- `mise run lint` — pass（workspace `cargo clippy --all-targets -- -D warnings`）
- `mise run test` — pass（workspace `cargo test`、192 test-result ok / FAILED なし）
- `mise run lint-l1-real-plugin` — pass（`--features host` clippy `-D warnings`）
- `mise run test-l1-real-plugin` — pass（fixture cdylib を実ロードする L1 E2E
  3/3、host.rs の process 経路変更を実 plugin 経路で検証）
- `mise run test-rt` — pass（RT contract name-filter lane）
- MSRV scoped lane: `mise exec rust@1.89.0 -- cargo test -p
  ruviform-audio-core -p ruviform-audio-plugin-bridge -p ruviform-plugin-vst3
  --all-targets` および `-p ruviform-plugin-vst3 --all-features` — pass
  （専用 MSRV task はこれらの crate を対象に含まないため scoped 実行）
- `cargo test -p ruviform-plugin-vst3`（default features）— 39 pass /
  `--all-features` — 65 pass + L1 `#[ignore]` 3 本は専用 lane で実走済み

### NotRun（実 OS/実デバイス lane — 本 slice 対象外）

- Windows/macOS/Linux 実 MIDI backend 実装（列挙・open・timestamp 取得）— NotRun
- 実 MIDI デバイスでの列挙・open・native timestamp 精度検証 — NotRun
- `#114 l1-real-verify` パターンの実デバイス opt-in 検証 lane — NotRun
- 実 VST3 host（DAW）上での plugin↔host MIDI event 往復 — NotRun
  （fixture cdylib の in-process L1 E2E のみ実走）

## Risks

- `kLegacyMIDICCOutEvent` を input direction で使うのは de-facto 慣行への依存であり、
  厳格な host が拒否する可能性がある。`IMidiMapping` 経由の正規経路は未実装。
- normalized `f32` ↔ `u32` は 7bit lattice 上 semantic exact だが任意の full-range
  `u32` では bit-exact でない（narrowing は文書化済み）。
- `BridgeEventList` は `MAX_BLOCK_EVENTS` 有界だが、host が巨大な event 数を渡す
  場合の overflow は stats の skipped で観測するのみ。

## Follow-ups

- Windows/macOS/Linux 実 MIDI backend 実装と `#114 l1-real-verify` 準拠の
  実デバイス検証 lane（Issue #253 残件・実 OS Gate）。
- `IMidiMapping`/`IUnitInfo` を持つ controller での CC→param 正規配送経路。
- plugin→host `outputEvents`（NoteOn/NoteOff/PolyPressure/`kLegacyMIDICCOutEvent`）
  と output parameter reporting。
- MIDI2/UMP/MPE generic capability は Issue #317 へ分離済み。
- Issue #253 の non-blocking follow-up（V9 truncation / V11 InputStage ledger /
  V12 anchor republish / V13 CC/param chase deferred contract）の包含判断。
