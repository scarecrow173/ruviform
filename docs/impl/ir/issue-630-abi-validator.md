---
id: "IMPL-630abiValidator07"
type: "impl"
status: "completed"
title: "Issue #630 外部 ABI validator slice — 実装記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-630-abi-validator.md"
    - "docs/plans/issue-630-abi-validator.md"
    - "docs/specs/issue-630-abi-validator.md"
    - "docs/designs/issue-630-abi-validator.md"
  changes:
    added:
      - type: "file"
        path: "ci/plugin-abi-validator/run.mjs"
      - type: "file"
        path: "ci/plugin-abi-validator/tests/run.test.mjs"
      - type: "file"
        path: "evidence/rf630/abi-validator-verification.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-abi-validator.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/fault.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/stateful.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/instrument.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/block.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/component.rs"
  implements:
    - "docs/tasks/issue-630-abi-validator.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-630-abi-validator.md"
  blocks: []
  supersedes: []
  superseded-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-630-abi-validator-exp.jsonl"
    rejected: []
---

# Issue #630 外部 ABI validator slice — 実装記録

## 実装したもの

### runner（`ci/plugin-abi-validator/run.mjs`）

- Node ESM・依存なし。`parseArgs`/`resolveValidators`/`runClapLeg`/
  `normalizeClapLeg`/`runVst3Leg`/`normalizeVst3Leg`/`stageVst3Bundle`/
  `evaluate`/`run` を export。
- tool 解決は flag > env > `--validator-dir` 規約 layout。推測しない。
- fixture build は `cargo build -p ruviform-plugin-test-clap
  -p ruviform-plugin-test-vst3`（`--no-build` で skip 可）。
- CLAP は 5 plugin id を個別 leg で実行。`test_fault` のみ
  `--exclude "param-(set|fuzz)"` + informational unfiltered leg。
- VST3 は DLL を spec layout の bundle に staging して validator 実行。
- verdict は canonical leg のみで集約。Passed のみ exit 0。
- evidence: `run.json`（schema `rf630-abi-validator-run/1`）+
  `clap-*.txt`/`vst3-validator.txt`。

### contract test（`ci/plugin-abi-validator/tests/run.test.mjs`）

18 test。verdict 語彙・tool/artifact 解決・JSON/summary 正規化・
fail-closed の `run()` 統合（missing tool/artifact → NotRun、
spawn 失敗 → Failed）を固定。

### fixture 修正（validator 指摘対応・全て test crate 内）

- CLAP `plugin.rs`/`fault.rs`: param module path `b"/"`→`b""`。
- CLAP `block.rs`/`plugin.rs`: fractional sample rate 受理、
  `MAX_EVENTS` 1024。
- CLAP `instrument.rs`: note-on/off event 処理。
- CLAP `stateful.rs`: `PluginStateImpl` が host handle を保持し、
  `load` 成功時に `clap_host_params.rescan(CLAP_PARAM_RESCAN_VALUES)`
  を要求（CLAP 契約: state load による値変化は host へ通知必須）。
  host 不在環境では rescan を skip。
- CLAP `Cargo.toml`: stateful が host handle を保持する旨コメント更新。
- VST3 `component.rs`: `terminate` を非終端化（Created/Active から
  再 setup 可能）、`setupProcessing` 受理 state 拡大、bus info 修正。

## 実行結果（canonical）

- `node ci/plugin-abi-validator/run.mjs` → verdict **Passed**, exit 0。
  - clap-validator 0.4.1: test_gain 26/17skip/1warn、test_fault 19/17/1
    （param 除外）、test_stateful 31/12/1、test_instrument 30/14/0、
    test_editor 17/26/1 — 全て 0 failed。
  - vst3-sdk-validator（VST 3.8.1）: 235 passed, 0 failed。
  - test_fault unfiltered informational leg: validator exit 101
    （tool 側 panic — `param-fuzz-sample-accurate`）→ Unsupported 記録。
- `node --test ci/plugin-abi-validator/tests/` → 18 pass, exit 0。
- warning は全 plugin で denormal 処理性能の情報通知（非失敗）。

## evidence

`evidence/rf630/abi-validator/run.json` + `clap-*.txt` +
`vst3-validator.txt`、`evidence/rf630/abi-validator-verification.md`。
