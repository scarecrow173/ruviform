---
id: "IMPL-034Ukaiqpkd3tCCzSFomK7"
type: "impl"
status: "completed"
title: "Issue #309 — PROBE-TIME時間コンテキスト・整列契約の実装"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/309"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio/src/typed/probe/time.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/typed_probe_time.rs"
      - type: "file"
        path: "docs/impl/ir/issue-309-probe-time-context-and-alignment.md"
    modified:
      - type: "file"
        path: "crates/ruviform-audio/src/typed/probe.rs"
      - type: "file"
        path: "crates/ruviform-audio/src/typed/runtime.rs"
      - type: "file"
        path: "docs/tasks/issue-309-probe-time-context-and-alignment.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-309-probe-time-context-and-alignment.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-309-probe-time-context-and-alignment.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md"
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
    - "docs/adr/issue-309-probe-time-context-placement-and-alignment-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-309-probe-time-alignment.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #309 — PROBE-TIME時間コンテキスト・整列契約の実装

## Summary

`typed::probe` に probe 観測の時計 identity を additive に注入し、複数 probe 観測の Frame/Clock/Epoch/Latency 整列可能性を型で判定する非 RT API を実装した。`ProbeClockMark`（`ClockDomainId`+`ClockEpochId` の copyable identity、`UNKNOWN` sentinel は alignment 不可）を `RecordMeta.clock`/`StatusRecord.clock` に追加し、mint は `ProbeBroker`/`pair()` のみが行い、RT deck は `ProbeCommand::SetClockMark` で copy を受け取って全 record/status に stamp する（`TapCtx.mark` 経由・command drain 後に refresh）。非 RT の `ProbeTimeContext` registry が domain/epoch/runtime_instance/sample rate/provenance/anchors/generation 窓を保持し、`wire_clock_mark`/`bump_epoch` で mark を発行する。`typed/probe/time.rs` に `ObservationSpan`/`AlignPolicy`/`Alignment`（`Aligned`/`Conditional`/`Incomparable`）/`align_observation_points`/`ObservationCoverage` を実装し、UNKNOWN clock・domain/epoch 不一致・rate 不一致・latency 不明・generation 跨ぎ・coverage 断絶を fail-closed に `Incomparable` へ倒す。resample・PCM 相関・信号内容推論は一切行わない。clock 語彙・frame 変換は media-core の `ClockDomainId`/`ClockEpochId`/`ClockAnchor`/`EngineFrame`/`TimeBase`/`map_stamp_to_frame` を再利用し、新しい clock 正本は作っていない。

## Context

- 実施タスク: `docs/tasks/issue-309-probe-time-context-and-alignment.md`
- 上流: `docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md`、`docs/designs/issue-309-probe-time-context-and-alignment.md`（D1–D8: hybrid mark 注入・media-core 語彙再利用・非 RT mint+`SetClockMark`・三値 alignment・coverage 再構成・#308 境界接続）、`docs/plans/issue-309-probe-time-context-and-alignment.md`、`docs/adr/issue-309-probe-time-context-placement-and-alignment-contract.md`、`docs/test-specs/issue-309-probe-time-alignment.md`
- 前提: Issue #185 `typed::probe` 基盤・Issue #308 staged rebind（`RebindReceipt`/`CapturePolicy`/generation 境界）が本 worktree に存在。media-core の clock 型と `map_stamp_to_frame`、audio-core 依存を再利用。

## Implementation

### Changes

- `crates/ruviform-audio/src/typed/probe/time.rs`（新規）— 公開 API の正本。`probe.rs` から re-export。
  - `ProbeClockMark { domain: ClockDomainId, epoch: ClockEpochId }` — copyable identity。`UNKNOWN` は `ClockDomainId::UNKNOWN` sentinel で `is_unknown()` 判定。UNKNOWN 同士でも alignment 不可。
  - `ProbeTimeContext` — 非 RT registry。domain/epoch/runtime_instance/sample_rate/provenance/`ClockAnchor` 受け皿/`GenerationRange` 窓を保持。`mark()`・`frame_base()`・`cursor_to_engine_frame()`（media-core `map_stamp_to_frame` 経由）を提供。
  - `LatencyKnowledge`（`Unknown`/`Frames(u64)`/`Bounded{..}` 相当の知識表現）、`GenerationRange`（`crosses()` で generation 跨ぎ検出）、`ProbePrecision`、`ObservationFormat`、`FrameInterval`（`intersection` 付き）、`CoverageClass`/`CoverageInterval`/`CoverageEvent`/`ObservationCoverage::reconstruct`（drain 済み event から sorted disjoint な区間分類を再構成。測定無音は `Valid`、未観測 gap は `Unobserved`、telemetry loss は `Unknown` へ降格）。
  - `AlignmentBasis`（`SharedCursor`/`EngineFrame`）、`AlignmentCondition`、`IncomparableReason`（全 variant `#[non_exhaustive]`）、`AlignPolicy`（`struct_excessive_bools` を reason 付き `#[expect]` で正当化した flat policy bag）、`ObservationSpan<'a>`、`Alignment`（`Aligned{basis,offset,error_budget}`/`Conditional{..}`/`Incomparable{reason}`）。
  - `align_observation_points` — `AlignEval`/`latency_term`/`evidence_conditions`/`coverage_phase` に分割した三値判定。SharedCursor は latency 差を A→B offset に、EngineFrame は両側 anchor 互換時のみ `map_stamp_to_frame` で共通座標へ写像。rate 差は `allow_rate_conversion` 明示時のみ `TimeBase`/`MediaDuration`/`Rounding::NearestEven` で変換し residue を `error_budget` に加算。`max_error_frames` 超過・generation 跨ぎ・hard coverage 断絶は `Incomparable`。
- `crates/ruviform-audio/src/typed/probe.rs`
  - `mod time` + 全公開型の re-export。`RecordMeta.clock`・`StatusRecord.clock` を additive 追加。
  - `ProbeCommand::SetClockMark { mark }`（lifetime command）。`ProbeDeck.current_mark`（初期値 `UNKNOWN`）を drain 時に更新し、全 emit record/status に stamp。
  - `ProbeBroker.time_context`/`time_context_mut`/`wire_clock_mark`/`bump_epoch` を追加。`pair()` が probe 専用 namespace（`0x5256_4650_0000_0000`、media-core well-known ID より上位で衝突回避）から domain と runtime_instance を atomic に mint。`bump_epoch` は epoch を進めて新 mark を queue し、channel full 時は broker/deck が一時的に diverge し得るため caller の retry を要求。
- `crates/ruviform-audio/src/typed/runtime.rs` — `TapCtx.mark` を additive 追加し、command drain 後に `deck.current_mark()` で refresh（`frame_cursor` は不変）。
- `crates/ruviform-audio/tests/typed_probe_time.rs`（新規・23 tests）— unit 級の alignment/coverage 分岐と、実 runtime 経由の mark stamp・PCM bit-identical・RT no-alloc を検証。

### Decisions During Implementation

- **`UNKNOWN` を `Option` ではなく sentinel domain で表現** — `ProbeClockMark` を常に値として持たせ、`is_unknown()` で判定する方が `RecordMeta` の additive 拡張として自然で、全 emit 経路で `Option` 分岐が不要になる。`ClockDomainId::UNKNOWN` は alignment の入口で必ず `Incomparable` に倒す。
- **mint は broker のみ・RT deck は copy のみ** — `pair()` で domain/runtime_instance を atomic 採番し、`SetClockMark` は SPSC command として非 RT→RT に一方向送信。RT thread は mint/registry/align に一切触れない（`set_clock_mark_is_alloc_free_on_rt_path` で検証）。
- **`align_observation_points` は phase 分割** — entry の fail-closed 判定 → basis 解決（`prefer_engine_frame` policy 考慮）→ format/rate 検査 → `latency_term` → `evidence_conditions`（generation/provenance demotion）→ `coverage_phase`（view 写像・hard discontinuity・joint valid coverage・excluded intervals）→ error budget 判定 → `Aligned`/`Conditional` の順で、各段が `IncomparableReason` を返す構造にした。
- **coverage は event sweep で再構成** — `ObservationCoverage::reconstruct` は `CoverageEvent` を position で sort し、data/status candidate を priority sweep で解決。status は data に優先し、persistent state を引き継ぎ、未説明 gap を `Unobserved`、`lost_records>0` を `Unknown` へ降格する。
- **rate 変換は明示 policy のみ** — `allow_rate_conversion: false`（既定）では rate 不一致を `Incomparable{RateMismatch}`、true では `NearestEven` 変換 + residue を `error_budget` に計上し `max_error_frames` で fail-closed。

## Related Experiments

### Adopted

- なし（新規 experiment log なし。設計で選定済みの hybrid mark 注入・非 RT registry・三値契約をそのまま実装）。

### Rejected

- consumer 側 clock 推測・自動 resample・PCM 相関 — 設計の non-goal どおり実装しない。
- `EngineFrame` anchor 生成 — render/OBS bridge の責務（Deferred Concerns）として受け皿のみ実装。

## Validation

- OS: Windows 11 x64（Git Bash） / toolchain: `cargo 1.99.0`（mise pin）
- `git fetch origin main && git merge origin/main` — clean merge 後に全 lane を再検証
- `mise run fmt-check` — exit 0
- `mise run check` — exit 0（`cargo check --workspace --all-targets`）
- `mise run lint` — exit 0（`cargo clippy --workspace --all-targets -- -D warnings`）
- `mise run test` — exit 0（`cargo test --workspace`・181 test groups・0 failed。`tests/typed_probe_time.rs` 23/23 pass、`tests/typed_probe.rs` 既存 18 件非退行）
- `mise run test-rt` — exit 0（RT フィルタ lane・`rt_probe_process_allocates_nothing`・`set_clock_mark_is_alloc_free_on_rt_path`・`rt_typed_process_no_alloc` を含む RT no-alloc 非回帰を確認）

NotRun（未実測・明記）: 実デバイス/実 plugin 経路での probe 観測、consumer repo（synvelis #107/#109・audio-lens）側の alignment 利用（本 Issue のスコープ外・Issue は Open 継続前提）、`EngineFrame` anchor の実生成（render/OBS bridge の後続 Issue）。

## Consumer Impact Materials

- **synvelis**: **no direct impact（将来の受け皿）** — `align_observation_points`/`ObservationCoverage` は consumer が複数 probe 観測を比較する際の型付き判定 API。SF-04 #107/SF-06 #109 が必要とする「整列可能か・どの basis か・error_budget いくらか・どの区間が比較可能か」を非 RT 層で提供する土台になるが、consumer 側接続は未実装。
- **audio-lens**: **no impact** — probe typed runtime の consumer 利用は現時点で未接続。

## Risks

- `RecordMeta`/`StatusRecord` への `clock` 追加で telemetry payload が増加 — `ProbeClockMark` は 16 bytes の copyable 型で additive。帯域影響の実測は consumer 接続時に行う（plan Risks どおり実装時の計測は consumer 側課題として残す）。
- `bump_epoch` が channel full で broker/deck mark を一時 diverge させ得る — `wire_clock_mark` の retry を呼出側に要求する契約として文書化済み。
- live device では device timestamp 未配線のため epoch が `UNKNOWN` のまま残り得る — `Incomparable{UnknownClock}` で保守的に倒れる設計どおりの挙動。

## Follow-ups

- Issue #309 は consumer 受入（synvelis #107/#109）を含むため Open 継続 — 残件: consumer 側接続・実測証拠。
- device clock anchor 供給（`OutputCallbackInfo`/`InputCallbackInfo` 配線）・`EngineFrame` anchor 生成は別 Issue（design の Deferred Concerns）。
