---
id: "SPEC-034Uwjri6KbgwOU8k9kjh0"
type: "spec"
status: "approved"
title: "Issue #309 PROBE-TIME slice — 複数Probe時間アライメント fixture 検証"
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
        path: "docs/specs/issue-309-probe-time-align-fixture.md"
      - type: "file"
        path: "docs/designs/issue-309-probe-time-align-fixture.md"
      - type: "file"
        path: "docs/test-specs/issue-309-probe-time-align-fixture.md"
      - type: "file"
        path: "docs/plans/issue-309-probe-time-align-fixture.md"
      - type: "file"
        path: "docs/tasks/issue-309-f1-align-fixture.md"
      - type: "file"
        path: "docs/impl/ir/issue-309-probe-time-align-fixture.md"
    modified:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/309"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md"
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
    - "docs/plans/issue-309-probe-time-context-and-alignment.md"
    - "docs/test-specs/issue-309-probe-time-alignment.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-309-probe-time-context-placement-and-alignment-contract.md"
  refines:
    - "docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #309 PROBE-TIME slice — 複数Probe時間アライメント fixture 検証

## What Should Be Built

Issue #309（複数 Probe の汎用 Frame/Clock/Epoch/Latency 整列可能性・
欠落比較契約）の **fixture 検証 slice**。先行 slice（PR #499、
`docs/plans/issue-309-probe-time-context-and-alignment.md`）で実装
済みの `ProbeClockMark`/`ProbeTimeContext`/`align_observation_points`/
`ObservationCoverage` 契約を、**実 `RtRuntime` + 実遅延 DSP + 実
audio fixture** の経路で検証する統合テストを
`crates/ruviform-audio/tests/` に追加する。公開 API・内部実装は
変更しない（契約が fixture で破綻した場合のみ最小修正を検討し、
その場合は本 spec を改訂する）。

検証対象:

1. **既知 latency 差を持つ 2+ Probe の align**: 実遅延を持つ
   processor 2 段の chain に 3 probe（latency 0 / La / La+Lb）を
   Monitor/Capture で観測し、drain した実 record から
   `ObservationSpan`/`ObservationCoverage` を再構成して
   `align_observation_points` を呼ぶ。期待値は fixture 信号から
   独立に計算し、offset・overlap interval・per-block Peak/RMS・
   PCM 波形の一致を assert する。
2. **Incomparable/Conditional の実 record 経路での明示**: epoch
   bump（Seek/Loop/Device Restart の共通表現）・generation 再構成
   （republish+rebind）・format/rate/latency/precision の各境界を、
   合成 span ではなく実 record 由来の span で検証する。
3. **測定無音と loss/unobserved の区別**: 無音 fixture 区間は
   `Valid` coverage + Peak=0/RMS=0 として比較対象に残り、
   Probe OFF 区間・`LostBlocks`（chunk pool 枯渇）・
   `LostRecords`（telemetry 溢れ）は coverage class として分類
   され、比較 mask（`excluded_intervals`/Valid 区間）から除外
   される — loss のある波形を連続 PCM として比較しない。
4. **RT/nonRT 境界**: probe 稼働中の `process` が alloc 0 を維持
   すること（既存 `assert_no_alloc` lane と同一入口）、
   `align_observation_points`/coverage 再構成が非 RT の bounded
   計算であること、f32 RT 観測値を f64 native 精度と混同しない
   こと（`MixedPrecision` → `Conditional`）を fixture で確認する。

## Why Now

先行 slice は契約型と unit 級分岐・mark 配線を実装したが、
「実 audio fixture で expected offset/interval/Peak/RMS/gaps が
一致する」という Issue #309 受入条件の中核は未検証のまま残って
いる。Synvelis SF-04 #107/SF-06 #109 が消費する generic
comparison 契約の Ruviform 側正本として、consumer 接続（cross
gate #219/#294）前に fixture 層の証拠を確定させる必要がある。

## Users and Value

- **Synvelis（SF-04 #107 / SF-06 #109）**: DAW Probe 比較の前段
  となる generic align 契約が、実 runtime の record stream に対し
  期待どおり offset/interval/level/gap を返すことの Ruviform 側
  実測証拠。
- **Ruviform #219/#294 cross-consumer gate**: consumer-verify が
  参照する Test Command・fixture・判断基準の正本。
- **後続の consumer 接続 slice**: `ObservationSpan` を実 record
  から再構成する手順の実例（本 slice の test helper が雛形）。

## Scope

### In Scope

- `crates/ruviform-audio/tests/typed_probe_time_align.rs`（新規）
  — 実 `GraphRuntime`/`RtRuntime` + 実遅延 `GraphProcessor`
  fixture による複数 Probe align・Incomparable/Conditional・
  coverage/loss mask の検証。
- 本 doc チェーン（spec/design/test-spec/plan/task/impl record）
  と各 `docs/*/README.md` index の更新。

### Out of Scope

- `crates/ruviform-audio/src/` の公開 API・内部実装の変更
  （fixture が契約破綻を検出した場合の最小修正は別途判断）。
- `ruviform-io-*`・`ruviform-ui*`・`ruviform-observability`・
  `ruviform-plugin-host`（他 dispatch 稼働中・consumer 側）。
- `.github/`・`mise.toml` の変更。
- 実デバイス/実 plugin 経路・OS 実機差・Synvelis/AudioLens 側の
  消費接続・実測（cross-consumer gate #219/#294 の責務）。
- `EngineFrame` anchor の実生成・device clock 配線（design の
  Deferred Concerns どおり後続 Issue）。
- PCM resample・PCM 相関・sound causality 推論（Issue の non-goal
  を維持）。

## Requirements

- fixture の遅延 processor は `latency()` の宣言値と実遅延を
  一致させる（宣言だけの嘘 latency で PCM 一致を偽装しない）。
- 期待値（offset・overlap・Peak/RMS・gap 区間）は fixture 信号と
  probe 契約から独立に計算し、実装の出力を期待値として使わない。
- Peak/RMS の照合は f64 で独立計算した期待値との tolerance 比較
  と、probe 間の bit 一致（純粋遅延は f32 copy で bit-exact）の
  両方で行う。
- loss/unobserved 区間は「比較対象外」として明示的に assert し、
  その区間の PCM/metric を連続データとして参照しない。
- 全検証は `mise run` の canonical タスク経由で実行し、結果を
  impl record に正直に記録する（未実施は NotRun）。

## Acceptance Criteria

- [ ] 実 audio fixture で latency 既知の 2+ Probe（3 点:
      latency 0 / La / La+Lb）が `Aligned{basis: SharedCursor}`
      を返し、offset が latency 差と一致、overlap interval が
      期待区間と一致する。
- [ ] 同一 fixture の MetricRecord（Monitor）と CaptureChunk
      （Capture）の Peak/RMS/PCM が、offset 補正後の対応区間で
      期待値および probe 間で一致する。
- [ ] epoch bump 後の span 対が `Incomparable{EpochMismatch}`、
      generation 再構成を跨ぐ span が `Incomparable{GenerationSpan}`
      （または policy 緩和時 `Conditional{CrossGeneration}`）を返す。
- [ ] Probe OFF 区間・`LostBlocks`・`LostRecords` が coverage
      class として分類され `Conditional{PartialCoverage}` の
      `excluded_intervals` に現れ、測定無音区間は `Valid` のまま
      比較対象に残る。
- [ ] latency `Unknown` が既定 policy で `Incomparable`、明示
      bound policy でのみ `Conditional{LatencyBounded}` になる
      ことを実 record 由来 span で確認する。
- [ ] probe 稼働中 `process` の alloc 0（`assert_no_alloc`）と
      既存 typed_probe/typed_probe_time 試験の非退行を確認する。
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt`
      / `test-msrv` が pass。

## Deferred Design Concerns

- `ObservationSpan` を drain 済み record から再構成する consumer
  向け helper の crate 内配置 | consumer（Synvelis）が具体的な
  消費形を確定してから | cross-consumer gate #219/#294 | 現時点
  では test helper のみで十分、公開 API の早期確定を避ける。
- Seek/Loop/Device Restart の区別 | Ruviform の cursor series は
  epoch bump で不連続を表現し、原因の区別は consumer（DAW
  transport）の責務 | Synvelis SF-04/SF-06 | 本 slice は epoch
  bump を代表機構として検証する。
- 実デバイス clock anchor・`EngineFrame` basis の実データ検証 |
  anchor 生成は render/OBS bridge の後続 Issue | 別 Issue | 本
  slice は `SharedCursor` basis のみを実 fixture で検証する。

## Sources and Relations

- Issue #309 本文（受入条件の正本）。
- 先行 slice 文書チェーン: `docs/{specs,designs,plans,tasks,
  test-specs,impl/ir}/issue-309-probe-time-*.md`（本 slice は
  `refines` 関係）。
- `docs/adr/issue-309-probe-time-context-placement-and-alignment-
  contract.md` — mark 注入・三値契約の決定記録。
- `crates/ruviform-audio/tests/typed_probe_time.rs` — 先行 slice
  の unit/配線試験（本 slice はその fixture 拡張）。
