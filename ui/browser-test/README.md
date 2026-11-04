# @ruviform/ui-browser-test

Issue #310 (RF-GT-01) の framework-neutral 実ブラウザ GUI 検証ハーネス。
Playwright が実 Chromium DOM 上で `ui/components` / `ui/viz` / `ui/binding` /
`ui/showcase` の Web Components を駆動し、gesture・keyboard/focus/a11y・
receipt・theme・canvas 描画・query/viewport・lifecycle の契約を検証する。

## 構成

- `pages/` — 決定的 fixture HTML(showcase / DAW-shaped / analysis-shaped /
  lifecycle)。import map で `@rvf/*` を workspace の `dist/` 出力へ解決する。
- `page/` — ページ側 `window.__rvf` 実装(`tsc -p tsconfig.build.json` で
  `dist/page/` へ emit)。
- `src/` — Node 側の session/driver/assertion/reporter:
  - `harness.ts` — Hermetic route serve(`http://rvf.test`、workspace dist の
    み fulfill)+ 決定的 clock/seed/viewport の session。
  - `control.ts` — shadow part 越しの pointer/wheel/keyboard/focus driver。
  - `events.ts` / `receipts.ts` — gesture/receipt log の意味的 oracle。
  - `states.ts` / `theme.ts` / `a11y.ts` — attribute/ARIA/token/a11y 検証。
  - `canvas.ts` — Canvas2D pixel probe(色 coverage/affinity、plot 空間)。
  - `evidence.ts` — `test-results/evidence.json` reporter(lane="browser"、
    env/git revision/fixed clock/seed/各テスト outcome を記録)。
- `test/` — 契約 spec(gesture/states/theme/DAW/analysis/lifecycle/viz)。

## 実行

```sh
mise run check-ui-browser-test   # 型検査(ui/showcase dist を前提)
mise run test-ui-browser         # page build + Chromium install + 全 spec
```

Linux で初回のみ system ライブラリが必要:
`npm --prefix ui/browser-test exec -- playwright install-deps chromium`。

## 証拠の扱い

`test-results/evidence.json`・trace・screenshot は CI で
`ui-browser-evidence-<os>` artifact として保存される。**browser lane の
PASS は OS WebView / native GPU / 製品 E2E の PASS ではない**
(それぞれ RF-GT-04 #313 / RF-GT-05 #314 の lane)。Cross-consumer gate は
Issue #294。

## 依存方針

`@playwright/test` は peer + devDependency のテスト専用。production
runtime・標準 UI 配布物の必須依存には含めない。UI framework 非依存の
plain TypeScript を維持する。
