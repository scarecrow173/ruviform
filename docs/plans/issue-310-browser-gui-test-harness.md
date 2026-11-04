---
id: "PLAN-034Rp5NNiOYZYrwCt2kHQ7"
type: "plan"
status: "in-progress"
title: "Issue #310 Browser GUI Test Harness 実装計画"
created: "2026-11-03"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/310"
  changes:
    added:
      - "ui/browser-test/package.json"
      - "ui/browser-test/package-lock.json"
      - "ui/browser-test/tsconfig.json"
      - "ui/browser-test/tsconfig.build.json"
      - "ui/browser-test/playwright.config.ts"
      - "ui/browser-test/README.md"
      - "ui/browser-test/src"
      - "ui/browser-test/page-src"
      - "ui/browser-test/pages"
      - "ui/browser-test/test"
    modified:
      - "mise.toml"
      - ".github/workflows/ci.yml"
      - "ui/.gitignore"
      - "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-310-browser-gui-test-harness.md"
  implemented-by:
    - "docs/tasks/issue-310-01-browser-harness-package.md"
    - "docs/tasks/issue-310-02-shared-fixtures-and-drivers.md"
    - "docs/tasks/issue-310-03-specs-ci-evidence.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-310-browser-gui-test-harness.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-310-browser-gui-test-harness.md"
    - "docs/designs/issue-310-browser-gui-test-harness.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-310-browser-gui-test-harness.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #310 Browser GUI Test Harness 実装計画 Implementation Plan

## Goal

`ui/browser-test`（`@ruviform/ui-browser-test`, private）を新設し、Playwright 実ブラウザ上で Showcase 実 DOM の Gesture/Receipt/Theme/Viz/a11y/lifecycle を決定的に検証する共有 Harness と、mise・CI への接続を一気に実装する。

## Scope

単一実装 stream。境界: `ui/browser-test/` 新規 + `mise.toml`/`ci.yml`/`ui/.gitignore`/`AGENTS.md` の lane 追記のみ。既存 `ui/*` の source・build・test は一切変更しない。docs（spec/design/plan/test-spec/task/ir）は別 commit。

## Upstream Documents

- `docs/specs/issue-310-browser-gui-test-harness.md`（approved）
- Issue #310 本文（Acceptance 正本）、Issue #257（設計 Owner）

## Design Inputs

- `docs/designs/issue-310-browser-gui-test-harness.md`（approved）— 構成・依存境界・serve・protocol・決定性の全契約は design 正本に従う。

## File Map

| Path | Responsibility |
| --- | --- |
| `ui/browser-test/package.json` / `package-lock.json` | private module、devDeps=`@playwright/test`/`typescript`/`@types/node`、peerDeps=`@playwright/test`。scripts: `typecheck`/`build`/`test:browser` |
| `ui/browser-test/tsconfig.json` | `tsc --noEmit`（src+page-src+test+config、DOM lib + node types） |
| `ui/browser-test/tsconfig.build.json` | `page-src/*.ts` → `dist/page/*.js` |
| `ui/browser-test/playwright.config.ts` | chromium project・固定 viewport/DPR・`retain-on-failure` trace・evidence reporter・`test-results/` 出力 |
| `ui/browser-test/src/protocol.ts` | `window.__rvf` page API の型宣言（Node/page 共有契約） |
| `ui/browser-test/src/serve.ts` | `page.route` hermetic 静的 serve |
| `ui/browser-test/src/harness.ts` | `openHarness(page, name, opts)` → session（goto・ready poll・api 呼出・advanceClock） |
| `ui/browser-test/src/poll.ts` | `expect.poll` 薄 wrapper `waitFor` |
| `ui/browser-test/src/control.ts` | ControlHandle（drag/wheel/key/focus/dblclick/property） |
| `ui/browser-test/src/events.ts` | `rvf-*` event 記録読取・終端 exactly-one・init 検査 |
| `ui/browser-test/src/receipts.ts` | commands/receipts 読取・drain・receipt 状態遷移検査 |
| `ui/browser-test/src/states.ts` | disabled/readOnly/pending/rejected/unsupported/invalid 検査 |
| `ui/browser-test/src/theme.ts` | theme 書換・token computed style・part 検査 |
| `ui/browser-test/src/canvas.ts` | pixel probe（fillRatio/gap）・backend kind・drawAll |
| `ui/browser-test/src/a11y.ts` | role/tabindex/aria・keyboard 到達性 |
| `ui/browser-test/src/fixtures.ts` | `test.extend`（harness fixture・clock install） |
| `ui/browser-test/src/evidence.ts` | `evidence.json` reporter |
| `ui/browser-test/src/index.ts` | 公開面 export |
| `ui/browser-test/page-src/rvf-page.ts` | `installRvfPage`（event 記録・gateway expose・query log・drawAll） |
| `ui/browser-test/page-src/{showcase,daw,analysis,lifecycle}.ts` | 各ページ mount + expose |
| `ui/browser-test/pages/{showcase,daw,analysis,lifecycle}.html` | import map + module script |
| `ui/browser-test/test/*.spec.ts` | 実ブラウザ spec 7 本 |
| `mise.toml` | `check-ui-browser`/`build-ui-browser`/`test-ui-browser` |
| `.github/workflows/ci.yml` | `ui-verify` に playwright install(cache)+test-ui-browser+artifact upload |
| `ui/.gitignore` | `test-results/`・`playwright-report/`・`blob-report/` |
| `AGENTS.md` | lane 表 + 暫定規約追記 |

## Implementation Sequence

- [ ] T1: package 骨格（package.json/tsconfig×2/config/gitignore/README）+ `serve.ts` + `protocol.ts` + `page-src/rvf-page.ts` + ページ 4 枚 + smoke spec で serve→mount→ready 疎通
- [ ] T2: Node 側 utility 一式（harness/poll/control/events/receipts/states/theme/canvas/a11y/fixtures/evidence）+ index export
- [ ] T3: spec 7 本実装・mise タスク・CI lane・evidence・`mise run test-ui-browser` 実走 green・deps 監査 spec

## Dependencies and Risks

- `@playwright/test` の追加は新規依存（理由: Issue が Playwright 第一候補を指定、独自 DSL 禁止。dev+peer に閉じる旨 PR に記録）。
- Playwright browser binary は `npx playwright install chromium` が network 必須。CI は cache、ローカル未導入環境では lane は not-run と明記。
- `page.clock` と rAF/RO の干渉リスク → viz は `__rvf.drawAll()` 同期 tick に限定（design §決定性）。
- showcase `dist` 未生成で spec が 404 → `test-ui-browser` が `build-ui-showcase` 相当を内包。

## Task Breakdown

- `docs/tasks/issue-310-01-browser-harness-package.md`（T1）
- `docs/tasks/issue-310-02-shared-fixtures-and-drivers.md`（T2、T1 依存）
- `docs/tasks/issue-310-03-specs-ci-evidence.md`（T3、T2 依存）

## Verification

- [ ] `mise run check-ui-browser` — tsc --noEmit pass
- [ ] `mise run build-ui-browser` — page-src→dist/page emit
- [ ] `mise run test-ui-browser` — Chromium 全 spec pass + `ui/browser-test/test-results/evidence.json` 生成
- [ ] `mise run check-ui-showcase` / `test-ui-showcase` — 非回帰
- [ ] CI `ui-verify` browser lane: install→test→artifact upload の記述が workflow に存在
- [ ] `deps.spec.ts`: Ruviform package.json に React/Solid/実 DOM renderer の必須依存がないことを検査

## Open Questions / Gaps

- CI での Playwright ブラウザ DL 可否は本 env では未検証 → workflow は cache 付き標準手順で記述し、初回実行の証拠は PR の実走で確認する（未取得なら lane を not-run として記録するフォールバック手順を README に明記）。
- WebGL2 は headless Chromium の SwiftShader で取得可能な見込みだが、未取得環境では「capability なし」を証拠化する設計（design §canvas）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatched worker、subagent 不使用の指示）。
- Independent work streams: none（T1→T2→T3 直列）。
- User approval required before dispatch: yes（Issue 承認済み前提で実施）。
- If approved: T1→T2→T3 を task 単位で commit。
- If declined or unavailable: 同一 dependency graph で inline 実行。

## Review Handoff

- 重点: page.protocol（`window.__rvf`）と drain の receipt scripting が「固定 sleep なし・決定的」要件を満たすか。`page.clock` 方針（design §決定性）。
- Consumer 利用時の型公開面（`src/index.ts`）が最小か。
