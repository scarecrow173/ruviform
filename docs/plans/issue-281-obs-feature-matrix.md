---
id: "PLAN-034XzFeciUWKQZJGHhBzaK"
type: "plan"
status: "approved"
title: "Issue #281 OBS G0 feature matrix lane 実装計画"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/281"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-281-obs-feature-matrix.md"
      - type: "file"
        path: "docs/tasks/issue-281-obs-feature-matrix.md"
      - type: "file"
        path: "docs/impl/ir/issue-281-obs-feature-matrix.md"
      - type: "file"
        path: "docs/impl/exp/issue-281-obs-feature-matrix.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "evidence/rf275/obs-acceptance-ledger.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-281-obs-feature-matrix.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-281-obs-feature-matrix.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "evidence/rf275/obs-acceptance-ledger.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-281-obs-feature-matrix.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-281-obs-feature-matrix.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #281 OBS G0 feature matrix lane 実装計画 Implementation Plan

## Goal

`ruviform-observability` の 4 optional feature について default・
各単独・all-features の compile/lint/test matrix を実行する
mise task 3 本と、それを呼ぶ 3 OS の CI lane
（`observability-verify` + `observability-verify-macos`）を追加し、
受入台帳の G0 行を更新する。

## Scope

単一 implementation stream。変更は `mise.toml`・
`.github/workflows/ci.yml`・`evidence/rf275/obs-acceptance-ledger.md`
・doc chain のみで、`crates/` のコード変更は含まない
（matrix が既存コードで成立するかの確認が目的）。
分割の必要なし。

## Upstream Documents

- `docs/specs/issue-281-obs-feature-matrix.md`（approved）

## Design Inputs

- `docs/designs/issue-281-obs-feature-matrix.md`（approved）

## File Map

- `mise.toml` — `check-observability` / `lint-observability` /
  `test-observability` task を io-compression matrix ブロック
  （L223-238）の直後に追加。各 task は 6 組合せ
  （default・logging・tracing-export・allocation-probe・
  integration・all-features）を `&&` 連結。
- `.github/workflows/ci.yml` — `observability-verify`
  （`matrix.os: [ubuntu-latest, windows-latest]`、
  `fail-fast: false`）と `observability-verify-macos`
  （`macos-latest`）を compression-verify/io-structured-verify
  の後に追加。`ci-summary` の `needs` に 2 job を追加。
- `evidence/rf275/obs-acceptance-ledger.md` — §3 の G0 行・
  §4.2 の関連行を lane 新設の事実で更新（CI 未実走分は
  `NotRun` 維持）。必要に応じ §0 証拠一覧にローカル実走の
  E-LOCAL 行を追加。
- `docs/specs|designs|plans|tasks|test-specs|impl/ir|impl/exp/
  issue-281-obs-feature-matrix.md` — 本 doc chain。
- 各 docs dir の `README.md` index — doc-maintenance
  `index apply` で再生成（生成物）。

## Implementation Sequence

- [x] `mise.toml` に 3 task を追加（io-compression パターン踏襲）。
- [x] `ci.yml` に 2 job を追加し `ci-summary` needs を更新。
- [x] Windows ローカルで `mise run check-observability` →
  `lint-observability` → `test-observability` を実走（全て exit 0）。
- [x] `actionlint`・workflow 契約 test・TOML 検査を実行。
- [x] `obs-acceptance-ledger.md` の G0 行を更新。
- [x] impl/ir・impl/exp を実績で記述し、docs index を再生成。
- [ ] `git fetch origin main && git merge origin/main` で最新 main
  を取り込み、最終検証後に commit/push/PR。

## Dependencies and Risks

- 依存: なし（既存コードへの変更なし。matrix が失敗した場合は
  別途報告 — 本 slice では crates/ を触らない）。
- Risk: Windows/macOS で feature 単独組合せの test が失敗する
  可能性 → lane の正しい Fail として扱い、修正は別 slice。
- Risk: `ci-summary` で新 lane が `undeclared-*` note 付きで
  集約される → verdict は fail-closed で正しい。`summary.mjs`
  宣言同期は編集範囲外（design Deferred 参照）。
  → 解消: coordinator が `docs/development-environment.md` と
  `ci/summary/`（`summary.mjs` + tests）の範囲拡張を承認し、
  `REQUIRED_JOBS`/`REQUIRED_CHECKOUT_LEGS` と期待リストを
  同期更新済み（undeclared 注記なし・fail-closed 維持）。

## Task Breakdown

- `docs/tasks/issue-281-obs-feature-matrix.md` — 単一 task
  （本 slice 全体）。依存分解の必要なし。

## Verification

- [x] `mise run fmt-check`（rustfmt — exit 0）。
- [x] `mise run check-observability` exit 0（Windows、6 組合せ）。
- [x] `mise run lint-observability` exit 0（同上）。
- [x] `mise run test-observability` exit 0（同上）。
- [x] `mise run check ruviform-observability` /
  `lint ruviform-observability` / `test ruviform-observability`
  （変更範囲の package 検証として既存入口も実行、全て exit 0）。
- [x] `mise exec -- actionlint` が `ci.yml` を通過（exit 0）。
- [x] `node --test` で `ci/daily/tests/ci-yml-contract.test.mjs`・
  `ci/summary/tests/summary.test.mjs`・
  `ci/consumer-verify/tests/` の契約 test が通る
  （10/10・31/31・64/64 Pass）。
- [x] `mise tasks` が新 task を認識（TOML 妥当性）。
- [ ] docs index 再生成と doc-status lint の整合。

## Open Questions / Gaps

- 解消済み: `docs/development-environment.md` の lane 一覧更新と
  `ci/summary/summary.mjs`（+ tests）の宣言同期は、当初は編集範囲外
  だったが coordinator が範囲拡張を承認したため本 slice で実施済み
  （compression-verify/io-structured-verify と同じ宣言パターン）。
- 残件: merge 後対象 SHA の Full CI 実走と台帳 G0 行の
  `NotRun → Pass` 更新は統合検証段階の作業（本 PR では NotRun
  のまま記録）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 worker が直接実装。
  subagent 不使用 — タスク契約）。
- Independent work streams: none。
- User approval required before dispatch: yes → review-protocol
  のレビューで代替（blocker なしを承認として扱う）。

## Review Handoff

- Reviewer focus: (a) `&&` 連結 task の組合せ網羅性
  （6 組合せの欠落・typo）、(b) ci.yml job が既存 lane の
  sha 証拠・fail-closed 契約を満たすか、(c) 台帳更新が
  NotRun を Pass に偽装していないか、(d) needs 追加が
  summary evaluator の undeclared 経路で正しく fail-closed
  になるか。
