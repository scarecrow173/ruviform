# RF127 Linux 配布設計 — 既存 CI（Ubuntu）照合レジャー — Issue #127

- Issue: https://github.com/scarecrow173/ruviform/issues/127
- 収集日: 2026-11-14（UTC）
- 対象リポジトリ: scarecrow173/ruviform
- 作業 branch: `docs/127-linux-distribution`
- 収集方法: `gh run list` / `gh run view` / `gh api .../artifacts` / `gh run view --job <id> --log`（読み取りのみ。CI の増設・変更は行っていない）
- 本 slice の範囲: **Linux（`ubuntu-latest` hosted runner）の既存 CI lane** の対象 revision と成功 Run の照合。macOS/Windows leg・実 deb/AppImage/Flatpak 作成・署名・実機 GUI/Plugin 検証は対象外。

## 1. 判定語彙

`POLICIES.md` §3 に従う。`Pass`=対象 revision で実測成功の記録あり、`NotRun`=実行可能だが未実施、`Deferred`=利用要件成立まで保留、`Unsupported`=機構上の非対応。CI job の green は job/step 成功の証拠であり、個別 capability の Pass はテスト名・実行数が run log で確認できるものに限定する。

## 2. 対象 revision と Run の対応

### 2.1 main HEAD と最新成功 Run

| 項目 | 値 |
| --- | --- |
| 収集時点の main HEAD | `621490e8fa63a92bc5740e2030a7fbf40a5dbb1e`（PR #618 merge・2026-11-14。収集中に `c49e2a81`→`621490e8` へ前進したが、いずれも ci run なし） |
| main HEAD の `ci.yml` run | **なし**（`ci.yml` は workflow_dispatch 専用。次回は daily-ci scheduler の dispatch 待ち）→ **NotRun（Pending）** |
| main 上の最新 completed success run | run [43439434737](https://github.com/scarecrow173/ruviform/actions/runs/43439434737) — headSha `365c74ac4b8e696044fb9bae386dfd03f86ecf32`（PR #607 merge）、2026-11-13T14:05:23Z 開始、conclusion=`success`、全 20 job（19 lane + ci-summary）success |
| 直前の失敗 run（記録） | run [43436082797](https://github.com/scarecrow173/ruviform/actions/runs/43436082797) — headSha `05c7fc3b`（PR #603 merge）、`verify (ubuntu-latest)` の `actionlint (changed workflows)` prek hook が Fail → ci-summary も Fail。後続 commit で解消され 43439434737 で全緑。**Fail の記録として保持**（成功に転用しない） |
| `365c74ac..621490e8` の差分 | PR #608〜#618（code・docs・evidence 混在）。`crates/`・`mise.toml`・`.github/workflows/ci.yml` に差分あり（io-e2e fault seam・plugin ParentLease・ui-test window_job・ui-webview 文言・CI lint 配置変更等）→ `365c74ac` の CI 証拠は現行 main HEAD のコードを**直接は覆わない**。補助証拠として SHA 差分とともに分離記録する |

### 2.2 GPUI lane の系譜

| 出来事 | commit / PR | run |
| --- | --- | --- |
| `ui-gpui-verify` / `ui-gpui-offscreen-verify` lane 追加（Issue #314 RF-GT-05） | `ae70b57e`（PR #566） | — |
| 初期 main run で GPUI lane が Fail（Issue #582 として修正） | run [43140254535](https://github.com/scarecrow173/ruviform/actions/runs/43140254535)（`4f31460e`）・[43270011255](https://github.com/scarecrow173/ruviform/actions/runs/43270011255)（`246e364e`） | gpui 系 job = failure |
| 修正 merge（cold-cache `cargo fetch --locked`・libc pin・moxcms 境界） | `3692c6fa`（PR #593、Issue #582） | — |
| **GPUI lane を含む main 初の全緑 run** | `3692c6fa` | run [43326874974](https://github.com/scarecrow173/ruviform/actions/runs/43326874974)（18 job 全 success） |
| `msrv-verify` lane 追加（Issue #594） | `d6818bc0`（PR #595） | — |
| `ui-gpui-verify-macos` lane 追加（Issue #598） | `e93c438a`（PR #599） | run [43428172820](https://github.com/scarecrow173/ruviform/actions/runs/43428172820)（`e93c438a`、20 job 全 success） |
| Linux lint/hygiene 集約・native check/build/test の OS 配置変更 | `f5875b25`（PR #608） | — |
| 最新全緑 run（現行 lane 構成を全て含む） | `365c74ac` | run [43439434737](https://github.com/scarecrow173/ruviform/actions/runs/43439434737) |

### 2.3 関連 workflow の最新 run

| workflow | 最新 run | 対象 SHA | 結果 |
| --- | --- | --- | --- |
| `daily-ci.yml`（scheduler） | [43347698018](https://github.com/scarecrow173/ruviform/actions/runs/43347698018)（2026-11-13T01:49:15Z） | `3692c6fa` | success（decide / decide-consumer-verify とも success） |
| `consumer-verify.yml`（AL/SV 非回帰 lane・ubuntu-latest） | [43347745016](https://github.com/scarecrow173/ruviform/actions/runs/43347745016)（2026-11-13T01:49:39Z） | ruviform_rev=`3692c6fa` | success（preflight / verify-audiolens / verify-synvelis / summary 全 success） |

## 3. Ubuntu lane 台帳（run 43439434737・SHA `365c74ac`）

全 job conclusion=`success`。job URL は run 配下の個別 job。

| lane（job 名） | カバー範囲 | 主な実行内容（ci.yml / log 確認） | job URL | verdict |
| --- | --- | --- | --- | --- |
| `verify (ubuntu-latest)` | Native Build/Link・Headless・Lint | `prek run --all-files`、`fmt-check`、`lint-full`（`cargo clippy --workspace --all-targets --profile ci -- -D warnings`）、`test-full`（`cargo test --workspace --profile ci`）。当該 SHA の verify lane に明示的 `build-full`/`check-full` step はなく（windows/macos leg の log でも同じ 3 task のみ確認）、build/link は clippy 全 target compile と test binary の build/link/run で担保。システム依存 `libasound2-dev`（cpal/ALSA）を apt install | [job](https://github.com/scarecrow173/ruviform/actions/runs/43439434737/job/131932892523) | **Pass**（`test result: ok` 201 block・`FAILED` 0 を log で確認） |
| `rt-verify` | Headless/RT | `cargo test --workspace --locked rt_` 系（`mise run test-rt`）。strict `rt_` contract test 数 **166** を事前 step で計測後実行 | [job](https://github.com/scarecrow173/ruviform/actions/runs/43439434737/job/131932892476) | **Pass** |
| `icc-verify` | Feature | `ruviform-video-proc --features icc` の test+lint（`test-video-proc-icc`/`lint-video-proc-icc`） | [job](https://github.com/scarecrow173/ruviform/actions/runs/43439434737/job/131932891821) | **Pass** |
| `compression-verify` | Feature | io-compression feature matrix（default + gzip/deflate/zstd + combined）の check/test/lint | [job](https://github.com/scarecrow173/ruviform/actions/runs/43439434737/job/131932892436) | **Pass** |
| `io-structured-verify` | Feature | io-structured feature matrix の check/test/lint | [job](https://github.com/scarecrow173/ruviform/actions/runs/43439434737/job/131932892475) | **Pass** |
| `ui-verify (ubuntu-latest)` | Headless/UI（旧 WebView 系を含む） | UI npm deps 導入後、webview/headless/components/viz/binding/guitest/showcase/browser-test の check/test/build 系 mise task + Playwright Chromium browser test。Linux leg は WebKitGTK dev headers を apt install | [job](https://github.com/scarecrow173/ruviform/actions/runs/43439434737/job/131932892672) | **Pass**（旧 WebView 経路の証拠であり GPUI 配布の必須条件ではない — design §Legacy 分離を参照） |
| `bench` | Headless/bench smoke | `mise run bench-smoke` | [job](https://github.com/scarecrow173/ruviform/actions/runs/43439434737/job/131932892836) | **Pass** |
| `l1-real-verify (ubuntu-latest)` | Feature/L1 Plugin host | `ruviform-plugin-clap`/`ruviform-plugin-vst3` `--features host` の build + lib/bin test（65+2+74+2 passed）+ `l1_real_adapter_e2e`（`--ignored`、3+3 passed）。repo 内 fixture plugin の L1 host E2E であり実商用 Plugin ではない | [job](https://github.com/scarecrow173/ruviform/actions/runs/43439434737/job/131932892510) | **Pass**（fixture L1。実 Plugin EXT-L3 ではない） |
| `ui-gpui-verify (ubuntu-latest)` | Feature/Headless（GPUI Driver） | GPUI Linux 依存 apt セット導入 → `cargo fetch --locked` → driver boundary guard・driver contract・parity・native slot・component-view・golden・scenario contract・`check/lint/test-ui-gpui`（`gpui-test uitest` feature）・consumer observation seam | [job](https://github.com/scarecrow173/ruviform/actions/runs/43439434737/job/131932892804) | **Pass**（lib 55 + consumer_observation 22 + lab_story 4 + scenario_contract 7 + windowed_uitest 2 等、全 `0 failed` を log で確認。`gpui-test` headless であり実 display/実入力ではない） |
| `ui-gpui-offscreen-verify` | Headless/L0 offscreen（GPUI・X11 実 window） | GPUI 依存 + `xvfb xauth openbox x11-utils fonts-dejavu-core fontconfig` を apt install → `mise run test-ui-offscreen-xvfb`（Xvfb+openbox+lavapipe で `cargo test -p ruviform-ui-gpui --features real-scenario --test real_window -- --test-threads=1`）。証拠 artifact `offscreen-evidence-attempt-1` upload | [job](https://github.com/scarecrow173/ruviform/actions/runs/43439434737/job/131932892829) | **Pass**（`real_window` **3 passed / 0 failed**・102.92s を log で確認。hosted Ubuntu での初の GPUI offscreen 実測。Xvfb+openbox+lavapipe の L0 環境であり実機 GUI ではない） |
| `msrv-verify` | MSRV（rust@1.89.0） | `test-ofx-msrv`・`test-ui-test-msrv`・`test-observability-msrv`・`test-io-msrv` の 4 crate set を `mise exec rust@1.89.0` で実行 | [job](https://github.com/scarecrow173/ruviform/actions/runs/43439434737/job/131932892455) | **Pass**（本 slice ではローカル MSRV タスクは実行しないが、CI 上の MSRV lane 成功は記録する） |
| `ci-summary` | 集約 | 全 lane の fail-closed verdict 集約 + `test-ci-summary` self-test + checkout-sha artifact 照合 | [job](https://github.com/scarecrow173/ruviform/actions/runs/43439434737/job/131937542327) | **Pass**（verdict=PASS） |

## 4. checkout SHA 証拠（Issue #366 sha evidence）

`ci.yml` の各 lane は `actions/checkout` に `ref: ${{ github.sha }}` を pin し、`git rev-parse HEAD` を `checkout-sha-<leg>-attempt-<n>` artifact として upload する。本 run の artifact を `gh api` で取得し、以下の leg の実測 SHA が `365c74ac4b8e696044fb9bae386dfd03f86ecf32` と一致することを確認した。

| leg | 実測 SHA |
| --- | --- |
| `verify-ubuntu-latest` | `365c74ac…`（一致） |
| `ui-gpui-verify-ubuntu-latest` | `365c74ac…`（一致） |
| `ui-gpui-offscreen-verify` | `365c74ac…`（一致） |
| `rt-verify` / `l1-real-verify-ubuntu-latest` / `msrv-verify` | artifact 名の存在を `gh api .../artifacts` で確認（個別取得は代表 leg に限定） |

## 5. 受入条件カテゴリへの写像

| 受入条件の観点 | 対応する既存 CI 証拠 | verdict |
| --- | --- | --- |
| Native Build/Link（Linux） | `verify (ubuntu-latest)` の `lint-full`（clippy `--all-targets` compile）+ `test-full`（workspace test binary の build/link/run）。当該 SHA の verify lane に明示的 `build-full`/`check-full` step は存在せず、build/link/run は clippy 全 target compile と test 実行で担保 | **Pass @ `365c74ac`**（現行 HEAD `621490e8` は NotRun(Pending)） |
| Headless（Linux） | `test-full` の headless test 群、`rt-verify`（rt_ 166 件）、`ui-verify` headless UI contract、`ui-gpui-verify` の `gpui-test`/`uitest` headless、`ui-gpui-offscreen-verify` の Xvfb+openbox+lavapipe 実 window（L0） | **Pass @ `365c74ac`** |
| Feature（Linux） | `icc-verify`・`compression-verify`・`io-structured-verify`・`ui-gpui-verify`（`gpui`/`gpui-test`/`uitest` feature）・`l1-real-verify`（`host` feature）・`msrv-verify` | **Pass @ `365c74ac`** |
| 依存（Linux） | 各 lane の apt 依存導入（`libasound2-dev`・GPUI 依存セット・offscreen runtime・WebKitGTK headers）+ `cargo fetch --locked`/`Cargo.lock` 解決。**dlopen 依存（`x11-dl`/`ash`）は link 時依存に現れないため `ldd` 相当の検査 lane は存在しない** | build/実行系は **Pass @ `365c74ac`**。dlopen 実行時依存の網羅検査は **CI 定義なし = NotRun**（実機/配布 package 検証時の検査項目として design に記録） |
| ABI（Linux） | 専用 ABI lane は存在しない。domain ABI 契約（musical-transport ABI 等）は `test-full` の workspace test に内包。native link+run の成功が ABI 破壊の主要検出経路 | **Pass @ `365c74ac`（test-full 内包分）**。専用 ABI 検査 lane は **CI 定義なし = NotRun**（増設はしない — POLICIES §4） |
| 実機 GUI / 実 Plugin / 実 Device（Linux） | 既存 CI に該当 lane なし（`ui-gpui-offscreen-verify` は Xvfb headless であり実 display ではない。`l1-real-verify` は repo 内 fixture plugin） | **Deferred / NotRun**（利用要件発生まで。NotRun≠Pass） |
| Consumer 非回帰（AL/SV・ubuntu） | `consumer-verify.yml` run [43347745016](https://github.com/scarecrow173/ruviform/actions/runs/43347745016) @ ruviform_rev `3692c6fa`（verify-audiolens/verify-synvelis success） | **Pass @ `3692c6fa`**（最新 main HEAD `621490e8` の run なし → HEAD 分は NotRun(Pending)） |

## 6. 正直な記録（Pass に繰り上げない項目）

- **現行 main HEAD `621490e8` の OS-L1 CI は NotRun（Pending）**。`365c74ac` の成功は祖先 SHA の証拠であり、`365c74ac..621490e8` に `crates/`・`mise.toml`・`ci.yml` の差分があるため HEAD の Pass とはしない。統合検証は merge 後の対象 SHA の必須 CI（daily-ci dispatch）を Coordinator が確認する。
- **`ui-gpui-offscreen-verify` の Pass は L0（headless）証拠**であり、実機 display・実 GPU・実入力・実 session の Pass ではない。Xvfb+openbox+lavapipe（mesa ≥24.2）のソフトウェア Vulkan 環境での window 生成・描画・pixel readback の実測。
- **実 deb/AppImage/Flatpak の作成・署名・install 検証は本 Issue の範囲外**であり、いずれの形式も `保留`（design §Package Format 参照）。
- **実 Linux GUI（実 window 操作）・実 Plugin Dock・実 Device は Deferred/NotRun**。#388/#389/#419 の実機 lane と各 Consumer の利用要件発生時に評価する。
- **`ui-verify (ubuntu-latest)` の成功は旧 WebView/WebKitGTK 経路を含む lane の成功**であり、GPUI 配布の必須条件の証拠としては扱わない（design の Legacy 分離節を参照）。

## 7. 参照

- Issue #127 本文（受入条件の正本）: https://github.com/scarecrow173/ruviform/issues/127
- 設計正本（PR #529/#573 由来・本 slice で更新）: `docs/designs/linux-distribution-design.md`、先行版 `docs/designs/issue-127-linux-dist-design.md`
- 関連証拠: `evidence/rf388-os-l1-ci/report.md`（同型の OS-L1 CI 照合・Issue #388）、`docs/impl/ir/gt-05-ci-offscreen-lanes-slice.md`（L0 offscreen lane の Docker Debian 実測）、`docs/impl/exp/linux-container-device-and-arch-verification.jsonl`（旧 GTK 経路のコンテナ検証）
- CI 定義: `.github/workflows/ci.yml`（workflow_dispatch 専用・全 lane 無条件実行 + fail-closed `ci-summary`）、`.github/workflows/daily-ci.yml`（scheduler）、`.github/workflows/consumer-verify.yml`（AL/SV 非回帰）
