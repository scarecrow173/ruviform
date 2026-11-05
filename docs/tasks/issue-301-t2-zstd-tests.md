---
id: "TASK-AsjkeCFskQSQZMsB5Tgl5H"
type: "task"
status: "completed"
title: "Issue #301 T2 — zstd format/contract/API 試験"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/301"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-compression/tests/zstd_format.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-compression/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/encoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/decoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/api_surface.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/issue-301-io-compression-zstd.md"
    - "docs/test-specs/issue-301-io-compression-zstd.md"
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

# Issue #301 T2 — zstd format/contract/API 試験

## Implementation

`tests/zstd_format.rs`（`--cfg feature="zstd"`）に golden round-trip・
single/concatenated frame・skippable accept（unit 非計上）/reject
（`Unsupported`）・checksum 混在（`IntegritySummary`）・checksum 破損
（`ChecksumMismatch`）・window 超過（`LimitExceeded`）・corrupt body
（`CorruptData`）・truncated・trailing・wrong magic（`InvalidFormat`）・
reserved FHD bit・DID≠0（`Unsupported`）・budget 厳密（ちょうど/+1）・
大量小 frame 累積・`compressed_bytes_consumed` 論理消費・
`zstd` crate encode/decode interop を実装。`encoder_contract.rs`/
`decoder_contract.rs` に `#[cfg(feature="zstd")] mod` を追加し共通契約
（冪等 finish・Drop・Failed latch・NotExhausted・AtOutputLimit 相当）を
pin。`api_surface.rs` に zstd 公開型の Send bound・`common/mod.rs`
`encode_*` helper の cfg 拡張。

## Risks and Mitigations

| risk | mitigation |
| --- | --- |
| dev-dep `zstd` crate が matrix を遅くする | test のみで compile・非 default |
| 試験 helper が zstd byte を手組 | `zstd` dev-dep で frame 生成・必要最小の byte 加工のみ手組 |

## Verification

`mise run test-io-compression`（feature matrix 全緑）。
