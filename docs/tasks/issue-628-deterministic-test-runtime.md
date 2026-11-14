---
id: "TASK-034XBNDF8ghODBVWqwPPlR"
type: "task"
status: "in-progress"
title: "Issue #628 T1 — deterministic core crate 実装・検証・docs chain"
created: "2026-11-14"
updated: "2026-11-14"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-628-deterministic-test-runtime.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-628-deterministic-test-runtime-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-628-deterministic-test-runtime-plan.md"
    - "docs/specs/issue-628-deterministic-test-runtime.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-628-deterministic-test-runtime.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-628-deterministic-test-runtime-plan.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-628-deterministic-test-runtime.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 T1 — deterministic core crate 実装・検証・docs chain

## Work

`crates/ruviform-audio-test` test-only crate を新設し、deterministic core
（VirtualClock・PCM fixtures・MIDI trace・独立 Oracle・Run metadata）を
実装する。Issue #628 first slice のみ — scripted backend・fault injector・
production 配線・CI lane・consumer oracle・3 OS 実走は後続 slice。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-628-deterministic-test-runtime.md`（approved）
- `docs/designs/issue-628-deterministic-test-runtime-design.md`（approved）
- `docs/plans/issue-628-deterministic-test-runtime-plan.md`（approved）
- `docs/test-specs/issue-628-deterministic-test-runtime.md`（approved）
- `docs/designs/issue-627-virtual-first-verification-contract-design.md`
- `POLICIES.md`

## Dependencies

- `ruviform-rt` `MasterClock`（atomic read モデル先例）
- `ruviform-media-core` `transport_schedule`/`midi`/`time`（production 契約）
- `ruviform-audio-core` `buffer`/`time`（PCM buffer 契約）
- `crates/ruviform-audio-device/tests/mock_input_backend.rs`（`assert_no_alloc`
  + `rt_` prefix 先例）
- `crates/ruviform-ui-test`/`ruviform-io-e2e`（test-only crate 宣言先例）

## Affected Files

- `crates/ruviform-audio-test/`（新設 — Cargo.toml/README/src/tests）
- `Cargo.toml`（root — `members` 追加のみ）
- `Cargo.lock`（再生成）
- `crates/README.md`（索引行追加）
- `docs/{specs,designs,plans,tasks,test-specs}/issue-628-*.md`（docs chain）
- `docs/impl/{ir,exp}/issue-628-deterministic-test-runtime.md`（impl 記録）
- `evidence/rf628/`（検証記録）
- `docs/*/README.md`（doc-maintenance/sync_impl_index 再生成）

## Done When

- [ ] `crates/ruviform-audio-test` が workspace member として build でき、
  `mise run check|lint|test ruviform-audio-test` が Pass する。
- [ ] 同一 seed/fixture version/clock 設定から PCM block・MIDI trace・
  run metadata が決定論的に生成され、再現テストが Pass する。
- [ ] `rt_` prefix テストが `assert_no_alloc` で clock-read /
  generator-fill の alloc-free を証明し、`mise run test-rt` で実際に
  収集・実行される（テスト名で確認）。
- [ ] 独立 PCM 数値/byte Oracle・MIDI trace Oracle が独立 fixture/実装で
  検証され、期待値を被試験コードから生成していない。
- [ ] test-only の公開範囲が明文化され、production crate からの依存が
  ないことを確認できる。
- [ ] Issue #628 の残受入は本 slice 対象外として docs に明記されている。

## Verification

- [ ] `mise run fmt-check` — Pass
- [ ] `mise run check ruviform-audio-test` — Pass
- [ ] `mise run lint ruviform-audio-test` — Pass
- [ ] `mise run test ruviform-audio-test` — Pass
- [ ] `mise run test-rt` — `rt_` prefix テストが実際に収集・実行される
- [ ] PCM oracle: silence/impulse/ramp/square/noise/constant は Exact、
  sine/sweep は libm+tolerance で独立検証
- [ ] MIDI oracle: play-through trace が期待列と一致、seek/loop で
  receipt/fault ledger が production 契約通りに記録
- [ ] run metadata が `environment_kind=deterministic-backend`・
  `level=OS-L1`・seed/fixture_version/clock を保持
