---
id: "TASK-034Z9pVw6xJmZ5sNtQeUgD"
type: "task"
status: "done"
title: "Issue #630 test_instrument slice — task"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-630-test-instrument.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-630-test-instrument.md"
  implements:
    - "docs/plans/issue-630-test-instrument.md"
  implemented-by:
    - "docs/impl/ir/issue-630-test-instrument.md"
  depends-on:
    - "docs/plans/issue-630-test-instrument.md"
    - "docs/tasks/issue-630-test-stateful.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-test-instrument.md"
    - "evidence/rf630/test-instrument-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-630-test-instrument.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-630-test-instrument.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_instrument slice — task

## Work

両 fixture cdylib に実 instrument class `test_instrument` を追加し、
NoteOn/NoteOff・velocity・CC・transport が実 ABI 経由で決定的
audio に到達することを in-process host-path test と L1
out-of-process runner test の両方で検証する。あわせて MIDI event
が L1 shared-memory wire を通過できるよう fixed-record codec を
wire contract owner（`ruviform-plugin-core`）に実装し、CLAP host
bridge に MIDI/transport forwarding を追加する。

### VST3

- `instrument.rs` 新規: DSP law（voice/release/CC/tremolo/click）
  + `TestInstrumentComponent`（`InputSurface::MidiTransport`）+
  unit tests。
- `events.rs` 新規: `IEventList`→`EventKind::Midi`、
  `ProcessContext`→`TransportInfo` の独立 translator。
- `component.rs`: `InputSurface` enum + `with_surface` +
  `collect_block_events`/`block_transport` helper（`process` の
  `too_many_lines` を整合的に解消）。
- `controller.rs`: `INSTRUMENT_CONTROLLER_CID`。
- `factory.rs`: `countClasses` 10、index 8/9 = instrument pair、
  `CLASS_ROWS` テーブル集約。
- `host_path.rs`: 5 plugin、instrument discovery、in-process
  note lifecycle test。
- `l1_real_test_plugin_e2e.rs`: descriptor pin + helpers +
  4 instrument L1 cases。

### CLAP

- `instrument_core.rs` 新規: DSP law の neutral-contract 実装。
- `instrument.rs` 新規: clack entry/descriptor/processor + tests。
- `block.rs`: `translate_instrument_event` +
  `transport_from_clap`。
- `plugin.rs`/`lib.rs`: 4 つ目 plugin dispatch。
- `host_path.rs`/`l1_real_test_plugin_e2e.rs`: 同上 mirror。

### 共通（production 側 forwarding/wire）

- `ruviform-plugin-clap/src/host.rs`: `EventKind::Midi`→CLAP
  note/raw-MIDI encode、`TransportInfo`→CLAP transport 投影。
- `ruviform-plugin-core/src/wire.rs`:
  `encode_event_record`/`decode_event_record`。
- `ruviform-plugin-host/src/l1/data.rs` +
  `ruviform-plugin-export/src/lib.rs`: record codec への移行。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-630-test-instrument.md`
- `docs/specs/issue-630-test-instrument.md`
- `docs/designs/issue-630-test-instrument.md`
- `docs/designs/issue-630-test-plugin-suite.md`
- `docs/designs/issue-630-l1-runner.md`
- `docs/test-specs/issue-630-test-instrument.md`

## Dependencies

- `issue-630-test-plugin-suite`（fixture base）
- `issue-630-l1-runner`（L1 out-of-process runner）
- `issue-630-test-stateful`（直前 slice の fixture/L1 pattern）

## Affected Files

- `crates/ruviform-plugin-test-vst3/src/{instrument,events,component,controller,factory,lib}.rs`
- `crates/ruviform-plugin-test-vst3/{Cargo.toml,tests/host_path.rs,tests/l1_real_test_plugin_e2e.rs}`
- `crates/ruviform-plugin-test-clap/src/{instrument,instrument_core,block,plugin,lib}.rs`
- `crates/ruviform-plugin-test-clap/{Cargo.toml,tests/host_path.rs,tests/l1_real_test_plugin_e2e.rs}`
- `crates/ruviform-plugin-clap/src/host.rs`
- `crates/ruviform-plugin-core/src/wire.rs`
- `crates/ruviform-plugin-host/src/l1/data.rs`
- `crates/ruviform-plugin-export/src/lib.rs`
- `Cargo.lock`
- `docs/specs|designs|plans|tasks|test-specs|impl/ir|impl/exp|evidence/rf630/`

## Done When

- VST3 factory が 10 class（5 plugin × comp/ctrl）を expose し、
  instrument pair が `InputSurface::MidiTransport` で MIDI +
  transport を消費する。
- CLAP custom factory が 4 plugin を expose し、`test_instrument`
  が note/raw-MIDI event + transport snapshot を消費する。
- production `Vst3Library`/`ClapLibrary` ABI 経由の in-process
  note lifecycle（retrigger `-1.0` → single NoteOff → release
  silence）が pass。
- L1 runner 経由の 4 instrument case（lifecycle / release /
  CC+offset / transport click+seek+loop）が実走・pass。
- `EventKind::Midi` が L1 shared-memory wire の 33-byte record
  codec で round-trip し、malformed record は defined rejection。
- `fmt-check`/`check`/`lint`/`test`/`check-l1`/`lint-l1`/
  `test-l1`/`test-rt`/`prek` が全て pass。
- doc chain（spec→design→plan→task→test-spec→impl/ir→impl/exp）
  と evidence が揃い、index が再生成されている。
