---
id: "SPEC-034Rp1AEJDKiDTpPq3H4Yz"
type: "spec"
status: "approved"
title: "Issue #310 framework-neutral Browser GUI Test Harness (Playwright・Showcase実DOM検証)"
created: "2026-11-03"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/310"
    - "https://github.com/scarecrow173/ruviform/issues/257"
  changes:
    added:
      - type: "file"
        path: "ui/browser-test/package.json"
      - type: "file"
        path: "ui/browser-test/package-lock.json"
      - type: "file"
        path: "ui/browser-test/tsconfig.json"
      - type: "file"
        path: "ui/browser-test/tsconfig.build.json"
      - type: "file"
        path: "ui/browser-test/playwright.config.ts"
      - type: "file"
        path: "ui/browser-test/README.md"
      - type: "dir"
        path: "ui/browser-test/src"
      - type: "dir"
        path: "ui/browser-test/page-src"
      - type: "dir"
        path: "ui/browser-test/pages"
      - type: "dir"
        path: "ui/browser-test/test"
      - type: "doc"
        path: "docs/designs/issue-310-browser-gui-test-harness.md"
      - type: "doc"
        path: "docs/plans/issue-310-browser-gui-test-harness.md"
      - type: "doc"
        path: "docs/tasks/issue-310-01-browser-harness-package.md"
      - type: "doc"
        path: "docs/tasks/issue-310-02-shared-fixtures-and-drivers.md"
      - type: "doc"
        path: "docs/tasks/issue-310-03-specs-ci-evidence.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "ui/.gitignore"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/issue-310-browser-gui-test-harness.md"
    - "../plans/issue-310-browser-gui-test-harness.md"
  depends-on:
    - "https://github.com/scarecrow173/ruviform/issues/258"
    - "https://github.com/scarecrow173/ruviform/issues/259"
    - "https://github.com/scarecrow173/ruviform/issues/260"
    - "https://github.com/scarecrow173/ruviform/issues/261"
    - "https://github.com/scarecrow173/ruviform/issues/263"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/311"
    - "https://github.com/scarecrow173/ruviform/issues/312"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/314"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  defers: []
  deferred-by: []
---

# Issue #310 framework-neutral Browser GUI Test Harness (Playwright・Showcase実DOM検証)

## What Should Be Built

GUI Testing v1.0 / RF-GT-01 の実装。既存 `ui/core`・`ui/components`・`ui/viz`・`ui/binding`・`ui/showcase` の headless / happy-dom 検証を置き換えず、**実ブラウザ DOM 上でのユーザー操作・表示・Accessibility** を継続検証する共有 Harness を新設する。

- Playwright (@playwright/test) を採用し、独自 E2E DSL は作らない。Consumer は通常の Playwright Test API と、Ruviform 提供の共通 Fixture / Assertion / Driver を利用する。
- 共有 Harness は framework-neutral な plain TypeScript。Ruviform 共通 UI / Host / Showcase / Runner の**必須依存に Playwright やフレームワーク（Solid/React/Vue 等）を追加しない** — `@ruviform/ui-browser-test` は private パッケージとして分離し、Playwright は devDependency（Consumer 側利用時は peerDependency）に閉じる。
- 検証対象は `ui/showcase` の実 DOM マウント（`mountShowcase` のテーマ行列、`mountDawFixture` / `mountAnalysisFixture` の Consumer-shaped fixture）と、dist ビルド配布面（`tsc` 出力の ES modules）をブラウザが実際にロードする経路。

## Why Now

- Issue #257 の横断設計で RF-GT-01 として合意済み（2026-11-04）。#258〜#263 の headless 資産は main にマージ済みで、次の未検証領域が「実ブラウザ DOM 上の操作・表示・a11y」。
- Synvelis（SV-GUI-01〜05）と AudioLens が共有 Harness を前提に Consumer 側 GUI 検証を設計中。#294（Cross-Consumer Gate）が本 Harness の証拠を入力にする。
- happy-dom は Canvas2D/WebGL2・実レイアウト・フォーカス/pointer capture を再現できず、Gesture 終端・Receipt 表示・stale/gap 描画の意味的検証には実ブラウザが必要。

## Users and Value

- **Synvelis / AudioLens（Consumer Owner）**: 製品側テストが Ruviform 提供の mount・Pointer/Keyboard/Gesture 終端・Receipt・Canvas/a11y の共通 Utility をそのまま使い、独自 E2E 基盤を持たずに済む。Consumer 真正シナリオ・Golden は各 Consumer が所有する。
- **Ruviform メンテナ**: Showcase/Components/Viz/Binding の実ブラウザ回帰を CI で継続検証できる。失敗時 Trace と環境・revision・seed 記録が #294 へ接続される。
- **後続 RF-GT-04/05**: 本 Harness の実 DOM 証拠を、OS WebView 実操作（#313）・Golden/Native 描画（#314）と区別して参照できる。

## Scope

### In Scope

- 新規 private パッケージ `ui/browser-test`（`@ruviform/ui-browser-test`）: Node 側 Utility（mount/page 起動・CustomEvent 収録・Gesture Driver・Receipt スクリプト・Canvas2D/WebGL2 smoke・a11y 確認・Theme/Shadow Part・状態属性・lifecycle）と、ページ側補助モジュール（`page-src/` → `dist/page/*.js`）。
- Serve 層: Playwright `page.route` による hermetic な静的ファイル serve（外部プロセス・ポート不要）。`ui/showcase/dist` の ES modules をブラウザへ供給。
- 検証ページ: showcase 行列・DAW-shaped fixture・analysis-shaped fixture・lifecycle（remount/reconnect）の各 HTML ページ — すべて `mountShowcase` / `mountDawFixture` / `mountAnalysisFixture` の既存公開 API のみを使用し、製品型を共通公開 API に持ち込まない。
- 実ブラウザ spec（Chromium 既定）: Gesture Begin/Preview/Commit/Cancel、keyboard/focus/a11y、Command/Receipt の重複・誤確定なし、property/event 反映、Theme 変更・Reference Light/Dark、disabled/readOnly/pending/rejected/unsupported、missing/NaN gap、stale/long-form、source revision/generation、resize/DPR、reconnect/remount/StrictMode 相当の重複防止、Canvas2D/WebGL2 smoke、Synvelis 形状・AudioLens 形状 fixture での同一 Harness 利用。
- 決定的実行基盤: 固定 Viewport/DPR・`page.clock` による wheel-idle 等の timer 決定化・seed 固定のデモデータ・状態到達を待つ wait（固定 sleep 禁止）・失敗時 Trace/スクリーンショット・`evidence.json`（環境・revision・seed・pass/fail/not-run）。
- mise タスク `check-ui-browser` / `build-ui-browser` / `test-ui-browser` と `ui-verify` CI ジョブへの browser lane 追加（ブラウザバイナリ cache 付き）。

### Out of Scope

- Desktop OS の実操作・実 WebView window（RF-GT-04 / #313）、Native GPU 実測、Golden/pixel 厳密比較・Native offscreen（RF-GT-05 / #314）。
- DAW/解析の Command 意味論・Project/Job 正本（Consumer 責務）、製品の真正 E2E シナリオと Golden（Consumer Owner）。
- IPC conformance/replay（RF-GT-02 / #311）、Secure Test Host Control（RF-GT-03 / #312）。
- React/Solid 等フレームワーク実装着の検証 fixture（UIF-02/03 は Consumer 側。Synvelis SolidJS fixture は Synvelis 側に配置）。
- `browser PASS` を OS WebView/Native GPU/製品 E2E PASS とみなすこと（証拠を別集計）。

## Requirements

- Playwright を第一候補として採用し、Consumer は通常の `test`/`expect` API と Ruviform 共有 Fixture を利用。独自 DSL・独自テストランナーを作らない。
- 共有 Fixture/Utility が少なくとも次をカバーすること: 汎用 mount/unmount、typed DOM property 読み書き、CustomEvent（detail/bubbles/composed）収録・検査、Theme/Shadow Part 補助、Pointer/Keyboard/Focus Driver、Gesture 終端（begin/preview/commit/cancel の exactly-one 終端）検査、Receipt 表示（pending/accepted/applied/rejected/unsupported）検査、Canvas2D/WebGL2 smoke、a11y（role/name/aria 状態/keyboard 到達）確認。
- Fixture は決定的: 固定 Viewport/deviceScaleFactor、`page.clock` による timer 制御、seed 固定の showcase デモデータ。固定 sleep を使わず、イベント/状態の到達を Node 側 poll で待つ。
- Screenshot は補助証拠（失敗時 + 明示オプトイン）とし、意味的 Oracle（DOM 属性・event log・canvas pixel 解析・gateway 記録）を置き換えない。
- 新規依存は `@playwright/test` とその transitive のみ。`ui/browser-test` の devDependency + peerDependency に閉じ、既存 `ui/*`・Rust ワークスペース・標準配布の必須依存に追加しない。依存追加理由を PR に記録。
- mise タスク・CI は既存 `ui/*` lane の規約（`npm --prefix … ci --no-audit --no-fund` → typecheck/test/build、lockfile 固定）に従う。ブラウザバイナリの取得は CI cache で再現可能にし、取得不能時のフォールバック手順を文書化。
- ブラウザ lane 失敗時の Trace（Playwright trace）・スクリーンショット・`evidence.json`（OS・Node・Playwright/browser 版・git revision・seed・各 spec の pass/fail/not-run）を artifact として記録。

## Acceptance Criteria

- [ ] 共通 Showcase 実 DOM で Gesture Begin/Preview/Commit/Cancel、keyboard/focus/a11y を実行し、Command/Receipt の重複・誤確定なし（exactly-one 終端 + receipt 経路の検査が spec で pass）。
- [ ] Web Components の property/event・remount/reconnect/Theme 変更で Source/Project 真値の改変・Subscription 重複/資源漏れなし（gateway の subscribe/unsubscribe・received 記録で検査）。
- [ ] Canvas/WebGL 描画・missing/NaN gap・stale/long-form・viewport の意味的検証と artifact が再現可能（status part・event・pixel probe が一致）。
- [ ] Synvelis 形状（`mountDawFixture`）・AudioLens 形状（`mountAnalysisFixture`）の別 fixture で同じ Harness が利用可能。Ruviform に React/Solid 等の必須依存なし（deps 監査 spec で検査）。
- [ ] PR CI の実走 pass/fail/not-run 証拠、環境・revision・seed・失敗時 Trace を記録し #294 へ接続（`evidence.json` + CI artifact）。

## Deferred Design Concerns

- WebKit/Firefox project の CI 常設: 依存 binary 増・時間増のため Chromium を必須 lane とし、他 engine は `RVF_BROWSERS` 環境変数の opt-in。必要になれば #313/#314 側の matrix 拡張で再検討。
- 実 OS WebView driver との共有境界: RF-GT-04 確定後、Harness API のうち OS 中立部分の再利用可否を #313 で見直す。
- Visual Golden/pixel 厳密比較: RF-GT-05 が Owner。本 Harness の pixel probe は smoke/意味的検証に限定。

## Sources and Relations

- 正本: GitHub Issue #310（Acceptance・境界は Issue 本文優先）。
- 設計 Owner: Issue #257 §10（UIC gates）・§13.5（UIF gates）・RF-GT 索引（2026-11-04）。
- 既存実装: #258 ui/core、#259 ui/components、#260 ui/viz、#261 ui/binding、#263 ui/showcase（`mountShowcase`/`mountDawFixture`/`mountAnalysisFixture`・demo gateway/backend 再利用）。
- Cross-Consumer: #219 / #294（本 lane の証拠を Gate 入力として接続）。
