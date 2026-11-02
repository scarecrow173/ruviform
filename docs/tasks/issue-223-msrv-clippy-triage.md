---
id: "TASK-034QsXMp7VDgLZ2W5FaZxq"
type: "task"
status: "done"
title: "Issue #223 MSRV 1.89 限定 pedantic clippy 失敗の整理"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/223"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #223 MSRV 1.89 限定 pedantic clippy 失敗の整理

## Work

Issue #223 の実測に基づき、MSRV Rust 1.89 上でのみ発火する既存 pedantic clippy 失敗を分類・整理する。Owner 方針（lint gate は pin toolchain 1.99.0 のみ、両 version で同一 lint 結果を要求しない）を前提に、各失敗が実契約違反か toolchain 版差かを個別判定し、意味のあるものはコード修正、単純な版差は MSRV 側ポリシー（lint 除外対象の明文化）で扱う。

## Classification

- `normal-plan-task`

## Upstream Documents

- [Issue #223](https://github.com/scarecrow173/ruviform/issues/223) — 追跡 Issue・実測リストの正本

## Dependencies

- なし（#199/#201 の Gate ブロッカーではない独立した追跡 Issue）

## Affected Files

- `crates/ruviform-numeric/src/linalg.rs` — `invert` の `needless_range_loop` expect 解消
- `crates/ruviform-math/src/{simd_poc,dsp}.rs`・`crates/ruviform-math/tests/simd_poc/*.rs` — 1.89 で unfulfilled となる `suboptimal_flops` expect
- `crates/ruviform-plugin-core/src/event.rs` — `must_use_candidate`
- `crates/ruviform-viz/src/{display,spectrogram}.rs` — `range_plus_one` 誤検出
- `AGENTS.md`・`Cargo.toml` — MSRV clippy 除外対象の明文化

## Done When

- [x] Issue 実測の各失敗が「コード修正」「属性修正」「文書化された除外」のいずれかで個別処理されている
- [x] `mise run fmt-check` / `check` / `lint` / `test` が pin toolchain（1.99.0）でグリーン
- [x] `cargo +1.89.0 clippy --workspace --all-targets -- -D warnings -A clippy::float_cmp -A clippy::similar_names` がグリーン（除外は文書化済みポリシー）
- [x] `cargo +1.89.0 build --workspace` が警告なしで成功

## Verification

- [x] 上記 Done When 全項目を実測し、IR に結果を記録する（`docs/impl/ir/issue-223-msrv-clippy-triage.md` Validation 節を参照）
