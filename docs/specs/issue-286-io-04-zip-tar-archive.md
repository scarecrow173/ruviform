---
id: "SPEC-034UAmZTk0TIdPDXB52i1O"
type: "spec"
status: "approved"
title: "Issue #286 IO-04 ZIP/TAR Archive — Entry API・安全な展開・終了/公開契約 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/plans/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/tasks/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/test-specs/issue-286-io-04-zip-tar-archive.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-302-io-archive-zip.md"
    - "docs/specs/issue-303-io-archive-tar.md"
    - "docs/specs/issue-304-io-archive-extract.md"
    - "docs/designs/issue-302-io-archive-zip.md"
    - "docs/designs/issue-303-io-archive-tar.md"
    - "docs/designs/issue-304-io-archive-extract.md"
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/specs/issue-296-io-file-os-backend.md"
    - "docs/specs/issue-300-io-compression-gzip-deflate.md"
    - "docs/specs/issue-301-io-compression-zstd.md"
    - "docs/designs/issue-300-io-compression-gzip-deflate.md"
    - "docs/designs/issue-301-io-compression-zstd.md"
    - "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/specs/issue-306-io-composition-e2e.md"
    - "docs/designs/issue-306-io-composition-e2e.md"
    - "docs/specs/issue-409-scoped-enumeration.md"
    - "docs/designs/issue-409-scoped-enumeration.md"
    - "docs/designs/rooted-read-capability.md"
    - "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/285"
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

## What Should Be Built

IO-04「ZIP/TAR Archive — Entry API・安全な展開・終了/公開契約」
（Issue #286、親 #282）の **正本設計文書** を `docs/` チェーン
（spec → design → test-spec → plan → task）として作成する。
具体的には `docs/designs/issue-286-io-04-zip-tar-archive.md`
を新設し、Issue #286 本文の合意済み設計（v0.1・v0.2、
2026-11-04）と最終整合性レビュー追記（実装 Issue 分解
#302/#303/#304）を、次の構成要素として文書化する。

1. **Reader/Writer/Entry モデル**: ZIP（`Read + Seek` の
   indexed reader/writer）と TAR（`Read`/`Write` の streaming
   reader/writer）という形式別 capability を尊重し、単一の
   不十分な最小 trait へ統合しない。共通の format 中立型
   （`ArchiveName`/`EntryKind`/`ArchiveEntryInfo`）と、
   形式が実際に記録する情報のみを保持する原則。
2. **Backend 隠蔽**: ZIP/TAR の具体 backend（外部 `zip`/`tar`
   crate）を公開せず、内部 Parser + `flate2`（rust_backend）・
   `crc32fast` の非公開依存で構成する契約。Serde 必須依存なし・
   `std::io` ベース・公開型は Ruviform 独自型のみ。
3. **Entry 所有権と明示 finish**: Entry Reader/Writer の親
   Archive への排他的 `&mut` 借用、`finish(self)` 系の明示的
   完了証明、TAR の `Ready`/`EntryOpen`/`Pending`/`Failed`
   状態機械と `discard_pending`/`discard_remaining`、途中 Drop
   を未検証のまま扱う規則。
4. **検証済み終了と Receipt**: `verify_all`/`finish` が返す
   `ArchiveReadReceipt`/`EntryReadReceipt`（entries_total/
   verified/unverified/payload_bytes・`EntryChecksum`・
   `EntryDisposition`）が `verified: bool` 一値に丸めない契約。
   ZIP CRC32 と TAR payload checksum 非存在（`NotPresent`）の
   区別、真正性 ≠ 整合性検出の明記。
5. **Secure Extraction**: `SafeRelativePath` の Path Policy
   （traversal/絶対/drive/UNC/予約名/ADS/非 UTF-8 拒否）、
   canonical 衝突（exact/Unicode NFC/case-fold/file-dir）、
   link/special 拒否、`ArchiveReadLimits` の書込前・実測両層
   budget、排他的隔離 Staging Root、handle/FD 相対 + no-follow
   + CreateNew の OS primitive、identity 照合付き cleanup と
   `ExtractionRecoveryHandle`、TOCTOU を「保証できる範囲」と
   「保証しない範囲」で正直に分ける脅威モデル。
6. **終了/公開契約の分離**: Entry `finish` → Archive `finish`
   → Compression `finish`（tar.gz 等の合成層）→ IO-01
   `StagedFile` seal/publish の各成功を混同しない契約。
   `ExtractedTree` は検証済み Staging Tree の所有権であり、
   不変 snapshot・Directory 原子的 Publish・Consumer Commit
   ではない。Directory replace/rename の原子性は OS 依存で
   無条件保証しない。
7. **Format 初期サポート範囲**: ZIP = classic（非 ZIP64）・
   Stored/Deflate・暗号化なし。TAR = USTAR/GNU magic・PAX
   `x`/`g`/`X`・GNU `L`/`K`・base-256 数値読取。sparse・
   special・暗号化・未知 method・multi-disk・ZIP64-required は
   `Unsupported`。
8. **Consumer 利用面**: AudioLens（Manifest/Dataset/Archive
   要件・#139/#143）と Synvelis（Project Snapshot・ZIP/TAR
   Backup・Secure Import・#82/#84/#25）が schema/version を
   Ruviform に移さず利用できる面と責務分界。
9. **他 IO gate との証拠項目対応**: Issue #286 の
   API/ZIP/TAR/SEC/EXT/INT/COMP/DEP/OWN/WRT/REC/OS 証拠項目と
   Design Acceptance Gate を、per-implementation test-spec
   （#302/#303/#304）・#288 matrix 行（A-N1〜A-X5）・合成 E2E
   （#306）へ対応付ける索引。

## Why Now

- IO-04 の実装は main に揃っている: `ruviform-io-archive`
  クレートが #302（ZIP・PR #352）・#303（TAR・PR #357）・
  #304（Secure Extraction・PR #362）で merge 済み。ただし
  Issue #286（設計 Issue）自体の正本設計文書は未作成で、
  ZIP/TAR の形式別契約・安全な展開・終了/公開契約が
  per-implementation 文書（#302/#303/#304）に分散した
  ままである。
- IO-06 cross-consumer gate（#288）の受入 matrix は IO-04 を
  A-N1〜A-X5 の行で参照しており、その根拠となる設計正本が
  #286 に求められている。
- 合成 E2E（#306・`ruviform-io-e2e`）と IO-01 File 契約
  （#295/#296）・IO-03 Compression（#300/#301）・rooted
  capability（#409）が main に存在し、Archive→StagedFile・
  tar.gz 合成の実 OS 証拠が得られている。#286 の設計文書は
  これらの実装成果を消費して「設計として何が固定され、
  何が Consumer 責務か」を一箇所に記録する役割を持つ。

## Users and Value

- **Ruviform maintainer / IO-04 Owner**: Entry API・所有権/
  finish 契約・secure extraction の脅威モデル・OS/FS 別保護
  レベル・budget・error 分類の正本を得て、後続の Format 追加・
  backend 更新・Consumer 統合の判断基準を固定できる。
- **AudioLens / Synvelis Owner**: ZIP/TAR Backup・Secure
  Import 等の利用面と、schema/version/Project semantics を
  Ruviform に移さない責務分界、publish 前検証の順序を確認
  できる。
- **#288（IO-06 gate）Owner**: IO-04 matrix 行（A-N1〜A-X5）
  の根拠となる設計正本と証拠項目の対応を辿れる。
- **後続の実装・保守担当**: per-implementation 詳細は
  #302/#303/#304 の設計・test-spec を正本として参照し、本書
  では横断契約のみを確認できる。

## Scope

### In Scope

- `docs/` 配下の文書チェーン（spec/design/test-spec/plan/task）
  と生成 index のみ。**コード変更なし**。
- Issue #286 本文（合意済み設計 v0.1・v0.2・2026-11-04・
  最終整合性レビュー追記）の要求事項の文書化。
- IO-01（#295/#296）・IO-02（#284）・IO-03（#300/#301）・
  per-implementation 実装（#302/#303/#304）・合成 E2E
  （#306）・rooted capability（#409）・cross-consumer gate
  （#288）の main 上の設計・実装成果を消費した、Reader/
  Writer/Entry 契約・所有権/finish・検証済み終了・secure
  extraction・終了/公開契約・Consumer 利用面・証拠項目対応の
  記述。

### Out of Scope

- `crates/`・`Cargo.toml`・`Cargo.lock`・`mise.toml`・
  `.github/` の変更（本 slice は docs 設計専用）。
- per-implementation の詳細 API・parser 内部・test 規則の
  再定義（#302/#303/#304 の設計・test-spec が正本。本書は
  横断契約と索引のみを所有する）。
- AudioLens/Synvelis repo の変更・実測（cross-repo gate、
  #219/#294 の consumer-verify が Owner）。
- 新規 CI Workflow・証拠 YAML/JSON スキーマ・Dashboard の
  作成。
- Directory 原子的 Publish・Consumer 固有の Project
  Commit/CAS/Backup schema の Ruviform 側実装（各 Consumer
  責務）。
- ZIP64・暗号化 ZIP・sparse・special file 復元・nested
  archive の受理（初期範囲外・`Unsupported`）。

## Requirements

- Reader/Writer/Entry 契約: ZIP `Read + Seek` indexed・TAR
  `Read`/`Write` streaming の形式別 API 族、共通 format 中立
  型（`ArchiveName`/`EntryKind`/`ArchiveEntryInfo`）と
  「形式が実在する情報のみ記録」の原則を明文化する。
- Backend 隠蔽: 外部 `zip`/`tar` crate 非採用・内部 Parser +
  非公開 codec 依存（`flate2` rust_backend・`crc32fast`）・
  Serde 非依存・`std::io` ベース・公開型は Ruviform 独自型
  のみ、を根拠（実装・test-spec の audit 規則）と共に明文化
  する。
- 所有権/finish 契約: Entry の排他的借用・明示 `finish` の
  みが完了証拠・TAR Pending 状態機械と discard 経路・Writer
  poison を明文化する。
- 検証済み終了: `ArchiveReadReceipt`/`EntryReadReceipt` の
  typed summary・`EntryChecksum`/`EntryDisposition`・ZIP CRC
  vs TAR checksum 非存在・真正性と整合性の区別を明文化する。
- Secure Extraction: 信頼モデル（Untrusted 既定）・拒否規則
  （Path Policy・canonical 衝突・kind/budget）・OS/FS 別保護
  レベル（unix dirfd / windows NT handle 相対 / その他
  `Unsupported`）・TOCTOU 保証境界・cleanup/Recovery を
  明文化する。
- 終了/公開契約: Entry→Archive→Compression→File publish の
  finish 連鎖の分離、`ExtractedTree` の保証境界、Directory
  Publish 非保証を明文化する。
- Format 範囲: ZIP classic Stored/Deflate・TAR
  USTAR/GNU/PAX の初期受理範囲と `Unsupported` 分類を明文化
  する。
- Consumer 利用面: AudioLens/Synvelis が schema/version を
  Ruviform に移さず利用できる面と責務分界を記述する。
- 証拠項目対応: Issue #286 の API/ZIP/TAR/SEC/EXT/INT/COMP/
  DEP/OWN/WRT/REC/OS 証拠項目と Design Acceptance Gate を、
  既存 per-implementation test-spec（#302/#303/#304）・
  #288 matrix 行（A-N1〜A-X5）・#306 E2E へ対応付ける索引を
  設計文書に含める。
- 「設計の合意 ≠ テスト完了」を明記し、実装済み部分と
  未検証・Consumer 側未実施部分を分離して記録する。

## Acceptance Criteria

- [ ] ZIP/TAR の具体な Ruviform 型・Reader/Writer/Entry
  API・所有権/seek 差異を含む正本設計文書が `docs/designs/`
  に存在する。
- [ ] safe extraction の脅威モデル・拒否規則・OS/FS 別の
  保護レベル表が記述されている。
- [ ] limits・入出力/展開時の実績計測・Cancellation・途中
  失敗時の保全が記述されている。
- [ ] Binary Golden / negative fixtures（Zip Slip、Symlink、
  hardlink、name collision、zip bomb）への対応付けが記述
  されている。
- [ ] Compression との組合せ（tar.gz 等）と backend type
  非露出の確認が記述されている。
- [ ] Archive→StagedFile publish で正常終了を契約化する
  記述がある。
- [ ] Consumer 固有 Backup schema/Project semantics を
  Archive へ混入しない記述がある。
- [ ] 実装可能な詳細設計/ADR/試験項目への参照索引がある。
- [ ] 変更が `docs/` のみであり、`crates/`・`Cargo.toml`・
  `Cargo.lock`・`mise.toml`・`.github/` に差分がない。

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
  Consumer Commit/CAS | IO-01 単一 File Publish の拡張解釈
  ではなく別契約。Consumer 要件確定時 | `ExtractedTree` を
  正式公開へ繋ぐ契約が未定義のまま残るリスク。
- `openat2`/`RESOLVE_BENEATH`・`cap-std` 採用・非 UTF-8
  名受理 | 現行の dirfd/handle 相対 + no-follow +
  CreateNew で主要脅威をカバー。kernel 保証の追加検証は
  後続候補 | 残存する理論上の probe→delete race の扱い。
- Consumer 側の実測・移行（SV ZIP/TAR Backup・Secure
  Import、AL archive 要件） | cross-repo gate（#219/#294
  consumer-verify）と各 Consumer Issue が Owner であり本
  slice の対象外 | 設計どおりの互換性が実測で
  裏付けられないまま採用されるリスク。
- macOS/Linux 実機での extract backend 実測 | 本 slice の
  作業環境は Windows のみ。unix backend は `check-linux`
  で型/lint 補助済み、実機試験は CI lane 依存 | OS/FS 差異
  （reparse point・dir sync 等）が未検証のまま残るリスク。

## Sources and Relations

- 正本 Issue: [#286](https://github.com/scarecrow173/ruviform/issues/286)
  （IO-04 合意済み設計 v0.1・v0.2・2026-11-04・最終整合性
  レビュー追記 — 実装 Issue #302/#303/#304）。
- 親設計正本: [#282](https://github.com/scarecrow173/ruviform/issues/282)
  （汎用 I/O 基盤 横断設計）。
- 実装 Issue: IO-IMP-08/09/10 = #302/#303/#304（merge 済み、
  PR #352/#357/#362）。IO-01 = #295/#296。IO-03 =
  #300/#301。IO-05 = #305。合成 E2E = #306。rooted
  capability = #409。
- Gate 接続: #288（IO-06 cross-consumer gate — 本書の証拠
  項目の一部が同 gate の IO-04 matrix 行 A-N1〜A-X5 の
  根拠となる）。
- Consumer Issue: AudioLens #139/#143、Synvelis #82/#84/#25、
  Synvelis Project #17（PAP-14）（状態は 2026-11-09 時点の
  snapshot）。
- 参照設計文書: `docs/designs/issue-302-…`・`issue-303-…`・
  `issue-304-…`（per-implementation 正本）、`docs/designs/
  issue-295-…`・`issue-296-…`（IO-01 File 契約）、
  `docs/designs/issue-300-…`・`issue-301-…`（IO-03
  Compression）、`docs/designs/issue-306-…`（合成 E2E）、
  `docs/designs/issue-409-…`・`rooted-read-capability.md`
  （rooted capability）、`docs/designs/issue-288-…`
  （cross-consumer gate）、`crates/ruviform-io-archive/`
  （実装済み crate — 設計の確定値の根拠）。
