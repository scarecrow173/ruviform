---
id: "IMPL-034Ux2ZcFoBhisTdYZmYtY"
type: "impl"
status: "in-progress"
title: "Issue #309 — PROBE-TIME複数Probe時間アライメントfixture検証"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/309"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio/tests/typed_probe_time_align.rs"
      - type: "file"
        path: "docs/impl/ir/issue-309-probe-time-align-fixture.md"
    modified:
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/tasks/issue-309-f1-align-fixture.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-309-f1-align-fixture.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-309-probe-time-align-fixture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-309-probe-time-align-fixture.md"
    - "docs/designs/issue-309-probe-time-align-fixture.md"
    - "docs/test-specs/issue-309-probe-time-align-fixture.md"
    - "docs/impl/ir/issue-309-probe-time-context-and-alignment.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-309-probe-time-align-fixture.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-309-probe-time-align-fixture.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #309 — PROBE-TIME複数Probe時間アライメントfixture検証

## Summary

PR #499 で着地済みの PROBE-TIME 契約（`ProbeClockMark` /
`ProbeTimeContext` / `ObservationSpan` / `ObservationCoverage` /
`align_observation_points`）を、実 `GraphRuntime` + `RtRuntime` +
実遅延 DSP + 決定的 audio fixture 上で end-to-end 検証する
fixture slice を追加した。`src/` 実装は変更していない — 契約は
fixture が証明するまで正しいと仮定せず、実 record stream から
span/coverage を再構成して契約の全分岐を実測した。

## Context

- task: `docs/tasks/issue-309-f1-align-fixture.md`
- plan: `docs/plans/issue-309-probe-time-align-fixture.md`
- spec: `docs/specs/issue-309-probe-time-align-fixture.md`
- design: `docs/designs/issue-309-probe-time-align-fixture.md`
- test-spec: `docs/test-specs/issue-309-probe-time-align-fixture.md`
- 先行 slice: `docs/impl/ir/issue-309-probe-time-context-and-alignment.md`
  （PR #499 / `0f5b5737` — 契約本体と `tests/typed_probe_time.rs`）

## Implementation

### Changes

- `crates/ruviform-audio/tests/typed_probe_time_align.rs`（新規、
  11 test）:
  - `DelayDsp`: channel 毎の固定長 delay line を持つ test-local
    `GraphProcessor`。`latency()` の宣言値と `process` の実遅延が
    一致し、既存 mock DSP（宣言のみで PCM を遅延しない）の代わりに
    fixture の ground truth とする。
  - fixture graph `Source → DelayDsp(64) → DelayDsp(64) → Sink`
    に latency 0 / 64 / 128 の 3 probe を装着
    （d1 input / d1 output / sink input）。
  - 決定的 stereo fixture（channel 別周波数・block 毎振幅、
    block 5–6 は無音）を absolute frame index で生成し、
    Peak/RMS を f64 で独立再計算して f32 実測と照合。
  - drain した `MetricRecord`/`StatusRecord`/`CaptureChunk` を
    `OwnedEv` に複写し、`ObservationCoverage::reconstruct` +
    record meta から `ObservationSpan` を再構成（test 内 helper、
    公開 API 追加なし）。
  - `rt_capture_process_allocates_nothing`: `rt_` prefix で
    `test-rt` lane に選択される RT no-alloc 実測（capture 3 probe
    稼働中の全 `process` を `assert_no_alloc` で実行）。
- docs chain: spec/design/test-spec/plan/task + 各 README index +
  本 IR record。

### Decisions During Implementation

- **fixture signal の無音判定は latency 補正後の観測窓で行う**:
  cursor block index の `is_silent_block` では delay 分だけずれて
  無音でない区間を無音と誤判定したため、「独立 f64 期待 peak が
  厳密に 0」の区間を測定無音と定義し直した。pre-roll の delay
  line 出力 0 も測定無音（Valid coverage）として正しく扱われる。
- **telemetry loss fixture は capacity 6**: block 0 は
  `drain_commands` の `Installed` status 3 件 + metric 3 件 =
  6 record を吐く。capacity 4 では status のみ残り metric が全損
  する probe が生じ coverage が空になったため、block 0 がちょうど
  収まる 6 とし、block 1–2 の metric overflow で
  `lost_records > 0` を確定的に発生させた。
- **PCM は bit-exact、level は tolerance**: 純粋 delay なので
  offset 補正後の PCM は `to_bits` で完全一致を要求。Peak/RMS は
  f32 累積（`ChannelLevel::measure`）と f64 独立期待の丸め差を
  許容する。
- **`F64Native` は手動 span 調整**: Ruviform probe は f32 のみ
  emit するため、mixed-precision 境界は実 span の `precision`
  field を差し替えた単一 field mutation で検証（AudioLens 側の
  native f64 source は consumer 責務）。

## Related Experiments

### Adopted

<!-- なし — approach は既知（既存契約への fixture 追加）のため exp なし -->

### Rejected

<!-- なし -->

## Validation

<!-- 実行結果を完了時に記録 -->

## Risks

- fixture は `SharedCursor` basis のみを対象とし、`EngineFrame`
  anchor evidence は後続 slice（design の scope 外宣言どおり）。
- `assert_no_alloc` は Windows dev profile での実測であり、
  他 OS/CI lane の RT 保証は既存 `rt_*` test 群と同じ範囲。

## Follow-ups

- 全 OS 実機検証・Synvelis/AudioLens cross-consumer 接続は
  Issue #309 の残件（cross gate #219/#294 で記録）。
