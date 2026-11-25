---
id: "DISC-rf630-ac-inventory-01"
type: "discovery"
status: "final"
title: "Issue #630 AC disposition census — VST3/CLAP Test Plugin Suite AC 行の証拠 inventory"
created: "2026-11-24"
updated: "2026-11-26"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
  changes:
    added:
      - type: "doc"
        path: "evidence/rf630/closeout-verification.md"
    modified:
      - type: "doc"
        path: "docs/discovery/issue-630-ac-inventory.md"
      - type: "doc"
        path: "docs/specs/issue-630-ac-inventory.md"
      - type: "doc"
        path: "docs/test-specs/issue-630-ac-inventory.md"
      - type: "doc"
        path: "evidence/rf630/ac-inventory.md"
      - type: "doc"
        path: "docs/evidence/plugin-format-evidence.md"
      - type: "doc"
        path: "docs/knowledge/plugin-format-adapters.md"
  depends-on:
    - "POLICIES.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-fault-modes.md"
    - "docs/specs/issue-630-consumer-oracle.md"
    - "docs/specs/issue-630-abi-validator.md"
    - "docs/specs/issue-388-virtual-first-ac.md"
  related:
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "docs/specs/issue-631-ac-inventory.md"
    - "https://github.com/scarecrow173/ruviform/issues/867"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/920"
  verified-by:
    - "docs/test-specs/issue-630-ac-inventory.md"
metadata:
  issue:
    number: 630
    url: "https://github.com/scarecrow173/ruviform/issues/630"
---

## 概要

Issue #630（VST3/CLAP Test Plugin Suite — 実 ABI の fixture 群と独立 oracle
検証）の受入条件 7 行を、canonical ledger と既存証拠に突合し、行ごとの現行
verdict を棚卸しした close-out 判定材料である。本 slice は証拠の突合と分類
のみを行い、実装・verdict 意味の変更・Issue 本文の更新は行わない。

本版は closeout docs-only slice（2026-11-26・base SHA
`ab6dc99fe8935ac09082d05e2176b10e729ba263`）での再 census に基づく改訂で
ある。census 手法・参照した全証拠の sha/count・実行した検証コマンドは
`evidence/rf630/closeout-verification.md` が正本として記録する。

## 証拠 inventory

### canonical ledger（read-only 正本）

| ledger | sha / 条件 | case 内訳 |
| --- | --- | --- |
| `evidence/rf630/plugin-dock-real-clap.jsonl`（read-only 正本） | `2dfd2e0fc0e3928669ccd51dad330b0b2795903b`（`run_id=rf630-plugin-dock-real-clap`・lane `test-ui-plugin-dock-real`・`RUVIFORM_PLUGIN_DOCK_TEST_CLAP`・windows/x86_64） | 14 case = **14 pass** — `real-clap-editor-{input,focus,popup}` は `ruviform.editor-ops` seam 経由の effect-verified、`real-clap-session-{crash,hang}` は fault-carrier で pass。旧 `a0e34564` ledger の 9 pass + 5 unsupported は editor verbs/fault-carrier 実装により解消 |
| `evidence/rf631/plugin-dock-real-scenario.jsonl`（VST3 leg・read-only） | `2dfd2e0fc0e3928669ccd51dad330b0b2795903b` | 32 case 全 pass — `test_editor` の実 child HWND・画素・input/focus/popup/resize/redock・DComp repro・session。focus は `set_focus_target` 3 target 6-leg・popup は `set_popup` 2 kind 4-leg の verb-driven oracle |
| `evidence/rf631/plugin-dock-scenario.jsonl`（headless） | `8a000038ad687117d4f3d428d086459f47092001` | 9 case = 7 pass + `editor-attach` unsupported + `dock-pixels` not_run |
| `evidence/rf630/plugin-dock-real-clap-headless.jsonl` / `evidence/rf631/plugin-dock-real-headless.jsonl`（`--blocked-probe`） | `8a000038ad687117d4f3d428d086459f47092001` | CLAP 14 row・VST3 32 row 全 `not_run` + `environment_blocked` — fail-closed 実証 |
| `evidence/rf630/abi-validator/run.json` | schema `rf630-abi-validator-run/2`・実走 HEAD `5099cbe5836b5d53735951463e9244e7a840991c`（2026-11-23） | evaluation verdict Passed・canonical 7/7 leg。clap-validator pin-47e2424（`0.4.1-5-g47e2424`・MIT）全 6 CLAP leg Passed + Steinberg VST3 SDK validator 3.8.1（`v3.8.1_build_84`・MIT License）282 passed/0 failed。`test_fault` の `param-(set\|fuzz)` 宣言的除外は設計由来で継続、preset-discovery 除外は解消済み、unfiltered fault leg は informational Unsupported（exit 101・tool panic） |
| `evidence/rf630/consumer-oracle/manifest.json` | schema `rf630-fixture-contract/1`・created 2026-11-21・dep 非汚染 2026-11-25 再測 | 全 7 fixture class pin + `consumerOracleMap` 5 行 + dep 非汚染（audio-lens 非含有・synvelis は自主 dev-dep） |

### slice 別検証記録

`evidence/rf630/` 配下の `*-verification.md` が `test_gain`/
`test_instrument`/`test_stateful`/`test_fault`/`test_editor`/`test_spawn_fail`・
preset surface・L1 runner・CLAP legs・ABI validator（rerun + preset-discovery
reeval）・consumer oracle・fault modes・output drain・editor verbs
（`editor-verbs-impl/`）・fault-carrier の canonical=mise 実走を記録。CI 構造の
根拠は `.github/workflows/ci.yml`（`workflow_dispatch` のみ）と
`.github/workflows/daily-ci.yml`（main 検証の集約 dispatch）の
`verify`/`verify-macos`/`l1-real-verify`/`l1-real-verify-macos`（read-only 参照）。

## 対象 AC と証拠対応の論理

- **AC1（実ファイルを製品 Host 本番 Path で Discover/Instantiate/Process/
  Uninstantiate・Fake Plugin Port 非経由）**: VST3 は `Vst3Library`、CLAP は
  `ClapLibrary` の production bridge を経由する host_path tests・headless
  `host-lifecycle` case・L1 e2e・real dock leg が実 cdylib を駆動。Fake
  Plugin Port 経由の成功を Pass 根拠にしていない。
- **AC2（既知 PCM/Frame/Tempo/Loop/Automation/MIDI/offset/State/Preset/RT/
  Crash/Hang/Timeout/Isolation を独立 Oracle で検証）**: 4 fixture 群 +
  spawn/pre-Hello fault + L1 fan-out/isolation + real-session legs が対応。
  golden 値・host 観測・外部 validator が独立 oracle。CLAP leg の crash/hang は
  `test_editor` fault-carrier（共有 `Fault` param・VST3 #885 の mirror）で
  pass 化済み。残件は OS-API MIDI 経路（rf#762 gate）のみ。
- **AC3（外部 Validator/Reference Host・自己検証バイアス排除・SDK/License
  記録）**: clap-validator pin-47e2424 + VST3 SDK validator 3.8.1 の canonical
  7/7 leg Passed。fixture は `ruviform-plugin-vst3` internals 不使用の独立実装。
  `test_fault` の `param-(set|fuzz)` 宣言的除外（fixture が意図的 crash を arm
  する設計由来）と informational unfiltered leg の Unsupported（param-fuzz
  tool panic・exit 101）は non-pass として記録し、upstream release 後の再評価は
  #867。preset-discovery crawl/load は pin-47e2424 で success 実測・除外解消済み。
- **AC4（Native Editor 実 Window/child HWND/画素/Input/Focus/Resize/Popup/
  Close/再 Open・#388/#389/#532 DComp Fail 再現・Fake/Floating のみで
  Pass にしない）**: VST3 leg 32 case・CLAP leg 14 case ともに全 pass。
  CLAP の input/focus/popup は `ruviform.editor-ops` seam 経由の
  effect-verified（title token・caller-queue `GetFocus`・popup HWND oracle）。
  qualified pass 注記: `set_focus_target(HostWindow)` leg は両 ABI で
  `accepted-declared`・`effect_verified:false`（container への OS focus move
  は lane-topology 依存・同-queue `host_path` lane が真の oracle）。
- **AC5（version/digest/RT/OS/SHA で再現・安全な Timeout/kill）**: ledger の
  `repo.sha`/`plugin-artifact` sha256/`environment`/`case_seed` + watchdog
  kill + `SessionDeadlinePolicy` ZERO detach + `InstanceFailed` exactly-once。
- **AC6（3OS OS-L1 Build/Link/Headless/ABI を既存 CI に統合・Windows GUI-L2/
  EXT-L3 は opt-in lane 分離）**: `test-full` が fixture crate の
  build/link/headless を 3OS で、`test-l1-real-plugin` が L1 e2e を 3OS で
  実走。GUI-L2/EXT-L3 は manual/opt-in。`ci.yml` は workflow_dispatch のみで
  `daily-ci.yml` が main 検証を担う（2026-11-26 ユーザー指定どおり手動
  dispatch はゲートにしない）。merge 後 CI 実走証拠は Coordinator gate。
- **AC7（sv#101/#127/#128/#176・al#127 の独立 Consumer Oracle 照合・AudioLens
  新依存を強制しない）**: manifest `rf630-fixture-contract/1` が contract
  face pin + 5 行 map + dep 非汚染 grep を提供 — ruviform 側 done。
  consumer 実走 legs は external-owner。sv#176 S1 の headless legs は
  synvelis PR #640 で実走済みだが Native Dock/PD 受入は未実走であり、
  false Pass にしない。

## 制約・限界

- real-window ledger（VST3/CLAP 両 leg）は read-only 正本 — 本 slice では
  再生成しない（GUI 必須 lane は占有範囲外かつ computer-use/実 GUI 受入は
  2026-11-26 ユーザー決定により NotRun）。
- 宣言的除外・unsupported・not_run の行を Pass と記録しない。集計は
  `evidence/rf630/ac-inventory.md` の行別表が正本。
- Issue checkbox は AC1〜AC6 が `[x]`・AC7 が `[ ]`（2026-11-26 確認）。
  AC7 の checkbox 適用と Issue close は Coordinator 判定。
- `test-os-endpoint-midi-e2e` は rf#629 scope で #630 の直接 AC 行ではないが、
  AC1/AC2 の完全経路に関わる残件として併記する（provisioned runner =
  rf#762 gate）。
