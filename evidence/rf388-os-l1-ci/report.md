# RF388 OS-L1 既存CI証拠照合 report — Issue #388

- Issue: https://github.com/scarecrow173/ruviform/issues/388
- 対象 SHA: `e93c438adc4af5b498e10926fe859ac1338ac390`（main HEAD・PR #599 merge 時点で固定）
- 検証 run: Full CI `ci.yml` workflow_dispatch run [43428172820](https://github.com/scarecrow173/ruviform/actions/runs/43428172820) — 2026-11-13 dispatch、conclusion=`success`、`ci-summary` verdict=`PASS`（19 lane job + ci-summary の計 20 job が全て success）
- 収集日: 2026-11-13
- 作業 branch: `docs/388-os-l1-ci-evidence`
- 本 slice の範囲: Issue #388 の OS-L1 受入条件（既存 CI 内での契約・境界・Fallback/Detach/RT非干渉の Compile/Link/Headless 照合、macOS/Linux L1、検証 Scope 明記、#262/#419 引き渡し）に限定。Windows L2/L3 実 Plugin Dock 画素（#532 DComp 決裁待ち）と実 Plugin 受入（SV #176/#389 Owner）は対象外で Issue は open 継続。

## 検証スコープと Oracle（受入条件「検証Scope」対応）

| 項目 | 内容 |
| --- | --- |
| OS | `windows-latest` / `ubuntu-latest` / `macos-latest`（GitHub hosted runner） |
| 採用 Feature | workspace default（`check-full`/`lint-full`/`test-full`/`build-full` は `--all-features` なし）+ `gpui`/`gpui-test`/`uitest`（`ui-gpui-verify` 3 lane の `test-ui-gpui`）+ `real-scenario`（`ui-gpui-offscreen-verify` の `test-ui-offscreen-xvfb`）+ `host`（`l1-real-verify` 系の fixture cdylib + L1 runner bin）+ MSRV `rust@1.89.0`（`msrv-verify` の 4 crate set） |
| Device/Format | 音声・MIDI・Plugin は repo 内 fixture のみ。実商用 Plugin binary・物理 Device・外部環境は対象外（Windows でも未利用分は必須としない） |
| Run URL / Oracle | 上記単一 run URL。Oracle = 各 job/step の success、cargo test の `test result` 集計行、`checkout-sha-*` artifact の実測 SHA、`ci-summary` の fail-closed verdict |
| 新規 CI Workflow | 作成していない（既存 `ci.yml` の dispatch run のみを証拠とする） |

## Run / Job 一覧（全て conclusion=success）

Run: https://github.com/scarecrow173/ruviform/actions/runs/43428172820 （walltime 約 13 分、2026-11-13T12:47:36Z 開始・13:00:36Z 完了）

| job | OS | 結果 | job URL |
| --- | --- | --- | --- |
| verify | ubuntu-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691153) |
| verify | windows-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691209) |
| verify-macos | macos-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691243) |
| rt-verify | ubuntu-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691112) |
| icc-verify | ubuntu-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691223) |
| compression-verify | ubuntu-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894690963) |
| io-structured-verify | ubuntu-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691092) |
| ui-verify | ubuntu-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691255) |
| ui-verify | windows-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691163) |
| ui-verify-macos | macos-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894690674) |
| ui-gpui-verify | ubuntu-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691161) |
| ui-gpui-verify | windows-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691180) |
| ui-gpui-verify-macos | macos-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691184) |
| ui-gpui-offscreen-verify | ubuntu-latest (Xvfb) | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691093) |
| bench | ubuntu-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691316) |
| l1-real-verify | ubuntu-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691202) |
| l1-real-verify | windows-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691699) |
| l1-real-verify-macos | macos-latest | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894690956) |
| msrv-verify | ubuntu-latest (rust@1.89.0) | success | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691257) |
| ci-summary | ubuntu-latest | success（verdict=PASS） | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131900486762) |

## checkout SHA 照合（Issue #366 sha evidence）

全 19 checkout leg の `checkout-sha-*-attempt-1` artifact を取得し、実測 `git rev-parse HEAD` を照合した。**全 leg が `e93c438adc4af5b498e10926fe859ac1338ac390` で一致**（対象 SHA と同一）。`ci-summary` も event-sha / summary-checkout / workflow revision の 3 者一致を確認済み。

| leg | 実測 SHA |
| --- | --- |
| verify-ubuntu-latest / verify-windows-latest / verify-macos | `e93c438a…` |
| rt-verify / icc-verify / compression-verify / io-structured-verify | `e93c438a…` |
| ui-verify-{ubuntu,windows}-latest / ui-verify-macos | `e93c438a…` |
| ui-gpui-verify-{ubuntu,windows}-latest / ui-gpui-verify-macos / ui-gpui-offscreen-verify | `e93c438a…` |
| bench / l1-real-verify-{ubuntu,windows}-latest / l1-real-verify-macos / msrv-verify | `e93c438a…` |

## 受入条件 ↔ lane/test ↔ verdict 対応表

判定語彙は `POLICIES.md` §3 に従う。CI green は job/step 成功と cargo test 集計を根拠とし、個別 Case の Pass はテスト名・実行数が run log で確認できるものに限定する。実機・実 Plugin・実描画が絡む行は Pass に繰り上げず NotRun/Deferred とする。

### AC1: 公開契約・GPUI/OS private handle境界・Fallback/Detach/RT非干渉の Compile/Link/Headless 試験（3 OS・対象 SHA）

| 契約項目 | lane / test（mise task） | 実行 OS | 観測結果 | verdict |
| --- | --- | --- | --- | --- |
| 公開契約: `NativeSurfaceSlot` 型契約 + Fake Driver + 2-phase rollback（NSC-01〜13） | `test-ui-native-slot`（`cargo test -p ruviform-ui-test --test native_slot_contract`）in `ui-gpui-verify` | ubuntu / windows / macos | 各 OS で `running 13 tests` → `13 passed; 0 failed`（run log 確認） | **Pass**（headless 契約層。実 Dock Pass ではない） |
| 公開契約: Driver contract fixture（UI-DVR-01〜08、`ui_dvr_06_native_slot_unsupported_stale_destroy` 含む） | `test-ui-driver-contract` in `ui-gpui-verify` | 3 OS | `7 passed; 0 failed`（ubuntu log 確認） | **Pass** |
| 公開契約: driver parity / component-view / golden core / scenario contract | `test-ui-parity`(14) / `test-ui-component-view`(17) / `test-ui-golden`(8) / `test-ui-scenario-contract`(fake 16 + gpui 7) in `ui-gpui-verify` | 3 OS | 全て `0 failed`（ubuntu/macos log 確認） | **Pass** |
| GPUI/OS private handle 境界（公開 API への HWND/NSView/X11/GPUI handle 漏出なし） | `check-ui-driver-boundary`（`gui-boundary-guard --offline`、dep-graph + public signature 検査）in `ui-gpui-verify` | 3 OS | step success ×3 | **Pass** |
| Fallback（capability 非対応は明示 refusal であり fake success にしない） | NSC `unsupported_capabilities_are_explicit_refusals_not_fake_successes` + driver_contract `ui_dvr_06_native_slot_unsupported_stale_destroy` | 3 OS | 上記 test 集計に内包・`0 failed` | **Pass**（契約層の明示 refusal。実 fallback UI は範囲外） |
| Detach（lease release / teardown / re-dock commit の detach→再 attach） | NSC `release_lease_detaches_the_child_and_a_new_lease_is_distinct` / `teardown_detaches_child_and_emits_revoke_then_destroyed` / `dock_move_commit_reattaches_under_a_new_slot_and_revokes_the_old` | 3 OS | 上記 13 件に内包・`0 failed` | **Pass**（契約層。実 window 上の detach は範囲外） |
| 2-phase move rollback（NSC-12 abort / NSC-13 commit） | `test-ui-native-slot` 内 `dock_move_abort_rolls_back_with_the_live_lease_kept` / `dock_move_commit_reattaches_under_a_new_slot_and_revokes_the_old` | 3 OS | 上記 13 件に内包・`0 failed` | **Pass**（`DockWorkspace` begin/commit/abort + slot busy 結線の契約・モデル層） |
| Compile/Link: GPUI adapter `gpui` feature | `check-ui-gpui` / `lint-ui-gpui`（`cargo check`/`clippy -D warnings --features gpui`）in `ui-gpui-verify` | 3 OS | step success ×3 | **Pass** |
| Compile/Link + Headless: GPUI adapter `gpui-test uitest` feature（`windowed_test_app` AUT bin の Build/Link を含む） | `test-ui-gpui`（`cargo test -p ruviform-ui-gpui --features "gpui-test uitest"`）in `ui-gpui-verify` | 3 OS | ubuntu: lib 55 + `windowed_test_app` bin compile/link/run + consumer_observation 22 + lab_story 4 + scenario_contract 7 + windowed_uitest 2、全 `0 failed`。windows: lib 77 + 同 bin link + 同 test 群 `0 failed`。macos: lib 55 + 同 bin link + 同 test 群 `0 failed` | **Pass**（headless `gpui-test`。`windowed_uitest` は `RVF_WINDOWED_UITEST` 未設定で no-op 契約、実 window 操作は GUI-L2 で別途） |
| Headless render / scenario（GPUI 実 element path） | `test-ui-scenario-contract` gpui 側・`test-ui-gpui` lib tests（`gpui-test` TestAppContext） | 3 OS | 上記集計に内包 | **Pass** |
| Linux 実 window pixel（Xvfb+openbox+lavapipe、X11 GetImage readback） | `test-ui-offscreen-xvfb`（`real-scenario` feature、`tests/real_window.rs`）in `ui-gpui-offscreen-verify` | ubuntu (Xvfb) | `3 passed`（`lab_story_al_waveform_seek_real_window` / `lab_story_sv_fader_dock_real_window` / `real_os_input_scenario`、105s）。証拠 artifact `offscreen-evidence-attempt-1` upload | **Pass**（headless 環境下の実 X11 window pixel。物理 display・実利用 GUI ではない） |
| RT 非干渉（RT contract・alloc-free 経路） | `test-rt`（`cargo test --workspace rt_`）in `rt-verify` + `test-full` 内 `rt_*` in `verify` | ubuntu（rt-verify）/ 3 OS（verify） | rt-verify: strict `rt_` contract test 数 **166** を事前確認 step で計測後 `test-rt` success。verify windows: `test result: ok` 201 block・`FAILED` 0、`rt_*` 各 test `ok` | **Pass**（headless RT 契約。実 Plugin GUI hang 時の RT 影響は #389 共同検証・範囲外） |
| Workspace 全体の Compile/Lint/Test（default feature） | `lint-full` + `test-full` in `verify`（ubuntu/windows）・`verify-macos` | 3 OS | 全 step success。windows log で `test result: ok` 201 block・`FAILED` 0 | **Pass** |
| MSRV（rust@1.89.0）での契約再検証 | `test-ui-test-msrv`（`ruviform-ui-test` 全 target）in `msrv-verify` | ubuntu (rust@1.89.0) | `native_slot_contract` **13 passed** を含む全 suite `0 failed`。他 3 task（ofx/observability/io）も success | **Pass** |

### AC2: #262/#419 への引き渡し

| 項目 | 成果物 | verdict |
| --- | --- | --- |
| L1 完了可否・実 Native Dock 非保証・Floating/Generic/Mock ≠ Dock Pass の明記 | `evidence/rf388-os-l1-ci/handoff-262-419.md`（本 slice の成果物。Issue コメント投稿は Coordinator が実施） | **Done（文書化）** — L1 契約/CI 証拠は完了、実 Native Dock は未保証（DComp 画素問題 #532 未解決・実 Plugin Dock は Deferred） |

### AC3: macOS/Linux L1 のみ必須

| OS | 必須範囲 | lane / test | 観測結果 | verdict |
| --- | --- | --- | --- | --- |
| macOS | Native Build/Link・必要 Feature・Headless/RT | `ui-gpui-verify-macos`（boundary guard・driver contract・parity・native slot 13・component-view・golden・scenario・check/lint/test-ui-gpui・consumer observation の 11 step 全 success）、`verify-macos`（prek/fmt/lint-full/test-full）、`ui-verify-macos`、`l1-real-verify-macos`（fixture cdylib + `host` L1 E2E）、`msrv-verify`（ubuntu host・rust@1.89.0） | 全 job success、checkout SHA=`e93c438a…` | **Pass** |
| Linux | 同上 | `ui-gpui-verify (ubuntu-latest)`、`verify (ubuntu-latest)`、`ui-gpui-offscreen-verify`（Xvfb 実 window pixel 3 tests）、`rt-verify`（166 rt_ tests）、`l1-real-verify (ubuntu-latest)`、`msrv-verify`、io/icc/compression lane | 全 job success、checkout SHA=`e93c438a…` | **Pass** |
| macOS/Linux | 実 GUI・物理 Device・実 Plugin（商用 binary） | 既存 CI に該当 lane なし。`l1-real-verify-macos`/`(ubuntu)` は repo 内 fixture plugin の L1 host E2E であり実商用 Plugin ではない | — | **Deferred / NotRun**（利用要件発生まで。NotRun≠Pass） |

### AC4: 検証 Scope 明記・新規 Workflow 不作成

| 項目 | 判定 | 根拠 |
| --- | --- | --- |
| 採用機能・OS/Device/Format・Run URL/Oracle の明記 | **Pass（本 report「検証スコープと Oracle」節）** | 上表のとおり採用 feature・runner OS・fixture のみの Device/Format・単一 Run URL・Oracle を明記 |
| 新規 CI Workflow を作らない | **Pass** | 証拠は既存 `ci.yml` run 43428172820 のみ。`consumer-verify.yml` は workflow_dispatch 専用で本 SHA の run なし（最新 run 43347745016 は別 SHA `3692c6fa`）→ Consumer Oracle は **NotRun**（本 slice の必須範囲外・記録のみ） |

## 既知ギャップ（注記のみ・本 slice では修正しない）

- **`ruviform-poc-native-dock` の `gpui`/`vst3` feature は CI で compile されない。** 同 crate の `default = []` で、`gpui`/`vst3` は opt-in feature。`check-full`/`lint-full`/`test-full`/`build-full` は `--all-features` を付けないため default feature のみが compile され、`src/main.rs` は feature 無しでは stub（`exit(2)`）に退化し `src/poc/` 配下の実配線コード（`GpuiSurfaceAdapter` + `PluginEditorSession` + `InProcessHost`）は CI のどの lane でも型検査・link されない。`poc-native-dock`/`poc-vst3-editor`/`poc-native-dock-layout`/`poc-native-dock-windowing` の各 mise task は `cargo run --features gpui[,vst3]` の手動 lane で `ci.yml`/`daily-ci.yml` に未登録。現状の CI 証拠は「契約層（fake/headless/gpui-test）」までで、consumer 層 PoC の feature-enabled compile/link は手動 Windows 実施分（`evidence/rf388-native-surface-slot/` 等）のみが証拠。feature matrix への追加は別 Issue での判断事項。
- `test-ui-gpui`（`gpui-test uitest`）の compile で `surface.rs` の `clip_of` に `dead_code` warning が出る（ubuntu log 確認）。test 成功には影響しない既存の警告であり、本 slice では未対応。

## 実施していない検証（NotRun / Deferred — Pass に繰り上げない）

- Windows L2/L3: 実 Plugin の Native Dock 画素・Dock/Tab/Resize/Clip/Focus/ReDock・GUI/RT 非干渉の実機検証（#532 DComp 決裁待ち・SV #176/#389 Owner・本 slice 対象外）。
- macOS/Linux の実 GUI・物理 Audio/MIDI Device・実商用 Plugin binary（利用要件発生まで Deferred）。
- `consumer-verify.yml` の Consumer Oracle（workflow_dispatch 専用・本 SHA `e93c438a` の run なし）。
- 実 Native Dock Pass 全般: Floating Window 重ね・Generic Parameter Editor・Mock/Fake Surface は Dock Pass としない（Issue 本文の定義どおり）。

## 判定まとめ

- **OS-L1（本 slice の必須範囲）: Pass。** 対象 SHA `e93c438a` の既存 Full CI で、公開契約（NSC-01〜13・UI-DVR-*）・GPUI/OS private handle 境界・Fallback 明示 refusal・Detach/2-phase rollback・GPUI feature の Compile/Link/Headless・RT 非干渉が 3 OS で照合できた。macOS/Linux は L1 のみ必須であり両 OS の Native Build/Link・必要 Feature・Headless/RT を確認。実 GUI/物理 Device/実 Plugin は Deferred（NotRun）として分離記録。
- **Issue #388 は open 継続。** 残る阻害要因は Windows L2/L3 の実 Plugin Dock 画素（#532 DComp 決裁待ち）と実 Plugin 受入（SV #176/#389 Owner）であり、OS-L1 証拠の欠落ではない。
