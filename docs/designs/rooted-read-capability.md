---
id: "DESIGN-6GZ1DYjAInxUNz0dwJRwN7"
type: "design"
status: "approved"
title: "ruviform-io-file Rooted Read Capability 設計"
created: "2026-11-07"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/304"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-file/src/rooted.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/rooted/resolve.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/vfs.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/vfs/unix.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/vfs/windows.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/vfs/unsupported.rs"
      - type: "file"
        path: "crates/ruviform-io-file/tests/rooted_read.rs"
      - type: "file"
        path: "crates/ruviform-io-file/tests/common/mod.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-file/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract/fs.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-archive/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "docs/designs/issue-304-io-archive-extract.md"
    deleted:
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract/unix.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract/windows.rs"
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/rooted-read-capability.md"
    - "../adr/rooted-read-capability-placement.md"
    - "issue-304-io-archive-extract.md"
    - "issue-296-io-file-os-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-304-io-archive-extract.md"
    - "../plans/issue-304-io-archive-extract.md"
    - "ruviform-architecture-design-baseline.md"
    - "../designs/issue-468-rooteddir-canonical-premise.md"
    - "../specs/issue-468-rooteddir-canonical-premise.md"
    - "../adr/issue-468-rooteddir-canonical-premise.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/rooted-read-capability.md"
    - "../adr/rooted-read-capability-placement.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/rooted-read-capability.md"
  references: []
  defers: []
  deferred-by: []
review:
  cycle: 2
  verdict: "ACCEPT — cycle-1 F1-F9 全件 FIXED; cycle-2 R-1〜R-12 を本 revision に反映"
  cycle-2-fixed:
    - "R-1 ancestor chain を Arc 共有化（dup 不要、scoped clone で fd 増殖なし）"
    - "R-2 VfsDir::stat_self 追加（metadata(\"\")/link→root の NodeInfo）"
    - "R-3 Windows per-op access/share/options 表完成（delete に OBJ_DONT_REPARSE）"
    - "R-4 RootedDir::open は open-first + handle 由来 canonical path"
    - "R-5 Windows \\foo / C:foo target → EscapesRoot"
    - "R-6 空 link target → NotFound"
    - "R-7 R=='/'・target==R の pre-loop 受理"
    - "R-8 probe.strength を per-object authoritative gate に確定"
    - "R-9 arch 一覧の VfsHandle/OpenFlags 除去"
    - "R-10 tainted は sticky（決定的 EscapesRoot、早期 error を抑制）"
    - "R-11 Linux O_PATH fstat=probe 同一 fd 化・open_regular は kind 検査のみ"
    - "R-12 remove_file の EISDIR 保証は unix-scoped と明記"
  blocking-fixed:
    - "F1 absolute-link `..` kernel-faithful 化 (two-state TEXTUAL/FD walk)"
    - "F2 identity strength floor — 弱い FS は pin/open_pinned Unsupported"
    - "F3 scoped RootedDir が ancestor fd chain を保持"
    - "F4 Windows RS1 version gate + per-op flags/share 明示"
    - "F5 VfsHandle 宣言 + file_identity op"
    - "F6 fs.rs→vfs mapping 表 + remove_file/remove_dir 分離 + create 意味固定"
    - "F7 io→FileErrorKind 完全写像表 + NoFollow per-API 規則"
    - "F8 Windows 絶対 link 正規化/reparse tag 分類/case 比較"
    - "F9 root enumeration — 空 RelativePath"
---

# ruviform-io-file Rooted Read Capability 設計

## 概要

Synvelis SV85/PR180（canonicalize+containment 後の path 再解決による
root 外 bytes/facts 露出）を解消するため、`ruviform-io-file` に
read-only の rooted capability `rooted` module を追加する。
`RootedDir` が root directory handle を保持し、component 解決・
open・metadata・enumeration をすべてその handle 相対で行う。
OS ルールは `#[doc(hidden)]` の workspace 内部共有層 `vfs` に
集約し、`ruviform-io-archive` の extract backend も同じ層を
consume する（ADR: `docs/adr/rooted-read-capability-placement.md`）。

## 背景

- SV `LocalFolderProvider`/`LocalBlobDir`（PR180 head `d27bbee`）は
  locator を `canonicalize`+`starts_with(root)` で検査したあと
  `File::open`/`fs::metadata`/`fs::read_dir` で path を再解決する。
  leaf / 中間 component の symlink・reparse・rename 差し替えで
  root 外 object の bytes/facts が返り得る。EOF digest は返却済み
  byte を取り消せず、pin の digest は外部内容の hash oracle に
  なり、post-open `is_file` は open 時点の FIFO/device 待ち・
  副作用を防がない。
- IO-IMP-10（Issue #304）は同一問題を **write 側** で解き、
  `extract/{fs,unix,windows}.rs` に fd/handle 相対・no-follow・
  `NodeIdentity` の私有 primitive を実装済み。同設計は dir-
  relative primitive の共通化を Deferred Design Concern として
  記録しており、本設計がその読取側の実現である。

## 設計目標

1. **同一 handle 保証**: `open_regular` が返す reader と
   metadata/length は同一 opened object に由来する。`open_pinned`
   は identity 照合で差し替え object の bytes を返さない。
2. **root 外の byte/facts を出さない**: すべての解決が root dir
   handle 相対で、path 文字列の再解決を行わない（唯一の trust
   boundary は `RootedDir::open` の root canonicalize のみ）。
   保証は root inode が属する FS tree の kernel namespace 意味論に
   従う — unix の mount/bind mount 横断は kernel の実体として
   そのまま通過する（SV canonicalize も同一の穴。scope する
   文脈は「意味論・限界」表参照）。
3. **nonregular を通常 open へ流さない**: regular file 以外は
   blocking/side-effect あり得る open を行わず typed 失敗。
4. **正直な Unsupported**: 保証不能な OS/FS では typed
   `Unsupported` を返し、path-based fallback に縮退しない。
   identity の弱い FS では `pin`/`open_pinned` が `Unsupported`。
5. **現 SV 意味論の保持**: `WithinRoot` は現 SV の
   canonicalize+containment 契約（root 内 link は辿る・`..` は
   kernel 規則で解決）を、capability の構造的制約が許す範囲で
   忠実に再現する。`NoFollow` は別の厳格 policy として明示的に
   選ぶ。

## 非目標

spec の Out of Scope に同じ。write op の公開・archive 公開契約
変更・`openat2`/`RESOLVE_*`・cap-std 系 framework・公開 mock・
SV 側修復・万能 filesystem 防御・非 UTF-8 locator 正規化は
行わない。

## アーキテクチャ

```text
ruviform-io-file
├── rooted            (public, read-only capability)
│   ├── RootedDir     root handle + canonical root + options
│   │                 + ancestor fd chain (scoped dir)
│   ├── RelativePath  検証済み相対 locator (空 = self)
│   ├── FollowMode    NoFollow | WithinRoot
│   ├── resolve.rs    two-state TEXTUAL/FD component walk
│   └── NodeInfo/NodeKind/NodeId/NodePin/DirEntry/RootedFile
└── vfs               (#[doc(hidden)] pub — workspace 内部共有)
    ├── trait VfsOps / VfsDir
    ├── struct VfsProbe / VfsEntry / NodeIdentity / VfsKind
    ├── unix.rs       openat 系 + O_NOFOLLOW (+ Linux O_PATH 2段)
    ├── windows.rs    NtCreateFile/NtOpenFile RootDirectory 相対
    └── unsupported.rs  全 op が Unsupported

ruviform-io-archive
└── extract/fs.rs     PlatformExtractOps/StagingDir/StagingFile 等の
                      session 向け trait 形は維持し、実装を vfs 呼び出しの
                      薄い adapter にする。extract/{unix,windows}.rs は
                      削除し OS ルールを vfs へ集約
```

- `vfs` は `#[doc(hidden)]` で `publish = false` 前提の内部共有面。
  semver 対象外・`ruviform-io-archive` staging backend 専用を doc で
  明記する。到達可能な `pub` enum には `#[non_exhaustive]` を付ける
  （doc-hidden でも `clippy::exhaustive_enums` のゲート対象）。
- `rooted` は `vfs` を消費する唯一の公開面であり、`vfs` の型は
  公開 API に露出しない。
- 依存方向は `ruviform-io-archive → ruviform-io-file`（下向き）。
  io-archive の `libc`/`windows-sys` 直接依存は不要になる。

## 公開 API

```rust
// crates/ruviform-io-file/src/rooted.rs

pub struct RootedDir { /* ops: Arc<dyn VfsOps>,
                          chain: Vec<Arc<dyn VfsDir>>, // root→self の
                          //   resolved ancestor chain (chain[0]==root、
                          //   chain.last()==この dir。Arc 共有なので
                          //   scoped clone は fd を増やさない)
                          root: Arc<RootShared{canonical, options}> */ }

impl RootedDir {
    /// root dir handle を保持する capability を返す。**open を先に
    /// 行い、canonical path は可能なら handle 由来で取得する**
    /// （Linux `/proc/self/fd/N` readlink・Windows
    /// `GetFinalPathNameByHandleW`・unix `F_GETPATH`）— (path→object,
    /// canonical-string) の一致を atomic に近く保つ。handle 由来の
    /// canonical 取得ができない backend では **open 後に
    /// `fs::canonicalize` で anchor を得る**（順序は open →
    /// canonicalize — この residual race が唯一の trust boundary
    /// として明記される）。非 dir は `NotDirectory`。
    pub fn open(path: &Path) -> Result<Self, FileError>;
    pub fn open_with(path: &Path, options: RootedOptions) -> Result<Self, FileError>;
    /// capability root の canonical path（scoped dir でも root を指す）。
    /// 絶対 link の prefix 判定に使う文字列基準。pin した inode が
    /// 保証であり、path 文字列は link 判定のポリシーにのみ使う。
    pub fn canonical_root(&self) -> &Path;

    pub fn metadata(&self, rel: &RelativePath, mode: FollowMode)
        -> Result<NodeInfo, FileError>;
    pub fn open_regular(&self, rel: &RelativePath, mode: FollowMode)
        -> Result<RootedFile, FileError>;
    /// leaf regular file の identity token。非 regular は `NotRegularFile`。
    /// identity が弱い FS（下記「identity の強さ」）では `Unsupported`。
    pub fn pin(&self, rel: &RelativePath, mode: FollowMode)
        -> Result<NodePin, FileError>;
    /// resolve→open 後に identity を `pin` と照合。不一致は
    /// `IdentityMismatch`（bytes を返さない）。
    pub fn open_pinned(&self, rel: &RelativePath, pin: &NodePin, mode: FollowMode)
        -> Result<RootedFile, FileError>;
    /// `rel` の指す dir の entry 列（`.`/`..` 除外）。`rel=""` で
    /// この dir 自身を列挙（root 列挙は `root.read_dir("")`）。
    /// 各 entry の kind は常に leaf no-follow 判定。
    pub fn read_dir(&self, rel: &RelativePath, mode: FollowMode)
        -> Result<Vec<DirEntry>, FileError>;
    /// `rel` の指す dir に降りた scoped capability。`rel=""` は自身
    /// （ancestor chain は Arc 共有で fd を増やさない）。resolved
    /// ancestor chain を保持し、
    /// link target 中の `..` は chain 内を pop する（capability root
    /// を下回ると `EscapesRoot`）。
    pub fn open_dir(&self, rel: &RelativePath, mode: FollowMode)
        -> Result<RootedDir, FileError>;
    /// browse loop guard 用のこの dir の identity。
    pub fn dir_identity(&self) -> Result<NodeId, FileError>;
}

/// `Read + Seek` を実装する regular file reader + open 時点の同一
/// object facts。
pub struct RootedFile { /* file: File, info: NodeInfo */ }
impl Read for RootedFile { /* delegate */ }
impl Seek for RootedFile { /* delegate */ }
impl RootedFile {
    pub fn info(&self) -> &NodeInfo;      // open 時点の同一 handle facts
    pub fn into_file(self) -> File;       // std File を取り出す
}

#[non_exhaustive] pub enum FollowMode { NoFollow, WithinRoot }
#[non_exhaustive] pub enum NodeKind   { File, Dir, Symlink, Other }

pub struct RelativePath { /* components: Vec<String>（0個 = self） */ }
impl RelativePath {
    /// `/` 区切り locator。空文字列は 0 成分（self）として受理。
    /// `..`/`.`/絶対/`\`/`:`/NUL/制御文字/Windows 予約名・末尾
    /// `.`/空白・component>255B は `InvalidInput`。
    pub fn new(locator: &str) -> Result<Self, FileError>;
    pub fn from_component(name: &OsStr) -> Result<Self, FileError>; // 1 成分
    pub fn components(&self) -> impl Iterator<Item = &str>;
}

/// `Default` 実装あり（builder でなく素朴な pub-field struct +
/// `#[non_exhaustive]` で拡張余地を残す）。
#[non_exhaustive]
pub struct RootedOptions {
    pub max_hops: u32,             // link 展開回数上限 (default 40)
    pub max_depth: u32,            // locator 成分数上限 (default 40)
    pub max_components_total: u32, // 展開込み総成分 budget (default 512)
    pub max_open_dirs: u32,        // 同時保持 dir handle 上限 (default 64)
                                   //   — fd budget。超過は LimitExceeded
}

/// open/metadata 時点で確定した object facts。
#[non_exhaustive]
pub struct NodeInfo {
    /// resolved object の kind（WithinRoot で link を辿った先。
    /// NoFollow または非 link は leaf 自身）。
    pub kind: NodeKind,
    /// leaf の no-follow kind（SV `via_symlink` 報告に対応）。
    pub leaf_kind: NodeKind,
    pub byte_len: Option<u64>,
    pub modified: Option<SystemTime>,
    /// resolved object の identity（loop guard・pin 比較に使える）。
    pub identity: NodeId,
    /// identity の強さ（下記「identity の強さ」）。
    pub identity_strength: IdentityStrength,
}

#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IdentityStrength {
    /// kernel-exported handle または birth time で delete+recreate を
    /// 判別できる（pin/open_pinned が有効）。
    Strong,
    /// ctime/slot 由来のみ — 同一 timestamp tick の inode 再利用を
    /// 判別できない。pin/open_pinned は `Unsupported`。
    Weak,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId { /* opaque — volume+index+kind+discriminator(+handle) */ }

/// leaf regular file 向けの opaque identity token。SV の content-digest
/// pin ではなく object-identity pin（digest 検証は consumer が
/// reader 上で行う責務のまま）。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodePin { /* id: NodeId（Strong のみ発行） */ }

#[non_exhaustive]
pub struct DirEntry {
    pub name: OsString,          // 非 UTF-8 は RelativePath 化不能（呼出し側で skip）
    pub kind: NodeKind,          // leaf no-follow 判定（Symlink を可視化）
}
```

- `metadata(rel, mode)` は 1 call で leaf kind（no-follow）と
  resolved object の kind/len/modified/identity/strength を返す。
  `WithinRoot` で leaf link の target が root 外なら `EscapesRoot`
  — その場合 leaf が link だった事実を知りたい呼び出し側は
  `metadata(rel, NoFollow)` の 2-call pattern を使う（SV の
  `via_symlink` 報告レシピとして doc に明記）。
- `open_regular(rel, mode)` は同一 handle の metadata/length と
  `Read + Seek` reader を対応させて返す。nonregular（dir/link
  under NoFollow/FIFO/device/socket）は typed 失敗。
- `FollowMode::NoFollow` の per-API 規則:
  - `metadata`: leaf が link なら `kind = Symlink` で成功を返す
    （link の identity/len は link 自身）。
  - `open_regular`: leaf link は `NotRegularFile`。
  - `open_dir`/`read_dir`: leaf link は `NotDirectory`。
  - 中間 component の link（全 API）: `NotDirectory`（traverse 不能）。
- `FollowMode::WithinRoot` は中間・leaf どちらの link も同じ規則で
  解決する（SV canonicalize 相当）。

### FileErrorKind 拡張（`src/error.rs`）

既存 `{Io, Unsupported, InvalidInput}` に追加（`#[non_exhaustive]`
維持）。全 io error は adapter 境界で正規化し、生の `io::Error`
は `source` に残る。

| `FileErrorKind` | 発生条件 / 写像元 |
| --- | --- |
| `Io` | 上記以外すべて（`EACCES`/`EPERM`/`EIO`/`EMFILE`/`EBADF`…）。`io::ErrorKind::PermissionDenied` は source で保持されるが kind は `Io`。 |
| `Unsupported` | `io::ErrorKind::Unsupported`、非 unix/windows target、RS1 未満 Windows、`/proc` 不在の Linux、identity 弱い FS での `pin`/`open_pinned`、guarantee 不能 FS。 |
| `InvalidInput` | `io::ErrorKind::InvalidInput`、`RelativePath` 文法違反。 |
| `NotFound` | `ENOENT`/`io::ErrorKind::NotFound`/`STATUS_OBJECT_NAME_NOT_FOUND` 系。 |
| `NotDirectory` | `ENOTDIR`/`NotADirectory`/`STATUS_NOT_A_DIRECTORY`、dir 期待位置の非 dir、NoFollow での中間・leaf link（`open_dir`/`read_dir`）。 |
| `NotRegularFile` | `EISDIR`、nonregular leaf（FIFO/device/socket/dir）、`open_regular`/`pin` の leaf link under NoFollow、leaf link の `open_regular` での `O_NOFOLLOW` ELOOP。 |
| `EscapesRoot` | 絶対 link の root prefix 外、TEXTUAL フェーズが capability root に復帰しない、`..` が stack floor（capability root）を割る、link target 解決が root 外 object に触れる。 |
| `LoopDetected` | hop 上限超過、link 環の展開（resolve 側が raise — kernel `ELOOP` とは区別）。 |
| `IdentityMismatch` | `open_pinned` の identity 不一致（差し替え検出）。 |
| `LimitExceeded` | `max_depth`/`max_components_total`/`max_open_dirs` 超過。 |

`SymlinkEncountered` という専用 variant は**設けない**（上表の
per-API 規則で `NotRegularFile`/`NotDirectory` に吸収し、kind 判別が
必要な呼び出し側は `metadata` で leaf kind を取る — 2 種類の
「link だった」を 1 つの API で分ける必要があるのは metadata の
みで、それは `kind` フィールドで表現できる）。

## vfs 層（`#[doc(hidden)]` 内部共有）

component 相対 op はすべて **no-follow のみ**（`no_follow` 引数は
取らない — link follow は resolve 層が `read_link` で明示的に行う
唯一の経路）。write 系 op は archive staging の現行意味をそのまま
引き継ぐ（下表）。

```rust
#[doc(hidden)]
pub trait VfsOps: Send + Sync + fmt::Debug {
    /// 絶対 path で dir を開く（唯一の path 解決点。caller の
    /// trust boundary）。unix `open(O_DIRECTORY|O_CLOEXEC)` /
    /// windows `NtOpenFile`。
    fn open_dir_path(&self, path: &Path) -> io::Result<Box<dyn VfsDir>>;
    /// 開いた `File` の identity/probe（unix `fstat` /
    /// windows `FileIdInfo`+`FileBasicInfo`+`FileStandardInfo`）。
    /// post-open の regular/identity 検証と archive `StagingFile::
    /// identity` に使う。
    fn stat_file(&self, file: &File) -> io::Result<VfsProbe>;
}

#[doc(hidden)]
pub trait VfsDir: Send + Sync + fmt::Debug {
    /// 診断 path（identity の証明ではない）。
    fn path(&self) -> &Path;
    /// この dir 自身の identity（unix `fstat` / windows `FileIdInfo`）。
    fn identity(&self) -> io::Result<NodeIdentity>;
    /// この dir 自身の probe（`metadata("")`・link target が root
    /// 自身に終わる場合の `NodeInfo` 用）。unix `fstat`、
    /// windows self handle への Basic+Id+StandardInfo。
    fn stat_self(&self) -> io::Result<VfsProbe>;
    /// child `name` の no-follow probe（kind/identity/len/mtime/
    /// strength）。unix `fstatat(AT_SYMLINK_NOFOLLOW)`;
    /// windows attrs-only open（`FILE_OPEN_REPARSE_POINT`、
    /// `OBJ_DONT_REPARSE` なし）+ info 問合せ → reparse object の
    /// attributes/tag を返す。
    fn probe(&self, name: &OsStr) -> io::Result<VfsProbe>;
    /// child dir の no-follow open（link/非 dir は失敗）。
    /// unix `openat(O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC)`;
    /// windows `NtOpenFile(FILE_DIRECTORY_FILE|FILE_OPEN_REPARSE_POINT)`
    /// + post-open attributes で reparse flag を検査し link なら失敗。
    fn open_dir(&self, name: &OsStr) -> io::Result<Box<dyn VfsDir>>;
    /// child file の検証付き read open — per-OS の「nonregular を
    /// 通常 open へ流さない」実装（下記 leaf 確定手順）。
    /// 返り値は `(File, 同一 object の VfsProbe)`。
    fn open_regular_leaf(&self, name: &OsStr) -> io::Result<(File, VfsProbe)>;
    /// `name` の link target 文字列を読む（OS raw 形 — `\??\` 等の
    /// namespace 変換は resolve 層）。非 link は `InvalidInput`。
    fn read_link(&self, name: &OsStr) -> io::Result<OsString>;
    /// entry 列挙。`.`/`..` を除外し、各 entry に no-follow kind を
    /// 付ける。unix は呼出しごとに `openat(dirfd, ".")` で独立 offset の
    /// fd を取り `fdopendir`（共有 dirfd の offset を壊さない）;
    /// `DT_UNKNOWN` は entry ごとに `probe` で fallback 分類。
    /// windows は `NtQueryDirectoryFile` を `RestartScan` で呼ぶ。
    fn list(&self) -> io::Result<Vec<VfsEntry>>;

    // ---- archive staging 用（現行意味を固定で引き継ぐ） ----
    /// 排他 dir 作成（unix `mkdirat(0700)`、windows
    /// `NtCreateFile(FILE_CREATE|FILE_DIRECTORY_FILE|
    /// FILE_OPEN_REPARSE_POINT)` + DIR_ACCESS+SHARE_ALL）。
    /// `AlreadyExists`（link 含む全既存名を拒否）。
    fn create_dir(&self, name: &OsStr) -> io::Result<Box<dyn VfsDir>>;
    /// 排他 file 作成（unix `openat(O_WRONLY|O_CREAT|O_EXCL|
    /// O_NOFOLLOW, 0600)`、windows `NtCreateFile(FILE_CREATE|
    /// FILE_NON_DIRECTORY_FILE|FILE_OPEN_REPARSE_POINT)` +
    /// `FILE_GENERIC_WRITE|FILE_READ_ATTRIBUTES` + ShareAccess=0）。
    fn create_file(&self, name: &OsStr) -> io::Result<File>;
    /// leaf file 削除（unix `unlinkat(0)` — swap した dir は EISDIR で
    /// 消えない。windows `NtDeleteFile(OBJ_DONT_REPARSE)` — kind
    /// 非検査で dir も消えるので、kind 検証は session 側の
    /// probe→kind 分岐で担保する = 現行意味）。
    fn remove_file(&self, name: &OsStr) -> io::Result<()>;
    /// leaf 空 dir 削除（unix `unlinkat(AT_REMOVEDIR)` — swap した
    /// file は ENOTDIR で拒否。windows `NtDeleteFile(OBJ_DONT_REPARSE)`）。
    fn remove_dir(&self, name: &OsStr) -> io::Result<()>;
}

### Windows per-op flags/access/share（vfs windows backend 契約）

| op | Access | Share | Options/Attrs |
| --- | --- | --- | --- |
| `open_dir_path` | `FILE_LIST_DIRECTORY\|FILE_TRAVERSE\|FILE_READ_ATTRIBUTES\|SYNCHRONIZE` | `SHARE_ALL` | `FILE_DIRECTORY_FILE\|FILE_SYNCHRONOUS_IO_NONALERT`; attrs `OBJ_CASE_INSENSITIVE`（path-level、trust boundary） |
| `open_dir`/`stat_self`/`identity` | 同上 DIR_ACCESS | `SHARE_ALL` | `FILE_DIRECTORY_FILE\|FILE_OPEN_REPARSE_POINT\|FILE_SYNCHRONOUS_IO_NONALERT`; attrs `OBJ_DONT_REPARSE\|OBJ_CASE_INSENSITIVE` + post-open attrs で reparse flag 検査（link → 失敗） |
| `probe`/`read_link` | `FILE_READ_ATTRIBUTES\|SYNCHRONIZE` | `SHARE_ALL` | `FILE_OPEN_REPARSE_POINT\|FILE_SYNCHRONOUS_IO_NONALERT`; attrs `OBJ_CASE_INSENSITIVE`（**`OBJ_DONT_REPARSE` なし** — leaf reparse object を開いて tag/target を得る） |
| `open_regular_leaf` | `FILE_GENERIC_READ\|SYNCHRONIZE` | `SHARE_ALL` | `FILE_NON_DIRECTORY_FILE\|FILE_OPEN_REPARSE_POINT\|FILE_SYNCHRONOUS_IO_NONALERT`; attrs `OBJ_CASE_INSENSITIVE` |
| `list` | dir handle の DIR_ACCESS を流用 | — | `NtQueryDirectoryFile` `RestartScan` |
| `create_dir` | DIR_ACCESS | `SHARE_ALL` | `NtCreateFile FILE_CREATE\|FILE_DIRECTORY_FILE\|FILE_OPEN_REPARSE_POINT\|FILE_SYNCHRONOUS_IO_NONALERT`; attrs `OBJ_DONT_REPARSE\|OBJ_CASE_INSENSITIVE`; `FILE_ATTRIBUTE_DIRECTORY` |
| `create_file` | `FILE_GENERIC_WRITE\|FILE_READ_ATTRIBUTES` | **ShareAccess=0** | `NtCreateFile FILE_CREATE\|FILE_NON_DIRECTORY_FILE\|FILE_OPEN_REPARSE_POINT\|FILE_SYNCHRONOUS_IO_NONALERT`; attrs `OBJ_DONT_REPARSE\|OBJ_CASE_INSENSITIVE`; `FILE_ATTRIBUTE_NORMAL` |
| `remove_file`/`remove_dir` | — | — | `NtDeleteFile`; attrs `OBJ_DONT_REPARSE\|OBJ_CASE_INSENSITIVE`（link object を消し target を辿らない） |

`OBJ_DONT_REPARSE` は Windows 10 RS1 (build 14393) 以降でのみ有効で、
それ未満では silent-ignore される — vfs ops 初期化時に
`RtlGetVersion`（または同等）で build ≥14393 を検査し、未満なら
全 op `Unsupported` とする。`FILE_OPEN_REPARSE_POINT` は単独でも
leaf no-follow を保つ（中間 traversal の差し替え検出に
`OBJ_DONT_REPARSE` を重ねる二重防御）。

### vfs 型

#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VfsProbe {
    pub identity: NodeIdentity,
    pub kind: VfsKind,            // File|Dir|Symlink|Other（non_exhaustive）
    pub byte_len: Option<u64>,
    pub modified: Option<std::time::SystemTime>,
    pub strength: IdentityStrength, // 内部型 — rooted の IdentityStrength と同値
}

#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VfsEntry {
    pub name: OsString,
    pub kind: VfsKind,
}
```

- archive `StagingDir`→`VfsDir` の mapping（session 意味を保存）:

  | `StagingDir` (現行) | vfs 対応 |
  | --- | --- |
  | `identity()` | `VfsDir::identity` |
  | `probe_identity(name)` | `VfsDir::probe(name).identity` |
  | `create_dir(name)` | `VfsDir::create_dir`（意味は現行どおり固定） |
  | `open_dir(name)` | `VfsDir::open_dir` |
  | `create_file(name)` | `VfsDir::create_file`（`File` を `UnixFile`/`WinFile` で wrap） |
  | `remove_file`/`remove_dir` | 同名 op（`unlinkat` flag 分離を保持） |
  | `StagingFile::identity` | `VfsOps::stat_file(&file).identity` |
  | `create_staging_root(parent)` | `ops.open_dir_path(absolute(parent))` → retry `create_dir(unique)` |
  | `open_staging_root(path)` | `ops.open_dir_path(parent)` + `parent_vfs.open_dir(leaf)` — **leaf は必ず component no-follow open で、path-level 一括 open に展開しない**（swapped link at root name を辿らない現行意味を保持） |
  | `StagingRoot::remove_root` | adapter が保持する parent `VfsDir` + leaf name + identity 再検査 → `remove_dir` |

- `RootedDir::open` の `open_dir_path` は caller の trust boundary
  （canonicalize 済み path をそのまま開く — leaf no-follow は不要、
  canonical path は実 dir を指す前提。非 dir は backend が
  `O_DIRECTORY`/`FILE_DIRECTORY_FILE` で失敗 → `NotDirectory`）。

## resolve アルゴリズム（`rooted/resolve.rs`）

### 状態と不変条件

```text
chain: Vec<Box<dyn VfsDir>>   // chain[0] = capability root の dir handle
                              // chain[i] = 解決済み ancestor dir（すべて
                              //   実 dir の handle。link は絶対に載せない）
hops: u32                     // link 展開回数（>max_hops → LoopDetected）
total: u32                    // 処理済み成分総数（>max_components_total → LimitExceeded）
```

locator の各成分を `probe` → `open_dir` で順に stack に積む。
`..` は locator からは来ない（`RelativePath` が拒否）が、link
target からは来る — `chain.len()>1` なら pop、`==1`（capability
root floor）なら `EscapesRoot`。`..` の pop は resolved ancestor
handle を戻すだけなので kernel `..` と同じ「解決済み parent に
戻る」意味（かつ rename 攻撃にも強い — fd は inode を pin する）。
`chain.len()` が `max_open_dirs` を超えるなら `LimitExceeded`。

### 成分規則

- 成分 `probe(no-follow)` → `Dir` → `open_dir` → push。
- `Symlink` → `read_link` → 下記 link 解決（hop+1）。`NoFollow`
  では leaf 位置ならそのまま `kind=Symlink` を報告、中間位置は
  `NotDirectory`。
- `File`/`Other` → leaf 位置以外では `NotDirectory`。
- leaf 位置の `File`/`Other`/`Dir` → 各 API の leaf 規則へ。

### link target 解決（WithinRoot）

`read_link` の target を OS 区切り（unix `/`、windows `/` と `\`）
で成分列に分解。`.・空` は除去。`..` と name を残す。

- **相対 target**: 現在の `chain` 上でそのまま解決（name は
  `probe`→dir なら push・leaf 規則、`..` は pop/floor→EscapesRoot、
  link は再帰的に hop+1）。
- **絶対 target**（unix `/` 始まり、windows drive/UNC/`\??\` 系 —
  windows の namespace bridge は下記）は **two-state walk**:

```text
mode = TEXTUAL, virtual = []            // canonical root path への
                                        // 一致を文字列で追う段階
tainted = false                          // 一度 R-prefix を外れたら
                                        // capability 内解決に戻れない
if R.components().is_empty() or virtual == R.components():
    mode = FD; chain = [root_handle]    // root が `/` の退化 case と
                                        // target==R の pre-loop 受理
for comp in target.components():
    total += 1; if total > max_components_total: LimitExceeded
    if TEXTUAL:
        if comp == "..": virtual.pop() or clamp(空)   // kernel `/..`=/ に倣う
        else: virtual.push(comp)
        if virtual == R.components(): mode = FD; chain = [root_handle]
        else if !is_prefix_of(virtual, R.components()): tainted = true
    else /* FD */:
        if comp == "..":
            if chain.len() > 1: chain.pop()
            else: mode = TEXTUAL; virtual = R.parent_components(); tainted = true
        else:
            probe/open_dir/comp-or-link（通常規則; link は再帰）
            // tainted 中に TEXTUAL へ出ても R へ復帰しない限り FD に戻れない
            // tainted 中の更なる解決失敗（NotFound/LoopDetected 等）で
            // 早期 return せず、tainted が勝つ — error kind を決定的に
            // するため、TEXTUAL/FD どちらでも「この target の解決を
            // 打ち切る」条件が揃った時点で EscapesRoot を返す
after: mode == FD かつ !tainted → resolved object; さもなければ EscapesRoot
       （tainted は sticky — どこかで R-prefix を外れた target は
        必ず EscapesRoot になる決定的な規則）
```

ポイント:

- **prefix 判定は lexical**（R は canonical — 実 dir が保証された
  成分列。TEXTUAL で `..` を fold しても、prefix 部分は link を
  含まない実 dir 列なので kernel と一致する）。R-prefix を一度でも
  外れた target は tainted として capability 内解決を認めない —
  `/outside/../r/x` のような「外へ出て戻る」target は SV の
  canonicalize では受理され得るが、外 object を開かずには判定
  不能なため**保守的に `EscapesRoot`**（意図的な縮小として明記。
  外 intermediate を経由する解決はどのみち capability では扱え
  ない）。
- **`..` は suffix で fold しない**（旧設計の F1 欠陥の修正）—
  lexical fold は「cancel される成分が link」のとき kernel 意味と
  ずれる。suffix は raw 成分のまま FD stack で解決し、`..` は
  resolved dir の pop なので kernel の post-substitution 意味と
  一致する。
- 絶対 target が R-prefix 完全一致しない場合は TEXTUAL のまま終了
  → `EscapesRoot`。
- hop は再帰的な link 展開すべてで共有して消費する（環は必ず
  `LoopDetected` で停止）。

### Windows の link target namespace bridge

canonical root は `\\?\X:\…`（std canonicalize の verbatim 形）。
reparse buffer の substitute name は NT namespace で来ることが
あるため、prefix 比較の前に以下で統一する:

| target 形 | 処理 |
| --- | --- |
| `\??\X:\…` | `X:\…` に写像（junction/mount-point の常用形） |
| `\??\UNC\srv\sh\…` | `\\srv\sh\…` に写像 |
| `\\?\X:\…` / `\\?\UNC\…` | verbatim prefix を剥がして `X:\…`/`\\…` へ |
| `X:\…` / `\\srv\sh\…` | そのまま（DOS symlink の常用形） |
| `\??\Volume{guid}\…` | drive letter への写像は保証不能 → `Unsupported` |
| `\foo\…`（単一 `\` 始まり、same-drive absolute） | volume 判定ができない → `EscapesRoot` |
| `C:foo`（drive-relative） | 同上 → `EscapesRoot` |
| 相対 target | `\\?\` root 内の相対成分として扱う（`\\` では始まらない） |
| 空 target（`""`） | kernel `ENOENT` 相当 → `NotFound` |

prefix 比較は **component-wise かつ case-insensitive**（NT の
`OBJ_CASE_INSENSITIVE` name lookup に合わせる。per-directory
case-sensitivity が立つ FS では「lexical 一致 ≠ kernel 一致」に
なり得るが、その場合のずれは in-root の wrong-accept/wrong-reject
に留まり escape にはならない — 限界として明記）。

reparse tag → `VfsKind`/`NodeKind` 分類: `IO_REPARSE_TAG_SYMLINK`
と `IO_REPARSE_TAG_MOUNT_POINT`（junction）は `Symlink`（target
解決対象）。その他（`APPEXECLINK`/`CLOUD`/`DEDUP`/非 name-
surrogate tag）は `Other` で、**決して辿らない**。

## leaf 確定手順（`open_regular` / `open_pinned`）

resolve で leaf 名が確定したあと、vfs の `open_regular_leaf(name)`
が per-OS で「nonregular を通常 open へ流さない」を実装する:

- **Linux**: `openat(dirfd, name, O_PATH|O_NOFOLLOW|O_CLOEXEC)` —
  inode を pin するだけで block も device open 意味も発生しない
  （O_PATH fd への `fstat` は Linux 3.6+ で動作）。**その fd への
  `fstat` が probe そのもの**（同一 object — probe→open 窓は
  構造的に存在しない）。`S_ISREG` でなければ close して
  `NotRegularFile`。regular なら
  `open("/proc/self/fd/N", O_RDONLY|O_CLOEXEC)` で同一 inode の
  読取 fd を得て、reopen 後 `fstat` の identity が O_PATH fd の
  fstat と一致することを検証する（magic link 解決は inode 参照で
  path 再解決を行わない — 偽 `/proc` mount 検出の二重防御。
  reopen 時に inode 単位の `MAY_READ`/LSM 検査は再実行される）。
  `/proc` 不在・reopen 不能なら `Unsupported`。
- **他 unix（macOS 等、O_PATH なし）**:
  `openat(O_RDONLY|O_NOFOLLOW|O_NONBLOCK|O_CLOEXEC)` → `fstat`
  で `S_ISREG` を検証（regular かどうかの判定は open fd 自身 —
  probe→open の concurrent rename で別 regular file が来ても
  in-root regular なので受理し、非 regular なら `NotRegularFile`）
  → **返却前に `fcntl(F_SETFL)` で `O_NONBLOCK` をクリア**
  （返す `File` の read 系が blocking 意味で動くため）。
  O_NONBLOCK で FIFO 待ちは発生せず、device open 副作用のみが
  残留リスク（doc に明記）。pin 系の identity 照合だけが
  `stat_file` で別途行う。
- **Windows**: `NtOpenFile(RootDirectory=dir, name,
  FILE_NON_DIRECTORY_FILE|FILE_OPEN_REPARSE_POINT|
  FILE_SYNCHRONOUS_IO_NONALERT, FILE_GENERIC_READ,
  SHARE_ALL)`。synchronous non-alert open は async I/O の pend を
  生じない（「待たない」ではなく「synchronous 完了する」— NTFS
  での dir entry 経由で device/pipe object は実質到達しない）。
  開放後 `FileBasicInfo`+`FileIdInfo`+`FileStandardInfo` で
  attributes（`REPARSE_POINT`/`DIRECTORY` なし）・kind・identity・
  len を検証。`OBJ_DONT_REPARSE` は leaf では使わない（leaf の
  reparse object を**開く**必要があるため — traversal の中間
  component では差し替え検出に使う）。
- `open_pinned` は `VfsOps::stat_file(&file)`（または
  `VfsProbe`）の identity を `NodePin` と照合し、不一致は
  `IdentityMismatch`（reader 未返却）。

## identity の強さ（pin/open_pinned の floor）

`NodeIdentity` = volume+index+kind tag+discriminator（+kernel
handle）。`IdentityStrength`:

| backend | Strong | Weak |
| --- | --- | --- |
| linux gnu | `name_to_handle_at` 成功 **または** statx `STATX_BTIME` | 上記なく ctime のみ |
| apple | `st_birthtimespec` 取得 | — |
| 他 unix | stat が birth time を報告 | ctime のみ（musl 等 `cfg(gnu)` 外はここに落ち得る） |
| windows NTFS/ReFS | `FileIdInfo` の 128-bit id + volume serial | — |
| windows FAT/exFAT/他 FS | — | slot 由来 id+荒い timestamp（`GetVolumeInformationByHandleW` の fs 名で判定） |

`pin`/`open_pinned`/`open_regular` のうち、**pin と open_pinned
だけが Strong を要求**する（Weak → `Unsupported`。判定は
`VfsProbe.strength` = per-object の authoritative gate — backend
global ではなく、FAT/NTFS 混在環境でも object 単位で正しい）。
`metadata`/
`open_regular`/`read_dir` は Weak でも動く（swap 検出保証を約束
しない — ただし open_regular の同一 handle facts は常に有効）。
SV 側は `open_pinned` が `Unsupported` なら自前の digest verify
経路だけに縮退できる — capability が嘘の保証を名乗るより正直な
`Unsupported` が正しい。

## browse 降下（SV browse-descend 対応）

- `read_dir(rel, mode)` は `.`/`..` を除く entry 列を返し、各
  entry の kind は leaf no-follow 判定（link entry を `Symlink`
  で可視化 — SV `via_symlink` 報告に対応）。
- `open_dir(rel, WithinRoot)` は leaf link→dir（root 内）も降下
  できる。scoped `RootedDir` は resolved ancestor chain を保持し、
  chain 内の `..` は合法（capability root より下には出ない）。
- browse loop guard は caller の責務 — `dir_identity()` が canonical
  path 集合の代替として使える `NodeId` を返す。
- `read_dir`/`list` は呼出しごとに独立した enumeration state を
  取る（unix は `openat(dirfd,".")` で新 fd、windows は
  `NtQueryDirectoryFile` `RestartScan`）— 共有 dirfd の offset
  干渉はない。

## 意味論・限界

| 項目 | 契約 |
| --- | --- |
| 絶対 symlink | TEXTUAL prefix 判定（canonical root component 一致・case 規則は OS 準拠）+ raw suffix の FD walk。R-prefix を外れる target は tainted → `EscapesRoot`（SV canonicalize が受理し得る「外を経由して戻る」形は保守的に拒否 — 意図的な縮小として明記）。同じく **同一 dir の非 canonical alias 綴り**（`/var` vs `/private/var`・8.3 短縮名・bind mount・`subst`・firmlink 由来の別綴り）で書かれた in-root target も kernel 的には内だが lexical 判定では外 → 保守的 `EscapesRoot`（alias を判定するには capability が排除する path 再解決が必要になるための意図的縮小。受理されるのは `canonical_root()` の成分列で綴られた target のみ）。 |
| `..` | locator 内の `..` は `RelativePath` が拒否。link target 内の `..` は resolved chain の pop（kernel post-substitution 意味に忠実）。capability root floor を割ると `EscapesRoot`。scoped dir の chain 内は合法。 |
| symlink loop | hop 上限（既定 40）で `LoopDetected`。link 環は hop 消費で必ず停止。 |
| hop/depth limit | `max_depth`（locator 成分、既定 40）/`max_components_total`（展開込み、既定 512）/`max_open_dirs`（同時 dir handle、既定 64）。`LimitExceeded`。resolve は成分ごとに 1 個まで dir fd を保持（EMFILE 自体は `Io`）。 |
| reparse point | SYMLINK/MOUNT_POINT は `Symlink` として target 解決対象、その他 tag は `Other` で不辿。`FILE_OPEN_REPARSE_POINT` で link object を可視化、中間 traversal は `OBJ_DONT_REPARSE` 相当で差し替え検出。**Windows 10 RS1 (build 14393) 未満は `OBJ_DONT_REPARSE` が silent-ignore されるため ops 初期化時に version gate → `Unsupported`**。 |
| mount | unix mount/bind mount 横断は kernel namespace の実体としてそのまま通過する（SV canonicalize も同じ穴 — 非回帰だが「root 外 byte/facts を出さない」は **mount namespace 内の FS tree に scope される**と正直に明記。autofs の traverse 副作用も起こり得る）。同一 `st_dev` の bind mount は identity でも検出不能。Windows mount は reparse で link 扱い。 |
| 同時 rename/swap | 各 component は no-follow dir handle から辿る。差し替えは当該 step で `NotFound`/`EscapesRoot`/`IdentityMismatch`/`NotRegularFile` のいずれかになり、**解決 walk の届く範囲外の object は開かない**（authority は取得済 handle subtree への委譲 — 取得済 scoped dir が root 外へ rename された後の継続 authority も正当: 下段「scoped dir の移動」参照）。resolved ancestor fd は inode を pin するので walk 中の ancestor rename でも脱出しない。 |
| 権限 | handle 相対でも OS の権限検査は有効。permission 失敗は `Io`（source に `PermissionDenied`）。root handle の権限は `RootedDir::open` 時に確立。 |
| identity | `NodeId`/`NodePin` は volume+index+kind+discriminator(+handle) の opaque 公開形。同 FS 内の同一 object で Eq。ctime 変動（chmod/link/metadata touch）で「同一 object が変化」も mismatch になり得る（fail-closed 安全側 — SV の content-pin とは意味論が異なり、doc に明記）。weak FS では pin 系が `Unsupported`。 |
| case sensitivity | unix = byte/UTF-8 名の exact、windows = NT の case-insensitive lookup。絶対 link の prefix 比較は windows で case-insensitive（per-dir case-sensitive FS とのずれは in-root wrong-accept/reject の限界として明記）。 |
| 非 UTF-8 name | entry name は `OsString` で返す。`RelativePath` は UTF-8 のみ受理し、非 UTF-8 entry は locator 化不能（呼出し側で skip — SV と同じ）。 |
| FIFO/device/socket | `open_regular` は開かず `NotRegularFile`。Linux は O_PATH 参照で副作用なしに判定、他 unix は nonblock open+即検証（device open 副作用のみ残留、明記）、Windows は synchronous non-alert open+attributes 判定。 |
| root の差し替え | `RootedDir` は open 時点の root inode を pin。root 名の rename/差し替え後も能力は pin した inode に閉じる。絶対 link の prefix 判定は canonical 文字列基準のまま（inode と文字列のずれは link 受理ポリシーの不整合として明記 — escape ではない）。 |
| scoped dir の移動 | authority は取得済 dir handle（inode subtree）への委譲であり、path namespace の継続的包含ではない。scoped dir が取得後に root 外へ rename されても、handle は移動した subtree を正当に読む（cap-std 系の標準的意味論）。namespace 連続性を各 op で再検証する仕組みは存在せず、再 canonicalize は本 capability が排除する TOCTOU の再導入なので採用しない — 宣言契約として明記し黙示の保証とはしない。 |
| staging 意味 | `create_dir`/`create_file`/`remove_file`/`remove_dir` は archive 現行意味をそのまま vfs へ（0700/0600、exclusive、`FILE_READ_ATTRIBUTES`、ShareAccess=0、`unlinkat` flag 分離）。`open_staging_root` は parent open+leaf no-follow component open に分解（一括 path open で leaf を辿らない）。 |

## セキュリティ・安全性分析

- **TOCTOU**: 検査（canonicalize）と利用（open）を分離する SV の
  構造的欠陥を、open 自体を handle 相対にすることで除去する。
  `RootedDir::open` の canonicalize が唯一の trust boundary。
- **hash oracle**: `pin` が外部内容を digest し得る問題は、
  pin→open が同一 identity でなければ失敗する `open_pinned` と、
  resolve が root 外を構造的に開かないことで排除する（SV 側の
  digest 自体は consumer の責務）。`NodePin` は object-identity
  token であり content pin ではない — SV は digest 検証を reader
  上で継続する（併用で defense-in-depth）。
- **blocking/side-effect open**: nonregular を ordinary open へ
  流さない設計で open 時点の FIFO 待ち・device 副作用を除去
  （残留は非 Linux unix の probe→open window のみ、明記）。
- **unsafe**: `#[expect(unsafe_code)]` + SAFETY comment の局所
  許可のみ（既存 vfs/extract 実装の流儀を継承）。
- **path fallback 禁止**: `/proc` 不在・RS1 未満・identity 弱い
  FS では `Unsupported`/`Io` で失敗し、静かに path 解決へ縮退
  しない。
- **identity 弱い FS**: `pin`/`open_pinned` が `Unsupported` を
  返すことで「差し替えを検出できないのに検出した風」を防ぐ。

## テスト方針

`docs/test-specs/rooted-read-capability.md` が正本。要旨:

- **scripted vfs mock（`#[cfg(test)]`）**: 擬似 FS tree
  （`TestNode{kind,id,target,data,outside,strength}`）を共有
  mutate 可能にし、leaf/intermediate swap・`..` fold の kernel
  意味・absolute link の two-state walk 分岐・tainted 規則・
  escape/loop/limit/失敗分類を OS 非依存で RED→GREEN。
  `outside=true` node の data/facts が返却・digest・metadata に
  混入しないことを assert。
- **実 FS 回帰（`tests/rooted_read.rs`）**: leaf/intermediate
  swap・`WithinRoot` 正常系（leaf/中間/絶対 link・`..` link
  target・browse 降下）・escape（絶対/`..` 溢れ/外 link/
  tainted）・loop/limit・nonregular（dir・fifo）・windows
  reparse・`FileErrorKind` 分類・weak identity → `Unsupported`。
- **archive 非回帰**: `cargo test -p ruviform-io-archive` 既存
  suite が vfs 集約後も全 pass。
- **正規入口**: `mise run fmt-check`/`check`/`lint`/`test`。

## Deferred Design Concerns

- `openat2`/`RESOLVE_BENEATH`/`RESOLVE_NO_XDEV`（mount 横断を
  kernel 保証で閉じる候補）・`cap-std` 系採用は別 Issue 候補。
- vfs の独立 crate 昇格（3 つ目の消費者・独立版管理が必要に
  なった時点で ADR 再評価）。
- `read_dir` の streaming iterator・`read_at` 系・mode/uid/gid 等の
  拡張 facts。
- write 側 rooted capability の公開（archive staging の read 面は
  ExtractedTree→ReadbackFile の既存経路を維持）。
- 非 UTF-8 locator を正規化せずに通す低層 API。
- `\??\Volume{guid}\` target を drive-letter root と同一 volume
  として受理する写像（GetVolumeInformation 等、必要性が出たら）。
