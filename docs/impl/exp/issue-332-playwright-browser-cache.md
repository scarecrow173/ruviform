# Issue #332 CI-OPT-05 Playwright Chromium cache Before/After（2026-11-04）

- 正本の計測契約: `docs/designs/issue-327-cross-repo-ci-baseline.md`（指標定義・lane 分類・Cold/Warm・Before/After テンプレート）
- 計測ツール: `ci/metrics/collect-ci-metrics.mjs`（Before）+ `gh api` step timing 抽出（After、同一 schema）
- Raw 証拠: `docs/impl/exp/data/issue-332-playwright-browser-runs.jsonl`（229 行: before run 41780346388 の ui-verify step 全量 + after-cold run 41815446750 の 12 job / ui-verify step 全量）
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

### After(warm) — cache hit 予定 run

| step | ubuntu | windows | macos |
| --- | --- | --- | --- |
| Install Playwright browser system deps (Linux) | TBD | TBD | TBD |
| Read Playwright browser version/revision | TBD | TBD | TBD |
| Restore Playwright Chromium cache | TBD(hit) | TBD(hit) | TBD(hit) |
| Prepare Playwright Chromium | TBD(no-op期待) | TBD | TBD |
| Validate Playwright Chromium launch | TBD | TBD | TBD |
| Run UI browser tests | TBD | TBD | TBD |

（次 push run 確定後に実測値で更新）

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

| 指標 | Before(p50、n=16成功run) | After(cold、1 run) |
| --- | --- | --- |
| workflow wall_time_s | 308 | ~290（ui-verify windows 4m49s が critical path） |
| ui-verify job 時間 | ubuntu ~223s級 / windows ~284s級 / macos ~207s級（代表 run） | 4m19s / 4m49s / 3m6s |
| runner_total_s | ~1475-1823 | （1 run のみ・参考） |

注: After は 1 run の冷 data のみ。warm 反復で browser install の ~8-16s が消え、post save ~4-6s も消える（key 一致で save skip）ため、warm の ui-verify は cold 比で数秒〜十数秒短い見込み。

## 5. 残課題

- warm 実測の反映（本 doc §2 After(warm)・impl record）。
- Playwright bump 時の cold 再計測は不要（key が自動切替・設計どおり miss→install）。
- runner 全体時間への寄与は数秒級 — #333 の change-impact DAG 側で大きい構造的最適化が別途進行中。
