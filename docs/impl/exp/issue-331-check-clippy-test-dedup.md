---
id: "EXP-331-check-clippy-test-dedup"
title: "Issue #331 check/clippy/test 重複評価実測と verify lane 統合効果"
created: "2026-11-04"
updated: "2026-11-04"
---

# Issue #331 check/clippy/test 重複評価実測と verify lane 統合効果(2026-11-04)

- 計測契約: `docs/designs/issue-327-cross-repo-ci-baseline.md` + `docs/designs/issue-331-check-clippy-test-dedup.md`
- 計測ツール: `mise exec -- node ci/metrics/collect-ci-metrics.mjs --repo scarecrow173/ruviform --workflow ci --runs 5 --logs-run 41816075282 --logs-run 41816923217 --logs-run 41820870299 --out ci-metrics-331`
- Raw 証拠: `docs/impl/exp/data/issue-331-verify-step-runs.jsonl`(verify job step 時間 180 rows・5 runs × 3 OS)

## 1. 測定条件

| 項目 | 値 |
| --- | --- |
| ローカル計測環境 | Windows 11 x64 / Git Bash / rust 1.99.0(mise pin)/ `CARGO_INCREMENTAL=0` / `--profile ci` |
| 計測方法 | 隔離 target dir 2 系列。A: `cargo check` → `cargo clippy` → `cargo test`、B: `cargo clippy` → `cargo check` → `cargo test`。`--message-format=json` で compiler-artifact を per-unit 集計 |
| 対象 run(baseline) | 41816075282(#329 cold)・41816923217(#329 warm)・41817451132・41818226222・41819703706・41820208123・41820870299(main push run) |
| workspace | 34 packages(lib 32 / bin 8 / test target 50 / bench 4 / example 9 / custom 3) |

## 2. ローカルユニット集合実測

### ユニット数(--message-format=json artifact 集計)

| 系列 | コマンド | wall | ユニット内訳 |
| --- | --- | --- | --- |
| A | `cargo check`(cold) | 31s | 292 units = 140 ws + 152 dep |
| A | `cargo clippy`(check 後) | 9s | 152 dep **fresh 共有** + 140 ws を clippy-driver で再コンパイル |
| A | `cargo test` | 77s | 別系列(codegen + cfg(test)) |
| B | `cargo clippy`(cold) | 32s | 292 units = 140 ws + 152 dep(check と同一集合) |
| B | `cargo check`(clippy 後) | 8s | 152 dep fresh + 140 ws を rustc で再コンパイル |
| B | `cargo test` | 101s(並列計測負荷による膨張) | — |

- check/clippy は同一の workspace/package/target/feature/profile 選択で同一ユニット集合(292)を評価。
- 依存ユニット 152 は両コマンドで artifact 共有される。
- workspace メンバー 140 ユニットは `rustc` vs `clippy-driver` の compiler identity 差で二重コンパイル — **これが除去可能な重複 frontend pass**。
- `cargo test` は codegen ユニット(≈114 dep build units)+ `cfg(test)` ws ユニット(≈135)を持ち check-mode と別責務。共有は build-script/proc-macro 系 ≈39 ユニットのみ。
- `cfg(clippy)` / `cfg(not(clippy))` の使用は `crates/` 配下にゼロ — check と clippy でコードパス分岐なし。

## 3. CI baseline step 時間(変更前 run・秒)

`Type-check workspace (mise run check-ci)` / `Lint workspace (mise run lint-ci)` / `Test workspace (mise run test-ci)`:

| run | ubuntu check/lint/test | windows check/lint/test | macos check/lint/test |
| --- | --- | --- | --- |
| 41816075282(cold) | 49/10/100 | 77/14/128 | 89/19/163 |
| 41816923217(warm) | 31/10/87 | 51/12/84 | 76/20/140 |
| 41817451132 | 23/8/72 | 68/17/143 | 81/13/124 |
| 41818226222 | 20/6/64 | 49/12/102 | 47/9/132 |
| 41819703706 | 20/5/65 | 56/14/91 | 69/17/159 |
| 41820208123 | 16/10/75 | 51/13/94 | 55/16/107 |
| 41820870299 | 47/10/97 | 92/18/158 | 78/15/125 |

- check step は dep metadata コンパイルを吸収するため 16–92s と変動大。
- lint step は deps fresh 状態で 5–20s — 二重 ws-frontend 評価が step 単位で確認できる。
- 除去効果の期待値: check step 消滅分のうち lint 側へ移る dep 処理を差し引いた ws 再評価 + step overhead ≈ warm で 10–20s/job、cold でより大きい。

## 4. sccache 解釈

- #329 で verify/ui-verify に sccache 導入済み。warm run で ws メンバーは cache hit するが、check→clippy の rustc→clippy-driver 切替は compiler identity が異なり artifact を共有しないため、独立 check step は依然として 140 ws ユニット分の frontend 評価(または cache 問い合わせ)を要求する。
- sccache hit rate(warm verify: ubuntu 47.3% / windows 62.5% / macos 51.6%)は本変更後も dep 側で維持される見込み。統合による hit rate 悪化要因なし(check と clippy で dep 側 cache key は同一)。

## 5. 変更後 CI run(PR run)

- NotRun — PR 作成・`gh pr checks --watch` 完了後に `collect-ci-metrics.mjs` で対象 run を収集し `issue-331-verify-step-runs.jsonl` へ追記・本節を更新する。

## 6. ローカル検証結果

- `mise run verify-ci`(新列 fmt-check → lint-ci → test-ci): Pass(Windows 11・rust 1.99.0)。
- `mise exec -- actionlint .github/workflows/ci.yml`: Pass。
- `mise exec -- prek run --all-files`: Pass。
- 注入実験 8 ケース: 全件期待 lane で検出(IR「等価性証拠」表参照)。
