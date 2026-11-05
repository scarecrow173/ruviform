---
id: "SPEC-6UnMiBIsE4whNsJmSW8SQX"
type: "spec"
status: "approved"
title: "Issue #304 ruviform-io-archive — Secure Extraction・Root Capability・Path Policy・Staging Recovery (IO-IMP-10)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/304"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract/path.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract/fs.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract/unix.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract/windows.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract/session.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract/failure.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/extract_path.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/extract_secure.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/extract_staging.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-archive/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/limits.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-archive/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "Cargo.lock"
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
    - "docs/specs/issue-303-io-archive-tar.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/specs/issue-296-io-file-os-backend.md"
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

# Issue #304 ruviform-io-archive — Secure Extraction・Root Capability・Path Policy・Staging Recovery (IO-IMP-10)

## What Should Be Built

既存 `crates/ruviform-io-archive` に `extract` module を追加する。
新規クレートは作らない。IO-04（Issue #286）v0.1/v0.2 で合意済みの
「Untrusted Archive を既存の正式出力先へ直接展開しない」「隔離
Staging Root の排他的作成・Handle 相対/No-follow/CreateNew・
`ExtractionSession -> ExtractedTree`・Cleanup/Recovery Handle・
保証不能環境は Unsupported」をコンパイル可能な契約にする。

公開面（`extract` module）:

- `SafeRelativePath` — `ArchiveName` とは別型の、対象 OS/FS で安全な
  相対 path として検証済みの entry 展開先。`ArchiveName -> SafeRelativePath`
  変換が唯一の検証入口（Path Policy）。Secure Default は**全 OS で
  同じ tree を意味する名のみ**を受理する:
  - `/` 区切りのみ（`\`・`:` は拒否）、`..`/`.`/空 component 拒否、
    絶対 path・drive（`C:` 等）・UNC（`//`）拒否。
  - Windows 互換性を壊さない名のみ受理: 予約名
    （`CON`/`PRN`/`AUX`/`NUL`/`COM1-9`/`LPT1-9`・拡張子付き含む・
    大小無視）、末尾 `.`/空白 component、`<>|"?*` 等の NTFS 禁止
    文字、制御文字、NUL。
  - UTF-8 必須（raw bytes を損なわない `ArchiveName` とは対称に、
    FS 展開可能名は UTF-8 に限定。非 UTF-8 は `InvalidInput` で
    拒否 — Windows では表現不能、unix 限定受理は portable 契約を
    壊すため）。
- Path collision 検出 — Unicode normalization + case 畳み込みキーで
  `Foo.txt`/`foo.txt`・NFC/NFD 差・file/dir 衝突（`a` file と
  `a/b`）・同一 entry の重複を**書込前**に拒否する。ZIP は plan
  phase（全 entry を検証してから書込開始）、TAR は streaming 中の
  逐次検出（書込済み entry があっても後続衝突は拒否→cleanup）。
- `ExtractOptions` — `ArchiveReadLimits` を保持。Extraction 追加の
  `max_path_depth` budget を `ArchiveReadLimits` に追加する（既存
  field は reader と共用。sparse 論理 size は TAR `S` が reader 層で
  `Unsupported` のため到達しない — budget は「宣言値の事前拒否 +
  実測の途中遮断」で代替されることを doc に明記）。
- `ExtractionSession` — 信頼済み parent（`AsRef<Path>`、絶対化して
  保持）内に一意名の専用 Staging Root を**排他的**に作成し所有する
  session。`extract_zip`（`&mut ZipArchiveReader`）/ `extract_tar`
  （`TarArchiveReader` by value → inner 返却）が検証済み tree を
  返す。`abort` で明示取消。Drop は best-effort cleanup。
- `ExtractedTree` — Archive として検証した Staging Tree の所有権。
  `path`（診断・handoff 用）・`receipt`・`into_path`（handoff、
  所有権移譲で cleanup しない）・`cleanup`・Drop best-effort。
  **不変 snapshot・署名済み tree・完了した Directory Publish では
  ない**（同権限の外部変更は可能 — rustdoc に明記）。
- `ExtractionFailure` — 失敗診断（`ArchiveError`）と
  `Option<ExtractionRecoveryHandle>`。extract 失敗時は内部 cleanup
  を試行し、失敗/不完全なら Recovery Handle を同梱する。
- `ExtractionRecoveryHandle` — Staging Tree の識別（recorded root
  identity）を保持。`inspect`（Present/Absent/Mismatch/Unknown の
  `RecoveryTreeState`）と `cleanup(self)`。Drop は**削除しない**
  （不確定な tree を無条件に再帰削除しない契約）。
- `CleanupReceipt` / `ExtractionCleanupFailure` — 削除実績と残留・
  失敗分類。

非公開の FS 境界（`extract/fs.rs` 等）は IO-01 `PlatformFileOps` と
同型の private adapter 契約 — **unix**: `openat`/`mkdirat`/`unlinkat`
の dirfd 相対 + `O_NOFOLLOW`/`O_DIRECTORY`/`O_CREAT|O_EXCL`。
**windows**: `NtCreateFile`/`NtOpenFile`/`NtDeleteFile` の
`RootDirectory` 相対 + `OBJ_DONT_REPARSE` + `FILE_OPEN_REPARSE_POINT`
+ `FILE_(NON_)DIRECTORY_FILE`。その他 target は `Unsupported`。
Handle 相対の逐 component no-follow 降下で「書込も削除も Staging
Root の外側へ出ない」ことを primitive として保証する。IO-IMP-02 の
`PlatformFileOps`/`OsFileOps` は `pub(crate)` で File Publish 専用の
ため共通化しない（File Publish と Directory Transaction の混同は
設計上禁止）。

## Why Now

Issue #302/#303 で ZIP/TAR の Reader 契約（`ArchiveName` raw bytes・
`EntryKind` + link target・budget・finish 状態機械）が確定し、#304
の Extraction 層が実装可能になった。外部由来 archive の安全な展開は
Consumer（AudioLens Dataset・Synvelis Project Import）の前提要件で
あり、path 正規化だけでは防げない TOCTOU・reparse/symlink 攻撃に
対して OS 別の実 primitive を持つ Root Capability が IO-04 設計の
核心として残っている。

## Users and Value

- 後続 Consumer: Synvelis Project/Revision・AudioLens Job/Dataset が
  Untrusted archive を「正式出力先を汚さない隔離展開 → 検証済み
  tree」として扱える。正式 Commit/CAS/Directory Publish は
  Consumer 責務のまま。
- Ruviform 基盤: Zip Slip・drive/UNC・reparse・Unicode/casefold 衝突・
  expansion bomb への拒否規則が OS 別の保証レベルとともに契約化
  される。

## Scope

### In Scope

- `src/extract.rs` + `src/extract/{path,fs,unix,windows,session,failure}.rs`
  の追加（`pub mod extract`）。
- `ArchiveReadLimits` への `max_path_depth` field 追加（`#[non_exhaustive]`
  struct の追加 field — 既存 Consumer は `default()` + mutate 経路の
  ため互換）。
- Path Policy（`ArchiveName -> SafeRelativePath`）・canonical 衝突
  検出・kind policy（File/Directory のみ、Symlink/HardLink/Special/
  Unsupported は拒否）・書込前 budget 拒否（宣言 size・entry 数・
  path byte/depth・累積 total）。
- 隔離 Staging Root の排他的作成（parent 絶対化・一意名・排他
  create・root handle 保持・root identity 記録）。
- `ExtractionSession`/`ExtractedTree`/`ExtractionReceipt`/
  `ExtractionFailure`/`ExtractionRecoveryHandle`/
  `ExtractionRecoveryInfo`（`RecoveryTreeState`）/`CleanupReceipt`/
  `ExtractionCleanupFailure`。
- OS 別 backend: unix（`openat`/`mkdirat`/`unlinkat`/`fstat` +
  `O_NOFOLLOW`/`O_DIRECTORY`/`O_CREAT|O_EXCL`、libc 依存追加）・
  windows（`NtCreateFile`/`NtOpenFile`/`NtDeleteFile`/
  `NtQueryInformationFile`/`GetFileInformationByHandleEx(FileIdInfo)`
  + `RootDirectory`/`OBJ_DONT_REPARSE`/`FILE_OPEN_REPARSE_POINT`、
  windows-sys 依存追加）。その他 target → `Unsupported`。
- node identity（unix: `fstat` dev+ino・windows: `FileIdInfo`
  volume+file index）の記録と、cleanup 時の**作成時 identity との
  照合付き降下削除**（差し替え検出時はその subtree を削除せず
  残留として報告）。
- 否定試験: Zip Slip・drive/UNC・windows reserved/ADS・unicode/
  casefold・file/dir 重複・Symlink/Hlink・reparse・special file・
  各 budget 超過・取消・writer error・Staging/Recovery の identity/
  誤削除防止。
- crate doc/README・`crates/README.md` 台帳更新・`unicode-normalization`
  依存追加（canonical 衝突キーの NFC 畳み込み用。理由を PR に記録）。

### Out of Scope

- 正式公開（Directory Publish/rename・Consumer Project Commit・CAS・
  Directory Transaction）— `ExtractedTree` は verified staging の
  所有権のみ。IO-01 `StagedFile` の単一 file publish を Directory/
  multi-file transaction に拡大解釈しない。
- Symlink/HardLink/Special/属性・権限復元（手動許可は独立した権限
  レビューが必要 — 本 Issue は一律拒否）。
- Source Archive 入力自体の同時改変 race の保証（ZIP の seek 再
  読込は改変 bytes を観測し得る — 保証外とし、固定入力/信頼済み
  領域へのコピーを Consumer 契約として rustdoc に明記）。
- 永続 Recovery Journal（process crash 後の handle 復元は Consumer
  責務）。`tar.gz` 圧縮層の終端検証（IO-03 の契約として分離 — TAR
  `finish` との独立性は #303 で pin 済み）。
- Sparse（TAR `S`）の展開（reader 層で `Unsupported` のまま）。
- Nested archive の再帰展開・ファイル数/Inode/CPU/Heap 全コストの
  厳密保証（byte budget 範囲のみ）。
- `cap-std` 等の外部 crate への依存（IO-IMP-02 と同様に raw FFI で
  primitive を直接検証する — backend 型は非公開、unsafe は SAFETY
  comment + 局所 `#[expect]` のみ）。
- Linux/macOS 実機での人手検証（CI lane + Windows 実測で代替し、
  未実測分は正直に報告）。

## Requirements

- `default = []` 維持。追加依存は `libc`（unix）・`windows-sys`
  （windows・`Win32_Foundation`/`Win32_Storage_FileSystem`/
  `Win32_System_IO`/`Wdk_Foundation`/`Wdk_Storage_FileSystem`
  feature）・`unicode-normalization`（canonical 衝突キーの NFC）に
  限定し、IO-IMP-02 と同じ raw FFI + private adapter 方針を継承。
- MSRV 1.89・edition 2024・`workspace.lints` 継承・`unsafe` は
  `#[expect(unsafe_code)]` + SAFETY comment の局所許可のみ。
- 公開 enum（`EntryKind` 既存・`RecoveryTreeState` 等の新規）は
  `#[non_exhaustive]`。
- 全 API に non-RT 契約を明記（blocking FS I/O + alloc）。
- 「`exists()`/`metadata` の事前チェック + path 操作」で安全性を
  主張しない — 保護は dirfd/handle 相対 + no-follow + CreateNew の
  primitive で与え、できない性質は `Unsupported`/文書化。
- `finish` 系の成功のみを完了証拠とし、Drop を完了として扱わない。
- `ExtractedTree` を不変 snapshot と呼ばない。

## Acceptance Criteria

- [ ] AC1（Path Policy 否定行列）: `..`・絶対 path・drive/UNC・
      `\`・`:`（ADS）・予約名・末尾 `.`/空白・NTFS 禁止文字・
      空/`.` component・非 UTF-8・NUL が `SafeRelativePath` 変換で
      拒否されることを `tests/extract_path.rs` で pin する。
- [ ] AC2（衝突検出）: casefold 重複・NFC/NFD 差・file/dir 衝突・
      同名重複・同一 canonical の dir 同名が、書込前（ZIP plan
      phase）または該当 entry の書込前（TAR）に拒否されることを
      pin する。
- [ ] AC3（Kind/Budget 拒否）: Symlink/HardLink/Special/Unknown
      kind、entry 数・entry/total 展開 byte・path byte/depth・
      metadata byte 超過（zip bomb 含む）が書込前/途中に
      `Unsupported`/`LimitExceeded`/`InvalidInput` で拒否され、
      超過 byte が FS に出ないことを pin する。
- [ ] AC4（Staging/Recovery identity）: Staging Root の排他的作成・
      root/entry identity 記録・`ExtractedTree`/`ExtractionSession`
      の Drop best-effort cleanup・`abort`・`ExtractionFailure`
      の `Option<RecoveryHandle>`・`inspect` 状態分類・`cleanup` の
      identity 照合・差し替え（symlink/junction/foreign dir）時の
      誤削除拒否を実 FS 試験（`tests/extract_staging.rs`）で pin
      する。Windows 分はローカル実測、unix 分は CI を証拠とする。
- [ ] AC5（Round-trip・退行なし）: 正常系 ZIP/TAR の展開が
      `ExtractedTree` + receipt を返し、既存 ZIP/TAR テストが
      退行しない。`mise run fmt`/`check`/`lint`/`test` が緑で、
      MSRV 1.89・`unsafe` 局所許可のみ・`default=[]`・依存 DAG
      （io-file への新規依存なし）を確認する。
- [ ] AC6（脅威境界の文書化）: OS 別の保護レベル（dirfd/handle
      相対・no-follow・CreateNew）・保証外の性質（Source 改変
      race・ExtractedTree の非不変性・保証不能 target）を crate
      doc/README に明記する。

## Deferred Design Concerns

- IO-IMP-02 の `OsFileOps` との共通基盤化（dir-relative ops の
  共通化）| 両者が成熟し File Publish / Directory Transaction の
  契約差が確定してから | 現時点では `pub(crate)` で File Publish
  専用であり、混同は設計上禁止されている。
- 非 UTF-8 entry 名の unix 限定受理 | portable 契約の確定後 |
  Windows 表現不能のため一律拒否を初期契約とする。
- Recovery Journal 永続化・`ExtractedTree` の CAS/署名 handoff |
  Consumer の Commit 契約確定後 | 本 Issue は process 内の
  identity/owner 契約のみ。
- Linux `openat2`（`RESOLVE_BENEATH` 等のより強い kernel 保証）への
  移行 | 動作 kernel 範囲の確定後 | `openat` + `O_NOFOLLOW` の逐
  component 降下は portable で、openat2 は補強候補。
