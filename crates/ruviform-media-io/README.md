# ruviform-media-io

Ruviform メディア基盤のバックエンド非依存メディア I/O 抽象境界クレート（Issue #20）。

## 責務

コンテナ demux/mux とコーデック decode/encode は**分離された trait 族**であり、
単一の read/write 抽象に統合しない（media-io ADR、ベースライン「メディア I/O 抽象」）:

- `Demuxer` — コンテナ → `Packet`（ストリーム列挙・パケット読み出し・
  時間ベースのシーク*要求*）
- `Muxer` — `Packet` → コンテナ（未検証: 実装者なし）
- `Decoder` — `Packet` → `Frame`（`send`/`receive` ポンプ +
  `flush` 残留状態リセット）
- `Encoder` — `Frame` → `Packet`（未検証: 実装者なし）

周辺の境界値型: `Packet`/`PacketFlags`, `AudioFrame`/`Frame`,
codec レベル記述 `CodecDescriptor`/`CodecId`/`CodecFormat` を持つ `StreamInfo`,
入力ポート `MediaSource`, 能力交渉（query 側）`Capabilities`/`Features`,
正規化エラーモデル `Error`/`Result`/`ErrorKind`。

本クレートは**バックエンド実装を持たない**: 全 trait は opt-in 依存として選ばれる
個別 `ruviform-io-*` クレートが実装し、外部（バックエンド）の型・エラー・
feature 名はこの公開 API に一切現れない — 表面は Ruviform 値型
（`ruviform-media-core` / `-audio-core` / `-video-core`）と `std` 型のみ。
ドメイン型は*使用*するが再エクスポートはしない — バックエンドクレートは
ドメインクレートへ直接 `path` 依存を取る。

## 層・許可依存

- 層: Domain foundations（設計ベースライン クレート表 — 処理・境界の段）
- 許可依存: `ruviform-media-core`, `ruviform-audio-core`, `ruviform-video-core`
  のみ（ベースライン クレート表）。外部クレート依存なし・バックエンドクレート
  なし — `cargo tree -p ruviform-media-io` はこの 3 本のエッジのみを示す。

## RT 契約

**非 RT 専用。** クレート全体が I/O／デコードスレッド境界の内側に置かれる
（ベースライン スレッドモデル節）: 全 trait メソッドは I/O でブロックし得、
確保し得、無界のコーデック処理を行い得るため、RT パスからの呼び出しは
契約違反。データ型は `Send` — パケット・フレーム・記述子は有界キュー経由で
I/O/デコード側から処理側へ移送される。API ごとのラベルは
`**RT contract:** non-RT only`。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表、「メディア I/O 抽象」、スレッドモデル）
- 抽象境界 ADR: docs/adr/media-io-backend-abstraction.md
- RT 契約 ADR: docs/adr/realtime-contract-and-verification-policy.md
- Issue #20（抽象境界確定・実装）
