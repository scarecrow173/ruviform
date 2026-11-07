---
id: "TASK-F1hO5AvuiammHw0t5nBKxP"
type: "task"
status: "done"
title: "T1 vfs 共有層抽出 — io-file への OS primitive 集約と archive adapter 化"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-file/src/vfs.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/vfs/unix.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/vfs/windows.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/vfs/unsupported.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-file/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-archive/src/extract/fs.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/Cargo.toml"
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
  implements:
    - "docs/plans/rooted-read-capability.md"
  implemented-by: []
  depends-on:
    - "docs/plans/rooted-read-capability.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rooted-read-capability.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 vfs 共有層抽出 — io-file への OS primitive 集約と archive adapter 化

## Work

`ruviform-io-archive/src/extract/{fs,unix,windows}.rs` の OS 相関
primitive（dir handle 相対 open/create/remove/rename・
`fstat(at)`/`GetFileInformationByHandleEx`・`NodeIdentity`・
readlink・dir 列挙・`open_path_handle` 相当の安全参照）を
`ruviform-io-file::vfs`（`#[doc(hidden)] pub`）へ移す。

- `vfs.rs`: `VfsOps`/`VfsDir`/`VfsEntry`/`VfsProbe`/`OpenFlags`/
  `NodeIdentity` trait・`#[cfg(target_os)]` factory・
  `#[cfg(test)]` scripted mock（`TestNode{kind,id,target,data,
  outside}` を共有 mutate 可能な擬似 FS）。
- `vfs/unix.rs`: `openat`/`openat(O_PATH|O_NOFOLLOW)`/
  `fstatat(AT_SYMLINK_NOFOLLOW)`/`readlinkat`/`fdopendir`/
  `mkdirat`/`unlinkat`/`renameat`/`fstat`・`dup` 閉じた allocs・
  `#[expect(unsafe_code)]` + SAFETY comment（IO-IMP-10 と同形）。
- `vfs/windows.rs`: `NtCreateFile`/`NtOpenFile` RootDirectory・
  `FILE_OPEN_REPARSE_POINT`/`OBJ_DONT_REPARSE` 等・
  `FileBasicInfo`/`FileStandardInfo`/`FileIdInfo`・
  `NtQueryDirectoryFile` 列挙・Wdk feature 群を io-file
  `Cargo.toml` の `windows-sys` に追加。
- `vfs/unsupported.rs`: 全 op `Unsupported`/`io::ErrorKind::Unsupported`。
- archive `extract/fs.rs` は `PlatformExtractOps`/`StagingDir`/
  `StagingFile`/`StagingRoot` trait 形を維持したまま vfs 呼び出しの
  adapter にし、`extract/{unix,windows}.rs` を削除。
- archive `Cargo.toml` から `libc`/`windows-sys` を外し
  `ruviform-io-file = { path = "../ruviform-io-file" }` を追加。
- `Cargo.lock` 再生成。

## Verification

- `cargo test -p ruviform-io-archive`（既存 `extract_*` 全 suite
  + session/records/cleanup/recovery）が vfs 集約後も全 pass。
- `mise run check`・`mise run lint`（workspace, `-D warnings`）。
- archive crate README の依存図説を更新。
