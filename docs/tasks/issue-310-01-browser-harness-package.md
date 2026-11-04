---
id: "TASK-034Rp7jV966SX0fG78cOBL"
type: "task"
status: "done"
title: "Issue #310-T1 browser-test パッケージ骨格・serve・page protocol"
created: "2026-11-03"
updated: "2026-11-03"
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
    - "docs/plans/issue-310-browser-gui-test-harness.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-310-browser-gui-test-harness.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #310-T1 browser-test パッケージ骨格・serve・page protocol

## Work

`ui/browser-test`（`@ruviform/ui-browser-test`, private, `"type": "module"`）の骨格を新設する。

- `package.json`/`package-lock.json`: devDeps=`@playwright/test`(固定)・`typescript`・`@types/node`、peerDeps=`@playwright/test`。scripts=`typecheck`/`build`/`test:browser`。
- `tsconfig.json`（noEmit・DOM lib・verbatimModuleSyntax・allowImportingTsExtensions・paths で `@rvf/showcase`→`../showcase/src/index.ts`）と `tsconfig.build.json`（`page-src/`→`dist/page/` emit）。
- `playwright.config.ts`: chromium 既定 project、固定 viewport `{960,640}`+`deviceScaleFactor:1`、trace `retain-on-failure`、screenshot `only-on-failure`、`test-results/` 出力、evidence reporter 配線。
- `src/protocol.ts`: `window.__rvf` page API 型（events/commands/receipts/drain/queries/drawAll/stats/prop/setProp/mount/unmount/clear/ready）。
- `src/serve.ts`: `page.route` hermetic 静的 serve（origin `http://rvf.test`、`/pages`・`/showcase/dist`・`/browser-test/dist` マッピング、MIME・`..` 拒否）。
- `page-src/rvf-page.ts`: `installRvfPage` — rvf-* event 記録（detail・bubbles/composed 記録）・gateway received/receipt expose・waveform query log・drawAll・ready flag。
- `page-src/{showcase,daw,analysis,lifecycle}.ts` + `pages/*.html`（import map `@rvf/showcase`→`/showcase/dist/showcase/src/index.js`、`@rvf/browser-test-page`→`/browser-test/dist/page/rvf-page.js`）。
- `ui/.gitignore` に `test-results/`・`playwright-report/`・`blob-report/` 追加。
- smoke spec（`test/smoke.spec.ts`）で serve→goto→`__rvf.ready`→`mountShowcase` DOM 確認まで疎通。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-310-browser-gui-test-harness.md`
- `docs/designs/issue-310-browser-gui-test-harness.md` §Components/§Serve/§ページ内 protocol/§決定性

## Dependencies

- なし（最初の task）。`ui/showcase` の `mountShowcase`/`mountDawFixture`/`mountAnalysisFixture` を利用するため `mise run build-ui-showcase` で dist 生成が前提。

## Affected Files

- `ui/browser-test/**`（新規）、`ui/.gitignore`

## Done When

- [ ] `npm --prefix ui/browser-test ci` で lockfile 復元、`run typecheck` が pass
- [ ] `run build` で `dist/page/*.js` が emit される
- [ ] smoke spec が chromium で pass（serve→ready→DOM）

## Verification

- [ ] `mise run check-ui-browser`
- [ ] `mise run build-ui-browser`（先に `mise run build-ui-showcase`）
- [ ] smoke spec 単体実行 pass（`npx playwright test test/smoke.spec.ts`）
