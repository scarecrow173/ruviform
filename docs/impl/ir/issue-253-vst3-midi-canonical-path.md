---
id: "IMPL-rf253vst3midicanonpath001"
type: "impl"
status: "completed"
title: "Issue 253 VST3 MIDI canonical path — IMidiMapping + plugin→host reporting"
created: "2026-11-12"
updated: "2026-11-12"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-253-vst3-midi-canonical-path.md"
      - type: "file"
        path: "docs/specs/issue-253-vst3-midi-canonical-path.md"
      - type: "file"
        path: "docs/designs/issue-253-vst3-midi-canonical-path.md"
      - type: "file"
        path: "docs/plans/implement-issue-253-vst3-midi-canonical-path.md"
      - type: "file"
        path: "docs/tasks/issue-253-vst3-midi-canonical-path.md"
      - type: "file"
        path: "docs/test-specs/issue-253-vst3-midi-canonical-path.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/processor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/midi.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/controller.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/gain.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/component.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/host.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/cap.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-253-vst3-midi-canonical-path.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-253-vst3-midi-canonical-path.md"
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
    - "docs/plans/implement-issue-253-vst3-midi-canonical-path.md"
    - "docs/test-specs/issue-253-vst3-midi-canonical-path.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-vst3-midi-canonical-path.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 253 VST3 MIDI canonical path — IMidiMapping + plugin→host reporting

## Summary

Issue #253 の (b)+(c) slice を実装した。(b) VST3 `IMidiMapping` 経由の
CC/channel-pressure/pitch-bend → normalized parameter 正規配送
（host→plugin 方向）と、(c) plugin→host reporting
（`outputParameterChanges`/`outputEvents` の null 配線解消 + host 側
収集 + 中立 `drain_output_reports` 契約）。fixture controller
（`GainController`）に `IMidiMapping` を実装し、fixture cdylib 経由の
in-process E2E（`host::tests`）で CC7→gain 配送・未マップ CC の
パススルー・双方向 report 往復を証明した。`cap.rs` の VST3
`param_reporting` を `Unavailable` → `Declared` に更新（adapter 面は
実装・fixture 証明済み、consumer 配線は未実施のため Verified ではない）。

## Context

- 実施タスク: [task](../../tasks/issue-253-vst3-midi-canonical-path.md)
- 上流: [spec](../../specs/issue-253-vst3-midi-canonical-path.md)、
  [design](../../designs/issue-253-vst3-midi-canonical-path.md)、
  [plan](../../plans/implement-issue-253-vst3-midi-canonical-path.md)
- 契約保証の正本: [test-spec](../../test-specs/issue-253-vst3-midi-canonical-path.md)
- 前提: PR #537（`9de1b2f4`）の双方向 MIDI 翻訳・`BridgeEventList`・
  `MockBackend`・fixture 検証済み基盤の上に構築。

## Implementation

### Changes

- `ruviform-plugin-core`:
  - `PluginProcessor::drain_output_reports(&mut self, out: &mut Vec<Event>)`
    を追加。default 実装は no-op（report しない plugin は無変更で動く）。
    `Plugin`/`ErasedPlugin` が同契約を mirror/forward する。
  - 契約は pull 型・非 RT（consumer buffer への append が allocate
    し得るため）。implementations は `process` 内から drain を呼ばない。
- `ruviform-plugin-vst3/src/midi.rs`:
  - `MidiAssignmentTable`（16ch × 130 controller の `ParamID` snapshot、
    `IMidiMapping::getMidiControllerAssignment` で `prepare` 時に捕捉）。
  - `control_assignment`/`routed_assignment`: `ControlChange`→`as_u7/127`、
    `ChannelPressure`→`kAfterTouch`/`as_u7/127`、`PitchBend`→`kPitchBend`/
    `bend_as_u14/16383`。`kNoParamId` は unmapped。
  - `fill_event_list` に `midi_map` 引数を追加し、routed event を
    `inputEvents` から抑制（param queue 側と単一 predicate
    `routed_assignment` で二重配送を防止）。
  - `MidiBridgeStats::param_routed` 追加。
- `ruviform-plugin-vst3/src/controller.rs`:
  - `GainController` が `IMidiMapping` を実装（volume CC7 全 channel →
    gain `ParamID`、他は `kNoParamId`/`kResultFalse`）。
- `ruviform-plugin-vst3/src/gain.rs`:
  - `GainProcessor` に bounded `pending_reports`
    （`REPORT_CAPACITY = MAX_BLOCK_EVENTS + 8`、`prepare` で reserve）。
    適用した `ParamValue` を echo report、`NoteOn` を MIDI report として
    queue。capacity 超過は defined drop。`drain_output_reports` 実装。
- `ruviform-plugin-vst3/src/component.rs`:
  - `GainComponent::emit_reports`: `process` 後に processor の report を
    `ProcessData.outputParameterChanges`（`addParameterData`+`addPoint`）と
    `outputEvents`（`outgoing_event`→`addEvent`）へ emit。null 出力・
    block 外 offset・queue 失敗・未対応 kind は `report_stats` で
    defined skip（block reject しない）。offset clamp なし。
- `ruviform-plugin-vst3/src/host.rs`:
  - `BridgeParamQueue`/`BridgeParameterChanges` に writable mirror を追加
    （input queue は read-only のまま、`addPoint` は writable のみ受理）。
  - `prepare_channels` で output queue/event-list を構築、`prepare` で
    `IMidiMapping` snapshot 捕捉。
  - `fill_param_queues` が `ParamValue` event と routed MIDI を queue へ
    分配し `param_routed` を計上。
  - `collect_output_reports`: plugin 出力を canonical `Event` に戻し
    `pending_reports` へ蓄積（frame offset で sort、容量超過・変換失敗は
    defined skip）。queue/event list は block 後に clear。
  - `PluginProcessor::drain_output_reports` で consumer へ公開。
  - `bounce_audio`/`prepare_channels` を inherent helper として抽出
    （`too_many_lines` lint 対応を兼ねた責務分離）。
- `ruviform-audio-plugin-bridge/src/cap.rs`:
  - VST3 `param_reporting` = `Declared`。L0/L1/CLAP は `Unavailable`
    のまま。`midi_channel_voice` は real-plugin 検証未実施のため
    `Declared` 維持。

### Decisions During Implementation

- **中立契約追加の判断（採用）**: `PluginProcessor::drain_output_reports`
  を driver-neutral な pull 型契約として追加した。根拠: (1) VST3 の
  `outputParameterChanges`/`outputEvents` は plugin→host の非同期 report
  面であり、入力 `EventList` とは逆方向のため既存 `process` 契約では
  表現不能。(2) 全 backend で `param_reporting: Unavailable` だったが、
  report 面は driver 固有概念ではなく「plugin が host へ返す canonical
  `Event` 列」という中立概念であり、default no-op により既存 plugin を
  破壊しない。(3) push 型（host が callback で受ける）ではなく pull 型
  （adapter が block 後に drain）を選択 — VST3 の process 呼出し境界と
  一致し、RT path の callback 再入を避けられる。VST3 固有概念
  （`IParameterChanges` 等）は adapter 内に閉じ込め、契約には漏らして
  いない。
- **`Declared` vs `Verified`**: adapter 面の出力 channel 実装と fixture
  E2E 証明は完了したが、host-graph consumer への配線（node 層での
  report 消費）と実 plugin/実 DAW での往復は未実施。capability 語彙の
  正直さのため `Declared` に留め、`Verified` 昇格は consumer 配線 +
  実機検証を待つ。
- **mapped MIDI の二重配送防止**: param queue への routing と
  `inputEvents` への suppression が単一 predicate
  （`routed_assignment`）を共有する設計とし、定義の divergence を
  構造的に排除した。
- **group != 0 の MIDI**: `IMidiMapping` は channel 単位の概念で
  UMP group を持たないため、nonzero group の routed 候補は defined
  skip（`param_routed` にも `translated` にも計上しない既存方針を維持）。

## Validation

実施済み（Windows 11 x64, rust 1.99.0 via mise）:

- `mise run fmt-check` — PASS
- `mise run check` — PASS（`cargo check --workspace --all-targets`）
- `cargo clippy -p ruviform-plugin-vst3 -p ruviform-plugin-core
  -p ruviform-audio-plugin-bridge --all-targets -- -D warnings` — PASS
  （補助証拠・canonical = mise）
- `cargo clippy -p ruviform-plugin-vst3 -p ruviform-plugin-core
  -p ruviform-audio-plugin-bridge --all-targets --features host
  -- -D warnings` — PASS（同上）
- `cargo test -p ruviform-plugin-vst3 --features host --lib` — 74 PASS
  （`imidi_mapping_routes_volume_cc_to_gain_param`、
  `unmapped_cc_passes_through_without_param_routing`、
  `plugin_output_reports_drain_as_canonical_events`、
  `assignment_table_capture_reads_fixture_controller`、
  `control_assignment_covers_cc_pressure_and_bend`、
  `routed_assignment_resolves_through_table`、
  `fill_event_list_suppresses_mapped_events`、gain report 2 件を含む）
- `cargo test -p ruviform-plugin-vst3 -p ruviform-plugin-core
  -p ruviform-audio-plugin-bridge`（default features）— PASS
- `mise run test-l1-real-plugin` — PASS
  （fixture cdylib + host runner build、`--lib --bins`、
  `l1_real_adapter_e2e -- --ignored` の `real_adapter_l1_full_series`
  が CLAP/VST3 両 adapter で PASS）
- `mise run lint-l1-real-plugin` — PASS
- `mise run lint` — PASS（`cargo clippy --workspace --all-targets
  -- -D warnings`）
- `mise run test` — PASS（`cargo test --workspace`）
- `mise run test-rt` — PASS（`cargo test --workspace rt_`）
- `git fetch origin main && git merge origin/main` — fast-forward to
  `317094b7`、競合なし（変更 crate との交差なし）。

NotRun / Unsupported:

- 実 OS MIDI backend（slice (a)）— 本 slice 対象外。Windows/macOS/Linux
  の実 MIDI input port・native timestamp・`InputStage` 配送・
  device-loss/recreation は Issue #253 の残 P1 gate。
- opt-in 実デバイス/実 plugin 検証（slice (d)）— 実機必須のため
  NotRun。`midi_channel_voice`/`param_reporting` の `Verified` 昇格は
  この検証待ち。
- 実 DAW 往復 — NotRun（実機 + 実 DAW 環境が必要）。
- host-graph consumer への report 配線 — `drain_output_reports` は
  adapter/fixture 面で証明済みだが、node 層の consumer は未配線
  （`Declared` の根拠）。

## Remaining

- Issue #253 は open のまま: 実 OS MIDI backend、opt-in 実デバイス
  lane、実 DAW 往復、report consumer 配線が残る。
- V9/V11/V12/V13 follow-up は本 slice の対象外（非 blocking）。
