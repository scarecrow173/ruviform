# ruviform-io-wav

`ruviform-media-io` 抽象境界の PCM WAV バックエンド（Issue #191）:
pure-Rust の RIFF/WAVE encoder + muxer。境界の `Encoder`/`Muxer`
trait に対する初の実装済みバックエンド。

## 責務

- encode: canonical planar `f32` `AudioFrame` を受け、宣言された wire
  `SampleFormat`（`U8`/`S16`/`S24`/`S32`/`F32`）へ変換し interleaved
  `Packet` を生成（`CodecId::Pcm`）。
- mux: `Packet` payload を `data` chunk へ連結。構築時に
  `RIFF/WAVE` + `fmt ` + `data` ヘッダを即時書込み、`finalize()` で
  サイズ backpatch + pad + flush → `WavReceipt` を返す。
- `fmt ` チャンク: Mono/Stereo は `WAVEFORMATEX`（tag 1 / 3）、
  その他の layout は `WAVEFORMATEXTENSIBLE`（tag 0xFFFE + speaker
  mask + SubFormat GUID）。
- capability 申告: `capabilities()` は `MUX | ENCODE`・
  `encode_codecs = [Pcm]` のみ（demux/decode は持たない）。
  `wav_capabilities()` が format 粒度の Supported 集合
  （wire formats・single stream・RIFF only・RF64/BWF 非対応・
  `u32::MAX` 上限）を返す。
- 変換 policy（宣言済・決定的）: `NaN → 0`、`±Inf/|x|>1 → 飽和`、
  round half away from zero、dither なし（上流責務）。
  F32 wire は IEEE-754 をそのまま書く（clip しない）。
- completion 契約: `finalize()` → `WavReceipt` が唯一の完了レシート。
  `Drop` は best-effort であり完了保証とみなさない。永続化
  （sync + atomic publish + recovery）は `ruviform-io-file` の
  `SealedFile::publish` の責務。

## 既知の制限（明示）

- RF64（4 GiB 超）・BWF `bext`・複数 stream・非 PCM codec・
  `WavCapabilities` 外の wire format は `ErrorKind::Unsupported`。
- demux/decode は提供しない（`ruviform-io-symphonia` が WAV decode
  を担当する。本クレートの round-trip テストは dev-dependency でそちらを使う）。
- plain WAV は非ゼロ開始 pts を表現できないため、packet pts は
  連続性の順序情報として扱い `WavReceipt::first_pts_ticks` で診断報告する。
- atomic publish・durability・recovery は `ruviform-io-file` の
  `SealedFile::publish` の分類済み契約に従う（達成不能な保証は
  `Unsupported`/failure で報告。本クレート自身は publish を行わない）。

## 層・許可依存

- 層: メディア I/O バックエンド（opt-in・分離クレート）
- 許可依存: `ruviform-media-io`（抽象境界）+ ドメインクレート直接依存
  （`ruviform-media-core`・`ruviform-audio-core`）
- 外部依存: なし（pure-Rust、unsafe 不使用）。
- dev-dependency: `ruviform-io-symphonia`（round-trip 検証専用）、
  `ruviform-io-file`（contract test の staged publish 専用）。

## RT 契約

**非 RT 専用。** encode/mux/file/FS は I/O でブロックし得、確保し得る。
RT audio callback から呼ばせない。境界を越える型は `Send`。

## 関連

- 要件正本: GitHub Issue #191
- spec/adr/design: `docs/specs/issue-191-pcm-wav-output-backend.md`、
  `docs/adr/issue-191-pcm-wav-output-backend-*.md`、
  `docs/designs/issue-191-pcm-wav-output-backend.md`
- 抽象境界 ADR: `docs/adr/media-io-backend-abstraction.md`
