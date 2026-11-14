---
id: "PLAN-034XCT9Qm6U9QcMldwvZAH"
type: "plan"
status: "completed"
title: "Issue #306 G01/G05 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/large_staged.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/benches/io_composition.rs"
      - type: "file"
        path: "evidence/rf306/large-bench-report.md"
      - type: "file"
        path: "docs/plans/issue-306-io-lane-completion.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-e2e/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-306-io-lane-completion.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-306-io-lane-completion.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-io-lane-completion.md"
    - "docs/designs/issue-306-io-lane-completion.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-306-io-lane-completion.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 G01/G05 実装計画 Implementation Plan

## Goal

`ruviform-io-e2e` に G01（実 4GiB 境界 opt-in 試験）と G05
（criterion 性能 baseline bench）の 2 lane を追加し、実走証拠を
`evidence/rf306/` に記録する。

## Scope

単一 implementation stream。`crates/ruviform-io-e2e` 内に閉じ、
`mise.toml`・CI・他クレート・docs index 生成物（`Cargo.lock`
の依存一覧差分を除く）を変更しない。

## Upstream Documents

- `docs/specs/issue-306-io-lane-completion.md`

## Design Inputs

- `docs/designs/issue-306-io-lane-completion.md`（approved）

## File Map

- `crates/ruviform-io-e2e/tests/large_staged.rs` - G01 opt-in 試験
  （`#[ignore]`・preflight・5 scenario × 3 サイズ・FNV-1a
  checksum・DiskFull wrapper）
- `crates/ruviform-io-e2e/benches/io_composition.rs` - G05 criterion
  bench（6 cell + peak alloc probe + env metadata）
- `crates/ruviform-io-e2e/Cargo.toml` - criterion dev-dep +
  `[[bench]]` 登録
- `crates/ruviform-io-e2e/src/lib.rs` - lane 一覧へ large_staged /
  io_composition を追記
- `crates/ruviform-io-e2e/README.md` - 責務・検証節の更新
- `evidence/rf306/large-bench-report.md` - G01/G05 実行証拠
- `Cargo.lock` - io-e2e 依存一覧への criterion 追加（生成物）

## Implementation Sequence

- [ ] `Cargo.toml` に criterion dev-dep + `[[bench]]` を追加し、
  `cargo check -p ruviform-io-e2e --all-targets` で lock 更新と
  compile を確認
- [ ] `tests/large_staged.rs` を実装（preflight + scenario matrix
  + checksum/pattern helper）
- [ ] `benches/io_composition.rs` を実装（cells + CountingAlloc +
  env metadata）
- [ ] `src/lib.rs` / `README.md` を更新
- [ ] `mise run fmt-check` / `check-io-e2e` / `lint-io-e2e` /
  `test-io-e2e` / `test-io-e2e-corpus` を実行
- [ ] G01 opt-in lane を実走し結果を記録
- [ ] `mise run bench` を実走し io_composition 実測値を記録
- [ ] `evidence/rf306/large-bench-report.md` を作成
- [ ] docs index を generator で再生成

## Dependencies and Risks

- criterion `0.8.2` は `Cargo.lock` 既収録（ruviform-rt dev-dep）。
  io-e2e の依存一覧への追加のみで version 解決は変わらない見込み。
- G01 は temp に実 4GiB 級を書く。temp 配置ドライブと空き容量を
  preflight で観測し、不足時は NotRun（Pass/Fail 偽装なし）。
- `mise run bench` は workspace 全量のため実行時間が長い —
  ローカル手動 lane として受容（canonical = mise、cargo bench
  直接実行は補助証拠扱い）。
- `bench-smoke`（CI）は io_composition を含まない — drift を
  証拠・PR 本文に記録。

## Task Breakdown

- T1: Cargo.toml + bench/test 実装
- T2: lib.rs/README 更新 + lane 記述
- T3: 検証 lane 実行 + G01/G05 実走 + evidence 記録
- T4: docs index 再生成 + review + PR

## Verification

- [ ] `mise run fmt-check` exit 0
- [ ] `mise run check-io-e2e` exit 0（bench target も compile 検査）
- [ ] `mise run lint-io-e2e` exit 0（`clippy --all-targets -D
  warnings`、bench を網羅）
- [ ] `mise run test-io-e2e` exit 0（非回帰。bench は通常
  target 外）
- [ ] `mise run test-io-e2e-corpus` exit 0（G08 非回帰）
- [ ] `mise run test ruviform-io-e2e --test large_staged --
  --ignored --nocapture` 実走 → Pass/NotRun を evidence 記録
- [ ] `mise run bench` 実走 → io_composition 実測値を evidence
  記録

## Open Questions / Gaps

- WAV RIFF 実ファイル 4GiB variant は実装しない（design で
  NotRun 判断済み — `wav_golden` discard-sink が既存根拠）。
- bench の絶対性能値は環境依存 — baseline は同一環境での相対
  比較用であり、consumer 固有閾値は別 Issue が正本。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
  consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatch された worker が
  直接実装）
- Independent work streams: none
- User approval required before dispatch: yes（Orca dispatch 経由で
  承認済み）
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- reviewer focus: G01 の NotRun 誠実性（skip 理由・Pass 偽装
  なし）、G05 の閾値/判定の事前定義、unsafe 局所許可の SAFETY
  記述、drift（bench-smoke・development-environment.md）の記録。
