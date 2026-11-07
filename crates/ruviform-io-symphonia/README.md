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
  `make_encoder()`。`open_demuxer` 返却の `SymphoniaDemuxer` が `make_decoder()`
  （canonical planar `f32`）と `make_native_decoder()`（Issue #151 の非破壊
  native 尺度 `f64` 経路）を持つ。
- 型純度: 公開面は Ruviform 抽象のみ。バックエンドの型・エラー・feature 名は
  公開シグネチャに一切現れない（依存は非公開の実装詳細、エラーは正規化
  `Error` へ写像し詳細は type-erased `source` のみ、外部型の再エクスポートなし）。
- バックエンド feature 範囲: default features のみ — ロイヤリティフリーの
  codec/format 集合（WAV/PCM, FLAC, Vorbis/Ogg, Matroska, ADPCM）。
  `capabilities()` は意図的に控えめに申告し、boundary `CodecId` レジストリが
  名付け *かつ* default ビルドが実際に登録する codec のみを挙げる
  （`Pcm`, `Flac`, `Vorbis`）。
- opt-in feature（Issue #154）:
  - `mp4` → `symphonia/isomp4`。ISOBMFF コンテナ demux（PCM-in-MP4
    audio・AAC・video track の sample table / 宣言 facts を含む）。
    feature なしで MP4 を probe すると `ErrorKind::Unsupported`。
  - `aac` → `mp4` + `symphonia/aac`。AAC decode を
    `capabilities()` の decode codec へ追加する（特許係属 codec は
    配布者判断で切替えるため `mp4` 単体では有効化しない — ライセンス ADR）。
    AAC ビットストリーム実体の decode 検証は実 fixture が必要で、
    本クレートの generated fixture 範囲外（capability/demux 経路のみ
    検証済み）。
- video facts（Issue #154）: video track は `StreamFacts.video` に
  宣言された pixel 寸法を運び、`decodable: Fact::Unsupported`。
  `nominal_frame_rate` は MP4 が宣言 frame rate を持たない（`stts`
  の per-sample duration のみ）ため `None` のまま — VFR は packet
  tick 列で観測される。
- 観測事実（Issue #151）: `Track`/`AudioCodecParameters` の実フィールドを
  `StreamFacts`/`CodecDescriptor`/`ContainerInfo`/`BackendInfo` へ写像し、
  位置付き sniffing でコンテナ家族を同定。`Packet` は index・生 tick
  （`TimestampProvenance` 付き）・duration ticks・`trim_start`/`trim_end`
  を運ぶ。`capabilities()` は `Features::NATIVE_AUDIO` を含めて申告する
- PoC の既知制限（隠さずフォローアップとして記録）: 連結 Ogg は先頭リンクで
  終了、シーク不可ソースは構築時プローブで正直に報告し
  `Demuxer::seek` は `ErrorKind::Unsupported`。
- 借用ソース入口（Issue #400）: `with_borrowed_demuxer(&mut source, f)` —
  借用 `Read + Seek`（`SealedFile::open_readback` の `!Send` な
  `ReadbackFile` を含む）から、encoded 全量コピーなしに所有型
  `Demuxer` を駆動するスコープ付きブリッジ。呼び出し側スレッドが
  `serve` ループとして `&mut source` を保持し、spawn した 1 本の
  worker が `sync_channel(1)` 背後の proxy 経由で `read`/`seek` を
  逐次フォワードする — 飛行中の encoded バイトは単方向あたり
  ≤64KiB の 1 チャンクに有界。`f` は worker スレッド上で
  `&mut dyn Demuxer` を受け取り、その借用は `f` の外へ escape
  できない（box しても `Box<dyn Demuxer + '_>` は scope 寿命に
  束縛され、owned 化・偽 demuxer への swap・`R` 経由の return は
  いずれも不可）。`f` が作る所有値
  （`Packet`・`Frame`・`Box<dyn Decoder>`・その借用上の
  `SourceReader` が生んだ decoded 出力）は借用を含まなければ
  `R: Send` 経由で返せる。制約: `f` は worker スレッドで走り
  `Send` な captures のみ持てる・呼び出し側スレッドは `f` の間
  serve に専有される・`f` が呼び出し側スレッド上の処理を待つと
  デッドロック・ソースの最終位置は未規定。チャンネル切断は常に
  `io::Error`（`BrokenPipe`）として伝搬し、clean EOF には化けない。

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
- Issue #151（観測・非破壊 PCM 契約）: docs/adr/media-observation-non-destructive-pcm.md, docs/specs/issue-151-media-observation-contract.md
