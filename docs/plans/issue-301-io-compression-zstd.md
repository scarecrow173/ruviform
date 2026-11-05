---
id: "PLAN-41sS0eh4UmdvUPmw1c99BS"
type: "plan"
status: "approved"
title: "Issue #301 ruviform-io-compression Zstd Frame 実装計画 (IO-IMP-07)"
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
    modified:
      - type: "file"
        path: "crates/ruviform-io-compression/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-compression/README.md"
      - type: "file"
        path: "crates/ruviform-io-compression/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/receipt.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/encoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/decoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/api_surface.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/common/mod.rs"
      - type: "file"
        path: "Cargo.lock"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-301-io-compression-zstd.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-300-io-compression-gzip-deflate.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-301-io-compression-zstd.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #301 ruviform-io-compression Zstd Frame 実装計画 (IO-IMP-07)

## Scope Split

承認済み design（`docs/designs/issue-301-io-compression-zstd.md`）を
3 task に分割。各 task は独立して compile/test 可能な整合単位。

## File Map

| task | files |
| --- | --- |
| T1 | `Cargo.toml`、`src/lib.rs`、`src/stream.rs`、`src/zstd.rs`、`src/receipt.rs`、`src/error.rs`、`Cargo.lock`（生成） |
| T2 | `tests/zstd_format.rs`、`tests/common/mod.rs`、`tests/encoder_contract.rs`、`tests/decoder_contract.rs`、`tests/api_surface.rs` |
| T3 | `mise.toml`、`ci/impact/map.mjs`、`.github/workflows/ci.yml`（コメント）、`crates/README.md`、crate `README.md` |

## Dependency Graph

```text
T1 (backend dep + module + pump 一般化) -> T2 (zstd tests + 共有契約拡張)
                                      -> T3 (CI matrix + docs)  [T1 と並行可能だが matrix 緑は T1/T2 後]
```

## Ordered Tasks

### T1 — zstd module + backend pump

- `Cargo.toml`: `zstd = ["dep:zstd-safe"]`、`zstd-safe` optional dep
  （`default-features=false`・採用記録コメント）、dev-dep `zstd`。
- `src/stream.rs`: `OutPump` trait + `EncoderCore<W,P>` 一般化、
  flate2 部を `any(gzip,deflate)` cfg、`take_exact` を `any(gzip,zstd)`、
  `ZstdOut`/`ZstdIn` pump 追加（`#[cfg(feature="zstd")]`）。
- `src/zstd.rs`: options・`SkippableFramePolicy`・`Encoder`/`Decoder`・
  FHD parser・boundary scan・error 分類。
- `src/lib.rs`/`receipt.rs`/`error.rs` の cfg gate 拡張。
- 検証: `cargo check -p ruviform-io-compression --features zstd`。

### T2 — tests

- `tests/zstd_format.rs`: golden round-trip・single/concatenated・
  skippable accept/reject・checksum 混在/破損・window 超過・
  corrupt/truncated/trailing/wrong magic・reserved bit・DID・
  budget 厳密・消費 byte/prefetch 境界・`zstd` crate 相互検証。
- `encoder_contract.rs`/`decoder_contract.rs` に `#[cfg(feature="zstd")]`
  mod 追加（finish 冪等・Drop・Failed latch・NotExhausted・budget）。
- `api_surface.rs` に zstd の Send bound・`common/mod.rs` の cfg 拡張。
- 検証: `cargo test -p ruviform-io-compression --features zstd` /
  `--features gzip,deflate,zstd`。

### T3 — CI matrix + docs

- `mise.toml` 3 task の matrix に `zstd`・`gzip,deflate,zstd` を追加
  （`&&` 連結・lane-owner regex 互換を維持）。
- `ci/impact/map.mjs` features、`ci.yml` lane コメント更新。
- crate README（zstd 責務・skippable policy・window limit・backend
  表）・`crates/README.md` 行・crate-level doc。
- 検証: `mise run check-io-compression`/`lint-io-compression`/
  `test-io-compression` 全緑。

## Risk Register

| risk | 対応 |
| --- | --- |
| `cc` C toolchain が lane/ローカルで欠如 | ubuntu lane は gcc 有・Windows は MSVC。`zstd` は非 default feature のため default lane に波及しない。失敗時は PR で検出 |
| backend error name 文字列の分類ずれ | `source()` 保存 + Negative test pin。分類不能は `CorruptData` |
| `zstd-safe` 8.2.0 が新しすぎる | 2026-10-16 公開（minimum_release_age 7d 超過） |
| feature 組合せ dead code | matrix の `-D warnings` で各組合せを検証 |

## Verification Matrix

| 検証 | task | 実施 |
| --- | --- | --- |
| feature matrix check/lint/test | T1-T3 | `mise run {check,lint,test}-io-compression` |
| Issue Acceptance 否定試験 | T2 | `tests/zstd_format.rs` |
| workspace fmt/check/lint/test | 全体 | `mise run fmt/check/lint/test` |
| `zstd_safe` 非公開 | T2 | api_surface + cargo doc 目視 |

## Rollback

feature 単位で独立 — `zstd` feature のみの追加のため、revert は
feature/dep/test の除去で完結（既存 gzip/deflate に影響しない）。
