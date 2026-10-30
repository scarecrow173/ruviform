---
id: "SPEC-034TRm8kN4vWqZxYsJpCe"
type: "spec"
status: "implemented"
title: "Issue #151 メディア観測情報・非破壊 PCM 取得契約の拡張"
created: "2026-11-25"
updated: "2026-11-25"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/151"
  changes:
    added:
      - "crates/ruviform-media-core/src/fact.rs"
      - "crates/ruviform-media-core/src/observe.rs"
      - "crates/ruviform-io-symphonia/tests/observation_example.rs"
    modified:
      - "crates/ruviform-media-core/src/lib.rs"
      - "crates/ruviform-media-core/src/stream.rs"
      - "crates/ruviform-audio-core/src/format.rs"
      - "crates/ruviform-media-io/src/packet.rs"
      - "crates/ruviform-media-io/src/frame.rs"
      - "crates/ruviform-media-io/src/demux.rs"
      - "crates/ruviform-media-io/src/codec.rs"
      - "crates/ruviform-media-io/src/capability.rs"
      - "crates/ruviform-media-io/src/decode.rs"
      - "crates/ruviform-media-io/src/encode.rs"
      - "crates/ruviform-media-io/src/mux.rs"
      - "crates/ruviform-media-io/src/lib.rs"
      - "crates/ruviform-io-symphonia/src/demuxer.rs"
      - "crates/ruviform-io-symphonia/src/decoder.rs"
      - "crates/ruviform-io-symphonia/src/map.rs"
      - "crates/ruviform-io-symphonia/src/source.rs"
      - "crates/ruviform-io-symphonia/src/lib.rs"
      - "crates/ruviform-io-symphonia/tests/wav_decode_poc.rs"
      - "crates/ruviform-video/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/151"
  implemented-by: []
  depends-on:
    - "implement-media-io-abstraction-and-first-backend-poc.md"
    - "implement-media-data-model-api.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "https://github.com/scarecrow173/audio-lens/issues/95"
  refines: []
  refined-by:
    - "../adr/public-enum-non-exhaustive-policy.md"
  derives-from:
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/media-observation-non-destructive-pcm.md"
    - "../adr/realtime-contract-and-verification-policy.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/pdeljanov/Symphonia"
  defers: []
  deferred-by: []
---

# Issue #151 メディア観測情報・非破壊 PCM 取得契約の拡張

## What Should Be Built

Issue #95 の移行責務境界（Ruviform = 観測可能な基礎情報の供給、AudioLens = 観測・判定・ワークフロー）に基づき、Issue #20 のメディア I/O 境界を「精密オフライン解析の入力」として拡張する。

- **分類済み事実モデル `Fact<T>`（media-core・新規）**: 値あるいは欠落理由（`NotDeclared`/`Unsupported`/`Malformed`）を運ぶ `Copy` 可能な enum。`Option` が潰す「非宣言・非対応・不正」の区別を保持し、`Fact::default()` = `NotDeclared` で「未供給」が値に化けない。
- **タイムスタンプ provenance と生 tick 運搬（media-core・media-io）**: `TimestampProvenance::{Carried, Derived, Estimated}`、`TickTimestamp`（`i64` tick + provenance・time base 未確定でも観測可能）、`MediaTimestamp`。`Packet` に `index`・`pts`/`dts: Option<TickTimestamp>`・`duration` ticks・`trim_start`/`trim_end`（codec delay/padding）を追加し、`AudioFrame` は `MediaTimestamp` PTS を運ぶ。
- **フレームレート不問の観測計算 `observe_timing`（media-core・新規）**: 生 tick 系列から間隔分布（min/max/median/mean）・drift(ppm)・discontinuity・重複 index・孤立 drop 候補を報告する純粋計算。欠落・不連続の「解釈・異常判定」は利用側責務であり、本関数は事実のみを返す。
- **native サンプルエンコーディング（audio-core）**: `SampleFormat` に `U8/U16/U24/U32/S8/S16/S24/S32` を追加し、`F32`/`F64` と同一 enum で表現を一元化。
- **非破壊 PCM 経路（media-io）**: `NativeAudioFrame`（native 尺度の interleaved `f64` + native `AudioFormat` + `TickTimestamp` + `PcmTransform` 履歴）。`f64` 格納は全 ≤32bit 整数値と `f32`/`f64` を厳密保持。`PcmTransform::{IntegerWidenedToF64, Float32WidenedToF64}` は lossless、`Normalized/Quantized/Resampled` は lossy と分類。canonical `AudioFrame`（planar `f32`）は変更せず両立。
- **観測事実の同定（media-io）**: `ContainerInfo`/`ContainerFormat`/`ContainerDetail`、`BackendInfo`、`StreamFacts`（index・container 固有 id・`CodecTag`・言語・宣言原点 `start`・宣言 unit 数 `declared_units`・trim・`decodable`）、`VideoStreamFacts`、`CodecDescriptor::native_format`、`Features::NATIVE_AUDIO`、`Demuxer::container()`/`backend_info()`。
- **symphonia バックエンド（io-symphonia）**: `Track`/`AudioCodecParameters` の実フィールド（`time_base`, `start_ts`, `num_frames`, `delay`, `padding`, `language`, `sample_format`, `bits_per_sample`）を分類済み事実として写像。位置付き sniffing でコンテナ同定。`make_native_decoder()` で native 尺度 `f64` デコード経路を提供。`Packet` に index・trim・生 tick・provenance を載せる。

## Why Now

- AudioLens の `al-media`（wav.rs/timing.rs/avi.rs/mp4.rs）は現在ゲーム音声解析用の観測実装を自前で持ち、Issue #95 で「コンテナ解析/demux/decode と観測基礎情報は Ruviform が所有」が確定した。AudioLens が移行するためには、Ruviform 側の境界が必要な事実を供給できる必要がある。
- 既存の境界は `Option`/`MediaTime`/canonical `f32` 前提であり、欠落理由・生 tick・native 尺度・変換履歴を運べないため、AudioLens の解析入力として不完全。

## Users and Value

- **AudioLens（利用側）**: 境界型のみで「欠落・重複・不連続・ドリフト・原生 tick」系列と non-destructive PCM を収集し、Validity/Confidence/Provenance の判定へ渡せる（`observation_example` 統合テストが利用例）。
- **engines（`ruviform-audio`/`ruviform-video`）**: canonical `f32` 再生経路を保持しながら、時刻 base 未確定のストリームや生 tick を扱う解析入力を区別できる。
- **将来のバックエンド実装者**: 事実の分類と「捏造しない」規律が `Fact`/`StreamFacts`/`BackendInfo` で型レベルに示される参照実装として機能する。

## Scope

### In Scope

- `ruviform-media-core`: `Fact<T>`・`TimestampProvenance`・`TickTimestamp`・`MediaTimestamp`・`observe_timing`・`Discontinuity`/`IntervalStats`/`TimingObservation`。`StreamDescriptor::time_base` を `Fact<TimeBase>` に変更
- `ruviform-audio-core`: `SampleFormat` に native 整数・float エンコーディングを追加
- `ruviform-media-io`: `Packet` 拡張（index/生 tick/duration/trim）、`AudioFrame` の `MediaTimestamp` PTS、`NativeAudioFrame`/`PcmTransform`/`Frame::NativeAudio`、`CodecDescriptor::native_format`、`Features::NATIVE_AUDIO`、`ContainerInfo`/`ContainerFormat`/`ContainerDetail`/`BackendInfo`/`StreamFacts`/`CodecTag`/`VideoStreamFacts`、`Demuxer::container()`/`backend_info()`
- `ruviform-io-symphonia`: 事実写像・位置付きコンテナ sniffing・`make_native_decoder`・native 尺度 `f64` デコード・`Packet` の index/trim/provenance
- 既存 consumer の整合修正（media-io テストスタブ・`ruviform-video` テストモック）
- コードテスト・ドキュメント・ADR の差分反映

### Out of Scope

- AudioLens 側の実装変更（AudioLens は参照専用・変更しない）
- 動画 decode の実装（`VideoStreamFacts` は型のみ・動画バックエンドは後続 Issue）
- mux/encode のバックエンド実装（symphonia は decode のみ・`NATIVE_AUDIO` は decode 能力として申告）
- 複数トラック・独立クロックを持つ実コンテナ fixture（WAV は単一ストリームのみ。マルチトラック経路の検証は後続バックエンド Issue）
- フレームプール・zero-copy backing・mux 側契約の実証（Issue #20 の deferred concerns を継続）

## Requirements

- Rust edition 2024・MSRV（workspace `rust-version`）。`unsafe` 不使用。`publish = false`（workspace 継承）
- **事実は捏造しない**: コンテナ非宣言・バックエンド非対応・不正値は `Fact`/`Option` で表現し、`0`/デフォルト値で埋めない。`CodecDescriptor::codec_private` は opaque のまま verbatim 保持
- **RT/非 RT 契約の維持**: `ruviform-media-io` は非 RT 専用のままとし、データ型（`Packet`, `Frame`, `StreamInfo`, `ContainerInfo`, `Error`）は `Send`。media-core の `Fact<T: Copy>`/`TickTimestamp`/`MediaTimestamp` は `Copy` で RT-suitable、`observe_timing` は allocate するため non-RT only と明示
- **両経路の併存**: canonical planar `f32` の `AudioFrame` と native 尺度の `NativeAudioFrame` は別経路として併存し、精密入力を強制的に `f32` に縮約しない
- **外部型非露出**: symphonia の `Track`/`AudioCodecParameters`/`AudioBufferRef`/error は公開 API に露出せず、全て境界内で写像・正規化する（media-io ADR）
- **ストリーミング**: 全量読込を要求しない — `read_packet` 単位の逐次取得で観測・native decode が完了する（`observation_example` が単一パスの利用例）
- **公開 enum**: クレート外到達可能な全公開 enum に enum レベル `#[non_exhaustive]`（`Fact`, `TimestampProvenance`, `PcmTransform`, `ContainerFormat`, `ContainerDetail`, `CodecTag`, `Frame`）
- **検証方法**: in-memory WAV fixture（byte 構築・LFS 不使用）。観測・厳密性・非対応の各検証をコードテストで実施する

## Acceptance Criteria

- [ ] **AC1**: `Fact<T>` が `Known`/`NotDeclared`/`Unsupported`/`Malformed` を区別し、`default()` = `NotDeclared`。`0`/既定値と混同しないことが単体テストで検証される
- [ ] **AC2**: `Packet` が `index`・生 `pts`/`dts`（`TickTimestamp` + provenance）・`duration` ticks・`trim_start`/`trim_end` を運び、`AudioFrame` が `MediaTimestamp` PTS を運ぶ
- [ ] **AC3**: `observe_timing` が CFR・VFR・ギャップ・重複・原点オフセット・負間隔・空/単一系列を検証し、欠落・不連続・drift の事実をフレームレート不問で報告する
- [ ] **AC4**: `SampleFormat` が `U8`/`U16`/`U24`/`U32`/`S8`/`S16`/`S24`/`S32`/`F32`/`F64` を持ち、≤32bit 整数と `f32`/`f64` が `f64` に厳密保持されることをコードテストで検証する
- [ ] **AC5**: `NativeAudioFrame` が native 尺度の interleaved `f64` + `TickTimestamp` + `PcmTransform` 履歴を持ち、`is_lossy` で変換の損失を観測できる。`U8`/`S16`/`S24`/`S32`/`F32`/`F64` エンコーディングで e2e 厳密性検証済み
- [ ] **AC6**: `StreamInfo` が `StreamFacts`（index・container 固有 id・codec tag・言語・宣言原点・宣言 unit 数・trim・`decodable`）を持ち、`CodecDescriptor` が `native_format: Fact<CodecFormat>` を持つ
- [ ] **AC7**: `Demuxer` が `container()`/`backend_info()` を報告し、コンテナ同定が実 sniffed bytes からの家族分類（WAV/AVI/ISOBMFF/Ogg/Matroska/FLAC/Other）で行われる
- [ ] **AC8**: AudioLens 型利用例（境界型のみで生 tick 収集・観測・native decode）が統合テストとして存在する
- [ ] **AC9**: `mise run fmt-check` / `check` / `lint` / `test` と `mise exec -- prek run --all-files` がグリーン。`unsafe` 不使用
- [ ] **AC10**: 全公開 API に `**RT contract:**` ラベル、全公開 enum に `#[non_exhaustive]`、公開 API に symphonia 型・エラー・feature 名が露出しない
- [ ] **AC11**: README / 設計ベースライン / spec / ADR の記述が実装と一致する

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 判断・移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| 複数トラック・独立クロック・実コンテナでの VFR 検証 | `StreamId` キーの系列分離と `observe_timing` の契約は確定。実コンテナ fixture（WAV は単一ストリームのみ・symphonia の読める複数トラック形式は限定的）は本 spec では持たない | 後続の動画バックエンド Issue（FFmpeg 等で複数トラック・VFR 実データを読む） | 複数トラック経路が抽象だけで未検証のまま残る |
| `Packet` の duration が常に ticks であることの codec 一般性 | symphonia は `Duration`（u64）で供給し `u64` に写像済み | 後続バックエンド（FFmpeg 等）で単位の一般性を検証 | ticks 前提が codec 一般で破れる場合 API 改訂 |
| `NativeAudioFrame` の interleaved 表現の性能影響 | 解析入力（非 RT 専用）として許容。planar f64 等の代替は実測後に評価 | 非 RT 層の後続設計 | 大容量入力での allocation が要件とずれる |
| mux/encode 側の native/format 拡張 | `Frame::NativeAudio` は decode 経路のみ。mux 側の `Packet`・`CodecDescriptor` は同じ型を共有するが、encode 側ネイティブ経路は未検証 | 第一 encode 対応バックエンド Issue | 書き出し側契約が未検証のまま残る |
| `BackendInfo::version` の供給源 | symphonia 0.6.1 の静的版文字列を backend クレートが pin | 後続 Issue（バックエンド版を機械的に集約するなら） | バックエンド版の表記揺れ |

## Sources and Relations

- 一次ソース: GitHub Issue [#151 高忠実度メディア観測情報・非破壊PCM取得契約を拡張する](https://github.com/scarecrow173/ruviform/issues/151)（`relations.source`/`implements`）。AudioLens Issue [#95](https://github.com/scarecrow173/audio-lens/issues/95) の責務移行境界が前提。
- 上流文書（`relations.derives-from`）: [設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（「メディア I/O 抽象」・クレート表・スレッドモデル）、[media-io ADR](../adr/media-io-backend-abstraction.md)（外部型非露出・分離クレート）、[観測契約 ADR](../adr/media-observation-non-destructive-pcm.md)（本 spec の確定決定）、[RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)（API 単位の RT 保証ラベル・非 RT 専用の明示）。
- 前提（`relations.depends-on`）: spec [implement-media-io-abstraction-and-first-backend-poc.md](implement-media-io-abstraction-and-first-backend-poc.md)（Issue #20 の境界確定・`Packet`/`AudioFrame`/`StreamInfo` の既存契約）、spec [implement-media-data-model-api.md](implement-media-data-model-api.md)（`StreamDescriptor`/`MediaTime`/`AudioFormat`/`SampleFormat` の既存データモデル）。
- 適用ポリシー（`relations.refined-by`）: [公開 enum ADR](../adr/public-enum-non-exhaustive-policy.md)。
- 関連（`relations.related`）: [`AGENTS.md`](../../AGENTS.md)（unsafe 原則禁止・公開 enum 規約・暫定開発規約）。
- 参考（`relations.references`）: [Symphonia](https://github.com/pdeljanov/Symphonia)（参照実装バックエンド）。
