---
id: "DESIGN-034UApwItFwQ4wBpfOs2EC"
type: "design"
status: "approved"
title: "Issue #286 IO-04 ZIP/TAR Archive — Entry API・安全な展開・終了/公開契約 設計"
created: "2026-11-09"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "docs/specs/issue-286-io-04-zip-tar-archive.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/specs/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/plans/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/tasks/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/test-specs/issue-286-io-04-zip-tar-archive.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-286-io-04-zip-tar-archive.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-286-io-04-zip-tar-archive.md"
    - "docs/designs/issue-302-io-archive-zip.md"
    - "docs/designs/issue-303-io-archive-tar.md"
    - "docs/designs/issue-304-io-archive-extract.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/designs/issue-306-io-composition-e2e.md"
    - "docs/designs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/designs/issue-409-scoped-enumeration.md"
    - "docs/designs/rooted-read-capability.md"
    - "docs/designs/issue-300-io-compression-gzip-deflate.md"
    - "docs/designs/issue-301-io-compression-zstd.md"
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
    - "docs/specs/issue-306-io-composition-e2e.md"
    - "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-286-io-04-zip-tar-archive.md"
    - "https://github.com/scarecrow173/ruviform/issues/286"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/287"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/296"
    - "https://github.com/scarecrow173/ruviform/issues/300"
    - "https://github.com/scarecrow173/ruviform/issues/301"
    - "https://github.com/scarecrow173/ruviform/issues/302"
    - "https://github.com/scarecrow173/ruviform/issues/303"
    - "https://github.com/scarecrow173/ruviform/issues/304"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/409"
    - "https://github.com/scarecrow173/audio-lens/issues/139"
    - "https://github.com/scarecrow173/audio-lens/issues/143"
    - "https://github.com/scarecrow173/synvelis/issues/17"
    - "https://github.com/scarecrow173/synvelis/issues/82"
    - "https://github.com/scarecrow173/synvelis/issues/84"
    - "https://github.com/scarecrow173/synvelis/issues/25"
  defers: []
  deferred-by: []
---

# Issue #286 IO-04 ZIP/TAR Archive — Entry API・安全な展開・終了/公開契約 設計

## Context

Issue #286 は、独立クレート `ruviform-io-archive` の ZIP/TAR
Reader/Writer/Entry API・安全な展開（Secure Extraction）・
終了/公開契約を実装前に確定する IO-04 の設計 Issue である
（親 #282）。Issue 本文の「合意済み設計 v0.1・v0.2
（2026-11-04）」と「最終整合性レビュー — 実装 Issue への
引継ぎ」が要求の正本であり、本書はそれを docs チェーンの
正本設計書として文書化する。

**本書が固定するもの / 固定しないもの**

- 固定する: 形式別 Reader/Writer/Entry 契約（ZIP indexed・
  TAR streaming の capability 差を尊重し単一 trait へ統合
  しない）、format 中立の共通型、Backend 隠蔽（外部
  `zip`/`tar` crate 非採用・内部 Parser + 非公開 codec）、
  Entry 排他借用と明示 `finish` のみが完了証拠、検証済み
  終了の typed Receipt、Secure Extraction の脅威モデル・
  拒否規則・OS/FS 別保護レベル、Budget の宣言事前拒否 +
  実測途中遮断、Error 契約の語彙、Feature/Backend 確定値、
  Compression 合成（tar.gz 等）と終了/公開契約の分離、
  Consumer 利用面、証拠項目（API/ZIP/TAR/SEC/EXT/INT/
  COMP/DEP/OWN/WRT/REC/OS）と他 IO gate への対応索引。
- 固定しない: per-implementation の詳細 API・parser 内部・
  test 規則の実装詳細（#302/#303/#304 の設計・test-spec が
  正本）、実走テスト結果、Consumer repo の実測、backend の
  将来更新、Directory 原子的 Publish・Consumer Commit の
  契約。

**実装の現状（2026-11-10 時点の snapshot）**

- `crates/ruviform-io-archive` は main に存在し、#302
  （ZIP・PR #352）・#303（TAR・PR #357）・#304（Secure
  Extraction・PR #362）で merge 済み。`zip`/`tar`/`extract`
  module が実装され、`tests/{zip_reader,zip_writer,
  tar_reader,tar_writer,extract_zip,extract_tar,
  extract_staging}.rs` が契約を pin する。
- 2026-11-10 の受入再照合 slice で不足していた headless
  契約試験を追加した: ZIP `verify_all` の先頭失敗伝播と
  `into_inner` の非完了性、TAR Solaris `X` header の PAX
  record 適用、`discard_pending` の total budget 途中遮断、
  TAR PAX `path` override への Path Policy 適用
  （受理/拒否の両側）、Recovery `Absent` 状態と retry
  no-op、nested archive payload の非再帰受理。
- 検証入口は `mise run test-io-file`（`cargo test -p
  ruviform-io-file -p ruviform-io-archive` — io-archive を
  含む）・`check-io-file`/`lint-io-file`・`check-linux`
  （x86_64-unknown-linux-gnu 向け unix backend の補助
  type-check/lint）・`test-io-e2e`（#306 合成）。
- IO-06 cross-consumer gate 設計（#288）が IO-04 を matrix
  行 A-N1〜A-X5 として受入対象に含めている。
- Consumer 側: AudioLens #139/#143 は CLOSED、Synvelis
  #82 は CLOSED・#84 は OPEN・#25 は CLOSED（Issue 状態は
  変動し得る snapshot として記録）。

文書チェーン: [spec](../specs/issue-286-io-04-zip-tar-archive.md)・
[plan](../plans/issue-286-io-04-zip-tar-archive.md)・
[test-spec](../test-specs/issue-286-io-04-zip-tar-archive.md)・
[task](../tasks/issue-286-io-04-zip-tar-archive.md)。

## Scope

### In Scope

- 形式別 Reader/Writer/Entry 契約と format 中立共通型の
  正本記述。
- Backend 隠蔽・driver/backend 中立契約（外部 `zip`/`tar`
  非採用・内部 Parser + 非公開 codec・Serde 非依存）。
- Entry 排他借用・明示 `finish`・TAR 状態機械・Writer
  poison の所有権/終了契約。
- `ArchiveReadReceipt`/`EntryReadReceipt` の検証済み終了
  契約と checksum/disposition の意味論。
- Secure Extraction の脅威モデル・Path Policy・canonical
  衝突・kind/budget・Staging Root・OS/FS 別保護レベル・
  TOCTOU 境界・Cleanup/Recovery。
- 終了/公開契約（Entry→Archive→Compression→File publish
  の finish 連鎖分離・`ExtractedTree` の保証境界・
  Directory Publish 非保証）。
- Format 初期サポート範囲と `Unsupported` 分類。
- Cargo feature/Backend 確定値と依存 DAG・RT 制約。
- AudioLens/Synvelis の利用面と責務分界。
- 証拠項目（API/ZIP/TAR/SEC/EXT/INT/COMP/DEP/OWN/WRT/
  REC/OS・Design Acceptance Gate）から既存 test-spec・
  #288 matrix・実装 Issue/PR への対応索引。
- 設計合意と実装/検証の分離の明記。

### Out of Scope

- per-implementation 詳細設計の再定義（#302/#303/#304 が
  正本）。
- コード変更・テスト追加・CI 変更。
- Consumer repo の変更・実測（#219/#294 consumer-verify、
  AL #139/#143、SV #82/#84/#25 が Owner）。
- Directory 原子的 Publish・永続 Recovery Journal・
  Consumer Commit/CAS・Backup schema の Ruviform 側実装。
- ZIP64・暗号化 ZIP・sparse・special file 復元・nested
  archive の受理。
- 新規 CI Workflow・証拠スキーマ・Dashboard。

## Components and Boundaries — 責務分担

| 領域 | Owner | 本書との関係 |
| --- | --- | --- |
| IO-04 横断契約（形式別 Reader/Writer/Entry 骨格・共通型・所有権/finish・Receipt 語彙・Secure Extraction 脅威モデル・終了/公開分離・Feature/DAG・Consumer 面・証拠索引） | **Issue #286（本書）** | 正本 |
| ZIP indexed Reader/Writer・内部 Parser・CRC/CD 検証・Writer 状態機械 | **#302**（`docs/designs/issue-302-…`） | 詳細正本。本書は横断契約のみ所有 |
| TAR streaming Reader/Pending 状態機械・PAX/GNU 拡張・Writer 宣言 size | **#303**（`docs/designs/issue-303-…`） | 同上 |
| Secure Extraction（Path Policy・canonical 衝突・Staging Root・identity cleanup・Recovery Handle・OS backend） | **#304**（`docs/designs/issue-304-…`） | 同上 |
| File 公開契約（StagedFile type-state・PublishOptions・Receipt/Failure・PlatformFileOps・RecoveryHandle） | **#295**（`docs/designs/issue-295-…`） | Archive→File publish 合成が依存する下位契約 |
| 実 OS backend（renameat2/renamex_np/MoveFileExW・dir sync・shared vfs） | **#296**（`docs/designs/issue-296-…`） | publish および extract staging backend の OS 保証範囲の正本 |
| Stream Codec（Gzip/Deflate/Zstd の Encoder/Decoder・VerifiedEnd・budget） | **#300/#301**（`docs/designs/issue-300-…`・`issue-301-…`） | tar.gz 等の合成層の下位契約。Archive とは独立した finish |
| 旧 `StagedOutput`/旧 `PublishOptions` 撤去・WAV→StagedFile 移行 | **#305** | IO-04 以前の Media I/O 境界。本書は触れない |
| 合成 E2E（層横断境界試験） | **#306**（`docs/designs/issue-306-…`・`ruviform-io-e2e`） | Archive/Compression→StagedFile・extract の実 OS 証拠 |
| Rooted capability（`RootedDir`/`open_scoped_root`/bounded 列挙・shared vfs） | **#409**（`docs/designs/issue-409-…`・`rooted-read-capability.md`） | extract staging backend が共有する下位機構。本書は scoped I/O を所有しない |
| Cross-consumer/OS-FS/Security/性能/互換性 Gate | **#288**（`docs/designs/issue-288-…`） | 本書の証拠項目が同 gate の IO-04 matrix 行の根拠 |
| Cross-Repo CI 運用 | **#219/#294** | Consumer tuple 検証の実行機構 |
| Consumer 固有の Schema/Project/Dataset 意味論・Backup/Import・Golden・復旧 | **AL #139/#143、SV #82/#84/#25** | Consumer 側の受入 Owner |

原則: 本書は **横断契約の定義** を所有し、**per-implementation
詳細**は #302/#303/#304、**File/OS 保証**は #295/#296、
**圧縮層契約**は #300/#301、**実 OS 境界証拠**は #306、
**採否判定**は #288、**実行機構**は #219/#294 に委ねる。
責務の重複定義をしない。

## Reader/Writer/Entry モデル（形式別 capability）

Issue #286「合意済み契約」および v0.1-A/v0.2-A を正本として
固定する。ZIP と TAR は I/O capability が異なるため、単一の
不十分な最小 trait へ統合しない。

### 形式別 API 族（実装済みの確定形）

| Format | trait 要件 | Reader | Writer | 正本 |
| --- | --- | --- | --- | --- |
| ZIP | `Read + Seek`（indexed） | `ZipArchiveReader<R: Read + Seek>` — EOCD/中央 Directory を `ArchiveReadLimits` 内で解析、`entry_count`/`entry_info`/`entry_is_verified`/`open_entry`/`verify_all`/`into_inner` | `ZipArchiveWriter<W: Write + Seek>` — `start_entry`→`ZipEntryWriter`→entry `finish`→archive `finish`（CD+EOCD） | `docs/designs/issue-302-…`・`src/zip/{read,write,record}.rs` |
| TAR | `Read`（streaming）/ `Write`（streaming） | `TarArchiveReader<R: Read>` — `next_entry`/`entry_count`/`has_pending_entry`/`discard_pending`/`finish`/`into_inner` | `TarArchiveWriter<W: Write>` — `start_entry(name, declared_size, &TarEntryOptions)`→`TarEntryWriter`→entry `finish`→archive `finish`（2 zero block） | `docs/designs/issue-303-…`・`src/tar/{read,write,header}.rs` |
| 共通 | — | `extract` module — `ExtractionSession::extract_zip(&mut ZipArchiveReader)` / `extract_tar(TarArchiveReader)` | — | `docs/designs/issue-304-…`・`src/extract/` |

- ZIP は Seek 可能な indexed 形式のため、Entry は中央
  Directory の index で任意順に open でき、途中 Drop した
  Entry を未検証のまま別 Entry・同一 Entry を再 open
  できる。`verify_all()` は全 Entry を再読込して
  `ArchiveReadReceipt` を返す。
- TAR は one-way streaming のため `next_entry()` の逐次走査
  のみ。Entry を途中 Drop すると親 Reader は `Pending` に
  なり、`next_entry`/`finish` を拒否する。復帰は
  `discard_pending` のみ（budget 内 drain）。
- 「Open/Index 解析が成功した」ことと「全 Entry を展開・
  CRC 等で検証できた」ことは別の保証段階であり、Receipt で
  確認対象と保証範囲を区別する（`ArchiveReadReceipt` の
  entries_verified/unverified・payload_bytes）。

### Format 中立の共通型

- `ArchiveName` — Entry の格納名を **raw bytes のまま**保持
  （`as_bytes`/`to_str` は UTF-8 のみ `Some`/
  `to_string_lossy` は診断用）。Path 安全性の検査は一切
  行わない — それは `SafeRelativePath`（extract 層）の
  契約。非 UTF-8・ZIP raw name・TAR PAX/GNU・Unicode・
  casefold の差を失わない。
- `EntryKind`（`#[non_exhaustive]`）: `File`/`Directory`/
  `SymbolicLink`/`HardLink`/`Special`/`Unsupported`。
- `ArchiveEntryInfo` — `index`・`name`・`kind`・
  `link_name: Option<ArchiveName>`（TAR `1`/`2` のみ `Some`、
  ZIP は `None`）・`uncompressed_size`/`compressed_size`
  （`Option<u64>`、形式が実在する宣言値のみ）。全属性を
  無理に同一意味論へ変換しない。
- `ArchiveReadLimits`（`#[non_exhaustive]` + pub field）:
  `max_entries`（既定 10,000）・`max_metadata_bytes`
  （16 MiB）・`max_path_bytes`（4 KiB）・`max_path_depth`
  （32、extract 層で使用）・`max_entry_uncompressed_bytes`
  （4 GiB）・`max_total_uncompressed_bytes`（16 GiB）。
  宣言値の事前拒否と実測の途中遮断の両層で強制し、
  `Default` は緩めない。
- `ArchiveError`/`ArchiveErrorKind`（`#[non_exhaustive]`、
  12 variant）: `Io`/`InvalidInput`/`InvalidState`/
  `Truncated`/`Malformed`/`Inconsistent`/`ChecksumMismatch`/
  `SizeMismatch`/`LimitExceeded`/`Overlap`/`Unsupported`。
  backend/OS 固有 error 型は公開せず、`source()` の
  `dyn Error` 経由のみ診断へ到達する。

### 所有権と状態機械（実装済みの確定形）

Entry Reader/Writer は親 Archive Reader/Writer を排他的に
`&mut` 借用し、同時に複数の Entry を開けない（借用規則で
強制）。明示 `finish(self)` のみが完了証明であり、`Drop`
は完了を意味しない。

**ZIP Reader**:

```
Open -> EOCD/CD parse (budget 統合) -> MetadataParsed(= Reader)
MetadataParsed --open_entry(i)--> EntryOpen(ZipEntryReader 借用中)
EntryOpen --entry.finish()--> MetadataParsed（EntryVerified 記録）
EntryOpen --Drop(未 finish)--> MetadataParsed（unverified のまま・再 open 可）
MetadataParsed --verify_all()--> ArchiveReadReceipt / Err
```

- `open_entry` は index 検査 → 宣言 size の早期拒否 →
  seek → Local Header を CD と照合（name/method/flags/
  非 zero 時の size/crc）→ data 領域の境界・CD overlap
  検査 → Method dispatch の順で検証してから payload を
  stream する。
- `ZipEntryReader::finish` は残りを budget 内で drain し、
  実測 size・CRC32 を CD 宣言と照合して `EntryReadReceipt`
  を返す。`read()` の EOF だけを検証済みとはしない。
- `verify_all()` は全 Entry を順に local header 整合 +
  全量展開 + CRC/size 照合し、`max_total_uncompressed_bytes`
  を累積強制する。途中失敗はその Err を返す（partial
  receipt は返さない）。
- 入力が並行変更され得る場合の Snapshot 一貫性は保証しない
  — Reader は各アクセスで再 seek するため、外部変更は
  `Inconsistent`/`ChecksumMismatch` 等の失敗として表面化
  する（rustdoc 明記）。

**TAR Reader**:

```
Ready --next_entry(header ok)--> EntryOpen(TarEntryReader 借用中)
EntryOpen --entry.finish()--> Ready        （ReadToEnd 証明）
EntryOpen --entry.discard_remaining()--> Ready（Discarded 証明）
EntryOpen --Drop(未 finish)--> Pending
Pending --discard_pending(ok)--> Ready     （budget 内 drain）
Pending --next_entry/finish/discard失敗--> Failed または InvalidState
Ready|EndSeen --archive.finish()--> Finished(inner + ArchiveReadReceipt)
* --io/構造エラー--> Failed
```

- `next_entry` が Err を返した場合は Reader を `Failed` に
  する（逐次形式では header/拡張の parse 失敗・Limit 拒否・
  Unsupported のいずれも、その後の byte 位置を用いた継続を
  主張しない）。`Pending`/`Failed` 中の契約違反呼出しは
  `InvalidState`（非破壊）で archive 破損と分離する。
- `TarEntryReader::finish(self)` は payload 全消費時のみ
  成功（`EntryDisposition::ReadToEnd`）。`discard_remaining
  (self)` は残りを budget 内 drain して `Discarded` の
  Receipt を返す。TAR payload には checksum がないため、
  どちらも真正性/完全な破損不存在の保証ではない
  （`EntryChecksum::NotPresent` を明示）。
- `TarArchiveReader::finish(self)` は `Pending` を拒否し、
  残 Entry を discard として構造検証 drain → 終端 marker
  （zero block）→ EOF まで全 zero を確認して `(inner,
  ArchiveReadReceipt)` を返す。`entries_verified` は
  `ReadToEnd` 数、`entries_unverified` は Discarded + 途中
  Drop + finish drain 分。
- PAX `x`/`g`/`X`・GNU `L`/`K` 拡張は内部で消費し、実効
  name/link target/size に畳み込む（`g` global は文法検証
  のみで実効値へ適用しない — 全 Entry 改名の安全側遮断）。

**Writer（両形式共通の原則）**:

- Entry Writer と Archive Writer の状態を分離。
  `Ready -> EntryOpen -> Ready -> Finished`、失敗/未 finish
  Drop は `Failed`/poison。Entry Writer の借用中は別 Entry
  作成や Archive finish を禁止する。
- Entry の明示 `finish` 成功は Entry 構造完成、Archive の
  明示 `finish` 成功は Archive 完成（ZIP CD/EOCD・TAR 終端
  block）を示す。Compression Encoder finish・IO-01
  StagedFile seal/publish はさらに別契約。Drop 完了のみで
  正式 Publish を許可しない。
- ZIP Writer は `Write + Seek`（Local Header への
  patch-back が必要）。TAR Writer は `Write` のみだが
  Entry は宣言 size が必須 — 未知サイズ入力は caller が
  `take(remaining)` で cap するか Spool 等の別機能とする
  （本クレートは提供しない）。

### Receipt（検証済み終了の証拠）

- `EntryReadReceipt` — `info`・`uncompressed_bytes`（**実測**
  値、宣言値の代用ではない）・`checksum: EntryChecksum`
  （`Verified`/`NotPresent`）・`disposition:
  EntryDisposition`（`ReadToEnd`/`Discarded`）。
- `ArchiveReadReceipt` — `entries_total`/`entries_verified`/
  `entries_unverified`/`payload_bytes`（実測 uncompressed
  合計）。`verified: bool` 一値に丸めない。
- ZIP CRC32 は破損検出であり真正性（改ざん検出）ではない。
  TAR header checksum は header 自体の破損検出で payload の
  真正性ではなく、TAR payload には内容 checksum が存在
  しない。IO-03 の frame/member checksum・Consumer 独自
  Hash/署名は別の検証結果として扱う。

## Backend 隠蔽と Driver 中立性

Issue #286「合意済み契約」の Backend 非露出を正本として
固定する。実装は外部 `zip`/`tar` crate を採用せず、内部
Parser + 成熟 codec の非公開依存で構成する（#302/#303
設計の中核判断を継承）。

### 非公開とするもの

- 外部 `zip`/`tar` crate — 未採用。理由: IO-04 Gate の
  「Budget を Parse/Allocation 前・途中に強制」は backend
  が CD を一括確保する設計では事後確認しかできず満たせ
  ない。内部 Parser は EOCD/CD/Local Header の各読取・
  TAR header/拡張 payload の各段で budget 判定を挟める。
- codec backend: `flate2`（`rust_backend` = pure-Rust
  miniz_oxide の DEFLATE）・`crc32fast`（CRC32）・
  `unicode-normalization`（canonical 衝突の NFC）。いずれも
  型・エラー・feature 名を公開 API に露出しない
  （`Cargo.toml` comment が正本）。
- staging backend: `ruviform-io-file` の内部 `vfs` module
  （`VfsDir`/`VfsOps`/`NodeIdentity`/`platform_vfs_ops`）
  を path 依存で共有。unix/Windows の handle 相対
  primitive は vfs 層に集約され、io-archive は archive
  surface → vfs adapter のみを持つ。
- Serde 必須依存なし。公開 API は `std::io` trait/型と
  Ruviform 独自型のみで構成される。

### Driver 中立性

公開 API は特定 backend の具象実装に依存しない。
backend 差し替えは `Cargo.toml` の依存と module 内部に
閉じ、公開シグネチャ・Error 語彙・Options 型は Ruviform
型のみで構成される。Reader/Writer は `std::io::{Read,
Write, Seek}` のみを要求し、具体的な source/sink 型を
公開 API に露出しない。

### 検証根拠

backend 型非露出は per-implementation test-spec の規則で
pin されている（crate README「Backend 隠蔽」・`Cargo.toml`
comment・`issue-302`/`issue-303`/`issue-304` 設計の
Non-Functional Constraints）。#288 matrix 行 A-X4 が
「lexical check のみで race-safe を主張しない」脅威モデル
境界を、A-X3 が budget 両層強制を gate 項目として参照する。

## Format 初期サポート範囲と `Unsupported` 分類

Issue #286 H 節「Format 初期サポート」を正本として固定
する（実装済みの確定値）。

| Format | 受理するもの | `Unsupported`/拒否するもの |
| --- | --- | --- |
| ZIP read | classic（非 ZIP64）・Stored/Deflate・非暗号化・UTF-8 flag の name/raw bytes | 暗号化（bit0）・未知 method・multi-disk（非 0 disk 番号）・ZIP64-required 構造（飽和 EOCD field/EOCD64 locator） |
| ZIP write | Stored/Deflate（`ZipCompressionMethod`）・UTF-8 name・classic field 範囲 | ZIP64 出力（size `> u32::MAX`・count `> 0xFFFF`）・data descriptor 出力・archive comment |
| TAR read | POSIX ustar（name+prefix）・GNU magic・PAX `x`/`g`/`X`（`path`/`linkpath`/`size` 適用、`g` は文法検証のみ）・GNU `L`/`K`・base-256 数値 field | sparse（`S`）・special（`3`/`4`/`6`）・multi-volume・未知 typeflag（`V`/`M`/`N`/`D` 等） |
| TAR write | USTAR（name+prefix）+ 必要時 PAX `x`（`path`/`size`）・`add_directory`・file entry | GNU `L`/`K`・base-256 出力・link/special/sparse 出力（非対称: 読み側は受理、書き側は `Unsupported`） |
| extract | `EntryKind::File`/`Directory` のみ・`SafeRelativePath` 受理名 | `SymbolicLink`/`HardLink`/`Special`/`Unsupported` kind・全 traversal/衝突/budget 超過 |

- 「検出できるが扱わない」機能は黙って処理せず
  `ArchiveErrorKind::Unsupported` に正規化する（test-spec
  `issue-302` R8・`issue-303` R3・`issue-304` R3/R4 が pin）。
- sparse/special/link の「受理」は初期範囲外 — metadata と
  しては露出するが、復元・展開は行わない。link target は
  `ArchiveEntryInfo::link_name` で raw bytes のまま渡し、
  安全な扱いは extract 層の拒否規則が担う。

## Secure Extraction（脅威モデル・拒否規則・OS/FS 保証）

Issue #286 v0.1-D/E・v0.2-D/E/F を正本として固定する。
実装詳細は #304 設計・test-spec が正本であり、本書は
横断契約と脅威モデルの骨格を所有する。

### 信頼モデル

- **Untrusted（外部由来）を既定**とする。高水準
  Extraction（`ExtractionSession`/`extract_zip`/
  `extract_tar`）は常に Secure Default で動き、緩和する
  Option は存在しない（`ExtractOptions` は budget と
  copy buffer のみで、policy を緩めない）。
- **Trusted（自家生成・管理済み）Archive** は低水準 Reader
  API（`ZipArchiveReader`/`TarArchiveReader`）を caller が
  直接使う経路で扱う。Byte API（caller が開いた Reader を
  供給）と OS File を traverse する Convenience API は
  分離する設計であり、対象外ファイルの機密データが紛れ
  込まない既定とする。
- 信頼モデルの明示: 本層が守るのは「Archive の内容が
  staging 外へ書き出されない・展開先が差し替えられても
  誤削除しない」ことであり、Archive の真正性（誰が作った
  か）は保証しない。CRC/checksum は整合性検出であり
  authenticity ではない。

### Path Policy（`SafeRelativePath`）

`ArchiveName`（raw bytes・検証なし）から `SafeRelativePath`
（検証済み UTF-8 相対 component 列）への変換が唯一の境界。
「全 OS で同じ tree を意味する名のみ」を受理し、曖昧な名を
無断変換して成功扱いしない。

- **必須前提**: 非空・UTF-8・NUL 不含。非 UTF-8 は拒否
  （Windows 表現不能・unix 限定受理は portable 契約を破壊）。
- **区切り**: `/` のみ。`\`（Windows separator・unix では
  合法文字で意味が分岐）と `:`（Windows ADS/drive・unix
  では合法）を含む名は拒否。
- **component 拒否**: `..`・`.`・空・先頭 `/`（絶対）・`//`
  （UNC 相当）・末尾 `.`/末尾/先頭空白・制御文字。
- **Windows 禁止**: 予約名 `CON`/`PRN`/`AUX`/`NUL`/
  `COM1-9`/`LPT1-9`（大小無視・拡張子付き `con.txt` も
  stem 比較で拒否）・`<>|"?*`。
- **長さ**: component ≤ 255 byte・`max_path_bytes`・
  `max_path_depth` budget。
- **衝突**: canonical key = 全 Unicode `char::to_lowercase`
  + `unicode-normalization` NFC。同一 key の file/file・
  file/dir・dir 同一の重複を拒否。prefix が file として
  確定済みの entry（`a` file + `a/b`）も衝突。
  **ZIP は plan phase で全 entry を検証してから書込開始**
  （書込済み副作用ゼロで拒否できる）。**TAR は逐次検出**
  （後続衝突はその entry で拒否 → cleanup）。

### Kind・Budget 拒否

- `EntryKind::SymbolicLink`/`HardLink`/`Special`/`Unsupported`
  は `Unsupported` で拒否。File/Directory のみ受理。
- `ArchiveReadLimits` の全 budget（entry 数・metadata bytes・
  path bytes/depth・per-entry/total uncompressed）を
  **書込前の宣言値照合**と**実測の途中遮断**の両層で強制
  し、超過 byte は FS に出ない。宣言サイズは早期拒否の
  目安にすぎず、実測値の代用にしない。圧縮率は Hard Limit
  の代替でない。
- CPU/Heap/FS ディスク・Inode の全コストを byte budget
  だけで厳密保証しない（正直な境界として文書化）。

### OS/FS 別保護レベル

文字列検証だけでは同権限プロセスによる展開先の
symlink/reparse 置換（TOCTOU）を防げない。保護は path
文字列ではなく handle/FD 相対 primitive で与える。

| 対象 | backend | 保証 |
| --- | --- | --- |
| unix（Linux/macOS） | `openat`/`mkdirat`/`unlinkat` + `O_NOFOLLOW`/`O_DIRECTORY`/`O_CREAT\|O_EXCL` の dirfd 相対。identity は `statx`+`name_to_handle_at`（inode generation 含む）または `fstat`+birth/change-time discriminator | dirfd 相対 traversal・no-follow・排他 create・identity 照合 cleanup |
| Windows | `NtCreateFile`/`NtOpenFile`/`NtDeleteFile` を `RootDirectory` 相対 + `OBJ_DONT_REPARSE` + `FILE_OPEN_REPARSE_POINT` + `FILE_(NON_)DIRECTORY_FILE` で実行 | handle 相対 traversal・reparse 非追従・排他 create・`FileIdInfo`+`FileBasicInfo` identity 照合 cleanup |
| その他 target | `create_staging_root` が `Unsupported` を返すのみ | 保証を確立できない環境では動かさない（unsafe な path-string 走査へフォールバックしない） |

- `ExtractionSession::create(parent, options)` は信頼済み
  parent の下に一意名・排他作成の隔離 Staging Root を作り
  （`0700`/`0600` 相当の権限）、root handle + identity を
  記録する。既存 directory を開かない。
- 中間 component は `open_dir` で 1 段ずつ降下し、dir 以外・
  symlink・reparse への差し替えは primitive が拒否する。
  leaf の create は排他 create で既存名（symlink 含む）を
  上書きしない。
- **TOCTOU の正直な境界**: probe→delete 間・root remove
  直前の差し替えは理論上残るが、いずれも「tree 外への
  削除」ではなく「tree 内の foreign object を消す/消さない」
  の差に限定される。差し替え検出は per-component 降下で
  担保し、残存は「誤検出側（retained）」に倒して誤削除を
  回避する。Source archive の同時改変 race は保証外
  （reader の構造/CRC 検証で検出し得るが同一 inode の
  書換えは保証不能 — 入力は固定/信頼済み source に限定
  するよう doc 明記）。
- `cap-std` 等の採用・`openat2`/`RESOLVE_BENEATH` の追加
  kernel 保証は後続候補（Deferred）。文字列検査のみで
  敵対的な同権限プロセスに対して安全だと主張しない。

### Cleanup と Recovery

- `ExtractionSession`/`ExtractedTree` の Drop は best-effort
  cleanup（失敗は握りつぶし診断のみ — 完了・不変・公開の
  証拠ではない）。
- cleanup は作成時に記録した `NodeRecord`（path・kind・
  `NodeIdentity`）に従い、root/中間 dir/leaf の identity を
  照合しながら handle 相対に降下削除する。identity 不一致
  （foreign object・差し替え）は削除せず retained 計上。
  `remove_root` は empty root のみ。無条件の再帰削除は
  しない。
- 失敗時は `ExtractionFailure`（`ArchiveError` +
  `Option<ExtractionRecoveryHandle>`）。cleanup 成功なら
  `recovery: None`、失敗/不完全なら `Some(handle)`。
  `ExtractionRecoveryHandle` は `inspect()`
  （`RecoveryTreeState::{Present,Mismatch,Absent,Unknown}`）
  と明示 `cleanup()` を提供し、Drop しても不確定な Tree を
  削除しない。cleanup 失敗時も Handle と残留情報を維持する
  （`RecoveryCleanupError` が failure + handle を返す）。
- 明示 handoff（`ExtractedTree::into_path()`）した場合は
  handoff 先がライフサイクルを所有する。Process Crash 後の
  Handle 復元は保証せず、永続 Recovery Journal は Consumer
  が担当（IO-01 File RecoveryHandle と共通概念を維持するが
  Directory Tree を単一 File 用と無理に同型へ統合しない）。

### `ExtractedTree` の保証境界

- `ExtractedTree` は **検証した時点の Staging Tree の所有権**
  であり、不変の Filesystem Snapshot・署名済み Tree・完了
  した Directory Publish ではない。同権限の外部プロセスや
  FS 操作で内容は変更され得る。厳密性が必要なら入力
  Archive も先に信頼済み領域へ固定し、Consumer の
  Handoff/Commit 直前に必要な検証を実施する。
- `ExtractionReceipt`（entries/files/directories/
  payload_bytes/unverified_entries）は実測値を報告し、
  成功時 `unverified_entries` は常に 0（最初の未検証
  entry で停止する）。

## Resource Budget 契約

Issue #286 v0.1-F/v0.2-F「Extraction Budget・実績の計測」
を正本として固定する。

- `ArchiveReadLimits` の 6 budget は Reader/Parser 内部で
  必要なメモリ確保が行われる**前**に制御できる設計
  （#302: EOCD `cd_count`/`cd_size` の allocation 前早期
  拒否・record 累積 metadata・`name_len` 照合・`Vec` は
  `with_capacity` せず `push` 駆動。#303: 拡張 payload の
  宣言 size を Vec 確保前に残 budget と照合）。
- ZIP のヘッダ宣言展開サイズは早期拒否に使えるが、実測
  Byte 数の代わりにしない。Entry/全体の展開量を出力途中で
  累積監視し、上限超過 byte を正式 Sink/FS に書かない
  （Deflate 展開は出力側 cap・`read()` 時点で超過検出）。
- extract 層は宣言値を書込前に照合し、copy 中も実測を cap
  する（`copy_buffer_bytes` は 4 KiB〜1 MiB に clamp）。
- CPU 時間・heap・FS ディスク・Inode の全コストを byte
  budget だけで厳密保証しない。ZIP bomb・高 Entry 数・
  Nested・Sparse・Header bomb・破損等の否定テストを
  per-implementation test-spec が要求する。

## Error 契約

`ArchiveError`/`ArchiveErrorKind`（実装済みの確定値）:

- 12 variant（`#[non_exhaustive]`）: `Io`（下位
  `io::Error` を `source()` に保持）・`InvalidInput`（範囲外
  index・不正 name・policy 拒否）・`InvalidState`（状態機械
  違反・poison 後の操作）・`Truncated`（必須構造の途中
  EOF）・`Malformed`（署名不一致・構造的にあり得ない値・
  終端後後続 byte）・`Inconsistent`（Local Header と CD の
  矛盾等「2 つの記述の不一致」）・`ChecksumMismatch`
  （宣言 vs 実測 checksum — ZIP CRC32・TAR header chksum。
  整合性検出であり authenticity ではない）・`SizeMismatch`
  （実測 uncompressed size ≠ 宣言）・`LimitExceeded`（budget
  超過・どの budget かは message に明記）・`Overlap`（排他
  であるべき byte region の重複）・`Unsupported`（暗号化・
  未知 method・multi-disk・ZIP64-required・sparse/special・
  link kind・非対応 target）。
- `ExtractionFailure`/`ExtractionCleanupFailure`/
  `RecoveryCleanupError` は `ArchiveError` + recovery info
  を保持し、失敗分類を `Io` に丸めない。Drop は失敗を
  握りつぶす（診断のみ）。
- 「Format/構造の失敗」「Budget 超過」「状態機械違反」
  「Platform capability 不足」を分離した分類語彙を維持し、
  `InvalidInput`（policy/引数）と `InvalidState`（状態）と
  `Malformed`/`Inconsistent`（構造）を混同しない。

## Cargo Feature・Backend 選定（確定値）

### Feature matrix

`default = []`・feature なし。`publish = false` 継承・
MSRV 1.89・edition 2024・`unsafe` は `#[expect(unsafe_code)]`
+ SAFETY comment の局所許可（extract backend の FFI）。

| 依存 | 種別 | 用途 | 公開 API 露出 |
| --- | --- | --- | --- |
| `crc32fast = "1"` | crates.io | ZIP CRC32 | なし（内部実装） |
| `flate2 = "1"`（`default-features=false` + `rust_backend`） | crates.io | ZIP Stored/Deflate codec（pure-Rust miniz_oxide・C 依存なし） | なし |
| `unicode-normalization = "0.1"` | crates.io | extract canonical 衝突の NFC | なし |
| `ruviform-io-file`（path） | workspace | extract staging backend の shared vfs（`VfsDir`/`VfsOps`/`NodeIdentity`/`platform_vfs_ops`） | なし（内部 vfs） |
| `ruviform-io-compression`（dev-dep・`gzip` feature） | workspace | tar.gz 2 層 finish の合成テスト | — |

Serde 必須依存なし。依存追加の保守状況・ライセンス・
依存木サイズは `Cargo.toml` の comment と #302/#303/#304
設計の「Backend 選定」節が正本。

### 依存 DAG・RT 制約

- `ruviform-io-archive` → `ruviform-io-file` は extract
  staging backend の内部 vfs 共有のみ。他の面での相互依存
  はなく、`ruviform-io-file` から archive への逆依存は
  作らない（循環防止）。
- media-io / domain / engine / DSP / RT core への依存は
  持たない。Parser/Compression/Archive・ブロッキング I/O を
  RT コアへ混入しない（DAG 正本:
  `docs/designs/ruviform-architecture-design-baseline.md`・
  各 crate README）。
- 全 API 非 RT — I/O にブロックし alloc も行う。RT
  オーディオコールバックからは絶対に呼ばない
  （crate-level doc が正本）。公開 handle/error は
  compile-time `Send` assertion で pin。

## Compression との組合せ（tar.gz 等）

Issue #286「合意済み契約」の Codec 分離を正本として固定
する。

- Gzip/Deflate/Zstd の Stream Codec は **IO-03**（#300/
  #301）の責務であり、`ruviform-io-archive` は compression
  codec を実装しない。`tar.gz`/`tar.zst` は IO-03 の
  Encoder/Decoder と標準 `Read`/`Write` 合成による明示的
  結合であり、専用の結合 API は提供しない。
- **2 層 finish の独立性**: TAR 層の `TarArchiveReader::
  finish` は decode 済み stream 内の TAR 終端（zero block +
  全 zero 末尾）のみを証明し、圧縮 stream（member/trailer/
  CRC/圧縮 EOF）の検証は IO-03 decoder の `finish`
  （`VerifiedEnd` + `DecodeReceipt`）契約に分離する。TAR が
  先に EOF/終端扱いでも圧縮 Stream を検証済みとして扱わ
  ない（test-spec `issue-303` R12・`issue-304` R11・
  #288 matrix C-X4 が pin）。
- ZIP の内部 Codec（Stored/Deflate）は ZIP Adapter の
  capability として `ZipCompressionMethod` に閉じ込め、
  IO-03 への必須依存を作らない（依存 DAG 上も独立）。
- 完成証明の連鎖: Entry `finish` → Archive `finish` →
  Compression `finish` → `StagedFile` seal → publish は
  各層の独立した成功であり、いずれかの成功を他層の完了
  証拠にしない（#306 合成 E2E の「完成証明の直列化」と
  同一原則）。

## 終了/公開契約（Archive→StagedFile publish）

Issue #286「合意済み契約」の publish 分離を正本として
固定する。

- ファイルの Publish は **IO-01**（#295/#296）の
  `StagedFile`→`seal()`→`publish(PublishOptions)` 契約。
  Archive の完成を保証する `finish()` 成功**後のみ**安全な
  公開を認める。Entry/Archive/Compression のいずれの
  `finish` 成功も publish を代替しない。
- `ExtractedTree` は検証済み Staging Tree の所有権であり、
  単一 File の `StagedFile` publish とは別物。Directory
  replace/rename と公開結果不明は OS 依存であり、
  **無条件の Atomic Directory Publish を保証しない**。
  IO-01 の単一 File Publish を Directory/multi-file
  Transaction に拡張解釈しない（将来必要なら IO-01 で OS
  ごとの Capability を検討 — #304 Deferred）。
- 中断/破損/LimitExceeded 時は正式公開を拒否し Staging を
  Cleanup。Cleanup 失敗時の残留/Recovery 情報は独立に
  報告し、公開効果・不確定状態と混同しない。クラッシュ後
  の永続 Recovery Journal は Consumer 側で管理。
- Consumer 固有の Backup schema/Project semantics（SV の
  Project Snapshot/HEAD/Revision/Recovery、AL の
  Manifest/Dataset/Artifact hash）を Archive 層へ混入
  しない — Archive は byte 列の容器であり、schema/version/
  migration/canonical signed bytes/manifest semantics は
  各 Consumer が所有する。

## Consumer 利用面（AudioLens / Synvelis）

Issue #286「関連」および v0.2-6「Cross-Consumer」を正本
として固定する。Consumer の schema/version/migration/
canonical signed bytes/manifest semantics は各 Consumer が
所有し、Ruviform はそれらを移動しない。

### AudioLens（AL-IO-01 #139 — CLOSED・AL-IO-02 #143 — CLOSED）

- 利用面: Manifest・schema.json・index.json 等の
  Serde 型付き文書に加え、Dataset/Artifact の Archive
  要件（#139 が Manifest/Dataset/**Archive** 要件と回帰
  条件を含む）。Consumer 側の既存 schema を Ruviform の
  型へ移さず、Archive の byte 容器として利用する。
- 検証観点: Job 再開・Artifact hash・数値精度・既存
  Serializer の Golden Byte 差異は AL 側 golden test で
  担保。Archive の CRC は整合性検出であり authenticity
  ではないため、真正性が必要な箇所は Consumer 側の
  Hash/署名で補う。
- `ExtractionFailure` の `ArchiveError` + recovery handle
  は AL の job error/recovery へ写像できる（失敗分類を
  失わない）。

### Synvelis（SYN-IO-01 #82 — CLOSED・SYN-IO-02 #84 — OPEN・#25 — CLOSED）

- 利用面: Project Snapshot の Serde 型に加え、ZIP/TAR
  Backup・Secure Import（#82 が適合境界、#84 が
  DirectoryBundleStore/Snapshot/HEAD 保存移行、SV
  Project #17 PAP-14 が親要求）。Backup/Import の
  Project 整合・HEAD/Revision/Recovery の責務は SV が
  所有し、Ruviform Archive は容器 I/O のみを提供する。
- 検証観点: `ExtractedTree` は Directory Publish では
  ないため、SV の正式 commit/CAS/HEAD 更新は Consumer
  側の別契約。Untrusted な外部 Archive の Secure Import
  には `ExtractionSession` の Secure Default がそのまま
  使える。
- WAV PCM/Riff finalize/再 Open（#25 CLOSED の既存 E2E）
  は Media I/O（#305）側の責務であり IO-04 の対象外。

### Consumer 独立判定

#219/#288 の契約どおり、同一候補 Ruviform Revision でも
AudioLens と Synvelis は独立して判定する。IO-04 の採用は
consumer-verify.yml（#294）の候補 tuple lane で実測され、
`impacted`/`no impact`/`unknown` の影響分類と
`pass`/`fail`/`blocked`/`not run`/`unsupported` の Gate
結果で記録する。本書は要件の定義のみで、実施は各
Consumer repo が Owner。

## 証拠項目対応（Evidence Mapping）

Issue #286 の API/ZIP/TAR/SEC/EXT/INT/COMP/DEP/OWN/WRT/
REC/OS 証拠項目と Design Acceptance Gate を、既存の
正本（per-implementation test-spec・#288 matrix・合成
E2E・実装 Issue/PR）へ対応付ける索引。本書は参照のみを
持ち、証拠内容を複写しない。

### 証拠項目 → 正本対応表

| 証拠項目（Issue #286） | 内容 | 正本（test-spec / gate / 実装） |
| --- | --- | --- |
| API-01/02/03/04 | ZIP Seek 型・TAR Stream 型 Reader/Writer、Entry の Ownership/明示 finish、Backend 型非露出 | `issue-302` R6/R9/R11、`issue-303` R6/R7/R10/R13、`issue-304` R9、#288 matrix A-N1・本書「Reader/Writer/Entry モデル」「Backend 隠蔽」節 |
| ZIP-01 | Central Directory/ZIP64/CRC/Entry 検証と Archive 全体検証の区別 | `issue-302` R1–R5/R8、#302 設計「Index 構築」「Reader 状態機械」、本書「検証済み終了」節 |
| TAR-01 | PAX/GNU/終端/宣言 Size/未読 Entry の検証 | `issue-303` R1–R3/R8/R9、#303 設計「実効値の解決順位」「Reader 状態機械」、本書「所有権と状態機械」節 |
| SEC-01/02/03 | Zip Slip・絶対 Path/Drive/UNC・Symlink/Hard Link/Special・重複/Unicode/casefold/File-Dir 衝突の拒否 | `issue-304` R1–R3、`issue-306` R6、#288 matrix A-X1/A-X2、本書「Secure Extraction」節 |
| SEC-04/05/06 | Entry/Path/展開/Meta Budget、OS/FS 別 TOCTOU 保護レベル、Sparse/未知 Codec/暗号化 ZIP の Unsupported | `issue-302` R1/R7/R8、`issue-303` R4/R5/R3、`issue-304` R4/R9、#288 matrix A-X3/A-X4、本書「Resource Budget」「OS/FS 別保護レベル」「Format 初期サポート範囲」節 |
| EXT-01/02 | 隔離展開から検証済 Tree への遷移、Fail/Cancel/Cleanup 失敗と Recovery | `issue-304` R5–R8/R10、`issue-306` R6、#288 matrix A-N2/A-X5、本書「Cleanup と Recovery」「`ExtractedTree` の保証境界」節 |
| INT-01/02 | TAR.GZ/TAR.ZST と IO-03 の終端検証、Archive.finish→IO-01 File Publish | `issue-303` R12、`issue-304` R11、`issue-306`（層横断合成）、#288 matrix C-X4/E-N1、本書「Compression との組合せ」「終了/公開契約」節 |
| INT-03 | TAR.GZ/TAR.ZST の Archive/Compression 両終端の独立検証 | `issue-303` R12、`issue-304` R11、#288 matrix C-X4、本書「Compression との組合せ」節 |
| COMP-01 | AudioLens Dataset/Artifact Golden と精度・性能 | #288 matrix Consumer 非回帰判定表・#219/#294 consumer-verify・AL #139/#143 |
| COMP-02 | Synvelis Project Backup/Import/HEAD/Revision/Recovery の整合性 | 同上・SV #82/#84/#25 |
| COMP-03 | 両 Consumer の独立回帰（横断 Gate） | #288 matrix・consumer-verify（#219/#294）・AL #139/#143・SV #82/#84/#25 |
| DEP-01 | Features/MSRV/ライセンス/セキュリティと RT 依存 DAG | 本書「Cargo Feature・Backend 選定」節・`Cargo.toml` pin comment・#302/#303/#304 設計 Non-Functional・#288 gate の Cargo feature/OS/MSRV/license/security audit 項目 |
| OWN-01 | Entry Reader の排他的 Borrow と Archive Reader への安全な復帰 | `issue-302` R6、`issue-303` R6/R7、本書「所有権と状態機械」節 |
| OWN-02 | ZIP の途中 Drop でも未検証を維持したまま次 Entry を開け、`verify_all` で再検証可能 | `issue-302` R6、#302 設計「Reader 状態機械」 |
| OWN-03/04 | TAR 途中 Drop が Pending となり、`discard_pending` により予算内で復帰（I/O/エラー時の Failed も試験） | `issue-303` R6、#303 設計「Reader 状態機械」 |
| ZIP-02/03 | Entry CRC/実展開 Size、Local Header と CD の一致、全 Entry 再検証、入力変更の扱い | `issue-302` R4/R5/R6、#302 設計「Data and Control Flow」 |
| TAR-02/03 | Header/宣言サイズ/Padding/終端、PAX/GNU の実効名・長さ | `issue-303` R1/R2/R8/R9、#303 設計「TAR 構造と解析」 |
| WRT-01/02 | Entry/Archive の明示 finish、Writer 失敗・Drop で未完成の公開を禁止 | `issue-302` R9/R10、`issue-303` R10/R11、#288 matrix C-X4、本書「所有権と状態機械」「終了/公開契約」節 |
| EXT-03/04 | Staging 所有権移譲と `ExtractedTree` の保証境界、検証後に外部変更された Tree | `issue-304` R8/R10、本書「`ExtractedTree` の保証境界」節 |
| REC-01/02 | Cleanup 失敗後の Recovery Handle、Staging 差替え/敵対的並行変更下での誤削除回避 | `issue-304` R7/R8、#304 設計「Cleanup（verified 降下削除）」 |
| OS-01/02 | Linux/macOS/Windows の Directory Handle・No-follow/CreateNew/リンク・Reparse Point と脅威モデル別保証 | `issue-304` R9、`check-linux`（unix backend 補助）・CI lane（ubuntu+windows）・#304 test-spec Risk（macOS は非必須 lane/未実測として報告） |

### Design Acceptance Gate → 対応

| Gate 項目（Issue #286） | 対応する正本 |
| --- | --- |
| ZIP/TAR の具体な Ruviform 型・Reader/Writer/Entry API・所有権/seek 差異 | 実装済み API（`src/{zip,tar,entry,limits,error,receipt}.rs`）+ #302/#303 設計 + 本書「Reader/Writer/Entry モデル」節 |
| safe extraction の脅威モデル、拒否規則、OS/FS 別の保護レベル表 | 本書「Secure Extraction」節 + `issue-304` 設計/test-spec |
| limits・入出力/展開時の実績計測・Cancellation・途中失敗時の保全 | 本書「Resource Budget」節 + `issue-302` R1/R7・`issue-303` R4/R5・`issue-304` R4 |
| Binary Golden / negative fixtures（Zip Slip、Symlink、hardlink、name collision、zip bomb） | `issue-302`/`303`/`304` test-spec の fixture 行列・`issue-306` 合成 E2E |
| Compression との組合せと backend type 非露出の確認 | 本書「Compression との組合せ」「Backend 隠蔽」節 + `issue-303` R12・`issue-304` R11 |
| Archive→StagedFile publish で正常終了を契約化 | 本書「終了/公開契約」節 + `issue-306` E-N1/E-X1 |
| Consumer 固有 Backup schema/Project semantics を Archive へ混入しない | 本書「Consumer 利用面」節・crate README Non-goals・#288 脅威モデル境界 |
| 実装可能な詳細設計/ADR/試験項目 | #302/#303/#304 の docs チェーン（spec/design/plan/test-spec）+ 本書の対応索引 |

### #288 gate との接続

#288（IO-06）の受入 matrix は IO-04 を次の行で参照する
（`docs/designs/issue-288-…` が正本）:

- A-N1（正常: Entry index/stream read・`verify_all`/finish
  の receipt 正確性）→ `issue-302` R6・`issue-303` R7/R8。
- A-N2（正常: publish 済み archive の再 Open → verify →
  `ExtractionSession` 隔離展開 → `ExtractedTree`）→
  `issue-304` R5/R6・`issue-306` R6。
- A-X1（否定: Path Traversal）→ `issue-304` R1・
  `issue-306` R6。
- A-X2（否定: Link/重複/casefold/NFC-NFD 衝突）→
  `issue-304` R2/R3。
- A-X3（否定: Zip Bomb・budget 超過）→ `issue-302` R1/R7・
  `issue-303` R4/R5・`issue-304` R4。
- A-X4（否定: 構造破損・Pending・TOCTOU — lexical check
  のみで race-safe を主張しない）→ `issue-302` R2–R4・
  `issue-303` R1/R6・本書「OS/FS 別保護レベル」節。
- A-X5（否定: extract 途中 cancel/失敗時の staging
  cleanup）→ `issue-304` R6・`issue-306` R6。

IO-04 単体の実走証拠は `mise run test-io-file`（io-archive
含む）・`check-linux`（unix backend 補助）・`test-io-e2e`
（#306 合成）が入口。Consumer tuple の採否判定は #288
gate と consumer-verify（#219/#294）が Owner。

## Design Done / Implementation Done の分離

- **本 slice（docs）**: 横断契約・形式別 Reader/Writer/
  Entry 骨格・所有権/finish・Receipt 語彙・Secure
  Extraction 脅威モデル・終了/公開分離・Feature/Backend
  確定値・Consumer 面・証拠索引の **設計正本の文書化**。
  実装は #302/#303/#304 で merge 済みだが、本書の存在は
  「設計が文書化された」ことのみを意味し、Consumer 実測・
  OS 別実機・性能比較の完了を意味しない。
- **残る検証（実装/受入段階）**: Consumer 側の Golden・
  byte 互換・性能の独立検証（AL/SV Issue・consumer-verify
  tuple）、macOS/Linux 実機での extract backend、CI lane
  の継続監視。未実施は `not run`/`NotRun`/`Unsupported`
  と記録する。
- Issue #286 の受入条件のうち「API 署名/所有権が実際に
  利用可能」「Public API 監査」「Limits/security の実強制」
  「Error/Publish 連携」「feature/OS/MSRV/license/security
  audit と nonRT DAG」は #302/#303/#304 の実装と
  test-spec で既に検証済みの範囲を指し、本書はその正本
  への索引を提供する。「両 Consumer 回帰」は
  cross-consumer gate（#288・#219/#294）の実走段階の
  要件であり、本 slice の対象外。

## Risks and Trade-offs

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | 横断契約と per-implementation 正本の記述が drift する | med | med | 本書は骨格と索引のみ所有し、詳細は #302/#303/#304 へ参照。test-spec の規則が backend 非露出・budget・error 分類・状態機械を pin |
| 2 | 「byte 上限があるから heap/CPU/FS も bounded」と誤読される | med | high | Resource Budget 節で CPU/deserializer heap/無限 stream・inode/disk を非保証と明記。crate README・`limits.rs` doc と同一文言 |
| 3 | `ArchiveName` の raw bytes を安全な Path と誤認する | med | high | 「Path 安全性は `SafeRelativePath` の契約」を entry.rs rustdoc・本書で明記。extract 以外の経路で `to_string_lossy` を FS 名に使わない |
| 4 | CRC/checksum を真正性証明と誤用する | med | high | Receipt 語彙（`EntryChecksum::Verified`/`NotPresent`）と「整合性検出 ≠ authenticity」を receipt.rs・本書で明記。Consumer の Hash/署名は別層 |
| 5 | `ExtractedTree` を Directory Publish/不変 snapshot と誤認する | med | high | 「保証境界」節で明示。`into_path` handoff・Drop best-effort・identity 照合 cleanup を分離記述 |
| 6 | TAR の「読み切った」を「無破損」と混同する | med | med | `EntryChecksum::NotPresent` + `EntryDisposition`（`ReadToEnd`/`Discarded`）で構造検証と内容消費を区別。test-spec `issue-303` R7/R9 が pin |
| 7 | tar.gz で TAR finish と compression finish を混同する | med | med | 「Compression との組合せ」節で 2 層独立を明記。`issue-303` R12・`issue-304` R11・#288 C-X4 が pin |
| 8 | docs のみの変更が「Consumer 受入完了」と誤解される | low | med | Design/Implementation Done 分離節で明示。Consumer 実測は #288/#219/#294 の段階 |
| 9 | backend 更新で公開境界・budget 保証・kind 分類が壊れる | med | med | `=` pin 相当の版固定 + `Cargo.lock` + release-age ポリシー。upgrade は PR で明示的に行い per-implementation test-spec で再検証 |

## Deferred Design Concerns

- ZIP64 入出力・data descriptor Writer 出力・archive
  comment | 実装は `Unsupported` で拒否済み。大 Archive・
  `Write` のみ Sink の実需が出た時点 | ZIP64-required 構造を
  持つ入力を受理できないリスク。
- TAR sparse（`S`）・special file・multi-volume・PAX `g`
  実効値適用・Writer の GNU/base-256/link 出力 | 実装は
  `Unsupported` または構造検証のみ。Consumer 互換要件確定
  後 | 対応形式の Archive を扱えないリスク。
- Directory 原子的 Publish・永続 Recovery Journal・
  Consumer Commit/CAS・`ExtractedTree` への readback API |
  IO-01 単一 File Publish の拡張解釈ではなく別契約。
  Consumer 要件確定時 | `ExtractedTree` を正式公開へ繋ぐ
  契約が未定義のまま残るリスク。
- `openat2`/`RESOLVE_BENEATH` 追加・`cap-std` 検討・非
  UTF-8 名受理・IO-IMP-02 `OsFileOps`/vfs とのさらなる
  共通化 | 現行の dirfd/handle 相対 + no-follow +
  CreateNew + identity 照合で主要脅威をカバー。kernel
  保証の追加検証・実需確定後 | 残存する理論上の probe→
  delete race・表現不能名の扱い。
- Consumer 側の実測・移行（SV ZIP/TAR Backup・Secure
  Import、AL archive 要件） | cross-repo gate（#219/#294
  consumer-verify）と各 Consumer Issue が Owner | 設計
  どおりの互換性が実測で裏付けられないまま採用される
  リスク。
- macOS/Linux 実機での extract backend 実測 | 本 slice
  の作業環境は Windows のみ。unix backend は `check-linux`
  で型/lint 補助済み、実機試験は CI lane 依存 | OS/FS 差異
  （reparse point・dir sync 等）が未検証のまま残るリスク。

## Verification Notes

- [x] Reader/Writer/Entry モデル節が ZIP `Read + Seek`
      indexed・TAR `Read`/`Write` streaming の形式別 API
      族・共通 format 中立型（`ArchiveName`/`EntryKind`/
      `ArchiveEntryInfo`）・「形式が実在する情報のみ記録」
      を含み、#302/#303 設計への参照を持つこと。
- [x] Backend 隠蔽節が外部 `zip`/`tar` 非採用・内部
      Parser + `flate2` rust_backend/`crc32fast`/
      `unicode-normalization`/`ruviform-io-file` vfs の
      非公開依存・Serde 非依存・`std::io` ベースを含む
      こと。
- [x] 所有権/状態機械節が Entry 排他借用・明示 `finish`
      のみが完了証拠・TAR `Ready`/`EntryOpen`/`Pending`/
      `Failed`・`discard_pending`/`discard_remaining`・
      Writer poison を含むこと。
- [x] Receipt 節が `ArchiveReadReceipt`/`EntryReadReceipt`
      の typed summary・`EntryChecksum`/`EntryDisposition`・
      `verified: bool` 非丸め・真正性 ≠ 整合性を含むこと。
- [x] Secure Extraction 節が信頼モデル（Untrusted 既定）・
      Path Policy・canonical 衝突・kind/budget・OS/FS 別
      保護レベル表・TOCTOU 境界・Cleanup/Recovery を含む
      こと。
- [x] Format 範囲表の受理/`Unsupported` が `Cargo.toml`
      実装・crate README・#302/#303/#304 設計に一致する
      こと（ZIP classic Stored/Deflate・TAR USTAR/GNU/PAX・
      sparse/special/link/ZIP64/暗号化は `Unsupported`）。
- [x] Compression 組合せ節が tar.gz/tar.zst の 2 層
      finish 独立・ZIP 内部 codec の capability 閉じ込め・
      IO-03 への必須依存なしを含むこと。
- [x] 終了/公開契約節が Entry→Archive→Compression→
      StagedFile publish の finish 連鎖分離・`ExtractedTree`
      ≠ Directory Publish・無条件 Atomic Directory Publish
      非保証を含むこと。
- [x] 証拠項目対応表の API/ZIP/TAR/SEC/EXT/INT/COMP/DEP/
      OWN/WRT/REC/OS 各行が実在する test-spec rule/節・
      #288 matrix 行（A-N1〜A-X5）・Issue/PR 番号へ解決
      できること。
- [x] Consumer 利用面節が AL #139/#143・SV #82/#84/#25・
      SV Project #17 の Issue 番号と責務分界（schema/
      version は Consumer 所有・`ExtractedTree` ≠ Project
      Transaction）を含むこと。
- [x] 「設計合意と実装/検証の分離」節が存在し、未実施項目
      を `not run`/`NotRun`/`Unsupported` と記録する方針を
      明記していること。
