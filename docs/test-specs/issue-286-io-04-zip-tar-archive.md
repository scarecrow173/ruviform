---
id: "TSPEC-034UAuXWuQzQh1Tt9WnR1N"
type: "test-spec"
status: "approved"
title: "Issue #286 IO-04 ZIP/TAR Archive — Entry API・安全な展開・終了/公開契約 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/286"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-286-io-04-zip-tar-archive.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-286-io-04-zip-tar-archive.md"
    - "docs/designs/issue-286-io-04-zip-tar-archive.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #286 IO-04 ZIP/TAR Archive — Entry API・安全な展開・終了/公開契約 設計

## Purpose

Issue #286 の設計正本（[design](../designs/issue-286-io-04-zip-tar-archive.md)）
が「形式別 Reader/Writer/Entry 契約・Backend 隠蔽・所有権/
finish・検証済み終了 Receipt・Secure Extraction の脅威モデル・
終了/公開契約・Format 範囲・Consumer 利用面・証拠項目対応を、
実在する per-implementation 正本（#302/#303/#304）・IO-01/
IO-03・#288 matrix・#306 E2E への参照で構成し、設計合意と
実装/検証の完了を混同していない」ことを検証する。対象 spec は
[spec](../specs/issue-286-io-04-zip-tar-archive.md)、実施記録は
[task](../tasks/issue-286-io-04-zip-tar-archive.md) /
[plan](../plans/issue-286-io-04-zip-tar-archive.md) を参照。
コード変更を伴わない docs 成果物のため、検証は文書検査
（link/relation 解決・参照先の実在確認・内容の突合・lint）で
行う。本 test-spec は横断設計文書の正しさを対象とし、
per-implementation の実走テスト自体の合否・Consumer 実測・
macOS/Linux 実機 backend は対象外（実装/受入段階の要件）。

## Feature

IO-04 ZIP/TAR Archive 横断設計文書（形式別 Reader/Writer/
Entry 契約・所有権/finish・検証済み終了・Secure Extraction・
終了/公開契約・Consumer 利用面・証拠項目対応）

## Rules

- **R1（形式別 Reader/Writer/Entry 契約の完全性）**: 設計
  文書は ZIP（`Read + Seek` の indexed Reader/Writer・
  `entry_count`/`entry_info`/`open_entry`/`verify_all`/
  `into_inner`・`ZipEntryOptions`/`ZipCompressionMethod`）と
  TAR（`Read`/`Write` の streaming Reader/Writer・
  `next_entry`/`entry_count`/`has_pending_entry`/
  `discard_pending`/`finish`/`into_inner`・
  `TarEntryOptions`）の形式別 API 族を区別し、単一の
  不十分な最小 trait へ統合していないこと、format 中立の
  共通型（`ArchiveName`/`EntryKind`/`ArchiveEntryInfo`・
  「形式が実在する情報のみ記録」）を明記すること、Entry の
  排他的 `&mut` 借用・明示 `finish` のみが完了証拠・
  `Drop` が完了を意味しないことを記述する。検証根拠として
  `issue-302`/`issue-303` 設計・`src/{zip,tar,entry}.rs`・
  #288 matrix A-N1 への参照を持つ。
- **R2（Backend 隠蔽と driver 中立）**: 設計文書は外部
  `zip`/`tar` crate 非採用・内部 Parser + 非公開 codec
  依存（`flate2` rust_backend・`crc32fast`・
  `unicode-normalization`・`ruviform-io-file` 共有 vfs）・
  Serde 必須依存なし・`std::io` ベース・公開型は Ruviform
  独自型のみ・backend 型/Error/feature/version の非公開・
  `source()` の `dyn Error` 経由のみの診断到達を明記する。
  検証根拠として `crates/ruviform-io-archive/Cargo.toml`
  pin comment・`issue-302`/`303`/`304` 設計 Non-Functional
  Constraints・#288 matrix A-X4 への参照を持つ。
- **R3（参照の実在性）**: 設計文書・spec・plan・task が
  参照する Issue 番号（#282〜#306・#409・#219・#294・
  AL #139/#143・SV #17/#25/#82/#84）・PR 番号（#352/#357/
  #362）・docs パス（per-implementation spec/design/
  test-spec・IO-01 #295/#296・IO-03 #300/#301・#288・
  #306・#409・`rooted-read-capability.md`・baseline）・
  crate 名（`ruviform-io-archive`・`ruviform-io-file`・
  `ruviform-io-compression`・`ruviform-io-e2e`）・mise
  task 名（`test-io-file`/`check-io-file`/`lint-io-file`/
  `check-linux`/`test-io-e2e`）が作成時点で実在する。存在
  しない Issue/文書/command を根拠として記載しない。
- **R4（所有権と状態機械）**: ZIP Reader の open/entry
  finish/Drop 後再 open/`verify_all`・TAR Reader の
  `Ready`/`EntryOpen`/`Pending`/`Failed` 状態機械・
  `discard_pending`/`discard_remaining`/`finish` の
  drain 検証・Writer の `Ready -> EntryOpen -> Ready ->
  Finished`・poison・`InvalidState`/`Unsupported` の分離・
  `entry.finish()` が「read した」のではなく「検証済み
  receipt を返した」ことを記述する。検証根拠として
  `issue-302` R6・`issue-303` R6/R7・#302/#303 設計
  「Reader/Writer 状態機械」節への参照を持つ。
- **R5（Receipt 契約）**: `EntryReadReceipt`（`info`・
  `uncompressed_bytes` 実測・`checksum`・`disposition`）と
  `ArchiveReadReceipt`（`entries_total`/`entries_verified`/
  `entries_unverified`/`payload_bytes`）が typed summary で
  あり `verified: bool` 一値に丸めないこと、ZIP CRC32 と
  TAR payload checksum 非存在（`EntryChecksum::NotPresent`）
  の区別、真正性 ≠ 整合性検出であること、`EntryDisposition`
  （`ReadToEnd`/`Discarded`）が構造検証と内容消費を分ける
  ことを記述する。
- **R6（Secure Extraction）**: 信頼モデル（Untrusted 既定・
  `ExtractOptions` は budget と copy buffer のみで policy
  を緩めない）・`SafeRelativePath` の Path Policy（非空・
  UTF-8・`/` 区切りのみ・`..`/`.`/空/絶対/`\\`/`:`/drive/UNC/
  Windows 予約名/ADS/trailing dot-space/制御文字の拒否・
  component ≤255 byte・`max_path_bytes`/`max_path_depth`）・
  canonical 衝突（`char::to_lowercase` + NFC・exact/file-
  dir/case-fold/Unicode 衝突の拒否・ZIP plan-before-write・
  TAR 逐次検出）・kind 拒否（link/special は `Unsupported`）
  ・`ArchiveReadLimits` の宣言事前拒否 + 実測途中遮断・
  排他的隔離 Staging Root（`create` は信頼 parent 下に一意
  名・排他作成・`0700`/`0600` 相当）・OS/FS 別保護レベル
  表（unix dirfd+`O_NOFOLLOW`/`O_DIRECTORY`/`O_CREAT|O_EXCL`
  + identity 照合、Windows `NtCreateFile`/`NtOpenFile`/
  `NtDeleteFile` `RootDirectory` 相対 + `OBJ_DONT_REPARSE`
  + `FILE_(NON_)DIRECTORY_FILE` + identity 照合、その他
  `Unsupported`）・TOCTOU の正直な境界・identity 照合
  cleanup・`ExtractionRecoveryHandle`（`inspect`/
  `cleanup`・Drop は削除しない）を記述する。
- **R7（Format 初期サポート範囲）**: 受理/`Unsupported` の
  対応表（ZIP = classic 非 ZIP64・Stored/Deflate・非暗号化・
  UTF-8 flag/raw name、未知 method/multi-disk/ZIP64-required/
  暗号化は拒否・write は Stored/Deflate・UTF-8・classic
  field・data descriptor/archive comment/ZIP64 出力は拒否。
  TAR = USTAR/GNU magic・PAX `x`/`g`/`X`・GNU `L`/`K`・
  base-256 数値読取、sparse `S`/special `3`/`4`/`6`/multi-
  volume/未知 typeflag は拒否、write は USTAR+PAX `x`・
  link/special/sparse 出力は拒否）が `crates/ruviform-io-
  archive/src/{zip,tar,limits,error}.rs`・crate README・
  #302/#303/#304 設計の対応表と一致する。
- **R8（終了/公開契約と Compression 合成）**: Entry
  `finish` → Archive `finish` → Compression `finish`
  （tar.gz/tar.zst 等の 2 層）→ IO-01 `StagedFile` seal/
  publish の各成功を混同しないこと、TAR 層の finish は
  decode 済み stream 内の終端のみ証明し圧縮 trailer/
  checksum は IO-03 decoder `finish`/`VerifiedEnd` に分離
  されること、ZIP 内部 codec は `ZipCompressionMethod` に
  閉じ込め IO-03 への必須依存を作らないこと、`ExtractedTree`
  が検証済み Staging Tree の所有権であり不変 snapshot・
  Directory 原子的 Publish・Consumer Commit/CAS ではない
  こと、Directory replace/rename の原子性を無条件保証
  しないことを記述する。
- **R9（証拠項目対応の網羅性）**: Issue #286 の API/
  ZIP/TAR/SEC/EXT/INT/COMP/DEP/OWN/WRT/REC/OS 証拠項目と
  Design Acceptance Gate 全項目が、実在する per-implementation
  test-spec rule/節（`issue-302`/`303`/`304`）・#288 matrix
  行（A-N1〜A-X5・C-X4・E-N1 等）・実装 Issue/PR（#302/
  #303/#304・PR #352/#357/#362）へ対応付けられている。
  存在しない rule 番号・matrix 行・Issue/PR を根拠にしない。
- **R10（Consumer 利用面）**: AudioLens（Manifest・
  schema.json・index.json・Dataset/Artifact Archive 要件・
  Job 再開・Artifact hash・数値精度・#139/#143）と Synvelis
  （Project Snapshot・ZIP/TAR Backup・Secure Import・
  DirectoryBundleStore/Snapshot/HEAD・#82/#84/#25・SV
  Project #17 PAP-14）が schema/version/migration/
  canonical signed bytes/manifest semantics を Ruviform に
  移さず利用できる面として記述され、`ExtractedTree` ≠
  Project Transaction・Consumer 独立判定（#219/#294
  consumer-verify・#288 gate）が明記される。
- **R11（設計/実装分離）**: 「設計の合意 ≠ テスト完了・
  Consumer 受入完了」を明記し、実装済み部分（#302/#303/
  #304 merge）と未検証・Consumer 側未実施部分（consumer-
  verify tuple・macOS/Linux 実機・性能比較）を分離して
  `not run`/`NotRun`/`Unsupported` と記録する方針が示される。
- **R12（変更範囲）**: 差分が `docs/` の新規 `issue-286-*`
  文書と生成 index のみであり、`crates/`・`Cargo.toml`・
  `Cargo.lock`・`mise.toml`・`.github/` に変更がない。

## Examples

- Given Issue #286 v0.2-D の Secure Extraction 要件
  （Path Policy・canonical 衝突・OS/FS 別保護）、When 証拠
  項目対応表の SEC-01〜06 行を検査する、Then `issue-304`
  R1–R4/R9・#288 matrix A-X1/A-X2/A-X3/A-X4・本書の
  「Secure Extraction」節への参照を持つ行が存在する。
- Given `ArchiveReadReceipt` が `verified: bool` 一値に
  丸められていない設計、When Receipt 節を検査する、Then
  `entries_total`/`entries_verified`/`entries_unverified`/
  `payload_bytes` と `EntryChecksum`/`EntryDisposition` の
  区別・「真正性 ≠ 整合性」が明記されている（R5）。
- Given `ExtractedTree` が Directory Publish や不変
  snapshot と主張されていない設計、When 終了/公開契約節を
  検査する、Then「検証済み Staging Tree の所有権・同権限
  プロセスによる変更を排除しない・`into_path` handoff・
  Drop best-effort・無条件 Atomic Directory Publish 非保証」
  が記述されている（R8）。
- Given consumer-verify の Synvelis lane が `not run`、
  When Consumer 受入を判定する、Then 「両 Consumer 受入
  済み」とは記録されず `not run` のまま残る — 本書は
  Consumer 独立判定と設計/実装分離を明記している（R10/R11）。
- Given 本 PR の diff、When `git status`/`git diff --stat`
  を確認する、Then 変更は `docs/` の `issue-286-*` 新規
  文書と `docs/*/README.md` index のみ（R12）。

## Guarantees

- 横断契約の記述は per-implementation 正本（#302/#303/
  #304 の設計・test-spec）への参照で構成され、詳細の複写
  による drift を避ける構造。
- 所有権/finish・receipt 語彙・budget 両層強制・error
  分類・状態機械の検証は既存 test-spec の規則
  （`issue-302` R6/R9・`issue-303` R6/R7/R10・
  `issue-304` R4/R6/R9 等）に pin されており、本書はその
  索引を提供する。
- Backend 選定の版・MSRV・ライセンスは `Cargo.toml` の
  pin comment と per-implementation 設計の記録に一致し、
  独自の未検証な主張を追加しない。
- 証拠項目対応表の各行は実在する正本への参照のみを持ち、
  存在しない Issue/文書/command/rule 番号を根拠にしない。
- Consumer の schema/version/Backup/Project semantics 所有と
  `ExtractedTree` ≠ Project Transaction の責務分界が #288・
  crate README と同一原則で記述される。

## Non-goals

- per-implementation の実走テスト合否（`issue-302`/`303`/
  `304` test-spec と `mise run test-io-file` が正本）。
- Consumer repo での Golden/byte 互換/性能の実測
  （#219/#294 consumer-verify・AL #139/#143・SV #82/#84/#25
  が Owner）。
- macOS/Linux 実機での extract backend・`check-linux` の
  実機試験・CI lane の継続監視。
- 新 Format・backend 更新・ZIP64/sparse/special/link の
  受理・Directory 原子的 Publish・Consumer Commit/CAS の
  Ruviform 側実装。
- `crates/`・`Cargo.toml`・`mise.toml`・`.github/` の変更。

## Risk

- 本 test-spec が検証するのは横断設計文書の正しさであり、
  IO-04 の Consumer 受入（COMP-01/02/03）は #288 gate と
  consumer-verify の実走段階で別途検証される。設計文書が
  完璧でも Consumer 実測が `not run` なら Issue #286 の
  受入は未完である — この分離を崩すと「設計合意＝受入
  完了」の誤認が起きる。
- 参照先 Issue の状態（OPEN/CLOSED）は変動する。本書は
  2026-11-09 時点の snapshot（SV #82 CLOSED・#84 OPEN・
  #25 CLOSED、AL #139/#143 CLOSED）を記録しており、将来の
  状態変化で接続表の「役割」記述が古くなる可能性がある —
  snapshot 日付の明記で緩和する。
- 証拠項目対応表は per-implementation test-spec の rule
  番号を参照する。per-implementation test-spec が改訂され
  rule 番号が変わった場合、本書の対応表は stale になり得る
  — per-implementation 正本が改訂された場合は本書の対応
  表も追従する必要がある。
