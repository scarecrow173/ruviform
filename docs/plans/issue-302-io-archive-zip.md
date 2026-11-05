---
id: "PLAN-034RT7NR5zMcrEqmbWbWy3"
type: "plan"
status: "approved"
title: "Issue #302 ruviform-io-archive 実装計画 (IO-IMP-08)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/302"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-302-io-archive-zip.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-302-io-archive-zip.md"
    - "docs/designs/issue-302-io-archive-zip.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-302-io-archive-zip.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #302 ruviform-io-archive 実装計画 (IO-IMP-08) Implementation Plan

## Goal

新クレート `ruviform-io-archive` に Archive 共通公開型
（Error/EntryInfo/ArchiveName/EntryKind/Limits/Receipt）と、内部
Parser による ZIP Indexed Reader（Budget 統合・排他借用 Entry Reader・
`verify_all`）および明示 finish 型分離の ZIP Writer（Stored/Deflate）を
実装し、否定 fixture を含む検証を完遂する。

## Scope

単一実装ストリーム・1 plan・3 task。変更は
`crates/ruviform-io-archive/` 新設 + 共有台帳（root `Cargo.toml`
members・`Cargo.lock`・`crates/README.md`・本 doc チェーン）に限定。
TAR・Secure Extraction・ZIP64・StagedFile 合成は対象外。

## Upstream Documents

- `docs/specs/issue-302-io-archive-zip.md`（approved）
- Issue #302 本文（受入正本）・Issue #286 IO-04 v0.1/v0.2・Issue #282
  統合設計

## Design Inputs

- `docs/designs/issue-302-io-archive-zip.md`（approved — 内部 Parser
  選定根拠・Budget 強制点・状態機械・エラー分類・脅威境界）

## File Map

- `crates/ruviform-io-archive/Cargo.toml` — workspace メタ +
  `crc32fast` + `flate2`(rust_backend) + `[lints] workspace`
- `crates/ruviform-io-archive/README.md` — 責務・保証境界・脅威モデル
  方針・依存 backend の隠蔽宣言
- `crates/ruviform-io-archive/src/lib.rs` — crate doc・re-export・
  静的検証（non-RT 明記・Send 検証）
- `src/error.rs` — `ArchiveError`/`ArchiveErrorKind`
- `src/entry.rs` — `ArchiveName`/`EntryKind`/`ArchiveEntryInfo`
- `src/limits.rs` — `ArchiveReadLimits`
- `src/receipt.rs` — `ArchiveReadReceipt`/`EntryReadReceipt`/
  `EntryChecksum`
- `src/zip.rs` — module root・`ZipCompressionMethod`/`ZipEntryOptions`/
  re-export
- `src/zip/record.rs` — 非公開 Record/LE helper
- `src/zip/read.rs` — `ZipArchiveReader`/`ZipEntryReader`/`verify_all`
- `src/zip/write.rs` — `ZipArchiveWriter`/`ZipEntryWriter`
- `tests/zip_reader.rs` — Index/Drop/再 Open/verify_all・構造・Budget・
  否定 fixture
- `tests/zip_writer.rs` — finish 型分離・非 Completed・round-trip
- 共有台帳: `Cargo.toml`（members）・`Cargo.lock`・`crates/README.md`

## Implementation Sequence

- [ ] 1. クレート scaffold（Cargo.toml/README/lib.rs 骨格）+ workspace
      members 登録 + `cargo add crc32fast flate2 --no-default-features
      --features rust_backend`（flate2 側）
- [ ] 2. `error.rs`/`entry.rs`/`limits.rs`/`receipt.rs` の共通公開型
- [ ] 3. `zip/record.rs` + `zip/read.rs`（EOCD/CD parse・Index・
      ZipEntryReader・verify_all）
- [ ] 4. `zip/write.rs`（Local Header patch-back・CD/EOCD 出力・
      Poisoned 状態）
- [ ] 5. tests/ の fixture builder + 否定行列テスト
- [ ] 6. 共有台帳更新（crates/README.md）・`git merge origin/main`・
      最終検証・IR 記録

## Dependencies and Risks

- 新規依存 `crc32fast`・`flate2`(rust_backend → `miniz_oxide`/`adler2`)は
  いずれも MIT OR Apache-2.0・pure Rust・保守活発。Backend 型の公開
  非露出を rustdoc/公開面で確認する。
- 並行 worker（ruv-297 ruviform-io-structured / ruv-300
  ruviform-io-compression）が同時期に root `Cargo.toml`・
  `crates/README.md`・`Cargo.lock` を変更するため、最終検証前に
  `git merge origin/main` で自分の crate 追加分を保持しつつ解消する。
- リスク: Deflate の streaming 経路で実測 Byte が宣言値を超えた場合の
  即時遮断（read() の cap + decoder の buffered tail の扱い）。
  patch-back seek の失敗分類（Io vs InvalidState）。

## Task Breakdown

- T1（`docs/tasks/issue-302-t1-scaffold-reader.md`）: scaffold + 共通型 +
  ZIP Reader（Index/Entry/finish/verify_all）+ Reader 側テスト。
- T2（`docs/tasks/issue-302-t2-zip-writer.md`）: ZIP Writer + finish
  型分離 + 否定 fixture（Writer 起因の不正生成）+ round-trip。
- T3（`docs/tasks/issue-302-t3-ledger-verify.md`）: 共有台帳・main 取込・
  最終検証（fmt/check/lint/test）・IR 記録。

## Verification

- [ ] `mise run fmt` / `mise run fmt-check` / `mise run check` /
      `mise run lint` / `mise run test`（workspace 全体・新クレート込み）
- [ ] test-spec 行列: CRC 不整合・truncated・Local/CD 矛盾・overlap・
      Entry 数/大 Header/展開 Budget・Unsupported 分類が tests/ で緑
- [ ] 途中 Drop→別 Entry Open/再 Open・receipt 正確性・Writer 非
      Completed 化の pin 試験
- [ ] `default=[]`・`unsafe` なし・公開 API に backend 型なしの目視監査

## Open Questions / Gaps

- ZIP64 採否・data descriptor 出力・Secure Extraction との属性受け渡しは
  #303/#304/後続 Issue の確定事項 — 本 plan は `Unsupported` 明示拒否で
  境界を置く。
- `verify_all` が `max_total_uncompressed_bytes` 超過時に返す分類は
  `LimitExceeded`（途中までの実測値は Err 内 message にのみ記録し、
  partial receipt 型は作らない）。

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 worker が実装）
- Independent work streams: none（T1→T2→T3 直列）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 焦点: 内部 Parser による Budget 強制点の網羅性（pre/mid-parse）、
  Deflate 経路の実測遮断、Writer patch-back の失敗分類、否定 fixture が
  期待 kind に過剰適合していないか。
