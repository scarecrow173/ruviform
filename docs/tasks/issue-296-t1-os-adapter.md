---
id: "TASK-034RBIWkzzcEXV0X3v5ySr"
type: "task"
status: "done"
title: "T1 OsFileOps 実 OS アダプタ + factory 接続 (Issue #296)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-296-io-file-os-backend.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-296-io-file-os-backend.md"
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
    - "docs/test-specs/issue-296-io-file-os-backend.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 OsFileOps 実 OS アダプタ + factory 接続 (Issue #296)

## Work

`crates/ruviform-io-file/src/os.rs` を新設し、`PlatformFileOps` の実 OS
実装 `OsFileOps` を書く: temp sibling の `create_new` 排他生成・
`Write`/`Seek`・positioned readback（unix `read_at`/windows `seek_read`）・
`sync_file`（`sync_all`）・`prepare_for_publish`（ハンドル解放）・
`publish`（unix: `syscall(SYS_renameat2, RENAME_NOREPLACE)`+`link+unlink`
fallback、`renamex_np(RENAME_EXCL)` 系、windows: `MoveFileExW(0)`・
`fs::rename`）・`sync_directory`（unix dir fsync / windows
`FILE_FLAG_BACKUP_SEMANTICS`+read-write dir open + `sync_all`）・
`cleanup_temp`・`inspect_temp`・確定/あいまいエラー分類ヘルパ。
`platform.rs` の `staged_file()` を `OsFileOps::create` に接続し、trait に
`temp_path()` を追加（`mock.rs` も実装）。`staged.rs` の `create` で
`RecoveryInfo.temp_path` を記録し、「Unsupported」を返す既存テストを
実装済み仕様に更新。`Cargo.toml` に `libc`（unix）/`windows-sys`
（windows・`Win32_Foundation`+`Win32_Storage_FileSystem`）を target 限定で
追加し、`lib.rs` の backend 未実装記述を更新する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-296-io-file-os-backend.md`
- `docs/designs/issue-296-io-file-os-backend.md`
- `docs/specs/issue-296-io-file-os-backend.md`

## Dependencies

- 依存: `libc 0.2`（`cfg(unix)`）・`windows-sys 0.61`（`cfg(windows)`）
- 先行: #295 の `PlatformFileOps` 契約（変更しない）

## Affected Files

- `crates/ruviform-io-file/src/os.rs`（新設）
- `crates/ruviform-io-file/src/platform.rs` `staged.rs` `mock.rs` `lib.rs`
- `crates/ruviform-io-file/Cargo.toml`・`Cargo.lock`

## Done When

 - [ ] `StagedFile::create` が実 FS で temp を作り `temp_path()` を返す
- [ ] publish/abort/drop の各経路が型どおり compile する
- [ ] `mise run check` / `mise run lint` が `-D warnings` で通る

## Verification

 - [ ] `mise run check` / `mise run lint` / `mise run test`（io-file 部分が緑）
