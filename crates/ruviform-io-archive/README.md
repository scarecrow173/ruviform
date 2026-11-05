# ruviform-io-archive

Ruviform のアーカイブ基盤 — IO-04（Issue #286）設計に基づく
IO-IMP-08（Issue #302）の scaffold owner クレート。
アーカイブ形式中立の公開契約（Entry / Errors / Limits / Receipts）を確立し、
ZIP（Stored/Deflate）の indexed Reader/Writer と
TAR（ustar/GNU/PAX）の streaming Reader/Writer、および
IO-IMP-10（Issue #304）の secure `extract` モジュールを提供する。

## 責務

- `ArchiveError` / `ArchiveErrorKind` — 構造・切断・Local/中央 Directory
  不整合・checksum/size 不一致・budget・重複・非対応機能を分類する
  正規化失敗語彙。`Io` は下位 `io::Error` を `source()` に保持する。
- `ArchiveName` — エントリ名を raw bytes のまま保持（非 UTF-8 安全）。
  「filesystem に安全なパス」であることは一切保証しない — それは
  抽出層（IO-IMP-10）の契約。
- `EntryKind` / `ArchiveEntryInfo` — 形式が実際に記録するメタデータの
  共通ビュー（`File`/`Directory`/`SymbolicLink`/`HardLink`/`Special`）。
  TAR の link target は `link_name()` で raw bytes のまま露出する。
- `ArchiveReadLimits` — 解析前にも解凍中にも効く budget 群
  （entries・metadata bytes・path bytes・per-entry / total uncompressed）。
- `ArchiveReadReceipt` / `EntryReadReceipt` — 検証済み内容を正確に報告する
  typed receipt（単一 `verified: bool` ではない）。`EntryDisposition` が
  entry を最後まで読んだか構造的に破棄したかを記録する。

## ZIP 実装（`zip` module）

- `ZipArchiveReader<R: Read + Seek>` — EOCD を末尾から探索し中央
  Directory を budget 内で一括解析。`open_entry(index)` は排他
  `&mut` borrow の `ZipEntryReader` を返し、Local Header を CD と照合してから
  data を stream する。`verify_all()` は全エントリを実測検証
  （Local/CD 整合・measured size・CRC32・region 重複 audit）。
- `ZipEntryReader::finish()` — 残り payload を drain して measured
  size と CRC32 を確定させる**唯一の**エントリ検証完了証明。
  途中 Drop は未検証のまま残り、別エントリ・同一エントリの再 open を許す。
- `ZipArchiveWriter<W: Write + Seek>` — `start_entry` が zero 値 Local
  Header を書き、`ZipEntryWriter::finish` で seek して CRC/size を
  patch-back、`ZipArchiveWriter::finish` で中央 Directory + EOCD を書き出す。
  未完成 entry を Drop すると archive は poison され `InvalidState` —
  不完全なアーカイブは決して完成扱いにならない。
- 対応範囲: classic ZIP（非 ZIP64）・Stored/Deflate・暗号化なし。
  暗号化・未知 method・multi-disk・ZIP64-required 構造は
  `Unsupported` に正規化される。

## TAR 実装（`tar` module）

- `TarArchiveReader<R: Read>` — TAR は逐次形式のため、512 byte の
  header block を順に走査する streaming reader（`next_entry()`）。
  `x`/`g`/`X` PAX 拡張と `L`/`K` GNU long-name 拡張は内部で消費し、
  直後の実 entry の有効 name / link target / size に畳み込む
  （`g` global record は文法検証のみで適用しない）。
- `TarEntryReader` — 排他 `&mut` borrow の payload reader。
  宣言 size と total budget の両方で cap し、超過 byte は渡さない。
  `finish()` は payload 全消費後のみ成功し、block padding を検証して
  `EntryDisposition::ReadToEnd` を返す。`discard_remaining()` は残りを
  budget 内で drain して `Discarded` を返す。途中 Drop は archive を
  `Pending` にし、`discard_pending()` が唯一の回復経路。
- `TarArchiveReader::finish()` — 未到達 entry を構造的に drain 検証し、
  end-of-archive marker（zero block）と末尾が全て zero であることを
  確認する唯一の archive 完了証明。tar.gz 等の外側圧縮層の trailer
  検証は別層の `finish` 契約であり本検証には含まない。
- `TarArchiveWriter<W: Write>` — `start_entry(name, declared_size, opts)`
  が宣言 size 付き header を先行書き、`TarEntryWriter` が宣言量を cap、
  `finish` で padding を書き、archive `finish` で 2 個の zero block を
  書き出す。未完成 entry の Drop は archive を poison する。
  ustar 155/100 prefix で表せない長い name や octal 桁溢れの size は
  先行 `x` PAX record（`path=`/`size=`）で書き出す。
- 対応範囲: POSIX ustar（name+prefix）・GNU magic・`L`/`K`/`x`/`g`/`X`
  拡張・base-256 数値 field の読取。sparse（`S`）・special
  （`3`/`4`/`6`）・その他未知 typeflag・link entry の Writer 出力は
  `Unsupported`（link target は metadata 露出のみ。安全な復元は
  IO-IMP-10 の責務）。
- TAR payload には checksum が存在しない — entry receipt は常に
  `EntryChecksum::NotPresent`。payload の破損/真正性は一切証明しない。

## Secure Extraction 実装（`extract` module）

- `SafeRelativePath` — `ArchiveName` を展開用に検証した相対パス。
  UTF-8 必須・`..`/`.`/空・絶対パス・`\`・ドライブ・UNC・Windows
  予約名・ADS（`:`）・repeated separator を拒否。NFC + case-fold の
  canonical key で Unicode/大小文字・file/dir 衝突を検出する。
- `ExtractionSession` — 信頼済み parent の下に排他的な隔離 staging
  root を作り、`extract_zip` / `extract_tar` で検証済み
  `ExtractedTree` を返す。受け付けるのは通常 File/Directory のみ
  （symlink/hardlink/special・上書きは拒否）。entry `finish()` の
  payload 検証は維持される。
- Budget — entry 数・metadata bytes・path bytes/depth・per-entry /
  total uncompressed を書込前に強制し、宣言サイズに嘘がある zip
  bomb は実測で遮断する。
- `ExtractedTree` — `path()`/`into_path()`/`cleanup()`・Drop で
  ベストエフォート片付け。cleanup は記録済み node を最深優先で
  identity 検証してから削除し、外部差し替え物は決して消さない。
- `ExtractionRecoveryHandle` — cleanup 未完時の回復ハンドル。
  `inspect()` は present/absent/identity-mismatch/unknown を区別し、
  `cleanup()` 再試行でも記録 root と identity が一致しない限り
  削除しない。
- Platform backend — Windows は `NtCreateFile`/`NtOpenFile`/
  `NtDeleteFile` を RootDirectory 相対 + `OBJ_DONT_REPARSE` +
  `FILE_OPEN_REPARSE_POINT` で実行。Unix は `openat`/`mkdirat`/
  `unlinkat` + `O_NOFOLLOW` の dirfd 相対。保証を確立できない
  環境では `Unsupported` を返し、unsafe な path-string 走査には
  フォールバックしない。

## 脅威境界

- **Zip-slip**: `ArchiveName` は生の格納 bytes を返すのみ。
  `..` / 絶対パス / ドライブ / UNC / 予約名の拒否は抽出層の責務。
- **展開爆弾**: 宣言サイズは早期拒否の目安にすぎない。実出力は
  stream 中に計測し cap で遮断する — 超過 bytes は呼出し側に渡らない。
- **中央 Directory 不整合**: open 時の Local Header 照合と
  `verify_all` の全走査、エントリ/CD region 重複 audit で分類済み失敗にする。
- **TAR 宣言サイズの嘘**: 宣言と実 payload が食い違うと stream は
  自ずと desync し、次 header の `ChecksumMismatch`/`Malformed`/
  `Truncated`/`Unsupported` として表面化する。EOF 欠落は `Truncated`、
  padding の非 zero は `Malformed` として分類済みにする。
- **真正性**: CRC32 は破損検出であり暗号学的完全性証明ではない。
  TAR payload には checksum 自体が存在しない。

## Backend 隠蔽

`crc32fast`（CRC32）と `flate2`（`rust_backend` = pure-Rust
miniz_oxide の DEFLATE）は内部実装であり、型・エラー・feature 名を
公開 API に一切露出しない。`default = []`・feature なし・
`publish = false` 継承・MSRV 1.89。

## 層・許可依存

- 層: 汎用ファイル I/O 基盤（`ruviform-io-file` と同順位・相互依存なし）。
- 許可依存: `crc32fast`・`flate2`（rust_backend）・std のみ。
  GUI・オーディオデバイス・GPU・consumer 依存なし。

## RT 契約

- 全 API 非 RT — I/O でブロックし alloc も行う。
  RT オーディオコールバックからは絶対に呼ばない。

## 実装 Issue

- Issue #286: IO-04 ZIP/TAR Archive 設計（公開契約・脅威境界の正本）
- Issue #302: IO-IMP-08 — 本クレートの新設・共通型・ZIP indexed
  Reader/Writer・CRC/中央 Directory 検証・Budget
- Issue #303: IO-IMP-09 — TAR streaming Reader/Writer・
  USTAR/GNU/PAX・Pending 状態機械・宣言 size・EOF 検証
- Issue #304: IO-IMP-10 — Secure Extraction / Staging Recovery
