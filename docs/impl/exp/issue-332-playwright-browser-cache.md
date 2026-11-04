# Issue #332 CI-OPT-05 Playwright Chromium cache Before/After（2026-11-04）

- 正本の計測契約: `docs/designs/issue-327-cross-repo-ci-baseline.md`（指標定義・lane 分類・Cold/Warm・Before/After テンプレート）
- 計測ツール: `ci/metrics/collect-ci-metrics.mjs`（Before）+ `gh api` step timing 抽出（After、同一 schema）
- Raw 証拠: `docs/impl/exp/data/issue-332-playwright-browser-runs.jsonl`（358 行: before run 41780346388 の ui-verify step 全量 + after-cold run 41815446750 / after-warm run 41816549243 の各 12 job / ui-verify step 全量）
- 実装記録: `docs/impl/ir/issue-332-ci-opt-05-playwright-chromium-cache.md`
- PR: https://github.com/scarecrow173/ruviform/pull/341

## 1. 測定条件

| 項目 | Before | After(cold) |
| --- | --- | --- |
| run_id | 41780346388（main・push） | 41815446750（PR #341・pull_request 初回） |
| 対象 job | `ui-verify` × 3 OS | 同 |
| Playwright | `@playwright/test` 1.64.1 / chromium v1248 | 同（key: `playwright-<OS>-X64-pw1.64.1-chromium1248`） |
| cache 状態 | Playwright cache なし（NotApplicable） | 初回 run のため全 OS miss（log: `Cache not found for input keys` → download → post step で save） |
| 精度 | API 秒粒度 | 同 |

## 2. browser lane step 計測（秒）

### Before — run 41780346388

| step | ubuntu | windows | macos |
| --- | --- | --- | --- |
| Install Playwright browser system deps (Linux) | 6 | 0(skip) | 0(skip) |
| Run UI browser tests（= `test-ui-browser` 内で page build + chromium download + 30 tests 同居） | 28 | 40 | 44 |
| Upload UI browser evidence | 1 | 2 | 2 |

- download 実測: ubuntu job log で `Downloading Chromium` 系 ~9s を確認。step 時間に埋もれて分離不能だった。
- 全 run 横断（n=54 job、成功 16 run）: `test-ui-browser` step p50=30s min=0 max=66、install-deps p50=0 max=23。

### After(cold) — run 41815446750

| step | ubuntu | windows | macos |
| --- | --- | --- | --- |
| Install Playwright browser system deps (Linux) | 16 | 0(skip) | 0(skip) |
| Read Playwright browser version/revision | 1 | 1 | 1 |
| Restore Playwright Chromium cache | 0(miss) | 1(miss) | 1(miss) |
| Prepare Playwright Chromium (`install-ui-browser-chromium`) | 8(download) | 16(download) | 16(download) |
| Validate Playwright Chromium launch | 3 | 2 | 2 |
| Run UI browser tests（page build + 30 tests のみ） | 10 | 20 | 23 |
| Upload UI browser evidence | 2 | 1 | 1 |
| Post restore（cache save） | 4 | 6 | 5 |

### 分離計測の読み取り（issue 実装 4・Acceptance 対応）

| 指標 | ubuntu | windows | macos |
| --- | --- | --- | --- |
| cold browser setup（deps+meta+restore+install+validate） | 28 | 20 | 20 |
| cold download 有無 | あり（8s install） | あり（16s） | あり（16s） |
| 実 test 時間（page build+suite、cold） | 10 | 20 | 23 |
| Before の同居 step（download 込み） | 28 | 40 | 44 |

- cold でも実 test step は Before より短い — download が install step へ移り実計測が可視化。download 有無が step 単位で直接観測可能になったのが本変更の主効果。
- browser 準備合計（cold）は Before の埋め込み準備と概ね同等。cold では install-deps（ubuntu）と download が依然支配的 — cache による短縮効果は warm で発現。

### After(warm) — run 41816549243（同 PR の次 commit push、cache hit）

| step | ubuntu | windows | macos |
| --- | --- | --- | --- |
| Install Playwright browser system deps (Linux) | 18 | 0(skip) | 0(skip) |
| Read Playwright browser version/revision | 0 | 1 | 0 |
| Restore Playwright Chromium cache | 6(hit) | 4(hit) | 8(hit) |
| Prepare Playwright Chromium (`install-ui-browser-chromium`) | 1(no-op) | 1(no-op) | 1(no-op) |
| Validate Playwright Chromium launch | 2 | 2 | 2 |
| Run UI browser tests（page build + 30 tests のみ） | 14 | 12 | 20 |
| Upload UI browser evidence | 2 | 1 | 2 |
| Post restore | 0(save skip) | 0(save skip) | 0(save skip) |

- 全 OS で `Cache hit for: playwright-<OS>-X64-pw1.64.1-chromium1248` → install task が revision check で 1s no-op（download ゼロ）→ probe 成功 → post save skip（`Cache hit occurred on the primary key ..., not saving cache`）を log で確認。
- warm browser 準備合計（deps+meta+restore+install+validate）: ubuntu 27s（うち system deps 18s が支配）/ windows 8s / macos 11s。download コストは完全消滅。
- Before 同居 step（download 込み）との対比: ubuntu 28→（restore+install+validate=9s＋test 14s）、windows 40→19s、macos 44→31s。browser 関連 work はどの OS でも減少。

## 3. fallback / 検証能力の確認

| 観点 | 状態 |
| --- | --- |
| cache miss → 正規 install | cold run の log で `Cache not found` → Playwright installer download → probe 成功を確認 |
| 破損 → wipe+reinstall | ローカル実演（headless shell binary 破壊 → probe fail → guarded wipe → reinstall → `PROBE-OK`）。CI での破損経路は未発生（仕組みは同一 step） |
| version/revision 変更 → 新 key | key に `pw<version>-chromium<revision>` を内包・`restore-keys` なし（stale binary 不復元）。実際の bump は未発生だが key 構成で fail-closed |
| Evidence（screenshots/traces/evidence.json・30日） | upload step 不変・cold run 3 OS で artifact 生成確認 |
| browser PASS ≠ WebView/native GPU/製品 E2E | ui-verify の検証範囲・AGENTS.md の注意書き不変 |
| required contexts（12） | 全 pass・削減なし |

## 4. workflow 単位の Before/After

| 指標 | Before(p50、n=16成功run) | After(cold run 41815446750) | After(warm run 41816549243) |
| --- | --- | --- | --- |
| workflow wall_time_s | 308 | ~290（ui-verify windows 4m49s critical path） | ~294（ui-verify ubuntu 4m49s critical path） |
| ui-verify job 時間 | ubuntu ~223s / windows ~284s / macos ~207s（代表 run 41780346388） | 259s / 289s / 186s | 289s / 191s / 146s |
| browser 準備（download 等） | test step 内に同居・分離不能 | ubuntu 28s / windows 20s / macos 20s | ubuntu 27s（内 deps 18s）/ windows 8s / macos 11s |
| download 発生 | 毎 run（全 OS） | あり（cold のみ正しい動作） | **なし**（cache hit・no-op） |

注: ui-verify job 全体の変動には mise/cargo/npm 等の他 step の揺らぎも含まれ、browser 分以外を本変更の効果とは計算しない。browser lane として確実な効果は「warm では download ゼロ・install step 1s」「browser 関連 work の合計が windows 40→19s・macos 44→31s・ubuntu 28→23s（restore+install+validate+test）」に限定して読む。ubuntu の install-deps 18s と macos の test step（page build 主因・20s）は別レイヤーの支配項。大幅な構造的最適化は #333 側。

## 5. 残課題

- Playwright bump 時の cold 再計測は不要（key が自動切替・設計どおり miss→install）。
- runner 全体時間への寄与は数秒級 — #333 の change-impact DAG 側で大きい構造的最適化が別途進行中。
- ubuntu の `playwright install-deps` は cache 対象外（設計どおり毎 run 実行・16-18s）。system deps を更に短縮するには runner image 層の話になりスコープ外。
