---
id: "DESIGN-034RT7CvRTe3b8jPTFBHtq"
type: "design"
status: "approved"
title: "Issue #302 ruviform-io-archive — Archive 共通型・ZIP Indexed Reader/Writer・検証パイプライン設計"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/302"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-archive/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-archive/README.md"
      - type: "file"
        path: "crates/ruviform-io-archive/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/entry.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/limits.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/receipt.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/zip.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/zip/read.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/zip/write.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/zip/record.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/zip_reader.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/zip_writer.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-302-io-archive-zip.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-302-io-archive-zip.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-302-io-archive-zip.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #302 ruviform-io-archive — Archive 共通型・ZIP Indexed Reader/Writer・検証パイプライン設計

## Context

Issue #302（IO-IMP-08）は共通 I/O 基盤（Issue #282）における Archive
クレートの scaffold Owner。IO-04（Issue #286）v0.1/v0.2 で合意済みの
「ZIP は `Read + Seek` の Indexed Reader」「Entry の排他的借用・明示
finish」「Budget を Parse/Allocation 前・途中に強制」「Backend 型非
露出」をコンパイル可能な Rust 契約へ固定する。

本設計の中核判断は **ZIP backend を外部 crate（`zip` 等）にせず、内部
Parser + `flate2`(rust_backend) + `crc32fast` で構成する**こと:

- IO-04 の Gate は「Central Directory の事前解析で Backend が一括メモリ
  確保する場合、Parse 後の事後 Budget 確認では不十分。Parser/Reader
  初期化前・初期化中に制限が強制できる方式を検証する」を要求する。
  `zip` crate は CD を一括 `Vec` に読み込むため、Entry 数・Metadata
  Byte・Path 長を**解析途中**に打ち切る制御が取れない。
- 内部 Parser は EOCD/CD/Local Header の各フィールド読取ごとに Budget
  判定を挟め、overlap・Local/CD 整合・truncated の検出位置を正確に
  分類できる。ZIP の構造は固定長 Record + 可変 3 領域で単純であり、
  全部署点が監査可能。
- Deflate 展開・圧縮と CRC32 は独自実装せず、成熟した pure-Rust
  backend（`flate2` の `rust_backend` = `miniz_oxide`、`crc32fast`）を
  **非公開依存**として使う。型・エラー・feature 名は公開 API に出ない。

## Scope

### In Scope

- `crates/ruviform-io-archive`（std + `crc32fast` + `flate2`/rust_backend、
  `default=[]`、`[lints] workspace`）。
- 共通公開型: `ArchiveError`/`ArchiveErrorKind`、`ArchiveName`、
  `EntryKind`、`ArchiveEntryInfo`、`ArchiveReadLimits`、
  `ArchiveReadReceipt`、`EntryReadReceipt`。
- `zip` module: `ZipArchiveReader`/`ZipEntryReader`/`ZipArchiveWriter`/
  `ZipEntryWriter`/`ZipCompressionMethod`/`ZipEntryOptions`。
- 内部 Parser（EOCD/CD/Local Header、Budget 統合、overlap 監査）と
  Stored/Deflate の入出力経路。否定 fixture を含むテスト。

### Out of Scope

- TAR（#303）・Secure Extraction（#304）・ZIP64/暗号化/Multi-disk/他
  Method・archive comment・data descriptor 出力・StagedFile 合成・
  serde・RT 経路。

## Components and Boundaries

### モジュール構成

| file | 責務 |
| --- | --- |
| `src/lib.rs` | crate doc（保証境界・non-RT・脅威モデル方針）・re-export・静的検証 |
| `src/error.rs` | `ArchiveError`（kind + message + `io::Error` source）、`ArchiveErrorKind` |
| `src/entry.rs` | `ArchiveName`（raw bytes 保持）、`EntryKind`、`ArchiveEntryInfo` |
| `src/limits.rs` | `ArchiveReadLimits`（pub field + `#[non_exhaustive]` + Default） |
| `src/receipt.rs` | `ArchiveReadReceipt`、`EntryReadReceipt` |
| `src/zip.rs` | `zip` module root・公開型 re-export・`ZipCompressionMethod`・`ZipEntryOptions` |
| `src/zip/record.rs` | 内部 Record 定義・LE parse/serialize ヘルパ（非公開） |
| `src/zip/read.rs` | `ZipArchiveReader`・`ZipEntryReader`・Index 構築・`verify_all` |
| `src/zip/write.rs` | `ZipArchiveWriter`・`ZipEntryWriter`・patch-back・CD/EOCD 出力 |

### 共通型の契約

- `ArchiveError { kind: ArchiveErrorKind, message: Cow<'static,str>,
  source: Option<io::Error> }`。`kind()`・`source()` を公開し、Display は
  kind + message +（あれば）entry 文脈。
- `ArchiveErrorKind`（`#[non_exhaustive]`）:
  `InvalidInput` / `InvalidState` / `Truncated` / `Malformed` /
  `Inconsistent` / `ChecksumMismatch` / `SizeMismatch` /
  `LimitExceeded` / `Overlap` / `Unsupported` / `Io`。
  - `Truncated` — 構造読取が EOF に達した（短いファイル・切れた CD）。
  - `Malformed` — 署名不一致・範囲外オフセット・構造的にあり得ない値。
  - `Inconsistent` — Local Header と CD が矛盾（name/method/size/crc）。
  - `ChecksumMismatch` / `SizeMismatch` — 展開結果が宣言と不一致。
  - `Overlap` — Entry 領域 × CD 領域・Entry 領域同士の重なり。
  - `Unsupported` — 暗号化・未知 Method・Multi-disk・ZIP64 必須・
    非 UTF-8 を受理できない経路等、「検出できるが扱わない」機能。
  - `LimitExceeded` — `ArchiveReadLimits` のいずれかの超過（どの
    Budget かは message に明記）。
- `ArchiveName { raw: Vec<u8> }` — Archive 内の元名称を**無変更**で保持。
  `as_bytes()`・`to_str() -> Option<&str>`（UTF-8 のみ）・
  `to_string_lossy()`。Path への変換・Traversal 検査は本型の責務**では
  ない**（#304 の `SafeRelativePath` が受け取る）。
- `EntryKind`（`#[non_exhaustive]`）: `File` / `Directory` /
  `SymbolicLink` / `HardLink` / `Special` / `Unsupported`。
  ZIP では name の trailing `/` または外部属性の dir bit → `Directory`、
  unix mode `S_IFLNK` → `SymbolicLink`、その他通常 Entry → `File`。
- `ArchiveEntryInfo { index, name: ArchiveName, kind,
  uncompressed_size, compressed_size }` — Format に実在する宣言値のみ
  （ZIP は全て CD 宣言値）。
- `ArchiveReadLimits`（pub field + Default）:
  `max_entries: u32 = 10_000`、`max_metadata_bytes: u64 = 16 MiB`、
  `max_path_bytes: u32 = 4_096`、`max_entry_uncompressed_bytes: u64 =
  4 GiB`、`max_total_uncompressed_bytes: u64 = 16 GiB`。`Default` を
  緩めない既定値にし、Consumer が個別に緩和/厳格化する。
- `EntryReadReceipt { info: ArchiveEntryInfo, uncompressed_bytes: u64,
  checksum: EntryChecksum }`、`EntryChecksum`（`#[non_exhaustive]`）:
  `Verified` / `NotPresent`（ZIP は常に `Verified` のみ返す）。
- `ArchiveReadReceipt { entries_total: u32, entries_verified: u32,
  entries_unverified: u32, payload_bytes: u64 }` — `verify_all` 成功時の
  サマリ。`verified` 一値に丸めない。

### Reader 状態機械

```
[Open]  -> EOCD locate -> CD parse (budget 統合) -> MetadataParsed(= Reader)
MetadataParsed --open_entry(i)--> EntryOpen(ZipEntryReader 借用中)
EntryOpen --entry.finish()--> MetadataParsed（EntryVerified を記録）
EntryOpen --Drop(未 finish)--> MetadataParsed（EntryUnverified のまま）
MetadataParsed --verify_all()--> AllEntriesVerified / Err(...)
```

- `ZipEntryReader<'a, R>` は `&'a mut ZipArchiveReader<R>` を保持し、
  同時に 1 つしか存在し得ない（借用規則で強制）。
- `open_entry` 時: index 検査 → 宣言 `uncompressed_size` vs
  `max_entry_uncompressed_bytes` 早期拒否 → seek(local_offset) →
  Local Header 読取 → CD との整合検査（name/method/flags/非 zero 時の
  size/crc 一致）→ data 領域の境界・CD overlap 検査 → Method dispatch。
- `ZipEntryReader: Read` — Stored は残 byte を `Take` 相当で直通、
  Deflate は `flate2::read::DeflateDecoder` を `Take` 化した下流に被せ、
  **出力側で残許容量を cap** する（展開爆弾の超過 Byte を呼出し側へ
  出さない）。実測 size が CD 宣言を超えたら即 `SizeMismatch`。
- `finish(self)` — 残りを Budget 内で drain → 実測 size・CRC32 を CD
  宣言と照合 → `EntryReadReceipt`。`read()` の EOF だけを検証済みとは
  しない。`verified` フラグは Reader 側 Index に記録される。
- `verify_all()` — 全 index を順に「local header 整合 + 全量展開 +
  CRC/size 照合」し、`max_total_uncompressed_bytes` を累積強制。
  途中失敗はその Err を返す（partial receipt は返さない）。
- 入力が並行変更され得る場合の Snapshot 一貫性は保証しない — Reader は
  各アクセスで再 seek するため、同一対象を外部変更された入力は
  `Inconsistent`/`ChecksumMismatch` 等の失敗として表面化する
  （rustdoc に明記）。

### Index 構築（open 時）と Budget 強制点

1. `stream_len`（Seek::end）で file 長を確定。`< 22` → `Truncated`。
2. 末尾 `min(len, 22 + 65_535)` Byte を**固定長バッファ**で後方走査し
   EOCD signature `0x06054b50` を検出。`pos + 22 + comment_len ==
   file_len` を要求（trailing garbage は受理しない）。
3. EOCD 検査: disk 番号/CD 開始 disk が非 0 → `Unsupported`
   （Multi-disk）。`count`/`cd_size`/`cd_offset` のいずれかが
   `0xFFFF`/`0xFFFFFFFF` 飽和、または EOCD64 locator 存在 → `Unsupported`
   （ZIP64 必須。採用候補だが本 Issue では検証せず明示拒否）。
4. **Allocation 前の早期拒否**: `cd_count > max_entries` →
   `LimitExceeded`。`cd_size > max_metadata_bytes` → `LimitExceeded`。
   `cd_offset + cd_size > eocd_offset` → `Malformed`/`Overlap`。
5. CD record を逐次 parse: 各 record の固定部 46 Byte 読取 →
   `name_len <= max_path_bytes`・record 全長を `cd_size` 残量に対して
   検査 → name bytes を `max_path_bytes` 以下に限定して `Vec` 確保
   （宣言先行確保しない）。record ごとに累積 metadata byte を数え
   `max_metadata_bytes` を途中でも強制。`Vec<CdRecord>` は
   `cd_count` 上限を pre-check 済みの上で `with_capacity` しない
   （`push` 駆動）— 宣言 count と実 record 数の不一致は `Malformed`。
6. Record 監査: `local_offset + 30 <= cd_offset`（local header が CD
   領域を指さない）・`local_offset + 30 + comp_size` の下界領域が他
   Entry の下界領域および CD 領域と重ならない（offset ソートで
   O(n log n) の overlap 監査）。真の data 領域は local の可変長のため
   ここでは下界のみ — 厳密な data-end 検査は open_entry / verify_all の
   Local Header 読取時に行う。

### Writer 状態機械

```
Ready --start_entry--> EntryOpen(ZipEntryWriter 借用中)
EntryOpen --entry.finish()--> Ready（CD record 確定）
EntryOpen --Drop(未 finish)--> Poisoned（後続 start_entry/finish は InvalidState）
Ready --finish()--> Finished（CD + EOCD 出力・inner W を返す）
```

- `start_entry(name: &str, opts: ZipEntryOptions)` — name 検査
  （非空・NUL なし・`<= u16::MAX` Byte）。現在位置を `local_offset`
  として Local Header を **size/crc 未確定のまま** 書き、Entry Writer
  を返す。general purpose flag は UTF-8 bit(11) のみ立て、bit0
  （encryption）・bit3（data descriptor）は立てない。
- `ZipEntryWriter: Write` — 入力 byte で CRC32 を更新し、Stored は下流へ
  直通、Deflate は `flate2::write::DeflateEncoder`（`Write` 実装）で
  圧縮して下流へ。圧縮後 byte 数を計数する。
- `entry.finish()` — encoder `finish` → seek で Local Header の
  crc/size 欄を確定値で patch → 末尾へ seek 復帰 → CD record を
  `Vec` に push → `EntryReadReceipt` 相当の書込サマリは返さず
  `Result<(), ArchiveError>`（receipt 情報は EntryInfo で取得可）。
- `ZipArchiveWriter::finish(self)` — 全 CD record + EOCD を書き
  `W` を返す。entry 数 `> 0xFFFF`・size `> u32::MAX` 等 ZIP64 が必要な
  値は `Unsupported`（ZIP64 出力は作らない）。
- Drop は完了証拠にしない: 未 finish の ArchiveWriter を Drop した場合
  何も保証しない（下流は中途半端な ZIP を持つ — 呼出し側が
  `StagedFile` 相当の publish 制御で正式公開を行うのは #304/Consumer
  合成の責務）。

### 脅威境界（契約として明記・実装は #304）

- Zip Slip 的 Path: 本クレートは `ArchiveName` を raw bytes で受け渡す
  だけで、Path 正規化・`..`/絶対 Path/Drive/UNC/予約名の拒否は行わない
  — #304 の `SafeRelativePath`/`ExtractPolicy` が責務。Reader が「安全な
  Path」を約束しないことを rustdoc に明記。
- 展開爆弾: 宣言 size は早期拒否材料に過ぎず、実測 Byte を展開途中に
  累積監視して超過を遮断する。圧縮率は Hard Limit の根拠にしない。
- CD 不整合: Local/CD 整合・overlap・truncated は本 Issue の検証対象。
  CRC は整合性検出であり真正性（改ざん検出）ではない。
- TOCTOU/実 FS 展開: 範囲外（#304）。

## Data and Control Flow

- Read: caller → `ZipArchiveReader::new(R, limits)` → EOCD/CD parse →
  `entry_info(i)` / `open_entry(i)` → `ZipEntryReader.read` →
  `entry.finish()` → `EntryReadReceipt`；または `verify_all()` →
  `ArchiveReadReceipt`。`into_inner()` で `R` を回収。
- Write: caller → `ZipArchiveWriter::new(W)` → `start_entry` →
  `ZipEntryWriter.write*` → `entry.finish()` → （繰返し）→
  `writer.finish()` → `W`（CD/EOCD 確定済み）。

## Non-Functional Constraints

- non-RT のみ。全 API がブロッキング I/O + alloc を許容。
- `unsafe` 不使用・MSRV 1.89・edition 2024・`default = []`。
- 依存: `crc32fast`（CRC32、MIT OR Apache-2.0、保守活発・依存木最小）、
  `flate2 = { default-features = false, features = ["rust_backend"] }`
  （DEFLATE、MIT OR Apache-2.0、miniz_oxide 経由で pure Rust・C 依存
  なし）。Backend 型はいずれも公開 API に露出しない。
- 公開 enum は全て `#[non_exhaustive]`。`ArchiveReadLimits` は
  `#[non_exhaustive]` + pub field（外部からのリテラル構築を禁じつつ
  field 変更は許可）。
- 展開経路は下流 `Read` 呼出しのバッファを超えて一括確保しない
  （`read_to_end` を強制しない）。CD parse の `Vec` は pre-check 済み
  `cd_count` でのみ上界を持つ。

## Risks and Trade-offs

- 内部 Parser は `zip` crate より新規コードだが、Budget 強制点と overlap/
  整合監査が明示でき、負の fixture で構造欠陥ごとに期待 kind を pin
  できる。ZIP 構造は単純で監査可能な範囲に収まる。
- EOCD 走査は末尾から最初に整合する署名を採用する。埋め込み EOCD 偽装
  （zip-in-zip）で外側が先に見える限りでは誤検出し得るが、`comment_len`
  整合 + `cd_offset/cd_size` の範囲監査で実害は `Malformed` 拒否方向。
- Writer の patch-back は `W: Seek` に依存（`Write` のみの Sink には
  非対応 — 設計どおり `Write + Seek` 要件）。
- Local Header の可変長により open 時 overlap 監査は下界ベース。
  data-end の厳密監査は Entry open/verify_all で補完する階層設計。
- 非 UTF-8 name は raw で保持するが、`to_str` は `None` を返す —
  `String` 前提の Consumer への受け渡しは lossy/拒否の選択を #304 に
  委譲する（ここで勝手に正規化しない）。

## Deferred Design Concerns

- ZIP64 入出力（EOCD64 locator/record、wide field）| Backend 検証後
  採用候補（Issue #286 H 節）・大 Archive 実需 | 巨大 Archive を
  Unsupported と拒否し続けるリスク。
- data descriptor(bit 3) の Writer 出力 | `Write` のみ Sink の実需 |
  Seek 非対応 Sink で書けないリスク（現状は契約どおり Unsupported）。
- `ArchiveName` → `SafeRelativePath` の検証 API | #304 の Extraction
  Policy 確定後 | Path 安全性を本型へ誤って内蔵するリスク。
- `verify_all` の中断/Cancel・partial receipt | 実績計測の実需 |
  大 Archive で途中失敗時に部分情報を返さない制約。

## Verification Notes

- [ ] EOCD/CD/Local Header の負 fixture 群で各 `ArchiveErrorKind` が
      期待どおり分類されること（test-spec 行列と一致）。
- [ ] Budget が「宣言値の事前拒否」と「実測の途中遮断」の両層で機能する
      こと（deflate 爆弾・過大宣言・過大 CD）。
- [ ] Entry Drop→再 Open・`verify_all` の receipt 正確性・Writer の
      未 finish 非 Completed 化が型とテストで pin されること。
- [ ] `flate2`/`crc32fast` の型・エラーが公開 API に現れないこと
      （`cargo public-api` 相当の目視監査 + rustdoc）。
