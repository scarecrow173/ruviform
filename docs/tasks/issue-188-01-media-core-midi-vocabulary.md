---
id: "TASK-5dlmLfhxOjW0IccOc3xUBO"
type: "task"
status: "done"
title: "media-core: midi + timed_input vocabulary, mapper and NoteLedger"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-188-timed-midi.md"
  implemented-by:
    - "../impl/ir/issue-188-timed-midi.md"
  depends-on:
    - "docs/plans/issue-188-timed-midi.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-188-timed-midi.md"
  references: []
  defers: []
  deferred-by: []
---

# media-core: midi + timed_input vocabulary, mapper and NoteLedger

## Work

`ruviform-media-core` に `midi` と `timed_input` モジュールを追加する。

- `midi.rs`: `MidiChannel(0..=15)`/`MidiGroup(0..=15)`/`NoteKey(0..=127)` validated newtype + `MidiRangeError`、`MidiKind`（`#[non_exhaustive]`、NoteOn/NoteOff/ControlChange/PitchBend/PolyPressure/ChannelPressure/ProgramChange）、`MidiEvent{group,channel,kind}`、`MidiValue` MSB-justified 変換 helper（`from_u7`/`from_u14`/`from_u16`/`as_u7`/`as_u14`/`as_u16` + bend `i32` 版）、`NoteLedger`（bounded `NOTE_LEDGER_CAPACITY`、`note_on`/`note_off`/`feed`/`is_active`/`active_count`/`release_all`/`take_overflow`、MIDI1 duplicate-pitch count 管理、force-release+overflow flag）。
- `timed_input.rs`: `ClockDomainId`、`InputStamp{ticks,base,domain,provenance}`、`TimedInputEvent<P>`、`ClockAnchor{domain,stamp_ticks,stamp_base,frame,calibration}`、`MapRejection`（`#[non_exhaustive]`、UnmappedDomain/EpochMismatch/BeforeAnchor）、`MappedInput<P>`、`map_stamp_to_frame`（anchor delta→`convert`→単回 NearestEven→anchor.frame 加算、provenance=min(stamp,anchor.calibration)）。
- `lib.rs` で `pub mod midi; pub mod timed_input;`。公開 API は doctest + doc。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-188-timed-midi.md`
- `docs/designs/issue-188-timed-midi-detailed.md`（§1, §2）

## Dependencies

- なし（media-core 内部の追加のみ。`EngineFrame`/`MediaTimeBase`/`TimestampProvenance`/`convert`/`NearestEven` は既存公開 API）。

## Affected Files

- `crates/ruviform-media-core/src/midi.rs`（新設）
- `crates/ruviform-media-core/src/timed_input.rs`（新設）
- `crates/ruviform-media-core/src/lib.rs`（公開）

## Done When

- [ ] `MidiChannel`/`MidiGroup`/`NoteKey` が範囲外を `MidiRangeError` で defined rejection する
- [ ] `MidiValue` の MSB-justified 変換が 7bit/14bit/16bit/32bit を無損失往復する
- [ ] `map_stamp_to_frame` が anchor/epoch/domain 規約通りに `UnmappedDomain`/`EpochMismatch`/`BeforeAnchor` を返し、単回 NearestEven で写像する
- [ ] `NoteLedger` が note_on/off/is_active/release_all/take_overflow を提供し、容量超過で force-release+overflow flag を立てる
- [ ] 全公開 enum が `#[non_exhaustive]`、公開 API に doctest
- [ ] `rt_` no-alloc テストが存在する

## Verification

- [ ] `mise run check` / `lint` / `test` / `test-rt` green（midi/timed_input 単体テスト含む）
