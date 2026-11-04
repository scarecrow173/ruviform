---
id: "PLAN-xLvFbRwmfJj1P2olT3eT9q"
type: "plan"
status: "approved"
title: "Issue #332 CI-OPT-05 実装計画"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/332"
    - "docs/specs/issue-332-playwright-browser-cache.md"
    - "docs/designs/issue-332-playwright-browser-cache.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/designs/issue-327-cross-repo-ci-baseline.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-332-playwright-browser-cache.md"
    - "docs/designs/issue-332-playwright-browser-cache.md"
  implemented-by:
    - "docs/tasks/issue-332-t1-playwright-browser-cache.md"
  depends-on:
    - "docs/designs/issue-332-playwright-browser-cache.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-332-playwright-browser-cache.md"
    - "docs/designs/issue-332-playwright-browser-cache.md"
  derived-by:
    - "docs/tasks/issue-332-t1-playwright-browser-cache.md"
    - "docs/test-specs/issue-332-playwright-browser-cache.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-332-playwright-browser-cache.md"
  references:
    - "mise.toml"
    - ".github/workflows/ci.yml"
  defers: []
  deferred-by: []
---

# Issue #332 CI-OPT-05 実装計画 Implementation Plan

## Goal

`ui-verify` ジョブの browser lane を「system deps → browser バイナリ（cache 化）→ launch 検証 → test」に step 分離し、browser install を `test-ui-browser` から独立 mise task へ切り出す。検証能力・Evidence・required checks は不変。

## Scope

単一の実装ストリーム。mise.toml の task 分離・ci.yml の cache/準備/検証 step 追加・AGENTS.md/#327 設計 doc 同期は同一論理変更であり分割しない。

## Upstream Documents

- `docs/specs/issue-332-playwright-browser-cache.md`（AC-1〜AC-6）

## Design Inputs

- `docs/designs/issue-332-playwright-browser-cache.md`（cache key・path pin・fallback・step/lane 設計）

## File Map

- `mise.toml` — `install-ui-browser-chromium` 新設（`depends = ["install-ui-browser-test"]`、`run = npm --prefix ui/browser-test exec -- playwright install chromium`）。`test-ui-browser` の `run` から install 行を除去し `depends` へ追加。
- `.github/workflows/ci.yml` — `ui-verify` job に `PLAYWRIGHT_BROWSERS_PATH` env、version/revision 算出 step、`actions/cache` restore、prepare step、launch 検証 step を追加。
- `AGENTS.md` — 検証入口テーブルに `install-ui-browser-chromium` 行追加・`test-ui-browser` 行更新、ui-verify 説明文に cache/準備/検証 step を明記。
- `docs/designs/issue-327-cross-repo-ci-baseline.md` — Deferred Design Concerns の Playwright 項目を #332 で解決済みに更新（lane 表は既に `playwright` 行を持つため表自体の変更なし）。

## Implementation Sequence

- [ ] mise.toml: `install-ui-browser-chromium` を `install-ui-browser-test` の直後に追加。
- [ ] mise.toml: `test-ui-browser` の `depends` に `install-ui-browser-chromium` を追加し `run` から `playwright install chromium` 行を除去（`[page build, npm test]` の 2 行へ）。
- [ ] ci.yml: `ui-verify` job の `env` に `PLAYWRIGHT_BROWSERS_PATH: ${{ github.workspace }}/.cache/ms-playwright` を追加。
- [ ] ci.yml: `Install Playwright browser system deps (Linux)` の後・`Run UI browser tests` の前に、version/revision 算出 → cache restore → prepare → launch 検証の 4 step を挿入。
- [ ] AGENTS.md: 検証入口テーブルと ui-verify 記述を新構造へ同期。
- [ ] docs/designs/issue-327-cross-repo-ci-baseline.md: Deferred 項目の更新。
- [ ] ローカル検証（fresh/warm/単独実行・擬似破損 fallback）→ 計測記録 → impl record + exp 記録 → managed index 再生成。

## Dependencies and Risks

- `node_modules/playwright-core/browsers.json` は install-ui-browser-test 完了後に存在（setup-ui で保証）。算出 step は `set -euo pipefail` で fail-closed。
- `actions/cache@v6.1.0`・`jdx/mise-action@v5.2.0` は repo 現行 pin と同一系列を使う（新規 action 依存なし）。
- ブロッカーなし。#329/#331（Rust lane）・#333（DAG 化）とは非重複の独立変更。

## Task Breakdown

- `docs/tasks/issue-332-t1-playwright-browser-cache.md` — 上記 File Map 全体（mise.toml + ci.yml + AGENTS.md + #327 doc + 検証）。単一タスクでコミットは docs chain / mise+ci+AGENTS / impl record の論理単位に分割。

## Verification

- [ ] `mise run install-ui-browser-chromium` が単独で `playwright install chromium` を実行する（install-ui-browser-test の depends 経由）。
- [ ] `mise run test-ui-browser` が depends で browser install を解決し、page build + suite が緑（install が task 内でも走ること）。
- [ ] `$PLAYWRIGHT_BROWSERS_PATH` 設定時に install/test が同一 path を参照すること（CI 想定 path でのローカル確認）。
- [ ] 擬似破損（browsers path 内 binary 削除/破壊）で launch 検証 → wipe → reinstall → 検証成功へ落ちること。
- [ ] `mise exec -- actionlint .github/workflows/ci.yml` と `mise exec -- prek run --all-files` が緑。
- [ ] Before/After: browser setup（prepare+validate+deps）・download 有無・test 時間を PR CI run で分離記録し impl/exp に転記。

## Open Questions / Gaps

- 3 OS matrix の実差・cache hit/miss の実値は CI run でのみ確定（ローカルは Windows のみ）。Linux/macOS は CI run 確認を未検証項として報告。
- GH-hosted runner の CDN 速度が高いため warm の絶対効果は中程度と見込む。After 計測で確定。

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

- 焦点: cache key が version/revision/OS-arch の pin 源に厳密か、launch 検証が marker-only 破損を検知できるか、`PLAYWRIGHT_BROWSERS_PATH` の env 一貫性（install・validate・test の同一 path）、検証能力（Evidence・fixture・step 分割の失敗特定性）が削られていないか。
