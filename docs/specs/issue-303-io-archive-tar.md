---
id: "SPEC-h6TZICrhpaneHHENSs2SP8"
type: "spec"
status: "approved"
title: "Issue #303 ruviform-io-archive — TAR Streaming Entry/Pending・PAX/GNU・EOF検証 (IO-IMP-09)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/303"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-archive/src/tar.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/tar/header.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/tar/read.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/tar/write.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/tar_reader.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/tar_writer.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-archive/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/entry.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/receipt.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/README.md"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/common/mod.rs"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-302-io-archive-zip.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/specs/issue-300-io-compression-gzip-deflate.md"
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

# Issue #303 ruviform-io-archive — TAR Streaming Entry/Pending・PAX/GNU・EOF検証 (IO-IMP-09)

## What Should Be Built

既存 `crates/ruviform-io-archive`（IO-IMP-08 / Issue #302 で新設済みの
scaffold Owner）に `tar` module を追加する。新規クレートは作らない。

TAR は Central Directory を持たない逐次形式であり、ZIP の indexed
契約ではなく IO-04（Issue #286）v0.2 で合意済みの **Streaming +
Pending** 契約に従う。共通型（`ArchiveError`/`ArchiveEntryInfo`/
`ArchiveName`/`EntryKind`/`ArchiveReadLimits`/Receipt）は #302 確立物を
再利用し、TAR に実在する情報のみを記録する。

公開面（`tar` module）:

- `TarArchiveReader<R: Read>` — 逐次 Reader。`next_entry()` で
  `Option<TarEntryReader>` を返す Streaming API。`Read + Seek` を要求
  しない（圧縮 decoder 等の one-way stream にそのまま乗る）。
- `TarEntryReader<'_, R>` — 親 Reader の排他的 `&mut` 借用。
  `std::io::Read` を実装し、宣言 size で上限を打つ。
- Reader 状態機械: `Ready -> EntryOpen -> Ready`。Entry Reader の
  途中 Drop は `Pending`（残余 byte を明示消費するまで `next_entry`
  と archive `finish` を `InvalidState` で拒否）。I/O・構造エラーで
  継続不能なら `Failed`。回復は `discard_pending()`（budget 内で残
  payload + padding を消費）のみ。
- `TarEntryReader::finish(self)` — 呼出し側が payload を完全消費した
  場合のみ成功する `ReadToEnd` 証明。`discard_remaining(self)` は残
  りを budget 内で消費して `Discarded` 証明を返す。`EntryDisposition`
  で両者を区別する。
- TAR payload には内容 checksum が無い — Receipt の
  `EntryChecksum::NotPresent` で「終端まで読み切った」ことと
  「真正性/破損不存在」を混同しない契約を明示する。
- `TarArchiveReader::finish(self)` — 未消費 Entry を budget 内で
  discard 検証した後、TAR 終端（zero block + 全 zero 末尾）を検証し
  `ArchiveReadReceipt` と inner を返す。`Pending` 中は拒否する。
- `TarArchiveWriter<W: Write>` — `start_entry(name, declared_size,
  options)` → `TarEntryWriter`（`Write`、宣言長で cap）→ Entry
  `finish`（実書込 == 宣言の確認 + padding 出力）→ Archive `finish`
  （2 zero block 終端）。未 finish Drop は archive を poison し、
  後続を `InvalidState` にする。`Write + Seek` は要求しない。
- TAR 構造検証: header checksum・宣言サイズ・padding・USTAR
  （name prefix 合成）・GNU（`ustar ` magic・`L`/`K` longname・
  base-256 数値）・PAX（`x`/`g`/`X` record の `path`/`linkpath`/
  `size` 実効値）・終端マーカー・後続 byte を分類済みエラーで区別。
- Sparse（`S` typeflag）・Special（`3`/`4`/`6` char/block/fifo）・
  その他未対応 typeflag は `Unsupported` で明示拒否。symlink/
  hardlink（`1`/`2`）は Entry メタ（`EntryKind` + link target）として
  露出するに留め、抽出時の安全処理は IO-IMP-10（#304）の範囲。

## Why Now

Issue #302 の scaffold が main に入り、共通 Entry/Error/Limits 契約が
確定した。#304（Secure Extraction）は TAR 入力を主要対象とするため、
Pending/discard 契約と PAX/GNU 実効名解決を先に固定しないと
Extraction 層が設計できない。IO-03（#300 gzip）も merged 済みであり、
`tar.gz` 二層 finish の組合せ境界を本 Issue で契約化する。

## Users and Value

- 後続実装 Issue: #304 が `TarEntryReader` の Pending/discard 契約と
  `ArchiveName`/link target メタを入力として Secure Extraction を
  実装する。
- Ruviform Consumer: POSIX/GNU 系ツールが生成する `.tar` /
  `.tar.gz` / `.tar.zst` を、脅威境界を契約化した Streaming Reader で
  検証しながら読む。`tar.gz` 時の Compression decoder finish（CRC/
  trailer）は IO-03 の契約として分離される。

## Scope

### In Scope

- `crates/ruviform-io-archive` への `tar` module 追加（`tar.rs` +
  `tar/header.rs` + `tar/read.rs` + `tar/write.rs`）。内部 Parser は
  自前実装（`tar` crate の Iterator は暗黙 skip/先読みを行い Pending
  契約を満たせないため不採用 — 外部 backend 依存は追加しない）。
- `TarArchiveReader` / `TarEntryReader` / `TarArchiveWriter` /
  `TarEntryWriter` / `TarEntryOptions` の公開型と rustdoc
  （状態機械・Pending/discard 契約・non-RT 明記）。
- 共通契約の整合的拡張（必要最小限）:
  - `EntryReadReceipt` に `EntryDisposition::{ReadToEnd, Discarded}`
    を追加（TAR の discard 証明を型で区別。ZIP は常に `ReadToEnd`）。
  - `ArchiveEntryInfo` に `link_name: Option<ArchiveName>` を追加
    （TAR `1`/`2` typeflag の link target を Entry メタとして露出。
    ZIP は `None`）。
- TAR 構造検証: header 固定 512 byte・checksum（chksum field を
  space 扱いした全 byte 和）・octal/base-256 数値 field・USTAR
  prefix 合成・GNU `L`/`K`・PAX record 文法・終端 zero block・
  後続 byte の分類。
- Budget 強制: `max_entries`（surfaced Entry 数・途中でも打切）、
  `max_metadata_bytes`（header block + 拡張 payload の累積）、
  `max_path_bytes`（実効 name/link target）、
  `max_entry_uncompressed_bytes`（宣言値の事前拒否）、
  `max_total_uncompressed_bytes`（read/discard の累積・途中遮断）。
- `tests/tar_reader.rs`・`tests/tar_writer.rs`・共通 fixture builder
  拡張による否定試験（bad checksum・truncated header・非 octal
  field・超過 budget・不正 typeflag・Sparse/Special・Pending 違反
  等）。
- crate README・crate doc・`crates/README.md` 台帳の更新。

### Out of Scope

- Secure Extraction 実装（path traversal 拒否・symlink/hardlink の FS
  作成・TOCTOU・Staging/Recovery）→ IO-IMP-10（Issue #304）。
  本 Issue は `ArchiveName`/link target の受け渡しまで。
- Sparse file（GNU `S`）・Special file（`3`/`4`/`6`）の「復元」
  （Entry メタとしても `Unsupported` で拒否）。GNU multi-volume
  （`M`）・old GNU longname（`N`）・dump dir（`D`）等も同様に
  `Unsupported`。
- `tar` backend crate・外部アーカイブライブラリへの依存追加。
- gzip/zstd codec の実装（IO-03 既存）。`tar.gz` 合成は Consumer 側の
  明示的 compose であり、TAR 層は decode 済み stream だけを見る。
- Indexed/random access の fake 実装（TAR に中央 Index は存在せず、
  全走査 index の提供は行わない）。
- Writer 側の symlink/hardlink 出力・sparse・独自拡張出力・
  blocking factor padding（20 block 単位の record padding は書かず、
  仕様最小の 2 zero block のみ）。
- IO-01 StagedFile との publish 合成・実 FS への展開・Consumer 固有
  schema・serde 依存・RT 経路。

## Requirements

- `default = []` 維持・新規依存なし（TAR Parser は crate 内実装。
  `crc32fast`/`flate2` は ZIP 用の既存内部依存で TAR 経路では使わない）。
- MSRV 1.89・edition 2024・`workspace.lints` 継承・`unsafe` 不使用。
- クレート外から到達可能な全公開 enum に `#[non_exhaustive]`。
- non-RT のみ — 全ての解析・検証はブロッキング I/O と alloc を伴い、
  RT callback から呼ばない契約を rustdoc に明記する。
- Budget は Parser/内部バッファが確保する**前・途中**に強制する。
  PAX/GNU 拡張 payload の宣言 size は alloc 前拒否に使い、累積
  metadata byte も途中で強制する。
- `finish` 成功のみを正常完了の証拠とし、Drop を完了として扱わない。
- TAR EOF（zero block + 全 zero 末尾）と下層 Compression stream の
  EOF/trailer（IO-03 decoder `finish`）は別契約 — TAR 層の `finish`
  成功だけで圧縮 stream を検証済みと主張しないことを rustdoc に明記。

## Acceptance Criteria

- [ ] AC1: `tar` module が `TarArchiveReader<R: Read>` /
      `TarEntryReader` / `TarArchiveWriter<W: Write>` /
      `TarEntryWriter` / `TarEntryOptions` を公開し、`next_entry()`
      /`finish(self)`/`discard_pending()`/`discard_remaining(self)`
      の状態機械と契約が rustdoc で確定している。
- [ ] AC2: Entry Reader 途中 Drop → `next_entry`/`finish` が
      `InvalidState`（Pending 拒否）→ `discard_pending` で budget 内
      消費・正常復帰、drain 失敗（I/O・構造）で `Failed` となることを
      テストで pin する。
- [ ] AC3: Header checksum 不一致・header block の short read・
      octal でも base-256 でもない数値 field・宣言 size に対する実
      データ不足・non-zero padding・終端 marker 無しの EOF・終端後の
      非 zero 後続 byte が、設計どおりの `ArchiveErrorKind`
      （`ChecksumMismatch`/`Truncated`/`Malformed`）に分類されることを
      否定 fixture で pin する。
- [ ] AC4: USTAR name+prefix 合成・GNU `L`/`K` longname・PAX `x`
      record の `path`/`linkpath`/`size` 実効値が raw bytes を失わず
      `ArchiveName`/link target に反映されることをテストで pin する。
      `x`/`g`/`L`/`K`/`X` 以外の typeflag・`S` sparse・`3`/`4`/`6`
      special が `Unsupported` で拒否されることも pin する。
- [ ] AC5: `max_entries`・`max_metadata_bytes`・`max_path_bytes`・
      `max_entry_uncompressed_bytes`・`max_total_uncompressed_bytes`
      が事前・途中の双方で強制され、超過時は `LimitExceeded` で超過
      byte を下流へ出さないことをテストで pin する。
- [ ] AC6: `finish`（ReadToEnd）と `discard_remaining`（Discarded）で
      Receipt の `EntryDisposition` が区別され、TAR の Receipt が
      `EntryChecksum::NotPresent` を返すこと（内容 checksum 非存在の
      明示）・Archive `finish` の `ArchiveReadReceipt`（entries_total/
      verified/unverified/payload_bytes）が正確であることを pin する。
- [ ] AC7: Writer が宣言長と実書込量の境界を守ること
      （write cap・不足 finish → Err + archive poison・padding 出力・
      `finish` せず Drop → `InvalidState`）と、長名の USTAR prefix
      分割・PAX `x` 出力・大 size の base-256/PAX 表現をテストで pin
      する。Writer 出力を自前 Reader で round-trip する試験がある。
- [ ] AC8: `tar.gz` 合成（テストでは decode 済み stream 上の TAR
      finish と compression decoder 側 finish が独立に検証されること）
      の二層境界をテストと rustdoc で pin する。
- [ ] AC9: `mise run fmt` / `mise run check` / `mise run lint` /
      `mise run test` が緑で、ZIP 側既存テストが退行しないこと。
      MSRV 1.89・`unsafe` なし・`default=[]`・non-RT 明記・依存 DAG
      （io-compression 等への新規依存なし）を確認する。

## Deferred Design Concerns

- `EntryDisposition` の `Discarded` を「明示 skip」以外（例: 範囲
  read）へ拡張するか | 実需が確定してから | 半読み skip を精密に
  分類し過ぎて API が複雑化するリスク。
- `next_entry` 失敗時に自動 `Pending`（明示 skip 可能な Unsupported
  Entry の透過 skip）を許すか | #304 の Extraction 要件確定後 |
  Unsupported で Reader 全体を Failed にする現行契約は保守的だが
  厳しすぎる可能性。
- PAX `g`（global）record の `path`/`linkpath`/`size` 適用 | POSIX
  解釈の実装間差と実需が確定してから | global が全 Entry の実効名を
  書換える挙動は安全側に無効化している（構造検証のみ）。
- Writer の symlink/hardlink 出力・GNU `L`/`K` 出力・blocking factor
  padding | Consumer の出力互換要件が固まってから | 出力は USTAR+
  必要時 PAX `x` の最小構成であり、GNU 独自拡張を読み専用とする
  非対称を許容している。
- Indexed access（全走査で構築する仮想 index） | Consumer の random
  access 実需 | 逐次形式へ index 契約を誤って被せるリスク（本 Issue
  は streaming 契約のみで明示的に対象外）。
