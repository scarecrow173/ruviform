---
id: "TSPEC-034RTrq34tAM6KDpPaC83H"
type: "test-spec"
status: "approved"
title: "Issue #279 OBS-04 plugin-host 4 Counter Source登録・寿命・degraded試験"
created: "2026-11-04"
updated: "2026-11-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/279"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-277-observability-core-source-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/plugin-endpoint-contract-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-279-plugin-host-counter-source-registration.md"
    - "docs/designs/issue-279-plugin-host-counter-source-registration.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-279-plugin-host-counter-source-registration.md"
    - "docs/designs/issue-279-plugin-host-counter-source-registration.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/278"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/278"
  deferred-by: []
---

# Issue #279 OBS-04 plugin-host 4 Counter Source登録・寿命・degraded試験

## Purpose

Issue #279（OBS-04）の Plugin Host 4 Counter Source 登録・Descriptor・Lease
寿命・typed degradation が spec AC-1〜AC-7 を満たすことを固定する試験仕様。
OBS-07 Collector / OBS-03 tracing_export が未実装のため、本仕様は
`ObservationRegistrar` ポートの Domain 側消費（登録・寿命・degrad）と既存
Counter 契約の非回帰のみを検証し、Collector/変換層の試験は各 Issue に委ねる。

## Feature

L1 audio Instance の mint/destroy に伴い、既存 4 診断 Counter が Observation
Source として Domain 宣言の semantics 付きで登録・退避され、未接続/失敗時も
plugin processing を止めない。

## Rules

- Rule: `PluginHost::new()`/`Default` の `observation_registrar` は `None` —
  registrar 未接続では一度も `register` を呼ばず、既存の mint/destroy/query
  経路と同値で動く（既存互換・後方互換の既定）。
- Rule: registrar 接続時、audio `CreateInstance` の `Creating → Live` flip に
  成功した mint だけが 4 Source を `scope=Instance` で登録する。dedicated /
  bundled / supervisor restart の全 mint 経路に同じ規約が効く。
- Rule: video Instance は登録しない（`InstKind::Video` は counter を持たない
  構造的不変条件）。L0 in-process 経路も対象外。
- Rule: 4 Descriptor は `kind=Counter`・`aggregation=Cumulative`・`unit=Count`・
  `scope=Instance` で、Issue #121 の意味表に忠実な `SourceSemantics` を持つ。
  `passthrough_submits` は `SnapshotOnly`（warmup 込み増分を無条件 WARN しない）、
  3 つの fault counter は `OnIncrease` + High impact。
- Rule: per-slot 登録は独立 — 1 本が `Refused(RegistrationError)` でも他 3 本は
  登録され、plugin processing は継続する（typed degradation。誤 Series 合成や
  fallback key 変造をしない）。
- Rule: 同名 key・異 Instance は `SourceId`/`SourceGeneration` で区別され、
  re-mint（generation+1 の新 handle）は新 cell = 新 `SourceIdentity` を得る。
- Rule: `destroy`（明示・`Drop` 経由問わず）は writer quiescence
  （`drain_endpoint_channels` + `remove_instance` による owner writer の消滅）
  の**後**に `retire` する（`Active → Retiring`、ticket 退避）。failed-create
  rollback（Live flip 未到達）では登録自体が起きない。
- Rule: leg swap（`arm`/`arm_apply`）・endpoint checkout/return・Failed
  convergence は cell が継続するため Source も継続する — 再登録しない
  （同一 cell の再登録は `SourceReused` かつ誤 Series 合成の構造的原因）。
- Rule: RT path（`L1Instance::process`/leg writers/`RtLegCounters` の全
  increment 点）に Registry Lock・alloc・IO・Subscriber・Callback を追加
  しない。`RegistrationLease`/`RetiredSource`/`ObservationRegistrar` handle は
  RT leg に一切届かない。

## Examples

- Given `PluginHost`（registrar 未接続）、when audio instance を mint →
  `audio_counters` query、then 4 counter は従来どおり増分・参照でき、
  `source_leases` は 4 スロット全て `Unattached`。
- Given in-crate recording registrar（正常）、when dedicated spawn で mint、
  then `register` が 4 回呼ばれ、key は `l1.dropped_submits` /
  `l1.passthrough_submits` / `l1.results_failed_observed` /
  `l1.results_dropped`、scope は `Instance`、kind は `Counter`、全 slot が
  `Registered(Active)`。
- Given registrar が 2 番目の `register` で `CapacityExceeded`、then mint は
  成功し、slot は `Registered, Refused(CapacityExceeded), …` の混在を保持し、
  `audio_counters` query は 4 counter 全て同値で答える。
- Given live instance + Active leases、when `destroy`、then 全 lease が
  `Retiring` に遷移し `RetiredSource` ticket が退避される；registrar 側で
  `begin_final_drain` → `mark_removed` を駆動すると ticket 経由で `Removed`
  まで観測できる。
- Given `create_instance` が `Live` flip 前に失敗（reply error / converge
  fold）、then `register` は一度も呼ばれない（rollback は未登録で終わる）。
- Given bundled pool の 2 instance（同名 key）、when 両 mint、then 4 × 2 =
  8 Source が登録され各 `SourceId` は一意、誤 Series 合成は起きない。
- Given destroy 済み instance と supervisor restart（`Instantiate` の再実行）、
  then 新 mint は新 cell + 新 `SourceIdentity`（generation 前進）を得て、
  旧 ticket は `Removed` に進行済み。

## Guarantees

- `l1/sources.rs` mod tests が register/unattached/Refused/retire/ticket 退避を
  in-crate test registrar で決定的に検証する（`ObservationRegistrar` は
  object-safe なので test impl を `Arc<dyn>` で供給可能 — OBS-02 が保証）。
- `create_instance` harness 試験（`create_harness`/`spawn_create` と同機構）が
  mint→登録・失敗→未登録の配線を検証する。
- `PluginHost` 側の試験が `set_observation_registrar` → bundled/dedicated mint
  → destroy→retire の公開経路を検証する。
- 既存 `rt_`/`test` lane（`audio_counters_*`、RT leg、counters 増分試験）が
  無改変で pass し、4 Counter の意味・増分位置・writer 配置が回帰しない。
- `cargo build` で `RegistrationLease`（`Send` のみ）が `L1Instance`/`InstanceParts`
  経路で所有でき、`Shared`/`InstShared`（`Send + Sync` 要求）に置かれないことが
  コンパイルで担保される。

## Non-goals

- Collector の `poll`/Baseline/Rate/History/Snapshot/`MeasurementValidity` の
  試験（OBS-07 #289 — 実 Registry は存在しない）。
- `tracing_export` の event_code/schema/Severity 一致（OBS-03 #278 — 変換層
  未実装）。
- 実機 timing/latency の計測・実 Plugin/実 GUI 経路（headless 検証のみ）。
- AudioLens/Synvelis 経路の OBS-06 証跡（cross-repo gate #281）。
- `passthrough_submits` warmup の `Duration` 化検証（mint 時に sample rate
  未定 — spec/design の Deferred 記録どおり）。

## Risk

- Registrar 注入だけが行われて RT path に Registry handle が漏れると、RT
  クリティカルパスに lock/alloc を持ち込み `test-rt` レーンで検出され得る。
- `Refused` を握りつぶすと「Collector attach 失敗でも plugin は継続」の
  契約は守れても degraded の観測ができず、後続 Diagnostics が原因を
  判別できない — slot 記録が唯一の正直な証跡。
- `retire` を `remove_instance` より前に置くと `results_dropped` の
  residual flush が quiescence 宣言後に走り、Collector 側で drain 不全の
  誤解釈を生む（順序が pin されるべき不変条件）。
