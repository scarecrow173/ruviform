---
id: "IMPL-630presetSpawnFault07"
type: "impl"
status: "completed"
title: "Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — 実装記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-630-preset-spawn-fault.md"
    - "docs/plans/issue-630-preset-spawn-fault.md"
    - "docs/specs/issue-630-preset-spawn-fault.md"
    - "docs/designs/issue-630-preset-spawn-fault.md"
    - "docs/adr/issue-630-test-spawn-fault-runner-argv.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/preset.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/preset.rs"
      - type: "file"
        path: "evidence/rf630/test-preset-spawn-fault-verification.md"
      - type: "file"
        path: "docs/discovery/issue-630-preset-spawn-fault.md"
      - type: "file"
        path: "docs/specs/issue-630-preset-spawn-fault.md"
      - type: "file"
        path: "docs/designs/issue-630-preset-spawn-fault.md"
      - type: "file"
        path: "docs/plans/issue-630-preset-spawn-fault.md"
      - type: "file"
        path: "docs/tasks/issue-630-preset-spawn-fault.md"
      - type: "file"
        path: "docs/test-specs/issue-630-preset-spawn-fault.md"
      - type: "file"
        path: "docs/adr/issue-630-test-spawn-fault-runner-argv.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-preset-spawn-fault.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-preset-spawn-fault.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-preset-spawn-fault-exp.jsonl"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/stateful.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/factory.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/bin/ruviform-plugin-test-vst3-l1-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_fault_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/stateful.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/bin/ruviform-plugin-test-clap-l1-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_fault_test_plugin_e2e.rs"
  implements:
    - "docs/tasks/issue-630-preset-spawn-fault.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-630-preset-spawn-fault.md"
  blocks: []
  supersedes: []
  superseded-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-630-preset-spawn-fault-exp.jsonl"
    rejected: []
---

# Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — 実装記録

詳細な時系列イベントは companion JSONL
`docs/impl/exp/issue-630-preset-spawn-fault-exp.jsonl`
（`experiment_event.v1` schema）を参照。

## 実装したもの

### VST3 preset surface（`crates/ruviform-plugin-test-vst3`）

- `src/preset.rs` 新設: preset catalog（4 件・`RFST` codec と同一
  byte 表現）、`PresetSurface`（live gain cell 共有）、
  controller 側 `IUnitInfo`/`IUnitHandler`/`IUnitHandler2` と
  component 側 `IProgramListData`/`IUnitData` の COM 実装、
  in-process ABI test。
- `src/stateful.rs`: `TestStream` を共有可能化し preset/state が
  同一 gain cell を参照。`TestStatefulComponent` alias/impl を
  preset.rs 側へ集約。
- `src/factory.rs`: stateful controller 分岐で preset interface を
  query 対応に拡張。

### CLAP preset surface（`crates/ruviform-plugin-test-clap`）

- `Cargo.toml`: `clack-extensions` `preset-discovery` feature を
  有効化（Coordinator 承認済み。`state-context` は preset load が
  既存 state blob を再利用するため不採用）。
- `src/preset.rs` 新設: static catalog（VST3 と同一 4 preset）、
  `PresetDiscoveryFactoryWrapper`、in-process ABI test
  （host-side indexer/receiver/probe 経由）。
- `src/stateful.rs`: `clap.ext.preset-load`
  （`PluginPresetLoadImpl`）を実装し preset 選択を gain 復元へ
  写像。state blob と codec を共有。
- `src/lib.rs`: entry factory に preset-discovery extension を登録。

### spawn/pre-Hello fault（両 crate の l1-runner bin）

- `SpawnFault` enum（`exit`/`abort`/`hang`/`stderr`/`bad-fixture`）
  を bin に追加。`--ruviform-test-fault=MODE` を fixture load 後・
  `run()` 前に消費。`run()` 本体は無変更（no-argv 契約維持）。

### `test_spawn_fail`

- VST3: `CLASS_ROWS` 10→11・`countClasses` 更新・`createInstance`
  非 dispatch（`kInvalidArgument`）。
- CLAP: `plugin_count` 5→6・descriptor 追加・`create_plugin → None`。

### L1 oracle

- 両 `tests/l1_fault_test_plugin_e2e.rs` に spawn-fault section
  追加（計 16 tests per ABI）: pre-Hello exit/abort/hang
  （Timeout pin）/stderr capture/usage/bad-fixture、descriptor
  mismatch `Handshake`、init-phase `PluginFailed`。
- `PRE_HELLO_DEATH` = `Transport`/`ExitObserved`/`Internal` 集合
  oracle（高速死亡の分類 race を吸収）。
- `tests/host_path.rs`: descriptor 数・catalog 期待を更新。

## 実行結果（canonical）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `cargo fmt --all -- --check` | 0 | Passed（補助） |
| `cargo clippy -p ruviform-plugin-test-{vst3,clap} --features l1-runner --all-targets -- -D warnings` | 0 | Passed（補助） |
| `cargo test -p ruviform-plugin-test-{vst3,clap}` | 0 | 91 + 66 tests pass（補助） |
| `mise run test-l1-real-plugin` | 0 | adapter 3+3 / real 10+10 / fault 16+16 / stateful 2+2 全緑（canonical） |
| `mise run test-rt` | 0 | Passed（canonical） |
| `mise exec -- prek run --all-files` | 0 | Passed（canonical） |

canonical = `mise.toml` の task。`cargo` 直接実行は crate 単位の
補助確認であり CI lane の正本ではない。全検証は local worktree
実行であり `origin/main` merge 後の統合検証は未実施。

## evidence

`evidence/rf630/test-preset-spawn-fault-verification.md` —
実装内容・検証結果・正直な境界（mid-handshake 死亡の注入不可、
wire `CreateInstance` dormant、`InstanceLimit` 到達不能、
validator opt-in NotRun、検証区分）を記録。
