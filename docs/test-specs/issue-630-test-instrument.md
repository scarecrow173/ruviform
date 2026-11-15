---
id: "TSPEC-034Z9qCx7vBnM3rTsWfXyE"
type: "test-spec"
status: "approved"
title: "Issue #630 test_instrument slice — test specification"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-test-instrument.md"
    - "docs/designs/issue-630-test-instrument.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-test-instrument.md"
  implements:
    - "docs/specs/issue-630-test-instrument.md"
    - "docs/designs/issue-630-test-instrument.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "evidence/rf630/test-instrument-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-instrument.md"
    - "docs/designs/issue-630-test-instrument.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-630-test-instrument.md"
    - "docs/designs/issue-630-test-instrument.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_instrument slice — test specification

## Purpose

production host bridge の MIDI（`IEventList` / CLAP note+raw-MIDI）
と transport（`ProcessContext` / CLAP transport snapshot）forwarding
を、production adapter 内部を再利用しない独立 fixture plugin が
実 ABI で検証できるようにする。本 spec は `test_instrument` class
が満たすべき oracle として残し、後続 slice の fixture 拡張時にも
参照される。

## Feature

**Instrument test plugin event/transport oracle** — `test_instrument`
class の fixture plugin が、production host bridge 経由で MIDI event
と transport snapshot を受け取り、決定的 audio（voice・release・
CC・click）を生成し、L1 out-of-process runner でも同一 oracle が
成立する。

## Rules

- **R1 — Voice law**: `NoteOn`（velocity `u32::MAX`）は同 key を
  retrigger し、block の retrigger frame（offset 0 または event
  offset）で triangle `-1.0` extreme × velocity 1 × gain 1 ×
  volume 1 を出力。dual-mono（全 channel 同一値）。
- **R2 — Release law**: `NoteOff` は固定 2400-frame linear release
  を**開始**する（毎回再武装 — test は単一 NoteOff のみ送る）。
  release 完了後は exact silence（`s == 0.0`、signed zero 許容）。
- **R3 — CC law**: CC7 = master volume（`u32` scale-to-max、
  default `u32::MAX`）。CC7 = 0 は `-0.0` を出力（数値的 silent、
  `s == 0.0` で判定）。CC7 = `u32::MAX` は full level。
- **R4 — Offset law**: event `frame_offset` 前は voice 非発音、
  offset frame 自身が retrigger。voice は後続 block に sustain
  するため pre-offset silence は仮定しない。
- **R5 — Transport click law**: playing + musical timing
  （120 BPM / PPQN 960 / 48 kHz → 1 tick = 25 frame）で、block 内
  次 PPQN boundary に `0.5` impulse。position 956 → frame 100、
  940 → frame 500。click block は impulse frame 以外全 silent・
  stereo-identical。stopped は silent。loop range は per-block
  derivation を変えない。
- **R6 — Determinism**: 同一 event 列の delivered block は
  bit-identical（L1 lookahead は配送を遅らせるのみで内容を変え
  ない）。
- **R7 — L1 latency**: delivered block は source call より
  1〜3 block 遅延 — phase window は全 possible source call が
  同一 phase の範囲に限定し、遷移 call は未 assert。underrun
  （全 zero block）は sounding block（`out[0] == -1.0` を持つ）
  と区別して skip。
- **R8 — Wire codec**: `EventKind::Midi` は L1 shared-memory の
  33-byte fixed record で round-trip。`encode_event_record` は
  `EVENT_WIRE_SIZE` まで zero-pad、`decode_event_record` は
  malformed data と非 zero padding を reject。
- **R9 — Independent implementation**: fixture の event/transport
  translator は production adapter と独立 — 変換 law は neutral
  contract（`MidiValue`/`TransportInfo`）のみに依存し、adapter
  内部を import しない。
- **R10 — Fixture presence**: L1 test は cdylib/runner 欠落を
  assert-fail とし silent skip しない。

## Examples

- **E1 — VST3 note lifecycle (in-process)**: `Vst3Library` で
  `test_instrument` instance、note-on retrigger → `left[0] ==
  -1.0`、single note-off → 2400 frame 後 exact silence。
- **E2 — CLAP note lifecycle (in-process)**: 同上、`ClapLibrary`
  経由。
- **E3 — L1 VST3 instrument suite**: descriptor handshake →
  process → 4 case（lifecycle / release / CC+offset / transport
  click+seek+loop）→ teardown。全て latency-margin window で assert。
- **E4 — L1 CLAP instrument suite**: 同上、CLAP descriptor/id で実走。
- **E5 — L1 wire MIDI**: `EventKind::Midi`（NoteOn/NoteOff/CC）
  が 33-byte record で encode→decode round-trip、nonzero padding
  は reject。
- **E6 — Discovery**: VST3 factory `countClasses` = 10（5 plugin
  descriptor、instrument features `["Instrument", "Fx"]`）、CLAP
  `plugin_count` = 4（instrument features `instrument`/
  `synthesizer`/`stereo`）。

## Guarantees

- VST3 factory `countClasses` = 10、CLAP `plugin_count` = 4 を
  in-process host-path test が assert する。
- `test_instrument` descriptor（VST3: component CID-derived UID、
  name `test_instrument`、vendor `Ruviform Test`、version
  `0.0.0`、features `["Instrument", "Fx"]`；CLAP:
  `org.ruviform-test.clap-test-instrument`）が production
  discovery で独立に観測される。
- in-process host-path test が production `Vst3Library`/
  `ClapLibrary` ABI 経由の note lifecycle を pass する。
- L1 E2E test が `l1-runner` feature + `#[ignore]` で gate され、
  実走で pass する。missing artifact は assert-fail。
- `rt_` filter test が `process`/`clap_process` の
  allocation-free を継続証明する。

## Non-goals

- `#628` MIDI `attach()` runtime-integration（deferred）。
- pitch-bend/pressure/program-change/note-expression の DSP 反映
  （fixture law では defined no-op — event surface の存在証明が
  目的）。
- production adapter crate の DSP/translation 層の変更
  （CLAP host bridge の forwarding と wire codec のみ）。
- 外部 SDK validator、preset/patch、GUI、foreign plugin。

## Risk

- **L1 latency misattribution**: lookahead により phase 境界の
  delivered block が前 phase の source を反映し得る → R7 の
  window 規則で排除。これが drop されると flaky oracle になる。
- **signed zero 誤判定**: `-0.0` を bit-exact `+0.0` と比較すると
  false negative → R3/R7 で `s == 0.0` IEEE 比較を固定。
- **NoteOff 再送**: test が NoteOff を毎 block 送ると release が
  永久に再武装され silence に到達しない → R2 で単一 NoteOff を
  固定。
- **wire desync**: 33-byte slot に可変長 encoding を packed
  stream として読むと event boundary がずれる → R8 の record
  codec で固定 stride を保証。
