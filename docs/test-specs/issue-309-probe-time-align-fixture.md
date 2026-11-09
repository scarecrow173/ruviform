---
id: "TSPEC-034UwjAIH8nOcoNlsyjent"
type: "test-spec"
status: "approved"
title: "Issue #309 PROBE-TIME slice — アライメント fixture 検証契約"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/309"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-309-probe-time-alignment.md"
  refines:
    - "docs/test-specs/issue-309-probe-time-alignment.md"
  refined-by: []
  derives-from:
    - "docs/designs/issue-309-probe-time-align-fixture.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-309-probe-time-align-fixture.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #309 PROBE-TIME slice — アライメント fixture 検証契約

## Purpose

先行 test-spec（`docs/test-specs/issue-309-probe-time-alignment.md`）
が固定した PROBE-TIME 契約の性質を、**実 runtime + 実 audio
fixture の record stream** で保証する。合成 span の unit 試験が
契約の分岐を網羅するのに対し、本 spec は「drain した実 record
から span を再構成したとき、fixture の既知 latency・信号・gap
に対して offset/interval/Peak/RMS/gaps が期待どおりになる」
ことを固定する。実装が契約と異なる形で確定した場合は本 spec を
改訂する。

## Feature

複数 Probe 観測の実 fixture アライメントと欠落比較
（PROBE-TIME fixture slice）

## Rules

### Rule: 既知 latency 差の複数 Probe は SharedCursor で Aligned になり offset は latency 差に一致する

- 実 `RtRuntime` 上で latency 0 / La / La+Lb の 3 probe が同一
  `(domain, epoch)`・同一 generation・同一 format の record を
  出すとき、`align_observation_points` は `Aligned{basis:
  SharedCursor}` を返し、`offset == L_b - L_a`（B 座標への
  frame 差）・`overlap` が両 span interval の写像後交差と一致する。
- `Aligned` は `error_budget == 0`（rate 同一・provenance
  Derived・coverage 完全）を期待し、根拠なき `Conditional`
  降格をしない。

### Rule: offset 補正後の Peak/RMS/PCM は fixture 期待値と一致する

- probe `P`（latency `L`）の cursor block `p` の観測は入力信号
  `[p-L, p+64-L)` に対応する。MetricRecord の per-channel
  peak/rms は fixture 入力から独立計算した f64 期待値と tolerance
  内で一致し、純粋遅延経路の probe 間対応（`q = p + offset`）は
  f32 bit-exact で一致する。
- CaptureChunk の PCM も同じ対応で bit-exact に一致し、
  `RecordMeta`（generation/sample_position/effective_latency/
  clock）が span 構成の根拠として一貫している。

### Rule: epoch/generation/format/rate/latency/precision の境界は実 record で明示される

- `bump_epoch` 後に収集した record を持つ span 対は
  `Incomparable{EpochMismatch}`（anchor 証拠なし）。generation
  再構成を跨ぐ span は `require_same_generation` 既定で
  `Incomparable{GenerationSpan}`、緩和 policy でのみ
  `Conditional{CrossGeneration}`。
- latency `Unknown` は既定 policy で `Incomparable{LatencyUnknown}`、
  `require_known_latency:false` + `unknown_latency_bound_frames`
  指定時のみ `Conditional{LatencyBounded}`。format/rate の不一致は
  `Incomparable{FormatMismatch}`/`Incomparable{RateMismatch}`、
  f32 RT vs f64 native の精度混在は `Conditional{MixedPrecision}`。

### Rule: 測定無音は Valid に留まり、loss/unobserved は比較 mask から除外される

- 全 zero を feed した区間は `CoverageClass::Valid` のまま
  Peak=0/RMS=0 として比較対象に残る。
- Probe OFF 区間・`LostBlocks`（pool 枯渇）・`LostRecords`
  （telemetry 溢れ）は対応する coverage class として再構成され、
  `Conditional{PartialCoverage}` の `excluded_intervals`（B 座標）
  に現れる。除外区間の PCM/metric を連続データとして比較・参照
  しない。
- `lost_records > 0` の window は `CoverageClass::Unknown` に
  降格し、有効区間として数えない（先行 spec の Rule を fixture
  で確認）。

### Rule: RT/nonRT 境界を fixture でも維持する

- probe（Monitor/Capture）稼働中の `rt.process` は
  `assert_no_alloc` 窓で alloc 0 を維持する（実遅延 DSP の
  delay line 更新を含む）。
- `align_observation_points`/coverage 再構成は非 RT で、入力
  record/interval 数に線形の bounded 計算であり、RT thread から
  到達不能である。
- f32 RT 観測値と f64 native 値を同一精度として比較しない
  （`MixedPrecision` condition または明示 tolerance が必須）。

## Examples

- Given `Source → Delay(64) → Delay(64) → Sink` に 3 probe
  （latency 0/64/128）を Monitor で install し 8 block の既知信号
  （block 5–6 は無音）を流したとき、When drain→span 再構成→
  `align_observation_points(p0, p2)` を呼ぶと、Then
  `Aligned{SharedCursor, offset: 128}`・`overlap == [128, 512)`
  が返り、metric の peak/rms が offset 対応で一致する。
- Given 同一 fixture を Capture で観測し p2 を block 2–3 に
  `Off` にしたとき、When align(p0, p2) を呼ぶと、Then
  `Conditional{PartialCoverage}` で `excluded_intervals` が
  Off 区間（B 座標 `[128, 256)` 相当）を含み、無音 block
  （`[320, 448)`）は excluded に含まれない。
- Given `pool_chunks == probe 数` で drain を 1 block 分遅らせた
  とき、Then `LostBlocks` status・`lost_blocks` counter・
  coverage `LostBlocks` 区間が観測され、当該区間は aligned
  比較から除外される。
- Given `bump_epoch` で epoch 1 に進めた後の record を持つ span
  対、Then `Incomparable{EpochMismatch}`。republish+rebind を
  跨ぐ span は `Incomparable{GenerationSpan}`。
- Given `set_mode(Off)` 区間・`LostBlocks`・`LostRecords`・
  `Unobserved` を含む drain を `ObservationCoverage::reconstruct`
  に通すと、Then 各区間が対応 class に分類され、telemetry loss
  window は `Unknown` に降格する。

## Guarantees

- fixture は公開 API のみを消費し、`src/` の型・既定値・
  private 経路に触れない。
- 期待値は fixture 信号・`effective_latency`・block 対応から
  独立に計算され、実装出力を期待値として再利用しない。
- `Aligned`/`Conditional`/`Incomparable` の判定は先行契約
  （`align_observation_points`）に委譲し、fixture は結果の
  妥当性を検証するだけで再実装しない。
- 先行 slice の保証（mark 未配線 `UNKNOWN`・PCM bit-identical・
  loss counter 分離・RT no-alloc）を退行させない。

## Non-goals

- `EngineFrame`/anchor basis の実データ fixture（anchor 生成は
  後続 Issue）。
- 実デバイス clock・実 plugin・macOS/Linux 実機・consumer repo
  接続（NotRun/Unsupported として記録）。
- PCM resample・PCM 相関・spectral/level の新規解析
  （AudioLens の責務）。
- coverage/loss mask の consumer 向け公開 helper の新設
  （test helper に留める — spec Deferred Concerns）。
