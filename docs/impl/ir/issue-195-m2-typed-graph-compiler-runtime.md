---
id: "IMPL-034QgTkRw195m2audIoRtXwQ"
type: "impl"
status: "completed"
title: "Issue #195 M2 typed graph audio compiler/runtime: schedule, bounded buffers, PDC, stateful RT ownership, plan activation"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio/src/typed/mod.rs"
      - type: "file"
        path: "crates/ruviform-audio/src/typed/buffer.rs"
      - type: "file"
        path: "crates/ruviform-audio/src/typed/io.rs"
      - type: "file"
        path: "crates/ruviform-audio/src/typed/compile.rs"
      - type: "file"
        path: "crates/ruviform-audio/src/typed/runtime.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/typed_compiler.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/typed_runtime.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/typed_activation.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/legacy_golden.rs"
      - type: "file"
        path: "docs/impl/ir/issue-195-m2-typed-graph-compiler-runtime.md"
    modified:
      - type: "file"
        path: "crates/ruviform-audio/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio/src/lib.rs"
      - type: "file"
        path: "docs/test-specs/issue-195-m2-typed-graph-compiler-runtime-tests.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/m2-t1-typed-compiler-buffer-io.md"
    - "docs/tasks/m2-t2-typed-rt-runtime-activation.md"
    - "docs/tasks/m2-t3-tests-ownership-poc-faults.md"
    - "docs/tasks/m2-t4-golden-traceability-verification.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-195-m2-typed-graph-compiler-runtime-audit.md"
    - "docs/specs/issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md"
    - "docs/adr/m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md"
    - "docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md"
    - "docs/plans/implement-issue-195-m2-typed-graph-audio-runtime.md"
    - "docs/test-specs/issue-195-m2-typed-graph-compiler-runtime-tests.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "Stable Runtime Slot ownership (processor instance persists across plan generations in a stable registry slot)"
    rejected:
      - "Plan-owned state (instance rebuilt per plan → state reset; measured reset confirmed, rejected for state continuity)"
      - "Separate plan-retire channel (SnapshotGuard already retires its Arc; dropped the duplicate path)"
---

# Issue #195 M2 typed graph compiler/runtime implementation

## Summary

`ruviform-audio` に `typed` module を新設し、#184 の `ValidatedGraph`
を非 RT `Compiler` で immutable `PreparedPlan` へコンパイルし、単一
RT owner の `RtRuntime` が snapshot slot 経由の last-good plan を
実行する経路を実装した。`GraphRuntime`(非RT) が compile・generation
付き install/remove command・publish・retire 収集を所有し、RT
callback は arena・delay ring・processor registry すべてを
construction 時に確保済みで、warm-up 後 no-alloc/no-block/no-OS-I/O/
no-destructor。旧 `AudioNode`/`Graph`/`GraphPlan`/`PlanSwap` path は
無変更で golden fixture 下に共存。

## Context

正本: `docs/adr/m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md`
（Stable Runtime Slot 採用）。設計:
`docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md`。
計画: `docs/plans/implement-issue-195-m2-typed-graph-audio-runtime.md`。
テスト仕様:
`docs/test-specs/issue-195-m2-typed-graph-compiler-runtime-tests.md`。

## Implementation

### Changes

- `src/typed/compile.rs`: `Compiler` + `CompilerConfig`(max_frames/
  alignment/max_slots/max_channels/max_delay_lines/max_delay_frames/
  max_processors)。pass 分割: processor 割当(`NodeFactory` →
  Reuse/Boundary/New・stable pslot・removal 検出・allocator 解放) →
  port→slot binding + deterministic schedule → accumulated latency +
  merge 点 PDC delay-line(`max(inputs accum)-accum[edge]`) → aliasing
  audit → boundary input/output table。拒否は `CompileError`:
  `UnsupportedEdgeDelay`/`MissingProcessor`/`BufferAlias`/`SlotOverflow`/
  `DelayLineOverflow`/`DelayOverflow`/`ProcessorOverflow`/`Factory`/
  `Inconsistent`。u32→u16 channel 変換は checked。
- `src/typed/buffer.rs`: `AlignedSamples`(power-of-two alignment・非 RT
  確保・unsafe なし)・`Slot`(channels/valid_frames/seq stamp・
  `mark_written`/`fill_zero`/`configure`)・`DelayLine`(per-channel
  write cursor ring・`configure` reset・`process_add` は bounded・
  no-alloc)・`SlotSpec`/`DelaySpec`。
- `src/typed/io.rs`: `NodeIo<'_>` — 入力 port は producer slot をその
  場で読み、fan-in/channel-mapped 入力は scratch slot を参照。
  `RefCell` 内部可変で `&self` API。
- `src/typed/runtime.rs`: `GraphProcessor` trait(object-safe・
  latency/activate/event/process/deactivate・`FnMut` blanket impl)、
  `RuntimeCommand`(Install/Remove・generation 付き)、`NodeEvent`
  (generation 付き)、`GraphRuntime`(compile→command push→
  `SnapshotWriter::publish`・`collect` が snapshot+processor 両
  retire 路を drain)、`RtRuntime`(`Send`/`!Sync` 単一 owner・
  `process`/`input_writer`/`output_reader` が RT-suitable・
  generation 不一致 command/event は遮断+計数・`Drop` で全
  deactivate+retire)。`SharedStatus` は stale_events/stale_commands/
  uninit_reads/retire_overflows/plan_retire_overflows/fault_flags/
  active_generation を atomic で公開。
- `crates/ruviform-audio/Cargo.toml` + `lib.rs`: `ruviform-graph`
  path 依存と `pub mod typed;` のみ追加。旧 module 無変更。
- tests: `tests/common/mod.rs`(chain/merge/mapped fixture・stateful
  `MockDsp`・`MockFactory`・feed/drain) + 4 integration ファイル
  (下記 Validation の test 名が正本マッピング)。

### Decisions During Implementation

- **Plan-retire 二重経路の廃止**: `SnapshotGuard::drop` が既に自身の
  `Retirer` 経由で `Arc` を退避するため、runtime 側の独自 plan-retire
  channel は作らず `SnapshotWriter::collect` に一本化（AGENTS「正しい
  境界で修正」）。guard の drop は `reader.read()` ごとに発生し、
  `plan_retire_overflows` は reader 側 guard-drop 飽和を観測する
  （publish の displaced `Arc` は writer の local pending queue 行き
  でこの channel を満たさない — snapshot.rs 契約確認済み）。
- **`RefCell<RtState>` 採用**: `SnapshotGuard` の `reader` borrow と
  callback が mutate する registry/seq を型レベルで分離。`!Sync` は
  interior `RefCell` 由来で「単一 RT owner」を型で保証。
- **uninitialized-read は zero-fill degrade + fault 計数**: producer
  slot の `stamp != seq` を consumer materialize で検出し、読み出し
  自体を拒否（stale data を返さない）。boundary 未 feed でも安全。
- **`use std::sync::Mutex` は test/mock のみ**: RT 側は SPSC + atomics
  + `RefCell` のみ。`RtContext` capability 契約を全 RT API で保持。

## Validation

実行コマンドと結果（worktree `rf195-m2-audio-runtime`、Rust 1.99.0、
`origin/main` merge 後の最終検証）:

| コマンド | 結果 |
| --- | --- |
| `mise run fmt-check` (`cargo fmt --all -- --check`) | PASS |
| `mise run check` (`cargo check --workspace --all-targets`) | PASS（全クレート） |
| `mise run lint` (`cargo clippy --workspace --all-targets -- -D warnings`) | PASS |
| `mise run test` (`cargo test --workspace`) | PASS（0 failures・typed 系 36 件を含む） |
| `mise run test-rt` (`cargo test --workspace rt_`) | PASS（`rt_typed_process_no_alloc` を含む全 rt_ test 緑） |
| `cargo test -p ruviform-audio --test typed_compiler` | PASS（11 件） |
| `cargo test -p ruviform-audio --test typed_runtime` | PASS（6 件） |
| `cargo test -p ruviform-audio --test typed_activation` | PASS（12 件） |
| `cargo test -p ruviform-audio --test legacy_golden` | PASS（5 件） |
| `cargo test -p ruviform-audio`（lib + doctest 合計） | PASS（16 unit + 1 doctest） |

`assert_no_alloc` 監査は warm-up 後の `rt.process` 区間（feed 済み・
第 2 callback）で 0 allocation。`SnapshotReader::read` 初回の debt
node alloc は暖機として契約どおり監査窓外。

### G2 受入 Gate 証拠

| Gate | 判定 | 証拠（test 名 = `cargo test -p ruviform-audio --test <file> <name>` で再現可能） |
| --- | --- | --- |
| G2-01 deterministic compile | **PASS** | `typed_compiler::g2_01_chain_compiles_to_deterministic_schedule_and_slots`（src→proc→sink 順・producer slot/出力 port 数・boundary table）、`g2_01_repeated_compile_is_byte_identical`（同一入力 → byte 同一 plan）、`g2_01_fan_in_gets_scratch_slot`（2-edge merge に scratch） |
| G2-02 invalid 拒否（publish 前） | **PASS** | `typed_compiler::g2_02_*`（Declared edge delay・Processor の Boundary・factory 失敗・slot/processor/delay overflow・removal 検出 + allocator 解放・channel map binding）— 全て `CompileError` として個別識別 |
| G2-03 measured latency/PDC | **PASS** | `typed_runtime::g2_03_pdc_compensates_latency_difference_at_merge`（latency 3 vs 0 の merge で fast path edge に delay=3 を実測: frame0/2=2.0、frame3=13.0・`total_latency=3`・`plan.delays` 1 本）。graph-internal PDC のみで ADC/DAC・録音位置補正は scope 外（test 内 comment で明記） |
| G2-04 Plan-owned vs Stable Slot | **PASS** | `typed_activation::g2_04_stable_slot_preserves_state`（reuse → state 1→3 継続・install 1 回のみ）、`g2_04_plan_owned_replaces_state`（New → state リセット・deactivate→activate 順序）、`g2_04_install_remove_order_and_deferred_destruction`（remove → deactivate+retire）、`g2_04_rt_runtime_is_send_not_sync`（単一 owner の型証明） |
| G2-05 fault 系 | **PASS** | rebuild 失敗+last-good=`g2_05_publish_failure_retains_last_good`・activation conflict=`g2_05_event_channel_full_rejected`/`CommandChannelFull`・retire 飽和=`g2_05_retire_queue_saturation_flagged_leak_safe`・plan-retire 飽和=`g2_05_plan_retire_saturation_flagged`・stopped collector=`g2_05_retirement_deferred_and_collector_drain_observed`・shutdown=`g2_05_shutdown_retires_processors`・stale 遮断=`g2_05_stale_generation_events_rejected`/`g2_05_stale_commands_discarded_on_next_activation`（計数付き） |
| G2-06 frames/bounds/RT 契約 | **PASS** | `typed_runtime::g2_06_end_to_end_chain_at_64_128_256`・`g2_06_variable_and_odd_frame_counts`（1/13/96/200/255）・`g2_06_frames_above_max_rejected_and_zero_is_noop`（`FramesExceedMax{257,256}`・0 frame no-op）・`g2_06_unwritten_input_triggers_uninitialized_read_fault`（uninit_reads 計数+flag・stale 非読取）・`rt_typed_process_no_alloc`（warm-up 後 `assert_no_alloc` 窓内 process） |
| G2-07 legacy 共存 | **PASS** | `legacy_golden.rs` 5 件（single-bus chain・GraphPlan drive・PlanSwap・Mixer・closure node が golden 通り変換）+ lib 内既存 16 unit test 不変で緑 |
| G2-08 EX/TA traceability | **PASS（一部明示 unverified）** | test-spec「EX/TA Traceability Matrix」節を実装済み test 名で更新済み。EX-002 fanout・EX-004 sidechain/aux は fixture なしで明示 unverified。EX-013/014/015/017/018/019・TA-001/002/003/005/009/010/011 は M2 範囲外 N/A |

### EX/TA 対応（詳細は test-spec matrix 節が正本）

- verified: EX-001/003/005/006/007/008/010/011/012/016相当/020、
  TA-004/006/007/008/012。
- 明示 unverified（範囲内・fixture 未整備）: EX-002 fanout（scratch/
  aliasing 経路で間接担保）、EX-004 sidechain/aux、EX-009 の実 DAW
  意味付き edit。
- N/A: EX-013(MIDI)/014(prefetch)/015/017/018/019、TA-001/002/003/
  005/009/010/011。

## Risks

- `Direction` は `#[non_exhaustive]` — compiler は unknown direction
  を `Inconsistent` で拒否するが、将来 variant 追加時に要再評価。
- `plan_retire_overflows`/`retire_overflows` は leak-on-full の累積
  signal であり、飽和継続は leaked memory の増加を意味する —
  capacity/`collect` cadence の tuning は driver 側責務（design 記録
  済み・`Retirer` 契約どおり）。
- EX-002/004 の fixture 未整備は `unverified` として明示 — 追加分は
  後続 issue で補完が望ましい。
- Processor factory の install/remove command が同一 generation に
  収まらない場合の partial-push は publish 前 rollback でカバー済み、
  channel capacity は呼び出し側が delta 見積りで渡す設計。

## Follow-ups

- #196 plugin 実装: `GraphProcessor` を規格 bus（CLAP/VST3）の
  processor 実装で充足する経路。`PrepareOutcome::Boundary`/
  `MissingProcessor` の意味を plugin host 側で具体化。
- EX-002 fanout・EX-004 sidechain/aux の dedicated fixture 追加。
- `NodeEvent` を M3 の TimedEvent/Control payload へ拡張（Non-goal
  として分離済み）。
- `GraphRuntime::collect` の呼び出し cadence を consumer 側の
  control tick に合わせた運用契約（device/adapter issue 側）。
