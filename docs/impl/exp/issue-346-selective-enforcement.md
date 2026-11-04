# Issue #346 選択実行有効化 — Before/After メトリクス記録（2026-11-05）

- 計測契約の正本: `docs/designs/issue-327-cross-repo-ci-baseline.md`
- 収集器: `ci/metrics/collect-ci-metrics.mjs`（`gh api` read-only）
- Raw 証拠: `docs/impl/exp/data/issue-346-enforcement-before-runs.jsonl`（4723 行・15 run / 14 job × run 分の timing record）
- Runbook: `docs/designs/issue-346-ruleset-migration-runbook.md`
- 関連: `docs/impl/exp/issue-333-impact-based-ci.md`・`data/issue-334-shadow-classification-run-41840536142.jsonl`・`data/issue-335-shadow-classification-run-41848239157.jsonl`

## 1. 測定条件

| 項目 | 値 |
| --- | --- |
| 測定日 | 2026-11-05（UTC） |
| 収集 | `node ci/metrics/collect-ci-metrics.mjs --repo scarecrow173/ruviform --workflow ci --runs 15` |
| 対象 | `scarecrow173/ruviform` workflow `ci`（直近 15 run・`conclusion=success && run_attempt=1` は 11 件） |
| 状態 | **Before** — Shadow Mode（全 lane 実行・`impact`/`ci-gate` は非 required 観測 job・12 required contexts） |

## 2. Before メトリクス（Shadow・全 lane 実行）

### run 集計

| 指標 | 値 | 備考 |
| --- | --- | --- |
| wall_time p50 | **307s** | n=11（success & attempt1） |
| wall_time p95 | **497s** | min 245s・max 497s |
| critical_path（代表 run 41851002767） | **235s** | 最長 lane（ui-verify windows 221s）+ `ci-gate`（14s） |
| runner_total（代表 run） | **1474s** | 14 job 合計（12 lane + impact + ci-gate） |

### lane 別 exec 合計（代表 run 41851002767）

| lane | exec_s | 構成 |
| --- | --- | --- |
| ui-verify | 517 | 130（ubuntu）+ 221（windows）+ 166（macos） |
| rust-verify | 502 | 105 + 220 + 177 |
| l1-real-verify | 209 | 37 + 81 + 91 |
| bench | 91 | ubuntu |
| rt-verify | 87 | ubuntu |
| icc-verify | 33 | ubuntu |
| impact + ci-gate | 35 | 21 + 14（Shadow 期は critical path 外の並列 job） |

### runner 逼迫の実測（有効化前倒しの動機）

- `push` run [41851785028](https://github.com/scarecrow173/ruviform/actions/runs/41851785028)
  （merge commit `33d93dd`・attempt 3）: **queued 2279s 超**を観測時点で記録
  （macOS arm64 runner 割当待ち）。required lane が queue→cancel される実害の
  生データ。

## 3. After メトリクス（選択実行有効化後）

**未実測** — 本 PR merge 後の selective run 群で同一手順により収集し、下表を
更新する。有効化の実効証（`expected:false` lane の `skipped` → gate
`NotApplicable`）は merge 後の後続 PR（docs-only 等）で確認する前提。

| 指標 | Before | After（未実測） | 見込み（推定・実測ではない） |
| --- | --- | --- | --- |
| wall p50 | 307s | — | full PR は不変〜微増。selective PR は lane 数に応じ縮小 |
| critical_path | 235s | — | **全 run で +~20–25s 増**（`impact` が `needs` で直列化。Shadow 期は並列・critical path 外） |
| runner_total | 1474s | — | docs-only なら ui(517)+l1(209)+rt(87)+icc(33)+bench(91) ≈ **937s 削減**。単一 crate（例: math → verify+rt+bench+l1）なら ui+icc ≈ 550s 削減。full PR は 0 |
| macOS leg 数/run | 4（verify+ui+l1 macos + 余分な windows 依存） | — | selective 分類で macOS leg を持たない lane は leg ごと skip → queue 滞留の露出削減 |

見込みの根拠: Before 代表 run の lane exec 実測（§2）から `expected:false`
lane の exec 合計を引いた差分。`impact` 直列化 +21s・`ci-gate` ~14s は全 run
共通の固定増分として含む。**実測ではなく推定**であり、p50/p95 は母集団が
selective 分類を含んでから再計測する。

## 4. 監視継続（偽陰性検出）

Shadow 期と同一の検出点が有効化後も有効:

- `gate-report` artifact の `not-expected-lane-failure/cancelled` 行 = 偽陰性
  （skip された lane が本来 fail）。検出時は runbook §監視の対応
  （`CI_IMPACT_DISABLE=1` + 分類契約修正）。
- `over-executed` 行（expected:false なのに実行） = 偽陽性・コストのみ。
- `expected-lane-skipped` = `if:` 配線破損。
- Shadow 実績サマリ: PR #345（full・偽陰性検出器未発動）・#347・#348
  （いずれも selective・`falseNegatives: 0`）。

## 5. 測定限界

| 項目 | 状態 |
| --- | --- |
| After 実測 | **未実測** — merge 後の後続 run で収集予定（本 PR は ci-config 変更のため decision=full で全 lane 実行） |
| selective 率の母集団統計 | NotRun — selective PR が日常運用に混ざってから p50/p95 を算出 |
| runner 内部 queue 分解 | Unsupported — `job_wait_s` 近似のみ（baseline §9 と同じ制約） |
| ruleset 移行タイミングの影響 | 移行中 deadlock 窓内の PR は re-run 済みデータとして除外して比較する |
