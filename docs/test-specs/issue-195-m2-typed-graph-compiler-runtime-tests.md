---
id: "TSPEC-034QfqUJnhyPPfGZsl7kXW"
type: "test-spec"
status: "approved"
title: "Issue #195 M2 typed graph compiler/runtime: schedule, buffers, PDC, ownership, activation, faults, legacy coexistence"
created: "2026-11-01"
updated: "2026-11-01"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/195"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md"
    - "docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md"
    - "docs/adr/m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md"
  verified-by: []
  references:
    - "docs/test-specs/audio-engine-tests.md"
    - "docs/test-specs/plugin-rt-retirement-tests.md"
  defers: []
  deferred-by: []
---

# Issue #195 M2 typed graph compiler/runtime test spec

## Purpose

Issue #195 の G2-01〜G2-08 受入を、`ruviform-audio` の integration test `tests/typed_compiler.rs`・`tests/typed_runtime.rs`・`tests/typed_activation.rs`・`tests/legacy_golden.rs`（共通 fixture は `tests/common/mod.rs`）と既存 suite で保証する。実行した検証コマンド・結果・未検証項目を impl record に記録する。

## Feature

Typed Graph IR → 非 RT Compiler → immutable PreparedPlan → Stable Runtime Slot 単一 owner RT runtime（compile/slot/buffer/PDC/activation/generation/fault/legacy 共存）。

## Rules

- R-SCHED: 有効 multi-bus DAG（Main+Aux+Sidechain）は deterministic schedule（`StableHandleMap` runtime index 順）と slot mapping を持ち、同一入力の 2 回 compile で同一 plan。
- R-BUF: slot は prepare 時に最大数・`max_frames`・alignment≥64B で確保。input/output slot は disjoint（aliasing 拒否）。producer slot は consumer より先行 write（uninit-read 拒否 = fault+zero-fill degrade）。
- R-REJ: `UnsupportedEdgeDelay`/`MissingProcessor`/`BufferAlias`/`SlotOverflow`/`DelayOverflow`/`BoundaryConflict`/`Factory` は個別に識別可能な失敗として非 RT で返る。incomplete plan は publish されない。
- R-PDC: merge 点で `edge_delay = max(inputs accum) - accum[edge producer]`。`total_latency` が報告され、`max_delay_frames` 超過は `DelayOverflow`。PDC は graph 内整列のみ（ADC/DAC・録音位置補正と独立）。
- R-OWN: 同一 NodeId の processor は plan を跨いで同一 instance（状態継続）。install/remove は generation 付き command のみ。旧 generation event は遮断+計数。同一 instance は callback あたり 1 回のみ process。
- R-FAULT: rebuild 失敗・activation conflict（command channel full）・retirement queue full・collector stopped・shutdown で last-good 維持・RT destructor なし・fault が非 RT 観測可能。
- R-FRAMES: `frames <= max_frames` で 64/128/256/可変列が正しく処理。`frames > max_frames` は `FramesExceedMax`。`frames == 0` は no-op。
- R-RT: RT 区間の alloc/blocking/OS-I/O は `alloc_counter` で 0。RT destructor なし。
- R-COMPAT: 旧 `AudioNode`/`Graph`/`GraphPlan`/`PlanSwap` golden fixture が byte 同一 PCM を出し、既存 suite は不変で緑。

## Examples

- `ex01_main_chain`: Source(boundary)→Processor→Sink(tap) で compile+process・output PCM deterministic。
- `ex02_fanout_single_run`: 1 出力→2 consumer。producer は 1 回のみ process・両 consumer は同一 slot を read（aliasing なし）。
- `ex03_fanin_mix_order`: 2 branch→merge input で deterministic sum（edge 宣言順）。
- `ex04_sidechain_aux`: Sidechain/Aux port が別経路で供給・main 出力に混入しない。
- `ex05_layout_map`: layout 相違接続は `ChannelMap` 明示で受理・未宣言は validator 側 `ImplicitConversionRequired`（M1 既検証・ここでは compile 経由で確認）。
- `ex06_reject_cycle_and_missing`: zero-delay cycle・missing node/port は validator→prepare で拒否（publish なし）。
- `ex07_pdc_merge`: 128 vs 512-frame latency branch が merge で整列（PCM 位相一致）。384-frame 補正。
- `ex08_latency_rebuild_continuity`: latency 変更で再 compile・同一 NodeId processor の状態継続・last-good plan 維持。
- `ex09_structural_edit_no_rt_block`: 再生中の構造 edit は非 RT compile+publish・RT block なし・revision/status が返る。
- `ex10_stateful_reuse`: 同一 stateful processor が二重 process/drop なしに再利用。
- `ex11_retire_saturation`: retirement queue full → `mem::forget` ではなく `retire_overflows` 観測可能・RT drop なし。
- `ex12_control_no_recompile`: fader/send 相当の event は topology recompile なしで配送（generation 照合）。
- `ex20_outdated_project_status`: 失敗 spec 後の requested/published 区別と active plan の status が区別可能。

## Guarantees

- `mise exec -- cargo test -p ruviform-audio --test typed_compiler --test typed_runtime --test typed_activation --test legacy_golden` exit 0。
- `mise run test`/`test-rt` 緑・既存不変（`rt_` 命名の RT 監査 test 含む）。
- EX-001〜EX-020・TA-001〜TA-012 の対応表を impl record に記録（N/A・未検証を明示）。

## EX/TA Traceability Matrix（実装済み test 名 → 対象 ID・検証状況）

| Test (実装名) | Covers / 状況 |
| --- | --- |
| `g2_01_chain_compiles_to_deterministic_schedule_and_slots` + `g2_01_repeated_compile_is_byte_identical` + `g2_06_end_to_end_chain_at_64_128_256` | EX-001・R-SCHED・TA-004 — verified |
| `g2_01_fan_in_gets_scratch_slot` + `g2_03_pdc_compensates_latency_difference_at_merge` | EX-003・TA-004/006 — verified（fan-in は scratch 経由の deterministic sum） |
| `g2_02_channel_mapped_edge_binds_map` | EX-005・TA-004 — verified（ChannelMap 明示受理。未宣言の ImplicitConversionRequired は M1 validator 側検証済み） |
| `g2_02_declared_edge_delay_rejected`/`g2_02_processor_kind_without_instance_rejected`/`g2_02_factory_failure_propagates_and_consumes_nothing`/`g2_02_slot_and_processor_overflows_rejected`/`g2_02_delay_overflow_rejected`/`g2_02_removed_node_is_scheduled_for_removal` | EX-006・R-REJ・TA-004 — verified（cycle/missing の validator 拒否は M1 既検証） |
| `g2_03_pdc_compensates_latency_difference_at_merge` | EX-007・R-PDC・TA-006 — verified（実測 3-frame delay・total_latency=3・ADC/DAC・録音補正とは別レイヤと明記） |
| `g2_04_stable_slot_preserves_state` + `g2_02_reuse_keeps_slot_and_skips_install` | EX-008・EX-010・R-OWN — verified（state continuity・same pslot・no reinstall） |
| `g2_04_plan_owned_replaces_state` | EX-010 対照（Plan-owned では state リセット）・ADR 選定根拠 — verified |
| `g2_04_install_remove_order_and_deferred_destruction` + `g2_04_rt_runtime_is_send_not_sync` | R-OWN・単一 RT owner・install/remove 順 — verified |
| `g2_05_stale_generation_events_rejected`/`g2_05_stale_commands_discarded_on_next_activation` | EX-012・R-OWN（generation 照合・旧 event/command 遮断+計数）・TA-008 — verified |
| `g2_05_publish_failure_retains_last_good`/`g2_05_event_channel_full_rejected` | EX-020・R-FAULT（rebuild 失敗・activation conflict → last-good・requested/published 区別）・TA-008 — verified |
| `g2_05_retire_queue_saturation_flagged_leak_safe`/`g2_05_plan_retire_saturation_flagged`/`g2_05_retirement_deferred_and_collector_drain_observed`/`g2_05_shutdown_retires_processors` | EX-011・R-FAULT（retirement saturation・stopped collector・shutdown・leak-on-full・RT destructor なし）・TA-008 — verified |
| `g2_06_end_to_end_chain_at_64_128_256`/`g2_06_variable_and_odd_frame_counts`/`g2_06_frames_above_max_rejected_and_zero_is_noop`/`g2_06_unwritten_input_triggers_uninitialized_read_fault`/`rt_typed_process_no_alloc` | G2-06・R-BUF/R-FRAMES/R-RT・TA-007 — verified（64/128/256・可変・zero・over-max・uninit・no-alloc 監査） |
| `legacy_golden.rs`（5 test） | G2-07・R-COMPAT・TA-012・EX-016 相当 — verified（旧単一 bus Graph/GraphPlan/PlanSwap/Mixer 不変） |
| 未検証（M2 範囲内だが fixture なし） | EX-002 fanout（同一 slot 共有 read — scratch/alias 経路で間接）、EX-004 sidechain/aux、EX-009 の実 DAW 意味付き edit — 明示的 unverified |
| N/A（M2 範囲外） | EX-013(MIDI)・EX-014(prefetch)・EX-015・EX-017・EX-018・EX-019、TA-001/002/003/005/009/010/011 |

## Non-goals

- plugin 実接続・規格 bus（M4）。TimedEvent/Control payload（M3）。実デバイス I/O・offline WAV（#191・TA-010 は N/A）。Synvelis DAW 意味。

## Risk

- 本保証を落とすと Issue #195 の G2 受入証拠が消失し、typed path が検証なしに main へ残る。
