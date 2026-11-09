---
id: "ADR-034Uja8P9oU2FgZCaNy8u9"
type: "adr"
status: "accepted"
title: "issue-309 probe time context placement and alignment contract"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/309"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/308"
    - "https://github.com/scarecrow173/ruviform/issues/280"
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
    - "realtime-contract-and-verification-policy.md"
    - "public-enum-non-exhaustive-policy.md"
    - "issue-185-generic-probe-contract-placement-and-rt-handoff-model.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "media-observation-non-destructive-pcm.md"
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
    - "docs/designs/issue-308-probe-rebind-atomic.md"
    - "docs/designs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  refines:
    - "issue-185-generic-probe-contract-placement-and-rt-handoff-model.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/synvelis/issues/107"
    - "https://github.com/scarecrow173/synvelis/issues/109"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/219"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/280"
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# issue-309 probe time context placement and alignment contract

## Context and Problem Statement

Issue #309（PROBE-TIME）は、#185 で確立した汎用 probe の非 RT 観測面に対し「任意の 2 点以上の Node/Port 観測が同じ時間座標系で比較・整列できるか」を判定する契約を要求する。現状の `RecordMeta` は `generation`+`sample_position`（`RtState.frame_cursor`）+`effective_latency` を持つが、clock domain・epoch identity・runtime instance を持たない。`frame_cursor` は callback 毎に `wrapping_add(frames)` で前進し publish で reset されず、generation は runtime ごとの単調 counter で instance 間一意でない — このままでは異なる時間系列の観測を誤って整列できる。

残る設計判断は (a) 時間 context の配置と record への注入形、(b) clock/epoch 語彙の出所、(c) epoch/discontinuity の権威と RT 配線、(d) 整列 API の結果語彙、(e) 欠落・無効区間（loss mask）の表現、の 5 点である。#308 が確立した世代境界（`RebindReceipt`/`CapturePolicy`）を消費し、clock epoch/segment quality の deferred 境界を本 Issue で閉じる。

## Decision Drivers

* RT 契約（alloc 0・非ブロッキング・OS I/O なし・destructor なし）を `process` 区間で維持すること（RT 契約 ADR）
* record の自己証明性: drain された record 列だけで「この `sample_position` がどの座標系か」を判定できること
* 「Epoch 取得が実装済みと仮定しない」— clock/epoch が不明な場合に `UnknownClock`/`Unalignable` を返し、`UNKNOWN == UNKNOWN` を同期証拠にしないこと（audio-core `same_clock_domain` 規則）
* plan `generation`（slot binding staleness key）と clock epoch（frame cursor 系列の不連続境界）を別 identity として扱い、互いを含意しないこと
* OBS-05（#280）の Clock 語彙・RT bridge を再利用し、別 Clock 正本・別 RT writer/SPSC を作らないこと
* latency 不明・別 epoch・rate 変換不明・over/underrun・data loss・sample discontinuity を条件なしに `Aligned` へ昇格しないこと。resampling・PCM 相関・sound causality の推論を行わないこと
* 測定無音と `Unobserved`/`Stale`/`LostBlocks`/`LostRecords`/Probe OFF/`Unsupported`/`Unavailable` を区別し、loss mask のある波形を連続 PCM として返さないこと
* 公開 enum は `#[non_exhaustive]`、全変更は additive（既存 record/API の意味を変えない）

## Considered Options

* **時間 context の配置**: (a) `RecordMeta` に clock identity mark を additive 追加し context 本体は非 RT registry に保持（hybrid）、(b) `RecordMeta` へ全 context を注入、(c) record は無変更で `generation`+`clock-context-id` の nonRT lookup のみ
* **clock/epoch 語彙**: (a) `ruviform-media-core` の `ClockDomainId`/`ClockEpochId`/`InputStamp`/`TimestampProvenance`/`ClockAnchor`/`EngineFrame`/`map_stamp_to_frame` を再利用、(b) probe 専用の clock/epoch 型を新設
* **epoch/discontinuity の権威**: (a) 非 RT registry が mint し lifetime `ProbeCommand` で RT へ配る（未配線は `UNKNOWN` mark）、(b) RT 側が自己採番、(c) device callback timestamp（`OutputCallbackInfo`/`InputCallbackInfo`）を直接利用
* **整列 API の形**: (a) `ObservationSpan`+`AlignPolicy` → `Aligned{basis,offsets,error_budget}`/`Conditional`/`Incomparable{reason}` の非 RT 三値契約、(b) consumer 側で個別に offset 計算する helper のみ提供、(c) 整列と PCM/resample 処理まで含む高水準 API
* **loss mask の表現**: (a) drain 済み record 列（chunk 区間+status+counters+`RebindReceipt`）から非 RT で coverage を再構成、(b) RT wire に型付き gap/loss record を追加

## Decision Outcome

* **配置**: (a) hybrid。`RecordMeta` に `ProbeClockMark { domain: ClockDomainId, epoch: ClockEpochId }` 相当の小さな `Copy` フィールドを additive 追加し、`ProbeTimeContext` 本体（frame `TimeBase`・runtime instance・format/channel・`LatencyKnowledge`・provenance・`ClockAnchor` 列・generation 窓）は非 RT registry（`ProbeCollector`/broker 側）に保持する。record が自己証明的に座標系を判定でき、context 本体の重複注入を避ける。全注入は telemetry 帯域を浪費し、純粋 lookup は record 解釈が registry version に依存して epoch 境界を跨ぐ join を unsafe にするため棄却。
* **clock/epoch 語彙**: (a) media-core 語彙を再利用。`frame_cursor` を「probe 時間系列の clock domain」として扱い、系列（=runtime instance の cursor 系列）ごとに非 RT registry が一意な `ClockDomainId` を採番（well-known 定数と衝突しない namespace）、`ClockEpochId` は同一 domain 内の不連続カウンタとする。`(domain, epoch)` が実質 `(instance, epoch)` を一意に決めるため record に第 3 の ID を増やさず runtime instance 識別を達成し、`ProbeTimeContext.runtime_instance` は registry 側の報告用フィールドとする。`same_clock_domain`/`same_epoch`/`map_stamp_to_frame`/`MapRejection`/`EngineFrame` の既存規則をそのまま適用する。
* **epoch 権威**: (a) 非 RT mint + lifetime command。系列登録時に `(domain, epoch=初期値)` を確定し、既存 lifetime `ProbeCommand` SPSC（`SetMode` と同経路）で `SetClockMark` 相当を RT へ配る。deck が `current_mark` を `Copy` で保持し `TapCtx` 経由で record に刻む。配線前・未到達期間は `domain=UNKNOWN` を刻み、alignment は `Incomparable{UnknownClock}` を返す。epoch bump（宣言された不連続）は非 RT が新 epoch を mint して同経路で配る。RT 側は mark を置き換えるだけで epoch の意味を解釈しない。device timestamp 直接利用は probe 経路未配線の現状で実装済みと仮定できず、RT 自己採番は restart 検出不能・二重正本のため棄却。`frame_cursor` 自体は変更せず、epoch が座標の有効区間を区切る。
* **整列 API**: (a) `ObservationSpan`+`AlignPolicy` → `Alignment` の非 RT 三値契約を `ruviform-audio::typed::probe` の非 RT 側に置く。`Aligned{basis,offsets,error_budget,overlap}`/`Conditional{…,conditions,excluded_intervals}`/`Incomparable{reason}`。`AlignmentBasis` は `SharedCursor{domain,epoch}`/`EngineFrame{epoch}`（anchor 経由）等で offset の解釈座標系を明示し、異 basis の offset を混ぜない。`IncomparableReason`/`AlignmentBasis`/`Alignment` はすべて `#[non_exhaustive]`。offset は basis 座標の `i64` frame 差、rate 変換は `TimeBase` 有理数演算+明示丸め規則で `error_budget` に残差を含める。latency は `LatencyKnowledge`（`Known{frames}`/`Unknown`/`NotApplicable`）で既知/不明を区別し、policy が既知を要求すれば `Incomparable{LatencyUnknown}`、bounded unknown を許せば `Conditional{LatencyBounded}` とする。resampling・PCM 相関・causality 推論は行わない。
* **loss 表現**: (a) 非 RT 再構成。`ObservationCoverage`（`Valid`/`Unobserved{reason}`/`LostBlocks`/`LostRecords`/`GenerationBoundary`/`RebindDropped`/`Off`/`Unsupported`/`Unavailable`/`Unknown` の半開区間分類）を drain 済み record 列から再構成する。chunk/metric の `[sample_position,+frames)` を `Valid` 候補、status の `sample_position` 以降を kind で上書き、position 飛びを `LostBlocks`/未観測として検出、`lost_records>0` の window は `Unknown` に降格する。型付き gap record の wire 追加は導出可能情報の重複・RT 帯域消費のため棄却。測定無音（Peak=0/RMS=0 の `Valid` 区間値）と未観測区間を coverage で区別し、loss mask のある波形を連続 PCM として返す API を提供しない。
* **#308 境界の接続**: generation は cursor 座標と直交するため、rebind 境界は座標不連続ではなく「観測対象の不連続」として扱う。`RebindReceipt`/status record/`meta.generation` から非 RT で generation segment を再構成し、`ObservationSpan.generation_range` が世代を跨ぐ場合は `AlignPolicy.require_same_generation` で `Incomparable{GenerationSpan}` または `Conditional{CrossGeneration}` を返す。#308 の all-or-nothing atomicity により中途半端な境界は発生しない。

### Consequences

* Good, because: 新規 crate・workspace members・RT wire 形式の破壊的変更が不要。`RecordMeta` への追加は `Copy` POD の additive field のみで既存 record の意味を変えない。media-core/audio-core/observability の既存 Clock 語彙と `same_clock_domain`/`same_epoch`/`map_stamp_to_frame` 規則をそのまま適用でき、OBS-05 RT bridge との二重正本を避けられる。epoch 未取得時は `UNKNOWN` mark → `Incomparable{UnknownClock}` で保守的に倒れ、誤整列を型で防ぐ。
* Bad, because: `RecordMeta` が数 byte〜十数 byte 増え telemetry 帯域に影響しうる（実装時に計測）。domain 採番の namespace 管理・epoch bump 経路の網羅性は実装時の検証事項として残る。device timestamp 未配線の間は live device での epoch が `UNKNOWN` のままとなり、EngineFrame 基準比較は anchor 供給まで `Incomparable` に留まる。

### Confirmation

* test-spec（`docs/test-specs/issue-309-probe-time-alignment.md`）の Rules に従い、同一 `(domain,epoch)`+既知 latency の整列・別 epoch/domain/instance・latency unknown・rate mismatch・coverage mask の各ケースが `Aligned`/`Conditional`/`Incomparable` で正しく分類されることを検証する。
* `assert_no_alloc` 系の RT 検証が probe ON/OFF/Capture 各 mode で非退行（`SetClockMark` は `Copy` 値の置換のみ）。
* `cargo tree -p ruviform-audio` で新規依存が層方向を維持することを確認する。

## Implementation Plan

* Affected paths（実装 dispatch。本 dispatch は docs のみ）: `crates/ruviform-audio/src/typed/probe.rs`（`ProbeClockMark`/`RecordMeta` additive 追加・`ProbeCommand::SetClockMark`・非 RT registry/`ObservationSpan`/`Alignment`/`ObservationCoverage`/`align_observation_points`）、`crates/ruviform-audio/src/typed/runtime.rs`（`TapCtx`/`current_mark` の additive 拡張）、`crates/ruviform-audio/tests/`（整列・coverage テスト）。
* Patterns to follow: lifetime `ProbeCommand` SPSC（`SetMode` と同経路・毎 callback drain）、`Copy` POD record、`#[non_exhaustive]` 公開 enum、`**RT contract:**` doc ラベル、`ChannelLevel`/`MetricRecord` の既存再利用、media-core `ClockAnchor`/`map_stamp_to_frame`/`EngineFrame`/`TimeBase` の語彙。
* Patterns to avoid: RT 側での epoch 採番・Vec 確保・registry lookup・destructor、wire への型付き gap record 追加、`UNKNOWN` 同士を同期証拠とみなす比較、resample/PCM 相関の自動実行、generation を epoch の代用にすること。
* Dependencies or configuration: 追加依存なし（`ruviform-media-core`/`ruviform-audio-core`/`ruviform-observability` の既存依存を再利用）。

## Verification

* [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`verify-ci` exit 0（実装 dispatch で実施）
* [ ] `rt_probe_*` 系テストで `assert_no_alloc` 窓内の alloc 0 が非退行
* [ ] 整列 API が `Aligned`/`Conditional`/`Incomparable` を理由・basis・error_budget・欠落区間つきで返す
* [ ] coverage 再構成が `Unobserved`/`LostBlocks`/`LostRecords`/generation boundary/Probe OFF を区別し、telemetry 欠落時に `Unknown` へ降格する
* [ ] probe ON/OFF/Capture で sink 出力 PCM が bit-identical（#308 test-spec の既存保証を壊さない）

## Pros and Cons of the Options

### 時間 context の配置

* **(a) hybrid: record に identity mark + registry に本体（採用）**: record が自己証明的で、context の重複注入を避ける。RT コストは `Copy` field 追加のみ。悪い点: record に型が 1 つ増え、mark 未配線期間は `UNKNOWN` で `Incomparable` が増える（保守的で正しい）。
* **(b) `RecordMeta` 全注入**: record が完全に自己完結。悪い点: `TimeBase`/anchors/format/latency を全 record に複製し telemetry 帯域を浪費。系列内でほぼ不変な情報を per-record に持つ必然性がない。
* **(c) 純粋 nonRT lookup**: record 形状不変。悪い点: `generation` は instance 間一意でなく、epoch 境界を跨ぐ record 列の解釈が registry version に依存して unsafe。record だけでは座標系を判定できない。

### clock/epoch 語彙

* **(a) media-core 語彙再利用（採用）**: `same_clock_domain`/`same_epoch`/`map_stamp_to_frame`/`EngineFrame`/`ClockAnchor` の既存規則がそのまま効き、OBS-05・audio-core との語彙統一。悪い点: `frame_cursor` を clock domain とみなす意味論的飛躍を文書で明示する必要がある（本 ADR と design で実施）。
* **(b) probe 専用型新設**: 意味論を自由に定義できる。悪い点: `ClockDomainId`/`ClockEpochId`/`InputStamp`/`ClockContext`/`MapRejection`/`EngineFrame` を再発明し、OBS-05 bridge・timed_input・transport_schedule との相互運用に変換層が必要になる。Issue が明示する「別 Clock 正本を作らない」に反する。

### epoch 権威

* **(a) 非 RT mint + lifetime command（採用）**: restart/再構成を非 RT が確実に検出して epoch を更新でき、RT は `Copy` 値の置換のみ。未配線は `UNKNOWN` で保守的。悪い点: command が RT に届くまでの窓では旧 mark の record が出る（consumer は status/`Installed` 等で窓を把握可能）。
* **(b) RT 自己採番**: 配線不要。悪い点: RT は runtime 再構成・device restart を「検出」する手段を持たず、cursor reset と epoch の対応を保証できない。二重正本化。
* **(c) device timestamp 直接利用**: device clock との直接相関が取れる。悪い点: probe 経路に `OutputCallbackInfo`/`InputCallbackInfo` が未配線で、typed `RtRuntime` は現状 offline render のみで駆動される。「Epoch 取得が実装済みと仮定しない」要件に反し、配線は別 Issue の実装事項。

### 整列 API の形

* **(a) `ObservationSpan`+`AlignPolicy`→三値（採用）**: basis・offsets・error_budget・reason を型で返し、consumer が比較可否を独自判断しない。悪い点: API 面が増える（ただし非 RT・additive）。
* **(b) helper のみ**: 最小 API。悪い点: 「Aligned/Conditional/Incomparable」の分類責務が consumer に分散し、Issue が防ぐ誤比較が残る。
* **(c) 高水準 API（resample/PCM 差分込み）**: consumer の実装量が減る。悪い点: Issue が明示的に禁止する自動 resampling/PCM 相関・tolerance 暗黙化を招く。

### loss mask の表現

* **(a) 非 RT 再構成（採用）**: wire 形式・RT 帯域を変えず、既存 record/status/counter/`RebindReceipt` の情報を集約。telemetry 欠落時は `Unknown` 降格で保守的。悪い点: status 欠落で本来分かるはずの区間が `Unknown` になる保守性コスト。
* **(b) 型付き gap record 追加**: 区間が wire 上で明示。悪い点: 導出可能な情報を RT 帯域で重複送信し、chunk/status の drop で gap record 自体も欠落しうる（blind spot は残る）。RT record サイズ・SPSC 容量設計への影響も大きい。

## More Information

* 一次要件: [Issue #309](https://github.com/scarecrow173/ruviform/issues/309)。上流: [#185](https://github.com/scarecrow173/ruviform/issues/185)（probe 基盤）、[#308](https://github.com/scarecrow173/ruviform/issues/308)（rebind 境界）、[#280](https://github.com/scarecrow173/ruviform/issues/280)（OBS-05 Clock/RT bridge）。
* Consumer: Synvelis [SF-04 #107](https://github.com/scarecrow173/synvelis/issues/107)・[SF-06 #109](https://github.com/scarecrow173/synvelis/issues/109)、cross-consumer [#219](https://github.com/scarecrow173/ruviform/issues/219)/[#294](https://github.com/scarecrow173/ruviform/issues/294)。
* 前提 ADR: [RT 契約](realtime-contract-and-verification-policy.md)、[公開 enum 非網羅](public-enum-non-exhaustive-policy.md)、[#185 probe 配置](issue-185-generic-probe-contract-placement-and-rt-handoff-model.md)。
* 詳細設計: [docs/designs/issue-309-probe-time-context-and-alignment.md](../designs/issue-309-probe-time-context-and-alignment.md)。検証契約: [docs/test-specs/issue-309-probe-time-alignment.md](../test-specs/issue-309-probe-time-alignment.md)。
