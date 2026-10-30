# ruviform-io-avi

`ruviform-media-io` 抽象境界の AVI バックエンド（Issue #154）:
外部メディアフレームワークに依存しない pure-Rust の RIFF/AVI demuxer と、
WAVEFORMATEX PCM audio・非圧縮 DIB video の decoder。

## 責務

- RIFF/AVI コンテナの probe + demux: `RIFF`/`AVI ` 検証、`LIST hdrl`・
  `LIST strl`（`strh`+`strf`）の解析、`LIST movi` の遅延スキャン、
  `00dc`/`01wb` 等の digit-prefixed stream chunk ID のストリーム対応付け。
- ストリーム分類: `vids` → video、`auds` → audio。壊れたヘッダは
  破棄せず `Fact::Malformed` として表現する。
- decode:
  - audio: WAVEFORMATEX PCM(`0x0001`)・IEEE float(`0x0003`) を
    canonical planar `f32` へ。native 経路では元の `SampleFormat`・
    元尺度の interleaved `f64` + `PcmTransform` 履歴を保持する。
  - video: `BITMAPINFOHEADER` DIB（24bit BGR / 32bit BGRA、bottom-up /
    top-down、4byte 行アライン）を `PixelFormat::Rgba8` へ変換。
    zero-length chunk は AVI の drop-frame 慣例として直前フレームを繰り返す。
- タイムスタンプ: AudioLens `al-media` の派生規則を再現 —
  stream ごとの `dwRate/dwScale` 時計、起点 `dwStart * dwScale`、
  video は chunk ごと `dwScale`、audio は `body_bytes * dwScale / dwSampleSize`
  （剰余保持）。生成刻印は `TimestampProvenance::Derived`。
- capability 申告: `capabilities()` は demux・decode・native-audio を報告し、
  decode codec は `[Pcm, Uncompressed]`。MJPEG 等の圧縮 fourCC は
  `decodable: Unsupported` の stream facts として正直に列挙する。
- 公開入口: `version()`, `capabilities()`, `open_demuxer()`。
  `AviDemuxer` が `make_decoder()` / `make_native_decoder()` を持つ
  （native video decoder は `ErrorKind::Unsupported`）。

## 既知の制限（明示）

- `Demuxer::seek` は `ErrorKind::Unsupported`（`idx1` 参照なしに
  `MediaTime` → `movi` オフセット写像を捏造しない）。
- OpenDML `rec ` ネストリスト（複数セグメント AVI）は
  `ErrorKind::Unsupported`。
- `movi` が `hdrl` より前に置かれたファイルはシーク可能ソースを要求し、
  非シーク可能なら `ErrorKind::Unsupported`。標準レイアウト
  （`hdrl` → `movi`）は非シーク可能ソースでも demux できる。
- 圧縮 video codec（MJPEG/DivX/H.264 fourCC）は demux facts のみ、
  decode は未対応（`Fact::Unsupported` → decoder 構築時 `ErrorKind::Unsupported`）。

## 層・許可依存

- 層: メディア I/O バックエンド（opt-in・分離クレート）
- 許可依存: `ruviform-media-io`（抽象境界）+ ドメインクレート直接依存
  （`ruviform-media-core`・`ruviform-audio-core`・`ruviform-video-core` —
  media-io がドメイン型を再エクスポートしないためベースラインが許容）
- 外部依存: なし（pure-Rust、unsafe 不使用）。

## RT 契約

**非 RT 専用。** プローブ・読み出し・デコードは I/O でブロックし得、
確保し得る。境界を越える型は `Send`。

## 関連

- 選択根拠: `docs/adr/select-media-backends-avi-and-mp4.md`
- サポートマトリクス正本: `docs/discovery/al-media-support-matrix.md`
- 抽象境界 ADR: `docs/adr/media-io-backend-abstraction.md`
- Issue #151（観測・非破壊 PCM 契約）: `docs/adr/media-observation-non-destructive-pcm.md`
