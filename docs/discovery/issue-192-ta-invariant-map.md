---
id: "DISC-034WYeBHWSdbvryUnNKkus"
type: "discovery"
status: "active"
title: "Issue #192 TA-001〜012 Invariant ↔ 現行API/Issue/L1テスト対応表 + 依存非循環性監査"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-192-ta-invariant-map.md"
    - "docs/designs/issue-192-ta-invariant-map-design.md"
    - "docs/plans/issue-192-ta-invariant-map-plan.md"
    - "docs/tasks/issue-192-ta-invariant-map-task.md"
    - "docs/test-specs/issue-192-ta-invariant-map.md"
    - "docs/impl/ir/issue-192-ta-invariant-map.md"
    - "docs/impl/exp/issue-192-ta-invariant-map.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/adr/media-io-backend-abstraction.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
  derived-by:
    - "docs/specs/issue-192-ta-invariant-map.md"
    - "docs/designs/issue-192-ta-invariant-map-design.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/186"
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/189"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/191"
    - "https://github.com/scarecrow173/ruviform/issues/194"
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/197"
    - "https://github.com/scarecrow173/ruviform/issues/198"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/251"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/287"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/464"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/synvelis/issues/22"
    - "https://github.com/scarecrow173/synvelis/issues/25"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "https://github.com/scarecrow173/synvelis/issues/84"
    - "https://github.com/scarecrow173/synvelis/issues/106"
    - "https://github.com/scarecrow173/synvelis/issues/109"
    - "https://github.com/scarecrow173/synvelis/issues/176"
    - "https://github.com/scarecrow173/synvelis/issues/177"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/audio-lens/issues/143"
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  defers: []
  deferred-by: []
---

# Issue #192 TA-001〜012 Invariant ↔ 現行API/Issue/L1テスト対応表 + 依存非循環性監査

## Summary

Issue #192 の残件のうち headless に可能な監査スライスとして、umbrella
設計 §7 の TA-001〜TA-012 各 invariant を「現行 API / crate・設計判断・
実装 Issue・既存 OS-L1 テスト・未達 Headless Gap」へ 1 対 1 で対応付け、
あわせて `cargo metadata` の静的読みで Ruviform generic 層の依存境界と
feature 別依存の非循環性を確認した。結論: 12 gate 全てに merge 済みの
現行 API・実装・既存 L1 テストが対応し、依存 DAG は default /
all-features の両 feature 集合で循環 0・禁止通常依存方向の違反なし。
残る Headless Gap は実機・実 Plugin・実 Consumer 受入に限定され、各 Open
Gate Issue（#251/#253/#243/#262/#313/#388/#389/#465/#219/#281/#294/#306
等）へ接続済みであり、本スライスの完了条件には含めない。

## Exploration Goal

umbrella tracking Issue #192（および tracking canonical
`docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`）
の受入管理面で未整備だった 2 点を埋めることが目的:

1. TA-001〜TA-012 の各 invariant について、umbrella 設計が要求する証拠
   （依存 DAG 監査・型境界・Device/Host 非依存 Build・Fan-out/PDC・
   Plugin 依存方向・RT 所有権・RT NoAlloc・Generation 誤配送防止・
   PPQN 精度・WAV E2E・Window/Job 分離・Consumer 互換）を、**現行の
   公開 API・crate 構成・設計判断（ADR/正本）・実装 Issue・既存 OS-L1
   テスト**へ具体的に対応付け、実装済みで検証も存在する領域と、実装は
   あるが実機/実 Consumer 受入が残る Headless Gap を分離して可視化する。
2. Ruviform generic 層が AudioLens 精度/CLI・Synvelis DAW/RT/Project 側へ
   依存しない境界、および Cargo Feature 別の依存非循環性を、CI 実走や
   Run URL 収集を行わず `cargo metadata` の静的読みで確認し、検査方法・
   観測値・違反有無を記録する。

対象外: 実 Device/OS Native Dock/両 Consumer の実測（各 Open Gate の責務）、
TA gate の正式 Close 判定、索引の main 統合（Owner/Coordinator の責務）、
CI 実走証拠の収集。

## Audit Method and Evidence Basis

- **対象 SHA**: `365c74ac4b8e696044fb9bae386dfd03f86ecf32`
  （`origin/main` HEAD、2026-11-14 時点の本 worktree）。
- **DAG 監査**: `mise exec -- cargo metadata --format-version 1 --locked`
  （default feature）と同 `--all-features` を実行し、
  `resolve.nodes[].deps[].dep_kinds`（`kind`/`target`）を workspace
  member 間の path 依存に限定して集計。kind `null` は normal として扱う。
  循環検出は解決済み edge 集合への DFS。manifest 宣言側は
  `packages[].dependencies` を併記。Python 集計スクリプトは補助証拠であり、
  canonical 検証入口は mise タスク（本監査は `cargo metadata` のみ使用、
  build/test は未実行 — 文書監査のため）。
- **Issue 状態**: `gh issue view` で 2026-11-14 に確認した OPEN/CLOSED
  スナップショット。umbrella 設計正本の 2026-11-09 snapshot からの差分
  （#284/#285/#286/#288/#305 が CLOSED 化等）は本書で更新する。
- **型境界・API 棚卸し**: `crates/*/src` の公開 export・`pub struct/enum`
  宣言・`#[cfg(test)]`/`tests/` のテスト関数名を grep で棚卸し。
- **テスト対応**: 各 crate の `tests/` と src 内 `mod tests` のテスト関数名
  を列挙し、test-spec（`docs/test-specs/issue-*.md`）の Rule/Example ID
  （PPQN-01〜12・TT-008/025〜030・OF-01〜06・EX-001〜006・G2-xx 等）と
  対応付けた。テストの再実行は行っていない（実施済み証拠は各 Issue/CI
  記録を参照）。

## TA-001〜TA-012 対応表（正本）

「現行 API」は crate::代表シンボル。「L1 テスト」は既存の OS-L1
（headless/native build・単体/統合テスト）証拠で、実機・実 Plugin・実
Consumer 受入を意味しない。「Gap」は本スライスでは埋めず Open Gate Issue
へ接続する未達項目。

| Gate | Invariant（umbrella §7 要求証拠・要約） | 現行 API / crate | 設計判断・正本 | 実装 Issue | 既存 OS-L1 テスト | Headless Gap → Open Gate |
| --- | --- | --- | --- | --- | --- | --- |
| TA-001 | 本番通常依存の Cargo DAG 非循環、dev/optional/feature 別監査 | workspace 42 crate の path 依存解決 graph（下記 §依存非循環性監査）。`gui-boundary-guard`（`ruviform-ui-test` bin・Issue #390）が `cargo metadata --offline` + `boundary.json` fixture で UI driver 境界を検査 | umbrella §3 禁止通常依存・§4 crate 配置。M0 監査 `docs/discovery/issue-194-m0-baseline-audit.md` | #194 CLOSED（M0・28 crate/67 edge 時点）・#193 G0。継続監視は #294 系 CI | 本監査の `cargo metadata` 実測（default 98 edge / all-features 103 edge・循環 0）。M0 監査の宣言 67 edge・循環 0。`gui-boundary-guard` fixture negative tests | DAG 非循環の常設 CI gate は無し（`cargo metadata` 監査は手動/M0 時点）。consumer-verify 拡張 → #294 OPEN。両 Consumer tuple → #219/#281 OPEN |
| TA-002 | Foundation/Graph/Core の公開型に DAW Project/Track/Take/Undo の混入なし | `ruviform-graph`（`GraphSpec`/`NodeDef`/`PortDef`/`ConnDef`/`ValidatedGraph`/`Validator`/`GraphProject`/`PublishedGraph`/`StableHandleMap`/`GraphRevision`）、`ruviform-media-core`、`ruviform-audio-core`、`ruviform-plugin-core` の公開型。`GraphProject` は graph revision 発行 container で DAW Project ではない。`ruviform-edit` は NLE モデルとして意図的に分離維持 | `synvelis-driven-generic-execution-boundaries` ADR（accepted）・umbrella §2 非目標・#464 consumer-neutral contract | #184 CLOSED（M1）・#464 CLOSED | 型棚卸し（本書 Evidence: generic crate の `pub struct/enum Project|Track|Take|Undo` ヒット 0、Synvelis 言及は doc comment のみ）。`poc_ex_001〜006` | 型混入の CI 静的 gate 無し → review 責務（umbrella 正本どおり継続） |
| TA-003 | Graph IR/Validator を Device/Host なしで Build/Test | `ruviform-graph`: normal 依存は `ruviform-audio-core` のみ（device/plugin-host/ui 非依存 — §DAG監査で確認）。`Validator`/`ValidatedGraph`/`PortFormat`/`AudioFormat`/`ChannelLayout`/`Direction`/`PortRole`/`NodeKind` | umbrella §5.1・§3 target DAG（G→AC,M のみ）・M1 #184 | #184 CLOSED（PR #225 `8ad88f51`）。consumer lowering → SV #106 OPEN | `crates/ruviform-graph/tests/poc_ex.rs`: `poc_ex_001_port_attributes_are_orthogonal_and_preserved`・`poc_ex_002_multi_bus_topology_validates`・`poc_ex_003_connection_level_rejections`・`poc_ex_004_graph_level_rejections`・`poc_ex_005_revisions_and_runtime_mapping`・`poc_ex_006_failure_preserves_published_and_main_only_golden` | 全 Consumer topology の実測 → SV #106 OPEN（SF-03 lowering） |
| TA-004 | Fan-out/in・Sidechain・Explicit Mapping・PDC・Buffer Lifetime の正しさ | `ruviform-graph::mapping::StableHandleMap`（channel mapping）。`ruviform-audio::typed`: `Compiler`/`CompiledPlan`/`ScheduledNode`/`SlotAllocator`/`Slot`/`DelayLine`/`AlignedSamples`/`EdgeBinding`/`DelaySpec`/`InputBinding`/`PreparedPlan`。legacy 互換 path: `graph::Graph`/`AudioNode`、`plan::GraphPlan`/`PlanSwap`/`RtPlan`、`mixer::Mixer` | umbrella §5.2（Prepared Schedule ≠ Stateful Processor）・`audio-engine-snapshot-swap` ADR（維持）・#195 M2 | #184/#195 CLOSED（PR #225/#227 `85f5caba`） | `typed_compiler.rs`: `g2_01_chain_compiles_to_deterministic_schedule_and_slots`・`g2_01_repeated_compile_is_byte_identical`・`g2_01_fan_in_gets_scratch_slot`・`g2_02_declared_edge_delay_rejected`・`g2_02_channel_mapped_edge_binds_map`・`g2_02_slot_and_processor_overflows_rejected`・`g2_02_delay_overflow_rejected`・`g2_02_reuse_keeps_slot_and_skips_install`・`g2_02_removed_node_is_scheduled_for_removal`。`typed_runtime.rs`: `g2_03_pdc_compensates_latency_difference_at_merge`・`g2_06_end_to_end_chain_at_64_128_256`・`g2_06_variable_and_odd_frame_counts`・`g2_06_unwritten_input_triggers_uninitialized_read_fault`・`rt_typed_process_no_alloc`。`legacy_golden.rs` 5 件 | 実機 RT 予算の最終受入 → #195 系残件（EXT-L3/実機）。SV lowering 経由の実測 → SV #106 |
| TA-005 | PluginHost は Audio Engine 通常依存なし、Audio Adapter は正方向のみ | `ruviform-plugin-host`: `PluginEndpoint`/`InProcessHost`/`PluginHost` composite/`supervisor`/`editor_session`/l1 runner。normal 依存 = audio-core・media-core・observability・plugin-core・rt・video-core（`ruviform-audio` は dev-dep のみ）。Adapter = `ruviform-audio-plugin-bridge`（`attach`/`node`/`dispatch`/`target`/`vault`/`factory`/`control`/`input`/`bus`/`cap`/`status`）が audio・plugin-host 双方へ正方向依存 | umbrella §1 訂正済み依存事実・§3 禁止通常依存・`plugin-rt-ownership-split` ADR | #196 CLOSED（PR #232 `1305538c`）。M0 で dev-only 依存を確認済み | 本監査 `cargo metadata`: plugin-host→audio は dev のみ、plugin-{clap,vst3}→plugin-host も dev-only、bridge→{audio,plugin-host} は normal（adapter 役割）。`plugin_graph_e2e.rs`・`l1_*_e2e.rs`・bridge `e2e.rs`/`frame_base.rs`/`midi_delivery.rs`/`midi_wiring.rs`/`param_delivery.rs` | 実 Backend/Plugin 互換の受入継続 → #253 OPEN（OS MIDI/VST3 backend）・実 plugin → `test-l1-real-plugin` lane（fixture のみ・実規格 plugin ではない） |
| TA-006 | 同一 Stateful Plugin/DSP が旧/新 RT Plan から同時に Process されない | `ruviform-audio::typed`: `RtRuntime::publish`/`ActivationError`/`SharedStatus`/`RuntimeCommand`、retire queue + collector。`ruviform-rt`: `retire.rs`（RetireQueue・saturation leak-safe）・`snapshot.rs`（Arc swap guard→retire）。legacy: `plan::PlanSwap`（`RwLock<Arc<GraphPlan>>`/`try_read`・last-good cache・`retire_channel`） | umbrella §5.2/§5.3・`plugin-rt-ownership-split`・`audio-engine-snapshot-swap` ADR（維持・再評価。typed runtime は互換 path と並行、置換は後継 ADR） | #195/#196 CLOSED | `typed_activation.rs`: `g2_04_stable_slot_preserves_state`・`g2_04_plan_owned_replaces_state`・`g2_04_install_remove_order_and_deferred_destruction`・`g2_04_rt_runtime_is_send_not_sync`・`g2_05_stale_generation_events_rejected`・`g2_05_publish_failure_retains_last_good`・`g2_05_stale_commands_discarded_on_next_activation`・`g2_05_retirement_deferred_and_collector_drain_observed`・`g2_05_retire_queue_saturation_flagged_leak_safe`・`g2_05_plan_retire_saturation_flagged`・`g2_05_event_channel_full_rejected`・`g2_05_shutdown_retires_processors`。rt: `rt_retire_*`/`rt_snapshot_*` 系。`legacy_golden.rs::legacy_plan_swap_golden` | 実 Plugin での最終確認 → #253 OPEN・consumer 実測（SV/AL） |
| TA-007 | RT に Alloc/Dealloc・Blocking・OS I/O・非 RT Destructor がない（既存 RT test＋review＋stress） | `ruviform-rt`: `spsc`（SPSC endpoint）・`retire`・`snapshot`・`alloc_counter`・`clock`・`context`（`RtContext` capability）・`processor`（`prepare`/`process` 分離）。`assert_no_alloc` crate を検証窓に使用。CLAP adapter は `thread-check` 拡張を `plugin.init` で照会 | `realtime-contract-and-verification-policy` ADR・`docs/development-guidelines.md` RT 検証レーン（`rt_` prefix + assert_no_alloc 窓）。静的検証不能制約は review 責務と明記 | #195/#196（typed runtime への適用）・RT lane は Issue #22 系で整備 | `mise run test-rt`。rt_: rt crate（`rt_retire_no_alloc`/`rt_try_push_pop_no_alloc`/`rt_snapshot_read_no_alloc`/`rt_endpoint_send_bounds` 等）、media-core（`rt_run_callback_no_alloc`/`rt_device_restart_no_alloc`/`rt_ledger_no_alloc`/`rt_map_stamp_no_alloc`）、audio typed（`rt_typed_process_no_alloc`/`rt_probe_process_allocates_nothing`）、plugin-core `boundary.rs` | 実機 RT 認証（デバイス/OS 依存）→ EXT-L3・Consumer 受入。OS/実機証拠は各 Issue/consumer-verify の責務 |
| TA-008 | Plan/Transport/Target/Instance Generation 不整合な Event を誤配送しない | `ruviform-media-core::transport_schedule`: `TransportPlan`/`TransportPlanBuilder`/`SpanContext`/`ScheduledEvent`/`TransportGeneration`/`PlanGeneration`/`ClockEpochId`/`LoopOccurrence`/`AppliedReceipt`/`ScheduleFault`/`DeliveryPolicy`/`BlockContract`/`event_generation_matches`/`event_engine_frame`。`ruviform-plugin-core`: `Event`/`EventKind::{ParamValue,Midi}`/`EventList`（frame_offset 昇順・同一 offset 順序保証）/`sort_events_by_frame_offset`/`merge_call_events`/`transport_bridge` | umbrella §5.4・Synvelis v0.5 §14 契約・#186/#188/#190 | #186/#188/#190 CLOSED（PR #226/#228/#234/#236/#252） | `plugin-core/tests/transport_scheduler.rs`: `tt008_command_receipt_lifecycle`・`tt025_loop_wrap_splits_and_transports`・`tt026_min_frames_exact_never_calls_under_min`・`tt026_degraded_defers_to_legal_boundary`・`tt027_in_block_unproven_not_treated_as_supported`・`tt028_event_process_relative_offsets`・`tt029_loop_occurrence_distinguishes_deliveries`・`tt010_same_frame_source_order`・`stale_plan_generation_event_not_delivered`・`tt009_restart_vs_seek_epoch_generation`・`tt012_span_budget_overflow_faults`・`tt029_zero_frame_flush_contract`・`tt029_l0_l1_min_frames_difference`・`tt030_offline_exact_vs_live_degraded`。media-core `transport_schedule.rs` tests: `stale_transport_generation_event_is_faulted_not_delivered`・`device_restart_vs_seek_distinct_epoch_generation`・`install_rejects_same_generation_different_plan`・`loop_events_fire_once_per_occurrence`・`same_frame_events_keep_source_order`・`min_frames_violation_*`・`pending_budget_rejects_overflow`。audio `typed_probe_*` の rebind/generation 系 | Plugin min_frames 問題 → #190/Synvelis 側で追跡。実 Plugin での確認 → #253 OPEN |
| TA-009 | Variable PPQN/Beat/Tempo/Loop → Sample Frame/Event の精度・順序維持 | `ruviform-media-core::music`: `Ppqn`（1..=u16::MAX 検証）・`TickPosition`・`BeatPosition`・`TempoMap`/`TempoMapBuilder`・`MusicalTimeInfo`・`MusicalPosition`・`PPQN`（旧 960 投影）・`Tick`/`TickDuration`。`time`: `MediaTime`/`TimeBase`/`FrameRate`/`Rounding`/`MediaRange`/`MediaDuration`。`timed_input`: `map_stamp_to_frame`/`ClockAnchor`/`ClockEpochId`/`TimedInputEvent` | `configurable-ppqn-resolution-independent-musical-time` ADR（accepted → #189 で実装済み）・旧 `musical-time-model` ADR は superseded・CLAP Fixedpoint 2^31 Erratum | #189 CLOSED（PR #226）。#186/#188（timed event/MIDI）CLOSED | media-core `music.rs` tests: `ppqn_validates`・`tick_position_binds_resolution`・`tick_position_projection`・`builder_sorts_and_validates`・`beat_to_media_constant_tempo`・`tempo_boundary_anchor_exact`・`beat_position_normalizes/arithmetic`。`time.rs` tests: precision matrix（`single_conversion_error_is_bounded_by_half_tick`/`round_trip_error_is_bounded_by_summed_half_ticks`/`chained_conversion_error_is_bounded_by_per_hop_half_ticks`/`conversion_exact_and_rounding`/`conversion_overflow_surfaces`）。test-spec `docs/test-specs/issue-189-m3-rational-beat-ppqn-tests.md` Rules 1〜10（PPQN-01〜12 の Ruviform 担当分）。plugin-core tt 系（§TA-008） | CLAP/VST3 adapter 丸め PoC の継続 → #253 OPEN。SV 側 PPQN/Timebase 適用は consumer 責務（#189 系設計どおり） |
| TA-010 | 実 PCM WAV Encode→Drain→Mux Finalize→再 Open 検証 | `ruviform-io-wav`: `WavEncoder`/`WavMuxer`/`WavReceipt`/`open_boxed_muxer`/`WavOutputSpec`。`ruviform-media-io`: `Source`/`Demuxer`/`Decoder`/`Sink`/`Encoder`/`Muxer` trait・`StreamInfo`/`Frame`/`Packet`/`CodecDescriptor`。`ruviform-io-file`: `StagedFile`/`SealedFile`（分類済み publish 契約）。`ruviform-audio::render`: `PreparedRender`/`RenderSpec`/`TailSpec`。`ruviform-exec`: `OfflineDriver`/`OfflineJob` | `media-io-backend-abstraction` ADR・IO-05 #287（旧 `StagedOutput`/`PublishOptions` を互換 Facade なしに削除する合意済み breaking change・#305 CLOSED で移行完了）・#191/#198 | #191/#198/#287/#305 CLOSED。IO E2E → #306 OPEN。consumer → SV #25 CLOSED・AL #143 CLOSED・SV #84 OPEN | `io-wav/tests/contract.rs`: `capabilities_are_honest_and_queryable`・`non_pcm_codec_is_unsupported`・`unsupported_wire_format_is_unsupported`・`multi_stream_is_unsupported`・undeclared/invalid/mismatch 系 rejection・`write_after_finish_and_double_finish_error`・`sink_write_error_is_io_not_completion`・`sink_error_at_finalize_is_io_not_completion`・`staged_abort/drop/no_overwrite/replace/disk_full` publish 安全系。`roundtrip.rs`: rate/layout/wire format 往復・`zero_frame_file_reopens_empty`・`nan_inf_and_clip_values_follow_declared_policy`・`drain_then_receive_then_none`・`send_after_drain_is_invalid_data`。audio `render.rs`: `of05_wav_reopen_verify`・`of06_failure_injection`・`of06_staging_policy`・`of06_publish_lifecycle`・`of01〜of04`。exec `offline.rs` tests。`io-e2e` `fault_boundary.rs`/`rooted_boundary.rs` | IO 合成 E2E・Fault/Fuzz/OS/FS → #306 OPEN。SV #84（StagedFile consumer 移行）OPEN。実機 FS/OS → EXT-L3 |
| TA-011 | Window Close で Application/Project-owned Job が無断 Cancel されない | `ruviform-ui::runtime`: `UiRuntime::handle_window`/`cancel_window_jobs`（opt-in・自動呼出なし）/`drain_jobs_routed`/`RoutedEvent`/`CloseDecision`/`WindowLifecycle`（driver 契約）。`ruviform-exec::job`: `JobScope`/`OfflineJob`/`JobEvent`/`CancellationToken`/`Progress`/`JobState`。`dock`: `DockWorkspace` two-phase move | umbrella §5.5 Job（window close と job cancel は独立）・#181/#182・`ui-architecture-webview-hybrid` → `driver-neutral-gui-…-issue-384` ADR（proposed=仮決定） | #181/#182 CLOSED（PR #237/#240 `d27b9572`） | `ui/tests/multi_window.rs`: `close_requested_does_not_cancel_jobs`・`subwindow_close_leaves_jobs_and_loop_running`・`destroy_is_distinct_from_close_request`・`deferred_close_stays_alive_then_destroyed`・`job_events_route_to_owner_window`・`delivery_to_dead_window_is_safe`・`multi_window_events_stay_separated`・`colliding_correlation_ids_route_by_window`。`runtime.rs`: `job_cancel_reaches_terminal_state`・`job_progress_*`。`dock.rs`: `window_close_applies_consumer_chosen_disposition`・two-phase move 系。exec `job.rs` tests: `cancel_moves_to_cancelling_then_finished_cancelled` 等 | 実 GUI での最終確認 → #243 OPEN（GPUI 実機検証）・#313 OPEN（OS capability matrix/real-GUI smoke）・#388/#389 OPEN（native dock/session）・#262 OPEN（GPUI 採否）・#465 OPEN（WebView 撤去） |
| TA-012 | Audio/Video/Plugin/CLI/UI/Legacy Consumer が互換 API で回帰 | 互換 surface: `ruviform-audio` legacy（`Graph`/`AudioNode`/`PlanSwap`/`Session`/`Mixer`/`render`）、`ruviform-plugin-{clap,vst3,ofx,export}` adapter、`ruviform-{video,video-proc,video-core}`、`ruviform-cli-sample`（leaf consumer）、`ruviform-{ui,ui-webview,ui-gpui,viz,edit}`、`ruviform-media-io`/`io-symphonia`/`io-avi`。`.github/workflows/consumer-verify.yml` + `mise run test-ci-consumer-verify` | #219 `consumer-compat-contract` spec（proposed）・#192 2026-11-02 両 Consumer 対等追補・umbrella §9（deprecation は Consumer 採用と実テスト後）・#281 OBS-06・#294 tuple gate | #219/#281/#294 OPEN。consumer: SV #28 OPEN・#83 CLOSED・#106 OPEN・#109 OPEN・#176/#177 OPEN、AL #127 OPEN・#142 CLOSED・#166 CLOSED | `legacy_golden.rs`: `legacy_single_bus_chain_golden`・`legacy_graph_plan_drive_golden`・`legacy_plan_swap_golden`・`legacy_mixer_golden`・`legacy_closure_node_golden`。`test-l1-real-plugin` lane（CLAP/VST3 fixture cdylib + `host` feature L1 E2E・6 件実走 — fixture であり実規格 plugin ではない）。`test-ofx*`/`test-plugin-core`/ui 各 lane。workspace `cargo test`（M0 時点 1784 pass） | 両 Consumer 独立受入未完 → #219/#281/#294 OPEN。SV #28/#106/#109・AL #127 OPEN。SV #84 OPEN。実 plugin/device → #253/#251 |

## Area 別 現行 API 対応（DAG / RT / Plugin / Time / Media / Window）

| Area | 主要 crate | 現行 API（代表） | TA 対応 |
| --- | --- | --- | --- |
| DAG / Graph IR | `ruviform-graph`・`ruviform-audio::typed` | `GraphSpec`/`Validator`/`ValidatedGraph`/`GraphProject`/`StableHandleMap`・`Compiler`/`CompiledPlan`/`ScheduledNode`/`SlotAllocator` | TA-002/003/004 |
| RT 基盤 | `ruviform-rt`・`ruviform-audio::typed::RtRuntime` | `Spsc`/`RetireQueue`/`Snapshot`/`alloc_counter`/`RtContext`/`Processor`・`publish`/`ActivationError`/`SharedStatus` | TA-006/007 |
| Plugin | `ruviform-plugin-{core,host,clap,vst3,ofx,export}`・`ruviform-audio-plugin-bridge` | `Event`/`EventList`/`EventKind`/`PluginDescriptor`/`ParamInfo`/`TransportSpan`・`PluginEndpoint`/`InProcessHost`/`PluginHost`/supervisor・bridge `attach`/`node`/`dispatch`/`target`/`vault` | TA-005/006/008 |
| Time / Event | `ruviform-media-core` | `Ppqn`/`TickPosition`/`BeatPosition`/`TempoMap`/`MusicalTimeInfo`・`MediaTime`/`TimeBase`/`FrameRate`/`Rounding`・`TransportPlan`/`ScheduledEvent`/`SpanContext`/Generation 系・`MidiEvent`/`NoteLedger`/`TimedInputEvent`/`map_stamp_to_frame` | TA-008/009 |
| Media / Offline | `ruviform-media-io`・`io-{symphonia,avi,wav,file,archive,structured,compression}`・`media-reader`・`ruviform-audio::render`・`ruviform-exec` | `Source`/`Demuxer`/`Decoder`/`Sink`/`Encoder`/`Muxer`・`StagedFile`/`SealedFile`・`WavEncoder`/`WavMuxer`/`WavReceipt`・`PrefetchProducer`/`PrefetchConsumer`・`PreparedRender`/`RenderSpec`・`OfflineDriver`/`OfflineJob` | TA-010 |
| Window / UI / Job | `ruviform-ui`・`ui-{webview,gpui,test,theme}`・`ruviform-exec::job`・`ruviform-viz` | `UiRuntime::handle_window`/`cancel_window_jobs`/`RoutedEvent`/`CloseDecision`/`WindowLifecycle`/`DockWorkspace`/`DataRegistry`・`JobScope`/`JobEvent`/`CancellationToken` | TA-011/012 |

## 依存非循環性監査（cargo metadata 静的読み）

### 方法

`mise exec -- cargo metadata --format-version 1 --locked`（default feature）と
`--all-features` の 2 回を実行し、`resolve.nodes[].deps[].dep_kinds` を
workspace member 間の path 依存に限定して edge を抽出（`kind: null` =
normal）。edge 集合に DFS を掛けて循環を検出。feature ゲート・dev/build
区別・target 条件は `dep_kinds` と manifest `dependencies` の
`optional`/`kind`/`target` で判定した。集計は補助 Python スクリプトで実施
（canonical = mise 経由の `cargo metadata` 実行）。

### 観測値（SHA `365c74ac`）

| 指標 | default features | all-features |
| --- | --- | --- |
| workspace members | 42 | 42 |
| resolve packages 総数 | 306 | 1021 |
| ws→ws normal+build edge | 98 | 103 |
| ws→ws dev edge | 26 | 26 |
| ws→ws 全 kind edge | 124 | 129 |
| **循環（normal+build）** | **0** | **0** |
| **循環（全 kind — dev 込み）** | **0** | **0** |
| target 条件付き path 依存 | 0 | 0 |
| git/外部 repo 由来依存 | 0（外部は全て crates.io registry） | 同左 |

all-features でのみ活性化する ws→ws edge（5 件）:

- `ruviform-plugin-clap → ruviform-plugin-export`（feature `host`、optional）
- `ruviform-plugin-vst3 → ruviform-plugin-export`（feature `host`、optional）
- `ruviform-poc-native-dock → ruviform-plugin-vst3`（feature `vst3`、optional・`host` feature 付き）
- `ruviform-ui-gpui → ruviform-ui-test`（feature `gpui`/`gpui-test`、optional）
- `ruviform-ui-gpui → ruviform-ui-theme`（feature `gpui`、optional）

M0（#194・2026-11-01・28 crate 時点）からの差分: member 28→42
（IO-IMP 系 `io-file`/`io-structured`/`io-compression`/`io-archive`/`io-wav`/
`io-e2e`/`media-reader`/`midi-device`、UI 系 `ui-gpui`/`ui-theme`/`ui-test`/
`poc-native-dock`、`audio-plugin-bridge`/`audio-analysis`/`numeric` 等の追加）、
resolved normal+build edge 67（manifest 宣言）→98、dev edge 5→26
（test-only crate の dev 依存増加を含む）。宣言↔解決の差分は feature ゲート
optional edge の活性化で説明でき、M0 が記録した `host` ゲート 2 edge と
整合する。

### 境界検査（umbrella §3 禁止通常依存 + Consumer 境界）

| 検査 | 結果 |
| --- | --- |
| Graph IR（`ruviform-graph`）→ Audio Engine/Plugin Host/GUI | 違反なし（normal 依存は `ruviform-audio-core` のみ） |
| Audio Engine（`ruviform-audio`）→ Plugin Host/plugin-* | 違反なし（plugin 系への依存 0） |
| Plugin Host（`ruviform-plugin-host`）→ Audio Engine | 違反なし（`ruviform-audio` は dev-dep のみ） |
| Plugin Host/plugin-{clap,vst3} → plugin-host | dev-only（通常依存ではない） |
| Observability（`ruviform-observability`）→ 重い Plugin/Graph/UI | 違反なし（normal 依存は `media-core`/`rt` のみ） |
| Foundation/Primitives → Engine/Adapter/UI/Consumer 製品層 | 違反なし（`math`/`numeric`/`rt`/`media-core`/`audio-core`/`video-core`/`exec`/`observability`/`dsp`/`audio-analysis`/`edit`/`viz`/`ui-theme` は下位層のみを指す） |
| generic 層 → AL 精度/CLI・SV DAW/RT/Project | repo 内に AL/SV への path/git 依存は存在しない（外部依存は全て crates.io）。`ruviform-cli-sample`/`ruviform-ui-test`/`ruviform-io-e2e`/`ruviform-poc-native-dock` は leaf であり本番 crate からの被依存 0 |
| `ruviform-audio-plugin-bridge` → {`ruviform-audio`, `ruviform-plugin-host`} | normal 依存あり — これは TA-005 の「Audio Adapter が正方向で両者を接続する」設計どおりの adapter 役割であり違反ではない |
| `ruviform-ui` → `ruviform-exec`/`ruviform-rt` | ui は consumer 側層であり foundation ではない。Job primitive（exec/rt）への依存は umbrella §5.5 の Job/UI 境界と整合 |
| `ruviform-graph` → `ruviform-audio-core` | target DAG の `G --> AC` 許可 edge（audio-core は domain foundation であり Audio Engine `ruviform-audio` ではない） |

**違反: なし。** default / all-features の両 feature 集合で循環 0、
umbrella §3 の禁止通常依存方向は全て不在、feature ゲート optional edge も
境界を侵さない。なお `gui-boundary-guard`（Issue #390）は UI driver 境界を
`cargo metadata` + fixture で CI 検査する既存の静的 gate だが、repo 全体の
DAG 循環を常設検査する CI lane は存在しない — これは違反ではなく監視 gap
（TA-001 の Gap 欄に記録）。

## Headless Gap → Open Gate 接続一覧

実 Device・OS Native Dock・両 Consumer 実測の未完了は、対応表の各 Gate
「Gap」欄のとおり次の Open Issue が Owner（本スライスの完了条件に含めない）:

| 領域 | Open Issue（2026-11-14 `gh` 確認） |
| --- | --- |
| 実 Audio Input / duplex・実機 capture | #251 OPEN（`test-audio-real-input` manual lane・`evidence/rf251-audio-input/`） |
| OS MIDI backend・VST3 翻訳 | #253 OPEN（`test-midi-real-device` manual lane・`evidence/issue-253-real-midi-lane/`） |
| GPUI 実機 GUI 検証 | #243 OPEN・#313 OPEN（`evidence/rf313-os-capability-matrix/`・`issue-313-windowed-uitest/`） |
| Native Dock / Plugin Editor Session | #388 OPEN・#389 OPEN（`evidence/rf388-389-native-dock/` 等） |
| GPUI Native 採否・WebView 撤去 | #262 OPEN・#465 OPEN |
| Cross-Consumer 互換契約・OBS-06・tuple gate | #219 OPEN・#281 OPEN・#294 OPEN |
| IO 合成 E2E・Consumer 受入 | #306 OPEN・SV #84 OPEN |
| Consumer 側継続追従・実装 | SV #28/#106/#109/#176/#177 OPEN、AL #127 OPEN |

## Key Issues and Assumptions

- umbrella 設計 §7 の TA-001〜012「Required evidence」文を invariant の正本
  として採用し、tracking canonical（migration 設計正本）の 2026-11-09
  snapshot 表を 2026-11-14 の実測で更新した。Issue 状態は変動するため
  本書の状態記述は 2026-11-14 時点の snapshot。
- 「merge 済み（headless/CI/PoC）」と「Consumer 受入・実機検証」は別状態
  として記録し、前者を後者の証拠にしない（POLICIES §3・umbrella §7）。
- `cargo metadata` の解決 edge は宣言 edge と一致しない（feature ゲート
  optional・dev-only・target 条件で活性化が変わる）。M0 が区別した
  「manifest 宣言 vs resolver 活性」の方法論を踏襲し、default/all-features
  の 2 集合で報告した。
- dev-dependency の上位参照（plugin-host→audio、plugin-{clap,vst3}→
  plugin-host、io-e2e の横断 dev 依存等）は umbrella §3 の「既存テスト用
  dev-dependencies の上位参照を同じ意味の循環と扱わない」に従い違反と
  しない。

## Alternatives and Comparison

監査手法の選択肢:

| Option | Pros | Cons | 採用 |
| --- | --- | --- | --- |
| `cargo metadata` 静的読み（本監査） | 再現可能・環境非依存・feature 集合を制御可能・CI 実走不要 | build 成功や runtime 振る舞いは示さない | 採用（タスク指定どおり） |
| `cargo tree` / 手動 Cargo.toml 読み | 軽量 | feature 活性・dev/build 区別の再現性が劣る | 不採用（metadata を一次証拠に） |
| CI 実走 + Run URL 収集 | 統合証拠として強い | 本スライス範囲外（Coordinator の証拠確認に残す）・headless 監査の目的に対し過剰 | 不採用（受入条件が静的確認を指定） |
| TA ごとの crate 全読み深掘り | 網羅性最大 | 42 crate 全読は監査目的に対し非効率・既存正本/テスト棚卸しで十分 | 不採用（索引レベル対応 + 代表 API 確認に限定） |

## Tentative Conclusions and Hypotheses

**Confirmed（実測・棚卸しで確定）:**

- TA-001〜012 の全項目に、merge 済みの現行 API・実装 Issue・既存 OS-L1
  テストの対応が存在する（対応表参照）。
- `cargo metadata` 実測で workspace 42 member・normal+build ws→ws edge
  98（default）/103（all-features）・dev 26・循環 0（両 feature 集合・
  dev 込み全 kind でも 0）。umbrella §3 の禁止通常依存方向は全て不在。
- AL/SV repo への Cargo 依存（path/git）は存在せず、外部依存は全て
  crates.io registry。Consumer 製品層 crate（cli-sample/ui-test/io-e2e/
  poc-native-dock）は leaf。
- Issue 状態の 2026-11-14 snapshot: #284/#285/#286/#288/#305 は CLOSED
  （2026-11-09 snapshot の OPEN から進捗）。#251/#253/#243/#262/#313/
  #388/#389/#464→CLOSED/#465/#219/#281/#294/#306 は OPEN のまま。

**Hypothesis / 要 Owner 確認:**

- DAG 非循環の CI 常設 gate 不在は「監視 gap」であり現行違反ではない。
  gate 化の要否は #294/#219 系の CI 設計と Owner 判断に委ねる。
- TA-002 の型混入は grep 棚卸しでは検出されなかったが、doc comment 内の
  Synvelis 言及（`audio-core/device.rs`・`observability` の例示）は
  実装混入ではない。公開型の網羅監査は review 責務の継続事項。

## Open Questions

- DAG 監査の再実施頻度・CI 化の要否（#294 consumer-verify tuple gate と
  の関係を含め Owner 判断）。
- TA gate の正式 Close 判定と #192 自体の Close（証拠棚卸し時）。
- `gui-boundary-guard` の fixture がカバーしない repo 全体 DAG の
  feature×target 組合せ（例: `cfg` 別依存は現状 0 件だが将来追加時の
  扱い）。
- Consumer 側の独立受入完了時期（SV #28/#106/#109・AL #127・SV #84）。

## Promotion Candidates

- [x] spec-doc needed: 本監査の文書チェーン
  `docs/specs/issue-192-ta-invariant-map.md`（spec）を作成し
  `relations.derives-from` で本書を参照。
- [ ] adr-doc needed: なし（設計判断の変更を伴わない監査記録）。
- [ ] issue needed: なし（Gap は全て既存 Open Issue に接続済み。新規
  Issue 起票は行わない）。

## Sources and Relations

- umbrella 設計（権威）: `docs/designs/synvelis-driven-target-architecture-and-migration.md`
  （approved 2026-11-01）§3 target DAG・§7 TA-001〜012。
- tracking canonical: `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
  （approved）TA gate 対応表・capability map・3 者 Ownership。
- M0 baseline: `docs/discovery/issue-194-m0-baseline-audit.md`（28 crate・
  67 edge・循環 0 の先行実測）。
- 実装 Issue 群: #181/#182/#184〜#191/#194〜#198（全 CLOSED）、
  #219/#243/#251/#253/#262/#281/#294/#306/#313/#388/#389/#465（OPEN）。
- Consumer Issue: Synvelis #22/#25/#28/#84/#106/#109/#176/#177、
  AudioLens #127/#143/#166（状態は 2026-11-14 snapshot）。
- 検証規約: `POLICIES.md`（OS-L1/GUI-L2/EXT-L3・Required/Deferred）、
  `docs/development-guidelines.md`（RT 検証レーン）。

## Evidence

### 2026-11-14: 静的監査実行記録

**対象 SHA**: `365c74ac4b8e696044fb9bae386dfd03f86ecf32`（`git rev-parse HEAD`）。
**環境**: Windows 11・Git Bash・mise 管理 toolchain。canonical 検証入口は
mise タスク;`cargo metadata` は `mise exec --` 経由で実行。

| # | コマンド | exit | 結果要点 |
| --- | --- | --- | --- |
| a | `mise exec -- cargo metadata --format-version 1 --locked` | 0 | 306 packages・42 workspace members |
| b | `mise exec -- cargo metadata --format-version 1 --locked --all-features` | 0 | 1021 packages（feature 全活性） |
| c | Python 集計（`resolve.nodes[].deps[].dep_kinds` → ws→ws edge・DFS 循環検出） | 0 | default: normal+build 98 / dev 26 / 全 kind 124・循環 0。all-features: 103 / 26 / 129・循環 0。target 条件 path 依存 0 |
| d | `gh issue view`（ruviform #184〜#465・synvelis #22〜#177・audio-lens #127〜#166 の状態確認） | 0 | §Headless Gap 表の OPEN/CLOSED snapshot |
| e | grep 棚卸し（`crates/*/src` の `pub use`/`pub struct|enum`/`#[test]`・`tests/` 関数名） | 0 | §対応表の API・テスト列の根拠 |

**再現手順**: 同 SHA の worktree で `mise exec -- cargo metadata
--format-version 1 --locked [--all-features]` を実行し、
`resolve.nodes[].deps[].dep_kinds` を workspace member 間に限定して
`kind`/`target` 別に集計すれば同一の edge 数・循環判定を再算出できる
（`kind: null` は normal 扱い）。Issue 状態は `gh issue view <N>
--repo <owner/repo> --json state,title` で再確認可能。

**検証の限界**: 本監査は静的読みと既存テスト棚卸しであり、テスト・build の
再実行、CI Run の収集、実機/実 Plugin/実 Consumer の実測は行っていない
（各 Open Gate・Coordinator の証拠確認の責務）。`cargo metadata` の解決
edge は feature・target・dev/build 条件の集計であり、公開 API の型境界
（TA-002）は grep 棚卸し＋review 責務の補完である。
