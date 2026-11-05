---
id: "PLAN-UecvheIvCz0oE2tUFPmFC0"
type: "plan"
status: "approved"
title: "Issue #303 ruviform-io-archive TAR 実装計画 (IO-IMP-09)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/303"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-303-io-archive-tar.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-303-io-archive-tar.md"
    - "docs/designs/issue-303-io-archive-tar.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-303-io-archive-tar.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #303 ruviform-io-archive TAR 実装計画 (IO-IMP-09) Implementation Plan

## Goal

既存 `crates/ruviform-io-archive` に内部 Parser による TAR Streaming
Reader（Pending/discard 状態機械・USTAR/GNU/PAX 実効値・budget 統合・
終端検証）と明示 finish 型分離の TAR Writer（宣言 size cap・padding・
zero block 終端）を実装し、否定 fixture を含む検証を完遂する。
共通型は `link_name`・`EntryDisposition` の最小拡張のみ。

## Scope

単一実装ストリーム・1 plan・3 task。変更は
`crates/ruviform-io-archive/` 内の `src/`・`tests/`・crate README と
ルート `crates/README.md` 台帳・本 doc チェーンに限定。`Cargo.lock`
は `mise run check` 等の再解決でのみ更新（依存追加なしのため実質
変更なしの見込み）。Secure Extraction・新規依存・他 crate は対象外。

## Upstream Documents

- `docs/specs/issue-303-io-archive-tar.md`（approved）
- Issue #303 本文（受入正本）・Issue #286 IO-04 v0.1/v0.2・Issue #282
  統合設計・Issue #302 確立の共通型

## Design Inputs

- `docs/designs/issue-303-io-archive-tar.md`（approved — 内部 Parser
  選定根拠・状態機械・実効値解決順位・budget 強制点・エラー分類・
  脅威境界）

## File Map

- `crates/ruviform-io-archive/src/tar.rs` — module root・re-export・
  module doc（形式範囲・Pending 契約・脅威境界）
- `src/tar/header.rs` — 非公開 header field 定数・checksum・octal/
  base-256 数値・PAX record parse・header 生成
- `src/tar/read.rs` — `TarArchiveReader`・`TarEntryReader`・
  `Ready/EntryOpen/Pending/EndSeen/Failed` 状態機械・
  `discard_pending`/`discard_remaining`/`finish` drain
- `src/tar/write.rs` — `TarArchiveWriter`・`TarEntryWriter`・
  `TarEntryOptions`・USTAR/PAX `x` 出力・poisoned
- `src/entry.rs` — `ArchiveEntryInfo.link_name` 拡張（modified）
- `src/receipt.rs` — `EntryDisposition` enum・`EntryReadReceipt.
  disposition` 拡張（modified）
- `src/zip/read.rs` — `EntryReadReceipt::new` 呼出しの整合
  （modified・最小）
- `src/lib.rs` — `pub mod tar`・crate doc・静的検証（modified）
- `tests/common/mod.rs` + `tests/common/tar.rs` — TAR byte 級
  fixture builder
- `tests/tar_reader.rs` — 構造/Pending/budget/実効値/否定行列
- `tests/tar_writer.rs` — finish 型分離・poison・cap・round-trip
- `crates/ruviform-io-archive/README.md`・`crates/README.md`（modified）

## Implementation Sequence

- [ ] 1. `src/tar/header.rs`（field 定数・checksum・octal/base-256・
      PAX record parse・header serialize）+ `src/receipt.rs`/
      `src/entry.rs` 共通型拡張 + `src/tar.rs` module root
- [ ] 2. `src/tar/read.rs`（streaming Reader・状態機械・budget・
      discard・finish）+ `tests/common/tar.rs` builder +
      `tests/tar_reader.rs`
- [ ] 3. `src/tar/write.rs`（header 出力・cap・finish/poison・
      PAX `x` 出力）+ `tests/tar_writer.rs`
- [ ] 4. crate doc/README・`crates/README.md` 台帳・`git merge
      origin/main`・最終検証・IR 記録

## Dependencies and Risks

- 新規依存なし（TAR Parser は内部実装。`crc32fast`/`flate2` は ZIP
  経路専用のまま）。`tar.gz` 試験は dev 側で既存 `flate2` の
  GzDecoder を decode 層として使い、TAR finish と decoder 側終端
  検証の独立性を pin する（新規 dev-dep・crate 間依存を作らない）。
- 共通型（`EntryReadReceipt`/`ArchiveEntryInfo`）の `pub(crate)`
  ctor 引数拡張は ZIP 側呼出しの機械的追従のみで、公開 API 互換を
  壊さない。
- リスク: 拡張 Entry（`x`/`g`/`L`/`K`）の pending 状態管理・header
  parse 失敗後の stream 位置主張（Failed 化で一貫させる）・
  `finish` auto-drain と unverified 計上の正確性・PAX record len
  自体の整数 parse（`len` 記述が payload 範囲外を指す入力）。
- 並行 worker が root `Cargo.toml`・`crates/README.md`・`Cargo.lock`
  を触る可能性 — 最終検証前に `git merge origin/main` で解消。

## Task Breakdown

- T1（`docs/tasks/issue-303-t1-tar-reader.md`）: header parse・共通型
  拡張・`TarArchiveReader`/`TarEntryReader`/Pending・reader 側
  fixture + 否定行列テスト。
- T2（`docs/tasks/issue-303-t2-tar-writer.md`）:
  `TarArchiveWriter`/`TarEntryWriter`・cap/finish/poison・PAX `x`
  出力・writer 側テスト + round-trip。
- T3（`docs/tasks/issue-303-t3-ledger-verify.md`）: crate doc/README・
  台帳・main 取込・最終検証（fmt/check/lint/test）・IR 記録。

## Verification

- [ ] `mise run fmt` / `mise run fmt-check` / `mise run check` /
      `mise run lint` / `mise run test`（workspace 全体）
- [ ] test-spec 行列: bad checksum・truncated header・非 octal field・
      padding・終端・後続 byte・typeflag 分類・PAX/GNU/USTAR 実効値・
      各 budget・Pending/discard・Writer cap/poison が tests/ で緑
- [ ] `tar.gz` 2 層 finish 独立性の pin 試験
- [ ] ZIP 側既存テスト非退行・`default=[]`・`unsafe` なし・公開 API
      に backend 型なしの目視監査

## Open Questions / Gaps

- `finish` の auto-discard 済み残 Entry を「unverified」に計上する
  集計規則（verified = `ReadToEnd` のみ）は本 plan で確定。
- `next_entry` の Unsupported/Limit 拒否後は Reader `Failed`（透過
  skip は行わない）— #304 で要件が変わるなら design 改訂とする。

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

- 焦点: 状態機械の Failed/Pending/InvalidState 分離の一貫性・budget
  強制点（拡張 payload alloc 前・累積途中）・`finish` drain の
  unverified 計上・PAX record len 文法の fail-closed・Writer 宣言長
  不足時の poison 必然性。
