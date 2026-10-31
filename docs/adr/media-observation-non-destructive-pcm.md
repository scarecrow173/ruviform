---
id: "ADR-034TRn2qP9wXkLmVcYbJdE"
type: "adr"
status: "accepted"
title: "メディア観測事実の分類モデルと非破壊 PCM 取得経路（Issue #151）"
created: "2026-11-25"
updated: "2026-11-25"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/151"
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
    - "media-io-backend-abstraction.md"
    - "realtime-contract-and-verification-policy.md"
    - "public-enum-non-exhaustive-policy.md"
    - "https://github.com/scarecrow173/audio-lens/issues/95"
  refines: []
  refined-by: []
  derives-from:
    - "media-io-backend-abstraction.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "https://github.com/scarecrow173/ruviform/issues/151"
  derived-by:
    - "../specs/issue-151-media-observation-contract.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/pdeljanov/Symphonia"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# メディア観測事実の分類モデルと非破壊 PCM 取得経路（Issue #151）

## Context and Problem Statement

Issue #95 で確定した AudioLens → Ruviform の責務移行境界に基づき、AudioLens が必要とするメディア観測情報（コンテナ/トラック/コーデック同定、PTS/DTS 生系列、タイムスタンプ原点、native サンプルフォーマット、codec delay/padding）を Ruviform の抽象境界から供給する。Issue #20 の境界は再生経路（canonical planar `f32`・`MediaTime` 前提）を確定したが、精密オフライン解析に必要な「事実そのまま」の運搬には 3 つの欠陥があった:

1. 欠落事実と `0`/デフォルト値の区別がつかない（`Option` は理由を運ばず、存在しない time base は捏造の余地を残す）。
2. タイムスタンプが `MediaTime` 固定で、time base が取れないストリームの生 tick が境界を越えられず、carried / derived / estimated の区別も失われる。
3. 音声デコードが canonical planar `f32` 正規化しか持たず、整数 PCM の元の尺度・値が失われ、変換履歴も観測できない。

観測・Validity/Confidence/Provenance の判定（欠落・不連続の解釈、異常ラベル）は AudioLens 側の責務であり、Ruviform は「分類済みの事実」を供給する境界を持つ。

## Decision Drivers

* 不正・不完全メタデータを `unknown`/`unavailable` で表現でき、存在しない事実を捏造しないこと（Issue #151 制約）
* carried（コンテナ記録値）と derived/estimated（導出・推定値）の区別が境界を越えて維持されること
* 整数 PCM の元の尺度と値・`f32` 正規化を避けられる raw/高精度経路があり、変換過程と精度損失が明示されること
* 大容量ファイルの全量読込を要求しないストリーミング/逐次取得
* 既存の canonical planar `f32` 再生契約と両立し、RT/非 RT 契約を維持すること
* 外部ライブラリ（symphonia）の型・列挙を公開 API に露出しないこと（media-io ADR）

## Considered Options

* **欠落事実の表現**: (a) `Option<T>` のみで運ぶ、(b) `Option` + 各フィールド個別のフラグ、(c) 分類 enum `Fact<T>`（`Known` / `NotDeclared` / `Unsupported` / `Malformed`）
* **生タイムスタンプの運搬**: (a) `MediaTime` のまま未知 base を `0` で埋める、(b) `Option<MediaTime>` + 別途 raw 値、(c) `TickTimestamp`（生 tick + provenance）を `Packet`/native frame に載せる
* **非破壊 PCM 経路**: (a) `AudioFrame` のサンプル型をジェネリック化、(b) native エンコーディング別のビュー型を増やす、(c) native 尺度の interleaved `f64` を運ぶ `NativeAudioFrame` を別型として追加（`f64` は全 ≤32bit 整数と `f32`/`f64` を厳密保持）
* **変換履歴**: (a) 記録しない、(b) `PcmTransform` の列挙履歴で変換と損失を明示

## Decision Outcome

以下の組み合わせを採用する。

* **`Fact<T>`（media-core）**: `Option` が潰す欠落理由を `NotDeclared`（コンテナ非宣言）/`Unsupported`（バックエンド非対応）/`Malformed`（宣言値が不正）に分類する。`Fact::default()` は `NotDeclared`。`Fact<T: Copy>` は `Copy` で RT-suitable 記述子に埋め込める。`StreamDescriptor::time_base` を `Fact<TimeBase>` に変更し、未知の base を捏造できない構造にする。
* **タイムスタンプ provenance + 生 tick 運搬（media-core / media-io）**: `TimestampProvenance::{Carried, Derived, Estimated}`、`TickTimestamp`（`i64` tick + provenance、time base 未解決でも観測可能）、`MediaTimestamp`（`MediaTime` + provenance）。`Packet` は `pts`/`dts` を `Option<TickTimestamp>`（生 tick・index・duration ticks・`trim_start`/`trim_end`）として運び、`AudioFrame` は `MediaTimestamp` PTS を運ぶ。base 未確定でも事実が失われず、推定値と記録値を混同しない。
* **観測計算 `observe_timing`（media-core）**: 生 tick 系列から間隔分布・drift(ppm)・discontinuity・重複・孤立 drop 候補をフレームレート不問で報告する純粋計算。「不連続の解釈・異常判定」は利用側の責務として残す（ストリーム別系列として観測し、独立クロックの混同を防ぐ）。
* **native サンプルフォーマット（audio-core）**: `SampleFormat` を `U8/U16/U24/U32/S8/S16/S24/S32/F32/F64` の実線エンコーディングへ拡張（既存 `F32`/`F64` と同一 enum で表現を一元化）。
* **非破壊 PCM 経路（media-io）**: `NativeAudioFrame` — native 尺度の interleaved `f64` + native `AudioFormat` + `TickTimestamp` + `PcmTransform` 履歴。`f64` 格納は全 ≤32bit 整数値と `f32`/`f64` ソースに対し厳密。`PcmTransform::{IntegerWidenedToF64, Float32WidenedToF64}` は lossless、`Normalized/Quantized/Resampled` は lossy と分類される。canonical `AudioFrame`（planar `f32`）経路は変更せず両立する。
* **観測事実の同定（media-io）**: `ContainerInfo`/`ContainerFormat`/`ContainerDetail`、`BackendInfo`（provenance: バックエンド名+版）、`StreamFacts`（index・container 固有 id・codec tag・言語・宣言原点 `start`・宣言 unit 数・trim・`decodable: Fact<()>`）、`CodecTag`（fourCC/数値タグを verbatim 保持）、`CodecDescriptor::native_format: Fact<CodecFormat>`、`Features::NATIVE_AUDIO` 能力フラグ。`Demuxer` に `container()`/`backend_info()` を追加。
* **符号なし整数の約定**: `U8`/`U16`/`U24`/`U32` は符号オフセット（wire 値 − 半幅）として報告し、無音 `U8` は `0.0` とするデコード音声慣行に揃える。

### Consequences

* 良い点: 欠落理由が型で保持され `0`/デフォルトと混同しない。base 未知でも生 tick 系列が境界を越える。整数 PCM の元の尺度・値が失われず、変換と損失が履歴として観測できる。AudioLens は境界型のみで観測・判定を組める（`observation_example` テストが利用例）。
* 悪い点: `Packet`/`AudioFrame`/`CodecDescriptor`/`Demuxer` の既存利用者（テストモック含む）はフィールド追加・シグネチャ変更の修正を要する（pre-release 方針どおり互換層は設けない）。`Fact` の分類はバックエンド実装者が正直に維持する必要があり、悪意ある分類誤記は構造的に防げない。

## Implementation Plan

* 影響するパス: `crates/ruviform-media-core`（`fact.rs`, `observe.rs`, `stream.rs`）、`crates/ruviform-audio-core`（`format.rs`）、`crates/ruviform-media-io`（`packet.rs`, `frame.rs`, `demux.rs`, `codec.rs`, `capability.rs`）、`crates/ruviform-io-symphonia`（事実の実装・位置付き sniffing・native decode）
* 従うパターン: 事実は分類を保持し `Option`/デフォルト値で潰さない。外部型は境界の内側に留め、symphonia の `Track`/`AudioCodecParameters`/`AudioBufferRef` は `map.rs`/`decoder.rs` でドメイン型へ写像する。逐次 `read_packet` + 単一パスの観測・デコードで全量読込を要求しない
* 避けるパターン: コンテナ非宣言の値を `0`/既定で埋めない。`MediaTime` に未知 base を縫い込まない。canonical `f32` 経路を native 経路の前提にしない
* 依存関係: `ruviform-media-io` → `ruviform-media-core`/`ruviform-audio-core`/`ruviform-video-core` の既存依存方向を維持。symphonia 型の露出なし

## Pros and Cons

**`Fact<T>` 分類 enum（採用）:**

* 良い点: 欠落理由が `Copy` な単一表現に集約され、`map` で分類を保持したまま値を変換できる。RT-suitable 記述子に埋め込める
* 悪い点: `Option` より記述が冗長。分類維持はバックエンドの規律に依存する

**`Option` + 個別理由フラグ:**

* 良い点: 既存 API 形状に近い
* 悪い点: 値と理由が別フィールドに分かれ不整合（値あり・理由あり等の不正組合せ）が可能になる

**`NativeAudioFrame`（f64 格納・採用）:**

* 良い点: 全対象エンコーディングを厳密に保持する単一表現。変換履歴で損失を明示できる。利用側はエンコーディング別の型分岐を持たない
* 悪い点: メモリがサンプルあたり 8 バイト。RT 用途には不向き（非 RT 専用の境界であり許容）

**エンコーディング別ビュー型:**

* 良い点: 省メモリ
* 悪い点: 型の爆発と利用側の分岐増加。`f64` の厳密保持で表現を一元化する目的に反する

## Verification

* [ ] `Fact` の欠落理由（`NotDeclared`/`Unsupported`/`Malformed`）が区別され、`0`/デフォルトと混同しないことが単体テストで検証される
* [ ] `observe_timing` が CFR/VFR/ギャップ/重複/原点オフセット/負間隔/空・単一系列をコードテストで検証される
* [ ] U8/S16/S24/S32/F32/F64 の native decode が値の厳密性（exact equality）で検証される
* [ ] AudioLens 型利用例（境界型のみで生 tick 収集 + 観測 + native decode）が統合テストとして存在する
* [ ] `Packet`/`AudioFrame`/`CodecDescriptor`/`Demuxer` の変更を全 consumer で検証（`mise run check`/`test` グリーン）
* [ ] 公開 API に symphonia 型・列挙・feature 名が露出しない

## More Information

* 一次ソース: GitHub Issue [#151 高忠実度メディア観測情報・非破壊PCM取得契約を拡張する](https://github.com/scarecrow173/ruviform/issues/151)
* 移行境界: AudioLens Issue [#95](https://github.com/scarecrow173/audio-lens/issues/95)、`al97-migration-adr` worktree の responsibility-map（AudioLens が観測・判定を担い、Ruviform が分類済みの事実を供給する）
* 関連 ADR: [media-io バックエンド抽象](media-io-backend-abstraction.md)（外部型非露出・分離クレート）、[RT 契約](realtime-contract-and-verification-policy.md)（非 RT 専用の明示）、[公開 enum ポリシー](public-enum-non-exhaustive-policy.md)
* 実装 spec: [issue-151-media-observation-contract.md](../specs/issue-151-media-observation-contract.md)
