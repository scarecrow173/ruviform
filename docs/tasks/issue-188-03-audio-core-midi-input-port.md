---
id: "TASK-gOmhMICXSl8m4R5UD5kRf5"
type: "task"
status: "done"
title: "audio-core: midi input port trait + mock"
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
    - "docs/tasks/issue-188-01-media-core-midi-vocabulary.md"
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

# audio-core: midi input port trait + mock

## Work

`ruviform-audio-core` に `midi` モジュールを追加する。`device.rs` の `DeviceEnumerator`/`Device`/`OutputStream` パターンを入力方向へ反復する。

- `MidiPortKind{External,Virtual,Loopback,Unknown}`、`MidiSupport{Unknown,Unsupported,Supported}`、`MidiInputCaps{timestamps,sysex,ump,virtual_ports}`。
- `MidiInputEnumerator`（`input_ports`/`default_input_port`）→`MidiInputPort`（`id`/`name`/`kind`/`capabilities`/`open`）→`MidiInputConnection`（`close`）の 3 層 trait。`open` は callback `Box<dyn FnMut(&[TimedInputEvent<MidiEvent>]) + Send>` を受け取り、backend input thread 側で RT contract（no alloc/block/panic/re-entry）。
- `MidiInputError` — `DeviceError` 相当の normalized error。
- `midi::mock` — `MockMidiEnumerator`/`MockMidiInput`/`MockMidiConnection`。`push(TimedInputEvent<MidiEvent>)` で queue へ積み、open 済 connection の callback 経由で drain。port kind/caps は宣言可。公開方法（`pub mod mock` か feature 隔離）は実装時に確定（bridge テストから使える必要）。
- `lib.rs` で `pub mod midi;`。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-188-timed-midi.md`
- `docs/designs/issue-188-timed-midi-detailed.md`（§4）

## Dependencies

- `docs/tasks/issue-188-01-media-core-midi-vocabulary.md`（`MidiEvent`/`TimedInputEvent` 語彙）

## Affected Files

- `crates/ruviform-audio-core/src/midi.rs`（新設）
- `crates/ruviform-audio-core/src/lib.rs`（公開）

## Done When

- [ ] Enumerator→Port→Connection の 3 層 trait が `device.rs` パターン通り定義される
- [ ] `MidiPortKind`/`MidiSupport`/`MidiInputCaps`/`MidiInputError` が定義される
- [ ] `MockMidiEnumerator`/`MockMidiInput` が enumerate/open/close/push/drain を実装
- [ ] `MidiSupport::Unknown` が supported 扱いされない（capability honesty）
- [ ] 全公開 enum が `#[non_exhaustive]`、公開 API に doc

## Verification

- [ ] `mise run check` / `lint` / `test` green（mock enumerate/open/push 単体テスト含む）
