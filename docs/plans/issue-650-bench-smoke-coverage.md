---
id: "PLAN-a9Ssxq76yRlE9D8PlcxJMe"
type: "plan"
status: "approved"
title: "Issue #650 bench-smoke lane coverage 拡張 — implementation plan"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/650"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-650-bench-smoke-coverage.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-646-numeric-bench-profile.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-650-bench-smoke-coverage.md"
    - "docs/designs/issue-650-bench-smoke-coverage.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-650-bench-smoke-coverage.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/650"
    - "https://github.com/scarecrow173/ruviform/issues/646"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  defers: []
  deferred-by: []
metadata:
  issue: 650
---

# Issue #650 bench-smoke lane coverage 拡張 — implementation plan

## Goal

`mise run bench-smoke`（CI `bench` job の入口）の対象を bench
target 全量（5 crate）へ拡張し、bench profile 破損を CI lane で
恒久的に検出できる状態にする。あわせて dev-env doc の
consumer-verify 記述を landed schema（Case schema v2）へ最小同期
する。

## Scope

単一 implementation stream。config 1 ファイル + doc 1 ファイル +
evidence + 内部文書チェーンのみで、並列分割の必要はない。

## Upstream Documents

- `docs/specs/issue-650-bench-smoke-coverage.md`

## Design Inputs

- `docs/designs/issue-650-bench-smoke-coverage.md`

## File Map

- `mise.toml` — `[tasks.bench-smoke]` の `run`/`description` を
  5 package 列挙へ更新。
- `docs/development-environment.md` — `bench-smoke` 行（L21）の
  コマンド列・説明更新、`test-ci-consumer-verify` 詳細行（L89）へ
  Case schema v2 の最小追記。
- `evidence/rf650/` — bench-smoke 実行抜粋・検証ログ・audit 結果。
- `docs/{specs,designs,plans,tasks,test-specs,impl}/
  issue-650-bench-smoke-coverage.*` — 本 slice の内部文書チェーン。
- `docs/*/README.md` — doc-maintenance index 再生成（生成物）。

## Implementation Sequence

- [x] bench target 実在の再確認（numeric `corr`・io-e2e
      `io_composition` の `[[bench]]` + benches source）
- [x] consumer-verify audit: dev-env doc 該当節を
      `ci/consumer-verify/evidence.mjs` schema と照合
- [x] `mise.toml` `bench-smoke` の `run`/`description` 更新
- [x] `docs/development-environment.md` `bench-smoke` 行 + L89
      drift 最小修正
- [x] `mise run bench-smoke` 実測 → exit 0・5 target 実行行を証拠化
- [x] `cargo bench --workspace --no-run` / `fmt-check` /
      `prek run --all-files` / `mise tasks` を検証
- [x] 文書チェーン・evidence・docs index 再生成

## Dependencies and Risks

- CI `bench` job は `mise run bench-smoke` を呼ぶ既存経路のため
  `.github/workflows/` 変更不要（実測で task 動作を確認）。
- 追加 2 crate の bench compile で job 時間は伸びるが smoke
  （`-- --test`）の範囲は不変。
- `*-msrv`・`*-full` は Daily Full CI 委譲で本 slice の必須検証に
  含めない（NotRun 記録）。

## Task Breakdown

- `docs/tasks/issue-650-bench-smoke-coverage.md`（単一 task）。

## Verification

- [x] `mise run bench-smoke` → exit 0（`corr`・`io_composition`
      を含む 5 bench target の実行行を `evidence/rf650/` に記録）
- [x] `mise exec -- cargo bench --workspace --no-run` → exit 0
      （Issue AC 由来の canonical 指定・補助証拠）
- [x] `mise run fmt-check` → exit 0
- [x] `mise exec -- prek run --all-files` → exit 0
- [x] `mise tasks` → `bench-smoke` description の 5 package 列挙を
      出力 snippet で確認
- [ ] `*-msrv`・`*-full`・3 OS 横断 — 本 slice の明示対象外
      （NotRun）

## Open Questions / Gaps

- なし。bench target は両 crate に実在し、`-p` 追加のみで
  coverage が揃うことを実測で確認済み。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 task・変更量小）
- Independent work streams: none
- User approval required before dispatch: no（coordinator 確定の
  受入条件に基づく是正タスク）

## Review Handoff

- `bench-smoke` 対象が workspace の bench target 全量へ揃い、
  #646 型の bench profile 破損は CI `bench` job で検出可能になる。
  残る gap: bench target を持たない crate の lib test target は
  smoke 対象外（`--no-run` workspace compile が別途必要 — Issue
  #646 で実施済みの手動検証として記録）。
