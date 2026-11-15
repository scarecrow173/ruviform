---
id: "DESIGN-034Z9mRw8yTgX3vLqPcSeB"
type: "design"
status: "approved"
title: "Issue #630 test_instrument slice — 設計"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-test-instrument.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-l1-runner.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-test-instrument.md"
  implements:
    - "docs/specs/issue-630-test-instrument.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-test-instrument.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-l1-runner.md"
  blocks:
    - "docs/plans/issue-630-test-instrument.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-instrument.md"
    - "docs/plans/issue-630-test-instrument.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-instrument.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-l1-runner.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_instrument slice — 設計

## Context

Issue #630 の既存 fixture（`test_gain`/`test_fault`/`test_editor`/
`test_stateful`）は全て `Fx` audio processor で、入力は parameter
event と PCM のみだった。production host bridge の MIDI
（VST3 `IEventList` / CLAP note+raw-MIDI event）と transport
（VST3 `ProcessContext` / CLAP `Process::transport`）forwarding
経路には独立 oracle がなく、host と fixture が同一 adapter 実装を
共有する self-verification loop の危険があった。本 slice は
両 fixture cdylib に実 instrument class を追加し、NoteOn/NoteOff・
velocity・CC・transport が実 ABI 経由で決定的 audio に到達することを
検証する。

## DSP law（両 format で同一の fixture-owned law）

`instrument_core.rs`（CLAP）/ `instrument.rs` 内 DSP 部（VST3）は
同一 law を独立に実装する（コード共有なし — binding 層と neutral
contract のみ共有）:

- **Voice**: 最大 `MAX_VOICES` の単音 triangle 発振器。
  周波数 `440 Hz * 2^((key-69)/12)`（等分音律、`f64` 係数表 —
  全 platform で同一 IEEE-754 結果）。位相は `u64` Q32.32
  accumulator を frame 毎に 1 回進め、波形は fraction 上位 24 bit
  を読み `[-1, 1]` triangle を生成（audio path に超越関数なし）。
- **NoteOn**: 同 key の voice を retrigger、なければ先頭 inactive
  voice、なければ最古（`max age`）を steal。`velocity == 0` の
  NoteOn は MIDI 1.0 running-status 慣習で note-off として扱う。
  channel/port/note-id は無視（channel-agnostic fixture）。
- **NoteOff**: 固定 `RELEASE_FRAMES = 2400` frame の linear
  release を開始、release が 0 で voice deactivate。
  **NoteOff は毎回 release を再武装する** — これは fixture law
  であり test は NoteOff を 1 回だけ送る。
- **CC**: CC7 = master volume（canonical `u32` scale-to-max、
  default `u32::MAX` = unity）、CC1 = tremolo depth（default 0）。
  他 controller / pitch-bend / pressure / program change は
  defined no-op。
- **Mix**: `out[c][n] = (Σ triangle(phase_v) * velocity_v *
  envelope_v) * gain * volume * tremolo + click[n]` — 4 Hz
  triangle LFO を CC1 depth で scale、全 output channel に同一
  値を書く（mono instrument → dual mono）。
- **Transport click**: transport が playing かつ musical timing
  （beats position + tempo）を持つ block で、block 内の次の
  PPQN boundary に `CLICK_AMP * volume`（= 0.5 × volume）の
  single-sample impulse を出力。tick→frame 変換は `i128` の
  exact ceiling division。各 block は自分の transport snapshot
  から click を導出するため、loop/seek/chase は配送された
  transport state のみで駆動される。stopped transport は silent。
- **Block offset**: event の `frame_offset` から適用 — offset
  前の frame は voice 非発音、offset frame 自身が retrigger
  frame（`-1.0` extreme）。

## VST3 側構造

- `instrument.rs`（新規）: 上記 DSP law の VST3 実装
  （`TestInstrumentComponent` — `TestComponent<P>` に
  `InputSurface::MidiTransport` を指定して構築）、unit tests。
- `events.rs`（新規）: fixture-owned の VST3 ABI→neutral 翻訳。
  `IEventList` の `NoteOn`/`NoteOff`/CC 系 event → neutral
  `EventKind::Midi`（`MidiValue` canonical scale law に整合）、
  `ProcessContext` → `TransportInfo`（`projectTimeMusic` PPQ を
  PPQN-960 tick に変換、tempo/timing flag を検証し欠落 timing を
  捏造しない）。production `ruviform-plugin-vst3` adapter の
  `midi.rs`/`component.rs` とは独立実装。
- `component.rs`: `InputSurface` enum（`ParamsOnly` /
  `MidiTransport`）追加、`TestComponent::with_surface`、
  `collect_block_events`（param events + 条件付き MIDI events →
  sort → `EventList`）、`block_transport`（surface 条件付き
  `ProcessContext` → `TransportInfo`）。stateless class は
  `ParamsOnly` のまま — `inputEvents`/`processContext` を読まない。
- `controller.rs`: `INSTRUMENT_CONTROLLER_CID` 追加
  （`ParamController::new(&GAIN_PARAM, 0)` — instrument の
  parameter surface は gain mirror）。
- `factory.rs`: `countClasses` 8→10、index 8/9 = instrument
  component/controller。`"Instrument|Fx"` subCategories を
  `PClassInfo2`/`PClassInfoW` に出力。3 つの `getClassInfo*`
  variant は `CLASS_ROWS` テーブル + `class_row` helper に
  集約（10 class で `too_many_lines` を超えたための整合的
  refactor — 情報源を 1 箇所に）。

## CLAP 側構造

- `instrument_core.rs`（新規）: 上記 DSP law の neutral-contract
  実装（`clack-*` 非依存 — `EventList`/`TransportInfo`/buffer
  view のみ使用）。CLAP `Process::transport` snapshot →
  `TransportInfo` の投影（`CLAP_PPQN = 960` 固定、
  `song_pos_beats`/`tempo`/loop/tempo-map flag を検証）。
- `instrument.rs`（新規）: `clack_plugin`/`clack_plugin_main`
  entry、descriptor（id
  `org.ruviform-test.clap-test-instrument`、name
  `test_instrument`、vendor `Ruviform Test`、version `0.0.0`、
  features `["instrument", "synthesizer", "stereo"]`）、
  main-thread/audio-processor 分割、unit tests。
- `block.rs`: `translate_instrument_event`（structured note
  event → `EventKind::Midi`、raw MIDI event → `EventKind::Midi`、
  `Match::All`/out-of-range PCKN・非有限 velocity・未対応 kind は
  defined skip）と `transport_from_clap`（CLAP transport →
  neutral `TransportInfo`）を追加。param translator は既存の
  まま。
- `plugin.rs`/`lib.rs`: 4 つ目の plugin dispatch、factory
  descriptor 追加。

## Production host bridge 側の変更（forwarding のみ）

- `ruviform-plugin-clap/src/host.rs`: neutral `EventKind::Midi`
  を CLAP ABI に encode — `NoteOn`/`NoteOff` は structured
  note event（PCKN: channel+key concrete、port/note-id `Match::All`）、
  CC/pitch-bend/poly-pressure/channel-pressure/program-change は
  raw MIDI 1.0 status/data bytes。`TransportInfo` を CLAP
  transport snapshot に投影（beats timeline + tempo + seconds
  timeline + loop range）。**変換 law は neutral
  `MidiValue`/`TransportInfo` contract が正本** — host bridge は
  encoder に徹し、fixture 側 decoder が独立に検証する。
- `ruviform-plugin-vst3` adapter: 変更なし — 既存の
  `IEventList`/`ProcessContext` forwarding がそのまま使われる
  （fixture 側 `events.rs` が独立 decoder として機能）。

## L1 wire codec（shared, format-neutral）

L1 shared-memory の event slot は `EVENT_WIRE_SIZE = 33` byte の
固定 stride だが、`encode_event` は可変長（MIDI event = 28 byte）
を出力していた — parent が 33 byte stride で stage し child が
packed stream として decode するため、MIDI event は wire 上で
表現不能だった（debug_assert で検出）。wire contract の owner で
ある `ruviform-plugin-core/src/wire.rs` に record codec を追加:

- `encode_event_record`: 通常 encoding の後に `EVENT_WIRE_SIZE`
  まで zero-pad。既存 output prefix を保持（開始長から resize）。
- `decode_event_record`: ちょうど 33 byte の 1 record を decode、
  malformed data と非 zero padding を reject。
- `ruviform-plugin-host/src/l1/data.rs`: staging を record codec
  に変更。`ruviform-plugin-export/src/l1`（`lib.rs`）: 各 33 byte
  record を独立に decode。slot layout と event count semantics は
  不変。

## RT boundary

- event/transport 翻訳は audio path 上で bounded scratch のみ
  （`EventScratch` ≤ `MAX_BLOCK_EVENTS`、click scratch は block
  毎に bounded）— allocation/lock/wait なし。
- VST3 `collect_block_events`/`block_transport` と CLAP
  `translate_*`/`transport_from_clap` は全て `&self`/`&mut`
  non-blocking の pure translation。
- fixture の `process`/`clap_process` は `rt_` filter test で
  allocation-free を継続証明。

## L1 E2E oracle 設計上の確定事項

- **lookahead latency**: L1 pipeline は 2 block lookahead +
  bounded drain を持ち、delivered block は source call より
  1〜3 block 遅れて到着する。phase 遷移をまたぐ call は assert
  せず、各 phase の「全 possible source call が同一 phase」に
  なる window のみ assert する。
- **underrun 判定**: pipeline は underrun 時に defined silence
  （全 zero）を書く — untouched buffer ではない。sounding block
  は常に `out[0] == -1.0`（retrigger frame）を持つため、全 zero
  block = underrun または正当な silence として skip する。
- **signed zero**: CC7 = 0 は voice を `-1.0 × 0.0 = -0.0` に
  する — 数値的に silent だが bit pattern は `+0.0` と異なる。
  silence 判定は `s == 0.0`（IEEE 比較）を使い、`-1.0`/`0.5`
  等の有意値のみ `to_bits()` で bit-exact assert する。
- **single NoteOff law**: fixture は NoteOff 毎に 2400-frame
  release を再武装する — test は NoteOn を数 block 送り、
  **1 回だけ** NoteOff、その後は empty event list。release
  完了（note-off block から ~38 block）を十分に超える tail で
  exact silence を assert。
- **offset gating**: note-on を frame offset 100 に置くと、
  retrigger frame 自身（`out[100] == -1.0`）が鳴り、post-offset
  segment は delivered block 間で bit-identical（voice は stateful
  で次 block の frame 0..100 に sustain するため pre-offset
  silence は assert しない）。
- **transport click**: 120 BPM / PPQN 960 / 48 kHz では 1 tick =
  25 frame。position_ticks = 956 → 次 boundary まで 4 tick →
  frame 100、940 → 20 tick → frame 500。click block は impulse
  frame 以外全 silent かつ stereo-identical。stopped → click@100
  → seek click@500 → looped click@100 の 4 phase × 8 call。

## Risks and Trade-offs

- **L1 event wire の可変長/固定長 mismatch**: 33 byte 固定 slot
  に 28 byte MIDI encoding を詰めると desync — record codec で
  zero-pad し decode 側が padding を検証する設計にした。
  これにより slot stride は不変のまま全 event kind が表現可能。
- **fixture/adapter self-verification**: fixture の event/transport
  translator は production adapter と独立実装 — 同じ bug を両側に
  埋め込む危険を排除するため、変換 law は neutral contract
  （`MidiValue` scale、`TransportInfo` field）のみに依存する。
- **VST3 `ProcessContext` timing flag**: musical timing が欠落
  した context で click を捏造しない — `projectTimeMusic`/
  `tempo`/`timeSigNumerator` 等の flag を検証し、欠落時は
  musical field を `None` にする（stopped 相当の silent）。
- **CLAP PPQN 非存在**: CLAP transport は beats 秒しか運ばない
  ため `CLAP_PPQN = 960` を fixture が固定 — VST3 側は host の
  `projectTimeMusic.ppq` を PPQN-960 に正規化するため、両 format
  で同一 tick math が成立する。

## Deferred Design Concerns

- `#628` MIDI `attach()` runtime-integration — coordinator 承認
  なしに実装しない（deferred）。
- CLAP `test_editor` class、外部 SDK validator、preset/bank、
  foreign plugin state — Deferred/Non-Goal 継続。
- 複数 voice の polyphony stress、pitch-bend/pressure/program
  change の DSP 反映 — fixture law では defined no-op（event
  surface の存在証明が目的であり全 MIDI kind の DSP 網羅では
  ない）。

## Verification Notes

- [x] `cargo fmt --all -- --check` — pass
- [x] `cargo clippy -p <touched crates> --all-targets -- -D warnings` — pass
- [x] `cargo test -p ruviform-plugin-test-vst3 --features l1-runner` — unit 84 + host_path 16 pass
- [x] `cargo test -p ruviform-plugin-test-vst3 --features l1-runner --test l1_real_test_plugin_e2e -- --ignored` — 7/7 pass（instrument 4 含む）
- [x] `cargo test -p ruviform-plugin-test-clap --features l1-runner --test l1_real_test_plugin_e2e -- --ignored` — 7/7 pass（instrument 4 含む）
- [x] `cargo test -p ruviform-plugin-test-clap --test host_path` — 11/11 pass
- [x] touched crates (`plugin-core`/`plugin-host`/`plugin-export`/`plugin-clap`/`plugin-vst3`/両 fixture) の `cargo test` — 全 pass
- [ ] `mise run test-l1-real-plugin`（canonical lane — 実施済み個別 test と同等、最終確認は evidence 参照）
- [ ] `mise run test-rt` / `prek run --all-files` — 未実施（evidence に実施結果を記録）
