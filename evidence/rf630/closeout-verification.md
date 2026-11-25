# Issue #630 closeout — 検証記録（docs-only slice）

Issue #630（VST3/CLAP Test Plugin Suite）の closeout docs-only slice として、
最新 main と canonical ledger/evidence を再 census し、AC disposition・docs
status・closeout 証拠を整合させた記録である。本ファイルは closeout の証拠
hub であり、実行証拠（census 手法・参照 sha/count・検証コマンドと exit
code）をここに集約する。AC 行別の生データ表は `evidence/rf630/ac-inventory.md`、
disposition 確定は `docs/specs/issue-630-ac-inventory.md` を参照。

## 実行環境と census 条件

| 項目 | 値 |
| --- | --- |
| 実施日 | 2026-11-26（UTC 2026-11-25T21:00Z 台） |
| OS | Windows (win32/x64)、Git Bash MINGW64 |
| worktree | Orca 管理 worktree（rf630-closeout） |
| ブランチ | `docs/630-closeout` |
| base SHA | `ab6dc99fe8935ac09082d05e2176b10e729ba263`（census 時点の origin/main と一致） |
| census 手法 | canonical ledger/証拠ファイルの **read-only parse**（Python による JSONL/JSON 集計）と `gh issue view` による read-only 参照。これは census 手法であって canonical 検証ではない。canonical 検証は本 slice では `mise exec -- prek run --all-files`（docs-only の正規コマンド） |

## Issue checkbox snapshot（2026-11-26・`gh issue view 630` read-only）

- state: **OPEN**
- AC1〜AC6: `[x]`（適用済み） / AC7: `[ ]`（未適用）
- 本 slice は Issue 本文・checkbox・close を変更しない。AC7 の checkbox 適用
  判断と Issue close は Coordinator の作業として残る。

## 参照した canonical ledger / evidence（read-only・再生成なし）

| 証拠 | sha / 条件 | count / verdict 集計 | qualification |
| --- | --- | --- | --- |
| `evidence/rf630/plugin-dock-real-clap.jsonl` | repo.sha `2dfd2e0fc0e3928669ccd51dad330b0b2795903b`・`run_id=rf630-plugin-dock-real-clap`・lane `test-ui-plugin-dock-real`・windows/x86_64 | 14 case = **14 pass**（non-pass なし） | `real-clap-editor-{input,focus,popup}` は `ruviform.editor-ops` seam 経由の effect-verified。`real-clap-session-{crash,hang}` は fault-carrier（`fault_armed` step pass・exit_code/exit_signal・watchdog kill 実測）。artifact digest `ruviform_plugin_test_clap.dll@7d7f99c1fe12…`（evidence row 記録） |
| `evidence/rf631/plugin-dock-real-scenario.jsonl` | repo.sha `2dfd2e0fc0e3928669ccd51dad330b0b2795903b`・`run_id=rf631-plugin-dock-real` | 32 case = **32 pass**（non-pass なし） | VST3 leg。artifact digest `ruviform_plugin_test_vst3.dll@0b2081ab8d25…` |
| 両 ledger の focus case | 同上 | `real-editor-focus` / `real-clap-editor-focus` とも verdict **pass** | **qualified pass**: `set_focus_target(HostWindow)` step は `landing:"accepted-declared"`・`effect_verified:false`。`f:0` token の delivery は検証済みだが、app-thread container への OS focus move materialize は lane-topology 依存で未検証。同-queue `host_path` lane が `GetFocus` end-to-end の真の oracle。verdict を fail に倒さず、かつ effect-verified Pass と混同しない |
| `evidence/rf631/plugin-dock-scenario.jsonl`（headless） | repo.sha `8a000038ad687117d4f3d428d086459f47092001` | 9 case = 7 pass + `editor-attach` unsupported + `dock-pixels` not_run | unsupported/not_run は fixture 法則の構造的 verdict（`test_gain` に editor surface なし）。real lane の `test_editor` が dock/画素判定を担う |
| `evidence/rf630/plugin-dock-real-clap-headless.jsonl` / `evidence/rf631/plugin-dock-real-headless.jsonl` | repo.sha `8a000038ad687117d4f3d428d086459f47092001`・`--blocked-probe` | CLAP 14/14 row・VST3 32/32 row 全 `not_run` + `environment_blocked` | fail-closed 実証 — headless 環境で real verdict を偽造しない |
| `evidence/rf630/abi-validator/run.json` | schema `rf630-abi-validator-run/2`・実走 HEAD `5099cbe5836b5d53735951463e9244e7a840991c`（2026-11-23T14:56–14:57Z） | evaluation verdict **Passed**・canonical **7/7 leg Passed**（failed 0・notRun 0・unsupported 0） | canonical legs: CLAP 6 plugin（test_gain 30成功/test_fault 22成功/test_stateful 34/test_instrument 33/test_editor 20/test_spawn_fail 8成功+35 declined は宣言的 expectation met）+ VST3 **282 passed/0 failed**。validator: clap-validator pin-47e2424（commit `47e242404df89f3c8454aaf1c02bb0caca6db2ce`・`0.4.1-5-g47e2424`・sha256 `b4bd645877bba3d5abe4300a84c5785312ce16f7d1179a2fd3177eedc45afe26`・license MIT）/ VST3 SDK validator（`v3.8.1_build_84`・commit `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`・sha256 `5889a89685cd0362941659011ec82d8584e0ebb39d77030d1a2b2cbd2d68a4fd`・license MIT License per `vst3sdk/LICENSE.txt`）。fixture artifact digest: CLAP `f8dc6cdc…`・VST3 `e3cbf964…` |
| 同上 — 非 Pass 記録 | 同上 | `test_fault` leg に `param-(set\|fuzz)` 宣言的除外（reason: fixture の stepped Fault param が意図的に abort/exit/hang を arm する設計由来・fatal surface は L1 out-of-process fault lane が担う）。informational unfiltered fault leg は **Unsupported**（exit 101・param-fuzz tool panic）で evaluation 集計に含まない | 宣言的除外を Pass に読み替えない。preset-discovery crawl/load の除外は pin-47e2424 で解消済み（raw output で success 実測）。upstream release 後の再評価は `scarecrow173/ruviform#867` |
| `evidence/rf630/consumer-oracle/manifest.json` | schema `rf630-fixture-contract/1`・created 2026-11-21 | 7 fixture class pin・`consumerOracleMap` 5 行（sv#101/#127/#128/#176 = Required/`headless-contract-match`・al#127 = NotApplicable） | dep 非汚染 2026-11-25 再測: audio-lens `Cargo.lock` は fixture crate 名非含有（exit 1 = 期待値）。synvelis `Cargo.lock` は含有するが consumer の自主採用（sv#176 S1・dev-dependency・test-only）で production DAG 非含有 — 「強制しない」は成立。consumer 実走 legs は全て external-owner。sv#176 S1 の headless legs は synvelis PR #640 で実走済みだが Native Dock/PD 受入は未実走 |
| `.github/workflows/ci.yml` / `daily-ci.yml` | closeout base `ab6dc99f`（read-only 参照） | `ci.yml` は `workflow_dispatch` のみ（push/PR 自動 trigger なし）。`verify`（ubuntu+windows）/`verify-macos` が `test-full`、`l1-real-verify`（ubuntu+windows）/`l1-real-verify-macos` が `test-l1-real-plugin` を実行。`ci-summary` は run-level fail-closed 集約だが PR required context ではない。`daily-ci.yml` が main 検証を集約 dispatch | AC6 は CI 構造として充足。merge 後対象 SHA の CI 実走証拠の収集は Coordinator gate |

## AC 行別 disposition（closeout 確定）

| # | disposition | 根拠の要約 |
| --- | --- | --- |
| 1 | **Pass** | 両 format の production bridge 経由 host_path tests・headless `host-lifecycle`・L1 e2e・real attach 実走。Fake Plugin Port 非経由 |
| 2 | **Pass**（残件: OS-API MIDI = rf#762 gate） | 4 fixture + spawn/pre-Hello fault + L1 isolation + real-session legs。CLAP crash/hang は fault-carrier で pass 化済み |
| 3 | **Pass**（宣言的除外は non-pass 記録） | canonical 7/7 leg Passed・pin-47e2424/MIT・VST3 3.8.1/MIT License・282 passed/0 failed |
| 4 | **Pass**（`HostWindow` focus は qualified pass） | VST3 32/32・CLAP 14/14 全 pass @`2dfd2e0f`。caveat は上表のとおり |
| 5 | **Pass** | ledger の sha/digest/seed/environment tuple + watchdog kill + deadline policy + exactly-once |
| 6 | **Pass**（CI 構造として充足） | `test-full`/`test-l1-real-plugin` の 3OS 収載・GUI-L2/EXT-L3 は opt-in。merge 後 CI 証拠は Coordinator gate |
| 7 | **部分 Pass**（ruviform 側 done・consumer 実走 external-owner） | contract match・al#127 NotApplicable・dep 非汚染は提供済み。false Pass にしない — `[ ]` 維持が整合 |

## external-owner / residual 一覧

- consumer 実走 legs（sv#101/#127/#128/#176・al#127）— external-owner。各
  consumer Issue が AC/lane 責務。sv#176 S1 の headless legs は実走済みだが
  Native Dock/PD 受入は未実走。
- `test-os-endpoint-midi-e2e`（rf#629 scope）— provisioned runner 不在で
  NotRun。runner は rf#762 gate（#630 直接対象外）。
- clap-validator upstream release — param-fuzz panic 修正と pin-47e2424 の
  release 版切替え再評価は `scarecrow173/ruviform#867`（OPEN）。
- `HostWindow` focus leg の OS focus move 検証 — lane-topology 依存のため
  real lane では `accepted-declared`。真の oracle は同-queue `host_path` lane。
- macOS/Linux GUI lane — POLICIES §2 の Deferred。
- implementable 残件は closeout 時点で解消済み（CLAP editor verbs・fault
  carrier ともに実装・実走 pass）。

## NotRun とした作業と理由

- **computer-use による実 GUI 受入: NotRun** — 2026-11-26 ユーザー決定
  「computer-use 受入は不要。今ある記録（real lane ledger pass）で十分として
  closeout を進める」に従う。
- **real lane の再実行: NotRun** — docs-only slice であり、canonical real
  ledger は read-only 正本。既存 ledger（`2dfd2e0f`・全 pass）を closeout
  証拠として採用する。
- **重い Rust 全体検証（`check-full`/`lint-full`/`test-full`/`build-full`/
  `verify-full`）・GUI/real lane 実走: NotRun** — docs-only 変更のため
  POLICIES §5 の比例原則で対象外。統合検証は merge 後の対象 SHA に対する
  Daily CI が担う（2026-11-26 ユーザー指定: 手動 dispatch をゲートにしない）。

## 実行した検証

| コマンド | 種別 | exit | 結果 |
| --- | --- | --- | --- |
| `mise exec -- prek run --all-files` | **canonical**（docs-only slice の正規コマンド） | 0 | Pass — 全 hook |
| `mise run fmt-check` | 補助 | 0 | Pass |

（注: 上表は最終実行の結果。実行ログの詳細は PR 本文の Verification 節と
同一内容。canonical/補助の区別は表の種別欄どおり。census に用いた Python
parse は検証ではなく read-only の集計手法。）

## Coordinator に残る作業

- Issue #630 の checkbox 最終判断（AC7 の扱い）と Issue close。
- docs index（`docs/**/README.md`）の統合時整備 — 本 slice は一切編集して
  いない。
- merge 後の対象 SHA に対する CI 実走証拠の収集（AC6 の統合検証ゲート）。
- AC7 の consumer 実走 legs は external-owner のまま残るため、close 判断時は
  本ファイルと `docs/specs/issue-630-ac-inventory.md` の部分 Pass 記述を参照。

## 補足: 範囲外で観測した不整合（報告事項）

- `crates/ruviform-plugin-test-clap/README.md` の `test_editor` 節は
  「Parameterless pass-through DSP（no parameters declared at all）」と記述
  しているが、現行実装（`src/editor.rs` の `TestEditorProcessor(TestFaultProcessor)`
  delegation・params extension surface）は共有 `Fault` param を持ち、real
  ledger の `fault_armed` step pass と整合する。crate README は fault-carrier
  導入以前の記述が残っている可能性がある。`crates/` は本 slice の占有範囲外
  のため修正していない — Coordinator/後続 slice への報告事項。
