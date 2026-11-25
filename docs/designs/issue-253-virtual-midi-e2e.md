---
id: "DESIGN-034Yo3tqQqDfgciLUTv3k8"
type: "design"
status: "approved"
title: "Issue #253 virtual-first OS-endpoint MIDI e2e verifier design"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "docs/specs/issue-253-virtual-midi-e2e.md"
    - "docs/specs/issue-629-midi-loopback.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-253-virtual-midi-e2e.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-253-virtual-midi-e2e.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-253-virtual-midi-e2e.md"
    - "docs/specs/issue-629-midi-loopback.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-253-virtual-midi-e2e.md"
    - "docs/test-specs/issue-253-virtual-midi-e2e.md"
    - "docs/tasks/issue-253-virtual-midi-e2e.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-253-virtual-midi-e2e.md"
    - "docs/designs/issue-629-midi-loopback.md"
  derived-by:
    - "docs/plans/issue-253-virtual-midi-e2e.md"
    - "docs/test-specs/issue-253-virtual-midi-e2e.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #253 virtual-first OS-endpoint MIDI e2e verifier design

## Context

Issue #253 の AC1/AC6 が要求する合成経路 — 実 OS MIDI backend から
attach・plugin・typed runtime を経て out-of-process VST3 fixture の
PCM 出力まで — を 1 本の `#[ignore]`-gated verifier で検証する。
構成部品は全て merge 済みであり、本設計はそれらを繋ぐ test のみを
対象とする（production コード変更なし）。

利用する既存契約（調査済み）:

- `ruviform_midi_device::backend()` → `SystemBackend` —
  `MidiInputEnumerator::input_ports` /
  `MidiOutputEnumerator::output_ports` の両方向 port を `Box<dyn …>`
  で返す。Windows では WinMM backend、非 Windows では
  `DeviceError::Unsupported`。
- input port: `info()`（`id`/`name`/`origin`/`capabilities` —
  `provides_timestamps`・per-open `clock_domain`）、`open()` →
  `MidiInput::read(&mut Vec<TimedMidiInput>, max) ->
  MidiReadStats`、`close()`。event は `TimedInputEvent` 相当の
  `TimedMidiInput`（`stamp.ticks` = `midiInStart` からの ms、
  `stamp.base` = `TimeBase::new(1, 1_000)`、`stamp.domain` = port
  宣言 domain、`provenance = Carried`）。
- output port: `info()`、`open()` → `MidiOutput::send` /
  `close`。port id は方向で体系が異なるため pair 照合は
  `info().name` の双方向一致（`loopback.rs` と同じ signature）。
- `InputTx::pump(&mut self, input: &mut dyn MidiInput) ->
  DeviceResult<PumpStats>`（`attach.rs` の input leg が生成）—
  `PumpStats` は `received`/`pushed`/`deferred`/
  `backend_dropped`/`backend_unsupported` を返し、累積 accessor
  `dropped_backend()`/`unsupported_backend()` を持つ。
- `attach(&mut PluginHost, PluginHandle, &AttachConfig) ->
  Attached` — `AttachConfig::input: Option<InputConfig>` で
  `input_tx` leg を有効化。`Attached` は `input_tx`/`control`/
  `node`/`ports` を返す。
- `PluginNodeControl::set_clock_anchor(Option<ClockAnchor>)` /
  `snapshot()` — `InputStageCounters`（`mapped`/`delivered`/`late`/
  `rejected`/`anchor_rejected`/`dropped_backend`/
  `unsupported_backend`/`dropped_capacity`）を観測する面。
- typed graph: `GraphBuilder` + `NodeDef`/`NodeKind`/`PortDef` +
  `ConnDef` + `Validator` → `Compiler::compile` →
  `GraphRuntime::publish` → `RtRuntime::process`。先例は
  `l1_real_test_plugin_e2e.rs::attach_integration`（48 kHz・
  stereo・512 frame block・Source→Processor→Sink）。
- fixture: 本 crate cdylib
  （`ruviform_plugin_test_vst3`）を sibling runner
  （`ruviform-plugin-test-vst3-l1-runner`、`l1-runner` feature で
  build）経由で `PluginHost::register_out_of_process` +
  `OutOfProcessSpec` が dlopen する。descriptor は
  `instrument_descriptor()` が fixture 既知値を pin する。
- `test_instrument` の音声 oracle: full-velocity NoteOn →
  triangle attack（frame 0 = −1.0 × velocity 正規化）・dual-mono・
  release 長 2_400 frame・決定的出力（scripted 先例と同じ）。

重要な設計上の発見・制約:

- **WinMM は仮想/物理を区別しない**（`MidiPortOrigin::Unknown`）。
  name match は loopback-style candidate であり virtual endpoint の
  証明ではない — verdict は `environment_kind=
  virtual-os-endpoint` を主張しない（fail-closed 境界）。
- **stamp は driver-domain の ms tick**。`×48`（48 000/1 000）の
  frame 換算が成立するのは `stamp.base == TimeBase::new(1,
  1_000)` が観測された場合のみ — foreign base は envelope 外の
  正直な `NotRun`、domain/provenance/単調性違反は `Fail`。
- **anchor は観測値から較正する**。`NoteOn` の実 stamp tick を
  `ClockAnchor{frame: EngineFrame(0, ANCHOR_FRAME),
  calibration: Estimated}` として publish し、PCM oracle が
  intra-block offset の着地を検証する。scripted 先例の
  「anchor を先に決めて event をその位置に置く」とは逆方向 —
  こちらは OS が stamp した時刻を engine frame に写す。
- **anchor 未 publish の pending event は待機する**（V9/V11/V12
  系の `anchor_none_pending_waits` 契約）— pump を graph process
  より先に済ませてよい。anchor より前の stamp を持つ event は
  `BeforeAnchor` として `rejected` に計数される（counter oracle
  の独立期待値になる）。
- **L1 lookahead = 2 block**。`OutOfProcessSpec` default
  `lookahead_blocks=2` × `max_frames=512` により engine block `b`
  の出力は drained slot `b+2` に現れる — PCM oracle は
  `first_sound == DELIVERED_LOOKAHEAD` と release tail の engine
  frame 写像で検証する。
- **非 Windows は `DeviceError::Unsupported`** — `#[cfg]` 分岐を
  置かず runtime 判定で `NotRun`（3OS lane green）。Win32 依存は
  一切追加しない。
- **lane 収集**: 新 file は `test-l1-real-plugin`（`--test` 個名
  指定）にも `test-midi-real-device`（対象 package 外）にも自動
  収集されない。`mise.toml` は別 Worker 占有のため lane 登録は
  本 slice 対象外 — compile/lint は `check-l1-real-plugin`/
  `lint-l1-real-plugin` の `--all-targets` で確認し、実走は
  補助証拠として個別実行する。

## Scope

### In Scope

- `crates/ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs`
  の新設 — fixture plumbing・attach rig・pair 選択・stamp 契約・
  anchor 較正・PCM/counter/close oracle・`#[test]` + `#[ignore]`。
- `crates/ruviform-plugin-test-vst3/Cargo.toml` の dev-dep 追加
  （`ruviform-midi-device` path dep のみ — `ruviform-audio-device`
  は backend 面を直接使わないため不要）。
- `Cargo.lock` の再生成差分（当該 dep 1 行）。
- docs chain と `evidence/rf253/` の検証記録。

### Out of Scope

- production コード全般（`crates/*/src/` は zero-diff）。
- `tests/l1_real_test_plugin_e2e.rs`・`tests/midi_wiring.rs` の
  変更（共有 helper の拡張が不要だった）。
- loopMIDI/WMS provisioning・`virtual-os-endpoint` evidence・
  reconnect/loss/xrun・Test Runtime 結線・consumer 照合・
  macOS/Linux backend・lane 登録（spec の Out of Scope と同一）。

## Components and Boundaries

- **`os_endpoint_midi_e2e.rs`（test file、唯一の成果物）**:
  - fixture plumbing: `fixture_lib_name`/`instrument_cid`/
    `instrument_descriptor`/`runner_program`/`fixture_path`/
    `assert_fixture_present`/`config`/`instantiate`/
    `drive_to_processing`/`instrument_host` — sibling file の
    instrument-only mirror。
  - attach rig: `AttachRig`（`PluginHost`/`GraphRuntime`/`RtRuntime`/
    `PluginNodeControl`/`InputTx`/port id）+ `build_rig` —
    `attach_integration` と同型の Source→Processor→Sink graph。
  - graph pumping: `feed_source`/`drain_sink`/`is_silent`/
    `pump_block`/`process_blocks` — scripted 先例と同じ 25/5 ms
    pacing。
  - OS endpoint: `expected_channel_voice_sequence`（独立 literal
    oracle）/`loopback_pair_candidates`/`select_loopback_pair`/
    `OpenedPair`/`open_loopback_pair`。
  - stamp/anchor: `stamp_contract_holds`/`match_expected`/
    `ObservingInput`（passive tap）/`PumpOutcome`/
    `pump_until_sequence`。
  - oracle: `assert_pcm_oracle`/`assert_counter_oracle`/
    `assert_close_semantics`。
  - entry: `os_endpoint_midi_loopback_drives_test_instrument`
    （`#[test]` + `#[ignore]`）。

## Data and Control Flow

1. `assert_fixture_present` → cdylib + runner binary の存在を
   assert（欠落 = Fail、skip ではない）。
2. `open_loopback_pair`: `backend()` で input/output を列挙 →
   `info().name` 双方向一致で pair 化 → timestamp capability
   gate → input→output の順で open（input 優先で送信前に購読を
   確立）。各 `NotRun` 条件は eprintln verdict + `None` return。
3. `build_rig`: fixture spawn → `Processing` まで駆動 →
   `attach(input leg)` → typed graph compile/publish。
4. stale queue drain（観測 tap 設置前に直接 read — ambient
   traffic は oracle 対象外）。
5. `ObservingInput` を被せて expected 4 event を output から送信
   （8 ms 間隔 — stamped span を 1〜2 engine block に収める）。
6. `pump_until_sequence`: `InputTx::pump` を観測列が expected の
   ordered subsequence になるまで回す（5 s deadline、超過 =
   Fail）。backend loss counter を累積観測。
7. `stamp_contract_holds`: 全観測 event の domain/`Carried`/
   非減少 ticks/`base=1/1000` を検査（foreign base = NotRun、
   他違反 = Fail）。
8. `set_clock_anchor`: 観測 `NoteOn` stamp → `ANCHOR_FRAME=100`
   （`Estimated`）。`f_off = 100 + Δstamp_ms × 48` で `NoteOff`
   の engine frame を算出し、必要 block 数を `MAX_BLOCKS=48`
   で fail-closed に制限。
9. `process_blocks`: `blocks_needed` 回 `rt.process` + drain。
10. oracle 3 連: `assert_pcm_oracle`（lookahead slot・
    intra-block attack・release tail・dual-mono）→
    `assert_counter_oracle`（stage snapshot + `InputTx` 透過 +
    `BeforeAnchor` rejected の独立期待値）→
    `assert_close_semantics`（両方向 `StreamInvalidated`）。
11. verdict 出力: pair 情報・counter 集計・「name match は
    virtual endpoint の証明ではない」旨の evidence note。

## Non-Functional Constraints

- `unsafe` なし・新規外部依存なし・新規 feature なし（dev-dep は
  workspace path dep のみ、Coordinator 承認済み）。
- `#[ignore]` + 手動 opt-in — canonical lane は compile/lint のみ
  収集し、実行は provisioning 済み環境の手動操作。
- 非 Windows で compile 通過 + runtime `NotRun`（`#[cfg]` で
  Win32 依存を隔離する必要がない — backend API が `Unsupported`
  を返す設計をそのまま利用）。
- 実行時間は bounded（pump deadline 5 s・`MAX_BLOCKS` 48・
  pacing は scripted 先例踏襲）。

## Risks and Trade-offs

- **name match の誤認**: 同名の無関係 port を pair と誤認し得る
  — 送信→受信が成立しなければ `Fail`（正直な失敗）であり
  false Pass ではない。複数 candidate は全列挙して先頭を使用。
- **stamp 粒度**: WinMM の ms stamp と 48 kHz frame の対応は
  `×48` の理論値 — 実測で intra-block offset が 1 frame ずれる
  可能性は attack 位置 assert が検出する（ずれは Fail であり
  観測値の記録になる）。
- **L1 pacing のフレーク**: child の処理追従は scripted 先例の
  25/5 ms cadence を踏襲 — lookahead slot より前の stale 結果は
  endpoint が discard するため oracle は slot index で固定される。
- **ambient traffic**: pair open 前の queue 滞留は tap 設置前に
  drain して除外 — ただし drain 後に到着する他 source の event
  は ordered-subsequence match で許容され、`rejected` 期待値は
  anchor より前の stamp のみを数える設計で吸収する。
- **`mapped == delivered` 仮定**: 全 event が処理済み block 範囲
  内に写像される前提 — stamped span が `MAX_BLOCKS` を超える
  場合は事前に `Fail` で打ち切る。

## Deferred Design Concerns

- seek/stop/queue overflow の扱い — #628 scripted 側が決定的
  event oracle を既担するため本 slice では重複実装しない。
- `virtual-os-endpoint` Pass の取得 — provisioning 確立後の
  後続 slice（本 verifier をそのまま実行する）。
- lane 登録（`mise.toml`）— 別 Worker 占有、Coordinator 統合時。

## Verification Notes

- [x] `cargo check -p ruviform-plugin-test-vst3 --features
  l1-runner --tests` — compile Pass（補助証拠）。
- [x] `cargo clippy -p ruviform-plugin-test-vst3 --features
  l1-runner --tests -- -D warnings` — lint Pass（補助証拠）。
- [x] `cargo check -p ruviform-plugin-test-vst3`（feature なし）
  — `#![cfg]` gate で file が空になり compile Pass（補助証拠）。
- [x] `mise run check-l1-real-plugin` / `lint-l1-real-plugin`
  （canonical — `--all-targets` + `l1-runner`）— exit 0。
- [x] 補助実走
  `cargo test -p ruviform-plugin-test-vst3 --features l1-runner
  --test os_endpoint_midi_e2e -- --ignored` — exit 0、
  `verdict: NotRun — NOT_PROVISIONED`（0 input / 1 output
  ports、assert 0 件）を `evidence/rf253/
  os-endpoint-midi-e2e-verification-windows-local.txt` に記録。

## 追記（2026-11-23）— canonical lane `test-os-endpoint-midi-e2e` 登録済み

Context 節の「lane 収集」（新 file が `test-l1-real-plugin` にも
`test-midi-real-device` にも自動収集されず、`mise.toml` は別
Worker 占有のため lane 登録は本 slice 対象外とする記述）と
Deferred Design Concerns の「lane 登録（`mise.toml`）— 別
Worker 占有、Coordinator 統合時」は、lane 統合 slice で解消
された。`mise run test-os-endpoint-midi-e2e` が本 verifier の
唯一の canonical 実行入口となった（`cargo build -p
ruviform-plugin-test-vst3 --features l1-runner` → `cargo test
-p ruviform-plugin-test-vst3 --features l1-runner --test
os_endpoint_midi_e2e -- --ignored --test-threads=1`）。
Non-Functional Constraints の「canonical lane は compile/lint
のみ収集し、実行は provisioning 済み環境の手動操作」という
当時の前提もこれに伴い更新される — lane は実行入口でもある
が、pair 未 provision では `verdict: NotRun — NOT_PROVISIONED`
で fail-closed に終わる（Pass 偽装なし）。`check-l1-real-plugin`/
`lint-l1-real-plugin` の `--all-targets` compile/lint 収集は
変わらず有効で、専用 lint lane は設けない。実行記録は
`evidence/rf253/`。

## 追記（2026-11-25）— RF#628 supply / Case Schema v1 / workflow stage

実装は no-command playthrough に限定して RF#628 `MidiScenario` →
`run_midi_trace` を stimulus supply に採用した。send list は hand-built literal
ではなく delivered `MidiTraceEntry` であり、送信前に monotonic invariant と
`midi_expected_playthrough`/`midi_trace_eq` の独立 oracle を通す。receive-side
oracle は独立 literal の ordered subsequence、PCM/counter oracle は実測 OS
timestamp に anchor したままである。つまり deterministic trace は「何を送るか」
の供給と名目 pacing を担い、「実 OS でいつ観測されたか」の証拠を捏造しない。

terminal verdict は Case Schema v1 `RunMetadata` として emit する。capability
gate（backend/列挙/pair/timestamp/clock domain/foreign stamp base）は `NotRun`、
pair open 以降の鎖・oracle・fixture artifact は `Fail`、全 oracle 通過のみ
`Pass` であり、panic path も catch して terminal record を best-effort で残す。
metadata evidence は name match が candidate にすぎないこと、acceptance-grade
解釈には workflow の RvfMidi provisioning provenance が必要なことを明示する。

`virtual-audio-driver-probe.yml` の `os-endpoint-midi-e2e` stage は、
fixture build と `--nocapture` 補助実行を行い、repo/SHA/level/environment/
result を検査して metadata JSON を artifact へ集める。`midi-endpoint-probe`
の `CANDIDATES_OBSERVED` と `midi-loopback` Pass を stage detail に併記するが、
これらが欠ける未 provisioning 環境での honest `NotRun` は absence-is-data
として error にしない。

stop/seek/overflow の責務 mapping は spec の 2026-11-25 追記を正本とする。
本 design の旧 deferred concern「Test Runtime 結線」は no-command playthrough
supply に限り解消され、transport command stimulus・overflow fault injection は
引き続き後続 case とする。
