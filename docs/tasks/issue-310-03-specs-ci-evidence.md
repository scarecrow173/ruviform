---
id: "TASK-034Rp7lRJbB15NEGHdCXpU"
type: "task"
status: "done"
title: "Issue #310-T3 実ブラウザ spec・Consumer-shaped fixture・CI/evidence 接続"
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
    - "docs/tasks/issue-310-02-shared-fixtures-and-drivers.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-310-browser-gui-test-harness.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #310-T3 実ブラウザ spec・Consumer-shaped fixture・CI/evidence 接続

## Work

実ブラウザ spec 群と mise/CI/evidence 接続を実装する。

- `test/gesture.spec.ts`: fader/knob/range/seekbar の drag begin→preview→commit exactly-one、Escape cancel、pointercancel/lostcapture cancel、wheel burst（`clock.runFor` で idle commit）、dblclick reset。
- `test/keyboard-a11y.spec.ts`: Tab/click focus、Arrow/PageUp/Home/End、role=slider + aria-valuemin/max/now/valuetext、disabled/readOnly で操作不活化。
- `test/receipts-states.spec.ts`: commit→`pending`→drain apply→`applied`+value 反映+revision +1、reject→`invalid`+revert、unsupported→`unsupported` 属性、pending 中の receipt 未到達での誤確定なし、delayed/reordered receipt、transport intent の receipt。
- `test/theme-lifecycle.spec.ts`: Reference Light/Dark + 変更 Theme で `rvf-theme` scope の token 変化（Source 真値不変）、remount/reconnect で subscribeCalls 二重化なし・commands 重複なし・StrictMode 相当の detach/attach 往復で leak なし（`unsubscribed` 対 `subscribeCalls` 均衡）。
- `test/viz.spec.ts`: `drawAll` 後の Canvas2D fill probe（非空白・`stats.framesDrawn`）、NaN gap 中央帯 fill 低下、`MemoryWaveformSource.push` revision bump → `stale` status、`setSize` で `queries()` に新 buckets の bounded query、DPR context、`backendKind`/`rvf-backend` capability 記録、meter seq gap → stale/dropped status。
- `test/consumers.spec.ts`: `mountDawFixture`（fader/transport/automation の gesture+receipt+subscribe）と `mountAnalysisFixture`（waveform/spectrum + QueryBinding の bounded query/resolve/stale/cancel）で同一 Harness API が利用可能。
- `test/deps.spec.ts`: Ruviform 全 package.json に react/solid/vue/svelte・実 DOM renderer の必須依存がないこと、`ui/browser-test` 以外に `@playwright/test` がないことを検査。
- `mise.toml` に `check-ui-browser`/`build-ui-browser`/`test-ui-browser`（showcase build 内包）。
- `.github/workflows/ci.yml` `ui-verify` に browser lane: ms-playwright cache（OS+lockfile hash）→ `npx playwright install chromium`（Linux は `--with-deps`）→ `mise run test-ui-browser` → `test-results/` artifact upload（`if: always()`）。
- `AGENTS.md` 検証入口表に 3 task 追記 + browser lane 暫定規約。
- `ui/browser-test/README.md`: 目的・依存理由・serve/clock/seed の決定性・CI 導入手順・browser 未 install 時の not-run 扱い・Consumer 利用例。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-310-browser-gui-test-harness.md`
- `docs/test-specs/issue-310-browser-gui-test-harness.md`
- `docs/tasks/issue-310-02-shared-fixtures-and-drivers.md`

## Dependencies

- T2 完了（全 utility 実装済み）。`ui/showcase` dist build が前提。

## Affected Files

- `ui/browser-test/test/**`、`ui/browser-test/README.md`、`mise.toml`、`.github/workflows/ci.yml`、`AGENTS.md`

## Done When

- [ ] 全 spec が `mise run test-ui-browser`（chromium）で pass
- [ ] `test-results/evidence.json` が生成され env/revision/seed/pass-fail を含む
- [ ] mise タスク 3 本が `mise run` で実行可能・CI lane が workflow に記述済み
- [ ] `deps.spec.ts` で React/Solid 等の必須依存なしを検証

## Verification

- [ ] `mise run check-ui-browser` / `build-ui-browser` / `test-ui-browser` 全 pass
- [ ] `mise run check-ui-showcase` / `test-ui-showcase` 非回帰
- [ ] CI workflow diff が artifact upload + cache を含む
