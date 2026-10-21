---
id: "PLAN-034JP0n5QlXSj34GK30GBz"
type: "plan"
status: "approved"
title: "Issue #22 RT 検証基盤の実施（assert_no_alloc 適用・thread-check 統合・検証文書）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/22"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/block.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/Cargo.toml"
      - type: "file"
        path: "README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/rt-verification-lane.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/rt-verification-lane.md"
    - "../designs/rt-verification-lane-design.md"
    - "../adr/rt-assert-no-alloc-thread-check.md"
  derived-by:
    - "../tasks/wire-clap-thread-check.md"
    - "../tasks/apply-assert-no-alloc-to-plugin-boundary.md"
    - "../tasks/document-rt-verification-scope.md"
  verifies: []
  verified-by:
    - "../test-specs/rt-verification-contracts.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #22 RT 検証基盤の実施（assert_no_alloc 適用・thread-check 統合・検証文書） Implementation Plan

## Goal

CI で検出可能な RT 違反を機械検査する基盤を整備する — `assert_no_alloc` による allocation 検出をプラグイン境界の RT 契約テストに適用し、CLAP `thread-check` 拡張相当のスレッド契約検査を `plugin-clap` の `process` に統合し、検証文書に保証範囲限定を明記する。

## Scope

単一実装ストリーム。`assert_no_alloc` 適用（plugin-clap/plugin-vst3）と thread-check 統合（plugin-clap）は同一 crate・同一テスト領域を触るため分割しない。`ruviform-rt` の `alloc_counter` 置き換え・VST3 スレッド検査・静的検証不能制約の機械検査はスコープ外。

## Upstream Documents

- [../specs/rt-verification-lane.md](../specs/rt-verification-lane.md)（`SPEC-034JOU6EjinMh0rhxE6yM9`・approved）
- [../adr/rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md)（accepted — `assert_no_alloc` 併存・`is_audio_thread` defined rejection の意味論）
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md)（accepted — 検証方針の確定決定）

## Design Inputs

- [../designs/rt-verification-lane-design.md](../designs/rt-verification-lane-design.md)（`DESIGN-034JOU7C43HBPJBsnz8JQP`・approved — データ/制御フロー・RT 制約・unsafe 正当化）

## File Map

- `crates/ruviform-plugin-clap/Cargo.toml` — `clack-extensions` に `thread-check` feature 追加・`assert_no_alloc` dev-dep 追加
- `crates/ruviform-plugin-clap/src/plugin.rs` — `GainShared` に `host`/`thread_check` 保持・`process` で `is_audio_thread` 照会・rejection・テストホスト `HostThreadCheckImpl` 実装・`rt_*` テストを `assert_no_alloc` 下に
- `crates/ruviform-plugin-vst3/Cargo.toml` — `assert_no_alloc` dev-dep 追加
- `crates/ruviform-plugin-vst3/src/block.rs` — `rt_prepare_process_finish_round_trip` 等を `assert_no_alloc` 下に・deferred コメント更新
- `README.md` — 保証範囲限定の明記

## Implementation Sequence

- [ ] `plugin-clap` の `clack-extensions` に `thread-check` feature 追加、`assert_no_alloc` dev-dep 追加
- [ ] `GainShared` に `host: Option<HostSharedHandle<'static>>`・`thread_check: Option<HostThreadCheck>` を追加し、`new_shared` で `get_extension` + `with_arbitrary_lifetime` で保持（SAFETY コメント付き）
- [ ] `process` 冒頭で `thread_check.is_audio_thread(&host)` を照会し `Some(false)` を defined rejection、`None`/`Some(true)` は従来処理
- [ ] テストホスト `RtCheckHost`（`HostHandlers`・`Shared` が `HostThreadCheckImpl`）を実装し、`is_audio_thread` を `AtomicBool` で制御可能にする
- [ ] `plugin-clap` の `rt_process_*`・ABI 通しテストを `assert_no_alloc` 下に配置
- [ ] `plugin-vst3` の `rt_prepare_process_finish_round_trip` 等を `assert_no_alloc` 下に配置・deferred コメント更新
- [ ] `README.md` に保証範囲限定を明記
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`・`bench-smoke`・`prek run --all-files` 全パス・doc 監査

## Dependencies and Risks

- `with_arbitrary_lifetime` の unsafe は plugin lifetime が host に包含される CLAP 契約で正当化 — ホスト側実装不備時の dangling リスクはテスト実測でカバー
- `assert_no_alloc` 適用で既存コードの allocation が検出された場合は原因特定して修正または deferred 記録
- `is_audio_thread` `None` は graceful degradation — ホスト非対応時は検査無効である旨を文書化

## Task Breakdown

- `wire-thread-check` — `clack-extensions` feature・`GainShared` host/thread_check 保持・`process` defined rejection
- `assert-no-alloc-lane` — plugin-clap/plugin-vst3 の `rt_*` テストへの `assert_no_alloc` 適用・dev-dep 追加
- `document-rt-verification-scope` — README/設計書の保証範囲明記・上流文書チェックボックス更新・全量検証・Issue 完了確認

## Verification

- [ ] `plugin-clap`・`plugin-vst3` の RT 契約テストが `assert_no_alloc` 下で全件パス
- [ ] `process` が `is_audio_thread = Some(false)` で defined rejection、`None` で従来動作を返す ABI 通しテスト
- [ ] `README.md`・設計書に保証範囲限定が明記されている
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス
- [ ] 監査で本変更起因 findings 0

## Open Questions / Gaps

- `PluginInstance::<H>` の `HostHandlers` derive で `declare_extensions` に `HostThreadCheck` を register する手続きが clack-host 0.2.0 でそのまま動くかは実装で確認する

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本セッションで逐次実施 — Orca worker 不使用のユーザー指示による）
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `with_arbitrary_lifetime` の SAFETY 正当化と `is_audio_thread` `Some(false)` → defined rejection の意味論がレビュー焦点。`process` への 1 FFI call 追加が RT 予算内であることの確認
