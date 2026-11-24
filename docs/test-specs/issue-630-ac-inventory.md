---
id: "TSPEC-rf630-ac-inventory-01"
type: "test-spec"
status: "approved"
title: "Issue #630 AC disposition census — AC×証拠 突合 test spec（fail-closed・verdict 正直性を含む）"
created: "2026-11-24"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-ac-inventory.md"
  changes:
    added:
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

## AC × 証拠 突合表

| # | AC 行（要約） | AC 種別 | 実測 verdict | 根拠証拠 |
| --- | --- | --- | --- | --- |
| 1 | VST3/CLAP 実ファイルを製品 Host 本番 Path で Discover/Instantiate/Process/Uninstantiate・Fake Plugin Port 非経由 | Required | **Pass** | VST3: `plugin-dock-scenario.jsonl` @`06a5a9e0` `host-lifecycle` pass + host_path tests + L1 e2e。CLAP: host_path tests + L1 e2e + `real-clap-editor-attach` pass（`plugin-dock-real-clap.jsonl` @`a0e34564`）。`evidence/rf630/verification.md`・`clap-test-gain-verification.md` |
| 2 | 既知 PCM/Frame/Tempo/Loop/Automation/MIDI/offset/State/Preset/RT/Crash/Hang/Timeout/Isolation を独立 Oracle で検証 | Required | **Pass**（残件: CLAP crash/hang・OS-API MIDI） | `evidence/rf630/test-{instrument,stateful,fault,preset-spawn-fault}-verification.md`・`single-component-verification.md`・`l1-runner-verification.md`。`plugin-dock-real-scenario.jsonl` `real-session-{crash,hang,timeout,rt-*}` pass。残件: `plugin-dock-real-clap.jsonl` `real-clap-session-{crash,hang}` unsupported・`test-os-endpoint-midi-e2e` NotRun（rf#762 gate） |
| 3 | Host 独立 Validator/Reference Host で実 ABI 検証・自己検証バイアス排除・SDK/License 記録 | Required | **Pass**（宣言的除外は non-pass 記録） | `abi-validator/run.json` schema /2: clap-validator **0.4.1**（MIT OR Apache-2.0）全 6 CLAP leg Passed + Steinberg VST3 SDK validator **3.8.1**（235 passed/0 failed）。宣言的除外 `preset-discovery-(crawl\|load)` + `param-fuzz` panic は未実施・Pass ではない（re-eval = #867） |
| 4 | Native Editor 実 Window/child HWND/画素/Input/Focus/Resize/Popup/Close/再 Open・DComp Fail 再現・Fake/Floating のみで Pass にしない | Required | **部分 Pass**（VST3 leg done・CLAP leg 5 unsupported） | `plugin-dock-real-scenario.jsonl` 32 pass（attach/画素/input/focus/popup/resize/redock/close・`real-dcomp-*` 再現）。`plugin-dock-real-clap.jsonl` 9 pass + `real-clap-editor-{input,focus,popup}`・`real-clap-session-{crash,hang}` unsupported |
| 5 | version/digest/RT/OS/SHA 再現スイート・故障注入の安全な Timeout/kill | Required | **Pass** | ledger 全行の `repo.sha`/`plugin_artifact` sha256/`case_seed`/`environment`。`l1:watchdog-kill`（exit 1・cause Timeout）+ `SessionDeadlinePolicy` ZERO detach expiry + `InstanceFailed` exactly-once・`host_survives`/`lane_settles`（`real-session-{crash,hang,timeout}` pass） |
| 6 | 3OS OS-L1 Build/Link/Headless/ABI を既存 CI に統合・Windows GUI-L2/EXT-L3 を opt-in lane 分離 | Required | **Pass**（CI 構造） | ci.yml `verify`/`verify-macos` の `test-full`（fixture crate の build/link/headless を 3OS）+ `l1-real-verify`/`l1-real-verify-macos` の `test-l1-real-plugin`（L1 e2e 3OS）。GUI-L2/EXT-L3 lane は manual/opt-in |
| 7 | sv#101/#127/#128/#176・al#127 の独立 Consumer Oracle 照合・AudioLens 新依存を強制しない | Required | **部分 Pass**（ruviform 側 contract match done） | `consumer-oracle/manifest.json` `rf630-fixture-contract/1`（7 fixture class pin + `consumerOracleMap` 5 行: sv#101/#127/#128/#176 = `headless-contract-match`・al#127 = `NotApplicable`）+ dep 非汚染 grep（両 consumer Cargo.lock 非含有・exit 1 = 期待値）。consumer 実走 legs = external-owner |

## verdict 正直性チェック

- Pass 判定は全て実 ABI cdylib（`ruviform_plugin_test_vst3.dll` /
  `ruviform_plugin_test_clap.dll`）を production Host path で駆動した
  実走記録が根拠であり、Mock/Fake Plugin Port の成功を含まない。
- CLAP leg の 5 unsupported は原因軸を分離して記録される: input/focus/popup
  は CLAP L0 ops surface の trait-default `Unsupported`、crash/hang は
  parameterless fixture + `plugin_id` keyed route で co-resident fault arm
  不可 — いずれも verdict を偽っていない。
- 宣言的除外（`preset-discovery-crawl`/`-load`・`param-fuzz`）は
  `run.json` schema /2 で Passed 行列に含めず non-pass として記録 — 除外を
  黙って Pass に読み替えない。
- blocked-probe ledger（`plugin-dock-real-clap-headless.jsonl` 14 row +
  `plugin-dock-real-headless.jsonl` 32 row）は全 `not_run`+
  `environment_blocked` を bin 自己検査で強制 — headless で real verdict を
  偽造しない。
- headless `dock-pixels`/`editor-attach` は fixture 法則の構造的 verdict であり、
  real lane の `test_editor` 実 editor 経路が dock/画素判定を担う。

## scope

- 本 spec は verdict の突合と正直性確認のみ。fixture・Host・route・catalog の
  意味変更、新規 slice、Issue 本文更新は含まない。
- real-window ledger 両 leg は read-only 正本 — 再生成しない。stale な
  headless/catalog ledger 4 本のみ `06a5a9e0` に再生成済み。
- CLAP leg 残件の実装化（implementable 分類）は本 slice 範囲外 —
  `docs/specs/issue-630-ac-inventory.md` の分類表で後続作業を指示する。

## lane record

| 確認 | 方法 | 結果 |
| --- | --- | --- |
| headless ledger 再生成（canonical） | `mise run test-ui-plugin-dock` | `plugin-dock-scenario.jsonl` → `06a5a9e0`・9 case = 7 pass + 1 unsupported + 1 not_run |
| console ledger 再生成（canonical） | `mise run test-ui-plugin-dock-console` | `plugin-dock-console.jsonl` → `06a5a9e0`・exit 0 |
| catalog list 再生成（補助証拠） | `mise exec -- cargo run --locked -p ruviform-plugin-dock-test --bin plugin-dock-console -- --list --out …` | `plugin-dock-console-list.jsonl`（9 row） |
| catalog include-real 再生成（補助証拠） | 同上 `--features real-dock -- --list --include-real --out …` | `plugin-dock-console-list-include-real.jsonl`（41 row・crash/hang/rt-boundary に `out-of-process-plugin-host` required を現行化） |
| canonical ledger 実測確認 | python3 で JSONL parse（read-only） | `plugin-dock-real-scenario.jsonl` 32 pass・`plugin-dock-real-clap.jsonl` 9 pass + 5 unsupported・blocked-probe 14+32 全 `not_run` |
| 文書 lint | `mise run fmt-check` + `mise exec -- prek run --all-files` | （PR 本文の Verification 節に結果を記録） |
