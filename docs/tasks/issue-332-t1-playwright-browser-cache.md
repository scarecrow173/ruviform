---
id: "TASK-O3Q2a0lPvct9PFKBgBig2j"
type: "task"
status: "in-progress"
title: "T1 Playwright Chromium cache・準備 step 分離（Issue #332）"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/332"
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
    - "docs/plans/issue-332-playwright-browser-cache.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-332-playwright-browser-cache.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-332-playwright-browser-cache.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-332-playwright-browser-cache.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 Playwright Chromium cache・準備 step 分離（Issue #332）

## Work

- `mise.toml` に `install-ui-browser-chromium` を新設（`depends = ["install-ui-browser-test"]`、`run = npm --prefix ui/browser-test exec -- playwright install chromium`）。
- `mise.toml` の `test-ui-browser` で `depends` に `install-ui-browser-chromium` を追加し、`run` から `playwright install chromium` 行を除去。
- `.github/workflows/ci.yml` の `ui-verify` job env に `PLAYWRIGHT_BROWSERS_PATH: ${{ github.workspace }}/.cache/ms-playwright` を追加。
- `.github/workflows/ci.yml` の `Install Playwright browser system deps (Linux)` の後に version/revision 算出・`actions/cache` restore・prepare・launch 検証の 4 step を挿入（key = `playwright-<os>-<arch>-pw<v>-chromium<rev>`、restore-keys なし、launch 検証 = `playwright screenshot` probe + 破損時 wipe+reinstall fallback）。
- `AGENTS.md` の検証入口テーブル（`install-ui-browser-chromium` 追加・`test-ui-browser` 更新）と ui-verify 記述を同期。
- `docs/designs/issue-327-cross-repo-ci-baseline.md` の Deferred 項目（Playwright cache 分類）を解決済みに更新。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-332-playwright-browser-cache.md`
- `docs/designs/issue-332-playwright-browser-cache.md`
- `docs/specs/issue-332-playwright-browser-cache.md`
- `docs/discovery/issue-332-playwright-browser-prep-inventory.md`

## Dependencies

- なし（先行タスクなし。#330 の install-ui-browser-test freshness 構造を前提とするのみ）。

## Affected Files

- `mise.toml`
- `.github/workflows/ci.yml`
- `AGENTS.md`
- `docs/designs/issue-327-cross-repo-ci-baseline.md`

## Done When

- [ ] `mise run install-ui-browser-chromium` が単独で `playwright install chromium` を実行する。
- [ ] `mise run test-ui-browser` が depends 経由で browser install を含み page build + suite が緑。
- [ ] ci.yml に cache restore・prepare・launch 検証 step があり、key が version/revision/OS-arch に紐づく。
- [ ] 擬似破損で launch 検証 → wipe → reinstall → 検証成功へ落ちる。
- [ ] ci.yml が actionlint 通過、AGENTS.md と実装が一致。
- [ ] Before/After が cold/warm 分離で記録される（CI run の step 計測 + job log の download 行）。

## Verification

- [ ] `mise run install-ui-browser-chromium`（install-ui-browser-test depends 経由 → install）。
- [ ] `mise run test-ui-browser` 単独で install 解決込み完走・30 spec 緑。
- [ ] `$PLAYWRIGHT_BROWSERS_PATH` 設定時に install/validate/test が同一 path を使う。
- [ ] browsers path 擬似破損 → launch probe fail → wipe+reinstall → probe pass。
- [ ] `mise exec -- actionlint .github/workflows/ci.yml`・`mise exec -- prek run --all-files` 緑。
- [ ] PR CI run で cold（cache miss）/warm（cache hit）の step 時間と download 有無が分離記録される。
