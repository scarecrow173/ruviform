# ruviform-media-core

Ruviform メディア基盤の共有メディア値型クレート。処理・I/O は持たない。

## 責務

Issue #18 で確定・実装済みのデータモデル API:

- 有理数時間: `TimeBase`, `MediaTime`, `MediaDuration`, `MediaRange`, `FrameRate`, `Rounding`
- コンテナレベルのストリーム記述子: `StreamId`, `StreamKind`, `StreamDescriptor`
- メディア共通 `Error` / `Result`
- 表示側 `Timecode` 変換（SMPTE ドロップフレームラベリング含む）
- トランスポート状態: `TransportStatus`, `TransportSnapshot`
- 音楽時間: `Tick`, `TempoMap`, `MusicalTimeInfo` — `PPQN` tick 座標系で、
  コンパイル済みテンポマップ経由で `MediaTime` と変換可能（Issue #90）
- 観測事実モデル（Issue #151）: `Fact<T>` — `Known`/`NotDeclared`/
  `Unsupported`/`Malformed` の分類で欠落理由を保持し、`0`/デフォルト値と
  混同しない。`StreamDescriptor::time_base` は `Fact<TimeBase>`
- タイムスタンプ provenance（Issue #151）: `TimestampProvenance`,
  `TickTimestamp`（生 tick・time base 未確定でも観測可能）, `MediaTimestamp`
- フレームレート不問の観測計算 `observe_timing` — 生 tick 系列の間隔
  分布・drift・discontinuity・重複・孤立 drop 候補を報告する純粋計算
  （不連続の「解釈・異常判定」は利用側責務）

## 層・許可依存

- 層: Primitives（設計ベースライン クレート表 A.2）
- 許可依存: Primitives（`ruviform-math`, `ruviform-rt`）。現時点ではどちらも未使用 —
  値型は `core`/`std` のみで足りる。
- `ruviform-exec` との相互依存は禁止（同階層依存禁止）。

## RT 契約

全型は `Copy` 値型: アロケーションなし・非ブロッキングで、全変換は純粋整数関数
（`i128`/`u128` 中間演算の正確な有理数演算 — `f64` は格納・コア表現に使わない）。
`Fact<T: Copy>`/`TickTimestamp`/`MediaTimestamp` も `Copy` で RT-suitable。
`observe_timing` は allocate するため `**RT contract:** non-RT only`。
API ごとの保証は各項目の `**RT contract:**` 行で文書化される
（RT ADR の API ごと保証規則）。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 A.2、「時間モデル」）
- RT 契約 ADR: docs/adr/realtime-contract-and-verification-policy.md
- 音楽時間モデル（Issue #90）: docs/designs/music-time-model.md, docs/adr/musical-time-model.md
- FrameRate・タイムコード拡張: docs/designs/time-model-extension.md, docs/adr/framerate-timebase-separation-and-timecode-extension.md
- 観測事実・非破壊 PCM 契約（Issue #151）: docs/adr/media-observation-non-destructive-pcm.md, docs/specs/issue-151-media-observation-contract.md
