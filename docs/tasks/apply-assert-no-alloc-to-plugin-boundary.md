---
id: "TASK-034JP1tb1wQI9ac2cEwBq4"
type: "task"
status: "todo"
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

## Done When

- [ ] `plugin-clap`・`plugin-vst3` の全 `rt_*` RT 契約テストが `assert_no_alloc` 測定窓内で実行される
- [ ] 現行コードで全件パス（0 allocation）
- [ ] deferred コメントが「Issue #22 で実測済み」の記述に更新されている
- [ ] `alloc_counter`（ruviform-rt）は変更なしで併存

## Verification

- [ ] `mise run test-rt` で新規適用テストが全パス
- [ ] `mise run lint`/`check` パス
