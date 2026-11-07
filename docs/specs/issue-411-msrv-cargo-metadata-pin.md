---
id: "SPEC-Y6JWBZ3CHy6qvkcyHzSK5H"
type: "spec"
status: "approved"
title: "Issue #411 — ruviform-ui-test の cargo_metadata 推移依存 MSRV 1.89 不整合を修復する"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/411"
    - "https://github.com/scarecrow173/ruviform/issues/390"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "crates/ruviform-ui-test/README.md"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_boundary.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "https://github.com/scarecrow173/ruviform/issues/390"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-390-driver-contract-lab.md"
    - "docs/designs/issue-390-driver-contract-lab.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/411"
  derived-by:
    - "docs/designs/issue-411-msrv-cargo-metadata-pin.md"
    - "docs/plans/issue-411-msrv-cargo-metadata-pin.md"
    - "docs/test-specs/issue-411-msrv-cargo-metadata-pin.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-411-msrv-cargo-metadata-pin.md"
  references:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/workspace-msrv-1-85-1-89-fearless-simd.md"
  defers: []
  deferred-by: []
---

# Issue #411 — cargo_metadata 推移依存の MSRV 1.89 不整合を修復する

## What Should Be Built

`ruviform-ui-test` が通常 dep として使う `cargo_metadata` の
推移依存 `cargo-platform` が、依存解決で 0.3.3（rustc 1.91 必須）
へ収束し、workspace MSRV 1.89 でのビルド・テストを不可能にする。
Synvelis 側 `mise run test-msrv` で `cargo-platform@0.3.3 requires
rustc 1.91`（exit 101）として観測済み。依存経路は
`ruviform-ui-test` → `cargo_metadata 0.23.x` →
`cargo-platform ^0.3.0`。

manifest の依存選定を、MSRV 1.89 と整合する既存 crate 系列へ
修復し、通常の Consumer 再解決でも対応 MSRV 範囲に収束する状態を
作る。lockfile のみを platform 0.3.2 へ下げる即時回避は再解決で
再発するため恒久修復としない。

## Acceptance Criteria

- `mise exec rust@1.89.0` 相当の MSRV toolchain で
  `ruviform-ui-test` の lib/bin/test target がビルド・テスト
  成功すること（本 PR では `mise run test-ui-test-msrv` 新設タスク
  で実測）。
- `cargo_metadata` manifest 制約が、通常の `cargo update` 相当の
  再解決で `cargo-platform` の MSRV 超過版（0.3.3 系）を選ばない
  こと — すなわち cargo_metadata 自体の `cargo-platform` 要求が
  MSRV 適合系列へ閉じていること。
- Issue #390 の driver boundary guard の型付き metadata parse・
  `--offline` 条件・検査範囲・semantic test（fixture 正負系 +
  live workspace）を弱体化しないこと。
- MSRV 引上げ・guard 除去・警告抑制・新外部依存追加・transitive
  だけを抑えるための直接 dep 追加で回避しないこと。
- RF 正規検証（fmt-check / check / lint / test /
  check-ui-driver-boundary / test-ui-driver-contract）が通ること。
- Consumer 側（Synvelis）の lock 更新と `test-msrv` 再検証は
  別 repo の別作業として root へ引き渡すこと。

## Non-Goals

- `cargo_metadata` 以外の依存の MSRV 棚卸し。
- Synvelis / audio-lens 側の変更・lock 更新・検証（別作業）。
- guard 検査ルール・ruleset・fixture 自体の仕様変更。
- CI workflow・Consumer 影響分類の変更。

## Links

- Design: [issue-411-msrv-cargo-metadata-pin](../designs/issue-411-msrv-cargo-metadata-pin.md)
- Plan: [issue-411-msrv-cargo-metadata-pin](../plans/issue-411-msrv-cargo-metadata-pin.md)
- Test Spec: [issue-411-msrv-cargo-metadata-pin](../test-specs/issue-411-msrv-cargo-metadata-pin.md)
- Task: [issue-411-msrv-cargo-metadata-pin](../tasks/issue-411-msrv-cargo-metadata-pin.md)
