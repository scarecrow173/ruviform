---
id: "PLAN-034XBMgjefqA3NPS50TU74"
type: "plan"
status: "approved"
title: "Issue #628 実施計画 — deterministic core crate 新設・検証・docs chain"
created: "2026-11-14"
updated: "2026-11-14"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/specs/issue-628-deterministic-test-runtime.md"
    - "docs/designs/issue-628-deterministic-test-runtime-design.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-628-deterministic-test-runtime-plan.md"
      - type: "file"
        path: "docs/tasks/issue-628-deterministic-test-runtime.md"
      - type: "file"
        path: "docs/test-specs/issue-628-deterministic-test-runtime.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-628-deterministic-test-runtime.md"
  implemented-by:
    - "docs/tasks/issue-628-deterministic-test-runtime.md"
  depends-on:
    - "docs/specs/issue-628-deterministic-test-runtime.md"
    - "docs/designs/issue-628-deterministic-test-runtime-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-628-deterministic-test-runtime.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-628-deterministic-test-runtime.md"
    - "docs/designs/issue-628-deterministic-test-runtime-design.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-628-deterministic-test-runtime.md"
  verified-by:
    - "docs/test-specs/issue-628-deterministic-test-runtime.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 実施計画 — deterministic core crate 新設・検証・docs chain Implementation Plan

## Goal

`crates/ruviform-audio-test` test-only crate を新設し、VirtualClock・
Deterministic PCM fixtures・MIDI trace・独立 Oracle・Run metadata の
deterministic core を実装し、既存検証 lane で確認する。

## Scope

単一 implementation stream。Issue #628 の first slice（deterministic
core）のみ。scripted backend・fault injector・production 配線・CI lane・
consumer oracle・3 OS 実走は後続 slice へ hand-off（spec Out of Scope
参照）。`mise.toml`・workflow・`docs/development-environment.md`・
`POLICIES.md`・`AGENTS.md` は変更しない。

## Upstream Documents

- `docs/specs/issue-628-deterministic-test-runtime.md`（approved）
- `docs/designs/issue-627-virtual-first-verification-contract-design.md`
  （Case Schema v1 / `environment_kind` の契約元）
- `POLICIES.md`（OS-L1/GUI-L2/EXT-L3・仮想優先・証拠要件）

## Design Inputs

- `docs/designs/issue-628-deterministic-test-runtime-design.md`（approved）

## File Map

- `crates/ruviform-audio-test/Cargo.toml` — package 定義、path dep 3 件
  （audio-core/media-core/rt）+ dev-dep `assert_no_alloc`、依存理由コメント、
  `[lints] workspace = true`。
- `crates/ruviform-audio-test/README.md` — test-only 宣言・責務・依存・
  公開範囲。
- `crates/ruviform-audio-test/src/lib.rs` — crate-level doc（test-only・
  production 非依存の明文化）+ module 公開。
- `crates/ruviform-audio-test/src/clock.rs` — `VirtualClockConfig`/
  `JitterSpec`/`VirtualClock`/`VirtualClockDriver`。
- `crates/ruviform-audio-test/src/pcm.rs` — `PcmPattern`/`PcmFixture`/
  `PcmSegment`/`PcmScenario`/`PcmBlock` + `fill`/`block` 生成。
- `crates/ruviform-audio-test/src/midi.rs` — `MidiCue`/`MidiScenario`/
  `MidiTraceConfig`/`run_midi_trace`/`MidiTrace`/`MidiTraceEntry` +
  `TraceSink`。
- `crates/ruviform-audio-test/src/oracle.rs` — PCM 独立参照生成・比較、
  MIDI trace 期待列・不変条件。
- `crates/ruviform-audio-test/src/metadata.rs` — `RunMetadata` +
  `EnvironmentKind`/`VerificationLevel`/`Requirement`/`Result` 写像。
- `crates/ruviform-audio-test/tests/*.rs` — 統合テスト + `rt_` prefix
  `assert_no_alloc` RT contract テスト。
- `Cargo.toml`（root）— `members` に `"crates/ruviform-audio-test"` 追加のみ。
- `crates/README.md` — 利用側層テスト共有基盤の行追加。
- `Cargo.lock` — workspace 依存変更に伴う再生成。
- `docs/{specs,designs,plans,tasks,test-specs}/issue-628-*.md` — docs chain。
- `docs/impl/{ir,exp}/issue-628-deterministic-test-runtime.md` — impl 記録。
- `evidence/rf628/` — 検証記録。
- `docs/*/README.md`（specs/designs/plans/tasks/test-specs/impl-ir/impl-exp）
  — doc-maintenance / sync_impl_index による再生成。

## Implementation Sequence

- [x] T1: spec → design → plan → test-spec → task docs chain 作成・承認
- [ ] T2: crate 骨格（Cargo.toml/README/lib.rs）+ workspace 登録
- [ ] T3: `clock` module（VirtualClock/Driver、atomic read、deterministic
  tick/monotonic、drift/jitter/offset、pause/resume、deadline）
- [ ] T4: `pcm` module（pattern/fixture/scenario/block、fill alloc-free、
  libm 非依存決定的生成）
- [ ] T5: `midi` module（scenario→plan→scheduler→trace capture）
- [ ] T6: `oracle` module（PCM 独立参照・比較、MIDI 期待列・不変条件、
  固定ベクトル）
- [ ] T7: `metadata` module（RunMetadata、Case Schema v1 語彙）
- [ ] T8: unit/integration/`rt_` `assert_no_alloc` テスト
- [ ] T9: 検証 lane（fmt-check/check/lint/test/test-rt）実行・evidence 記録
- [ ] T10: impl-doc（ir/exp）+ docs index 再生成 + review-protocol
- [ ] T11: commit/push/PR/worker_done

## Dependencies and Risks

- `TransportScheduler`/`SpanSink`/`event_engine_frame` は production
  契約 — 変更せず利用。MIDI output port は存在しないため trace は
  `SpanSink` capture が記録点（design 記録済み）。
- f64 poly sine と libm sin の差 — generator は poly 固定、oracle は
  libm+tolerance。integer-domain pattern は Exact 比較。
- `assert_no_alloc` の `rt_` テストは `Arc` clone を RT 経路で避ける
  設計が必要（`plan()` は `&Arc` 返却で clone 不要）。
- `Cargo.lock`・docs index は生成物 — merge 時は再生成で解消。

## Task Breakdown

- `docs/tasks/issue-628-deterministic-test-runtime.md`（T1〜T11 を内包）。

## Verification

- [ ] `mise run fmt-check` — Pass
- [ ] `mise run check ruviform-audio-test` — Pass
- [ ] `mise run lint ruviform-audio-test` — Pass（clippy `-D warnings`）
- [ ] `mise run test ruviform-audio-test` — Pass
- [ ] `mise run test-rt` — `rt_` prefix テストが実際に収集・実行される
  （テスト名で確認）
- [ ] PCM oracle: silence/impulse/ramp/square/noise/constant は Exact、
  sine/sweep は libm+tolerance で独立検証
- [ ] MIDI oracle: play-through trace が期待列と一致、seek/loop で
  receipt/fault ledger が production 契約通りに記録
- [ ] run metadata が `environment_kind=deterministic-backend`・
  `level=OS-L1`・seed/fixture_version/clock を保持

## Open Questions / Gaps

- `assert_no_alloc` の `rt_` テストで `Arc` clone が検出されるかは実装時に
  確認（`run_callback` 内では plan は既に install 済みで `Arc` refcount
  bump は発生しない設計）。
- `PcmFixture::fill` の `frames`/`channels` 検証は `checked_mul` で
  overflow を防ぐ（32-bit 対応）。
- run metadata の JSON serialize は本 slice では構造体のみ（consumer 側
  or 後続で serde 検討）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker dispatch）
- Independent work streams: none
- User approval required before dispatch: review-protocol で代替
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `TransportScheduler` の `run_callback` RT contract（allocation-free・
  bounded）を `SpanSink` 実装で遵守できるか。
- `PcmFixture::fill` の alloc-free 性（`&mut [f32]` への書き込みのみ）。
- oracle の独立性（generator/oracle が同一コードを共有しないこと）。
