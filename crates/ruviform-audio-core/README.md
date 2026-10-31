# ruviform-audio-core

Ruviform メディア基盤の音声データ型クレート。処理・I/O・デバイス実装は持たない。

## 責務

Issue #18 で確定・実装済みのデータモデル API とデバイス境界ポート:

- `Sample` sealed trait（`f32`/`f64`）
- 音声時間契約: `SampleRate`, `SampleIndex`
- チャネルレイアウト・スピーカーマスク: `ChannelLayout`, `SpeakerMask`, `SpeakerMaskSet`
- ストリームレベルフォーマット記述: `AudioFormat`, `SampleFormat`
  （canonical `F32`/`F64` に加え、Issue #151 で native エンコーディング
  `U8`/`U16`/`U24`/`U32`/`S8`/`S16`/`S24`/`S32` を追加 — codec が native に
  デコードする実線フォーマットを同一 enum で表現）
- チャネルメジャー連続プレーナ `AudioBuffer` と借用ビュー:
  `AudioBufferView`, `AudioBufferViewMut`, `BufferSpec`
- チャネル独立プレーナ借用ビュー `PlanarBufferView` / `PlanarBufferViewMut` —
  プラグイン API・デバイスコールバック等の処理境界での非連結チャネルスライスリスト用
- デバイス境界ポート `device` モジュール — `ruviform-audio`（エンジン）と
  `ruviform-audio-device`（アダプタ）はこのポート経由で接続し、
  engine ↔ adapter の直接依存を構造的に排除する。ポートは
  `ruviform_audio_core::device::*` 名前空間に置き、クレートルートでは
  再エクスポートしない（多数の項目を一つの「境界」として束ねるため）。

正準処理フォーマットは planar `f32`。interleaved サンプル順は
デバイス/ファイル境界のみに存在する（`AudioBuffer::from_interleaved` /
`copy_to_interleaved` / `copy_from_interleaved`）。

## 層・許可依存

- 層: Domain foundations（設計ベースライン クレート表 A.2）
- 許可依存: Primitives（`ruviform-math`, `ruviform-rt`）+ `ruviform-media-core`。
  実際に使用中: `ruviform-media-core`（`TimeBase`, `MediaTime`, `Rounding`,
  `Error`/`Result`）。
- `ruviform-video-core` への依存は禁止（同階層依存禁止）。

## RT 契約

バッファビューとアクセサはアロケーションなし・非ブロッキングの借用 —
RT-suitable。アロケートするコンストラクタ（`AudioBuffer::new`,
`AudioBuffer::try_new`, `AudioBuffer::from_interleaved`）は
`**RT contract:** non-RT only` ラベル付き。バッファは構築後に成長しない
（`push`/`reserve` 系 API は存在しない）。API ごとの保証は各項目の
`**RT contract:**` 行に記載される（RT ADR の API ごと保証規則）。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 A.2、「音声データモデル」）
- RT 契約 ADR: docs/adr/realtime-contract-and-verification-policy.md
- デバイス境界ポート設計（Issue #23 PoC）: docs/designs/audio-device-boundary-and-poc.md
- 観測事実・非破壊 PCM 契約（Issue #151）: docs/adr/media-observation-non-destructive-pcm.md
