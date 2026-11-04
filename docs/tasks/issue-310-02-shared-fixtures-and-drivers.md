---
id: "TASK-034Rp7kQsMmyk0At2QYrwX"
type: "task"
status: "done"
title: "Issue #310-T2 共有 Fixture/Driver/Assertion (gesture/receipt/state/theme/canvas/a11y)"
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
    - "docs/tasks/issue-310-01-browser-harness-package.md"
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

# Issue #310-T2 共有 Fixture/Driver/Assertion (gesture/receipt/state/theme/canvas/a11y)

## Work

Node 側共有 Utility（Consumer が通常の Playwright API と併用する Fixture/Driver/Assertion）を実装する。

- `src/poll.ts`: `waitFor(fn, {timeout,intervals})` — `expect.poll` 相当の状態到達 poll。固定 sleep 禁止を内部で強制。
- `src/harness.ts`: `openHarness(page, pageName, {clock, viewport})` → `HarnessSession`（`api()` 経由の `__rvf` 呼出、`advanceClock(ms)`、`drawAll()`、`events()/commands()/receipts()/queries()`、`unmount/mount`、`ready` poll）。serve route は session open 時に install。
- `src/control.ts`: `control(session, selector)` → `beginDrag`/`dragTo`/`commitDrag`（pointer down/move/up の segment 指定）、`wheel(deltaY)`、`pressKey(key)`、`focus()`、`dblclick()`、typed `prop(name)`/`setProp(name,v)`、`attr(name)`。shadow-piercing locator を内部で解決。
- `src/events.ts`: `gestureEvents(session, controlId?)` の型付き読取 + `expectGestureSequence`（begin→preview{0..N}→終端 exactly-one・同一 gestureId・sequence 単調）+ `expectComposedInit`（detail/bubbles/composed 検査）。
- `src/receipts.ts`: `commands(session)`/`receipts(session)`/`drain(session, mode)`/`expectReceiptPath`（commit→pending→applied/accepted/rejected/unsupported の遷移と 1 回性）+ `noDuplicateCommands`。
- `src/states.ts`: `expectDisabled`/`expectReadOnly`/`expectPending`/`expectRejected`/`expectUnsupported`/`expectInvalid`（属性 + aria-* の一貫検査）。
- `src/theme.ts`: `setTheme(session, scopeSel, ThemeInput)`、`tokenValue(session, el, tokenName)`（getComputedStyle custom property）、`partExists(session, el, partName)`。
- `src/canvas.ts`: `canvasFillProbe(session, vizSel, region)`（fill ratio 行列）、`expectNonBlank`、`expectGapRegion`（NaN gap 中央帯 fill 低下）、`backendKind`/`expectBackendRecorded`、`drawAll`/`pumpViz`。
- `src/a11y.ts`: `expectRole`/`expectTabIndex`/`expectAriaNow`/`expectAccessible`（role=slider 等 + aria-valuemin/max/now/valuetext + Tab/keyboard 到達）。
- `src/fixtures.ts`: `test = base.extend` — `harness` fixture（serve install・`page.clock` install・openHarness・後始末）、`browserName`/`dpr` 派生 fixture。
- `src/evidence.ts`: reporter（`evidence.json` 書出し、env/revision/seed/tests[]）。
- `src/index.ts`: 上記 + `test`/`expect` re-export。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-310-browser-gui-test-harness.md`
- `docs/designs/issue-310-browser-gui-test-harness.md` §Components/§決定性/§Gesture・Receipt/§Canvas
- `docs/tasks/issue-310-01-browser-harness-package.md`（protocol・serve 前提）

## Dependencies

- T1 完了（protocol・serve・page-src が動作）。`@playwright/test` API（`test.extend`・`page.clock`・locator shadow piercing）。

## Affected Files

- `ui/browser-test/src/**`、`ui/browser-test/src/index.ts`、`ui/browser-test/playwright.config.ts`（reporter 配線）

## Done When

- [ ] 全 utility が export され `run typecheck` pass
- [ ] `test.extend` fixture で clock install + harness open が動作
- [ ] gesture/receipt/state/theme/canvas/a11y の各 utility が少なくとも 1 spec で利用される

## Verification

- [ ] `mise run check-ui-browser`
- [ ] T3 の spec 実装で各 utility が実際に機能（import エラー・型不整合なし）
