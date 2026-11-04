---
id: "DISC-k7Pjkn8XoebpncwaTSISdf"
type: "discovery"
status: "resolved"
title: "Issue #332 CI-OPT-05 Playwright Chromium 準備・計測分離の棚卸し"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/332"
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/ruviform/issues/330"
    - "https://github.com/scarecrow173/ruviform/issues/310"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-310-browser-gui-test-harness.md"
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
    - "docs/designs/issue-330-ui-npm-ci-dedup.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-332-playwright-browser-cache.md"
    - "docs/designs/issue-332-playwright-browser-cache.md"
  verifies: []
  verified-by: []
  references:
    - "mise.toml"
    - ".github/workflows/ci.yml"
    - "ui/browser-test/package.json"
    - "ui/browser-test/package-lock.json"
    - "ui/browser-test/playwright.config.ts"
    - "ui/browser-test/node_modules/playwright-core/browsers.json"
    - "ci/metrics/collect-ci-metrics.mjs"
  defers: []
  deferred-by: []
---

# Issue #332 CI-OPT-05 Playwright Chromium 準備・計測分離の棚卸し

## Summary

`ui-verify` ジョブの browser lane 準備コストを実コードと計測データから棚卸しした。npm ci と showcase dist の重複は #330 で解消済み。残存する反復コストは **Chromium バイナリの毎 run ダウンロード**（`test-ui-browser` タスク内・3 OS 各 job）と **Linux の `playwright install-deps`（apt）** で、前者は browser install と test が同一 step/同一 mise task に同居しており計測分離不能。後者は OS system 依存であり browser キャッシュとは別物として維持が必要。

## Exploration Goal

Issue #332 実装項目 1「`mise.toml` `check-ui-browser-test` / `test-ui-browser` と CI の `playwright install-deps chromium` を監査。Showcase distビルド・npm ci・Chromium install・testの重複を切り分ける」に答え、cache key 設計に必要な「ブラウバイナリの pin 源・配置先・検証手段」を確定する。

## Key Issues and Assumptions

- `test-ui-browser` の `run` は `[page build, playwright install chromium, playwright test]` の 3 段。browser 取得時間とテスト時間が 1 step に集約され分離計測不能 — confirmed（Evidence §1）。
- `playwright install-deps chromium`（Linux apt・sudo）は OS system library の導入であり browser バイナリとは別レイヤー — confirmed。actions/cache の対象にできない。
- Playwright バージョン→Chromium revision の pin 源は `node_modules/playwright-core/browsers.json`（`chromium` entry の `revision`）。1.64.1 → 1248 — confirmed（Evidence §3）。
- ブラウザ配置先は `PLAYWRIGHT_BROWSERS_PATH` で上書き可能。未設定時は OS 既定（Linux `~/.cache/ms-playwright`・macOS `~/Library/Caches/ms-playwright`・Windows `%LOCALAPPDATA%\ms-playwright`）— confirmed（Evidence §3・job log）。
- `playwright install chromium` は revision 単位で冪等（存在すれば skip）— confirmed（実測）。
- ブラウザバイナリの launch 検証は `playwright screenshot --browser chromium about:blank <file>` で可能（独自コード不要）— confirmed（本機実測・probe.png 生成）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| actions/cache で `PLAYWRIGHT_BROWSERS_PATH` を固定 path に pin + version/revision/OS-arch key | 単一 path で 3 OS 共通・key が pin 源に厳密・miss で通常 install に自然 fallback | job env 追加・破損検知は別 probe が必要 | yes |
| OS 既定 path をそのまま cache | env 変更不要 | path が OS ごとに異なり restore path の記述が matrix 依存になる | no |
| `PLAYWRIGHT_BROWSERS_PATH=0`（node_modules 同梱） | hermetic | npm ci が毎回 wipe するので cache と相性が悪い・package dir 汚染 | no |
| apt system deps も cache 化（third-party action） | install-deps 数秒の削減 | `actions/*` 以外の action 依存を新たに信用する・root 権限・system lib の部分復元リスク | no（defer） |
| restore-keys で prefix fallback | playwright 更新時に古 revision が残り一部再利用 | 使われない revision を運ぶだけで帯域/容量の無駄・key 意図が曖昧になる | no |

## Tentative Conclusions and Hypotheses

- confirmed: 毎 run 3 OS 各 job が `chromium-1248` + `chromium_headless_shell-1248` + `ffmpeg-1013`（+Windows `winldd-1007`）を `cdn.playwright.dev` から取得（job log 実測・Evidence §2）。
- confirmed: `test-ui-browser` step 実測 ubuntu 15–24s / windows 26–39s / macos 21–42s（内部: page build + browser download + 30 spec）。`Install Playwright browser system deps (Linux)` は 4–23s。
- confirmed: metrics 収集器 `collect-ci-metrics.mjs` の step lane は `/playwright/i` → `playwright`、`/cache/i` → `cache-restore`、`mise run *-ui-*` → `ui` を既に分類済み。**step 名を分離すれば収集器の変更なしに browser setup / download / test が分離記録される**。
- hypothesis→confirmed: `PLAYWRIGHT_BROWSERS_PATH` を `${{ github.workspace }}/.cache/ms-playwright` に pin すれば cache path が `.cache/ms-playwright` 一つで 3 OS 共通になり `.gitignore` の `.cache/` 済み（収集対象外）。
- confirmed: revision 変更 → key 変更 → miss → `playwright install` が新 revision を取得。cache 破損 → launch probe fail → wipe + reinstall → probe 再検証。fail-closed に通常 install へ落ちる。

## Open Questions

- warm cache 時の `playwright install` no-op 時間（~1–3s 想定）と冷温の実差は PR CI run で確定（ローカルは Windows のみ）。
- GitHub-hosted runner の ms-playwright ダウンロードが既に Azure CDN 経由で速い（ubuntu 実測 ~9s）ため、cache hit の純粋な時間効果は小–中程度と見込む。効果の確定値は After 計測待ち。

## Promotion Candidates

- [x] spec-doc needed: cache key・fallback・分離計測・不変条件 → `docs/specs/issue-332-playwright-browser-cache.md` として起票。
- [ ] adr-doc needed: なし（actions/cache + mise task の適用であり新規アーキテクチャ判断ではない）。

## Sources and Relations

- Issue #332（要件源）、#327（Before/After 計測テンプレート・lane 分類正本）、#330（install 集約済み前提）、#310（browser-test harness 正本）。

## Evidence

### 2026-11-04 監査: mise.toml / ci.yml の現行構造

```
mise.toml
  install-ui-browser-test   run: npm --prefix ui/browser-test ci（freshness 付き）
  check-ui-browser-test     depends: install-ui-browser-test + build-ui-showcase
                            run: npm --prefix ui/browser-test run typecheck
  test-ui-browser           depends: install-ui-browser-test + build-ui-showcase
                            run: [ npm run build（page modules tsc emit）,
                                   playwright install chromium（download/restore check）,
                                   npm test（playwright test、30 spec） ]

ci.yml ui-verify（3 OS matrix）
  setup-ui → ... → check-ui-browser-test
  → Install Playwright browser system deps (Linux only, apt+sudo)
  → Run UI browser tests (mise run test-ui-browser)   ← install+build+test が 1 step
  → Upload UI browser evidence（test-results/、30 日保持、if: always()）
```

### 2026-11-04 Before 実測（collect-ci-metrics.mjs・直近 run の step timing）

| run | job | install-deps(Linux) | test-ui-browser step |
| --- | --- | --- | --- |
| 41810895205 | ubuntu / windows / macos | 6 / – / – | 23 / 36 / 42 |
| 41810176099 | ubuntu / windows / macos | 23 / – / – | 22 / 34 / 32 |
| 41809614467 | ubuntu / windows / macos | 5 / – / – | 23 / 39 / 39 |
| 41809379033 | ubuntu / windows / macos | 6 / – / – | 22 / 34 / 24 |
| 41793191157 | ubuntu / windows / macos | 4 / – / – | 15 / 35 / 21 |

job 126505288074（run 41809614467・ubuntu）log 抜粋: `Downloading Chrome for Testing 156.0.8078.4 (playwright chromium v1248)` → `/home/runner/.cache/ms-playwright/chromium-1248`、同 ffmpeg-1013・chromium_headless_shell-1248。**毎 run ダウンロードが発生**（ブラウザ cache 不在）。

### 2026-11-04 pin 源・配置先の確定（本機実測）

- `ui/browser-test/package-lock.json`: `@playwright/test` 1.64.1・`playwright` 1.64.1・`playwright-core` 1.64.1。
- `node_modules/playwright-core/browsers.json`: chromium `revision=1248`・chromium-headless-shell `1248`・ffmpeg `1013`・winldd `1007`（installByDefault=false）。
- `playwright install chromium --dry-run`（Windows）: install 先 `…/ms-playwright/chromium-1248`・`chromium_headless_shell-1248`・`ffmpeg-1013`・`winldd-1007`。
- launch probe: `playwright screenshot --browser chromium about:blank probe.png` 成功（headless-shell launch = test suite が使う binary 経路）。
