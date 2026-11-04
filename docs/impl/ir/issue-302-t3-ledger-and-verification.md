---
id: "IMPL-034RTBAAcyzygMEGzkuazL"
type: "impl"
status: "completed"
title: "Issue 302 T3 ledger and verification"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/302"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-302-t3-ledger-and-verification.md"
      - type: "file"
        path: "crates/ruviform-io-archive/README.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
  implements:
    - "docs/tasks/issue-302-t3-ledger-verify.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-302-t1-archive-scaffold-and-zip-reader.md"
    - "docs/impl/ir/issue-302-t2-zip-writer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-302-io-archive-zip.md"
    - "docs/impl/ir/issue-302-t1-archive-scaffold-and-zip-reader.md"
    - "docs/impl/ir/issue-302-t2-zip-writer.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 302 T3 ledger and verification

## Summary

クレート README・`crates/README.md` 索引・workspace `members` 登録・
生成 index 再生成・`Cargo.lock` 再解決と、workspace 全体の最終検証
（fmt / check / lint / test）を行う台帳・検証タスク。

## Context

- 実施タスク: [T3](../../tasks/issue-302-t3-ledger-verify.md)
- 上流: [plan](../../plans/issue-302-io-archive-zip.md)、
  T1/T2 IR（scaffold・reader・writer 実装）

## Implementation

### Changes

- ルート `Cargo.toml` の `members` に `crates/ruviform-io-archive` を
  `ruviform-io-file` 直後へ追加（Issue 番号と後続 #303/#304 の注記付き）。
- `crates/README.md` — 計数 33→34、「汎用ファイル I/O（非 RT 専用）」
  セクションへ `ruviform-io-archive` 行を追加。
- `crates/ruviform-io-archive/README.md` — 責務・脅威境界・backend
  隠蔽・層/許可依存・RT 契約・実装 Issue 一覧を io-file の構成に倣って記述。
- `Cargo.lock` — `cargo` 再解決で `crc32fast`/`flate2`/`miniz_oxide`/
  `adler2`/`simd-adler32`/`cfg-if` を追加（手編集なし）。

### Decisions During Implementation

- index/README 類は `doc_maintenance apply --force-index` で機械再生成
  する方針（手編集による drift を避ける）。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

実測（Windows 11 x64・Git Bash・rustc/cargo 1.99.0 mise pin）:

- `mise run fmt` → exit 0（`cargo fmt --all`）
- `mise run fmt-check` → exit 0（`cargo fmt --all -- --check`）
- `mise run check` → exit 0（`cargo check --workspace --all-targets`、48.6s）
- `mise run lint` → exit 0（`cargo clippy --workspace --all-targets --
  -D warnings`）
- `mise run test` → exit 0（`cargo test --workspace`、130 test binary
  全緑・0 failed。io-archive は `tests/zip_reader.rs` 32 +
  `tests/zip_writer.rs` 10 + doctest 7）
- `git fetch origin` → `git merge origin/main` → Already up to date
  （origin/main = 2ee375d、merge 不要で競合なし）
- 補助（canonical ではない）: `cargo test -p ruviform-io-archive` で
  クレート単位を先行確認済み

## Risks

- 並行 worker（ruv-297/ruv-300）が同時期に新規クレートを追加しており、
  `Cargo.toml`/`crates/README.md`/`Cargo.lock` で merge 競合が予想される
  — 最終検証前の `git merge origin/main` で自クレート追加分を保持して
  解消し、`Cargo.lock` は再解決で再生成する。

## Follow-ups

- merge 後の全検証を経て task docs のチェックボックスを完了化する
- PR 本文の Consumer 影響分類（Synvelis/AudioLens 独立判定）
