---
id: "DESIGN-630abiValidator03"
type: "design"
status: "approved"
title: "Issue #630 外部 ABI validator slice — 設計"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-abi-validator.md"
    - "docs/discovery/issue-630-abi-validator.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-abi-validator.md"
  implements:
    - "docs/specs/issue-630-abi-validator.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-abi-validator.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
  blocks:
    - "docs/plans/issue-630-abi-validator.md"
  supersedes: []
  superseded-by: []
---

# Issue #630 外部 ABI validator slice — 設計

## 構成

```
ci/plugin-abi-validator/
  run.mjs                  # runner（Node ESM・依存なし）
  tests/run.test.mjs       # node:test contract test
evidence/rf630/abi-validator/
  run.json                 # 構造化 run report（schema rf630-abi-validator-run/1）
  clap-<plugin>.txt        # 各 CLAP leg の生出力
  clap-test_fault-unfiltered.txt
  vst3-validator.txt
```

runner は「tool 解決 → fixture build → leg 実行 → verdict 集約 →
evidence 書き出し」の直列パイプライン。validator は subprocess
（`spawnSync`）で実行し、stdout/stderr/exit code/signal を記録する。

## tool 解決（fail-closed）

優先順位: 明示 flag（`--clap-validator`/`--vst3-validator`）→
環境変数（`CLAP_VALIDATOR`/`VST3_VALIDATOR`）→ `--validator-dir`
規約 layout（`<dir>/clap-validator[.exe]`・`<dir>/vst3-validator[.exe]`）。
見つからなければ `null` — 推測で PATH を探さない。

provisioning（本 slice で確定・runner 外で人間/CI が用意）:

- clap-validator `0.4.1` pinned release binary（sha256 は release page
  記載値と照合済み）。配置例: `%LOCALAPPDATA%/ruviform/plugin-abi-validator/`。
- VST3 SDK validator は SDK source（commit `3cdf9ca`）を CMake+MSVC で
  build した `validator.exe`。配置例: 同 dir の `vst3sdk-build/bin/Release/`。
  SDK license（`vst3sdk/LICENSE.txt`）はソース取得時に確認済み。

## leg モデル

各 leg は `{kind, plugin, pluginId, command, exitCode, counts,
failedCases, warnings, status, details}`。status 語彙は
Passed/Failed/NotRun/Unsupported。

- **clap-validator legs**: 5 plugin id それぞれに
  `validate --json --plugin-id <id> <dll>` を実行。`test_fault` のみ
  `--exclude "param-(set|fuzz)"` を付与（fatal fault mode が param
  変更で武装される設計上、param-mutating case は意図的 crash を誘発
  する — 除外理由を `leg.exclusion` として記録）。加えて fault の
  unfiltered run を `informational` leg として実行し、tool 側異常終了を
  証拠として残すが verdict には含めない。
- **vst3-sdk-validator leg**: `ruviform_plugin_test_vst3.dll` を
  `<tmpdir>/ruviform-plugin-test-vst3.vst3/Contents/x86_64-win/
  ruviform-plugin-test-vst3.vst3` に copy（bundle 名 = inner 名の spec
  layout）し、`validator.exe <bundleDir>` を実行。終了後 tmpdir を削除。

## verdict 集約（fail-closed）

`evaluate(legs)`: canonical leg（`informational` を除く）のうち
Failed があれば Failed、次に NotRun、次に Unsupported、全て Passed
なら Passed。exit code は Passed のみ 0。tool/artifact 不在・build
失敗は全 leg を NotRun に倒す。

## fixture 修正（validator 指摘への対応）

全て `crates/ruviform-plugin-test-{clap,vst3}` 内。production
adapter は zero-diff。

- CLAP: param module `b"/"`→`b""`、fractional sample rate 受理、
  `MAX_EVENTS` 1024、instrument note 処理、stateful load 時
  `clap_host_params.rescan(CLAP_PARAM_RESCAN_VALUES)` 要求
  （`PluginStateImpl` が host handle を保持）。
- VST3: `terminate` 非終端化・`setupProcessing` 受理 state 拡大・
  bus info 修正（component.rs）。

## 意図する mise task 定義（Coordinator が統合時に登録）

```toml
[tasks.test-plugin-abi-validator]
description = "Run external ABI validators (clap-validator + VST3 SDK validator) against the test-plugin suite — opt-in, NOT a CI-required gate"
run = "node ci/plugin-abi-validator/run.mjs"
```

opt-in lane 記法の先例は `mise.toml` の `test-ui-plugin-dock-real`
系。validator binary の配置は `%LOCALAPPDATA%/ruviform/
plugin-abi-validator/`（`--validator-dir` 既定候補として
`RUVIFORM_ABI_VALIDATOR_DIR` env で供給可能）。

## evidence schema（run.json）

`schema: "rf630-abi-validator-run/1"`。repo.head、platform、node
version、tools.{clapValidator,vst3Validator}.{path,sha256}、
artifacts.{clapDll,vst3Dll}.{path,sha256|missing}、build 結果、
legs[]、evaluation{verdict,exitCode,summary}。生出力は
`clap-*.txt`/`vst3-validator.txt` に分離し run.json は参照のみ
保持（rawOutput は leg から除去して書き出し）。

## リスクと対応

- clap-validator の tool 側 panic（`param-fuzz-sample-accurate`）
  → 非 0 exit + JSON 無しを Unsupported として記録（plugin defect
  と混同しない）。canonical run では fault の param case を宣言的に
  除外。
- validator の出力 format 変更 → summary 正規表現/JSON shape が
  合わなければ Failed（fail-closed）。
- SDK license の人間判断 → 本 slice では source build 経路で確定済み。
  license 本文は `vst3sdk/LICENSE.txt`、clap-validator は MIT/Apache-2.0。
