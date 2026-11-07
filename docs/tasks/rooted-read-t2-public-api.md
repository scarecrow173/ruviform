---
id: "TASK-PKzErsdcGHe0VY9E50OmL7"
type: "task"
status: "done"
title: "T2 rooted 公開 API + fd-stack resolve engine + FileErrorKind 拡張"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-file/src/rooted.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/rooted/resolve.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/rooted-read-capability.md"
  implemented-by: []
  depends-on:
    - "docs/plans/rooted-read-capability.md"
    - "docs/tasks/rooted-read-t1-vfs.md"
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

# T2 rooted 公開 API + fd-stack resolve engine + FileErrorKind 拡張

## Work

design（`docs/designs/rooted-read-capability.md`）の公開契約を
`crates/ruviform-io-file/src/rooted.rs` + `rooted/resolve.rs` に
実装する。

- `RootedDir`（root handle・canonical root・`RootedOptions` を
  `Arc<RootShared>` で共有し `open_dir` の scoped dir も同 capability
  内に閉じ込める）/`RelativePath`/`FollowMode`/`RootedOptions`/
  `NodeInfo`/`NodeId`/`NodePin`/`DirEntry`/`RootedFile`。
- `resolve.rs`: component-by-component fd-stack walk（`probe`/
  `open_dir`/`read_link` のみ、path 再解決なし）・絶対 link の
  lexical 正規化+canonical prefix 検査・`..` stack pop・
  hop/depth/total-components budget・`WithinRoot`/`NoFollow` 分岐。
- `open_regular` の leaf 確定: `open_path_handle`（Linux O_PATH /
  Windows `FILE_READ_ATTRIBUTES`）→ kind 検査 → 同一 inode の
  読取 fd → post-open identity 照合。`open_pinned` は `NodePin`
  との照合を追加。
- `error.rs`: `FileErrorKind` に `NotFound`/`NotDirectory`/
  `NotRegularFile`/`SymlinkEncountered`/`EscapesRoot`/
  `LoopDetected`/`IdentityMismatch`/`LimitExceeded` を追加
  （`#[non_exhaustive]` 維持・io error kind 写像を実装）。
- 全公開 enum `#[non_exhaustive]`・doc example は doctest・
  backend 型/feature 名を露出しない。
- scripted mock（T1 で導入）上で swap/escape/loop/limit/失敗分類の
  unit test を先に RED で書き、resolve engine 実装で GREEN にする。

## Verification

- `cargo test -p ruviform-io-file`（mock unit test・doctest）全
  pass。
- `mise run check`・`mise run lint`。
- io-file crate README に `rooted` の責務・保証・依存図を追記。
