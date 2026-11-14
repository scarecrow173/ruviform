---
id: "TSPEC-034XBMh4pr5313Dan0adpY"
type: "test-spec"
status: "approved"
title: "Issue #628 テスト仕様 — deterministic core の決定性・RT contract・独立 Oracle"
created: "2026-11-14"
updated: "2026-11-14"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/specs/issue-628-deterministic-test-runtime.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-628-deterministic-test-runtime.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-628-deterministic-test-runtime-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-628-deterministic-test-runtime-plan.md"
    - "docs/specs/issue-628-deterministic-test-runtime.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-628-deterministic-test-runtime-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-628-deterministic-test-runtime-plan.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-628-deterministic-test-runtime.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 テスト仕様 — deterministic core の決定性・RT contract・独立 Oracle

## Purpose

`crates/ruviform-audio-test` deterministic core が、同一 seed/fixture
version/clock 設定から再現可能な PCM/MIDI trace を生成し、RT-safe 経路が
alloc/lock/wait-free であること、および独立 Oracle が被試験コードと
期待値生成を共有しないことを保証する。本 slice の対象は deterministic
core のみであり、scripted backend・fault injector・production 配線・
CI lane・consumer oracle・3 OS 実走は対象外（後続 slice）。

## Feature

Deterministic Audio/MIDI Test Runtime — deterministic core

## Rules

- **R1 決定性**: 同一 `PcmFixture`/`PcmScenario`/`MidiScenario`/
  `VirtualClockConfig`/seed から、PCM block・MIDI trace・run metadata が
  bit-identical に生成される。`fill`/`block`/`run_midi_trace`/`tick_at` は
  libm 非依存・HashMap 非依存・f64 IEEE 四則のみ・pure integer PRNG。
- **R2 位置独立性**: PCM 生成は absolute frame の純関数 — `fill(start_frame)`
  を任意位置から呼んでも同一 frame に同一値を返す（noise は stateless
  per-frame hash、sine/ramp/square は fixed-point phase、sweep は f64
  closed-form）。
- **R3 RT contract**: `VirtualClock::tick`/`monotonic_nanos`、
  `PcmFixture::fill`、`run_callback`+`TraceSink` は allocation-free・
  non-blocking・I/O なし。`assert_no_alloc` の `rt_` prefix テストで証明。
- **R4 独立 Oracle**: PCM oracle は fixture spec を独立コードで再実装し
  （generator を呼ばない）、sine/sweep は libm+tolerance、他は Exact で
  比較。MIDI oracle は cue を独自 sort し play-through 期待列を生成
  （`TransportPlanBuilder`/`TransportScheduler`/`SpanSink` を呼ばない）。
- **R5 Case Schema v1**: run metadata は `environment_kind`・`level`・
  `requirement`/`result` 語彙が #627 契約（kebab-case）に従い、
  deterministic backend slice で実際に埋まる項目を保持する。
- **R6 test-only 境界**: crate は publish せず、production crate は
  本クレートに依存しない。path dep は `audio-core`/`media-core`/`rt` のみ、
  `assert_no_alloc` は dev-dep。
- **R7 公開 enum**: クレート外から到達可能な全公開 enum に
  `#[non_exhaustive]`。

## Examples

- **E1 PCM silence/impulse Exact**: `PcmFixture::new(48_000, 2,
  PcmPattern::Impulse{period_frames:4, amplitude:0.5})` の `fill` 結果を
  oracle `pcm_reference_block` と `pcm_bits_eq` で比較 → bit-identical。
- **E2 PCM sine tolerance**: `PcmPattern::Sine{freq_hz:440.0, amplitude:1.0}`
  の `fill` 結果を oracle（libm `f64::sin`）と `pcm_max_abs_diff <= 1e-5`
  で比較 → Pass。
- **E3 MIDI play-through**: `MidiScenario`（3 cues、NoteOn/CC/PitchBend）
  を `run_midi_trace` で実行し、oracle `midi_expected_playthrough` の
  期待 `MidiTraceEntry` 列と `frame`/`event`/`source_sequence` が一致。
- **E4 MIDI seek/loop**: `MidiTraceConfig.commands` に `Seek`/`SetLoop` を
  含めて `run_midi_trace` を実行し、`receipts`/`faults` が production
  `TransportScheduler` 契約通りに記録される（`AppliedReceipt.applied_frame`
  ・`ScheduleFault`）。
- **E5 VirtualClock determinism**: 同一 `VirtualClockConfig`（drift/jitter/
  offset 含む）で `advance_frames`+`sync` を 2 回実行し `tick()`/
  `monotonic_nanos()` が一致。pause 中は elapsed が進まず resume 後に
  反映されない。
- **E6 rt_ no-alloc**: `assert_no_alloc` 内で `clock.tick()`/
  `clock.monotonic_nanos()`/`fixture.fill(...)`/`run_callback`+`TraceSink`
  を実行 → allocation なし。
- **E7 run metadata**: `RunMetadata` が `environment_kind=deterministic-
  backend`・`level=OS-L1`・`scenario_seed`/`fixture_version`/`clock` を保持。

## Guarantees

- 同一 scenario seed + fixture version + clock 設定から決定論的 PCM/MIDI
  期待 trace を生成でき、run metadata で再現条件を保存できる
  （pure-Rust の決定性。3 OS 安定性の実走証拠は後続統合 slice）。
- clock-read / generator-fill / trace-capture 経路が `assert_no_alloc` の
  `rt_` テストで alloc/lock/wait-free であることを証明する。
- 独立 PCM 数値/byte Oracle・MIDI trace Oracle が独立 fixture/実装で
  検証され、production コード自身で期待結果を生成しない。
- test-only の公開範囲（crate README・`src/lib.rs` doc・spec/design）が
  明文化され、production crate からの依存がない。

## Non-goals

- scripted backend・fault injector・production pipeline 配線・CI lane
  統合・consumer oracle（AudioLens/Synvelis）・3 OS 実走証拠・Windows
  EXT-L3 — 後続 slice / 別 Issue。
- `TransportScheduler`/`SpanSink`/`MidiEvent`/`AudioBufferView` 等の
  production 契約自体の変更・拡張。
- MIDI clock・SysEx・System message・chase/stuck-note harness —
  production vocabulary/port の範囲確定後。
- transcendentals（`f32::sin`/`f64::sin` 等 libm）の bit-exact 3 OS
  byte 比較 — generator は libm 非依存で決定的、oracle は libm+tolerance。

## Risk

- oracle が generator と同一コードを共有すると期待値が自己参照になる
  → 独立実装・libm+tolerance・固定ベクトルで担保。
- `assert_no_alloc` が `Arc` clone 等を検出すると RT contract 違反 →
  `run_callback` 内で `Arc` refcount bump を避ける設計を確認。
- f64 poly sine と libm sin の差で byte 比較が落ちる → integer-domain
  pattern は Exact、transcendental は tolerance で分離。
