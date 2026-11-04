# Issue #327 3リポジトリ横断 CI 計測 Baseline（2026-11-04）

- 正本の計測契約: `docs/designs/issue-327-cross-repo-ci-baseline.md`（指標定義・lane 分類・Cold/Warm・比較条件・Before/After テンプレート）
- 計測ツール: `ci/metrics/collect-ci-metrics.mjs`（`gh api` read-only・Node 標準のみ）
- Raw 証拠: `docs/impl/exp/data/issue-327-ci-baseline-runs.jsonl`（7569 行・62 run / 458 job / 6947 step / 34 cache record）
- 実験イベントログ: `docs/impl/exp/issue-327-ci-baseline.jsonl`

## 1. 測定条件（比較条件の固定値）

| 項目 | 値 |
| --- | --- |
| 測定日 | 2026-11-04（UTC） |
| 収集 | `mise exec -- node ci/metrics/collect-ci-metrics.mjs --runs 20 --logs-run 41780346388 --logs-run 41770611162 --logs-run 41782019309` |
| 対象 repo | `scarecrow173/ruviform`・`scarecrow173/audio-lens`・`scarecrow173/synvelis`（全て public） |
| Run 蓄積（`actions/runs` total） | ruviform 171 / audio-lens 2 / synvelis 14 |
| Rust toolchain | 3 repo とも mise.toml pin `rust = 1.99.0`（components rustfmt,clippy）。synvelis `msrv-verify` は rust@1.89.0 を別途 install |
| その他 toolchain | ruviform: uv 0.13.5・prek 0.5.7・node 26.12.1。audio-lens: uv 0.13.5・node 26.12.1。synvelis: uv 0.13.4・prek 0.5.5・node 26.12.1 |
| profile | `ci`（`--profile ci`・`CARGO_INCREMENTAL=0`） |
| features | ruviform l1-real-verify は opt-in `host` feature（CLAP/VST3 fixture cdylib）。audio-lens は `features=["mp4","aac"]` の ruviform-io-symphonia のみ非 default |
| Cargo.lock | 3 repo とも tracked。cargo cache key に lockfile hash を内包（audio-lens の key は `*-cargo(-ui)-<lockhash>` 形式で観測） |
| GUI fixture | ruviform l1-real-verify = fixture cdylib（repo 内生成）。audio-lens = LFS メディア（`lfs: true` checkout）+ `baseline/` 回帰資産 |
| Runner | GitHub-hosted `ubuntu-latest`/`windows-latest`/`macos-latest` |
| 精度 | API 秒粒度。`job_wait_s` = `job.started_at − run.run_started_at` の近似。runner 内部 queue は分解不能 |

## 2. 必須検証 Baseline（Fail-Open 監査の基準集合）

`gh api repos/{o}/{r}/branches/main/protection` + `rulesets?includes_parents=true`（2026-11-04 実測。raw: `required-contexts` record）。

| repo | branch protection | required contexts | rulesets |
| --- | --- | --- | --- |
| ruviform | 有効（strict=true） | **12**: `verify`/`l1-real-verify`/`ui-verify` 各 (ubuntu,windows,macos)-latest + `rt-verify` + `icc-verify` + `bench` | 0 件 |
| audio-lens | **保護なし**（404） | 0（job 集合は §3 coverage_matrix で記録） | 0 件 |
| synvelis | **保護なし**（404） | 0（job 集合は §3） | 0 件 |

注: Issue 本文の「AudioLens `summary`・Synvelis 10 required contexts」は **workflow 内 gate job と将来 Ruleset 対象の job 集合**を指す。2026-11-04 時点で両 repo の main に強制力はない（#326/#333・AudioLens #147・Synvelis #145 の移行対象）。差分監査は required contexts と job 集合の両面で行う。

## 3. 検証 coverage matrix（job→検証種別、2026-11-04 の workflow YAML から再導出）

### ruviform `.github/workflows/ci.yml`（on: push/PR/dispatch、needs なし全並列）

| job (job_id) | instance | lane | 検証種別 |
| --- | --- | --- | --- |
| `verify` | ×3 OS | rust-verify | fmt-check・check-ci・lint-ci・test-ci（Rust 全 workspace）・prek |
| `l1-real-verify` | ×3 OS | l1-real-verify | 実規格 plugin L1 E2E（CLAP/VST3 fixture・`host` feature opt-in） |
| `ui-verify` | ×3 OS | ui-verify | headless UI 契約（ui/core）・components・guitest（npm+tsc+node --test） |
| `rt-verify` | 1 | rt-verify | RT 契約テスト（`rt_` 選択の非空強制） |
| `icc-verify` | 1 | icc-verify | ICC 変換検証 |
| `bench` | 1 | bench | bench-smoke |

required 外 workflow: `consumer-verify.yml`（dispatch のみ・preflight→verify-{audiolens,synvelis}→summary）、`consumer-impact-check.yml`（PR のみ・classification 機械補助）。

### audio-lens `ci.yml`（on: push/PR/dispatch。`ruviform-access`→各 lane→`summary`）

| job (job_id) | instance | lane | 検証種別 |
| --- | --- | --- | --- |
| `ruviform-access` | 1 | preflight | ruviform 依存 access-mode 解決（deploy_key/anonymous/unavailable） |
| `verify` | ×3 OS | rust-verify | pinned toolchain 解決・lockfile 単一 rev・headless 境界・fetch・check-ci/lint/test |
| `ui-verify` | ×3 OS | ui-verify | headless UI（vendor REV==lockfile SHA 強制） |
| `webview-verify` | ×3 OS | webview-verify | 実 webview（WebKitGTK headers 等） |
| `gui-real` | 1 | gui-real | 実 GUI gate（現状 not-run を記録する fail-closed placeholder） |
| `summary` | 1 | evidence | 全 lane 集約 gate（not-run/cancelled → fail） |

### synvelis `ci.yml`（on: push/PR/dispatch。各 lane 並列→`evidence`）

| job (job_id) | instance | lane | 検証種別 |
| --- | --- | --- | --- |
| `verify` | ×3 OS | rust-verify | fmt/check/lint/test・prek・branch-follow manifest |
| `rt-verify` | 1 | rt-verify | RT 契約（rt_ 選択の非空強制） |
| `gw10-verify` | ×3 OS | gw10-verify | GUI/Window lane（GW10 契約） |
| `msrv-verify` | 1 | msrv-verify | MSRV（rust@1.89.0） |
| `dep-guard` | 1 | dep-guard | 依存監査・credential cleanup |
| `evidence` | 1 | evidence | 全 lane 集約 gate（needs 全 job） |

## 4. 代表 Run と指標（`runs.jsonl` から再導出可能）

| repo / run | event | SHA | concl | wall_s | queue_s | exec_span_s | critical_path_s | runner_total_s | jobs |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ruviform [41780346388](https://github.com/scarecrow173/ruviform/actions/runs/41780346388) | push(main) | `8fb665f1` | success | 318 | 0 | 315 | **313** | 1738 | 12 |
| audio-lens [41770611162](https://github.com/scarecrow173/audio-lens/actions/runs/41770611162) | push(main) | `111d473b` | success | 326 | 0 | 324 | **319** | 1321 | 12 |
| synvelis [41782019309](https://github.com/scarecrow173/synvelis/actions/runs/41782019309) | pull_request | `326ef96d` | success | 228 | 0 | 224 | **221** | 906 | 10 |

critical path の内訳（実測・design の最長 needs chain）:

- ruviform: needs なし全並列 → **ui-verify (windows-latest) 単独 313s が critical path**（wall≈job exec＋queue≈0）。
- audio-lens: `ruviform-access(3) → verify (windows-latest)(313) → summary(3)` = 319。gui-real は placeholder で critical 外。
- synvelis: `verify (windows-latest)(218) → evidence(3)` = 221（critical path は verify-windows 終了を待つ evidence gate で確定）。

Issue 本文の観測値との照合: ruviform UI Windows ~313s=313 ✓・audio-lens verify Windows ~313s=313 ✓・synvelis verify Windows ~218s=218 ✓（いずれも job_exec_s で一致）。

## 5. Job 別 timing（代表 run・wait/exec 分離）

### ruviform 41780346388（秒）

| job | lane | wait | exec |
| --- | --- | --- | --- |
| ui-verify (windows-latest) | ui-verify | 5 | **313** |
| verify (windows-latest) | rust-verify | 3 | 273 |
| ui-verify (ubuntu-latest) | ui-verify | 3 | 253 |
| ui-verify (macos-latest) | ui-verify | 4 | 236 |
| verify (macos-latest) | rust-verify | 4 | 191 |
| verify (ubuntu-latest) | rust-verify | 3 | 135 |
| bench | bench | 3 | 77 |
| l1-real-verify (windows-latest) | l1-real-verify | 3 | 73 |
| rt-verify | rt-verify | 3 | 65 |
| l1-real-verify (macos-latest) | l1-real-verify | 3 | 52 |
| l1-real-verify (ubuntu-latest) | l1-real-verify | 3 | 40 |
| icc-verify | icc-verify | 3 | 30 |

lane 合計: `ui-verify=802・rust-verify=599・l1-real=165・bench=77・rt=65・icc=30`。step lane 合計: `rust=1245・mise=189・dep-fetch=60・runner-overhead=50・evidence=44・checkout=42・verify-other=43・cache-restore=24・playwright=6`。

### audio-lens 41770611162（秒）

| job | lane | wait | exec |
| --- | --- | --- | --- |
| ruviform-access | preflight | 2 | 3 |
| verify (windows-latest) | rust-verify | 8 | **313** |
| webview-verify (windows-latest) | webview-verify | 7 | 234 |
| verify (ubuntu-latest) | rust-verify | 14 | 183 |
| webview-verify (ubuntu-latest) | webview-verify | 7 | 180 |
| webview-verify (macos-latest) | webview-verify | 7 | 138 |
| verify (macos-latest) | rust-verify | 38 | 121 |
| ui-verify (windows-latest) | ui-verify | 7 | 77 |
| ui-verify (ubuntu-latest) | ui-verify | 7 | 37 |
| ui-verify (macos-latest) | ui-verify | 7 | 29 |
| gui-real | gui-real | 3 | 3 |
| summary | evidence | 323 | 3 |

lane 合計: `rust-verify=617・webview-verify=552・ui-verify=143・preflight=3・gui-real=3・evidence=3`。step lane 合計: `rust=785・mise=135・dep-fetch=119・checkout=94・verify-other=50・runner-overhead=47・ui=38・evidence=17・cache-restore=2`。注: `verify (macos-latest)` の job_wait=38s は runner 割当待ち（他 job の 2〜14s と比較して突出）。

### synvelis 41782019309（秒）

| job | lane | wait | exec |
| --- | --- | --- | --- |
| verify (windows-latest) | rust-verify | 3 | **218** |
| verify (macos-latest) | rust-verify | 4 | 177 |
| verify (ubuntu-latest) | rust-verify | 3 | 129 |
| msrv-verify | msrv-verify | 3 | 123 |
| gw10-verify (windows-latest) | gw10-verify | 3 | 76 |
| rt-verify | rt-verify | 3 | 65 |
| gw10-verify (macos-latest) | gw10-verify | 3 | 47 |
| gw10-verify (ubuntu-latest) | gw10-verify | 3 | 43 |
| dep-guard | dep-guard | 3 | 25 |
| evidence | evidence | 224 | 3 |

lane 合計: `rust-verify=524・gw10-verify=166・msrv-verify=123・rt-verify=65・dep-guard=25・evidence=3`。step lane 合計: `rust=534・evidence=138・mise=128・runner-overhead=28・checkout=29・cache-restore=9・dep-fetch=6・verify-other=5`（gate/Verify 系 step の合計。evidence job 自体は 3s で wait=224s — 全 lane 完了を待つ gate）。

## 6. 統計（`ci` workflow・`conclusion=success && run_attempt=1`・直近サンプル）

| repo | n(sampled) | n(success&att1) | wall p50_s | wall p95_s | min–max_s | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| ruviform ci | 20 | 14 | 302 | 458 | 283–458 | p95 は PR 集中時間帯の runner 待ちを含む |
| audio-lens ci | 2 | 2 | — | — | 253–326 | n<10 → 観測値 `[253,326]` のみ（p50/p95 不報告） |
| synvelis ci | 14 | 10 | 242 | 301 | 199–301 | failure 4 件は別枠（下表） |

failure 別枠（統計に含めない）: ruviform ci failure 6/20・consumer-impact-check failure 5/20・consumer-verify failure 3/4（verify-audiolens/verify-synvelis leg の実敗 → summary fail-closed 赤。情報 lane・required 外だが「consumer 検証が現状失敗しがち」として記録）・synvelis ci failure 4/14。

## 7. Cache / Cold-Warm（`--logs-run` 代表 3 run の job log 解析・`cache` record）

| job 群 | mise tool cache | cargo cache | 転送サイズ | 判定 |
| --- | --- | --- | --- | --- |
| ruviform 41780346388 全12 job | **miss**（`mise cache not found`） | **hit**（`Cache hit for restore-key`） | ~10.3–26.2 MB/job | mixed（mise cold・cargo warm） |
| audio-lens 41770611162 verify/webview | **miss** | **miss**（lockfile hash 変更で key 不一致）→ `Cache saved` | 未記録（save 側） | **cold** |
| audio-lens ui-verify×3 | miss | cache 行なし | — | cold（cargo cache 対象外 lane） |
| audio-lens ruviform-access/gui-real/summary | — | cache 行なし | — | no-cache job |
| synvelis 41782019309 verify/gw10/rt/msrv | **miss** | **hit** | ~MB 級（size-bytes record 参照） | mixed |
| synvelis dep-guard | miss | cache 行なし | — | no-cache job |
| synvelis evidence | — | cache 行なし | — | no-cache job |

観測事実:

- **mise tool cache は 3 repo 全 job で一貫して miss**（`mise-v1-*` key で `cache not found`）。job あたり mise 復元に ruviform ≈23s・synvelis ≈13s・audio-lens ≈14s（step lane `mise` 内訳）。系統的 cold であり後続最適化の対象候補。
- cargo cache は ruviform/synvelis で hit（warm）、audio-lens では `Cargo.lock`（ruviform rev）更新に伴う key 変化で miss→save（cold）。**audio-lens の lockfile 更新頻度が cache 効率の支配因子**。
- 転送コスト: ruviform hit 側 10.3–26.2 MB/job・restore step 1–2s（大きくない）。audio-lens は save 側コストのみ観測。
- Playwright cache: 現行 workflow に Playwright step なし → **NotApplicable**（#332 向けの手順のみ）。

## 8. Before/After 測定手順（後続 Issue 用）

1. Baseline 再取得: `mise exec -- node ci/metrics/collect-ci-metrics.mjs --runs 20 --out <dir>`（変更前 SHA の run 群）。
2. After 取得: 変更 merge 後の run 群で同コマンド。cache 解析は代表 run に `--logs-run`。
3. `docs/designs/issue-327-cross-repo-ci-baseline.md` の **Before/After テンプレート**に転記。必須: 条件欄（repo/SHA/toolchain/OS/profile/features/Cargo.lock/cache 状態/n）・指標列・`required_contexts`/`workflow_jobs` 差分。
4. 判定: 効果（wall/critical path/runner total の Δ）と非回帰（required contexts・job 集合・lane 結果の差分ゼロ）を分離して報告。`NotRun`/`Unsupported` を `Pass` にしない。

## 9. 測定限界・NotRun/Unsupported 明細

| 項目 | 状態 | 理由 |
| --- | --- | --- |
| runner 内部 queue 分解 | Unsupported | Actions API が公開しない。`job_wait_s` 近似で代替 |
| sccache | NotApplicable | 現行 workflow 未導入（#329 の評価対象） |
| Playwright cache | NotApplicable | 現行 workflow に Playwright step なし（#332） |
| rulesets required checks | NotRun(0 件) | 3 repo とも rulesets 未設定（branch protection のみ有効・ruviform） |
| billing multiplier | NotRun | API 未公開。runner_total は実時間で報告 |
| audio-lens p50/p95 | NotRun | n=2（CI 新設 Issue #142 直後）→ 観測値報告 |
| gui-real 実 GUI 検証 | NotRun(placeholder) | audio-lens `gui-real` job は 3s の gate 記録 placeholder — 実検証ではないことを明記 |
| consumer-verify の consumer leg | 一部 Fail | dispatch 直近 4 run で 3 失敗（verify-audiolens/verify-synvelis）。required 外の情報 lane |
| step 合計 vs job_exec 合計 | 観測限界 | step 時間和は runner_total の ~97–98%（ruviform 1703/1738・AL 1287/1321・SV 877/906）。残差は step 間の runner 処理・境界丸めで、どの lane にも帰属させない |

## 10. 主要観察（後続 Issue への入力）

1. **3 repo とも critical path は `verify`/`ui-verify` の windows-latest job**（ruviform 313s ui・audio-lens 313s verify・synvelis 218s verify）。wall time ≈ 最長 job であり、単一 lane の短縮が wall に直結する構造。
2. **mise tool cache の系統的 miss**（全 job・全 repo）。job あたり 13–23s の固定コスト。
3. **rust lane（compile/check/clippy/test）が step 時間の支配項**（ruviform 1245s・audio-lens 785s・synvelis 534s・いずれも run 全体合計）。
4. **audio-lens の cargo cache は lockfile hash 直結**で頻繁に cold 化（ruviform rev 追従の度に全再構築）。
5. **集約 gate（audio-lens `summary`・synvelis `evidence`）は wait が critical path 長に等しい**が exec は ~3s — gate 自体のコストではなく依存完了待ち。
6. **consumer-verify（ruviform 側 dispatch lane）の consumer leg が直近で失敗しがち**（verify-synvelis/verify-audiolens）。情報 lane・required 外だが Cross-Consumer 検証の実効性に関わる。
