---
id: "PLAN-6ZacrJMGjfi3GkTurYfEpj"
type: "plan"
status: "approved"
title: "Issue #294 consumer oracle lanes 実装計画 — probe/run/record 追加・rec() spec 拡張・contract test 更新"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-consumer-oracle-lanes.md"
    - "docs/designs/issue-294-consumer-oracle-lanes.md"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-294-consumer-oracle-lanes.md"
      - type: "doc"
        path: "docs/tasks/issue-294-consumer-oracle-lanes.md"
      - type: "doc"
        path: "docs/test-specs/issue-294-consumer-oracle-lanes.md"
      - type: "doc"
        path: "docs/impl/ir/issue-294-consumer-oracle-lanes.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-294-consumer-oracle-lanes.md"
  implemented-by:
    - "docs/tasks/issue-294-consumer-oracle-lanes.md"
    - "docs/test-specs/issue-294-consumer-oracle-lanes.md"
  depends-on:
    - "docs/designs/issue-294-consumer-oracle-lanes.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-294-consumer-oracle-lanes.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-consumer-oracle-lanes.md"
    - "docs/designs/issue-294-consumer-oracle-lanes.md"
  derived-by:
    - "docs/tasks/issue-294-consumer-oracle-lanes.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-consumer-oracle-lanes.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #294 consumer oracle lanes 実装計画

## Goal

`consumer-verify.yml` の両 verify job に consumer oracle lane（`oracle_probe` + run step(s) + record 3 分岐）を追加し、`rec()` spec を oracle/fixture_version 対応へ拡張する。`consumer-verify-yml-contract.test.mjs` を更新して構造契約を pin し、`docs/development-environment.md` の lane 列挙へ追記する。

## Scope

In: `.github/workflows/consumer-verify.yml`、`ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs`、`docs/development-environment.md`（consumer-verify 節の lane 列挙のみ）、doc chain（本 plan/task/spec/design/test-spec/impl ir）、`evidence/rf294/`。
Out: `evidence.mjs` 本体、`mise.toml`、`ci/daily`/`ci/tracker`/`ci/summary`、他 workflow、`Cargo.toml`/`Cargo.lock`、`crates/`、consumer repo、docs index（`docs/**/README.md` — Coordinator が統合時に再生成）。

## Task Breakdown

| task | 内容 | 検証 |
|---|---|---|
| [T-oracle](../tasks/issue-294-consumer-oracle-lanes.md) | workflow 両 job へ probe/run/record 追加 + `rec()` spec 拡張、contract test 更新、docs/environments 追記、evidence 記録 | `mise run test-ci-consumer-verify` / `mise exec -- actionlint .github/workflows/consumer-verify.yml` / `mise exec -- prek run --all-files` / 補助 `mise exec -- node --test ci/` |

## File Map

- `.github/workflows/consumer-verify.yml` — ヘッダ comment 追記、両 job の `oracle_probe`/`oracle`(/`oracle_rt`) step 追加、両 job の `rec()` 拡張 + FIXTURE_VER 抽出 + oracle lane 行追加。
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs` — `CASE_SPEC_RE` grammar、`rshaRows` 件数/slug、新規 oracle pin test。
- `docs/development-environment.md` — :131 の lane 列挙へ oracle lane 追記のみ。

## Risk Register

- **probe='1' だが run step が skipped（fetch 失敗等）で pass 化**: record 分岐 1 は `o(steps.oracle.outcome)` を使うため `skipped` と正直記録される — `o()` の skipped→skipped 写像を contract test で pin。
- **`rec()` spec の field 増加で既存行が壊れる**: `IFS='|' read` の後方互換（field 6/7 省略時は flag 非付与）を維持し、CASE_SPEC_RE で oracle+fixture のペア必須を pin。
- **`fixture_version` 抽出の脆さ（grep -A3 依存）**: Cargo.lock 形式変更で空になるが flag 省略（null）に留まり捏造しない — 抽出失敗が outcome を汚染しないことを design/IR に記録。
- **SV `--test` 併記誘惑**: file 別 verdict attribution のため別 step を維持 — contract test で `oracle`/`oracle_rt` の別 id・別 record 行を pin。
- **consumer ref が古く oracle test 不在**: probe='0' → owner 付き `not_run`（偽 pass 禁止）— 分岐を contract test で pin。

## Rollback

`.github/workflows/consumer-verify.yml` と contract test を PR 前 revision へ revert。oracle lane が消えるだけで既存 lane 構成への永続影響なし。
