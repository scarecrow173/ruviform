---
id: "PLAN-034RBIEnfNn4DeMAPw27Ii"
type: "plan"
status: "approved"
title: "Issue #330 CI-OPT-03 実装計画"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/330"
    - "docs/specs/issue-330-ui-npm-ci-dedup.md"
    - "docs/designs/issue-330-ui-npm-ci-dedup.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-330-ui-npm-ci-dedup.md"
    - "docs/designs/issue-330-ui-npm-ci-dedup.md"
  implemented-by:
    - "docs/tasks/issue-330-t1-mise-ui-install-dedup.md"
  depends-on:
    - "docs/designs/issue-330-ui-npm-ci-dedup.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-330-ui-npm-ci-dedup.md"
    - "docs/designs/issue-330-ui-npm-ci-dedup.md"
  derived-by:
    - "docs/tasks/issue-330-t1-mise-ui-install-dedup.md"
    - "docs/test-specs/issue-330-ui-npm-ci-dedup.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-330-ui-npm-ci-dedup.md"
  references:
    - "mise.toml"
    - ".github/workflows/ci.yml"
  defers: []
  deferred-by: []
---

# Issue #330 CI-OPT-03 実装計画 Implementation Plan

## Goal

`ui-verify` ジョブ内の `npm ci` を package-lock ごとに原則 1 回へ集約し、verify タスクは準備済み `node_modules` を使う構造に分離する（検証ステップ・Evidence・タスク名は不変）。

## Scope

単一の実装ストリーム。mise.toml のタスク再編・ci.yml 1 ステップ追加・AGENTS.md 同期は同一論理変更であり分割しない。

## Upstream Documents

- `docs/specs/issue-330-ui-npm-ci-dedup.md`（AC-1〜AC-6）

## Design Inputs

- `docs/designs/issue-330-ui-npm-ci-dedup.md`（install/freshness/depends 構造・ジョブ差分）

## File Map

- `mise.toml` — `install-ui-*` 7 本 + `setup-ui` 集約タスク新設、verify タスクから `npm ci` 除去と `depends` 付与、`build-ui-*` に sources/outputs。
- `.github/workflows/ci.yml` — `ui-verify` に `mise run setup-ui` 準備ステップ挿入。
- `AGENTS.md` — 検証入口テーブルの UI npm 系行を新構造へ同期（タスク説明 + ui-verify 記述）。

## Implementation Sequence

- [ ] mise.toml: `install-ui-<pkg>` タスクを 7 本追加（`run` = `npm --prefix ui/<pkg> ci --no-audit --no-fund`、`sources` = lockfile+manifest、`outputs` = `node_modules/.package-lock.json`）。
- [ ] mise.toml: `setup-ui` 集約タスク追加（`depends` = install-ui-* ×7）。
- [ ] mise.toml: 全 check/test/build UI タスクから `npm ci` 行を除去し `depends` に対応 install を設定。browser-test 2 タスクは `install-ui-browser-test` + `build-ui-showcase`。
- [ ] mise.toml: `build-ui-{components,viz,binding,showcase}` に `sources`（自 package + compile include する sibling src + tsconfig）と `outputs`（`dist/**/*`）を付与。
- [ ] ci.yml: `Restore cargo registry/git index cache` 直後に `Install UI npm dependencies (mise run setup-ui)` ステップ追加。
- [ ] AGENTS.md: UI npm 系タスク行・ui-verify 説明文を新構造へ更新。
- [ ] ローカル検証（fresh/warm/単独実行）→ 計測記録 → impl record → managed index 再生成。

## Dependencies and Risks

- mise `sources`/`outputs` freshness は 2026.10.1（pin 済み）で安定機能。skip 条件は output 実在 + source snapshot 一致で fail-safe 側（不明なら run）。
- `npm ci` が `node_modules/.package-lock.json` を書くことは npm 標準挙動（実測済み）。
- ブロッカーなし。CI-OPT-00（#327）は計測テンプレートの参照先であり、本変更は独立に着地可能。

## Task Breakdown

- `docs/tasks/issue-330-t1-mise-ui-install-dedup.md` — 上記 File Map 全体（mise.toml + ci.yml + AGENTS.md + 検証）。単一タスクでコミットは docs / mise+ci / impl-record の論理単位に分割。

## Verification

- [ ] `rm -rf ui/*/node_modules ui/*/dist` 後、`mise run setup-ui` → `npm ci` が 7 回のみ実行される。
- [ ] `mise run check-ui-headless` → `install-ui-core` が `sources up-to-date, skipping` となり typecheck 成功。
- [ ] `mise run test-ui-browser` → `build-ui-showcase` が skip され、Playwright suite が showcase dist 参照で緑。
- [ ] node_modules 削除後の `mise run test-ui-headless` 単独実行が依存解決込みで成功（Fresh checkout 互換）。
- [ ] `mise run test-ui-showcase`（deps.test.ts DAG audit 含む）が不変で緑。
- [ ] `actionlint`（prek 経路）で ci.yml が検証通過。
- [ ] Before/After: npm ci 回数 20→7/ジョブ、計測時間を impl record に記録。

## Open Questions / Gaps

- 3 OS matrix の実測は CI 実行でしか確認できない。ローカルは Windows のみ。Linux/macOS は CI run での確認を worker 報告の未検証項として明記する。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（mise.toml/ci.yml/AGENTS.md の確定的編集 + mise run 検証）。
- Independent work streams: none（単一タスク）。
- User approval required before dispatch: no（coordinator dispatch 済み作業）。
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 焦点: freshness の sources 指定が package の compile include（rootDir `..` の sibling src 横断）と一致しているか、`depends` 方向が deps.test.ts の DAG と矛盾しないか、`npm ci` 除去後も Linux の `playwright install-deps` 前置条件が成立するか。
