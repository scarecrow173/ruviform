---
id: "DESIGN-034Uja29JLpwyE6xff7quc"
type: "design"
status: "approved"
title: "Issue #309 — Probe時間コンテキストと観測整列契約の設計（PROBE-TIME）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
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
  depends-on:
    - "docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/overview.md"
    - "docs/designs/issue-308-probe-rebind-atomic.md"
    - "docs/designs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/issue-185-generic-probe-contract-placement-and-rt-handoff-model.md"
    - "../../AGENTS.md"
  refines:
    - "docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/308"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/synvelis/issues/107"
    - "https://github.com/scarecrow173/synvelis/issues/109"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/280"
  deferred-by: []
---

# Issue #309 — Probe時間コンテキストと観測整列契約の設計（PROBE-TIME）

## Context

Issue #185 の probe 基盤は `RecordMeta { probe, node, port, generation, sample_position, frames, channels, sample_rate, effective_latency }`（`crates/ruviform-audio/src/typed/probe.rs`）を全 record（`MetricRecord`/`CaptureChunk`/`StatusRecord`）に載せる。しかし `sample_position` は `RtState.frame_cursor` — `RtRuntime::process` が callback 毎に `wrapping_add(frames)` で前進させる「処理済み frame 数」であり、以下を区別できない:

- runtime instance（`GraphRuntime`/`RtState` の再構成で cursor は 0 から再開するが、`generation` は runtime ごとの単調 counter で instance 間一意でない）
- clock epoch（seek/loop/device restart/stream 再構成で座標の意味が変わりうるが、cursor 自体は reset されない）
- device/transport clock domain との関係（`OutputCallbackInfo`/`InputCallbackInfo` は audio-core に存在するが probe 経路へ未配線。`RtContext` は ZST で typed `RtRuntime` は現状 offline render 経路のみで駆動される）
- latency の既知/不明（`effective_latency: u32` は値のみで provenance を持たない）

#308（main 統合済み）が世代切替を atomic rebind・`RebindReceipt`・`CapturePolicy::{StopAtBoundary,Restart}` で確立し、「Clock Epoch / port identity / Segment quality classification」を明示的に #309 へ defer した。本設計はその deferred 境界を閉じる。

再利用する既存語彙（新規 Clock 正本は作らない）:

- `ruviform-media-core::timed_input`: `ClockDomainId`（`UNKNOWN`/`SAMPLE_CLOCK`/`MONOTONIC`）、`InputStamp`、`TimestampProvenance`（`Carried`/`Derived`/`Estimated`）、`ClockAnchor`、`map_stamp_to_frame`、`MapRejection`（`UnmappedDomain`/`BaseMismatch`/`EpochMismatch`/`BeforeAnchor`/`Overflow`）
- `ruviform-media-core::transport_schedule`: `ClockEpochId`、`EngineFrame`（`same_epoch`/`checked_offset`）
- `ruviform-observability::record::ClockContext`（`InputStamp`+`ClockEpochId`、別 epoch 比較不可を型で保証）— OBS-05（#280）の RT bridge record が運ぶ語彙
- `ruviform-audio-core::device`: `same_clock_domain`（equal かつ非 `UNKNOWN` のみ真）、`OutputCallbackInfo`/`InputCallbackInfo`/`StreamLatency`

## Design Decisions

### D1. 時間座標モデル: frame cursor を「probe 時間系列（probe time series）」の domain として扱う

`sample_position`/`frame_cursor` は **processing-span 座標**であり、media time・transport position・wall clock ではない。この座標を `ClockDomainId`+`ClockEpochId` で識別する:

- **`ClockDomainId`**: probe 時間系列ごとに非 RT registry が一意に採番する（media-core の well-known 定数 `UNKNOWN`/`SAMPLE_CLOCK`/`MONOTONIC` と衝突しない範囲。具体的な採番規則は実装 dispatch で確定 — 例: 専用 namespace の `u32` 空間）。異なる runtime instance・異なる frame cursor 系列は**異なる clock domain**である — これは語彙の意味論に忠実であり、「同じ domain id = 同じ時計」という既存規則（`same_clock_domain`）がそのまま適用できる。
- **`ClockEpochId`**: 同一 domain 内での不連続境界カウンタ。系列生成時に初期値（例: 0）を持ち、宣言された不連続（後述 D4）でインクリメントする。`EngineFrame`/`ClockEpochId` の「異 epoch は減算不可」規則をそのまま継承する。
- **runtime instance**: `ProbeTimeContext` 本体のフィールドとして保持する（Issue の必須意味に明記）。record 上の mark は `(domain, epoch)` のみで、domain が instance ごとに一意なため `(domain, epoch)` が実質 `(instance, epoch)` を一意に決める — record に第 3 の ID を増やさずに instance 識別を達成する。

### D2. `RecordMeta` への clock mark 注入（Issue 未決問の決定）

**決定: record には clock *identity* のみを載せ、context 本体は非 RT lookup に残す（hybrid）。**

- 追加するのは `ProbeClockMark { domain: ClockDomainId, epoch: ClockEpochId }` 相当の小さな `Copy` フィールド（`RecordMeta` は既に全 record で `Copy` POD）。RT コストは record あたり十数 byte の増加のみで、alloc・indirection・lock は発生しない。
- **全注入を棄却する理由**: `TimeBase`/`provenance`/anchors/format/latency 詳細は系列内でほぼ不変であり、全 record に複製すると telemetry 帯域を無駄に消費する。record が自己完結する必要があるのは「この `sample_position` がどの座標系のものか」という identity だけ。
- **純粋 lookup（record に何も足さない）を棄却する理由**: `(generation)` や probe id から context を引く設計では、epoch 境界を跨いだ record 列を consumer が join する際に「どの時点の context が有効だったか」を record 自身から判定できず、registry の version 管理が record 解釈の正しさに混入する。identity を record に刻めば、各 record は自己証明的に「比較可能か」を判定できる。
- **未設定時の既定**: mark が未配線の系列では `domain = ClockDomainId::UNKNOWN`（epoch 値は無意味）を刻む。`UNKNOWN` 同士の一致は同期証拠にならないため、alignment は必ず `Incomparable{reason: UnknownClock}` を返す — 「Epoch 取得が実装済みと仮定しない」要件に対応。

### D3. `ProbeTimeContext`（型名暫定）: 非 RT registry の系列メタデータ

非 RT 側（`ProbeCollector`/broker が所有する registry）に、系列ごとの context を保持する:

```text
ProbeTimeContext {
    series:      { domain: ClockDomainId,            // D1 で採番
                   epoch:  ClockEpochId,              // 現在の epoch
                   runtime_instance: RuntimeInstanceId }, // 非 RT 採番 u64
    frame:       { base: TimeBase,                    // frame interval の正本
                   sample_rate: u32 },                // RecordMeta と一致する宣言値
    format:      { channels: u16, channel_layout },   // format/channel metadata
    latency:     LatencyKnowledge,                    // Known{frames} | Unknown | NotApplicable
    provenance:  TimestampProvenance,                // frame cursor の導出根拠
    anchors:     Vec<ClockAnchor>,                   // EngineFrame/device clock への写像（あれば）
    generation_window: { first, last },              // この context が有効だった plan generation 範囲
}
```

- `generation_window` は「この時間系列がどの plan generation を観測したか」を記録する。generation は cursor 座標と直交する（publish で cursor は reset されない）ため、context は単一 generation に紐付かない — 窓として持つ。
- `anchors` は OBS-05 の `ClockAnchor`/`map_stamp_to_frame` をそのまま使い、frame cursor 座標 → `EngineFrame`（media time 系列）への写像を表す。offline render 経路では render driver が `FeedContext.engine_frame` と `frame_cursor` の対応を anchor として供給できる。anchor が無ければ EngineFrame 基準の比較は `Incomparable`。
- registry は非 RT・bounded（系列数 = runtime instance 数で有限）。record 解釈は registry を必須としない（identity は record 内にある）が、format/latency/anchors/provenance の解決には registry lookup が必要。

### D4. epoch/discontinuity の与え方（RT 配線を最小化）

- **初期 mark**: 非 RT 側で probe deck 構築/系列登録時に `(domain, epoch=初期値)` を確定し、RT へは既存の lifetime `ProbeCommand`（SPSC、`SetMode` と同じ経路）で `SetClockMark` 相当を送る。mark は probe 個別ではなく **series/deck スコープ**の命令として扱う（全 probe が同一 frame cursor 系列を共有するため）。`RtProbe`/deck が `current_mark` を `Copy` で保持し、`TapCtx` 経由で各 record に刻む。**配線前・command 未到達の期間は `UNKNOWN` mark** — 推測で埋めない。
- **epoch bump**: 宣言された不連続（runtime 再構成=新系列として新 domain、device stream restart、明示的な seek/loop 通知が将来配線された場合）は非 RT が新 epoch を mint して同じ command 経路で配る。RT 側は mark を置き換えるだけで、epoch の「意味」を解釈しない。
- **frame_cursor 自体は変更しない**: cursor は処理 frame の単純計数であり続け、epoch が座標の有効区間を区切る。`publish`（generation 変更）では cursor も epoch も変えない — 世代境界は D6 で別途扱う。
- **device clock の直接配線は本設計の対象外**: `OutputCallbackInfo`/`InputCallbackInfo` を probe 経路へ配線する実装は別 Issue。ただし `ProbeTimeContext.anchors` が device clock → frame cursor の `ClockAnchor` を将来受け入れられる形にしておく（`ClockDomainId` は device 側 domain を指せる）。

### D5. `align_observation_points(a, b, policy)` 非 RT 契約

配置: `ruviform-audio::typed::probe` 内の非 RT 側（collector/broker が見える層）。RT からは到達不能な pure 非 RT 関数で、Vec 確保・走査を許容するが bounded（入力は drain 済み record 列 or 明示的な区間指定）。

入力 `ObservationSpan`（観測区間の自己記述ビュー）:

```text
ObservationSpan {
    probe, point:        ProbeId + NodeId/PortId,
    clock_mark:          ProbeClockMark,           // domain+epoch（record 由来）
    interval:            FrameInterval,           // [start, end) in cursor frames
    generation_range:    GenerationRange,
    format:              { channels, sample_rate/timebase, channel_layout },
    latency:             LatencyKnowledge,        // effective_latency + 既知/不明
    coverage:            &ObservationCoverage,    // D7 の欠落・無効区間
    precision:           ProbePrecision,          // f32 RT 値であることの明示
}
```

出力 `Alignment`（`#[non_exhaustive]`）:

```text
Alignment =
  | Aligned    { basis, offsets, error_budget, overlap }
  | Conditional{ basis, offsets, error_budget, conditions, excluded_intervals }
  | Incomparable{ reason }
```

- **`AlignmentBasis`**（`#[non_exhaustive]`）: `SharedCursor{domain,epoch}`（同一系列・同一 epoch — frame 差分が直接有効）、`EngineFrame{epoch}`（双方が anchor 経由で `EngineFrame` に写像 — media time 基準）、将来の `DeviceClock{domain}`。basis は「どの座標系で offset を解釈するか」を明示し、異 basis の offset を混ぜない。
- **`offsets`**: A→B の frame 単位写像（basis 座標での `i64`）。同一 `SharedCursor` なら cursor frame 差、`EngineFrame` なら `checked_offset` 相当の検証済み差分。rate 変換が必要な場合は `TimeBase` の有理数演算で tick 単位へ落とし、丸め規則（最近接・半偶数、`media-core` の `Rounding` 語彙を再利用）と残差を `error_budget` に含める。
- **`error_budget`**: 整列の最大誤差を frames/ticks で明示。根拠: anchor calibration の `TimestampProvenance`（`Estimated` は誤差を広げる/`Conditional` へ降格）、rate 変換の丸め残差、latency 未知時の bound（下記）。
- **`IncomparableReason`**（`#[non_exhaustive]`）: `UnknownClock`（mark 未設定/`UNKNOWN` domain）、`DomainMismatch`、`EpochMismatch`、`SeriesMismatch`（共有 anchor なしの別系列）、`RateMismatch{a,b}`（変換根拠なし）、`FormatMismatch`、`LatencyUnknown`（policy が既知 latency を要求）、`CoverageEmpty`、`DiscontinuityInWindow`、`GenerationSpan`（D6 の generation 跨ぎ）、`AnchorMissing`/`AnchorRejected{MapRejection}`、`Overflow`。
- **`Conditional`**: `Aligned` の要件を部分的に満たす場合。例: `LatencyBounded{bound_frames}`（latency 不明だが policy が bounded unknown を許容）、`ProvenanceEstimated`（anchor/stamp が `Estimated`）、`PartialCoverage`（比較区間内に mask があるが除外指定済み）。条件と除外区間を必ず併記し、consumer が根拠を検証できるようにする。
- **`AlignPolicy`**: consumer 宣言の要求 — `require_same_generation`、`require_known_latency`、`max_error_frames`、`allow_conditional`、basis 優先順位。**policy なしの暗黙昇格はしない**: latency 不明・別 epoch・rate 変換不明・overflow/underrun・data loss・sample discontinuity を条件なしに `Aligned` へ上げない（Issue 要件）。resampling・PCM 相関・sound causality の推論は行わない。

### D6. #308 rebind 境界との接続

- plan `generation` は cursor 座標と直交する（publish で cursor/epoch は不変）。したがって generation 境界は **座標の不連続ではなく「観測対象の不連続」**: rebind で slot 解決・effective_latency・processor 状態が変わりうるため、同一 generation 内の区間同士だけが「同じ信号の同じ時刻」を保証する。
- `RebindReceipt`/`CapturePolicy` の効果は既存 status record（`CaptureStopped{reason:GenerationBoundary}`/`RebindDropped`/`CaptureStarted`/`Installed`）と `meta.generation` から非 RT で再構成できる。coverage 再構成（D7）は generation 境界を **segment boundary** として記録し、`ObservationSpan.generation_range` が単一世代を跨ぐ場合は policy が `require_same_generation` を立てていれば `Incomparable{reason: GenerationSpan}`（または `Conditional{CrossGeneration}`）を返す。
- #308 の原子性（all-or-nothing staged rebind）により「片側だけ新 generation」という中途半端な境界は発生しない — coverage は receipt の `bound`/`unbound`/`capture_stopped` をそのまま信頼できる。

### D7. loss mask / coverage の表現: 非 RT 再構成（wire に新 record を足さない）

**決定: 型付き gap record を RT wire に追加せず、drain 済み record 列から非 RT で coverage を再構成する。**

- `ObservationCoverage`: probe ごとの区間分類 — `Valid` / `Unobserved{reason}`（`Stale`/`BorrowConflict`）/ `LostBlocks` / `LostRecords` / `GenerationBoundary` / `RebindDropped` / `Off`（`ModeChanged`/`CaptureStopped` から）/ `Unsupported`・`Unavailable`（capability 報告由来）/ `Unknown`（telemetry 欠落で状態不明）。区間は半開 `[start,end)` の frame 座標。
- 再構成規則: `CaptureChunk`/`MetricRecord` の `[sample_position, +frames)` を `Valid` 候補とし、`StatusRecord` の `sample_position` 以降を対応 kind で上書き。chunk 列の position 飛びは `LostBlocks`（chunk pool 枯渇）または未観測区間として検出。telemetry の `lost_records > 0` は「status が欠けている可能性」を意味し、該当 window の coverage を `Unknown` に降格する（counter が不確実性の大きさを bound する）。
- **無音との区別**: `MetricRecord.levels` の Peak=0/RMS=0 は「測定されたデジタル無音」であり `Valid` 区間内の値。`Unobserved`/`Lost*`/`Off`/`Unknown` 区間には「無音」という値を割り当てない — consumer は coverage を見て「測定値なし」と「測定結果 0」を区別する。loss mask のある波形を連続 PCM 列として扱う API は提供しない（`ObservationSpan` は常に coverage 参照を伴う）。
- 再構成は非 RT・bounded（record 数に線形、Vec 確保可）。RT wire・`RecordMeta` の形は変えない（D2 の mark 追加のみ）。

### D8. metrics/PCM 比較の再利用方針

- Peak/RMS: 既存 `MetricRecord`（`ChannelLevel` contract、`ruviform-audio-analysis` 由来）をそのまま比較対象にする。整列は D5 の `Aligned`/`Conditional` 結果の `offsets`/`overlap` に従って区間を対応付ける。channel 対応は `format.channel_layout` が一致する場合のみ直接比較し、不一致は `FormatMismatch`。
- PCM 差分: `CaptureChunk` は `Aligned`（または `Conditional` で consumer が条件受理）かつ format/coverage 前提を満たす場合のみ bounded 区間で比較する。f32 probe 値は native f64 解析ソースの bit-exact 表現ではない — `ObservationSpan.precision` がこれを型レベルで明示し、AudioLens f64 との比較は #294 の別検証に委ねる。
- 数値 tolerance/peak rounding は `AlignPolicy`/比較 policy で明示必須（Issue 要件）。暗黙の epsilon を埋め込まない。

## Data Flow

```text
nonRT                                    RT (process 区間)
─────                                    ─────────────────
ProbeTimeRegistry                         ProbeDeck/RtProbe
  .register_series() → {domain,epoch0}      .current_mark (Copy, via
  .context(series) → ProbeTimeContext         lifetime ProbeCommand)
        │                                  TapCtx{..., mark}
        │ SetClockMark command ──────────►  run_site stamps
        │                                    RecordMeta.clock
        ▼                                  Metric/Capture/Status
ProbeCollector.drain()                            │ SPSC (既存)
  → records (meta.clock 付き) ◄───────────────────┘
  → ProbeCoverage::reconstruct(records)
  → align_observation_points(spanA, spanB, policy)
       → Aligned / Conditional / Incomparable
```

## Alternatives Considered

| 論点 | 選択 | 棄却案と理由 |
| --- | --- | --- |
| context 配置 | record に identity mark + 非 RT registry に本体（D2/D3） | 全注入: 帯域浪費・不変情報の重複。純粋 lookup: record 自己証明性の喪失、version 管理の混入 |
| clock 語彙 | media-core `ClockDomainId`/`ClockEpochId`/`InputStamp`/`ClockAnchor`/`EngineFrame` を再利用 | probe 専用語彙: OBS-05・audio-core の既存規則（`same_clock_domain`/`same_epoch`/`map_stamp_to_frame`）を再発明することになる |
| epoch authority | 非 RT mint → lifetime command で RT へ配る。未配線は `UNKNOWN` | RT 側自己採番: restart 検出不能・二重正本。device timestamp 直接利用: 未配線の現状で実装済みと仮定できない |
| loss 表現 | 非 RT で record 列から再構成（D7） | 型付き gap record を wire に追加: 導出可能な情報の重複、RT 帯域消費、counter 喪失時の blind spot は残る |
| alignment API | `ObservationSpan`+`AlignPolicy`→`Aligned/Conditional/Incomparable` | consumer 側個別実装: Issue が防ごうとしている誤比較が残る。自動 resample/PCM 相関: Issue が明示的に禁止 |

## Downstream Acceptance Criteria（Issue #309 実装受入 — 本 dispatch では実装しない）

- [ ] **DA-1**: 同 Clock/Epoch/Generation/Format で既知 Latency 差を持つ 2+ Probe を align し、実 Audio Fixture で expected offset/interval/Peak/RMS/gaps が一致する。
- [ ] **DA-2**: 別 Epoch/Seek/Loop/Device Restart、Generation 再構成、Sample Rate・Channel layout mismatch、Latency Unknown を `Incomparable`/`Conditional` として明示する。Clock 同期の証拠がある場合のみ条件付き対応。
- [ ] **DA-3**: 測定された無音と `Unobserved`/`Stale`/`LostBlocks`/`LostRecords`/Probe OFF/`Unsupported` を区別し、loss mask のある波形を完全連続 PCM として扱わない。
- [ ] **DA-4**: RT no alloc/lock/I/O、nonRT bounded comparison、f64 native 精度・AudioLens 既存 analysis/export/decoding/nonRT フローの非退行。
- [ ] **DA-5**: cross-consumer #219/#294 と Synvelis との意味的分界・全 OS 検証を記録する。

## Risks / Trade-offs

- **`RecordMeta` サイズ増**: `ProbeClockMark` 追加で全 record が数 byte〜十数 byte 増。telemetry 帯域への影響は実装時に計測（#104 PoC 系の手法）。影響が問題になる場合の退避案は「mark を telemetry record のみに載せ chunk は generation+series lookup」だが、chunk の自己証明性を失うため第一候補にはしない。
- **domain 採番の一意性**: 非 RT registry が払い出す `ClockDomainId` が media-core の well-known 値や device 側 domain と衝突しないことを実装時に保証する必要がある（namespace 分離 or registry 管理）。
- **epoch bump の網羅性**: device restart 等を検出して epoch を上げる経路が未配線の間は、同一 domain+epoch 内に「意味の異なる区間」が残りうる。これは `anchors` 不在→`EngineFrame` 基準比較が `Incomparable` になることで保守的に処理され、`SharedCursor` 基準の比較は「同一処理系列内の座標」として正しいまま — cursor 座標の意味論を「処理 frame 数」と明示することで誤用を防ぐ。
- **coverage 再構成の保守性**: telemetry 欠落時に `Unknown` へ降格するため、有効区間が実際より狭く報告されうる。これは「不明を有効と偽る」より安全側。

## Deferred Concerns

- device session callback への `OutputCallbackInfo`/`InputCallbackInfo` 配線と `StreamLatency` の probe context への反映（別 Issue — 本設計は `anchors`/`LatencyKnowledge` の受け皿のみ定義）。
- `EngineFrame`/media time anchor の生成を担う render driver / OBS-05 record bridge との接続実装（#280 側の消費設計）。
- PCM 波形差分の可視化・cause 候補推論（Synvelis #107/#109・Ruviform #292/#314）。
- `align_observation_points` の公開 API 名・型の最終確定（実装 dispatch で `probe::time` 等の module 配置と共に決定）。

## Verification Notes

- 本設計は文書のみ。実装時の検証は test-spec（`docs/test-specs/issue-309-probe.md`）の Rules に従い、`assert_no_alloc` 系の RT 検証を非退行とする。
- `docs/designs/overview.md` の probe 節を本設計の結果で更新する（`generation`+`sample_position` の整列記述を clock/epoch/instance identity 前提へ修正）。
