---
id: "TASK-2v8sucBEWaF0frYXFmed85"
type: "task"
status: "done"
title: "T2 AC1-AC6 試験群 + rustdoc/crate README (Issue #300)"
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
    - "docs/tasks/issue-300-t1-crate-core.md"
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

# T2 AC1-AC6 試験群 + rustdoc/crate README (Issue #300)

## Work

`crates/ruviform-io-compression/tests/` 以下に試験を実装し、crate
README と rustdoc を保証範囲通りに充実させる:

- `encoder_contract.rs`（AC1）: finish 冪等、partial write 全体受理、
  flush() sync flush + 下流伝搬、Drop が書込しない、Failed 後の
  write/finish が `InvalidState`、借用 writer の scope 後回復。
- `decoder_contract.rs`（AC3）: `finish()` が未読出力で `NotExhausted`、
  非空 buffer の `Ok(0)` のみ VerifiedEnd、0-len read は状態を変えない、
  `compressed_bytes_consumed` が論理消費と SingleUnit 残 byte の一致、
  caller BufRead prefetch との区別（CountingReader で実測）。
- `format_cases.rs`（AC2）: gzip golden（hand-rolled header/body/trailer）
  ・連結 member・corrupt trailer（crc/isize）・truncated（header/body/
  trailer 各段階）・trailing byte・FNAME/FCOMMENT/FEXTRA/FHCRC・
  raw deflate の checksum なし scope 範囲。
- `composition.rs`（AC6）: TAR 様 byte 列を `gzip::Encoder<&mut Vec<u8>>`
  で書き `Decoder` で復号、finish 順序維持。
- `limits` 試験（AC4）を decoder_contract/format_cases に織り込む:
  上限ちょうど vs +1 byte の厳密区別、`max_compressed`/`max_units`
  累積、`EncodeLimits` 出力 cap。
- rustdoc: `lib.rs` + 各 pub 型に doctest（公開 API の例は doctest に
  書く規約）。crate README: 責務・層/依存・feature 表・finish/Drop・
  Budget・Backend 隠蔽・保証範囲。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-300-io-compression-gzip-deflate.md`
- `docs/designs/issue-300-io-compression-gzip-deflate.md`
- `docs/test-specs/issue-300-io-compression-gzip-deflate.md`

## Dependencies

- 依存: T1 のクレート本体
- fixture: 非メディア byte fixture（in-test で生成。binary asset は使わない）

## Affected Files

- `crates/ruviform-io-compression/tests/*`（新設）
- `crates/ruviform-io-compression/src/lib.rs`・`src/*.rs`（rustdoc）
- `crates/ruviform-io-compression/README.md`

## Done When

 - [ ] AC1-AC6 の全受入項目が test 名として存在し緑
 - [ ] `cargo test -p ruviform-io-compression --all-features` が緑
 - [ ] `mise run test`（workspace）で feature-gated 試験が無効化されず、
       `cargo test` 個別 lane で全 format が緑

## Verification

 - [ ] `mise run test` + `cargo test -p ruviform-io-compression
       --all-features` が全緑
 - [ ] feature 個別の `cargo check` が緑（T1 と併せ）
