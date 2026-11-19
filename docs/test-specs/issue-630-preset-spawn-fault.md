---
id: "TSPEC-630presetSpawnFault06"
type: "test-spec"
status: "approved"
title: "Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — test specification"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-preset-spawn-fault.md"
    - "docs/designs/issue-630-preset-spawn-fault.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-preset-spawn-fault.md"
  implements:
    - "docs/specs/issue-630-preset-spawn-fault.md"
    - "docs/designs/issue-630-preset-spawn-fault.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-test-stateful.md"
    - "docs/test-specs/issue-630-test-fault.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — test specification

## Feature: preset surface（VST3/CLAP・in-process ABI）

### Rule: preset catalog は両 ABI で同一でなければならない

- Example: VST3 `IProgramListData` と CLAP preset-discovery
  catalog が同一 4 preset（同名・同 gain 値）を返す。

### Rule: preset 選択は live gain に反映されなければならない

- Example: preset 選択後に `ParamValue`/state save で gain が
  preset 値になり、`IUnitData`/`preset-load` 経由の復元も同一
  codec で往復する。

### Rule: preset state は既存 state blob と一致しなければならない

- Example: preset 選択 → state save → preset load → state save の
  byte 列が決定的（`RFST` codec・version=1）である。

## Feature: spawn/pre-Hello fault（L1 e2e・headless）

### Rule: fault は fixture load 後・Hello 前に発火しなければならない

- Example: `exit:N`/`abort`/`hang`/`stderr:N`/`bad-fixture` の
  各 mode が実 runner binary で pre-Hello に観測される。

### Rule: 親は定義済み error に収束し deadlock してはならない

- Example: `hang` は親の Hello deadline で `Timeout`、高速死亡は
  `Transport`/`ExitObserved`/`Internal`、fixture 失敗は同集合、
  descriptor 不一致は `Handshake` に pin される。

### Rule: stderr marker は capture 可能でなければならない

- Example: `stderr:N` が marker bytes を出力し、親の diagnostics
  query で `Found` として取得できる。

### Rule: 未認識 argv は fail-closed でなければならない

- Example: 未知の `--ruviform-test-fault=` 値・任意の余分 argv は
  `run()` の usage rejection に分類され、fixture は spawn されない。

### Rule: create 拒否 fixture は scan を壊してはならない

- Example: `test_spawn_fail` は descriptor 列挙を通過し、
  create/Initialize が `PluginFailed` で拒否される。他 plugin の
  scan・instantiate は影響を受けない。

## 検証コマンド（canonical）

- `mise run test-l1-real-plugin`（`--features l1-runner` +
  `-- --ignored` lane、実 cdylib + 実 runner binary）
- 補助: `cargo test -p ruviform-plugin-test-{vst3,clap}`（in-process
  ABI test）、`mise run test-rt`、`mise exec -- prek run --all-files`
