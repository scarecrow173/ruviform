---
id: "TSPEC-pWc14Y17FV06GN2600rX8h"
type: "test-spec"
status: "approved"
title: "Issue #304 ruviform-io-archive Secure Extraction・Staging/Recovery テスト仕様"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "docs/specs/issue-304-io-archive-extract.md"
    - "https://github.com/scarecrow173/ruviform/issues/304"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-archive/tests/extract_path.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/extract_secure.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/extract_staging.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-archive/tests/common/mod.rs"
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
    - "docs/designs/issue-304-io-archive-extract.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-304-io-archive-extract.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #304 ruviform-io-archive Secure Extraction・Staging/Recovery テスト仕様

## Purpose

`ruviform-io-archive` の `extract` 契約 — `ArchiveName ->
SafeRelativePath` の Path Policy・canonical 衝突検出・kind/budget
拒否・隔離 Staging Root の排他的作成・`ExtractionSession ->
ExtractedTree`・identity 照合付き Cleanup・Recovery Handle — を
byte 級 fixture（既存 ZIP/TAR builder）と実 FS 試験で pin する。
Issue #304 Acceptance（Zip Slip・drive/UNC・windows reserved/ADS・
unicode/casefold・file/dir 重複・Symlink/Hlink・reparse・special・
Entry/展開 byte/Metadata/Path 深さ budget・取消/Writer Error・
Staging Tree/RecoveryHandle の Identity・Drop/Cleanup 失敗/差し替え/
誤削除防止）が正本。

## Feature

Untrusted Archive を Staging Root に展開する際、Path Policy・衝突・
kind/budget の拒否規則が設計どおり `ArchiveErrorKind` に分類され、
書込も削除も Staging Root の外側へ出ないこと。失敗/取消時は
`ExtractionFailure` + `ExtractionRecoveryHandle` が発行され、
Cleanup は作成時 identity と照合した降下削除のみを行う。

## Rules

- R1（Path Policy 否定）: `SafeRelativePath::new` が `..`・`.`・
  空/絶対 component・`\`・`:`・drive（`C:`）・UNC（`//`・`\\`）・
  予約名（`CON`/`NUL`/`AUX`/`PRN`/`COM1`/`LPT9`・大小・拡張子付き）・
  末尾 `.`/末尾・先頭空白・NTFS 禁止文字（`<>|"?*`）・制御文字・
  NUL・非 UTF-8・`max_path_bytes`/`max_path_depth` 超過を拒否
  （`InvalidInput`/`LimitExceeded`）。`a/b/c.txt`・`dir/`・深い
  合法名は受理。
- R2（canonical 衝突）: `Foo.txt`/`foo.txt`・`é.txt`(NFC)/
  `e\u{301}.txt`(NFD)・同一 entry 重複・`a`(file) と `a/b`・
  `a/` と `a`(file) が同一 canonical key に畳まれ、書込前に
  `InvalidInput` で拒否される。dir 同士の同名（明示 entry と
  implicit 作成の一致）は受理。
- R3（kind policy）: `EntryKind::SymbolicLink`/`HardLink`/`Special`/
  `Unsupported` → `Unsupported` で拒否。File/Directory のみ受理。
- R4（Budget）: `max_entries`・`max_entry_uncompressed_bytes`・
  `max_total_uncompressed_bytes`・`max_path_bytes`・`max_path_depth`・
  `max_metadata_bytes` 超過（zip bomb 含む）が宣言 size の事前拒否
  と実測の途中遮断の両層で `LimitExceeded` に分類され、超過 byte が
  FS に出ない。TAR `S` sparse は reader 層 `Unsupported`（extract は
  到達しない）。
- R5（Staging 排他）: `ExtractionSession::create` が信頼済み
  parent 内に一意名 root を排他 create（`AlreadyExists` 衝突時は
  retry/失敗）し、root handle + identity を記録する。非対応 target・
  parent 非 dir → `Unsupported`/`Io`。
- R6（extract 状態機械）: `extract_zip`/`extract_tar` が plan/
  collision 済み entry を dirfd/handle 相対 + CreateNew + no-follow
  で書き込み、`entry.finish()`（ReadToEnd/Discarded）と archive
  finish を確認して `ExtractedTree` を返す。`abort` は verified
  cleanup で `CleanupReceipt`。
- R7（失敗・取消）: 構造・budget・FS・writer error で
  `ExtractionFailure`（`ArchiveError` + `Option<RecoveryHandle>`）。
  cleanup 成功なら `recovery: None`、失敗/不完全なら
  `Some(handle)`（`inspect`/`cleanup`）。`ExtractionRecoveryHandle`
  の Drop は削除しない。
- R8（identity cleanup）: cleanup は root/中間 dir/leaf の
  identity を records と照合して降下。差し替え（symlink・junction・
  foreign dir・inode 再利用）は削除せず retained 報告。
  `remove_root` は empty のみ。`CleanupReceipt`/`CleanupFailure` が
  実績/残留を分離報告。Drop の best-effort cleanup は失敗を
  握りつぶす。
- R9（OS 別 primitive）: unix は `openat`/`O_NOFOLLOW`/`O_DIRECTORY`
  `O_CREAT|O_EXCL`、windows は `NtCreateFile`/`NtOpenFile`/
  `NtDeleteFile` + `RootDirectory`/`OBJ_DONT_REPARSE`/
  `FILE_OPEN_REPARSE_POINT`/`FILE_(NON_)DIRECTORY_FILE`。他
  target は `Unsupported`。`exists()`-then-act は使わない。
- R10（非不変・handoff）: `ExtractedTree` は不変 snapshot/
  Directory Publish ではない。`into_path` は所有権移譲（cleanup
  しない）。Source archive の同時改変 race は保証外（doc 明記）。
- R11（`tar.gz` 2 層）: TAR finish（decode 済み stream 内部）と
  compression 層 finish（trailer/CRC）は独立 — TAR 層の成功が
  圧縮層を検証済みと主張しない（#303 で pin 済み、extract でも
  `TarArchiveReader` by value → inner 返却で維持）。

## Examples

- `EntrySpec` name `../evil.txt`・`/abs.txt`・`C:/win.txt`・
  `a\\b.txt`・`con.txt`・`a:b.txt`・`a/./b`・`a//b` を含む ZIP →
  extract が `InvalidInput`（`SafeRelativePath` 拒否）で失敗し、
  Staging Root は cleanup される。
- `Foo.txt` + `foo.txt` の ZIP → plan phase で canonical 衝突を
  `InvalidInput` で拒否。`a`(file) + `a/b.txt` → file/dir 衝突拒否。
- TAR `1`/`2`/`3`/`S` typeflag entry → `Unsupported`。
- `max_entries=2` の ZIP 3 entry → `LimitExceeded`。
  `declared_uncompressed` 巨大 deflate entry → 事前 `LimitExceeded`。
  deflate で実測が `max_entry_uncompressed_bytes` 超過 → 途中
  `LimitExceeded` で file に過剰 byte なし。
- TAR `a`(file) + `a/b.txt`(file) → canonical 衝突で 2 番目の
  entry で拒否 → 部分書込は cleanup で消去。
- `abort()` → staging 配下が消え、`CleanupReceipt.removed_nodes`
  が実績と一致。
- ExtractedTree drop → staging 配下が消える。staging root を foreign
  dir（junction/symlink）に差し替えて cleanup → 誤削除せず retained
  報告。`RecoveryHandle::inspect` が `Mismatch`/`Present`/`Absent` を
  返す。
- mock: `create_file` N 回目失敗 → `ExtractionFailure` + cleanup で
  書込済み entry が消去される。cleanup 失敗注入 → `recovery:
  Some(handle)` + retained 報告。

## Guarantees

- `SafeRelativePath` は「全 OS で同じ tree を意味する UTF-8 相対
  component 列」のみを受理し、`\`・`:`・drive/UNC・予約名・
  traversal を「変換」せず拒否する。
- 書込/削除はすべて dirfd/handle 相対 + no-follow + CreateNew の
  primitive で行い、文字列検証だけで安全性を主張しない。
- Cleanup は作成時 identity と照合した node のみを削除し、
  tree 外・foreign object・差し替え node を消さない。
- `finish`/`extract_*` の成功のみが完了証拠。Drop は best-effort
  cleanup であり、完了・不変・公開の証拠ではない。
- 保証不能（非 unix/windows・dirfd/handle 相対不可）な環境・性質
  （Source 改変 race・`ExtractedTree` の不変性・正式公開）は
  `Unsupported`/文書化で明示し、弱化して動かさない。

## Non-goals

- 正式公開（Directory Publish・Consumer Commit・CAS）・Recovery
  Journal・link/special 受理・非 UTF-8 名・nested archive・
  source 改変 race の保証。
- IO-IMP-02 `OsFileOps` との共通化・`cap-std`・`openat2`。
- Sparse（TAR `S`）展開（reader 層 `Unsupported` のまま）。
- CPU/heap/inode の全コストの byte budget 超の厳密保証。

## Risk

- `NtCreateFile`/`NtDeleteFile`/`OBJ_DONT_REPARSE`/`FILE_OPEN_REPARSE_POINT`
  の実挙動を Windows 実測で pin（junction・symlink・差し替え）。
  不確実な leaf 削除は `open_dir`+identity 照合へ置き換える。
- 実 FS 試験は `std::env::temp_dir` + 一意子 dir。CI（ubuntu+
  windows）とローカル（Windows）で backend を網羅し、macOS 分は
  非必須 lane/未実測として報告。
