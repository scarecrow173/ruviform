---
id: "PLAN-630presetSpawnFault04"
type: "plan"
status: "approved"
title: "Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — 実装計画"
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
        path: "docs/plans/issue-630-preset-spawn-fault.md"
  implements:
    - "docs/specs/issue-630-preset-spawn-fault.md"
    - "docs/designs/issue-630-preset-spawn-fault.md"
  implemented-by:
    - "docs/tasks/issue-630-preset-spawn-fault.md"
  depends-on:
    - "docs/specs/issue-630-preset-spawn-fault.md"
    - "docs/designs/issue-630-preset-spawn-fault.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — 実装計画

## フェーズ

1. **VST3 preset surface**: `preset.rs` 新設（catalog +
   `PresetSurface` + COM type）、`stateful.rs` の共有 gain cell
   対応、`factory.rs` の controller 分岐拡張、in-process ABI test。
2. **CLAP preset surface**: `preset-discovery` feature 有効化、
   `preset.rs` 新設（catalog + discovery factory）、`stateful.rs`
   に `preset-load` extension、in-process ABI test。
3. **spawn-fault runner**: 両 l1-runner bin に `SpawnFault` parse +
   `act()`、fixture load 後・`run()` 前に発火。
4. **`test_spawn_fail`**: VST3 `CLASS_ROWS`/`countClasses`/
   `createInstance` 非 dispatch、CLAP `plugin_count`/descriptor/
   `create_plugin → None`。
5. **L1 oracle**: 両 `l1_fault_test_plugin_e2e.rs` に spawn-fault
   section 追加、host-path descriptor 期待更新。
6. **検証**: fmt/clippy/unit/`test-l1-real-plugin`/`test-rt`/prek、
   evidence 記録、doc chain、commit/push/PR。

## ファイルマップ

- `crates/ruviform-plugin-test-vst3/src/{preset.rs,stateful.rs,
  factory.rs,lib.rs}` + `src/bin/ruviform-plugin-test-vst3-l1-runner.rs`
  + `tests/{host_path.rs,l1_fault_test_plugin_e2e.rs}`
- `crates/ruviform-plugin-test-clap/{Cargo.toml,src/preset.rs,
  src/stateful.rs,src/lib.rs}` + `src/bin/ruviform-plugin-test-clap-l1-runner.rs`
  + `tests/{host_path.rs,l1_fault_test_plugin_e2e.rs}`
- `evidence/rf630/test-preset-spawn-fault-verification.md`
- `docs/{discovery,specs,designs,plans,tasks,test-specs,adr,impl}/`
  の `issue-630-preset-spawn-fault` chain

## リスク

- vendored binding の interface 帰属誤認 → discovery で確定済み。
- CLAP feature flag の依存追加 → Coordinator 承認済み
  （`preset-discovery` のみ、`state-context` 不採用）。
- `InstanceLimit` probe の到達不能 → 採用せず削除（discovery 記録）。
