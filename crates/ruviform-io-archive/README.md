# ruviform-io-archive

Ruviform のアーカイブ基盤 — IO-04（Issue #286）設計に基づく
IO-IMP-08（Issue #302）の scaffold owner クレート。
アーカイブ形式中立の公開契約（Entry / Errors / Limits / Receipts）を確立し、
最初の形式実装として ZIP（Stored/Deflate）の indexed Reader/Writer を提供する。
後続の IO-IMP-09 TAR（Issue #303）・IO-IMP-10 Secure Extraction（Issue #304）は
本クレートの公開型に乗る。

## 責務

- `ArchiveError` / `ArchiveErrorKind` — 構造・切断・Local/中央 Directory
  不整合・checksum/size 不一致・budget・重複・非対応機能を分類する
  正規化失敗語彙。`Io` は下位 `io::Error` を `source()` に保持する。
- `ArchiveName` — エントリ名を raw bytes のまま保持（非 UTF-8 安全）。
  「filesystem に安全なパス」であることは一切保証しない — それは
  抽出層（IO-IMP-10）の契約。
- `EntryKind` / `ArchiveEntryInfo` — 形式が実際に記録するメタデータの
  共通ビュー（`File`/`Directory`/`SymbolicLink`/`Special`）。
- `ArchiveReadLimits` — 解析前にも解凍中にも効く budget 群
  （entries・metadata bytes・path bytes・per-entry / total uncompressed）。
- `ArchiveReadReceipt` / `EntryReadReceipt` — 検証済み内容を正確に報告する
  typed receipt（単一 `verified: bool` ではない）。

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

## 脅威境界（契約 — 実装は IO-IMP-10）

- **Zip-slip**: `ArchiveName` は生の格納 bytes を返すのみ。
  `..` / 絶対パス / ドライブ / UNC / 予約名の拒否は抽出層の責務。
- **展開爆弾**: 宣言サイズは早期拒否の目安にすぎない。実出力は
  stream 中に計測し cap で遮断する — 超過 bytes は呼出し側に渡らない。
- **中央 Directory 不整合**: open 時の Local Header 照合と
  `verify_all` の全走査、エントリ/CD region 重複 audit で分類済み失敗にする。
- **真正性**: CRC32 は破損検出であり暗号学的完全性証明ではない。

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
- Issue #303: IO-IMP-09 — TAR Entry/Pending（本 scaffold 上に構築）
- Issue #304: IO-IMP-10 — Secure Extraction / Staging Recovery
