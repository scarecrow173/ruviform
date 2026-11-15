---
id: "IMPL-034Z9rDv8wCoN4sTuVgXzF"
type: "impl"
status: "completed"
title: "Issue #630 test_instrument slice — 実装記録"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-630-test-instrument.md"
    - "docs/plans/issue-630-test-instrument.md"
    - "docs/specs/issue-630-test-instrument.md"
    - "docs/designs/issue-630-test-instrument.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/instrument.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/events.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/instrument.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/instrument_core.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/component.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/controller.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/factory.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/block.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_real_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/data.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/tasks/issue-630-test-instrument.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-630-test-instrument.md"
    - "docs/plans/issue-630-test-instrument.md"
    - "docs/plans/issue-630-test-stateful.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-test-instrument.md"
    - "evidence/rf630/test-instrument-verification.md"
    - "docs/impl/exp/issue-630-test-instrument.jsonl"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-630-test-instrument.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-630-test-instrument.jsonl"
    rejected: []
---

# Issue #630 test_instrument slice — 実装記録

## Summary

両 fixture cdylib に実 instrument class `test_instrument` を追加。
VST3 は 10 class factory（instrument comp/ctrl が index 8/9、
`InputSurface::MidiTransport`）、CLAP は 4 plugin factory
（`org.ruviform-test.clap-test-instrument`）。production host
bridge 経由の in-process note lifecycle + L1 out-of-process
instrument E2E（4 case × 2 format）を実装・検証した。あわせて
CLAP host bridge の MIDI/transport forwarding と L1 wire の
fixed-record codec を実装した。

## Context

`docs/tasks/issue-630-test-instrument.md`（status: completed）→
`docs/plans/issue-630-test-instrument.md` → `docs/specs/
issue-630-test-instrument.md` + `docs/designs/
issue-630-test-instrument.md`。prior slice（gain/fault/editor/
stateful）は全て `Fx` processor で event/transport surface を
持たず、production host の MIDI/transport forwarding に独立
oracle がなかった。

## Implementation

### Changes

**VST3** (`crates/ruviform-plugin-test-vst3/`):

- `src/instrument.rs`（新規）: DSP law（`MAX_VOICES` voice、
  steal-oldest、2400-frame release、CC7 volume、CC1 tremolo、
  4 Hz LFO、transport click）+ `TestInstrumentComponent` +
  unit tests。
- `src/events.rs`（新規）: fixture-owned `IEventList`→
  `EventKind::Midi` translator（`NoteOn`/`NoteOff`/CC 系、
  `Match`/PCKN 相当の VST3 event kind 判定、out-of-block offset
  drop）と `ProcessContext`→`TransportInfo` translator
  （`projectTimeMusic` PPQ→PPQN-960 tick、tempo/timing flag
  検証、欠落 timing は捏造しない）。
- `src/component.rs`: `InputSurface` enum（`ParamsOnly` /
  `MidiTransport`）+ `with_surface` + `collect_block_events`
  （param + 条件付き MIDI → sort → `EventList`）+
  `block_transport`（surface 条件付き `ProcessContext`→
  `TransportInfo`）。`process` の event 収集部を helper に
  抽出して `too_many_lines` を解消。
- `src/controller.rs`: `INSTRUMENT_CONTROLLER_CID`。
- `src/factory.rs`: `countClasses` 8→10、index 8/9 = instrument
  pair。`getClassInfo`/`getClassInfo2`/`getClassInfoUnicode` の
  3 variant 重複を `CLASS_ROWS` テーブル + `class_row` helper
  に集約（10 class で `too_many_lines` 超過のため）。
  `"Instrument|Fx"` subCategories を出力。
- `tests/host_path.rs`: 5 plugin count、instrument discovery
  （features `["Instrument", "Fx"]`）、`prepared_instrument_
  plugin` + in-process note lifecycle test（retrigger `-1.0` →
  single NoteOff → release silence）、stateless-decline に
  instrument 追加。
- `tests/l1_real_test_plugin_e2e.rs`: `instrument_descriptor`
  pin（CID→hex 導出）+ `host_block_transport`/`midi_event`/
  `note_on`/`note_off`/`cc`/`playing_transport`/
  `instrument_host` helpers + 4 L1 cases。

**CLAP** (`crates/ruviform-plugin-test-clap/`):

- `src/instrument_core.rs`（新規）: DSP law の neutral-contract
  実装（`clack-*` 非依存）。CLAP transport snapshot→
  `TransportInfo` 投影（`CLAP_PPQN = 960` 固定）。
- `src/instrument.rs`（新規）: clack entry/descriptor
  （`org.ruviform-test.clap-test-instrument`、features
  `["instrument", "synthesizer", "stereo"]`）/main-thread/
  audio-processor + tests。
- `src/block.rs`: `translate_instrument_event`（structured note
  → `EventKind::Midi`、raw MIDI → `EventKind::Midi`、`Match::
  All`/out-of-range/非対応 kind は defined skip）+
  `transport_from_clap`。param translator 注入で既存 3 call
  site を更新。
- `src/{lib,plugin}.rs`: 4 つ目 plugin dispatch + descriptor。
- `tests/host_path.rs`/`tests/l1_real_test_plugin_e2e.rs`:
  VST3 と同型の mirror（descriptor pin、helpers、4 L1 cases、
  in-process lifecycle）。

**Production 側（forwarding/wire のみ）**:

- `crates/ruviform-plugin-clap/src/host.rs`: `push_midi_event`
  で neutral `EventKind::Midi` を CLAP ABI に encode —
  `NoteOn`/`NoteOff` は structured note event（PCKN channel+key
  concrete、port/note-id `Match::All`）、CC/pitch-bend/
  poly-pressure/channel-pressure/program-change は raw MIDI
  1.0 bytes。`TransportInfo`→CLAP transport snapshot 投影
  （beats/tempo/seconds timeline + loop range）。
- `crates/ruviform-plugin-core/src/wire.rs`:
  `encode_event_record`/`decode_event_record` — 33-byte
  `EVENT_WIRE_SIZE` fixed record codec（zero-pad + padding
  検証）。
- `crates/ruviform-plugin-host/src/l1/data.rs`: staging を
  record codec に変更。
- `crates/ruviform-plugin-export/src/lib.rs`: export decode を
  per-record codec に変更。

### Decisions During Implementation

- **fixture-owned translator**: VST3 `events.rs` と CLAP
  `block.rs` の event/transport 翻訳は production adapter と
  独立実装 — self-verification loop を断つため、変換 law は
  neutral contract（`MidiValue` scale、`TransportInfo` field）
  のみに依存。
- **`InputSurface` enum**: `TestComponent` が class 毎に
  `ParamsOnly`/`MidiTransport` を持ち、instrument のみ
  `inputEvents`/`processContext` を読む — stateless class の
  `process` path は不変。
- **`CLASS_ROWS` 集約**: factory の 3 `getClassInfo*` variant が
  10 class で `too_many_lines`（118/100）を超過 → 単一
  `ClassRow` テーブル + `class_row` helper に集約し情報源を
  一元化（振る舞い不変、clippy gate 通過）。
- **`collect_block_events`/`block_transport` 抽出**: `process`
  が MIDI/transport 追加で 111/100 に → event 収集と transport
  導出を helper に抽出（`process` を 99 行に、責務分離も改善）。
- **L1 wire record codec**: `EVENT_WIRE_SIZE = 33` 固定 slot に
  可変長 `encode_event`（MIDI = 28 byte）を詰めると desync —
  wire contract owner（`plugin-core`）に record codec を追加し
  host staging/export decode を移行。slot stride と event
  count semantics は不変。
- **L1 latency window**: delivered block は source call より
  1〜3 block 遅延（lookahead 2 + drain）→ phase window を全
  possible source call が同一 phase の範囲に限定。
- **signed zero**: CC7=0 の `-0.0` は `s == 0.0` IEEE 比較で
  判定、`-1.0`/`0.5` 等の有意値のみ `to_bits()` bit-exact。
- **single NoteOff**: fixture は NoteOff 毎に release 再武装
  → test は 1 回だけ NoteOff + empty blocks（release ~38 block
  後の tail で exact silence assert）。
- **offset sustain**: retrigger voice は後続 block に sustain →
  pre-offset silence を仮定せず `out[offset] == -1.0` と
  post-offset segment の bit-identity を assert。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-630-test-instrument.jsonl` — DSP law、
  translator 独立性、L1 latency window、wire codec、oracle
  設計の確定事項を記録。

### Rejected

- なし。

## Validation

実行済み検証（全て pass）:

- `cargo fmt --all -- --check` — pass
- `cargo clippy -p ruviform-plugin-core -p ruviform-plugin-host
  -p ruviform-plugin-export -p ruviform-plugin-clap
  -p ruviform-plugin-test-clap -p ruviform-plugin-test-vst3
  -p ruviform-plugin-vst3 --all-targets -- -D warnings` — pass
- `cargo test -p ruviform-plugin-test-vst3 --features l1-runner`
  — unit 84 pass / host_path 16 pass
- `cargo test -p ruviform-plugin-test-vst3 --features l1-runner
  --test l1_real_test_plugin_e2e -- --ignored` — 7/7 pass
  （instrument 4 含む）
- `cargo test -p ruviform-plugin-test-clap --features l1-runner
  --test l1_real_test_plugin_e2e -- --ignored` — 7/7 pass
  （instrument 4 含む）
- `cargo test -p ruviform-plugin-test-clap --test host_path` —
  11/11 pass
- touched production crates（`plugin-core`/`plugin-host`/
  `plugin-export`/`plugin-clap`/`plugin-vst3`）の `cargo test`
  — 全 pass

## Code review fixes

Adversarial review（fresh-context subagent）で 3 件の CONFIRMED
finding を修正:

- **CLAP raw MIDI byte validation**: `translate_raw_midi` が
  `d1`/`d2` を無検証で `MidiValue::from_u7` に渡し、`> 0x7F` の
  malformed byte が wrap した canonical 値を生成していた
  （VST3 sibling の `legacy_cc_bytes` は `<= 0x7F` を reject）。
  `d1` は全 channel-voice status で、`d2` は program change /
  channel pressure 以外で 7-bit を要求するよう修正 —
  defined skip に統一。
- **`Match::All` channel の捏造**: structured note event の
  `Match::All` channel が `unwrap_or(0)` で channel 0 に化けて
  いた（doc は defined skip を謳う）。key と同じく skip に修正し、
  `midi_envelope` を concrete `u8` 受け取りに単純化 —
  neutral contract が all-channel を表現できない以上、
  捏造しないのが正直な写像。
- **L1 transport oracle の stale-replay false-pass**: phase 3
  （looped）が phase 1 と同一の click@100 を期待しており、
  underrun 時の `pass_through` が `last_output` を replay する
  defined 挙動により、looped transport 未配送でも検証済みに
  なり得た。phase 3 を `position_ticks = 950`（click@250、
  他 phase と frame が異なる）に変更 — stale replay は
  assertion を満たせない。all-zero block のコメントも
  pass-through replay を含む実態に合わせて修正。
- 併せて `host.rs` の未使用 `sample_rate` field を除去、
  `l1-runner` feature 有効時の clippy（`comparison_chain`/
  `semicolon_if_nothing_returned`/`doc_markdown`）を両 e2e
  ファイルで解消。

## Risks

- **L1 latency**: lookahead 変更時は window 再検証が必要。
- **wire codec**: `decode_event_record` は nonzero padding を
  reject — fixture が padding を残す bug を検出できる。
- **VST3 `ProcessContext` timing flag**: musical timing 欠落時は
  `None`（捏造しない）— host が flag を立てない場合 click は
  鳴らない（defined）。
- **CLAP PPQN 固定**: `CLAP_PPQN = 960` は fixture law — host
  の beats 位置を tick に正規化する際の丸めは `round` で
  deterministic。

## Follow-ups

- `#628` MIDI `attach()` runtime-integration — coordinator 承認
  待ち（deferred）。
- CLAP `test_editor` class、外部 SDK validator、preset/patch —
  Deferred/Non-Goal 継続。
- `mise run test-l1-real-plugin` canonical lane の最終確認 —
  evidence に実施結果を記録。
