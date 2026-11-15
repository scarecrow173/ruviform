# RF-VTEST-04 test_instrument slice — 検証記録

- Issue: `scarecrow173/ruviform#630`（test plugin suite の instrument slice）
- Slice: `test_instrument` plugin（VST3 + CLAP）— NoteOn/NoteOff・velocity・CC・transport click の deterministic oracle + in-process host-path + L1 out-of-process E2E
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints 正本）
- 記録日: 2026-11-16
- Case Schema v1 語彙・`environment_kind=test-plugin`。deterministic evidence と real-run evidence を分離。

## 実装概要

- `crates/ruviform-plugin-test-vst3/src/instrument.rs`（新規）: `TestInstrumentComponent`（`InputSurface::MidiTransport`）+ DSP law（`MAX_VOICES` triangle voice、steal-oldest、2400-frame release、CC7 volume、CC1 tremolo、4 Hz LFO、transport click）+ unit tests。
- `crates/ruviform-plugin-test-vst3/src/events.rs`（新規）: fixture-owned `IEventList`→`EventKind::Midi` translator と `ProcessContext`→`TransportInfo` translator（`projectTimeMusic` PPQ→PPQN-960 tick、tempo/timing flag 検証、欠落 timing は捏造しない）。production adapter とは独立実装。
- `crates/ruviform-plugin-test-vst3/src/component.rs`: `InputSurface` enum + `with_surface` + `collect_block_events`/`block_transport` helper（`process` の event 収集部を抽出し `too_many_lines` 解消）。stateless class は `ParamsOnly` のまま。
- `crates/ruviform-plugin-test-vst3/src/controller.rs`: `INSTRUMENT_CONTROLLER_CID`（gain-mirror `ParamController`）。
- `crates/ruviform-plugin-test-vst3/src/factory.rs`: `countClasses` 8→10、index 8/9 = instrument pair、`"Instrument|Fx"` subCategories、3 `getClassInfo*` variant を `CLASS_ROWS` テーブル + `class_row` helper に集約。
- `crates/ruviform-plugin-test-vst3/{Cargo.toml,tests/host_path.rs,tests/l1_real_test_plugin_e2e.rs}`: `ruviform-media-core` 依存、5 plugin discovery、in-process note lifecycle、L1 descriptor pin + helpers + 4 instrument cases。
- `crates/ruviform-plugin-test-clap/src/instrument_core.rs`（新規）: DSP law の neutral-contract 実装（`clack-*` 非依存）。`CLAP_PPQN = 960` 固定の transport 投影。
- `crates/ruviform-plugin-test-clap/src/instrument.rs`（新規）: clack entry/descriptor `org.ruviform-test.clap-test-instrument`（features `instrument`/`synthesizer`/`stereo`）/main-thread/processor + tests。
- `crates/ruviform-plugin-test-clap/src/block.rs`: `translate_instrument_event`（structured note + raw MIDI → `EventKind::Midi`、`Match::All`/out-of-range/非対応 kind は defined skip）+ `transport_from_clap`。
- `crates/ruviform-plugin-test-clap/src/{lib,plugin}.rs`: 4 つ目 plugin dispatch。
- `crates/ruviform-plugin-test-clap/{Cargo.toml,tests/host_path.rs,tests/l1_real_test_plugin_e2e.rs}`: VST3 mirror。
- `crates/ruviform-plugin-clap/src/host.rs`（production bridge、forwarding のみ）: `push_midi_event` で `EventKind::Midi`→CLAP note event（NoteOn/NoteOff、PCKN concrete、port/note-id `Match::All`）または raw MIDI 1.0 bytes（CC/pitch-bend/poly-pressure/channel-pressure/program-change）。`TransportInfo`→CLAP transport snapshot 投影。
- `crates/ruviform-plugin-core/src/wire.rs`: `encode_event_record`/`decode_event_record` — 33-byte `EVENT_WIRE_SIZE` fixed record codec（zero-pad、nonzero padding/malformed reject）。
- `crates/ruviform-plugin-host/src/l1/data.rs` + `crates/ruviform-plugin-export/src/lib.rs`: staging/decode を record codec に移行。
- `Cargo.lock`: `ruviform-media-core` 依存反映。

## DSP/Transport law（確定）

- Voice: 最大 `MAX_VOICES`、triangle `440 Hz * 2^((key-69)/12)`（f64 semitone 表）、`u64` Q32.32 phase を frame 毎 1 回進行、fraction 上位 24 bit で `[-1,1]` waveform。
- NoteOn: 同 key retrigger → 先頭 inactive → 最古 steal。velocity `== 0` は note-off 相当。channel/port/note-id 無視。
- NoteOff: 固定 `RELEASE_FRAMES = 2400` linear release を**毎回再武装** — test は単一 NoteOff のみ送る。
- CC: CC7 = master volume（canonical `u32` scale-to-max、default `u32::MAX`）、CC1 = tremolo depth（4 Hz triangle LFO）。他 event は defined no-op。
- Mix: `out[c][n] = (Σ triangle*velocity*envelope) * gain*volume*tremolo + click[n]` — 全 channel 同一値。
- Click: playing + musical timing で block 内次 PPQN boundary に `0.5*volume` impulse。tick→frame は `i128` exact ceiling division。120 BPM/PPQN 960/48 kHz = 25 frame/tick → position 956 → frame 100、940 → frame 500。stopped = silent。per-block derivation のため loop/seek/chase は配送 transport state のみで駆動。

## L1 wire codec（確定）

- L1 shared-memory event slot は `EVENT_WIRE_SIZE = 33` byte 固定 stride。従来 `encode_event` は可変長（MIDI = 28 byte）を出力し、parent が 33-byte stride で stage・child が packed stream として decode するため MIDI event は wire 上で表現不能だった（debug_assert で検出）。
- `plugin-core/src/wire.rs` に `encode_event_record`/`decode_event_record` を追加: 通常 encoding の後 `EVENT_WIRE_SIZE` まで zero-pad、decode はちょうど 33 byte の 1 record を検証（malformed data と非 zero padding を reject）。
- `plugin-host/src/l1/data.rs` の staging と `plugin-export/src/lib.rs` の decode を record codec に移行。slot stride と event count semantics は不変。

## L1 oracle 設計（確定）

- **lookahead latency**: delivered block は source call より 1〜3 block 遅延 — phase 遷移をまたぐ call は assert せず、全 possible source call が同一 phase の window のみ assert。
- **underrun**: pipeline は underrun 時に defined silence（全 zero）を書く — sounding block は常に `out[0] == -1.0`（retrigger extreme）を持つため区別可能。
- **signed zero**: CC7=0 は `-0.0` を出力 — silence 判定は `s == 0.0` IEEE 比較、`-1.0`/`0.5` 等は `to_bits()` bit-exact。
- **single NoteOff**: release は NoteOff 毎に再武装されるため test は 1 回だけ送信 + empty blocks。release 完了（~38 block）を超える tail で exact silence assert。
- **offset gating**: note-on を frame offset 100 に置き `out[100] == -1.0` + post-offset segment の bit-identity を assert（voice は次 block に sustain するため pre-offset silence は仮定しない）。
- **transport click**: 4 phase（stopped → playing@100 → seek@500 → looped@100）× 8 call、click block は impulse frame 以外全 silent・stereo-identical。

## 実行した検証（canonical = mise、実 exit code）

| command | result | exit |
| --- | --- | --- |
| `mise run fmt-check` | pass | 0 |
| `cargo check -p <touched crates> --features l1-runner` | pass | 0 |
| `cargo clippy -p <touched crates> --all-targets -- -D warnings`（feature-enabled 含む） | pass | 0 |
| `cargo test -p ruviform-plugin-test-vst3 --features l1-runner` | unit 84 + host_path 16 pass | 0 |
| `cargo test -p ruviform-plugin-test-vst3 --features l1-runner --test l1_real_test_plugin_e2e -- --ignored` | 7/7 pass（instrument 4 含む: `real_test_plugin_l1_instrument_note_lifecycle`/`_note_off_releases_to_silence`/`_cc_and_offset_gating`/`_transport_clicks`） | 0 |
| `cargo test -p ruviform-plugin-test-clap --features l1-runner` | unit 49 + host_path 11 pass | 0 |
| `cargo test -p ruviform-plugin-test-clap --features l1-runner --test l1_real_test_plugin_e2e -- --ignored` | 7/7 pass（instrument 4 含む、同名 case） | 0 |
| `cargo test -p ruviform-plugin-{core,host,export,clap,vst3}` | 全 pass | 0 |
| `mise run test-l1-real-plugin` | canonical lane — adapter e2e + test-plugin e2e（gain/stateful/instrument 含む）+ fault e2e 全 pass | 0 |
| `mise run test-rt` | `rt_` filter 全 pass（process/clap_process alloc-free 継続証明） | 0 |
| `mise exec -- prek run --all-files` | pass | 0 |

`*-msrv`/`*-full` lane は scope 外（prior slice と同様）。external SDK validator・非 Windows OS は out of scope。

## 診断記録（実バグ vs test oracle）

- **CLAP release-tail 非 silence**: 初回 L1 で tail に実 audio（`-0.73`）— bridge の NoteOff 喪失を疑ったが、fixture-level translation test・host `push_events` test・block-level test で全 link が正しいことを確認。根本原因は test が毎 block NoteOff を再送し fixture law（NoteOff 毎に 2400-frame release 再武装）により release が永久に完了しないこと。test を single NoteOff + empty blocks に修正 → pass。
- **`-0.0` false negative**: CC7=0 の silence が bit pattern `-0.0` で `+0.0` と不一致 — `s == 0.0` IEEE 比較に変更。
- **offset pre-silence 仮定**: retrigger voice は後続 block の frame 0..offset に sustain するため pre-offset silence assert は不成立 — `out[offset] == -1.0` + post-offset bit-identity に変更。
- **L1 event wire desync**: MIDI event（28 byte）が 33-byte slot に packed stream として書かれ child decode が desync — wire contract owner で record codec を実装し staging/decode を移行。

## Deferred / Out of scope

- `scarecrow173/ruviform#628` MIDI `attach()` runtime-integration — coordinator 承認なしに実装しない。
- CLAP `test_editor` class、外部 SDK validator、preset/patch、foreign plugin state。
- pitch-bend/pressure/program-change の DSP 反映（fixture law では defined no-op）。
- Issue #630 自体は open 継続（後続 slice 用）。
