---
id: "PLAN-034RBIiZIxCPVB11km1utN"
type: "plan"
status: "approved"
title: "Issue #296 ruviform-io-file 実 OS バックエンド実装計画 (IO-IMP-02)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/296"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-file/src/os.rs"
      - type: "file"
        path: "crates/ruviform-io-file/tests/os_publish.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
      - type: "file"
        path: "crates/ruviform-io-file/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/platform.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/staged.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/mock.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/specs/issue-296-io-file-os-backend.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-296-io-file-os-backend.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-296-io-file-os-backend.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #296 ruviform-io-file 実 OS バックエンド実装計画 (IO-IMP-02) Implementation Plan

## Goal

`PlatformFileOps` trait の実 OS 実装 `OsFileOps`（unix/Windows）を追加し、
`StagedFile` の atomic publish・durability・recovery を実 FS で動作させる。

## Scope

単一実装ストリーム・1 plan・3 task。変更は `crates/ruviform-io-file/`
内部 + `Cargo.toml`（依存）+ `Cargo.lock` + 本 doc チェーンに限定。
公開 API・パイプライン分類・Mock の契約駆動は変更しない。

## Upstream Documents

- `docs/specs/issue-296-io-file-os-backend.md`（approved）
- Issue #296 本文（受入正本）・Issue #283 IO-01 v0.4・Issue #282 統合設計

## Design Inputs

- `docs/designs/issue-296-io-file-os-backend.md`（approved — OS 別
  プリミティブ選択・エラー分類ポリシー・capability matrix・信頼境界）

## File Map

- `crates/ruviform-io-file/src/os.rs` — `OsFileOps` 本体。temp 生成・
  Write/Seek・positioned readback・sync_file・close・publish
  （unix: `syscall(SYS_renameat2)`/`renamex_np`/`link+unlink`、
  windows: `MoveFileExW(0)`/`fs::rename`）・dir sync・cleanup・inspect・
  確定/あいまいエラー分類。`#[expect(unsafe_code)]` 局所例外は syscall
  呼出し fn のみ。
- `crates/ruviform-io-file/src/platform.rs` — `staged_file()` を
  `OsFileOps::create` に接続。trait に `temp_path()` 診断メソッド追加。
- `crates/ruviform-io-file/src/staged.rs` — `create` で
  `RecoveryInfo.temp_path` を記録。既存の「create は Unsupported」
  テストを実装済み仕様に更新。
- `crates/ruviform-io-file/src/mock.rs` — `temp_path()` の Mock 実装。
- `crates/ruviform-io-file/src/lib.rs` — 「backend 未実装」記述を実装済み
  保証範囲の文書に更新（doc test の `no_run` は維持）。
- `crates/ruviform-io-file/Cargo.toml` — `libc`（unix）・`windows-sys`
  （windows・`Win32_Foundation`+`Win32_Storage_FileSystem`）追加。
- `crates/ruviform-io-file/README.md` — capability matrix・信頼境界・
  Windows 共有 Mode・Temp identity・close 限界・依存変更の文書化。
- `crates/ruviform-io-file/tests/os_publish.rs` — 実 FS 結合試験
  （複数プロセス CreateNew 競合・非上書き・durability evidence・
  recovery・誘導可能故障）。
- `Cargo.lock` — 依存追加の反映（`mise run check` で再解決・手編集しない）。

## Implementation Sequence

- [ ] T1: `OsFileOps` 実装 + factory 接続 + trait `temp_path()` + staged/mock
      追従 + 依存追加 + 既存 unit test 更新（コンパイル・`mise run check`
      `lint` が通る状態まで）。
- [ ] T2: `tests/os_publish.rs` 実 FS 試験 + adapter 単体試験（temp 名・
      分類ヘルパ）+ 実機検証（Windows NTFS で `mise run test`）。
- [ ] T3: README/lib.rs rustdoc の保証範囲・capability matrix 文書化 +
      impl record + 最終 `mise run verify-ci`。

## Dependencies and Risks

- `libc 0.2`・`windows-sys 0.61` は workspace 既出版に限定。`Cargo.lock`
  は cargo 再解決で更新。
- Linux/macOS の実機検証は本環境（Windows）では不可 — CI OS matrix
  （`verify` ジョブ `mise run test-ci`）に委譲する旨を検証報告に明記。
- `#[expect(unsafe_code)]` の scope は syscall fn に限定（`lint -D
  warnings` ゲートで未使用 expect は fail するため cfg 毎に正確に置く）。

## Task Breakdown

- `docs/tasks/issue-296-t1-os-adapter.md` — OsFileOps + factory + trait +
  依存 + 既存テスト追従。
- `docs/tasks/issue-296-t2-real-fs-tests.md` — 実 FS 試験・複数プロセス
  競合・adapter 単体試験。
- `docs/tasks/issue-296-t3-docs-verify.md` — rustdoc/README capability
  matrix・impl record・最終検証。

## Verification

- [ ] `mise run fmt` → `mise run check` → `mise run lint` → `mise run test`
      （dev profile・Windows NTFS）。
- [ ] `mise run verify-ci`（fmt-check→check-ci→lint-ci→test-ci・CI 同等）。
- [ ] 複数プロセス CreateNew 競合の実測ログ（成功 1・他 DestinationExists）。
- [ ] 未検証 OS（Linux/macOS）は CI matrix へ委譲と明記。

## Open Questions / Gaps

- 古いカーネル（renameat2 非対応）での link fallback 実測は本環境では
  不可能 — fallback 経路は単体試験可能な分離設計とし、実測は CI/将来環境。
- Windows `ACCESS_DENIED` probe 分類は heuristic — 設計書に記録済み。

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker 自身が実装）。
- Independent work streams: none（単一ストリーム・T1→T2→T3 順次）。
- User approval required before dispatch: yes — spec/design/plan は
  review-protocol で blocker なしとして approved 済み。

## Review Handoff

- 重点: エラー分類ポリシーの確定/あいまい境界・`link+unlink` fallback の
  採用条件・Windows `MoveFileExW(0)` の既存宛先振る舞い・dir sync の
  Windows GENERIC_WRITE 要件。
