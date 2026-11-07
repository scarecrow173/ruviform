---
id: "TASK-hupw1aLckqDaLNAlnDrTMF"
type: "task"
status: "done"
title: "Issue #411 — cargo_metadata 0.19 系への依存選定修復 + MSRV 検証入口"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/411"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-411-msrv-cargo-metadata-pin.md"
      - type: "file"
        path: "docs/designs/issue-411-msrv-cargo-metadata-pin.md"
      - type: "file"
        path: "docs/plans/issue-411-msrv-cargo-metadata-pin.md"
      - type: "file"
        path: "docs/test-specs/issue-411-msrv-cargo-metadata-pin.md"
      - type: "file"
        path: "docs/impl/ir/issue-411-msrv-cargo-metadata-pin.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_boundary.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/README.md"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-411-msrv-cargo-metadata-pin.md"
  implemented-by:
    - "docs/impl/ir/issue-411-msrv-cargo-metadata-pin.md"
  depends-on:
    - "docs/plans/issue-411-msrv-cargo-metadata-pin.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-411-msrv-cargo-metadata-pin.md"
    - "docs/designs/issue-411-msrv-cargo-metadata-pin.md"
    - "docs/test-specs/issue-411-msrv-cargo-metadata-pin.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-411-msrv-cargo-metadata-pin.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/390"
  defers: []
  deferred-by: []
---

# Issue #411 — cargo_metadata 0.19 系への依存選定修復 + MSRV 検証入口

## Work

design 確定どおり `cargo_metadata = "0.19"` へ切替、`mise.toml` に
`test-ui-test-msrv` を追加し、RED → 修復 → GREEN → 正規検証 →
独立 review → PR までを一貫して行う。

1. `test-ui-test-msrv` を mise.toml へ追加し RED を実測
   （`cargo-platform@0.3.3 requires rustc 1.91` exit 101 → 済）。
2. `cargo_metadata = "0.19"` へ変更、根拠を Cargo.toml コメントと
   design に記録。`mise run check` で Cargo.lock 正規再生成
   （platform 0.1.9 / cargo_metadata 0.19.2 → 済）。
3. 0.19.2 での呼出互換差異を最小調整（`.to_string()` → `.clone()`
   12 箇所、値は同一 → 済）。
4. `mise run test-ui-test-msrv` GREEN、および正規検証一式
   （fmt-check / check / lint / test / check-ui-driver-boundary /
   test-ui-driver-contract / test-ofx-msrv → 全 pass 済）。
5. doc 更新（development-environment.md opt-in 節、crate README
   stale 依存記述 → 済）。
6. main 取込・lock 再生成・独立 review・最終検証・commit/push/PR。
