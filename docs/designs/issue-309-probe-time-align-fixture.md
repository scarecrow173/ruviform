---
id: "DESIGN-034UwjzYYEPPjIvuXPl28f"
type: "design"
status: "approved"
title: "Issue #309 PROBE-TIME slice — 複数Probeアライメント fixture 設計"
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
    - "docs/specs/issue-309-probe-time-align-fixture.md"
    - "docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md"
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-309-probe-time-context-placement-and-alignment-contract.md"
  refines:
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-309-probe-time-align-fixture.md"
    - "docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #309 PROBE-TIME slice — 複数Probeアライメント fixture 設計

## Context

先行 slice（`docs/designs/issue-309-probe-time-context-and-alignment.md`、
D1–D8 確定済み）で `typed/probe/time.rs` に三値整列契約・
`ProbeClockMark`・`ProbeTimeContext`・`ObservationCoverage` が
実装され、`tests/typed_probe_time.rs` が合成 span と mark 配線を
検証済み。本 slice は契約を変更せず、**drain した実 record stream
から span を再構成し、実 audio fixture の期待値と照合する**
検証層を追加する。設計上の新規判断は fixture 構成と期待値の
独立計算方法に限定される。

## Scope

### In Scope

- `crates/ruviform-audio/tests/typed_probe_time_align.rs` —
  fixture graph・実遅延 DSP・drain→span 再構成 helper・期待値
  計算・各検証ケース。
- 本 doc チェーンと index 更新。

### Out of Scope

- `src/` 側の API 追加・変更（span 再構成 helper は test 内に
  閉じる — consumer 接続時に公開形を再検討）。
- `EngineFrame`/anchor basis の実 fixture（anchor 生成は後続
  Issue）、実デバイス・実 plugin・他 OS 実機。

## Components and Boundaries

### Fixture graph

`Source → DelayDsp(La) → DelayDsp(Lb) → Sink` の chain
（`tests/common` の builder 規約に従い `port_probe` で probe 点を
declare）。`La = Lb = FRAMES(64)` とし、全 latency 差が block
境界に揃う（offset = 64 / 128 frames）。3 probe:

| probe | point | TapKind | effective_latency |
| --- | --- | --- | --- |
| p0 | d1 input | NodeInput | 0 |
| p1 | d1 output | NodeOutput | La = 64 |
| p2 | sink input | GraphOutput | La+Lb = 128 |

`DelayDsp` は `latency()` が `D` を返し、かつ実際に入力を `D`
frame 遅延させる processor（channel 毎の固定長 delay line、
`copy_within`/`copy_from_slice` のみで RT no-alloc を維持）。
宣言 latency と実遅延の一致により、`RecordMeta.effective_latency`
と観測 PCM の時間関係が真になる。

### Fixture signal

block `k`（cursor `[64k, 64k+64)`）毎に決定的な channel 別信号を
feed し、全入力を `Vec<Vec<f32>>`（channel × absolute frame）
として保持する。block 5–6 は全 zero を feed し「測定無音」区間
を作る（Valid coverage + Peak/RMS=0 であることを後で区別する）。

### Drain → span 再構成（test helper）

`collector.drain` を 1 回の観測区間につき 1 回呼び、probe 毎に
`Vec<OwnedEv>`（Metric/Status/Captured+PCM copy）を集める。
`lost_records` は drain 前後の `collector.counts()` delta を使う。

- `ObservationCoverage::reconstruct(probe, lost_delta, events)` —
  `CoverageEvent::Data(meta)`/`Status(status)` に写像。
- `ObservationSpan` — `interval` = data record の `[min pos,
  max end)`、`generation_range` = 全 record の generation min/max、
  `format` = record の channels/sample_rate + spec の layout、
  `latency` = `LatencyKnowledge::Known{effective_latency}`、
  `clock_mark` = record の mark、`precision` = `F32Rt`、
  `context` = `None`（SharedCursor basis は context 不要）。

### 期待値の独立計算

probe `P`（latency `L`）の cursor block `p` における観測内容は
入力信号の `[p-L, p+64-L)`（負 index は delay line の 0 補完）。
fixture 入力 buffer から f64 で Peak/RMS を独立計算し、
`ChannelLevel`（f32）とは相対 tolerance で比較する。probe 間の
対応は `q = p + (L_b - L_a)`（= alignment の `offset`）で、
純粋遅延は f32 copy のため bit-exact を期待する。

## Data and Control Flow

1. `rig()` で compile/publish → `pair()` → `set_probe_deck`。
2. `broker.install` × 3 → `wire_clock_mark` → `capture`（必要時）
   → block 毎に `feed` + `rt.process(FRAMES)`。
3. 区間終端で `collector.drain` → owned event 列を probe 別に集約
   → coverage/span を再構成。
4. `align_observation_points(a, b, &AlignPolicy::default())` を呼び、
   `Aligned`/`Conditional`/`Incomparable` の basis・offset・
   overlap・excluded_intervals・reason を assert。
5. MetricRecord/CaptureChunk の level/PCM を期待値・相手 probe と
   照合。coverage class（Off/LostBlocks/LostRecords/Valid）と
   `excluded_intervals` の対応を assert。

## Decisions

- **D1: 実遅延 DSP を新設（嘘 latency を使わない）** —
  `MockDsp` は `latency()` を宣言するが PCM を遅延させないため、
  offset 補正後の PCM 一致を検証できない。宣言と実遅延が一致する
  `DelayDsp` を test file 内に置き、fixture の時間関係を真にする。
- **D2: span 再構成 helper は test 内に閉じる** — consumer の
  消費形が未確定の段階で公開 API を増やさない（spec Deferred
  Concerns）。helper は drain→span の手順を実演する雛形になる。
- **D3: epoch/generation の代表機構は `bump_epoch`/republish** —
  Seek/Loop/Device Restart は全て「宣言された時計不連続」として
  epoch bump に写像されるため、機構としての epoch bump と
  generation 境界を fixture で検証する。原因の区別は consumer
  責務であり本層の契約外。
- **D4: loss は実経路で誘発する** — Probe OFF は `set_mode`、
  `LostBlocks` は `pool_chunks` を probe 数に揃えて drain を遅らせる、
  `LostRecords` は telemetry capacity を絞って誘発する。合成
  status の捏造は coverage 再構成の unit 試験が既に担うため、
  fixture では実 RT 経路の発生を優先する。
- **D5: `MixedPrecision` は F64Native 側を手組み span で表現** —
  f64 native 観測源は AudioLens 側の責務であり Ruviform probe は
  常に f32。精度混在の Conditional 判定は、実 record 由来の
  F32Rt span と精度だけ差し替えた span で検証する。

## Risks

- `DelayDsp` の delay line 実装ミスで「latency 一致」が崩れ、
  PCM 不一致として顕在化する — 独立期待値計算と bit-exact 照合で
  検出可能（それが fixture の目的）。
- `sample_position` は block 単位のため sub-block の offset 精度は
  持たない — `La/Lb` を block 倍数に揃え、契約の frame 粒度と
  fixture 粒度を一致させる。
- telemetry/chunk channel の容量設定を誤ると意図しない
  `LostRecords`/`LostBlocks` が混入する — loss を期待する試験
  以外は既定容量 + 毎 block drain で決定的にする。
