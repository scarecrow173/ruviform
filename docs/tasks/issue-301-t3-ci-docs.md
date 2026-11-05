---
id: "TASK-nnP07tqKBoCJML7LH9xWsQ"
type: "task"
status: "completed"
title: "Issue #301 T3 — compression-verify lane matrix・crate 文書"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/301"
  changes:
    added: []
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "ci/impact/map.mjs"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "crates/ruviform-io-compression/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
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

# Issue #301 T3 — compression-verify lane matrix・crate 文書

## Implementation

`mise.toml` の `check-io-compression`/`lint-io-compression`/
`test-io-compression` の feature matrix に `zstd` 単独と
`gzip,deflate,zstd` 全有効を追加（`&&` 連結・lane-owner regex
`^mise run <task>` と `cargo .* -p ruviform-io-compression` 維持）。
`ci/impact/map.mjs` `LANES["compression-verify"].features += "zstd"`、
`.github/workflows/ci.yml` lane コメント更新。crate README（zstd 責務・
SkippableFramePolicy・max_window_log・backend `zstd-safe` + vendored C
`cc` build）・`crates/README.md` 行・crate-level doc 追記。

## Risks and Mitigations

| risk | mitigation |
| --- | --- |
| `cc` toolchain 不在で lane 失敗 | ubuntu-latest は gcc 有。失敗時は CI ログで検出 |
| matrix 肥大で lane 遅延 | 5 組合せ（default/gzip/deflate/zstd/全有効）に限定 |

## Verification

`mise run check-io-compression`/`lint-io-compression`/
`test-io-compression` 全緑 + `ci/impact/tests/classify.test.mjs`。
