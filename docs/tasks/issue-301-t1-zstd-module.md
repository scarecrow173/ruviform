---
id: "TASK-004I6K3yrO0KrqC8a5SB9v"
type: "task"
status: "completed"
title: "Issue #301 T1 — zstd feature/module と stream pump 一般化"
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
    modified:
      - type: "file"
        path: "crates/ruviform-io-compression/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-compression/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/receipt.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/error.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/issue-301-io-compression-zstd.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-301-io-compression-zstd.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #301 T1 — zstd feature/module と stream pump 一般化

## Implementation

`Cargo.toml` に `zstd = ["dep:zstd-safe"]` と optional `zstd-safe`
（`default-features = false`・採用記録コメント）、dev-dep `zstd`
（interop 検証用）を追加。`src/stream.rs` に private `trait OutPump`
（`feed`/`flush_sync`/`finish`）を導入して `EncoderCore<W, P: OutPump>`
に一般化（既存 `DeflateOut` がそのまま実装）。flate2 部を
`any(gzip,deflate)` cfg、`take_exact` を `any(gzip,zstd)` cfg へ分割。
`src/zstd.rs` に `SkippableFramePolicy`（`#[non_exhaustive]`）、
`EncodeOptions`/`DecodeOptions`、借用 `Encoder`/`Decoder`、FHD parser
（<=18B stack buffer）・boundary scan・`DParameter::WindowLogMax` +
header-parse window 強制・backend error 名分類を実装。
`lib.rs`/`receipt.rs`/`error.rs` の cfg gate に `zstd` を追加。

## Risks and Mitigations

| risk | mitigation |
| --- | --- |
| backend error 名の分類ずれ | `get_error_name` を `source()` 保存 + Negative test pin |
| header 二重計上 | 受理 header buffer を `InBuffer` として feed、reader 計上は take_exact 側のみ |
| `max_window_log` 範囲外 | `WINDOWLOG_LIMIT_DEFAULT` 既定・`InvalidInput` で拒否 |

## Verification

`mise exec -- cargo check -p ruviform-io-compression --features zstd` /
`--features gzip,deflate,zstd`。
