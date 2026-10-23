# ruviform-io-symphonia

`ruviform-media-io` 抽象境界の最初のバックエンド PoC（Issue #20）:
pure-Rust バックエンド（symphonia 0.6・default features のみ・MPL-2.0）上の
demux + decode のみの実装。

## 責務

- バックエンド抽象の参照実装: 境界の `Demuxer` と `Decoder` を実装する
  （`SymphoniaDemuxer`, `SymphoniaDecoder`）。`Muxer`/`Encoder` は能力交渉経由で
  表現 — query 側 `capabilities()` は demux/decode のみ報告し、request 側
  `make_muxer()`/`make_encoder()` は正規化 `ErrorKind::Unsupported` を返す。
- 公開入口: `version()`, `capabilities()`, `open_demuxer()`, `make_muxer()`,
  `make_encoder()`。
- 型純度: 公開面は Ruviform 抽象のみ。バックエンドの型・エラー・feature 名は
  公開シグネチャに一切現れない（依存は非公開の実装詳細、エラーは正規化
  `Error` へ写像し詳細は type-erased `source` のみ、外部型の再エクスポートなし）。
- バックエンド feature 範囲: default features のみ — ロイヤリティフリーの
  codec/format 集合（WAV/PCM, FLAC, Vorbis/Ogg, Matroska, ADPCM）。
  `capabilities()` は意図的に控えめに申告し、boundary `CodecId` レジストリが
  名付け *かつ* default ビルドが実際に登録する codec のみを挙げる
  （`Pcm`, `Flac`, `Vorbis`）。
- PoC の既知制限（隠さずフォローアップとして記録）: 連結 Ogg は先頭リンクで
  終了、gapless trim メタデータは境界を越えない、シーク不可ソースは
  構築時プローブで正直に報告し `Demuxer::seek` は `ErrorKind::Unsupported`。

## 層・許可依存

- 層: メディア I/O バックエンド（設計ベースライン クレート表 — opt-in・分離クレート）
- 許可依存: `ruviform-media-io`（抽象境界）+ ドメインクレート直接依存
  （`ruviform-media-core`, `ruviform-audio-core` — media-io がドメイン型を
  再エクスポートしないため、ベースラインがバックエンドクレートに許容する
  直接 path 依存）+ バックエンド `symphonia`（非公開の実装詳細）。

## RT 契約

**非 RT 専用。** クレート全体がスレッド境界の I/O／デコード側に置かれる:
プローブ・パケット読み出し・シーク・デコードは I/O でブロックし得、確保し得、
無界のコーデック処理を行い得る。API ごとのラベルは
`**RT contract:** non-RT only`。境界を越えるデータ型（`Packet`, `Frame`,
`StreamInfo`, `Error`）は `Send` であり、有界キュー経由で処理側へ移送できる。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（メディア I/O バックエンド行）
- 抽象境界 ADR: docs/adr/media-io-backend-abstraction.md
- Issue #20（第一バックエンド PoC）: docs/impl/ir/implement-io-symphonia-backend-poc.md
