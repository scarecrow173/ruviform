---
id: "SPEC-034RT7qvjzmpx6kytmM8O0"
type: "spec"
status: "approved"
title: "Issue #302 ruviform-io-archive — ZIP Indexed Entry Reader/Writer・CRC/中央Directory検証 (IO-IMP-08)"
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
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #302 ruviform-io-archive — ZIP Indexed Entry Reader/Writer・CRC/中央Directory検証 (IO-IMP-08)

## What Should Be Built

独立クレート `crates/ruviform-io-archive` を新設し、Workspace に登録する。
本クレートは IO-04（Issue #286）v0.1/v0.2 合意済み契約に基づく Archive
基盤の **scaffold Owner** であり、後続の IO-IMP-09（TAR・Issue #303）と
IO-IMP-10（Secure Extraction・Issue #304）が乗る共通の公開型を確立する。

公開面（Format 共通）:

- `ArchiveError` / `ArchiveErrorKind` — ZIP/TAR/Extraction が共有する
  分類済みエラー。構造不正・truncated・Local/CD 不整合・CRC/サイズ不一致・
  Limit 超過・overlap・Unsupported・InvalidState・Io を区別する。
- `ArchiveEntryInfo` / `ArchiveName` / `EntryKind` — Entry の元名称を
  非 UTF-8 を失わず保持する `ArchiveName`（raw bytes）と、Format が
  実在する情報のみを記録する Entry メタデータ。
- `ArchiveReadLimits` — `max_entries`・`max_metadata_bytes`・
  `max_path_bytes`・`max_entry_uncompressed_bytes`・
  `max_total_uncompressed_bytes`。Parser/Reader が内部確保を行う
  **前・途中** に強制可能な設計とする（事後確認だけを Hard Limit と
  呼ばない）。
- `ArchiveReadReceipt` / `EntryReadReceipt` — `verify_all` や Entry
  `finish()` の検証サマリ。`verified: bool` 一値に丸めない。

公開面（`zip` module）:

- `ZipArchiveReader<R: Read + Seek>` — EOCD + Central Directory を
  Budget 付きで解析し Index を構築。`entry_count` / `entry_info` /
  `open_entry` / `verify_all` / `into_inner`。
- `ZipEntryReader<'_, R>` — 親 Reader の排他的 `&mut` 借用。
  `std::io::Read` を実装し、`finish(self)` で Entry 終端・実展開
  サイズ・CRC の検証結果を `EntryReadReceipt` として確定する。
  途中 Drop は Entry を**未検証**のまま記録し、別 Entry（同一 Entry の
  再 Open を含む）を開ける。
- `verify_all()` — 全 Entry を再読込し、展開実績・CRC/サイズ・
  Local Header/Central Directory 整合を検証して `ArchiveReadReceipt`
  を返す。Open/Index 解析の成功は全 Entry 検証成功を意味しない。
- `ZipArchiveWriter<W: Write + Seek>` — `start_entry` →
  `ZipEntryWriter`（`Write`）→ `EntryWriter.finish` →
  `ArchiveWriter.finish` で Central Directory/EOCD を書き切る型分離。
  Entry Writer を `finish` せず Drop した Archive は Completed 扱いに
  しない（後続 `start_entry`/`finish` を `InvalidState` で拒否）。
- `ZipCompressionMethod::{Stored, Deflate}`、未知 Method・Encryption・
  Multi-disk・ZIP64 必須入力は `Unsupported`（ZIP64 は Backend 検証後の
  採用候補 — 本 Issue では明確に拒否する）。
- 内部 ZIP Parser（EOCD/CD/Local Header）と Deflate コーデック
  （`flate2` rust_backend / `crc32fast`）は**実装詳細として隠蔽**し、
  型・エラー・feature 名を公開 API に露出しない。

## Why Now

共通 I/O 基盤チェーン（Issue #282）の Archive 系起点。#303（TAR）と
#304（Secure Extraction）は本 Issue が置く `ArchiveError` /
`ArchiveEntryInfo` / `ArchiveName` / `ArchiveReadLimits` /
Receipt 型の上に構築されるため、scaffold の公開型を先に固定しないと
並行実装が着手できない。Consumer（Synvelis Project Backup/AudioLens
Dataset artifact）が Archive 展開を必要とする前に、脅威境界を契約化
した Reader/Writer を提供する必要がある。

## Users and Value

- 後続実装 Issue: #303 が共通 Entry/Error/Limits 型を再利用して TAR
  Streaming を追加、#304 が `ArchiveName` と Limits/Receipt を入力として
  Secure Extraction を実装する。
- Ruviform Consumer: 信頼モデルを明示した ZIP 読み書き
  （Indexed Reader・`verify_all`・明示 finish）を `std::io` 上で利用する。
  実 FS への安全な展開は #304 の責務。

## Scope

### In Scope

- `crates/ruviform-io-archive` 新設・Workspace 登録・`crates/README.md`
  のクレート台帳更新。
- Format 共通公開型（Error/EntryInfo/ArchiveName/EntryKind/Limits/
  Receipt）のコンパイル可能な定義と rustdoc（保証境界・non-RT 明記）。
- ZIP Indexed Reader: EOCD/CD 解析・Entry Index・排他借用
  `ZipEntryReader`（Stored/Deflate）・`finish` 検証・`verify_all`・
  途中 Drop→別 Entry Open。
- ZIP Writer: Entry/Archive の明示 `finish` 状態分離・未 finish Drop の
  非 Completed 扱い・Local Header/CD/EOCD 出力。
- 構造検証: Local Header vs Central Directory 整合（name/method/size/
  CRC）・truncated・overlap（Entry 領域 × CD 領域・Entry 領域同士）・
  Entry 数・大 Header/Metadata の各 Limit を Parse/Allocation 前・途中で
  強制。
- Archive 固有脅威（Zip Slip 的 Path 問題・展開爆弾・CD 不整合）への
  境界方針を rustdoc/README の契約として明記（実 Secure Extraction は
  IO-IMP-10）。
- 上記 Acceptance に対応する unit/integration テスト（否定 fixture 含む）。

### Out of Scope

- Secure Extraction 実装（隔離 Staging・TOCTOU・`ExtractedTree`・
  Recovery）→ IO-IMP-10（Issue #304）。本 Issue は Entry 名の受け渡し
  と Ownership/Limit の**契約合成**まで。
- TAR Reader/Writer → IO-IMP-09（Issue #303）。共通型は TAR を見据えた
  形にするが TAR 固有の Pending/Header 検証は実装しない。
- ZIP64 の読み書き（初期 Unsupported として明示拒否）、Encryption、
  Multi-disk、Stored/Deflate 以外の Method、archive comment、
  data descriptor の Writer 側出力（Writer は Seek により Local Header を
  確定値で patch する）。
- IO-01 StagedFile との publish 合成・実 FS への展開・Consumer 固有
  Schema/意味論・serde 依存・RT 経路。

## Requirements

- `default = []`（feature 最小）。依存は `crc32fast`（CRC32）と
  `flate2`（`rust_backend`、pure Rust のみ）に限定し、いずれも公開 API に
  型・エラー・feature 名を露出しない（Backend 隠蔽）。
- MSRV 1.89・edition 2024・`workspace.lints` 継承・`unsafe` 不使用。
- クレート外から到達可能な全公開 enum に `#[non_exhaustive]`。
- non-RT のみ — すべての解析・展開・圧縮はブロッキング I/O と alloc を
  伴い、RT callback から呼ばない契約を rustdoc に明記する。
- Budget は Parser/内部バッファが確保する**前・途中**に強制する。
  EOCD/CD 宣言値は早期拒否には使うが、実測 Byte 数の代替にしない
  （展開量は出力途中で累積監視し、超過 Byte を下流へ出さない）。
- CRC/Checksum は整合性検出であり真正性の保証ではない、を API 契約に
  明記する。未知 Method・暗号化・ZIP64 必須・Multi-disk は黙って
  処理せず `Unsupported` を返す。
- `EntryWriter`/`ArchiveWriter`/`EntryReader` の `finish` 成功のみを
  正常完了の証拠とし、Drop を完了として扱わない。
- 依存追加は `cargo add` で行い、メンテナンス状況・ライセンス
  （いずれも MIT/Apache-2.0 系）・依存木サイズの理由を PR に記録する。

## Acceptance Criteria

- [ ] AC1: `ruviform-io-archive` が workspace メンバーとして登録され、
      `ArchiveError`/`ArchiveErrorKind`/`ArchiveEntryInfo`/`ArchiveName`/
      `EntryKind`/`ArchiveReadLimits`/`ArchiveReadReceipt`/
      `EntryReadReceipt` が公開型として存在し、後続 Issue が利用する
      名称・意味論が rustdoc で確定している。
- [ ] AC2: EOCD/CD 解析が `max_entries`・`max_metadata_bytes`・
      `max_path_bytes` を Parse/Allocation 前・途中に強制し、truncated・
      大 Header・過剰 Entry 数の否定 fixture が `LimitExceeded`/
      `Truncated`/`Malformed` を返すことをテストで pin する。
- [ ] AC3: Entry Index 読取・途中 Entry Drop→別 Entry Open/再 Open・
      `verify_all` の `ArchiveReadReceipt`（Entry 数・完全読込数・
      実 Payload Byte 等）の正確性をテストで pin する。
- [ ] AC4: CRC 不一致・実展開サイズと宣言値の不一致・Local Header と
      CD の name/method/size 不整合・Entry 領域と CD 領域/他 Entry 領域の
      overlap が `ChecksumMismatch`/`SizeMismatch`/`Inconsistent`/
      `Overlap` として検出されることを否定 fixture で pin する。
- [ ] AC5: `ZipEntryWriter.finish` → `ZipArchiveWriter.finish` の
      型分離があり、Entry を `finish` せず Drop した Archive の
      `finish`/`start_entry` が `InvalidState` を返し未完成を Completed
      扱いしないことをテストで pin する。Writer 出力を自前 Reader で
      読み戻す round-trip 試験がある。
- [ ] AC6: 暗号化 flag・未知 Method・Multi-disk・ZIP64 必須入力が
      `Unsupported` を返すことをテストで pin し、Backend 型
      （`flate2`/`crc32fast`）が公開 API に露出しないこと・
      `default=[]`・MSRV 1.89・non-RT 明記・`unsafe` なしを確認する。
- [ ] AC7: 展開爆弾対策として `max_entry_uncompressed_bytes`/
      `max_total_uncompressed_bytes` が宣言値の早期拒否と実測累積の
      双方で強制され、超過 Byte が下流へ出ないことをテストで pin する。
- [ ] AC8: `ArchiveName`（raw bytes 保持・UTF-8/非 UTF-8 の区別）と
      Limits/Receipt が IO-IMP-10 の入力契約として使える形で公開され、
      Secure Extraction への受け渡し方針が rustdoc/README に明記される。
- [ ] AC9: `mise run fmt` / `mise run check` / `mise run lint` /
      `mise run test` が緑で、上記試験が workspace test から追跡可能。

## Deferred Design Concerns

- ZIP64（EOCD64 locator/record・saturated field の解釈）| Issue #286
  で「Backend 検証後採用候補」と合意・大 Archive の実需が固まってから |
  巨大 Archive を Unsupported と誤認するリスク（本 Issue は明示拒否で
  回避）。
- data descriptor（bit 3）形式の Writer 出力 | `Write` のみの Sink への
  非 Seek 出力の実需が固まってから | 現行は `Write + Seek` 必須で
  patch-back を使うため機能上の欠損はない。
- `ArchiveReadLimits` への `max_compressed_bytes`・`max_path_depth` 等の
  Format 別追加 | IO-IMP-09/10 の実装で実需が確定してから | 早期に抽象を
  増やすと貧弱な共通型に強制されるリスク。
- Entry 属性（mtime・unix mode・外部属性）の公開 | #304 の Extraction
  Policy で必要な属性が確定してから | 全属性を同一意味論に変換しようとして
  Format 差を失うリスク。

## Sources and Relations

- 正本: Issue #302 本文（実装範囲・Acceptance・完了条件）。
- 設計基準: Issue #286（IO-04）v0.1/v0.2 合意済み追補、Issue #282
  （統合設計正本・4 独立クレート方針・Backend 隠蔽・non-RT）。
- 先行実装の参考: `crates/ruviform-io-file`（Issue #295/#296 の
  型状態・Receipt/Failure・Recovery 契約の先例。依存はしない）。
- 検証・横断 Gate: Issue #288（IO-06）・Issue #219（Consumer 分類）。
