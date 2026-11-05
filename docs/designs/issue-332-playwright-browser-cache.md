---
id: "DESIGN-EJ86GHTwsKgyexnCdTpSJ5"
type: "design"
status: "approved"
title: "Issue #332 CI-OPT-05 Playwright Chromium cache・準備分離設計"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/332"
    - "docs/specs/issue-332-playwright-browser-cache.md"
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
  implemented-by:
    - "docs/plans/issue-332-playwright-browser-cache.md"
  depends-on:
    - "docs/specs/issue-332-playwright-browser-cache.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-310-browser-gui-test-harness.md"
    - "docs/designs/issue-330-ui-npm-ci-dedup.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-332-playwright-browser-cache.md"
    - "docs/discovery/issue-332-playwright-browser-prep-inventory.md"
  derived-by:
    - "docs/plans/issue-332-playwright-browser-cache.md"
    - "docs/test-specs/issue-332-playwright-browser-cache.md"
  verifies: []
  verified-by: []
  references:
    - "mise.toml"
    - ".github/workflows/ci.yml"
    - "ui/browser-test/package-lock.json"
    - "ci/metrics/collect-ci-metrics.mjs"
  defers: []
  deferred-by: []
---

# Issue #332 CI-OPT-05 Playwright Chromium cache・準備分離設計

## Context

`ui-verify` ジョブの browser lane は `test-ui-browser` タスクの `run` に `[page build, playwright install chromium, playwright test]` が同居しており、browser 準備時間と実テスト時間が CI step 上で分離できない。Chromium バイナリ（chromium-1248・chromium_headless_shell-1248・ffmpeg-1013・Windows は +winldd-1007）は cache が無く毎 run ダウンロードされる（job log 実測）。Linux では別途 `playwright install-deps chromium`（apt・sudo）が browser の OS system library を導入する。

本設計は (a) browser バイナリの actions/cache 化、(b) install の mise task / CI step 分離、(c) fail-closed fallback、(d) 分離計測の 4 点を定める。検証能力・Evidence・required checks は不変。

## Scope

### In Scope

- `PLAYWRIGHT_BROWSERS_PATH` の CI 側 pin と `actions/cache` 導入（browser バイナリのみ）。
- `install-ui-browser-chromium` mise task 新設と `test-ui-browser` の再構成。
- `ui-verify` ジョブへの key 算出・cache 復元・prepare・launch 検証 step の追加。
- `AGENTS.md` 同期・`docs/designs/issue-327-cross-repo-ci-baseline.md` の Deferred 項目更新。

### Out of Scope

- Playwright worker 並列化・テスト内容・Evidence・fixture 範囲の変更。
- Linux apt system deps の cache 化・取得方法の変更。
- `collect-ci-metrics.mjs` のコード変更（既存 lane 分類で完結）。
- Rust 側 lane・cargo cache・mise tool cache の変更。

## Components and Boundaries

### `PLAYWRIGHT_BROWSERS_PATH` pin（ci.yml job env）

`ui-verify` job の `env` に `PLAYWRIGHT_BROWSERS_PATH: ${{ github.workspace }}/.cache/ms-playwright` を追加する。根拠:

- OS 既定 path は OS ごとに異なり（`~/.cache`・`~/Library/Caches`・`%LOCALAPPDATA%`）、cache `path:` の記述が matrix 分岐になる。workspace 内の単一相対 path に pin すれば `path: .cache/ms-playwright` 1 行で 3 OS 共通になる。
- `.cache/` は `.gitignore` 済みであり、checkout（`clean` 既定）は restore より前に完了するため衝突しない。
- install 系 step と `playwright test`（test-ui-browser 内）の双方が同一 env を見るため、install 先と参照先の食い違いが起きない。
- ローカル開発には影響しない（env は CI job のみで設定。ローカルの `mise run test-ui-browser` は従来どおり OS 既定 path を使う）。

### Cache key（pin 源厳密化）

```
key: playwright-${{ runner.os }}-${{ runner.arch }}-pw<version>-chromium<revision>
```

- `<version>`: `ui/browser-test/node_modules/@playwright/test/package.json` の `version`（lockfile pin 済み、現 1.64.1）。
- `<revision>`: `ui/browser-test/node_modules/playwright-core/browsers.json` の `chromium` entry `revision`（現 1248）。
- 算出は `setup-ui`（install-ui-browser-test）完了後に `mise exec -- node -p` で読む。**key 算出専用 step を設け、空値は `set -euo pipefail` で job fail** とする（空 key への fallback は版違い cache 汚染を生むため fail-closed）。
- `runner.os`/`runner.arch` を必ず含める（browser バイナリは OS/arch 固有。将来の arm64 runner でも安全）。
- **restore-keys は付けない**。prefix 部分一致で古い Playwright 版の cache を restore しても使われるのは新 revision のみで、古 revision ディレクトリは保存時に cache を再肥大化させるだけ。exact-key のみで miss → 全量 download に fail-closed に落ちる方が単純で正しい。
- `hashFiles` は key に含めない。package-lock 更新→ version 変化（dependabot 系でなく手動 pin 管理）で key は必ず変わり、version+revision が一意に pin を表す。lockfile hash を足すと同 version でも lockfile 変更（他 package の更新）で miss になり効率を損なう。ただし revision が version と独立に変わり得るため両者を個別に key へ入れる（playwright-core が `@playwright/test` と別 schedule で bump される経路を塞がない）。

### mise task 分離

```toml
[tasks.install-ui-browser-chromium]
depends = ["install-ui-browser-test"]
run = "npm --prefix ui/browser-test exec -- playwright install chromium"
```

- `test-ui-browser` の `run` から `playwright install chromium` 行を除去し、`depends` に `install-ui-browser-chromium` を追加。`run` は `[page build, playwright test]` の検証本体のみになる。
- `playwright install chromium` は revision 単位の存在確認で冪等（warm 時 ~1–3s の no-op）。**mise の sources/outputs freshness は付けない**: browsers path が env 依存であり、mise の freshness より playwright 自身の revision 存在確認が authoritative で正確（task-state ではなく実バイナリ基準）。常時実行・内部 skip が保守側の設計。
- `check-ui-browser-test` は変更しない（typecheck は browser 不要）。

### `ui-verify` ジョブ差分（step 順）

```
… check-ui-browser-test（既存）
Install Playwright browser system deps (Linux)      … playwright lane（既存・apt・cache 対象外）
Read Playwright browser version/revision            … playwright lane（新規・key 算出）
Restore Playwright Chromium cache                   … cache-restore lane（新規・actions/cache）
Prepare Playwright Chromium (mise run install-ui-browser-chromium)
                                                    … playwright lane（新規・install）
Validate Playwright Chromium launch                 … playwright lane（新規・fallback 検証）
Run UI browser tests (mise run test-ui-browser)     … ui lane（既存・build+test のみになる）
Upload UI browser evidence                          … evidence lane（既存・不変）
```

step 名は `collect-ci-metrics.mjs` の STEP_LANES 正規表現に合わせ、`cache` 語を含むのは真の cache step のみ（key 算出 step は `Read Playwright …` とし cache-restore lane に誤分類されないようにする）。**収集器のコード変更は不要**。

### Fallback（fail-closed の回復経路）

| 事象 | 経路 |
| --- | --- |
| cache miss（初回・key 変更） | restore なし → `playwright install` が全量 download → post-job で新 key 保存 |
| Playwright バージョン / Chromium revision 変更 | version/revision が key 変化 → miss → 正しい revision を新規取得（古 cache は自然失効・7 日未使用 eviction） |
| cache 部分復元・破損（binary 欠損・破損 archive） | install の marker 確認を通る可能性 → **launch 検証が検知** → `$PLAYWRIGHT_BROWSERS_PATH` 全削除 → 再 install → 再検証。再検証でも失敗すれば job fail（破損 binary で test に進まない） |
| key 算出失敗（node_modules 欠落・browsers.json 形式変更） | `set -euo pipefail` で当該 step fail → test 未実行（黙って誤 key 化しない） |

launch 検証は `npm --prefix ui/browser-test exec -- playwright screenshot --browser chromium about:blank <tmp>/probe.png`（独自コード不要。headless-shell の実 launch = test suite が使う経路と同一）。Linux では system deps が必要なため step 順で install-deps の後に置く。`shell: bash` を明示し 3 OS 同一ロジックにする。

### OS system deps の分離（不変条件）

`playwright install-deps chromium`（apt・sudo・Linux のみ）は browser バイナリとは別レイヤーであり actions/cache の対象にできない（system library・root 権限）。現行 step をそのまま維持し、browser cache の有無に依らず毎 run 実行する。依存関係の向きは「system deps → browser binary launch → test」であり、cache hit 時も system deps は必要（runner image のプリインストールで既に充足している場合は apt がほぼ no-op で戻る、現行どおり）。

## Data and Control Flow

Cold（cache miss / key 変更）:

```
Read version/revision → cache miss（何も restore されない）
  → playwright install chromium（全量 download ~数–30s）
  → launch 検証 → test → post-job で key 保存
```

Warm（cache hit）:

```
Read version/revision → cache hit（.cache/ms-playwright 復元）
  → playwright install chromium（存在確認 no-op ~1–3s）
  → launch 検証 → test
```

破損（restore されたが launch 不能）:

```
cache hit → install（marker のみ残存なら skip）→ launch 検証 NG
  → rm -rf $PLAYWRIGHT_BROWSERS_PATH → install（再 download）→ launch 検証 → test
```

## Non-Functional Constraints

- 信頼境界: cache の内容は workflow 変更と同等の review 境界内（branch-scoped 保存、PR は base branch cache を restore 可能だが書き込みは自 branch key）。launch 検証が corruption を、テスト自体が意味的破壊を fail-closed に検知する。
- 可観測性: cache hit/miss は actions/cache の標準ログ（`Cache hit for restore-key` / `Cache not found for input keys`）に出、`collect-ci-metrics.mjs` の既存 CACHE_PATTERNS が拾う。download 有無は `Downloading Chrome for Testing` 行と prepare step 時間で判別可能。
- 可搬性: `shell: bash` は 3 OS の GitHub-hosted runner で利用可能。path pin は `github.workspace` 由来の絶対 path で、Windows の `D:\a\…` 形式も playwright が処理する。
- 検証能力の不変: Evidence 生成物・保持期間・fixture・`retries: 0`・30 spec は一切変更しない。`test-ui-browser` の depends 追加は install が常に走る保守側の変更であり skip を生まない。
- Rollback: 変更は mise.toml + ci.yml + AGENTS.md + docs。revert で inline install の旧構造に即復帰（cache 残骸は key 単位で自然失効・required checks 不変）。

## Risks and Trade-offs

- リスク: `PLAYWRIGHT_BROWSERS_PATH` pin が install/test の片方にしか効かないと参照先不一致になる。対策: job `env` で統一し、両方が同じ値を見ることを workflow 構造で保証。
- リスク: cache restore が古い Playwright 版の dir を持ち込み disk を圧迫。対策: restore-keys を付けない exact-key のみ（version 変更時は古 dir ごと復元されない）。
- リスク: archive 破損・部分復元で marker だけ残り install が skip。対策: launch 検証 step（marker ではなく実 launch で判定）→ 全削除再 install。
- リスク: `browsers.json` のスキーマ変更（`browsers[].revision` 以外への移行）。対策: key 算出の失敗を job fail にして黙認しない・`--dry-run` 等の代替源は Deferred。
- トレードオフ: `test-ui-browser` の depends により CI でも `playwright install` が 2 回評価される（prepare step + depends）。2 回目は存在確認 no-op（~1–3s）で、ローカル単独実行の自足性（fresh checkout で `mise run test-ui-browser` だけで完走）を優先する。
- トレードオフ: workspace 内 `.cache/ms-playwright` が job 終了まで runner disk を占有するが、Chromium+headless-shell+ffmpeg で ~400MB 級・runner 標準容量内。
- トレードオフ: GitHub runner の CDN 経路が既に高速（ubuntu ~9s）であり、warm での絶対的短縮は中程度。外部 download 依存の除去による耐性向上と分離計測が主効果。

## Deferred Design Concerns

- test 並列化（`workers` 変更） | suite 拡大で runner 負荷支配になった場合 | flake 率・競合・runner CPU の Before/After 記録が前提 | 現行デフォルト維持。
- apt system deps の cache 化 | install-deps が支配的になった場合 | `actions/*` 以外の action を信用境界へ入れる判断と system lib 部分復元の整合設計 | 現行 run 維持。
- key 算出の `--dry-run` 代替 | browsers.json スキーマ変更時 | CLI 出力 parse は脆いため現行の JSON 直読み優先 | browsers.json 不変の間は変更不要。
- browser cache を mise task-state freshness に移す | PLAYWRIGHT_BROWSERS_PATH 非依存の管理へ寄せたい場合 | env 依存 path を mise outputs に写す設計が別途必要 | playwright 自体の revision 確認が authoritative で十分。

## Verification Notes

- [x] `node_modules/playwright-core/browsers.json` の `chromium.revision=1248`・`@playwright/test` `version=1.64.1` を lockfile pin と照合（本機実測）。
- [x] `playwright screenshot --browser chromium about:blank` が headless-shell launch で probe.png を生成（本機実測）。
- [x] STEP_LANES の既存正規表現に新 step 名が `cache-restore`/`playwright`/`ui` へ正しく落ちることを表で確認（収集器変更なし）。
- [ ] Cold/Warm の step 時間・download 有無が CI run で分離記録されること（PR CI で確認）。
- [ ] 破損 cache で launch 検証→wipe→reinstall が動くこと（手動/擬似破損での確認、可能なら CI 実証）。
