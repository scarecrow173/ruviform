---
id: "DISC-rf630-ac-inventory-01"
type: "discovery"
status: "final"
title: "Issue #630 AC disposition census — VST3/CLAP Test Plugin Suite AC 行の証拠 inventory"
created: "2026-11-24"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-630-ac-inventory.md"
      - type: "doc"
        path: "docs/specs/issue-630-ac-inventory.md"
      - type: "doc"
        path: "docs/test-specs/issue-630-ac-inventory.md"
      - type: "doc"
        path: "evidence/rf630/ac-inventory.md"
    modified:
      - type: "file"
        path: "evidence/rf631/plugin-dock-scenario.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console-list.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console-list-include-real.jsonl"
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

## 証拠 inventory

### canonical ledger（read-only 正本 + 本 slice 再生成）

| ledger | sha / 条件 | case 内訳 |
| --- | --- | --- |
| `evidence/rf630/plugin-dock-real-clap.jsonl`（read-only 正本） | `a0e34564`（`RUVIFORM_PLUGIN_DOCK_TEST_CLAP`・`run_id=rf630-plugin-dock-real-clap`） | 14 case = **9 pass + 5 unsupported** — `real-clap-editor-attach`/`real-clap-dcomp-pixel`/`real-clap-editor-{resize,redock}`/`real-clap-editor-visibility`/`real-clap-session-{timeout,recovery,rt-*}` pass;`real-clap-editor-{input,focus,popup}`/`real-clap-session-{crash,hang}` unsupported |
| `evidence/rf631/plugin-dock-real-scenario.jsonl`（VST3 leg・read-only） | `a0e34564` | 32 case 全 pass — `test_editor` の実 child HWND・画素・input/focus/popup/resize/redock・DComp repro・session |
| `evidence/rf631/plugin-dock-scenario.jsonl`（headless・本 slice で HEAD 再生成） | `06a5a9e0` | 9 case = 7 pass + `editor-attach` unsupported + `dock-pixels` not_run |
| `evidence/rf630/plugin-dock-real-clap-headless.jsonl`（read-only） | `a0e34564`・`--blocked-probe` | 14 row 全 `not_run` + `environment_blocked` — fail-closed 実証 |
| `evidence/rf630/abi-validator/run.json` | schema `rf630-abi-validator-run/2`・実走 `1d18e297` | clap-validator 0.4.1 全 6 CLAP leg Passed + Steinberg VST3 SDK validator 3.8.1（235 passed/0 failed） |
| `evidence/rf630/consumer-oracle/manifest.json` | schema `rf630-fixture-contract/1`・re-pin `5ea61d83` | 全 7 fixture class pin + `consumerOracleMap` 5 行 + dep 非汚染 grep 実測 |

### slice 別検証記録

`evidence/rf630/` 配下の `*-verification.md` が `test_gain`/
`test_instrument`/`test_stateful`/`test_fault`/`test_editor`/`test_spawn_fail`・
preset surface・L1 runner・CLAP legs・ABI validator・consumer oracle・fault
modes・output drain の canonical=mise 実走を記録。CI 構造の根拠は
`.github/workflows/ci.yml` の `verify`/`verify-macos`/`l1-real-verify`/
`l1-real-verify-macos`（read-only 参照）。

## 対象 AC と証拠対応の論理

- **AC1（実ファイルを製品 Host 本番 Path で Discover/Instantiate/Process/
  Uninstantiate・Fake Plugin Port 非経由）**: VST3 は `Vst3Library`、CLAP は
  `ClapLibrary` の production bridge を経由する host_path tests・headless
  `host-lifecycle` case・L1 e2e・real dock leg が実 cdylib を駆動。Fake
  Plugin Port 経由の成功を Pass 根拠にしていない。
- **AC2（既知 PCM/Frame/Tempo/Loop/Automation/MIDI/offset/State/Preset/RT/
  Crash/Hang/Timeout/Isolation を独立 Oracle で検証）**: 4 fixture 群 +
  spawn/pre-Hello fault + L1 fan-out/isolation + real-session legs が対応。
  golden 値・host 観測・外部 validator が独立 oracle。残件は CLAP leg の
  crash/hang（implementable）と OS-API MIDI 経路（rf#762 gate）。
- **AC3（外部 Validator/Reference Host・自己検証バイアス排除・SDK/License
  記録）**: clap-validator 0.4.1 + VST3 SDK validator 3.8.1 の全 canonical leg
  Passed。fixture は `ruviform-plugin-vst3` internals 不使用の独立実装。
  宣言的除外（preset-discovery crawl/load・param-fuzz panic）は non-pass
  として記録し、upstream release 後の再評価は #867。
- **AC4（Native Editor 実 Window/child HWND/画素/Input/Focus/Resize/Popup/
  Close/再 Open・#388/#389/#532 DComp Fail 再現・Fake/Floating のみで
  Pass にしない）**: VST3 leg は `test_editor` 実 `WS_CHILD` `IPlugView` で
  全判定語彙 pass。CLAP leg は 9 pass + 5 unsupported（input/focus/popup の
  CLAP L0 ops surface 欠如 + crash/hang の fault arm 不在）= implementable。
- **AC5（version/digest/RT/OS/SHA で再現・安全な Timeout/kill）**: ledger の
  `repo.sha`/`plugin_artifact` sha256/`environment`/`case_seed` + watchdog
  kill + `SessionDeadlinePolicy` ZERO detach + `InstanceFailed` exactly-once。
- **AC6（3OS OS-L1 Build/Link/Headless/ABI を既存 CI に統合・Windows GUI-L2/
  EXT-L3 は opt-in lane 分離）**: `test-full` が fixture crate の
  build/link/headless を 3OS で、`test-l1-real-plugin` が L1 e2e を 3OS で
  実走。GUI-L2/EXT-L3 は manual/opt-in。merge 後 CI 実走証拠は Coordinator gate。
- **AC7（sv#101/#127/#128/#176・al#127 の独立 Consumer Oracle 照合・AudioLens
  新依存を強制しない）**: manifest `rf630-fixture-contract/1` が contract
  face pin + 5 行 map + dep 非汚染 grep を提供 — ruviform 側 done。
  consumer 実走 legs は external-owner。

## 制約・限界

- real-window ledger（VST3/CLAP 両 leg）は read-only 正本 — 本 slice では
  再生成しない（GUI 必須 lane は占有範囲外）。
- 宣言的除外・unsupported・not_run の行を Pass と記録しない。集計は
  `evidence/rf630/ac-inventory.md` の行別表が正本。
- Issue checkbox は全行 `[ ]` のまま — `[x]` 適用は Coordinator 判定。
- `test-os-endpoint-midi-e2e` は rf#629 scope で #630 の直接 AC 行ではないが、
  AC1/AC2 の完全経路に関わる残件として併記する（provisioned runner =
  rf#762 gate）。
