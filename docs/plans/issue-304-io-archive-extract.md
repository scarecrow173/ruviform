---
id: "PLAN-kMb7YLTQ28L2txLvTj6dcD"
type: "plan"
status: "approved"
title: "Issue #304 ruviform-io-archive Secure Extraction 実装計画 (IO-IMP-10)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/304"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-304-io-archive-extract.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-304-io-archive-extract.md"
    - "docs/designs/issue-304-io-archive-extract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-304-io-archive-extract.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #304 ruviform-io-archive Secure Extraction 実装計画 (IO-IMP-10) Implementation Plan

## Goal

既存 `crates/ruviform-io-archive` に `extract` module を追加し、
Secure Default の Path Policy（`ArchiveName -> SafeRelativePath`・
canonical 衝突検出・kind/budget 拒否）と、隔離 Staging Root の
排他的作成・handle 相対/No-follow/CreateNew による `ExtractionSession
-> ExtractedTree`・Cleanup/Recovery Handle を実装する。正式公開は
対象外。OS 別 backend（unix dirfd・windows NT handle 相対）は raw
FFI + private adapter で構成し、保証不能 target は `Unsupported`。

## Scope

単一実装ストリーム・1 plan・3 task。変更は
`crates/ruviform-io-archive/` の `src/`・`tests/`・crate README・
ルート `crates/README.md` 台帳・`Cargo.toml`（依存 3 件追加）・
`Cargo.lock`・本 doc チェーンに限定。既存 ZIP/TAR reader の挙動を
変えない。正式公開・Consumer schema・IO-IMP-02 共通化・追加 OS
backend は対象外。

## Upstream Documents

- `docs/specs/issue-304-io-archive-extract.md`（approved）
- Issue #304 本文（受入正本）・Issue #286 IO-04 v0.1/v0.2・
  `docs/specs|designs/issue-302|303`・`docs/specs|designs/issue-295|296`

## Design Inputs

- `docs/designs/issue-304-io-archive-extract.md`（approved —
  型一覧・Path Policy・FS trait・session 状態機械・cleanup の
  identity 照合・OS 別 primitive・脅威境界）

## File Map

- `src/extract.rs` — module root・re-export・脅威モデル doc
- `src/extract/path.rs` — `SafeRelativePath`・component 検証・
  canonical key（NFC+lowercase）
- `src/extract/fs.rs` — `PlatformExtractOps`/`StagingDir`/
  `StagingFile`/`NodeIdentity` trait + factory + `#[cfg(test)]` mock
- `src/extract/unix.rs` — dirfd backend（`openat`/`mkdirat`/`unlinkat`/
  `fstat`/`fstatat` + `O_NOFOLLOW`/`O_DIRECTORY`/`O_CREAT|O_EXCL`）
- `src/extract/windows.rs` — NT handle backend（`NtCreateFile`/
  `NtOpenFile`/`NtDeleteFile`/`NtQueryInformationFile` +
  `RootDirectory`/`OBJ_DONT_REPARSE`/`FILE_OPEN_REPARSE_POINT`/
  `GetFileInformationByHandleEx(FileIdInfo)`）
- `src/extract/session.rs` — `ExtractionSession`/`ExtractedTree`/
  `ExtractOptions`/`ExtractionReceipt`・plan/write/cleanup ロジック
- `src/extract/failure.rs` — `ExtractionFailure`/
  `ExtractionRecoveryHandle`/`ExtractionRecoveryInfo`/
  `RecoveryTreeState`/`CleanupReceipt`/`ExtractionCleanupFailure`
- `src/limits.rs` — `max_path_depth` 追加（modified・最小）
- `src/lib.rs` — `pub mod extract`・crate doc・Send 静的検証（modified）
- `Cargo.toml` — `libc`/`windows-sys`/`unicode-normalization` 追加（modified）
- `tests/common/` — extract 用 fixture 補助（modified・最小）
- `tests/extract_path.rs` / `extract_secure.rs` / `extract_staging.rs`
- `crates/ruviform-io-archive/README.md`・`crates/README.md`（modified）

## Implementation Sequence

- [ ] 1. `src/extract/path.rs`（`SafeRelativePath`・canonical key）+
      `src/extract/fs.rs`（trait・factory・mock）+ `src/extract/`
      module 骨格 + `ArchiveReadLimits::max_path_depth` +
      `tests/extract_path.rs` の否定行列
- [ ] 2. `src/extract/unix.rs` + `src/extract/windows.rs` backend +
      `src/extract/session.rs`/`failure.rs`（session/tree/receipt/
      failure/recovery）+ `tests/extract_staging.rs` の実 FS 試験
      （identity・Drop/cleanup・差し替え）
- [ ] 3. `tests/extract_secure.rs`（end-to-end 否定行列・collision・
      kind/budget・取消・writer error・round-trip）+ crate doc/README・
      `crates/README.md` 台帳・`git merge origin/main`・最終検証・IR

## Dependencies and Risks

- 依存追加 3 件: `libc`（unix backend — IO-IMP-02 と同 backend
  方針）、`windows-sys`（windows backend — 同）、
  `unicode-normalization`（canonical 衝突キーの NFC — `char::to_lowercase`
  で検出不能な normalization 差を拒否に必要）。
- リスク: `NtCreateFile`/`NtDeleteFile`/`OBJ_DONT_REPARSE` の実挙動
  （junction/symlink の leaf 差し替え・`FILE_CREATE` disposition・
  `NtDeleteFile` が link 自体に作用するか）を Windows 実測で pin
  する。`openat` 系の per-component 降下は実績十分だが、FS 非対応
  の dirfd 系があれば `Unsupported` で正直に失敗させる。
- `#[non_exhaustive]` struct への `max_path_depth` 追加は既存
  Consumer の `default()`+mutate 経路に影響しないが、receipt/
  struct literal 直構築をしていないかの差分監査が必要。
- 並行 worker が root `Cargo.toml`・`crates/README.md`・`Cargo.lock`
  を触る可能性 — 最終検証前に `git merge origin/main` で解消。

## Task Breakdown

- T1（`docs/tasks/issue-304-t1-path-policy.md`）: Path Policy・
  canonical 衝突・`max_path_depth`・FS trait/mock 骨格・
  `extract_path` 否定行列。
- T2（`docs/tasks/issue-304-t2-staging-backend.md`）: unix/windows
  backend・session/tree/receipt・cleanup の identity 照合・
  `extract_staging` 実 FS 試験。
- T3（`docs/tasks/issue-304-t3-secure-verify.md`）: `extract_secure`
  end-to-end 試験・crate doc/README・台帳・main 取込・最終検証・IR。

## Verification

- [ ] `mise run fmt` / `mise run fmt-check` / `mise run check` /
      `mise run lint` / `mise run test`（workspace 全体）
- [ ] `extract_path` 否定行列・`extract_secure` の衝突/kind/budget/
      取消/writer error/round-trip・`extract_staging` の identity/
      Drop/差し替え/recovery が tests/ で緑
- [ ] 既存 ZIP/TAR テスト非退行・`default=[]`・`unsafe` 局所許可のみ・
      backend 型非露出・依存 DAG（io-file 等への新規依存なし）の目視監査
- [ ] Required CI（verify: ubuntu+windows）・非必須 verify-macos は
      NotRun/Pending として記録

## Open Questions / Gaps

- `NtDeleteFile` の leaf が symlink/junction のとき link 自体を消すか
  （`OBJ_DONT_REPARSE` で link 自体に作用するか）を Windows 実測で
  確定する。不確実なら cleanup の root remove を `open_dir` + identity
  照合 + `remove_dir` に置き換え、leaf link の削除は file remove に
  限定する。
- `ExtractedTree` の readback API（`open_file` 等）が必要かは本
  Issue の試験には不要なため、Consumer 実需が出てから検討する。

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 worker が実装）
- Independent work streams: none（T1→T2→T3 直列）
- User approval required before dispatch: yes
- If approved: execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- 焦点: Path Policy の拒否規則網羅性（Windows 禁止名・UNC・ADS・
  unicode/casefold）・FS primitive の no-follow/CreateNew が文字列
  検証と別層で機能しているか・cleanup が tree 外削除を絶対にしない
  か（identity 照合 + per-component 降下）・TAR streaming 失敗時の
  `ExtractionFailure` + recovery 責務分離・`unsafe` 局所許可の範囲。
