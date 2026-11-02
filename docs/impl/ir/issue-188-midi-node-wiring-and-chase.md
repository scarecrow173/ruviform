---
id: "IMPL-034Qx6XJISaIL6dOuXcRPc"
type: "impl"
status: "completed"
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
- `node.rs`: `PluginNode` が `NoteLedger` と `last_ctx: Option<(SpanCtxKey, i64)>` を保持。`process()` は `apply_controls` → `input_stage.pump(input_rx)` → scheduler `plan()` snapshot → `run_callback` の順で、`CallSink::span()` が `prepare_discontinuity()` (ctx key 変化または position が前 span の期待後続位置と不一致のとき `ledger.release_all` → `release_events`、`chase_span_events` → `chase_scratch`) を呼び、物理 call ごとに `run_call()` が `[evict, release, chase, plan, live]` を `merge_call_events` で合成して ledger feed → endpoint `process` へ届ける。
- `attach.rs` / `lib.rs`: `AttachConfig.input: Option<InputConfig>` opt-in、`Attached.input_tx`、`AttachError::InputConfig`、public export を追加。
- `tests/midi_wiring.rs`: 実 `PluginHost` → `attach` → typed `GraphRuntime` 経路の integration test 14 本 (opt-in、anchor、offset 保持、replay、loop refire、seek release+chase、stop release、live note release、overflow report、RT no-alloc、同 epoch SetClock ジャンプ、foreign-epoch SetClock 拒否、ledger eviction synthetic off)。`transport_bridge.rs` 内に chase 単体テスト 7 本。

### Review-protocol fixes (Reviewer A/B → Validator → Judge: BLOCKED → fixed)

- V1: `last_ctx` を `(SpanCtxKey, i64)` 化し、playing 時は前 span の `ctx.position_at_offset(span_frames)` を期待後続位置として保持。同 epoch `SetClock` で frame cursor が跳び position が不連続になっても release+chase が走る。
- V2: `NoteLedger` に `evicted: [Option<LedgerKey>; NOTE_LEDGER_CAPACITY]` の有界 queue を追加。capacity eviction は旧 key を queue し、`feed` は `forget_evicted` で新 state への殺害を回避、`CallSink::drain_evicted` が毎 call offset 0 で synthetic NoteOff を merge 先頭へ届ける。`release_all` は evicted 債務も吐出し doc 契約どおりとなった。
- V4: `TransportScheduler::device_restart` は `mem::take(&mut self.pending)` をやめ、pending を in-place で reject して `clear()` — RT 上の dealloc と reserved capacity 喪失を解消。同じ容量喪失バグを `drain_receipts`/`drain_faults` でも修正 (`drain(..).collect()` で capacity 保持)。
- V5: `PendingKind::InstallPlan` で同 `PlanGeneration`・別 `Arc` の plan install を `RejectReason::StaleGeneration` で reject (receipt 記録)。同一 `Arc` reinstall は benign no-op のまま。
- V6: `merge_call_events` に `src_scratch: &mut [u8]` 由来 provenance を追加。NoteOff-priority は cross-source collision のみに適用し、同一 source の authored `[On, Off]` 順序 (zero-length note) を保存する。moved-off frontier で複数 collision の相対順も安定。
- V7: `chase_span_events` を `CHASE_SCAN_BUDGET = 2048` 有界の backward scan 化 — plan 長に非依存の O(budget×capacity) で position 直近から遡り、`ChaseStats::incomplete` が budget 超過を報告 (`ChaseOverflow` へ反映)。
- V8a: chase は `plan.generation == ctx.plan_generation` の snapshot のみで走る — mid-callback install で stale snapshot を chase しない。
- V10: `InputStage` は `feed()` 成否に関わらず `take_overflow()` を毎回 poll し、ledger eviction 由来の undercount を解消。
- V14: foreign-epoch `SetClock` を `Counter::StaleEpochControl` + `NodeReport::StaleEpochControl` で拒否 — scheduler が表現不能な epoch へ node cursor を遷移させない。

### Decisions During Implementation

- `AttachConfig::input: Option<InputConfig>` opt-in で input 経路を有効化。`Attached::input_tx: Option<InputTx>` が非 RT pump 境界 (backend `MidiInput::read` → bounded SPSC → `InputStage`) を担う。
- NoteLedger は `PluginNode` 所有とし、merge 後の delivered event list を feed する (live のみならず plan/chase 由来の note も追跡し、不連続で release_all)。
- Transport 不連続検出は span ctx key `(clock_epoch, transport_generation, plan_generation, occurrence)` の比較に加え、playing 中は position が前 span の期待後続位置と一致するかも確認する。position は frame-derived のため ctx key が変わらない同 epoch `SetClock` ジャンプも不連続として捕捉する (V1)。
- chase は `chase_span_events(plan, ctx, scratch)` (plugin-core) で、plan prefix (position ≤ ctx.position_ticks) を `CHASE_SCAN_BUDGET` 有界で backward scan し、generation-tag 一致イベントから「P 時点で発音中の note」の last NoteOn を offset 0 で再発火する (V7)。snapshot generation 不一致時は chase しない (V8a)。
- merge source order は `[evict, release, chase, plan, live]` — evicted-key synthetic off が最優先で、release/chase が旧 era の teardown を続けて届ける。same-key collision の NoteOff-priority は cross-source のみに適用され、authored same-source `[On, Off]` の順序は保存される (V2/V6)。

## Related Experiments

### Adopted

(None yet.)

### Rejected

(None yet.)

## Validation

- review-protocol: Reviewer A/B → Validator (13/14 confirmed) → Judge (BLOCKED: P1 V1/V2/V4/V6/V7 + blocking-P2 V5)。全件修正済み。V9/V11/V12 は non-blocking follow-up、V13 は deferred contract、V3 は rejected (dead defensive code)。
- `git merge origin/main` — clean merge 後に fmt/lint 違反 2 件を修正 (drain_collect allow・cast truncation)。
- `cargo fmt --all -- --check` — green。
- `cargo check --workspace --all-targets` — green。
- `cargo clippy --workspace --all-targets -- -D warnings` — green。
- `cargo test --workspace` — green (0 failures)。
- `cargo test --workspace rt_` — green (`rt_midi_wiring_no_alloc`、`rt_device_restart_no_alloc` 含む)。
- 新規テスト: `tests/midi_wiring.rs` 14 本、`transport_bridge.rs` chase 単体 7 本 (scan budget 含む)、`event.rs` merge provenance 2 本、`transport_schedule.rs` install-rejection/restart 3 本、`midi.rs` ledger evicted-key 3 本。

## Risks

- Chase は note 状態のみ再構成する。CC/pitch/param の last-value chase は未実装 (V13: deferred contract として follow-up へ明記)。
- `prepare_discontinuity` は deliverable span のみで走る。undeliverable span が連続すると release/chase は次の deliverable span まで遅延する (defined: undeliverable は silence + report 済み)。
- Backward chase の emission order は position 直近 key 順 — key 間の順序に契約上の意味はないが、forward 時代の順序に依存する下流があれば注意。

## Follow-ups

- OS MIDI backend adapter (`MidiInputEnumerator` on midir/coremidi) — issue #188 残件、実機未検証。
- VST3 `Vst::Event` note/legacy-CC 翻訳、note_expression/midi_sysex 語彙 — issue #188 残件。
- Controller/param chase (last-value automation chase, V13 deferred contract) — 録音 tap・engine 層と併せて分割 issue へ。
- V9: over-budget event window は call 全体を drop する — excess のみ truncate する縮退が望ましい。
- V11: `InputStage` の独立 ledger/counter (`release_all` 未使用) は node 権威と乖離し得る — 一本化または削除の検討。
- V12: clock-anchor republish の epoch/domain mismatch で pending input event が消費され得る — live-source 不連続扱いまたは pending 保全を検討。
- `install_plan`/`request` の doc は non-RT allocating と記載するが、node は callback 経路で呼び capacity guard により無割当で動作 — doc 更新検討。
