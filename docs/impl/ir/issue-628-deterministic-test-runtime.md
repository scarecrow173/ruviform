---
id: "IMPL-034XCdra59tUm0jQNLqYQw"
type: "impl"
status: "completed"
title: "Issue #628 deterministic test runtime core"
created: "2026-11-14"
updated: "2026-11-14"
owners:
  - "devin-worker"
relations:
  source: []
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-audio-test/"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-628-deterministic-test-runtime.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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
    adopted: []
    rejected: []
---

# Issue #628 deterministic test runtime core

## Summary

Issue #628 first slice として test-only クレート
`crates/ruviform-audio-test` を新設し、deterministic core
（VirtualClock・決定論的 PCM fixture・production
`TransportScheduler` 経由の MIDI trace・独立 oracle・Case Schema v1
run metadata）を実装した。approved spec/design/plan/test-spec/task
チェーン（`docs/{specs,designs,plans,tasks,test-specs}/
issue-628-deterministic-test-runtime.md`）に従う。

## Context

- spec: `docs/specs/issue-628-deterministic-test-runtime.md`（approved）
- design: `docs/designs/issue-628-deterministic-test-runtime-design.md`（approved）
- plan: `docs/plans/issue-628-deterministic-test-runtime-plan.md`（approved）
- test-spec: `docs/test-specs/issue-628-deterministic-test-runtime.md`
- task: `docs/tasks/issue-628-deterministic-test-runtime.md`
- run metadata 語彙: `scarecrow173/ruviform#627`（Case Schema v1）と
  POLICIES.md 2026-11-15 仮想優先改訂

## Implementation

### Changes

- `crates/ruviform-audio-test/Cargo.toml` — workspace metadata 継承、
  path 依存は `ruviform-audio-core` / `ruviform-media-core` /
  `ruviform-rt` のみ、dev-dependency は `assert_no_alloc = "1.1"`。
  依存追加理由をコメントで記録。
- `src/lib.rs` — crate-level doc で test-only 境界・RT 契約・本
  slice 範囲を宣言。
- `src/clock.rs` — `VirtualClockConfig`（offset/drift_ppm/jitter/
  epoch/sample_rate）+ `VirtualClock`（`MasterClock` 内包の atomic
  wait-free read + monotonic nanos）+ `VirtualClockDriver`
  （advance/pause/resume/seek/deadline）。tick/monotonic は `i128`/
  `u128` 整数 math のみで libm 非依存・bit 決定論的。
- `src/pcm.rs` — `PcmPattern`（Silence/Constant/Impulse/Ramp/
  Square/Sine/Sweep/Noise）、`PcmFixture::sample_at`/`fill`/
  `fill_channel`/`fill_view`/`block`、`PcmSegment`/`PcmScenario`
  （format/channel 変化と drop-frame gap を表現）。全 pattern が
  絶対 frame の純関数（position independence）。Sine は 2^32 fixed
  point phase + Taylor 近似（libm 非依存）、Sweep は瞬時周波数の
  閉形式積分、Noise は stateless per-frame splitmix64 hash。
- `src/midi.rs` — `MidiScenario`/`MidiCue` を production
  `TransportPlanBuilder` で `TransportPlan<MidiEvent>` に compile、
  `run_midi_trace` が production `TransportScheduler::run_callback`
  を block 駆動し `TraceSink`（`SpanSink` 実装）で delivered event
  を捕捉。seek/loop/pause 等の `TransportCommand` を due frame で
  投入可能。receipts/faults/fault_overflow も記録。
- `src/oracle.rs` — `pcm_reference_block`（libm `f64::sin` 使用の
  独立実装）、`midi_expected_playthrough`（scheduler 非経由の独立
  sort+frame 変換）、`pcm_assert_eq`/`pcm_bits_eq`/
  `midi_trace_eq`/`midi_trace_frames_monotonic`。
- `src/metadata.rs` — `RunMetadata`/`RunParameters` + Case Schema v1
  enum（`CaseRequirement`/`CaseResult`/`VerificationLevel`/
  `EnvironmentKind`）。`environment_kind=DeterministicBackend`、
  `level=OsL1` 固定の `deterministic()` コンストラクタ。
- `tests/{clock,pcm,midi,metadata,rt_contract}.rs` — 37 test + 3
  doctest。`rt_contract.rs` は `#[global_allocator]` 付き
  `assert_no_alloc` で clock read・PCM fill・`run_callback`+sink
  経路の allocation-free を検証（`rt_` prefix、`mise run test-rt`
  で収集）。
- `Cargo.toml`（root）`members` 追加、`crates/README.md` 索引行追加、
  `Cargo.lock` 再生成。

### Decisions During Implementation

- **MIDI trace の捕捉点は `SpanSink`**: repo に production MIDI
  output port は存在しないため、outgoing trace は `SpanSink::span`
  境界で捕捉する設計（design doc 記載どおり）。
- **Sine/Sweep は libm 非依存**: 生成側は fixed-point phase +
  Taylor 多項式で bit 決定論を担保し、oracle 側は libm `f64::sin`
  の独立実装を `AbsEpsilon` 比較に使う（生成経路と期待値経路を分離）。
- **`run_midi_trace` は implicit Play@0 + plan install**: scenario
  の基本形を簡潔にし、seek/loop/pause は `config.commands` で表現。
- **`PcmPattern` の pub field は spec-as-data**: 構築後の直接変更は
  validation を bypass するが deterministic な出力を維持する
  （`validate()` を公開）。
- **clippy cast lint は `#[expect]` で局所許可**: f64→f32 narrowing・
  u64→f64 phase 変換は deterministic spec の意図的 IEEE-754 演算で
  あり、reason 付き `#[expect]` で文書化（`#[allow]` 不使用）。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-628-deterministic-test-runtime-core.jsonl` —
  実装過程の観察記録（PCM planar slice の test bug 修正、
  `usize::from(u32)` 非存在、`assert_no_alloc` の
  `#[global_allocator]` 要件など）。

### Rejected

- なし。

## Validation

canonical 検証（`mise run`、Windows 11 x64・rustc 1.99.1）：

- `mise run fmt-check` — pass
- `mise run check ruviform-audio-test` — pass（`--locked --all-targets`）
- `mise run lint ruviform-audio-test` — pass（`-D warnings`）
- `mise run test ruviform-audio-test` — pass（37 tests + 3 doctests）
- `mise run test-rt` — pass（workspace 全体、`rt_` filter で
  `rt_contract.rs` の 4 test が実際に収集・実行されたことを出力で確認）

証拠: `evidence/rf628/verification.md`

## Risks

- Sine/Sweep の oracle 許容誤差（`AbsEpsilon(1e-5)`）は libm 版と
  多項式版の既知差分をカバーするが、将来 pattern 追加時は再校正が
  必要。
- `run_midi_trace` の implicit Play@0 は単一 plan scenario 前提。
  plan 途中差し替え・tempo map は後続 slice。
- `VirtualClock` の jitter は period 単位の step 関数であり、
  sample 単位ジッタではない（fixture 用途としては十分、design どおり）。

## Follow-ups

- scripted port-trait backend・fault injector・production pipeline
  配線・CI lane 統合・consumer oracle（AudioLens/Synvelis）は Issue
  #628 後続 slice。
- Windows WASAPI/WinMM virtual endpoint（`scarecrow173/ruviform#629`）、
  real ABI test plugin（`scarecrow173/ruviform#630`）、GPUI scenario
  runner（`scarecrow173/ruviform#631`）。
