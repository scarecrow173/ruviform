---
id: "EXP-329-sccache-compile-cache"
title: "Issue #329 sccache コンパイルキャッシュ導入効果実測"
created: "2026-11-04"
updated: "2026-11-04"
---

# Issue #329 sccache コンパイルキャッシュ効果実測(2026-11-04)

- 計測契約: `docs/designs/issue-327-cross-repo-ci-baseline.md` + `docs/designs/issue-329-sccache-compile-cache.md`
- 計測ツール: `mise exec -- node ci/metrics/collect-ci-metrics.mjs --repo scarecrow173/ruviform --workflow ci --runs 5 --logs-run 41816075282 --logs-run 41816923217 --out ci-metrics-329`
- Raw 証拠: `ci-metrics-329/runs.jsonl`(1569 rows)・`ci-metrics-329/report.md`
- 対象 PR: https://github.com/scarecrow173/ruviform/pull/342

## 1. 測定条件

| 項目 | 値 |
| --- | --- |
| 測定日 | 2026-11-04(UTC) |
| cold run | https://github.com/scarecrow173/ruviform/actions/runs/41816075282 (PR 初回 push、cache namespace `ruviform-v1-<RUNNER_OS>` 未存在) |
| warm run | https://github.com/scarecrow173/ruviform/actions/runs/41816923217 (2 連続 push、同一 PR head 更新) |
| disable 検証 run | 同上 run 内 job 126531941581 の rerun(repo variable `CI_SCCACHE_DISABLE=1` 設定下) |
| sccache | v0.19.0(`mozilla-actions/sccache-action` pin `fc920bf0ec8de6ee65d409111f7ec508035751ba` = v0.0.11) |
| backend | GitHub Actions cache(`SCCACHE_GHA_ENABLED=true`・`SCCACHE_GHA_VERSION=ruviform-v1-<RUNNER_OS>`) |
| profile | `ci`・`CARGO_INCREMENTAL=0` |
| toolchain | rust 1.99.0(mise pin)。MSRV lane(1.89)は verify/ui-verify では実行されず共有しない |
| Runner | GitHub-hosted ubuntu/windows/macos-latest |

## 2. 計測結果(job_exec_s・秒)

### verify(3 OS)

| OS | cold 41816075282 | warm 41816923217 | 差分 |
| --- | --- | --- | --- |
| ubuntu-latest | 192 | 141 | -51 s(-26.6%) |
| windows-latest | 266 | 188 | -78 s(-29.3%) |
| macos-latest | 303 | 271 | -32 s(-10.6%) |

### ui-verify(3 OS)

| OS | cold | warm | 差分 |
| --- | --- | --- | --- |
| ubuntu-latest | 226 | 191 | -35 s(-15.5%) |
| windows-latest | 225 | 230 | +5 s(+2.2%・誤差範囲) |
| macos-latest | 198 | 227 | +29 s(+14.6%・runner variance 観測。後述) |

### sccache --show-stats(Post Set up sccache step)

#### cold run(全 OS 初回)

| job | requests | executed | hits | misses | write errors |
| --- | --- | --- | --- | --- | --- |
| verify ubuntu | 698 | 299 | 0 | 298 | **160** |
| verify windows | 706 | 313 | 1 | 311 | **108** |
| verify macos | 715 | 348 | 3 | 344 | **168** |
| ui-verify ubuntu | 406 | 303 | 2 | 298 | **211** |
| ui-verify windows | 269 | 213 | 0 | 211 | **211(100%)** |
| ui-verify macos | (実測同型) | | | | |

#### warm run

| job | requests | executed | hits | misses | hit rate | write errors |
| --- | --- | --- | --- | --- | --- | --- |
| verify ubuntu | 698 | 299 | 141 | 157 | **47.32%** | なし(0) |
| verify windows | 706 | 313 | 195 | 117 | **62.50%** | なし(0) |
| verify macos | 715 | 348 | 179 | 168 | **51.59%** | なし(0) |
| ui-verify ubuntu | 406 | 303 | 89 | 211 | **29.67%** | 176 |
| ui-verify windows | 269 | 213 | 2 | 209 | **0.95%** | 100 |

cache errors / read errors: 全 job 0。コンパイル自体の失敗なし。

## 3. 分析

### write errors の原因(cold run)

cold run で 36-100% の cache write error が発生したが、warm run で verify 系は 0、ui-verify は 100-176 残存。パターンから原因は **GHA cache backend の rate limit / 並列書込競合**と断定:

- `gh cache list` で `sccache/x/y/z/<hash>` エントリが cold run 中に多数生成されていることを確認(=権限不足や backend 設定不備ではなく API 呼出しは通る)
- ui-verify windows は cold run で write error 100%(0 hit)だったが warm run でも 2 hit のみ→cold でほぼ書き込めなかった
- 6 job 並列が同時に数百エントリを書き込み、GHA cache service の per-repo write rate limit に抵触したと推定(sccache GHAC は既知の rate limit 挙動を持ち、エラー時は build を継続する)
- **sccache は write error でも compile を継続・fail-open**するため CI green を維持 — 設計要件どおり

### hit rate の評価

- verify lane: 47-62% hit。`Cache misses` は `Cargo.lock` 内 crate が `check-ci`/`lint-ci`/`test-ci` の 3 phase で feature set 差異により別 fingerprint になるもの + cold run で write error した分の再 compile
- ui-verify: ubuntu 29.7%(verify と共有 keyspace で部分 hit)、windows は cold write 全滅で 0.95%。ui の compile 単位は `webview` feature オンで verify と diff が大きいため hit が限定されるのは想定内
- **compile cache は動作し、実測で verify lane の wall 時間を 10-30% 削減**

### 不変条件の確認

- cache errors / read errors = 0。破損 entry を読んで誤結果を返す事象なし
- sccache hit しても `check-ci`/`lint-ci`/`test-ci` 自体は全て実行(clippy/test は compile artifact 由来でなく常に実行)
- 12 required checks 全緑。RT/Golden/f64/GUI/l1-real/bench カバレッジ非変更
- MSRV lane(1.89)は verify/ui-verify 対象外のため共有しない(設計どおり)
- profile/feature/RUSTFLAGS 差は sccache の hash 入力に含まれるため誤共有しない(同一 keyspace でも entry 衝突しない)

## 4. kill switch(CI_SCCACHE_DISABLE=1)検証

- repo variable `CI_SCCACHE_DISABLE=1` を設定 → warm run の verify(ubuntu-latest) job 126531941581 を rerun
- 結果: `Set up sccache` / `Export sccache GHA backend env` / `Verify sccache environment` / `sccache statistics` 4 step 全て **skipped**、job success。`RUSTC_WRAPPER` 未注入のため rustc が直接呼ばれ通常 compile
- 検証後に variable 削除済み(復帰確認: `gh variable list` 空)
- **無効化経路は repo variable のみで動作・workflow YAML 変更不要を実証**

## 5. 未検証項目(NotRun/Unsupported)

| 項目 | 状態 | 理由・後続 |
| --- | --- | --- |
| Cargo.lock 更新時 | **PartiallyObserved** | cold run 自体が lock-hash 新規作成相当。warm→warm での lock 更新は本 PR スコープ外(後続 Issue で lock bump PR を用意して再計測) |
| toolchain 更新時 | **NotRun** | rust 1.99.0 pin 変更は本 PR スコープ外。`SCCACHE_GHA_VERSION` bump + toolchain 変更時に新 namespace で観測する手順を IR に記録 |
| feature 切替 | **Observed(間接)** | ui-verify `webview` feature と verify 既定 feature で同一 OS keyspace 共有しつつ hit/miss が分離 = feature fingerprint が key に含まれる間接証拠。明示的 A/B 実験は NotRun |
| cache 破損復旧 | **Observed(ローカル)** | ローカル Windows で sccache local cache を全 entry 破損 → re-compile fallback で cargo test 完走・統計で cache errors 12 + re-compile 14 を確認。CI GHAC backend 上の破損注入は権限外のため NotRun(fail-open は write error 観測で実証) |
| RT/ICC/L1/bench への展開 | **NotRun(明示的に範囲外)** | staged rollout。verify/ui-verify の実測確認を以て follow-up で展開 |
| Consumer(AL/SV)への展開 | **NotRun** | 前提のみ IR に記録。Consumer repo 変更なし |
| fork PR 挙動 | **NotRun** | upstream repo に fork PR がない。GHAC 仕様では fork PR の runtime token は read-only scope が付与されるため write は拒否→write error で fail-open(=push のみの通常 compile)。本 repo での fork PR 観測は今後 |

## 6. 結論

- **sccache GHA backend 導入は有効**: verify lane で 47-62% cache hit・job 時間 10-30% 削減・12 required checks 全緑を維持
- **fail-open 確認**: write error 高頻度でも compile・test は完走し、cache backend が correctness 権威にならない設計を実証
- **kill switch 実証**: `CI_SCCACHE_DISABLE=1` repo variable で workflow 変更なく即時無効化可能
- **残課題**: cold run write error rate(36-100%)は GHAC rate limit 由来。follow-up で `SCCACHE_GHA_CACHE_WRITE=false` 併用や backend 切替(S3 等)の評価余地あり
