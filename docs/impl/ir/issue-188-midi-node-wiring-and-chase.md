---
id: "IMPL-034Qx6XJISaIL6dOuXcRPc"
type: "impl"
status: "in-progress"
title: "issue-188 midi node wiring and chase"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/tests/midi_wiring.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/transport_schedule.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/transport_bridge.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/input.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/control.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/status.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/node.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/attach.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/lib.rs"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-188-timed-midi.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-188-timed-midi.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-188-timed-midi-detailed.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-188-timed-midi.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# issue-188 midi node wiring and chase

## Summary

Issue #188 残件: `InputStage` を `PluginNode`/`CallSink` の block 実経路へ配線し、録音済みイベントの `TransportScheduler` 経由 replay・loop/seek chase を実装する。

## Context

PR #236 (`9d02cca9`) は語彙・stamp provenance・clock mapping・bounded `InputStage`・NoteLedger・merge primitives・CLAP 翻訳を実装したが、`InputStage` は standalone のままで `PluginNode` の block 経路に接続されておらず、plan event replay の loop/seek chase も未実装だった。issue #188 のクローズアウト審査コメントの「未充足の残件」2 項目が本 record の対象。

## Implementation

### Changes

- `transport_schedule.rs`: `TransportScheduler::plan()` getter、`ScheduledEvent::position_ticks()` accessor、`event_generation_matches(ctx, ev)` helper を追加。`run_callback` の span 生成は不変のまま、chase/replay が同じ generation 規則を再利用できるようにした。
- `transport_bridge.rs` (plugin-core): `chase_span_events(plan, ctx, scratch) -> (&mut [Event], ChaseStats)` を追加。plan prefix (`position_ticks <= ctx.position_ticks`) を走査し、`NoteLedger` 相当の semantics (retrigger count・evict-oldest・`NOTE_LEDGER_CAPACITY` 有界 table) で「position 時点で発音中」の key へ最新 NoteOn を offset 0 で再発火する。`scanned`/`chased`/`untracked`/`truncated` を `ChaseStats` で返す。
- `input.rs`: `InputStage::pump(&mut Consumer)` (callback head で RT channel → `pending` へ bounded drain)、非 RT 側 `InputTx::pump(&mut dyn MidiInput)` (staging Vec → SPSC、backpressure は deferred/backend_dropped で可視化) と `PumpStats`、`InputConfig` を追加。
- `control.rs`: `NodeControl::SetClockAnchor { anchor: Option<ClockAnchor> }` と `PluginNodeControl::set_clock_anchor` を追加 (input leg なしでは defined no-op)。`NodeReport::LedgerOverflow` / `NodeReport::ChaseOverflow { dropped }` を追加し、`dispatch.rs` は新 variant を non-attributed として扱う。
- `status.rs`: input 系 counter mirror、notes_released / notes_chased / ledger_overflow / chase_overflow / event_scratch_overflow counter を追加。
- `node.rs`: `PluginNode` が `NoteLedger` と `last_ctx: Option<SpanCtxKey>` を保持。`process()` は `apply_controls` → `input_stage.pump(input_rx)` → scheduler `plan()` snapshot → `run_callback` の順で、`CallSink::span()` が `prepare_discontinuity()` (ctx key 変化時に `ledger.release_all` → `release_events`、`chase_span_events` → `chase_scratch`) を呼び、物理 call ごとに `run_call()` が `[release, chase, plan, live]` を `merge_call_events` で合成して ledger feed → endpoint `process` へ届ける。
- `attach.rs` / `lib.rs`: `AttachConfig.input: Option<InputConfig>` opt-in、`Attached.input_tx`、`AttachError::InputConfig`、public export を追加。
- `tests/midi_wiring.rs`: 実 `PluginHost` → `attach` → typed `GraphRuntime` 経路の integration test 11 本 (opt-in、anchor、offset 保持、replay、loop refire、seek release+chase、stop release、live note release、overflow report、RT no-alloc)。`transport_bridge.rs` 内に chase 単体テスト 6 本。

### Decisions During Implementation

- `AttachConfig::input: Option<InputConfig>` opt-in で input 経路を有効化。`Attached::input_tx: Option<InputTx>` が非 RT pump 境界 (backend `MidiInput::read` → bounded SPSC → `InputStage`) を担う。
- NoteLedger は `PluginNode` 所有とし、merge 後の delivered event list を feed する (live のみならず plan/chase 由来の note も追跡し、不連続で release_all)。
- Transport 不連続検出は span ctx の `(clock_epoch, transport_generation, plan_generation, occurrence)` タプル比較で行い、deliverable span の先頭 call で `release_all` (offset 0) + chase を merge する。
- chase は `chase_span_events(plan, ctx, scratch)` (plugin-core) で、plan prefix (position ≤ ctx.position_ticks) 内の generation-tag 一致イベントから「P 時点で発音中の note」の last NoteOn を offset 0 で再発火する。
- merge source order は `[release, chase, plan, live]` — release/chase が旧 era の teardown を先に届け、same-key collision は `merge_call_events` の NoteOff-priority が正規化する。

## Related Experiments

### Adopted

(None yet.)

### Rejected

(None yet.)

## Validation

- `mise run fmt-check` — green (after `cargo fmt --all` pass).
- `mise run check` — green.
- `mise run lint` — green (`-D warnings`)。
- `mise run test` — green (114 test-result sections, 0 failures)。
- `mise run test-rt` — green (`rt_midi_wiring_no_alloc` 含む)。
- 新規テスト: `tests/midi_wiring.rs` 11 本 (実 `GraphRuntime` 経路で live input offset 保持・plan replay・loop refire・seek release+chase・stop/live release・overflow report・RT no-alloc)、`transport_bridge.rs` chase 単体 6 本。
- review-protocol: 実施中 (Reviewer A/B → Validator → Judge)。

## Risks

- 不連続判定キー `(clock_epoch, transport_generation, plan_generation, occurrence)` に `status`/position は含めない設計 — seek/stop/play/plan install は tg/pg bump、loop wrap は occurrence bump で捕捉する。同 key で position だけが飛ぶ経路があれば stuck note の余地がある (review 観点)。
- Chase は note 状態のみ再構成する。CC/pitch/param の last-value chase は未実装 (follow-up として明記) — seek landing mid-CC-ramp で controller 状態は chase されない。
- `prepare_discontinuity` は deliverable span のみで走る。undeliverable span が連続すると release/chase は次の deliverable span まで遅延する (defined: undeliverable は silence + report 済み)。

## Follow-ups

- OS MIDI backend adapter (`MidiInputEnumerator` on midir/coremidi) — issue #188 残件、実機未検証。
- VST3 `Vst::Event` note/legacy-CC 翻訳、note_expression/midi_sysex 語彙 — issue #188 残件。
- Controller/param chase (last-value automation chase) — 録音 tap・engine 層と併せて分割 issue へ。
