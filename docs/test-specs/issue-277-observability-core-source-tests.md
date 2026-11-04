---
id: "TSPEC-034Rp29Zp9BL0LmhcrU4IF"
type: "test-spec"
status: "approved"
title: "Issue #277 OBS-02 observability core/source contract tests"
created: "2026-11-03"
updated: "2026-11-03"
owners: ["scarecrow173"]
relations:
  source: []
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/designs/issue-277-observability-core-source-contract.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #277 OBS-02 observability core/source contract tests

## Purpose

Issue #277（OBS-02）の core primitive・source 登録/寿命契約が spec AC-1〜AC-7 を
満たすことを固定する試験仕様。契約が OBS-07 実装で再解釈されないよう、
「何を検証すれば契約を守ったと言えるか」の正本とする。

## Feature

軽量観測 Primitive（Counter/Gauge/Histogram）と Observation Source の
登録・寿命契約が、RT/非 RT 境界と責務分離を守って振る舞う。

## Rules

- Rule: 既存 `counter()` ペアの API・意味・RT 契約は回帰しない（単一 Counter の
  非減少・共有 cell・Relaxed・Clone/Drop 非 RT 境界を含む）。
- Rule: `Gauge` は単一 `Arc<AtomicU64>` のみで、`set`/`read`/`reader`/
  `GaugeReader::read` が no-alloc。単調増加は約束しない。
- Rule: `Histogram` は構築時に境界を検証し（非空・厳密昇順でなければ Error）、
  `observe` は有界に Bucket をインクリメントする。`snapshot` は Bucket 別
  Relaxed load で alloc する非 RT 操作かつ一貫 snapshot を約束しない。
- Rule: `MetricSource::new` は descriptor.kind と reader kind の不整合を
  `SourceBuildError::KindMismatch` で拒否する。
- Rule: `SourceLifecycle` は `Active → Retiring → FinalDrain → Removed` にのみ
  遷移し、不正遷移・終端到達後の遷移は `LifecycleError` を返す。
- Rule: `RegistrationLease::retire` は `Active → Retiring` を行い ticket を返す。
  retire せずに Drop した Lease は `abandoned` として観測可能であり、Reader
  cell（Registry 側所有）を消さない。
- Rule: `ObservationRegistrar` は object-safe で `Send + Sync`、Lease/状態共有に
  循環参照を持たない。
- Rule: Core/Source 型は `tracing`/`tracing-subscriber`/外部 IO/GUI に依存せず、
  crate `[dependencies]` は空（dev-dependency の `assert_no_alloc` のみ）。

## Examples

- Given `gauge(0)`、when `set(7)` して `read()`、then `7` が返り、reader clone も
  同じ値を読む（Counter と違い後の `set(3)` で `3` に減少する）。
- Given `histogram(&[10, 100])`、when `observe(5)`/`observe(50)`/`observe(500)`、
  then `snapshot()` は `[1, 1, 1]` を返す。
- Given `histogram(&[])`/`histogram(&[5, 5])`、then `HistogramError::
  {EmptyBoundaries, UnsortedBoundaries}`。
- Given Active lease、when `retire()` → collector が `begin_final_drain()` →
  `mark_removed()`、then lifecycle は Retiring→FinalDrain→Removed と進み、
  ticket からも観測できる。
- Given Active lease、when `begin_final_drain()` を直接呼ぶ、then
  `LifecycleError::InvalidTransition`（quiescence 宣言前の drain 不可）。
- Given lease を retire せず drop、then `is_abandoned() == true` かつ
  lifecycle は Retiring へ遷移する。
- Given Removed 到達後、when `mark_removed()`/`begin_final_drain()`、then
  `InvalidTransition`（Source 失効後の遷移失敗が決定的）。

## Guarantees

- 公開 API 例は doctest で検証される。
- `rt_` プレフィックス試験が `assert_no_alloc` 窓内で counter/gauge/histogram
  の RT 許可操作の no-alloc を実証する（既存レーンと同一機構）。
- 全公開 enum は `#[non_exhaustive]`（`lint` の `exhaustive_enums` で強制）。

## Non-goals

- 実 Registry・Poll・Baseline・History の試験（OBS-07）。
- Histogram `observe` の正式 RT-safe 宣言（no-alloc 実証のみ・宣言は保留）。
- 実機 timing/latency・blocking 検証（assert_no_alloc の範囲外・doc 契約）。

## Risk

- 契約だけの型をテストなしで置くと、OBS-07 が「実装不可能なポート」や
  「循環参照する Lease」に気付かない — コンパイル/決定的試験で事前に潰す。
- Counter 回帰は plugin-host(#121)・audio(#185) の計装を直接壊す。
