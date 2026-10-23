# ruviform-audio-device

オーディオデバイス I/O アダプタ: `ruviform_audio_core::device` の
デバイス境界ポートを `cpal` バックエンド上に実装するクレート（Issue #23 PoC）。

## 責務

- cpal をリンクする唯一の場所: cpal の型・エラー・feature 名はこのクレートの
  公開面に一切現れない。`Host`/`HostId` がバックエンドハンドルを非公開で包み、
  デバイス・ストリームはポートの `Box<dyn Device>` / `Box<dyn OutputStream>`
  trait オブジェクトとしてのみ呼び出し側へ届き、全バックエンドエラーはポートの
  `DeviceError` に正規化される。第 2 のバックエンド（cubeb 等）は feature
  スイッチではなく*別のアダプタクレート*になる。
- Host 列挙・選択: `default_host`, `available_host_ids`, `host`。
- フォーマット交渉（第一責務）: 開ける `F32` `AudioFormat` を優先 —
  バックエンド既定が別エンコーディングでも F32 対応レンジのみを
  対応空間として報告する。
- コールバック配管: エンジンは open 時に一度確保したスクラックスラブ上の
  有界 `AudioBufferViewMut` 経由で planar `f32` を書き込み、アダプタが
  バックエンド境界の interleaved へ詰め替える — `f32` interleaved は
  バックエンド向けエッジのみに存在する。
- 手動検証 example（`list_devices`, `play_sine`）は実機依存で CI 対象外。

## 層・許可依存

- 層: Optional platform adapters（設計ベースライン — ドメイン基盤の外側、
  engine と同じ利用側層。foundations/Primitives への下向き依存のみ）
- 許可依存: `ruviform-audio-core`（ポート定義元）, `ruviform-media-core`
  （デバイスクロックの `MediaTime`）, `cpal`（バックエンド・封じ込め）。
  エンジンクレート（`ruviform-audio`）にも他アダプタにも依存しない。

## RT 契約

列挙・フォーマット照会・open/start/stop・`now`・`buffer_size` は制御パス API —
非 RT（OS API を呼び、確保・ブロックし得る）。出力データコールバックは
バックエンドの RT スレッド上で動く: 構造上アロケーションなし・有界
（open 時スクラッチ + コールバックごとの借用、データ/エラークロージャ間に
`Arc`/`Mutex` なし）。API ごとの保証は `**RT contract:**` 行に記載。
CI 成功は RT 安全性の完全な証明ではない（realtime ADR）。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 `ruviform-audio-device` 行）
- デバイス境界・PoC 設計（Issue #23）: docs/designs/audio-device-boundary-and-poc.md
- バックエンド選定 ADR: docs/adr/select-audio-device-backend.md
- チャンク分割タイムスタンプ ADR（Issue #59）: docs/adr/chunk-outputcallbackinfo-timestamp.md
