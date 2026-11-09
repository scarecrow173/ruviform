---
id: "TASK-034UwjBX1xcKeAC33krCUQ"
type: "task"
status: "todo"
title: "F1 PROBE-TIME アライメント fixture 実装"
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
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-309-probe-time-align-fixture.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-309-probe-time-align-fixture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-309-probe-time-align-fixture.md"
  references: []
  defers: []
  deferred-by: []
---

# F1 PROBE-TIME アライメント fixture 実装

## Work

plan `docs/plans/issue-309-probe-time-align-fixture.md` の File Map
に従い、`crates/ruviform-audio/tests/typed_probe_time_align.rs`
を新規実装する。実 `GraphRuntime`/`RtRuntime` + 実遅延
`DelayDsp`（`latency()` 宣言と実遅延が一致）+ 決定的 audio
fixture で、drain した実 record から `ObservationSpan`/
`ObservationCoverage` を再構成し `align_observation_points` の
結果を検証する。公開 API・`src/` 実装は変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-309-probe-time-align-fixture.md`
- `docs/specs/issue-309-probe-time-align-fixture.md`
- `docs/designs/issue-309-probe-time-align-fixture.md`
- `docs/test-specs/issue-309-probe-time-align-fixture.md`
- `docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md`
- `docs/designs/issue-309-probe-time-context-and-alignment.md`

## Dependencies

- 先行 slice（PR #499）の `typed/probe/time.rs` 契約・
  `tests/typed_probe_time.rs` の helper 規約・`tests/common`
  fixture・Issue #308 rebind 機構。外部 blocker なし。

## Affected Files

- `crates/ruviform-audio/tests/typed_probe_time_align.rs`（新規）
- `docs/impl/ir/issue-309-probe-time-align-fixture.md`（実装 record）

## Done When

- [x] latency 0/La/La+Lb の 3 probe が `Aligned{SharedCursor}`・
      offset=latency 差・期待 overlap を返す（Monitor/Capture
      両モード）
- [x] MetricRecord/CaptureChunk の Peak/RMS/PCM が offset 補正後の
      対応区間で fixture 独立期待値・probe 間 bit-exact と一致
- [x] epoch bump → `Incomparable{EpochMismatch}`、generation
      再構成跨ぎ → `Incomparable{GenerationSpan}`（緩和 policy で
      `Conditional{CrossGeneration}`）
- [x] Probe OFF→`Off`・`LostBlocks`→`LostBlocks`・telemetry
      overflow→`Unknown` が coverage class + `excluded_intervals`
      に現れ、測定無音は `Valid` に留まる。
      `LostRecords`/`Unobserved`/`GenerationBoundary` class は
      実 runtime からは status 非送出または非決定的なため
      `tests/typed_probe_time.rs` の contract unit 層で検証済み
      （fixture は実経路の `Unknown` downgrade を検証）
- [x] latency `Unknown` が既定で `Incomparable`、bound policy で
      `Conditional{LatencyBounded}`、format/rate/precision 境界が
      明示される
- [x] probe 稼働中 `process` の `assert_no_alloc` alloc 0
      （`rt_capture_process_allocates_nothing` が `test-rt` lane
      で実行）
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全緑
      （`test-msrv` は該当 task 非存在のため NotRun）

## Verification

- [x] `mise run fmt-check` exit 0
- [x] `mise run check` exit 0
- [x] `mise run lint` exit 0
- [x] `mise run test` exit 0（新規 test file 全 pass・既存
      typed_probe 系非退行）
- [x] `mise run test-rt` exit 0
- [ ] `mise run test-msrv` — **NotRun**: `mise.toml` に
      `test-msrv` task は存在しない（`test-ofx-msrv`/
      `test-ui-test-msrv` のみ・本 slice 対象外）
- [ ] `git merge origin/main` 後の再検証

## 内容

`tests/typed_probe_time_align.rs` に以下を実装する（design D1–D5
に従う）:

- `DelayDsp`: channel 毎の固定長 delay line で実遅延を行う
  `GraphProcessor`。`latency()` は宣言値、`process` は
  `copy_within`/`copy_from_slice` のみ（RT no-alloc）。
- fixture graph `Source → DelayDsp(64) → DelayDsp(64) → Sink`
  と 3 probe（d1 input=NodeInput/latency 0、d1 output=
  NodeOutput/64、sink input=GraphOutput/128）。
- 決定的 channel 別 fixture 信号（block 単位生成・全入力を
  absolute frame index で保持・block 5–6 は無音）。
- drain→`CoverageEvent`/`OwnedEv` 集約→`ObservationCoverage::
  reconstruct`→`ObservationSpan` 構成の test helper。
- 期待値の独立計算（f64 peak/rms・offset 対応 PCM 照合）。
- 検証ケース: align offset/overlap・metric/PCM 一致・
  epoch/generation/format/rate/latency/precision 境界・
  Off/LostBlocks/LostRecords/無音の区別・`assert_no_alloc`。

## 完了条件

- 新規 test file が `cargo test -p ruviform-audio --test
  typed_probe_time_align` で全 pass（`mise run test` 内包）。
- impl record に実行した検証・結果・NotRun を記録する。
