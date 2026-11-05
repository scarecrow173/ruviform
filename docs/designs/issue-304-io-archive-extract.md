---
id: "DESIGN-mH3gJLoQj3rdU7xRXX8eZD"
type: "design"
status: "approved"
title: "Issue #304 ruviform-io-archive — Secure Extraction・Staging/Recovery・OS 別 Root Capability 設計"
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
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-304-io-archive-extract.md"
    - "docs/specs/issue-302-io-archive-zip.md"
    - "docs/specs/issue-303-io-archive-tar.md"
    - "docs/designs/issue-302-io-archive-zip.md"
    - "docs/designs/issue-303-io-archive-tar.md"
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
    - "docs/specs/issue-304-io-archive-extract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-304-io-archive-extract.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #304 ruviform-io-archive — Secure Extraction・Staging/Recovery・OS 別 Root Capability 設計

## Context

Issue #304（IO-IMP-10）は IO-04（#286）v0.2 D/E/F で合意済みの
「隔離 Staging Directory への安全な展開 → 検証済み `ExtractedTree`
→ Consumer 固有検証 → 正式公開（本 Issue 対象外）」の前半を実装
する。#302/#303 で確立した `ArchiveName`（raw bytes・検証なし）・
`EntryKind` + link target・`ArchiveReadLimits`・finish 状態機械を
入力とし、展開先 FS 側の安全性を本 Issue で契約化する。

中核判断は 3 点:

1. **保護は path 文字列ではなく handle/FD 相対 primitive で与える**。
   `..` や絶対 path の文字列検証だけでは、同権限プロセスによる
   展開中の dir 差し替え（TOCTOU）を防げない。IO-IMP-02 と同じく
   raw FFI + private adapter 境界で OS primitive を直接検証する。
   `cap-std` 等の外部 crate には依存しない（IO-IMP-02 と同方針で
   backend 型非露出・依存を最小化）。
2. **削除は「作成したものを identity 照合して消す」**。
   無条件の再帰削除は差し替えられた tree の中身や foreign object
   を巻き込み得る。cleanup は作成時に記録した node identity
   （unix `st_dev`+`st_ino`・windows `FILE_ID_INFO`）と照合しながら
   handle 相対に降下し、不一致は削除せず残留として報告する。
3. **`ExtractedTree` は不変 snapshot ではない**。検証した時点の
   staging tree 所有権のみを表し、同権限の外部変更は可能。
   正式公開は Consumer の別契約（本 Issue 対象外）として明記し、
   `into_path` handoff で所有権を移譲する。

## Scope

### In Scope

- `extract` module（`src/extract.rs` + `extract/{path,fs,unix,windows,session,failure}.rs`）。
- Path Policy: `ArchiveName -> SafeRelativePath`・canonical 衝突
  検出（Unicode NFC + case 畳み込み）・kind policy・書込前 budget。
- Staging Root 排他的作成・`ExtractionSession`/`ExtractedTree`/
  `ExtractionFailure`/`ExtractionRecoveryHandle`/receipt 型。
- unix（dirfd）・windows（NT handle 相対）backend・`Unsupported`
  target 分岐・`#[cfg(test)]` fault-injection mock。
- 否定試験・OS 別保証表・crate doc/README・依存追加
  （`libc`/`windows-sys`/`unicode-normalization`）。

### Out of Scope

- 正式公開・Directory Transaction・Consumer schema・Recovery
  Journal・link/special 受理・非 UTF-8 名・`openat2`/`cap-std`・
  source archive 改変 race の保証・nested archive・IO-IMP-02 との
  共通化（`OsFileOps` は `pub(crate)` かつ File Publish 専用 —
  Directory Transaction と混同しない設計上の境界）。

## Components and Boundaries

### 型一覧（`extract` module の公開面）

```rust
pub mod extract {
    // Path Policy
    pub struct SafeRelativePath { /* components */ }
    impl SafeRelativePath {
        pub fn new(name: &ArchiveName) -> Result<Self, ArchiveError>;
        pub fn components(&self) -> impl Iterator<Item = &str>;
        pub fn depth(&self) -> usize;
    }

    // Session/Tree
    pub struct ExtractionSession { /* staging root + plan + records */ }
    impl ExtractionSession {
        pub fn create(parent: &Path, options: ExtractOptions)
            -> Result<Self, ArchiveError>;
        pub fn staging_path(&self) -> &Path;
        pub fn extract_zip<R: Read + Seek>(self,
            archive: &mut ZipArchiveReader<R>)
            -> Result<ExtractedTree, ExtractionFailure>;
        pub fn extract_tar<R: Read>(self,
            archive: TarArchiveReader<R>)
            -> Result<(R, ExtractedTree), ExtractionFailure>;
        pub fn abort(self) -> Result<CleanupReceipt, ExtractionCleanupFailure>;
    }
    pub struct ExtractedTree { /* root handle + records + receipt */ }
    impl ExtractedTree {
        pub fn path(&self) -> &Path;
        pub fn receipt(&self) -> &ExtractionReceipt;
        pub fn into_path(self) -> PathBuf;        // handoff — 所有権移譲
        pub fn cleanup(self) -> Result<CleanupReceipt, ExtractionCleanupFailure>;
    }
    impl Drop for ExtractionSession / ExtractedTree { /* best-effort cleanup */ }

    // Options/Receipt
    pub struct ExtractOptions { pub limits: ArchiveReadLimits }
    pub struct ExtractionReceipt {
        /* entries, files_written, dirs_written, payload_bytes,
           archive entries_verified/unverified */
    }

    // Failure/Recovery
    pub struct ExtractionFailure {
        /* error: ArchiveError, recovery: Option<ExtractionRecoveryHandle> */
    }
    pub struct ExtractionRecoveryHandle {
        /* staging path + recorded root/node identity + 失敗時診断 */
    }
    impl ExtractionRecoveryHandle {
        pub fn inspect(&self) -> Result<ExtractionRecoveryInfo, ArchiveError>;
        pub fn cleanup(self) -> Result<CleanupReceipt, ExtractionCleanupFailure>;
        // Drop は削除しない
    }
    pub struct ExtractionRecoveryInfo { /* path + state + retained nodes */ }
    #[non_exhaustive]
    pub enum RecoveryTreeState { Present, Absent, Mismatch, Unknown }
    pub struct CleanupReceipt { /* removed/retained node 数・root_removed */ }
    pub struct ExtractionCleanupFailure { /* error + recovery info */ }
}
```

`ArchiveReadLimits` に `max_path_depth: u32`（default 32）を追加。

### Path Policy（`extract/path.rs`）

`ArchiveName`（raw bytes）から `SafeRelativePath`（検証済み UTF-8
component 列）への変換。Secure Default = **全 OS で同じ tree を意味
する名のみ**:

- **必須前提**: 非空・UTF-8・NUL 不含。非 UTF-8 は `InvalidInput`
  （Windows 表現不能・unix 限定受理は portable 契約を破壊）。
- **区切り**: `/` のみ。`\`（Windows separator・unix では合法文字
  で意味が分岐）と `:`（Windows ADS/drive・unix では合法）を含む
  名は拒否 — 同一 archive が OS 間で別の tree に解釈される名を
  「変換」せず拒否する。
- **component 拒否**: `..`・`.`・空・先頭 `/`（絶対）・`//`（UNC
  相当）・末尾 `.`/末尾/先頭空白・制御文字（U+0000-1F, U+007F）。
- **Windows 禁止**: 予約名 `CON`/`PRN`/`AUX`/`NUL`/`COM1-9`/`LPT1-9`
  （大小無視・拡張子付き `con.txt` も含む — stem 比較）・
  `<>|"?*`・`:`（前述）。
- **長さ**: component ≤ 255 byte・`max_path_bytes`・`max_path_depth`
  budget。
- **衝突**: canonical key = `unicase` ではなく `char::to_lowercase`
  全 Unicode 適用 + `unicode-normalization` NFC で構成。同一 key の
  file/file・file/dir・dir 同一の重複を `InvalidInput` で拒否。
  prefix が file として確定済みの entry（`a` file + `a/b`）も衝突。
  **ZIP は plan phase で全 entry を検証してから書込開始**（書込済み
  副作用ゼロで拒否できる）。**TAR は逐次検出**（後続衝突はその
  entry で拒否 → cleanup）。

### FS 境界（`extract/fs.rs` 非公開 trait + OS backend）

IO-01 `PlatformFileOps` と同型の private adapter:

```rust
trait PlatformExtractOps: Send + fmt::Debug {
    fn create_staging_root(&self, parent: &Path)
        -> io::Result<Box<dyn StagingDir>>;
}
trait StagingDir: Send + fmt::Debug {
    fn path(&self) -> &Path;
    fn identity(&self) -> io::Result<NodeIdentity>;      // 自 handle の id
    fn probe_identity(&self, name: &OsStr) -> io::Result<NodeIdentity>; // no-follow 照合
    fn create_dir(&self, name: &OsStr) -> io::Result<Box<dyn StagingDir>>;
    fn open_dir(&self, name: &OsStr) -> io::Result<Box<dyn StagingDir>>;
    fn create_file(&self, name: &OsStr) -> io::Result<Box<dyn StagingFile>>;
    fn remove_file(&self, name: &OsStr) -> io::Result<()>;
    fn remove_dir(&self, name: &OsStr) -> io::Result<()>;
    fn remove_root(self: Box<Self>) -> io::Result<()>;   // root 自身（parent 相対）
}
trait StagingFile: Write + Send + fmt::Debug {
    fn identity(&self) -> io::Result<NodeIdentity>;
}
```

`NodeIdentity { volume: u64, index: u128 }`（unix `st_dev`+`st_ino`・
windows `FILE_ID_INFO` の `VolumeSerialNumber`+`FileId.0`）。

- **unix backend**（`extract/unix.rs`）: parent を
  `open(O_RDONLY|O_DIRECTORY|O_CLOEXEC)`、root は
  `mkdirat(parentfd, unique, 0700)` + `openat(O_RDONLY|O_DIRECTORY|
  O_NOFOLLOW|O_CLOEXEC)`。create_dir=`mkdirat`+`openat`、open_dir=
  `openat`、create_file=`openat(O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW)`
  → `File::from_raw_fd`、identity=`fstat`、probe=`fstatat(
  AT_SYMLINK_NOFOLLOW)`、remove=`unlinkat`/`unlinkat(AT_REMOVEDIR)`。
- **windows backend**（`extract/windows.rs`）: `NtOpenFile`（open）
  `NtCreateFile`（create・`FILE_CREATE` disposition）に
  `OBJECT_ATTRIBUTES{ RootDirectory: parent handle, ObjectName:
  component UNICODE_STRING, Attributes: OBJ_CASE_INSENSITIVE |
  OBJ_DONT_REPARSE }`。dir 系 option=`FILE_DIRECTORY_FILE`、file 系
  =`FILE_NON_DIRECTORY_FILE`、共に `FILE_OPEN_REPARSE_POINT` +
  `FILE_SYNCHRONOUS_IO_NONALERT`。削除=`NtDeleteFile`（同一
  ObjAttr・`OBJ_DONT_REPARSE` で link 自体に作用）。identity=
  `GetFileInformationByHandleEx(FileIdInfo)`。NTSTATUS→`io::Error`
  は `RtlNtStatusToDosErrorNoTeb`。
- **その他 target**: `create_staging_root` が `ErrorKind::Unsupported`
  を返すのみ（弱化して動かさない）。
- **`#[cfg(test)]` mock**: `extract/fs.rs` 内に scripted backend
  （`create_file` N 回目失敗・cleanup 失敗・identity tamper）を置き、
  session/records の fault 経路を単体試験する。

**保護の論理**: `SafeRelativePath` の各 component は path 文字列
検証済みだが、それでも書込/削除は**すべて** dirfd/RootDirectory
相対 + no-follow で行う。中間 component は `open_dir` で 1 段ずつ
降下し、dir 以外・symlink・reparse への差し替えは primitive が
拒否（`ENOTDIR`/`STATUS_*`）する。leaf の create は `O_EXCL`/
`FILE_CREATE` の排他 create で既存名（symlink 含む）を上書きしない。

### Session 状態機械と records

```
ExtractionSession::create(parent, options)
  -> parent 絶対化・一意名 root を排他 create・root handle+identity 記録
  -> Session{ root: Box<dyn StagingDir>, records: Vec<NodeRecord>,
              canonical: HashMap<Vec<String>, NodeKind>, plan 済み? }
extract_zip(self, archive):
  1) plan phase: entry_info 全件 → kind 受理(File/Dir)・SafeRelativePath
     変換・宣言 size 事前拒否・canonical 衝突検出 → Vec<PlannedEntry>
  2) write phase: 順に open_entry → parent 確保（open_dir or create_dir
     逐 component）→ File=create_file+stream finish / Dir=create_dir
     → records に NodeRecord{path, kind, identity} を追加
  3) 全 entry の receipt を集約 → ExtractedTree
extract_tar(self, archive):
  next_entry ごとに plan 相当（validate+衝突）を行い書込 → entry.finish()
  → 末尾で archive.finish()（残 entry drain + 終端検証）→ (R, tree)
失敗時（構造・budget・FS・検証）:
  -> 内部 cleanup(records に従う verified 降下削除) を試行
  -> 成功: ExtractionFailure{ error, recovery: None }
  -> 失敗/不完全: ExtractionFailure{ error, recovery: Some(handle) }
abort(self): verified cleanup -> CleanupReceipt or CleanupFailure
```

`NodeRecord { relpath: SafeRelativePath, kind: Dir|File, identity:
Option<NodeIdentity> }`（`identity=None` は「予約済み未作成」— plan
中に失敗した場合の削除対象外）。

### Cleanup（verified 降下削除）

`cleanup(records)`:
1. root identity を再確認（held handle の `fstat`/`FileIdInfo` が
   recorded root id と一致 → 不一致なら**何も消さず**残留報告）。
2. `records` を depth 降順（leaf から親へ）で処理:
   - 対象 node の parent component 列を `open_dir` で root から降下。
     各 dir の `identity()` を records 側 identity と照合 →
     不一致/open 失敗 → その subtree を削除せず retained 計上。
   - leaf は `probe_identity`（no-follow）で identity 照合 → 一致時
     のみ `remove_file`/`remove_dir`。不一致 → retained。
3. `remove_root`（empty root を parent 相対で削除。non-empty 残存・
   差し替え・失敗は retained 報告）。
`CleanupReceipt { removed_nodes, retained: Vec<SafeRelativePath 相当の
診断 path>, root_removed }`。一部失敗でも処理を継続し、削除できた
分と残留を分けて報告（`ExtractionCleanupFailure` は先頭の失敗
error + recovery info を保持）。

これにより「同権限プロセスが root/中間 dir を symlink・foreign
dir・replica tree に差し替えても、本実装は tree 外の何も消さず、
foreign node を誤削除しない」ことを、dirfd/handle の per-component
保証として構成する（`exists()`-then-delete ではなく primitive）。

### Source 入力の境界（保証外の明記）

- ZIP `open_entry`/`verify_all` は seek 再読込を行う。呼出し側が
  `R` の下層を同時改変する race は**保証外**（reader の構造/CRC 検証
  で検出し得るが、同一 inode の書換えは保証不能）。doc は「input
  archive は固定/信頼済み source（自前 staging・排他 open 等）に
  限定せよ」と明記。
- TAR は one-way streaming — `finish` までに下層が改変されると構造
  検証の意味が変わり得るため同様に保証外。

### エラー分類

- Path/kind/policy 拒否 → `ArchiveErrorKind::InvalidInput`
  （symlink/hardlink/special 等の「存在するが受理しない」kind は
  `Unsupported`）。
- Budget 超過 → `LimitExceeded`。
- FS/backend の capability 不足（非 unix/windows・NtCreateFile
  不可・dirfd 不可）→ `Unsupported`。
- FS I/O 失敗 → `Io`（`ArchiveError::from_io` 経由・source に
  io::Error）。`AlreadyExists`/`ENOTDIR` 等は「衝突/差し替えの
  表面化」として `Io` ではなく `InvalidInput` に正規化して上位へ
  返す箇所を限定する（entry 名の canonical 衝突は policy 側で先に
  `InvalidInput`）。
- `ExtractionFailure`/`ExtractionCleanupFailure` は `ArchiveError`
  + recovery info。Drop は失敗を握りつぶす（診断のみ）。

## Data and Control Flow

```
caller -> ExtractionSession::create(trusted_parent)
         └ fs::create_staging_root(parent) [排他・handle]
caller -> session.extract_zip(&mut reader)
         ├ plan: entry_info -> SafeRelativePath -> canonical map
         ├ write: open_entry -> (open_dir/create_dir)*/create_file
         │        -> io::copy capped -> file -> entry.finish()
         └ Ok -> ExtractedTree{root,records,receipt}
caller -> tree.cleanup() / tree.into_path() / drop(tree)
```

失敗/取消・recovery・「正式公開は次段の独立契約」は v0.2 G/F と
同じ責務境界。

## Non-Functional Constraints

- non-RT・blocking FS I/O・alloc を rustdoc に明記。`Send` の
  compile-time assertion を lib.rs に追加。
- `unsafe` は `#[expect(unsafe_code)]` + SAFETY comment の局所許可。
- `default=[]`・serde 非依存・backend 型非露出（`dyn` trait は
  `pub(crate)`）。`libc`/`windows-sys`/`unicode-normalization` のみ
  追加。
- テストフィクスチャは byte 級 builder（既存 `tests/common`）と実
  FS（`std::env::temp_dir` + 一意サブ dir）。

## Risks and Trade-offs

- **windows `NtCreateFile`/`NtDeleteFile` は非公開 API 相当の FFI**:
  windows-sys が安定表面として提供するため採用。`OBJ_DONT_REPARSE`
  の意味・`FILE_OPEN_REPARSE_POINT` の leaf 挙動・`NtDeleteFile` が
  link 自体に作用することを実測試験（junction/symlink の差し替え
  検証）で pin する。NTSTATUS→Win32 変換で分類不能な status は
  `Io` に落ちる（種別の偽装はしない）。
- **unix `openat`/`O_NOFOLLOW`**: portable で実績十分。`openat2`
  `RESOLVE_BENEATH` の追加 kernel 保証は後続候補（deferred）。
- **identity 照合の残存 race**: probe→delete 間・root remove 直前の
  差し替えは理論上残るが、いずれも tree 外への削除ではなく「tree
  内の foreign object を消す/消さない」の差に限定される。差し替え
  検出は per-component 降下で担保し、残存は「誤検出側（retained）」
  に倒すことで誤削除を回避する方針を doc に明記。
- **TAR の plan phase なし**: streaming の性質上、書込済み entry が
  ある状態での衝突検出になる — 失敗時 cleanup が必須であり、
  `ExtractionFailure` の責務として分離する。
- **`ExtractedTree` の path 露出**: `into_path` handoff で path を
  返す設計は、所有権移譲後の tree を Consumer が責任を持つ契約。
  「不変 snapshot」ではないことを rustdoc・receipt に明記。

## Deferred Design Concerns

- `openat2`/`RESOLVE_BENEATH` 追加・`cap-std` 検討・非 UTF-8 名・
  永続 Recovery Journal・Directory Transaction・link/attribute 復元・
  zip bomb の CPU/heap 精密 model・`ExtractedTree` への readback API。
- IO-IMP-02 `OsFileOps` との dir-relative primitive 共通化（両者が
  成熟し責務差が確定してから）。

## Verification Notes

- `tests/extract_path.rs`: `SafeRelativePath` 否定/受理行列（Public
  API 経由・ArchiveName 構築可能）。
- `tests/extract_secure.rs`: crafted ZIP/TAR（既存 builder）での
  end-to-end 拒否・collision・kind/budget・取消/writer error・
  `tar.gz` 2 層。
- `tests/extract_staging.rs`: 実 FS での staging 排他・identity・
  Drop/cleanup・差し替え（symlink/junction・foreign dir）・recovery
  inspect/cleanup。Windows 分はローカル実測、unix 分は CI。
- `#[cfg(test)]` mock: fault injection（create_file N 回目失敗・
  cleanup 失敗・identity tamper）。
- `mise run fmt`/`check`/`lint`/`test` + Required CI（verify:
  ubuntu+windows）・非必須 verify-macos。
