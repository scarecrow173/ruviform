---
id: "TSPEC-CeWNpGq9NFmtzCCp3dkysw"
type: "test-spec"
status: "approved"
title: "Issue #411 — cargo_metadata MSRV 修復の検証契約"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/411"
  changes:
    added: []
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-411-msrv-cargo-metadata-pin.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-390-driver-boundary-lab.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-411-msrv-cargo-metadata-pin.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-411-msrv-cargo-metadata-pin.md"
    - "docs/designs/issue-411-msrv-cargo-metadata-pin.md"
  verified-by: []
  references:
    - "docs/test-specs/issue-390-driver-boundary-lab.md"
  defers: []
  deferred-by: []
---

# Issue #411 — cargo_metadata MSRV 修復の検証契約

## Purpose

Issue #411 の修復が「lockfile 一時回避」ではなく「manifest 依存
選定による MSRV 収束の恒久化」であること、および guard の型付き
検査意味が 0.19.2 下でも保持されることを pin する。

## Feature

`ruviform-ui-test` の `cargo_metadata` 通常 dep が、workspace
MSRV 1.89 と通常解決で整合する依存系列へ固定され、Issue #390
driver boundary guard の検査契約が退行しない。

## Rules

- Rule: MSRV toolchain（`mise exec rust@1.89.0`）で
  `ruviform-ui-test` の lib/bin/test target が compile・test
  pass する。MSRV 超過 package（rustc >1.89 要求）を dep tree が
  引かないこと。
- Rule: manifest 制約は `cargo_metadata = "0.19"` — その
  `cargo-platform` 要求（`^0.1.2`）が 0.2/0.3 系へ到達不能で、
  `cargo update` 相当の通常再解決でも platform 0.1.x に収束する。
  Cargo.lock で platform が 0.1.x に解決されること。
- Rule: 検査回避禁止 — MSRV 引上げ・guard/検査範囲/offline 条件/
  semantic test の弱体化・警告抑制・新外部依存・transitive だけを
  抑える直接 dep 追加をしない。
- Rule: `check-ui-driver-boundary` は `--offline` 既定条件のまま
  verdict `pass` を返し、`tests/driver_boundary.rs` の fixture
  正負 semantic が全て pass する。

## Examples

- RED（修復前実測）: `mise run test-ui-test-msrv` →
  `cargo-platform@0.3.3 requires rustc 1.91`・exit 101。
- GREEN（修復後実測）: 同 task で 76 test 全 pass
  （driver_boundary 10 / driver_contract 5 / e2e_process 15 /
  fake_driver 14 / headless_contract 32）。
- lock: `cargo_metadata` 0.23.1 → 0.19.2、`cargo-platform`
  0.3.3 → 0.1.9（`mise run check` による正規再生成）。
- 呼出互換: name 系 `*.to_string()` 12 箇所が clippy
  `implicit_clone` に抵触 → `*.clone()` へ調整（値は同一）。

## Guarantees

- `mise run test-ui-test-msrv` exit 0（Rust 1.89.0）。
- `mise run check-ui-driver-boundary` exit 0（verdict pass、
  dep-graph/signature/allowlist 全 pass）。
- `mise run fmt-check` / `check` / `lint` / `test` /
  `test-ui-driver-contract` / `test-ofx-msrv` exit 0。
- Consumer（Synvelis）側の lock 更新 + `test-msrv` は別 repo
  別作業として root へ引き渡す — 本検証の scope 外として明記。
