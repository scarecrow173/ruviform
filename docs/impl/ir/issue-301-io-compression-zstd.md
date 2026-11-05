---
id: "IMPL-st6t53cVR6Tr904Vm2Obxj"
type: "impl"
status: "completed"
title: "Issue 301 ruviform-io-compression zstd feature/module（IO-IMP-07）"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/301"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-compression/src/zstd.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/zstd_format.rs"
      - type: "file"
        path: "docs/specs/issue-301-io-compression-zstd.md"
      - type: "file"
        path: "docs/designs/issue-301-io-compression-zstd.md"
      - type: "file"
        path: "docs/plans/issue-301-io-compression-zstd.md"
      - type: "file"
        path: "docs/tasks/issue-301-t1-zstd-module.md"
      - type: "file"
        path: "docs/tasks/issue-301-t2-zstd-tests.md"
      - type: "file"
        path: "docs/tasks/issue-301-t3-ci-docs.md"
      - type: "file"
        path: "docs/test-specs/issue-301-io-compression-zstd.md"
      - type: "file"
        path: "docs/impl/ir/issue-301-io-compression-zstd.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-compression/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-compression/README.md"
      - type: "file"
        path: "crates/ruviform-io-compression/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/receipt.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/gzip.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/deflate.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/api_surface.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/decoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/encoder_contract.rs"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "ci/impact/map.mjs"
      - type: "file"
        path: ".github/workflows/ci.yml"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
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
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/tasks/issue-301-t1-zstd-module.md"
    - "docs/tasks/issue-301-t2-zstd-tests.md"
    - "docs/tasks/issue-301-t3-ci-docs.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-300-ruviform-io-compression-gzip-raw-deflate-compression-core.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-301-io-compression-zstd.md"
    - "docs/designs/issue-301-io-compression-zstd.md"
    - "docs/specs/issue-301-io-compression-zstd.md"
    - "docs/test-specs/issue-301-io-compression-zstd.md"
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

# Issue 301 ruviform-io-compression zstd feature/module

## Summary

既存 `ruviform-io-compression`（IO-IMP-06 scaffold）に opt-in `zstd`
feature を追加し、RFC 8878 frame の借用 `Encoder`/`Decoder`・明示的で
冪等な `finish`・VerifiedEnd・累積 budget・backend 隠蔽を gzip/deflate
と同一契約で実装した。frame header parse・window 上限・skippable
frame policy・checksum 有無の混在計上はクレート側で所有する。

## Context

- 上流: [spec](../../specs/issue-301-io-compression-zstd.md)・
  [design](../../designs/issue-301-io-compression-zstd.md)・
  [plan](../../plans/issue-301-io-compression-zstd.md)
- 実施タスク: [T1](../../tasks/issue-301-t1-zstd-module.md) /
  [T2](../../tasks/issue-301-t2-zstd-tests.md) /
  [T3](../../tasks/issue-301-t3-ci-docs.md)
- 前提 IR: [Issue #300](issue-300-ruviform-io-compression-gzip-raw-deflate-compression-core.md)
  — 共有語彙・`stream.rs` machinery・contract test 構造を再利用

## Implementation

### Changes

- `Cargo.toml`: `zstd = ["dep:zstd-safe"]` feature・optional `zstd-safe
  8.2.0`（`default-features = false`・BSD-3-Clause・rust-version 1.64 ≤
  MSRV 1.89・vendored C を `cc` で build、cmake 不要）を追加。dev-dep に
  `zstd 0.14`（wire-format interop 参照実装）を追加。依存追加記録を
  comment で同梱。
- `src/stream.rs`: 出力側を private `OutPump` trait に一般化し
  `EncoderCore<'w, W, P>` を導入（`DeflateOut` を実装化・呼び出し側を
  更新）。`ZstdOut`（`CCtx`・`compress_stream2`・`ZSTD_e_flush`/`e_end`
  drain）と `ZstdIn`（`DCtx`・`decompress_stream`・`InBuffer.pos` で
  byte-exact 消費・`Ok(0)` で frame end・`DParameter::WindowLogMax`）を
  追加。zstd backend error を正規化 `CompressionErrorKind` へ分類し
  backend 名は type-erased `source()` のみに保持。
- `src/zstd.rs`: format 別 `EncodeOptions`/`DecodeOptions`/
  `SkippableFramePolicy` と借用 `Encoder`/`Decoder`。magic・FHD
  reserved bit・`Window_Descriptor`・`Dictionary_ID`（`Unsupported`）・
  `Frame_Content_Size` を backend 到達前に parse して拒否/上限強制。
  `WholeInput` 連結 frame・`SingleUnit` 単一 frame・skippable skip/
  reject・checksum 有無の `IntegritySummary` 分離・`TrailingData`/
  `TruncatedInput`/`CorruptData`/`ChecksumMismatch`/`LimitExceeded`
  の正規化分類を実装。`compressed_bytes_consumed` は論理消費量で
  `BufRead` prefetch と分離。
- `src/lib.rs`/`error.rs`/`receipt.rs`: `zstd` cfg gate を共有面へ追加。
- tests: `tests/zstd_format.rs`（34 件・Acceptance の否定試験を網羅）
  + contract/API suite を 3 feature で共有化。`zstd` dev-dep で参照
  frame の decode と自前 encode 出力の参照 decode を相互検証。
- `mise.toml`: check/lint/test-io-compression matrix に `--features zstd`
  と `--features gzip,deflate,zstd` を追加。`ci/impact/map.mjs` の
  `features` に `zstd`。`.github/workflows/ci.yml` の lane comment 更新。
- crate README・`crates/README.md`: zstd 責務・許可依存・feature 一覧を
  追記。

### Decisions During Implementation

- backend は `zstd-safe` 低層 API を採用: `decompress_stream` が frame
  end で消費を止め `InBuffer.pos` で byte-exact な消費量を返すため、
  呼び出し側 `BufRead` の prefetch と receipt 会計を分離できる。高層
  `zstd` crate の `Decoder`/`Encoder` adapter は内部 buffering と
  Drop-write があり、frame 境界・`SingleUnit` の後続 byte 保持・
  「Drop は書かない」契約を保証できないため不採用。
- decode pump は「1 回の `decompress_stream` が produced bytes と
  frame end を同時に返す」ケースがあり、produce を返す前に frame end
  を latch する必要があった（初期実装は end を落として次回 read が
  `TruncatedInput` になる不具合を test で捕捉・修正）。
- skippable frame は data unit に計上しないが、その byte は圧縮入力
  budget には計上する（境界消費は確かに発生するため）。
- 生成 index は `doc_maintenance.js apply` で機械再生成（merge で
  #302 側と両側変更衝突したため再生成で解決）。

## Related Experiments

### Adopted

- `zstd-safe` `compress_stream2`/`decompress_stream` ベースの低層
  adapter — boundary・receipt 精度を維持したまま backend を隠蔽できた。

### Rejected

- `ruzstd`（純 Rust decoder）— byte-exact な frame 境界消費量の取得
  API がなく、receipt 契約を満たせない。
- 高層 `zstd` crate の stream adapter — Drop-write・内部 buffer による
  前方消費で IO-03 の境界契約を破るため dev-dep（参照実装）に限定。

## Validation

実測（Windows 11 x64・Git Bash・rustc/cargo 1.99.0 mise pin・MSVC
toolchain + cc）:

- `mise run fmt` / `fmt-check` → exit 0
- `mise run check-io-compression` → exit 0（default・gzip・deflate・
  zstd・gzip,deflate,zstd の 5 組合せで `cargo check --all-targets`）
- `mise run lint-io-compression` → exit 0（同 matrix で
  `cargo clippy --all-targets -- -D warnings`）
- `mise run test-io-compression` → exit 0（同 matrix・全 test binary
  0 failed; zstd で 34 format tests + contract/API 共有 suite + doctest）
- `mise run verify-ci` → exit 0（`fmt-check` → `lint-ci`（workspace
  clippy `-D warnings`）→ `test-ci`（workspace test・71+ test binary
  全緑）を main 取込後に再実施）
- `mise run test-ci-impact` → exit 0（52/52 — lane owner 導出・
  gate verdict 契約）
- `git merge origin/main`（85ccda4・Issue #302 取込）→ 生成 index の
  み両側追加で衝突、`doc_maintenance.js apply` で再生成解決・
  `Cargo.lock` は auto-merge + cargo 再解決で整合

未検証（実機差分が残り得る項目として記録）:

- ubuntu-latest CI 上での `zstd-sys` C build は本ローカル（MSVC + cc）
  での成功のみを根拠とする — linux/gcc では vendored C が同一手順で
  build される見込みだが、最終的な証明は `compression-verify` lane の
  Required check で行う。

## Risks

- `zstd-sys` の vendored C build はネイティブ toolchain を要求する
  （CI ubuntu-latest/gcc・ローカル MSVC/clang では実績あり）。
- skippable frame の無限連結は圧縮入力 budget でのみ制限される —
  データ frame を伴わない巨大 stream は bytes limit 未到達なら
  受理される（policy として accept した場合。設計上の意図どおり）。

## Follow-ups

- PR 作成後の Required check watch と consumer 影響分類の記録。
- pre-publish license review（`zstd-safe`/`zstd-sys` BSD-3-Clause）は
  workspace licensing ADR の確定に従うフォローアップ。
