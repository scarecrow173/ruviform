---
id: "TASK-034JP1tb1wQI9ac2cEwBq4"
type: "task"
status: "done"
title: "プラグイン境界 RT 契約テストへの assert_no_alloc 適用"
created: "2026-10-21"
updated: "2026-10-21"
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
    - "../plans/implement-issue-22-rt-verification.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-22-rt-verification.md"
    - "wire-clap-thread-check.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-22-rt-verification.md"
    - "wire-clap-thread-check.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/rt-verification-contracts.md"
  references: []
  defers: []
  deferred-by: []
---

# プラグイン境界 RT 契約テストへの assert_no_alloc 適用

## Work

1. `crates/ruviform-plugin-clap/Cargo.toml`・`crates/ruviform-plugin-vst3/Cargo.toml` の `[dev-dependencies]` に `assert_no_alloc = "1.1"` を追加（`ruviform-audio-device` と同じ指定形・理由コメント付き）
2. `plugin-clap` `src/plugin.rs`: `rt_process_drives_paired_stereo_block_allocation_free` を `assert_no_alloc::assert_no_alloc(|| { ... })` の測定窓内に移す — `GainShared::initialize`・`activated`・fixture 確保・`start_processing` は窓の外、`process_block` 呼出のみ窓内。「Issue #22 に deferred」コメントを実測済みの記述に更新
3. `plugin-clap` の `rt_process_full_stereo_block`（`gain.rs`）等、他の `rt_*` process テストにも同様に適用（測定窓の内外を既存パターンに倣う）
4. `plugin-vst3` `src/block.rs`: `rt_prepare_process_finish_round_trip` を `assert_no_alloc` 測定窓内に移し、「Issue #22 まで doc 契約」のコメントを実測済みに更新
5. 測定窓内で allocation が検出された場合は原因を特定し、修正可能なら修正・できなければ deferred として記録（spec の Out of Scope 節と整合させる）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-22-rt-verification.md](../plans/implement-issue-22-rt-verification.md)
- [../designs/rt-verification-lane-design.md](../designs/rt-verification-lane-design.md)
- [../adr/rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md)
- [../test-specs/rt-verification-contracts.md](../test-specs/rt-verification-contracts.md)（Rules 1–2）
- [wire-clap-thread-check.md](wire-clap-thread-check.md)

## Dependencies

- 先行タスク: [wire-clap-thread-check.md](wire-clap-thread-check.md)

## Affected Files

- `crates/ruviform-plugin-clap/Cargo.toml`（dev-dep）
- `crates/ruviform-plugin-clap/src/plugin.rs`（rt_ テスト）
- `crates/ruviform-plugin-clap/src/gain.rs`（rt_ テスト）
- `crates/ruviform-plugin-vst3/Cargo.toml`（dev-dep）
- `crates/ruviform-plugin-vst3/src/block.rs`（rt_ テスト・deferred コメント）
- `crates/ruviform-plugin-vst3/src/component.rs`（rt_ テスト3件）
- `crates/ruviform-plugin-clap/src/lib.rs`（crate doc の deferred 記述）

## Done When

- [x] `plugin-clap`・`plugin-vst3` の全 `rt_*` RT 契約テストが `assert_no_alloc` 測定窓内で実行される
- [x] 現行コードで全件パス（0 allocation）
- [x] deferred コメントが「Issue #22 で実測済み」の記述に更新されている
- [x] `alloc_counter`（ruviform-rt）は変更なしで併存

## Verification

- [x] `cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3` 全パス（clap 43 unit・vst3 28 unit、`rt_` 命名で `mise run test-rt` 対象）
- [x] `mise run lint`/`check` パス

## Completion Notes

- 測定窓の適用範囲:
  - `plugin.rs` `rt_process_drives_paired_stereo_block_allocation_free` — `process_block`（thread-check gate・lifecycle gate・frame bound・view・event 変換・processor・status map の全経路）を窓内。fixture/`EventBuffer` 確保は窓外。
  - `gain.rs` `rt_process_full_stereo_block` — `GainProcessor::process` を窓内。view/processor 構築・prepare は窓外。
  - `block.rs` `rt_prepare_process_finish_round_trip` — `prepare_block` + view アクセス + `finish_block` を窓内。fixture/scratch 確保は窓外。
  - `component.rs` `rt_process_copies_gain_zero_copy` / `rt_param_scan_stays_bounded_when_host_counts_lie` / `rt_param_scan_total_budget_bounds_worst_case` — `processor.process` COM vtable 呼出のみ窓内。activate/teardown・COM wrapper 構築は窓外。
- `#[global_allocator] AllocDisabler` はテストバイナリごとに1個: plugin-clap は `plugin.rs` の `mod tests`、plugin-vst3 は `block.rs` の `mod tests` に宣言（`#[cfg(debug_assertions)]` 付き・audio-device と同形式）。`gain.rs`/`component.rs` のテストは同一バイナリの同アロケータを共有。
- thread-check ABI テスト（T1）は `PluginInstance::process` の host 側 marshalling を含むため窓で囲まず、契約（rejection/透過）検証に留めた。測定対象はプラグイン側経路（Rule 1）のみ。
- 0 allocation で全件パス — deferred 記録は不要だった。
- `alloc_counter`（ruviform-rt）に変更なし。
