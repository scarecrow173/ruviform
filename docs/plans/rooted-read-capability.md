---
id: "PLAN-Y2emYF0VFlVKTxwsGWm4Ev"
type: "plan"
status: "done"
title: "ruviform-io-file Rooted Read Capability 実装計画"
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
  implements:
    - "docs/specs/rooted-read-capability.md"
  implemented-by:
    - "docs/tasks/rooted-read-t1-vfs.md"
    - "docs/tasks/rooted-read-t2-public-api.md"
    - "docs/tasks/rooted-read-t3-tests.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/rooted-read-capability.md"
    - "docs/designs/rooted-read-capability.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rooted-read-capability.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-io-file Rooted Read Capability 実装計画

## Goal

`ruviform-io-file` に read-only rooted capability（`rooted`）を
追加し、handle/fd 相対 traversal・`WithinRoot`/`NoFollow`・
identity pin により SV85/PR180 の root 外 bytes/facts 露出を
解消する。OS ルールは `#[doc(hidden)]` 共有層 `vfs` に集約し、
`ruviform-io-archive` extract backend も同層を consume する。

## Scope

- `crates/ruviform-io-file`:`vfs`（unix/windows/unsupported +
  `#[cfg(test)]` scripted mock）・`rooted`・`FileErrorKind` 拡張・
  `windows-sys` feature 拡張・crate README・doctest。
- `crates/ruviform-io-archive`:`extract/fs.rs` を vfs adapter 化・
  `extract/{unix,windows}.rs` 削除・`Cargo.toml` を
  `ruviform-io-file` 依存へ切替・crate README。
- `Cargo.lock` 再生成・`crates/README.md` 台帳・本 doc chain・
  `docs/designs/issue-304-io-archive-extract.md` への共通化済み
  追記。

## Upstream Documents

- `docs/specs/rooted-read-capability.md`
- `docs/designs/rooted-read-capability.md`
- `docs/adr/rooted-read-capability-placement.md`
- `docs/test-specs/rooted-read-capability.md`

## Tasks

| # | task doc | 内容 | 検証 |
| --- | --- | --- | --- |
| T1 | `docs/tasks/rooted-read-t1-vfs.md` | vfs 層を io-file へ抽出（`VfsOps`/`VfsDir`/`VfsProbe`/`NodeIdentity`/`OpenFlags`・unix/windows/unsupported・scripted mock）し、archive `extract/fs.rs` を adapter 化して `extract/{unix,windows}.rs` を削除。依存切替 | `cargo test -p ruviform-io-archive` 既存 suite 全 pass・`mise run lint` |
| T2 | `docs/tasks/rooted-read-t2-public-api.md` | `rooted` 公開 API（`RootedDir`/`RelativePath`/`FollowMode`/`RootedOptions`/`NodeInfo`/`NodeId`/`NodePin`/`DirEntry`/`RootedFile`）+ fd-stack resolve engine + `FileErrorKind` 拡張。scripted mock で swap/escape/loop/limit の RED→GREEN | mock unit test 全 pass・doctest・`mise run check/lint` |
| T3 | `docs/tasks/rooted-read-t3-tests.md` | 実 FS 回帰（leaf/intermediate swap・within-root link・escape/loop/limit/nonregular・windows reparse・失敗分類）+ README/index/lockfile + 全体検証 | `cargo test -p ruviform-io-file`・`mise run fmt-check/check/lint/test` |

## Sequence / Gates

1. T1 → archive 非回帰 green が gate。
2. T2 → mock RED（公開 API 骨格で失敗確認）→ GREEN、doctest 記述。
3. T3 → 実 FS・全体 lane・未実行 OS の正直な扱いを記録。
4. review-protocol（独立 A/B → Validator → Judge）は design doc に
   適用済みであることを確認し、実装 diff に対しても同一手順で
   blocking finding が無いことを確認してから push。
