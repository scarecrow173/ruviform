# ruviform-io-compression

Ruviform の汎用バイトストリーム圧縮基盤 — IO-03（Issue #285）の設計に
基づく非 RT 専用クレート。IO-IMP-06（Issue #300）で gzip・raw deflate の
Compression Core を実装した。`Encoder<'w, W: Write + ?Sized>` /
`Decoder<'r, R: BufRead + ?Sized>` の借用ストリーム型で、TAR → Gzip →
staged file のような `std::io` 経由の合成を消費者側に持ち込まずに提供する。

## 責務

- `gzip` — RFC 1952 member の encode/decode。ヘッダ（FEXTRA/FNAME/
  FCOMMENT/FHCRC 含む）と CRC32/ISIZE trailer を Ruviform 側で検証。
  `WholeInput` scope では連結 member を単一ストリームとして復号し、
  member 末尾の trailing byte を拒否する。`SingleUnit` scope では最初の
  member のみを復号し、後続 byte は呼び出し側の `BufRead` に残す
  （埋め込みストリーム用途）。
- `deflate` — RFC 1951 raw DEFLATE stream（zlib/gzip の container では
  ない）。checksum を持たないため integrity は「stream end の検証」のみ。
  単一 stream として扱い、`WholeInput` では stream 後の byte を拒否する。
- `EncodeOptions` / `DecodeOptions`（format 別）+ 共有語彙
  `EncodeLimits` / `DecodeLimits` / `DecodeScope` — 後続の IO-IMP-07
  （zstd・Issue #301）が同じ語彙を再利用できる汎用設計。
- `EncodeReceipt` / `DecodeReceipt` — `finish()` のみが返す観測済みの
  証跡（入力/出力/単位数・integrity 内訳）。
- `CompressionError` / `CompressionErrorKind` — backend・I/O の失敗を
  正規化分類に集約（backend 固有の詳細は erased `source()` のみ）。
  `Read`/`Write` 面では `io::Error` に包まれても `source()` 経由で
  kind を復元可能。

## 完了・検証セマンティクス

- Encoder: `Writing → Finishing → Finished | Failed`。`finish()` のみが
  成功の証明（冪等 — 保存済み receipt を返す）。`flush` は deflate sync
  flush であって完了ではない。`Drop` は一切 write しない — 未完了の
  member/trailer が黙って付くことはない。失敗した Encoder は terminal
  `Failed` となり、以後の操作は `InvalidState` を返す。
- Decoder: `Read` は `Ok(0)` を `VerifiedEnd` でのみ返す — format 末尾・
  trailer/checksum 検証・scope の入力境界契約がすべて成立したとき。
  `read(&mut [])` は状態を変えず、EOF の根拠にもならない。
  `Decoder::finish()` は `VerifiedEnd` のみ `Ok(DecodeReceipt)`、途中では
  `NotExhausted`（未読出力を破棄して辻褄を合わせない）。
- `compressed_bytes_consumed` はこの decoder が `BufRead` から論理的に
  消費した byte 数 — 呼び出し側 `BufReader` の prefetch 量とは区別する。

## Limits

`EncodeLimits`（入力/出力 byte）・`DecodeLimits`（圧縮入力/復号出力/
unit 数）は累積 budget としてこの層で強制する。上限ちょうどは許容、
1 byte 超過は `LimitExceeded`。CPU 時間・backend 内部メモリの hard
limit は宣言しない（呼び出し側/プラットフォームの責務）。

## 層・許可依存

- 層: 汎用圧縮 I/O 基盤（archive / file / media-io 消費者より下位・
  Primitives 同様の相互依存なし位置）。
- 許可依存: `flate2`（optional・`rust_backend` のみ）。`Compress`/
  `Decompress`/`Crc` の低層 primitive のみ使用し、gzip の member
  framing はクレート側で所有する — flate2 の型・エラー・version は
  公開 API に一切露出しない（backend-hiding 契約）。
- `default = []`・feature `gzip`/`deflate` で format 選択・
  `publish = false` 継承・MSRV 1.89。
- AudioLens Job/Dataset・Synvelis Project/Revision の意味論は持ち込まない
  — 消費者固有の契約は消費者側の責務。

## RT 契約

- 全 API 非 RT — `Read`/`Write` はブロック・alloc・無制限計算量を
  行い得る。RT オーディオコールバックからは絶対に呼ばない。

## 実装 Issue

- Issue #285: IO-03 圧縮設計（借用ストリーム・receipt・backend hiding）
- Issue #300: 本クレートの新設 — gzip・raw deflate の Compression Core
  （IO-IMP-06）
- Issue #282: File/Structured/Archive/Compression 横断設計（親 Issue）
- Issue #288: IO-06 横断消費者検証・OS/FS・セキュリティ・互換ゲート
- Issue #301: IO-IMP-07 — 後続の zstd 実装 Issue（共有語彙の再利用先）
