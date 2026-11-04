---
id: "TASK-GONrZ5DneYV2H4ryIMA4EK"
type: "task"
status: "done"
title: "T1 クレート scaffold + 共有型 + Gzip/Deflate 両 module (Issue #300)"
created: "2026-11-05"
updated: "2026-11-05"
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
    - "docs/plans/issue-300-io-compression-gzip-deflate.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-300-io-compression-gzip-deflate.md"
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
    - "docs/test-specs/issue-300-io-compression-gzip-deflate.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 クレート scaffold + 共有型 + Gzip/Deflate 両 module (Issue #300)

## Work

`crates/ruviform-io-compression/` を新設する: `Cargo.toml`
（`default=[]`・`gzip`/`deflate` features・optional `flate2` rust_backend
dep・依存理由コメント）、`src/lib.rs`（crate doc + re-export + cfg）、
`src/error.rs`（`CompressionError`/`Kind` + io 変換）、`src/options.rs`
（`DecodeScope`/`EncodeLimits`/`DecodeLimits`）、`src/receipt.rs`
（`EncodeReceipt`/`DecodeReceipt`/`IntegritySummary`）、`src/stream.rs`
（private deflate pump/byte 計測/state）、`src/gzip.rs`（header/trailer
codec + `Encoder`/`Decoder`/`EncodeOptions`/`DecodeOptions`）、
`src/deflate.rs`（同構造の単一 stream 実装）。
ルート `Cargo.toml` `members` に `ruviform-io-file` 直後へ追加、
`crates/README.md` の「汎用ファイル I/O」表へ行追加、`Cargo.lock` を
`mise run check` で再解決。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-300-io-compression-gzip-deflate.md`
- `docs/designs/issue-300-io-compression-gzip-deflate.md`
- `docs/specs/issue-300-io-compression-gzip-deflate.md`

## Dependencies

- 依存: `flate2 1.1.10`（`rust_backend`・optional）
- 先行: IO-03 #285 v0.1/v0.2 合意設計・#295/#296 の io-file 先例

## Affected Files

- `crates/ruviform-io-compression/`（新設一式）
- `Cargo.toml`（members）・`Cargo.lock`・`crates/README.md`

## Done When

 - [ ] `cargo check -p ruviform-io-compression` が default で通る
 - [ ] `--features gzip` / `--features deflate` / `--all-features` で
       全 format module が compile する
 - [ ] `mise run check` / `mise run lint` が `-D warnings` で通る
 - [ ] 公開 API に flate2 型が現れない（`cargo doc` + grep）

## Verification

 - [ ] `mise run check` / `mise run lint`（io-compression 部分が緑）
 - [ ] feature matrix 個別 check が全緑
