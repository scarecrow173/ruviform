---
id: "TSPEC-rf630-ac-inventory-01"
type: "test-spec"
status: "approved"
title: "Issue #630 AC disposition census — AC×証拠 突合 test spec（fail-closed・verdict 正直性を含む）"
created: "2026-11-24"
updated: "2026-11-26"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-ac-inventory.md"
  changes:
    modified:
      - type: "doc"
        path: "docs/test-specs/issue-630-ac-inventory.md"
  depends-on:
    - "docs/discovery/issue-630-ac-inventory.md"
    - "docs/test-specs/issue-630-consumer-oracle.md"
    - "docs/test-specs/issue-388-virtual-first-ac.md"
    - "POLICIES.md"
  verifies:
    - "docs/specs/issue-630-ac-inventory.md"
metadata:
  issue:
    number: 630
    url: "https://github.com/scarecrow173/ruviform/issues/630"
---

## 概要

Issue #630 の AC 7 行と canonical 証拠を 1 行ずつ突合し、verdict を固定する
test spec である。ledger 行の実測 verdict が正本であり、本 spec の表はそれを
誇張しない。fail-closed・verdict 正直性の確認手順を併記する。

本版は closeout docs-only slice（2026-11-26・base SHA `ab6dc99f`）での再
census に基づく改訂である。census は ledger/証拠ファイルの read-only parse
（Python による JSONL/JSON 集計 — census 手法であって canonical 検証ではない）
で行い、実行した canonical 検証コマンドと exit code は
`evidence/rf630/closeout-verification.md` に記録する。

## AC × 証拠 突合表

| # | AC 行（要約） | AC 種別 | 実測 verdict | 根拠証拠 |
| --- | --- | --- | --- | --- |
| 1 | VST3/CLAP 実ファイルを製品 Host 本番 Path で Discover/Instantiate/Process/Uninstantiate・Fake Plugin Port 非経由 | Required | **Pass** | VST3: `plugin-dock-scenario.jsonl` @`8a000038` `host-lifecycle` pass + host_path tests + L1 e2e。CLAP: host_path tests + L1 e2e + `real-clap-editor-attach` pass（`plugin-dock-real-clap.jsonl` @`2dfd2e0f`）。`evidence/rf630/verification.md`・`clap-test-gain-verification.md` |
| 2 | 既知 PCM/Frame/Tempo/Loop/Automation/MIDI/offset/State/Preset/RT/Crash/Hang/Timeout/Isolation を独立 Oracle で検証 | Required | **Pass**（残件: OS-API MIDI のみ） | `evidence/rf630/test-{instrument,stateful,fault,preset-spawn-fault}-verification.md`・`single-component-verification.md`・`l1-runner-verification.md`・`clap-fault-carrier-verification.md`。`plugin-dock-real-scenario.jsonl` / `plugin-dock-real-clap.jsonl` @`2dfd2e0f` の `real-(clap-)session-{crash,hang,timeout,rt-*}` 全 pass — CLAP crash/hang は `test_editor` fault-carrier の in-band arm（`fault_armed` step pass 実測）。残件: `test-os-endpoint-midi-e2e` NotRun（rf#762 gate） |
| 3 | Host 独立 Validator/Reference Host で実 ABI 検証・自己検証バイアス排除・SDK/License 記録 | Required | **Pass**（宣言的除外は non-pass 記録） | `abi-validator/run.json` schema /2（実走 HEAD `5099cbe5`）: clap-validator **pin-47e2424**（`0.4.1-5-g47e2424`・license MIT・sha256 `b4bd6458…`）全 6 CLAP leg Passed + Steinberg VST3 SDK validator **3.8.1**（`v3.8.1_build_84`・MIT License per `vst3sdk/LICENSE.txt`・sha256 `5889a896…`）**282 passed/0 failed**。`test_fault` の `param-(set\|fuzz)` 宣言的除外は設計由来で継続・Pass ではない。preset-discovery crawl/load は除外解消済み（success 実測）。param-fuzz panic の informational leg は Unsupported（re-eval = #867） |
| 4 | Native Editor 実 Window/child HWND/画素/Input/Focus/Resize/Popup/Close/再 Open・DComp Fail 再現・Fake/Floating のみで Pass にしない | Required | **Pass**（`HostWindow` focus leg は qualified pass） | `plugin-dock-real-scenario.jsonl` @`2dfd2e0f` 32 pass（attach/画素/input/focus/popup/resize/redock/close・`real-dcomp-*` 再現）。`plugin-dock-real-clap.jsonl` @`2dfd2e0f` 14 pass — `real-clap-editor-{input,focus,popup}` は `ruviform.editor-ops` seam 経由 effect-verified、`real-clap-session-{crash,hang}` は fault-carrier。**qualified pass**: 両 ABI の `set_focus_target(HostWindow)` step は `landing:"accepted-declared"`・`effect_verified:false`（`f:0` token delivery は検証済み・container への OS focus move は lane-topology 依存で未検証 — 同-queue `host_path` lane が真の oracle） |
| 5 | version/digest/RT/OS/SHA 再現スイート・故障注入の安全な Timeout/kill | Required | **Pass** | ledger 全行の `repo.sha`/`plugin-artifact` sha256/`case_seed`/`environment`。`l1:watchdog-kill`（exit 1・cause Timeout）+ `SessionDeadlinePolicy` ZERO detach expiry + `InstanceFailed` exactly-once・`host_survives`/`lane_settles`（`real-(clap-)session-{crash,hang,timeout}` pass） |
| 6 | 3OS OS-L1 Build/Link/Headless/ABI を既存 CI に統合・Windows GUI-L2/EXT-L3 を opt-in lane 分離 | Required | **Pass**（CI 構造） | ci.yml `verify`/`verify-macos` の `test-full`（fixture crate の build/link/headless を 3OS）+ `l1-real-verify`/`l1-real-verify-macos` の `test-l1-real-plugin`（L1 e2e 3OS）。GUI-L2/EXT-L3 lane は manual/opt-in。`ci.yml` は `workflow_dispatch` のみ・`daily-ci.yml` が main 検証を集約 dispatch |
| 7 | sv#101/#127/#128/#176・al#127 の独立 Consumer Oracle 照合・AudioLens 新依存を強制しない | Required | **Pass**（consumer 実走は external-owner follow-up — 未実走を Pass と偽装しない） | `consumer-oracle/manifest.json` `rf630-fixture-contract/1`（7 fixture class pin + `consumerOracleMap` 5 行: sv#101/#127/#128/#176 = `headless-contract-match`・al#127 = `NotApplicable`）+ dep 非汚染（2026-11-25 再測: audio-lens `Cargo.lock` 非含有・exit 1 = 期待値。synvelis `Cargo.lock` は含有するが consumer 自主 dev-dep・test-only で production DAG 非含有 — 強制ではない）。consumer 実走 legs = external-owner |

## verdict 正直性チェック

- Pass 判定は全て実 ABI cdylib（`ruviform_plugin_test_vst3.dll` /
  `ruviform_plugin_test_clap.dll`）を production Host path で駆動した
  実走記録が根拠であり、Mock/Fake Plugin Port の成功を含まない。
- CLAP leg の editor input/focus/popup は `ruviform.editor-ops` seam 経由の
  synthetic verb delivery であり OS input queue/focus routing そのものではない
  — ただし fixture の event log token・caller-queue `GetFocus`・popup HWND の
  実観測で effect を検証する effect-verified 判定であり、verdict を偽っていない。
  crash/hang は `test_editor` fault-carrier の in-band arm で実プロセス障害を
  観測する（`exit_code`/`exit_signal`・watchdog kill 実測）。
- `HostWindow` focus leg は `accepted-declared`・`effect_verified:false` を
  ledger step に明記し、effect-verified Pass と混同しない。かといって宣言
  landing が仕様どおり観測されている行を fail に倒れ込ませもしない。
- 宣言的除外（`test_fault` の `param-(set|fuzz)`）は `run.json` schema /2 で
  Passed 行列の集計に含めず non-pass として記録 — 除外を黙って Pass に
  読み替えない。informational unfiltered leg は evaluation 集計（7 leg）に
  含まれない Unsupported 記録である。
- blocked-probe ledger（`plugin-dock-real-clap-headless.jsonl` 14 row +
  `plugin-dock-real-headless.jsonl` 32 row・いずれも `8a000038`）は全
  `not_run`+`environment_blocked` を bin 自己検査で強制 — headless で real
  verdict を偽造しない。
- headless `dock-pixels`/`editor-attach` は fixture 法則の構造的 verdict であり、
  real lane の `test_editor` 実 editor 経路が dock/画素判定を担う。

## scope

- 本 spec は verdict の突合と正直性確認のみ。fixture・Host・route・catalog の
  意味変更、新規 slice、Issue 本文更新は含まない。
- real-window ledger 両 leg は read-only 正本 — 再生成しない。closeout では
  `2dfd2e0f` の既存 ledger を証拠採用し、real lane 再実行と computer-use
  実 GUI 受入は NotRun（2026-11-26 ユーザー決定）。

## lane record

| 確認 | 方法 | 結果 |
| --- | --- | --- |
| canonical ledger 実測確認（census） | python3 で JSONL parse（read-only・census 手法であり canonical 検証ではない） | `plugin-dock-real-scenario.jsonl` 32/32 pass @`2dfd2e0f`・`plugin-dock-real-clap.jsonl` 14/14 pass @`2dfd2e0f`・headless `plugin-dock-scenario.jsonl` 7 pass+1 unsupported+1 not_run @`8a000038`・blocked-probe 14+32 全 `not_run` @`8a000038` |
| ABI validator run 集計確認（census） | python3 で JSON parse（read-only） | schema /2・evaluation Passed 7/7・VST3 282 passed/0 failed・informational leg Unsupported exit 101 |
| consumer oracle manifest 確認（census） | python3 で JSON parse（read-only） | schema `rf630-fixture-contract/1`・7 fixture class・`consumerOracleMap` 5 行・dep 非汚染 2026-11-25 再測記録 |
| Issue checkbox snapshot（census） | `gh issue view 630`（read-only） | AC1〜AC6 `[x]`・AC7 `[ ]`・state OPEN（2026-11-26） |
| 文書 lint（canonical） | `mise exec -- prek run --all-files` | 結果は `evidence/rf630/closeout-verification.md` と PR 本文の Verification 節に記録 |
