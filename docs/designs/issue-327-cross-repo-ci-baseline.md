---
id: "DESIGN-034RBJLryk98hb4Yyq5wYG"
type: "design"
status: "approved"
title: "Issue #327 CI計測Baseline 計測契約・分類・比較手順"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "docs/specs/issue-327-cross-repo-ci-baseline.md"
  changes:
    added:
      - type: "directory"
        path: "ci/metrics"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-327-cross-repo-ci-baseline.md"
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-327-cross-repo-ci-baseline.md"
    - "docs/adr/cross-repo-ci-revision.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-219-consumer-cross-repo-ci.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-327-cross-repo-ci-baseline.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-327-cross-repo-ci-baseline.md"
  references:
    - "https://docs.github.com/en/rest/actions/workflow-runs"
    - "https://docs.github.com/en/rest/actions/workflow-jobs"
  defers: []
  deferred-by: []
---

# Issue #327 CI計測Baseline 計測契約・分類・比較手順

## Context

Issue #327 は後続 9 件の CI 最適化 Issue が共有する計測基盤。本 design は「何を・どう測り・どう比較するか」の**計測契約の正本**であり、計測スクリプト `ci/metrics/collect-ci-metrics.mjs` と実測 Baseline（experiment log + JSONL）がこの契約を実装・実証する。GitHub Actions REST API（`gh api`）が唯一のデータ源。新規監視基盤・証拠 DB は作らない。

前提（discovery 実測・2026-11-04）:

- 3 repo は public。Run/Job/Step timing・Job log は `gh api` で取得可能。
- required checks: Ruviform = classic branch protection 12 contexts。AudioLens/Synvelis = 保護なし（`summary` gate・将来 Ruleset 移行対象の job 集合として記録）。
- Run 蓄積: ruviform 171 / audio-lens 2 / synvelis 14。

## Scope

### In Scope

- 指標定義（Run/Job/Step 3 層、待機/実行の分離、critical path、総 Runner 時間、cache 系）。
- lane 分類表（job→lane、step→lane の辞書）と 3 repo への適用。
- Cold/Warm 判定・cache Hit/Miss/転送コストの取得手順。
- 比較条件の記録形式（repo/SHA/toolchain/OS/profile/features/Cargo.lock/GUI fixture/cache 状態）。
- 必須検証一覧 Baseline の再導出手順と Fail-Open 差分監査形式。
- Before/After 測定テンプレート（後続 Issue がそのまま使う記入様式）。
- `collect-ci-metrics.mjs` の入出力仕様。

### Out of Scope

- CI workflow の変更・最適化（後続 Issue）。
- 継続監視・自動収集（Issue 禁止）。
- Consumer repo 内ファイルの変更（read-only 観測）。
- タイミング攻撃的精度（API の秒粒度と runner 側 queue 内訳は扱わない — 後述の既知限界）。

## Components and Boundaries

| 構成要素 | 責務 | 正本 |
| --- | --- | --- |
| `ci/metrics/collect-ci-metrics.mjs` | `gh api` で Run/Job/Step(+任意で job log)を収集し指標を算出。JSONL raw + Markdown 表を出力 | スクリプト自体 |
| 本 design | 指標・lane 分類・比較条件・監査形式の定義 | 本ファイル |
| `docs/impl/exp/` experiment log + `data/*.jsonl` | 実測 Baseline の証拠保存（観測値はここにのみ記録） | impl-doc 規約 |
| required checks / job 集合 | `gh api` + workflow YAML から機械再導出。手書きコピーを正本にしない | API / YAML |

## Data and Control Flow

### 収集フロー

```text
for repo in {ruviform, audio-lens, synvelis}:
  1. workflows 列挙          GET /repos/{o}/{r}/actions/workflows
  2. runs 列挙 (per_page=30) GET /repos/{o}/{r}/actions/runs[?workflow_id&event&status]
  3. run 詳細                GET /repos/{o}/{r}/actions/runs/{run_id}
  4. jobs + steps            GET /repos/{o}/{r}/actions/runs/{run_id}/jobs?per_page=100&filter=all
  5. (任意) job log          GET /repos/{o}/{r}/actions/jobs/{job_id}/logs   ← cache Hit/Miss 用
  6. required contexts       GET /repos/{o}/{r}/branches/main/protection    (404 = 保護なし)
     + rulesets              GET /repos/{o}/{r}/rulesets?includes_parents=true
  7. workflow YAML           GET /repos/{o}/{r}/contents/{path}             (needs 静的解決・job 集合再導出)
```

認証は ambient `gh` の token。3 repo は public のため read のみで十分。rate limit は job log 取得が支配的（job 数 × run 数）なので log 解析は代表 run のみ行う。

### 指標定義（全て秒・ISO8601 差分）

| 指標 | 定義 | 備考 |
| --- | --- | --- |
| `workflow_wall_time_s` | `run.updated_at − run.created_at` | Issue が「5分18秒」と呼ぶ値。queue 込みの利用者体感時間 |
| `workflow_queue_s` | `run.run_started_at − run.created_at` | run 全体の起動待ち |
| `job_wait_s` (per job) | `job.started_at − run.run_started_at` | runner 割当・依存解決待ちの近似値。**実行時間と混ぜない** |
| `job_exec_s` (per job) | `job.completed_at − job.started_at` | runner 上の実行時間（Set up job/Complete job を含む） |
| `exec_span_s` | `max(job.completed_at) − min(job.started_at)` | job 群の実実行スパン（queue 除く） |
| `total_runner_time_s` | `Σ job_exec_s` | 総 Runner 時間。並列数に比例し wall time とは無関係に増減する |
| `critical_path_s` | `needs` DAG 上の最長 chain（各 node は job_exec_s、queue は含めない） | 全 job 並列なら最長 job と一致。AudioLens は `ruviform-access + max(verify*,ui*,webview*,gui-real) + summary` |
| `step_s` (per step) | `step.completed_at − step.started_at` | skipped step は 0 として別扱い |
| `runner_overhead_s` (per job) | `Set up job` + `Complete job` + `Post *` step 合計 | runner 側オーバーヘッドとして lane 集計から分離 |

### lane 分類

job→lane（job 名の先頭トークンで分類、` (os)` 接尾辞を除去）:

| lane | 判定（job name prefix） | 内容 |
| --- | --- | --- |
| `rust-verify` | `verify` | fmt-check/check-ci/lint-ci/test-ci の Rust 検証 |
| `ui-verify` | `ui-verify` | headless UI 契約・component・guitest（npm+tsc+node --test） |
| `webview-verify` | `webview-verify` | AudioLens の実 webview lane |
| `l1-real-verify` | `l1-real-verify` | 実規格 plugin L1 E2E |
| `gw10-verify` | `gw10-verify` | Synvelis GUI/Window lane |
| `rt-verify` | `rt-verify` | RT 契約テスト |
| `msrv-verify` | `msrv-verify` | MSRV 検証 |
| `icc-verify` | `icc-verify` | ICC 変換検証 |
| `bench` | `bench` | bench-smoke |
| `dep-guard` | `dep-guard` | 依存監査 lane |
| `gui-real` | `gui-real` | 実 GUI lane |
| `preflight` | `ruviform-access`, `preflight` | access-mode 解決 preflight |
| `evidence` | `evidence`, `summary` | 集約 gate/evidence |
| `consumer-impact` | consumer-impact-check 全体 | PR 機械補助（required 外） |
| `consumer-verify` | consumer-verify 全体 | Cross-Consumer 情報 lane（required 外） |

step→lane（job 内の時間分解。正規表現・先勝ち。実装: `collect-ci-metrics.mjs` `STEP_LANES`）:

| step lane | 判定（要約） | 例 |
| --- | --- | --- |
| `runner-overhead` | `Set up job`/`Complete job`/`Post *` | runner オーバーヘッド |
| `checkout` | `Checkout`/`Clone ` | clone + LFS + consumer clone |
| `cache-restore` | `cache` を含む step | cargo/mise/playwright cache 復元・保存 |
| `mise` | `Set up mise`/`Restore .*mise`/`mise-action`/`Install mise`/`Install .*toolchain`/`rustup` | tool 復元 |
| `dep-fetch` | `npm ci`/`npm install`/`cargo fetch`/`Restore CI-required tools`/`Fetch deps`/`Install .*headers`/`Install SSH credential` | 依存取得（cache 復元除く） |
| `playwright` | `playwright` | browser 系 |
| `rust` | `mise run fmt/check/lint/test/build/verify*`・`cargo *`・`*-ci lane`・`smoke lane`・`Build headless CLI` | Rust 検証 |
| `ui` | `mise run *-ui-*`/`tsc`/`node --test`/`npm --prefix` | UI 検証 |
| `evidence` | `summary`/`evidence`/`Upload`/`gate on`/`Guard`/`assert`/`Enforce`/`Require non-empty`/`not-run-is-not-pass`/`^Verify `/`Validate`/`Record real-GUI gate`/`Owner triage` | 証拠・集約・gate 検証 |
| `verify-other` | `mise run`（その他）/`Run `/`deploy-key`/`access`/`LFS`/`git `/`prek`/`config files`/`Cleanup credential`/`Measure disk`/`Pin ruviform` | lane 固有検証・雑務 |

判定不能な step は `other` に落とし、report で名列挙する（分類漏れを可視化し Fail-Open にしない）。

### Cold/Warm と cache 計測

- timing API だけでは cache Hit/Miss を区別できない。`--logs` 指定時、代表 run の各 job log から以下を抽出する:
  - `Cache hit for restore-key: <key>` / `Cache restored from key:` → **hit**（key と `Cache Size: ~N MB (B bytes)` の転送サイズ）
  - `Cache not found for input keys:` / `cache not found` / `Failed to restore` → **miss**
  - `Post job cleanup` 内 `Cache saved with key:` / `Caching` → save 有無・サイズ
  - mise-action log の `mise cache not found` / `Restoring mise cache` → mise cache 判定
- **Cold/Warm 判定**: job 内の cargo cache が miss ならその job は `cold`。run 全 job の cargo/mise cache 全 hit を `warm`、一部 miss を `mixed`、全 miss を `cold` と run 単位でも分類。log 未取得の run は `unknown` と明記（pass 扱いしない）。
- 転送コスト = `Cache Size` バイト数と restore/post step 時間の対応付け。

### critical path 計算

- job `needs` は workflow YAML から静的に読む（API jobs payload に needs は無い）。matrix job は job 名 ` (os)` を剥がして job id に正規化。
- `critical_path_s` = DAG 上で `Σ job_exec_s` が最大の chain（同時刻開始でも needs なき job は独立）。実装上は「needs を満たした時点で start 可能」の単純重み付き最長経路。
- 分断・unknown な `needs` 参照（他 workflow/呼出）は warn して `exec_span_s` を併記。

### 必須検証一覧 Baseline（Fail-Open 監査）

各 repo について以下を機械再導出して Baseline とする:

1. `required_contexts` = branch protection `required_status_checks.contexts` + rulesets の required checks（現状 3 repo とも rulesets 0 件）。404 は「保護なし」として明示記録（0 件と解釈しない）。
2. `workflow_jobs` = 計測 run の jobs API から得た job 名集合（matrix 展開後の instance 名）。
3. `coverage_matrix` = job → 検証種別（OS/feature/Golden/RT/GUI/MSRV/Consumer）の対応表。手書きだが再導出手順を併記。
4. Before/After 比較時、required_contexts と workflow_jobs の **差分**（消えた context・追加 context・skipped/not-run 化した lane）を報告する。差分ゼロ＋結果非退化を「検証非回帰」とする。

### 比較条件（Before/After 記録欄）

同一 Before/After 内で以下を固定・記録する:

- repo / run id / head SHA / event / run_attempt / conclusion
- Rust toolchain（mise.lock `rust` 値）/ runner OS / profile（dev vs ci）/ features / `Cargo.lock` 同異
- GUI fixture バージョン（該当 lane のみ）/ cache 状態（warm/cold/mixed/unknown）
- サンプル n・収集期間・p50/p95 または観測値一覧

### Before/After テンプレート（後続 Issue がコピーして使う）

```markdown
| 指標 | Before (run <id>) | After (run <id>) | Δ | 判定 |
| --- | --- | --- | --- | --- |
| workflow_wall_time_s (p50 または観測 n=) | | | | |
| exec_span_s | | | | |
| critical_path_s | | | | |
| total_runner_time_s | | | | |
| queue 合計 (Σ job_wait_s) | | | | |
| lane: <lane 名> job_exec_s | | | | |
| cache: cargo hit/miss・転送 B | | | | |
| cache: mise hit/miss | | | | |

| 検証項目 (Baseline coverage_matrix) | Before | After | 判定 |
| --- | --- | --- | --- |
| required_contexts 差分 | | | |
| workflow_jobs 差分 | | | |
| lane <名>: Pass/Fail/NotRun | | | |

条件: repo= SHA= toolchain= OS= profile= features= Cargo.lock= cache=
```

## Non-Functional Constraints

- スクリプトは依存ゼロ（Node 標準ライブラリのみ・`gh` CLI を subprocess 起動）。新規 npm 依存は持ち込まない。
- 既定は timing API のみ（rate limit 配慮）。log 解析は `--logs` opt-in。
- 出力: `--out <dir>` に `runs.jsonl`（run/job/step/caches の raw）+ `report.md`（表）。JSONL が証拠、report は派生。
- 失敗 run も計測対象だが Baseline 統計は `conclusion=success && run_attempt=1` に限定し、failure/cancelled/再試行は別枠で一覧する（成功実行との混合を禁止）。

## Risks and Trade-offs

- **API 精度**: `started_at`/`completed_at` は秒粒度・job 内に runner プロビジョニングを含む → 指標は「観測可能な近似」として定義済み。比較は同一 API の同一指標同士でのみ行う（局所一貫性）。
- **log 解析の脆さ**: log の行形式は runner/action の表示仕様依存 → hit/miss パターンを複数受け付け、不明行は `unknown` 集計して黙殺しない。
- **critical path の静的 needs 解決**: reusable workflow・動的 matrix は解決不能 → warn + exec_span 併記で Fail-Open しない。
- **n 不足**: AudioLens n=2 等では統計値を出さない（Issue 本文どおり観測値＋n）。
- **変更禁止**: 本 issue は計測のみ。収集で既存 CI・required checks・他 repo ファイルを一切変更しない。

## Deferred Design Concerns

- ~~Playwright lane の cache 分類~~ — **#332（CI-OPT-05）で解決済み**: `playwright` step lane は既存行で browser 準備 step（install-deps/prepare/validate）を分類し、browser binary cache の restore/save step は `cache-restore` lane・cache verdict は既存 CACHE_PATTERNS（`Cache hit for restore-key`/`Cache not found`/`Cache saved`）が汎用に拾う。収集器の変更なしに Cold/Warm・download 有無・実テスト時間の分離記録が成立（設計: `docs/designs/issue-332-playwright-browser-cache.md`）。
- runner プロビジョニング内訳 — GitHub 側未公開。待機は `job_wait_s` 近似のまま。
- billing multiplier（Windows/macOS の倍率）— 最適化のコスト評価が必要になった時点で拡張。

## Verification Notes

- [x] 3 repo の workflow job 名・needs・required checks が discovery 実測と一致（2026-11-04 実測値で記述）。
- [x] 全指標が Actions API の取得可能フィールドにのみ依存（run/job/step timing・job log・protection・contents）。
- [x] not-run/unknown を pass と読み替えない経路が lane 分類（`other` 可視化）・cache（`unknown`）・検証集合差分で Fail-Open にならない。
- [ ] スクリプト実装後、代表 run で算出値が API 生値と一致すること（task の検証項）。
