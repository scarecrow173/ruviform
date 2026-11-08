---
id: "DESIGN-034Rp1KqXNzTDTKRXTbqHx"
type: "design"
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
    - "../specs/issue-310-browser-gui-test-harness.md"
  implemented-by:
    - "../plans/issue-310-browser-gui-test-harness.md"
  depends-on:
    - "https://github.com/scarecrow173/ruviform/issues/257"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/issue-310-browser-gui-test-harness.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #310 framework-neutral Browser GUI Test Harness (Playwright・Showcase実DOM検証)

> **歴史/旧方式（legacy）**: 本書は Issue #310 当時の WebView/DOM 前提
> GUI テスト基盤（`ui/browser-test`・Playwright 実ブラウザ検証）の記録である。
> 現行の GUI 検証標準は Driver-neutral 契約 + GPUI 標準 Driver（Issue #384）と
> driver-lab の Fake/GPUI parity 検証（`check-ui-driver-boundary` /
> `test-ui-driver-contract` / `test-ui-parity`）。当該 lane は legacy/preview
> として保持され、物理撤去は Issue #465 P2 の別判断（gate: #419 採用判定 +
> consumer Native 受入）。棚卸し正本:
> [issue-465-legacy-webview-inventory](../discovery/issue-465-legacy-webview-inventory.md)。

## Context

#258〜#263 で headless（happy-dom / node --test）層は main にマージ済み。残る未検証領域は**実ブラウザ DOM** — 実 Custom Elements・Shadow DOM・pointer capture・focus・Canvas2D/WebGL2・レイアウト/resize 上での Gesture 終端・Receipt 表示・stale/gap 描画。RF-GT-01（本 Issue）がその共有 Harness を実装する。

設計上の制約（Issue #257・#310 本文）:

- framework-neutral: Solid/React/Vue を必須依存にしない。Harness 自体も plain TS。
- 独自 E2E DSL 禁止: Playwright の `test`/`expect` をそのまま使い、Ruviform 側は Fixture/Utility のみ提供。
- Playwright は本番 Runtime・標準 UI 配布の必須依存に入れない → 独立 private パッケージ化。
- browser PASS ≠ OS WebView/Native GPU/製品 E2E PASS。証拠を別集計。

## Scope

### In Scope

- `ui/browser-test` パッケージ一式（serve・session・drivers・assertions・page-src・pages・spec・config・reporter）。
- mise タスク 3 本と `ui-verify` への 3 OS browser lane 追加。
- `evidence.json` + Playwright trace の artifact 記録。

### Out of Scope

- RF-GT-02〜05 の各 lane（IPC replay / Test Host Control / OS Driver / Visual Golden）。
- Consumer 製品 fixture・真正シナリオ・Golden。
- WebKit/Firefox の必須 lane 化（opt-in のみ）。

## Components and Boundaries

### パッケージ構成 `ui/browser-test/`（`@ruviform/ui-browser-test`, private, `"type": "module"`）

```text
package.json / package-lock.json    @playwright/test + typescript + @types/node のみ
tsconfig.json                       typecheck 対象 = src + page-src + test + playwright.config.ts
tsconfig.build.json                 page-src/*.ts → dist/page/*.js（browser で serve するモジュール）
playwright.config.ts                固定 viewport/DPR・trace retain-on-failure・evidence reporter
README.md                           lane 説明・依存理由・CI 導入手順・Consumer 利用手順
src/                                Node 側公開 API（Consumer も import 可能、TS source 配布）
  index.ts      公開面エクスポート
  serve.ts      page.route による hermetic 静的 serve（外部 process/port 不要）
  harness.ts    openHarness → session { page, pageApi 呼出し, advanceClock, drawAll, close }
  poll.ts       状態到達 poll（expect.poll 薄 wrapper + waitForCondition）
  control.ts    ControlHandle: pointer drag / wheel / keyboard / focus / dblclick drivers + typed property
  events.ts     rvf-* CustomEvent 記録の読取 + Gesture 終端 exactly-one・CustomEvent init 検査
  receipts.ts   gateway.received/receipts の読取 + drain(apply/accept/reject/unsupported) + 検査
  states.ts     disabled/readOnly/pending/rejected/unsupported/invalid の属性・ARIA 検査
  theme.ts      rvf-theme.theme 書換 + token computed style 取得 + ::part 検査
  canvas.ts     canvas pixel probe（fill ratio / gap 判定）+ WebGL2 capability + drawNow/pumpNow 呼出
  a11y.ts       role/tabindex/aria-valuenow/keyboard 到達性の検査
  fixtures.ts   test.extend の共通 fixture（harness page open + clock install）
  evidence.ts   環境・revision・seed・pass/fail/not-run を evidence.json へ書く reporter
page-src/                           ブラウザ内で動く TS モジュール（build して serve）
  rvf-page.ts   installRvfPage(window): rvf-* event 記録 + gateway expose + query log + drawAll
  showcase.ts   mountShowcase + demo pump + expose
  daw.ts        mountDawFixture + expose（Synvelis-shaped）
  analysis.ts   mountAnalysisFixture + expose（AudioLens-shaped）
  lifecycle.ts  remount/reconnect 検証用マウント操作 expose
pages/                              serve される HTML（import map で dist module を解決）
  showcase.html / daw.html / analysis.html / lifecycle.html
test/                               Playwright spec（既定 chromium）
  gesture.spec.ts / keyboard-a11y.spec.ts / receipts-states.spec.ts
  theme-lifecycle.spec.ts / viz.spec.ts / consumers.spec.ts / deps.spec.ts
```

### 依存境界

- `ui/browser-test` → devDependency `@playwright/test`・`typescript`・`@types/node`。`peerDependencies` にも `@playwright/test` を宣言し、Consumer が自身の Playwright バージョンで利用できる形にする（lockfile 固定は本 repo の devDependency 側で担保）。
- 他 `ui/*` パッケージから `ui/browser-test` への依存は禁止（逆方向のみ）。Rust ワークスペース・`ui/showcase` 配布面・`ruviform-ui-webview` の必須依存には何も追加しない。
- page-src は `ui/showcase` の **dist ビルド**（`mise run build-ui-showcase` の出力）を import map 経由でロードする。TS source をブラウザへ直接供給しない — 配布と同じ `.js` 面を検証するため。
- Node 側 src は `page-src` を import しない（process 境界）。`window.__rvf` の protocol 型は `src/protocol.ts` に宣言し page-src 側も実装する（片方向契約、import 共有なし）。

### Serve 層（`src/serve.ts`）

`webServer` プロセスを使わず `page.route("**/…")` で request を interception し、workspace 上のファイルを `route.fulfill` で返す。

- URL 空間: 固定 origin `http://rvf.test`。`/pages/*.html` → `ui/browser-test/pages/`、`/browser-test/dist/` → `ui/browser-test/dist/`、`/showcase/dist/` → `ui/showcase/dist/`。
- MIME: `.js`/`.mjs` → `text/javascript`（module script 要件）、`.html` → `text/html`、`.json` → `application/json`、その他 404。`..` 脱出は拒否。
- 利点: ポート衝突なし・外部 process なし・CI で追加 daemon 不要・完全 hermetic。

### ページ内 protocol（`window.__rvf`）

各ページ module は mount 後に `installRvfPage` を呼び、`window.__rvf` に次を expose する（Node 側は `page.evaluate` で読み書き）:

```ts
interface RvfPageApi {
  readonly ready: boolean;
  // rvf-gesture/rvf-transport/rvf-viewport/rvf-select/rvf-backend を
  // document レベルで収録（bubbles+composed で shadow 越境済み）
  events(type?: string): RecordedRvfEvent[];   // {type, detail(jsonable), controlId, seq}
  commands(): unknown[];                       // gateway.received の intent+ctx
  receipts(): unknown[];                       // deliver 済み receipt の記録
  drain(mode: "apply" | "accept" | "reject" | "unsupported"): number;
  queries(): unknown[];                        // waveform source query log（bounded LOD 検証用）
  drawAll(): void;                             // viz pumpNow+drawNow の決定的 tick
  stats(): Record<string, unknown>;            // viz stats/backendKind/status text
  prop(sel: string, name: string): unknown;    // typed property 読取（control/value/descriptor 等）
  setProp(sel: string, name: string, v: unknown): void;
  mount(): void;                               // lifecycle ページ用: unmount→mount
  unmount(): void;
  clear(): void;
}
```

- receipt は fixture が script する（auto-drain しない）— delay/reorder/reject を決定的に再現するため。
- `queries()` はページ側で `MemoryWaveformSource.query` を wrap して記録 — bounded query（buckets≈plot px）・resize 再 query・stale 破棄の意味的 oracle。

### 決定性の担保

| 要素 | 手段 |
| --- | --- |
| Viewport/DPR | `use.viewport={960,640}` + `deviceScaleFactor:1`（DPR 試験は専用 context `deviceScaleFactor:2`） |
| Clock | `page.clock.install({now: 固定})` を fixture で install。wheel-idle(200ms)等 timer は `clock.runFor` で進める |
| Seed | showcase demo data は決定的（`makeDemo*` 固定）。ページ URL query で seed 明示・evidence へ記録 |
| 待機 | 固定 sleep 禁止。`expect.poll` / Node 側 evaluate loop で状態到達を待つ。`drawAll()` で viz を同期 tick し rAF 依存を排除 |
| rAF | page.clock で rAF も仮想化されるため、viz frame は `__rvf.drawAll()` で pumpNow+drawNow を直接呼ぶ |

### Gesture / Receipt の意味的検査（`events.ts` + `receipts.ts`）

- pointer drag → `rvf-gesture` 列が `begin, preview{0..N}, commit|cancel` で **終端は exactly one**・同一 gestureId・sequence 単調を検査。
- commit 後 `pending` 属性が立ち、`drain("apply")` で receipt `applied` → pending 解消・controlled `value` が commit 値へ・gateway revision が +1。
- reject/unsupported で `invalid`/`unsupported` 反映 + `revertOnReject` の復帰を検査。
- StrictMode 相当: detach→attach・remove→append 後も subscribeCalls 増加が +0（対象 target への重複購読なし）、remount 後の再 drag で commands が 2 重発行されない。
- transport: `rvf-transport` detail `{action, controlId, sequence}` がクリックで 1 件・remount 後も二重 dispatch なし。

### Canvas / stale / gap の意味的検査（`canvas.ts`）

- Canvas2D smoke: `drawAll` 後に plot canvas の `getImageData` で fill-ratio を計測 → 描画が空白でないこと・`stats.framesDrawn>0`・`lastOpCount>0`。
- WebGL2 smoke: `rvf-backend` event/`backendKind` で capability を記録。WebGL2 利用不可環境では「取得不可」を証拠化（Canvas2D fallback の recorded reason を検査）— 失敗ではなく観測値として扱う（browser PASS を GPU PASS にしない）。
- NaN gap: デモ波形 ch0 の 0.45–0.55 gap 区間で lane 中央帯の fill-ratio が非 gap 区間より有意に低い（0 fill 補間ではなく明示 gap）ことを pixel probe で検査。
- stale/long-form: `MemoryWaveformSource.push` で revision bump → view が `stale` status+overlay を出す → `queries()` に再 query が記録される。`setSize` で plot px が変わると buckets も変わる（bounded LOD）。
- DPR/resize: `page.setViewportSize` 変化 → `queries()` に新 resolution の query が追加記録。

### Evidence（`src/evidence.ts` + config）

- 独自 reporter（list 併用）が `test-results/evidence.json` に `{ env: {os, node, playwright, browsers[]}, git: revision, seed, startedAt, tests: [{title, status(pass/fail/skipped/not-run), durationMs, errors[]}] }` を出力。
- `trace: "retain-on-failure"`・`screenshot: "only-on-failure"`・`video` なし（screenshot は補助証拠）。
- CI で `ui/browser-test/test-results/` を artifact upload（`if: always()`）。

## Data and Control Flow

```text
mise run test-ui-browser
 ├─ npm --prefix ui/showcase ci + run build     (dist を serve 対象として生成)
 ├─ npm --prefix ui/browser-test ci + run build (page-src → dist/page)
 └─ npm --prefix ui/browser-test run test:browser
      └─ playwright test
           ├─ fixture: context(viewport/DPR/clock 固定) → page
           ├─ serve: page.route で /pages, /showcase/dist, /browser-test/dist を fulfill
           ├─ goto /pages/<name>.html → page-src module が showcase dist を import
           │    → mountShowcase/mountDawFixture/mountAnalysisFixture → __rvf ready
           ├─ spec: Playwright mouse/keyboard → DOM events → rvf-* intent
           │    → binding → gateway.received（__rvf.commands）
           │    → spec が __rvf.drain で receipt script → binding.ack → 属性/ARIA/表示
           └─ reporter: evidence.json + trace（失敗時）
```

## Non-Functional Constraints

- **再現性**: 全入力（viewport/DPR/clock/seed/テーマ）が決定的。失敗は trace+evidence で再現可能。
- **分離**: browser lane の失敗は browser lane の失敗（headless/OS/GPU と別集計）。
- **hermetic**: 外部 server・network・OS 状態に依存しない（Playwright binary 取得のみ network）。
- **YAGNI**: 共有 Utility は Issue が要求する項目のみ。Consumer 製品意味論・Golden・複数 engine 必須化は入れない。

## Risks and Trade-offs

| リスク | 対応 |
| --- | --- |
| `page.clock` と内部 rAF/ResizeObserver の干渉 | viz は `drawAll()` 同期 tick に限定。RO は browser 内部で clock 非依存。 |
| Playwright binary 未取得環境 | `test-ui-browser` は `playwright install chromium` を前置し、不能時は手順書で「実ブラウザ未検証」と明記（本番検証ゲート化しない）。 |
| page.route serve と module の相対 import | import map で `@rvf/showcase`→`/showcase/dist/...` を解決。page-src は `paths` で TS source に型解決し、emit された .js は bare specifier のまま import map に任せる。 |
| CI 時間増（3 OS × browser install） | `~/.cache/ms-playwright`/`%LOCALAPPDATA%\ms-playwright` を lockfile hash key で cache。chromium のみ。 |
| showcase dist 未生成で test 実行 | `test-ui-browser` が showcase build を内包し、dist 欠落時は明示エラー。 |

## Deferred Design Concerns

- WebKit/Firefox project 常設・`--with-deps` 相当の system deps 管理 — #313 の OS driver 設計時に再検討。
- `page.clock` と併用不能な将来ページ（rAF 依存が強い描画ループ）への対応 — 必要時に `useClock:false` opt-out で拡張。
- OS WebView lane との Harness API 共有 — RF-GT-04 の host 抽象が固まってから再利用を判断。

## Verification Notes

- [ ] `mise run check-ui-browser`（tsc --noEmit）が lockfile 復元後に pass。
- [ ] `mise run test-ui-browser` がローカル Chromium で全 spec pass（evidence.json 生成）。
- [ ] `mise run build-ui-browser` が page-src → dist/page を emit し、serve 経路が機能。
- [ ] 既存 lane 非回帰: `mise run check-ui-showcase`/`test-ui-showcase` 等は未変更で pass。
- [ ] CI `ui-verify` で browser lane の pass/fail/not-run 証拠と artifact upload が記録される。
