---
id: "SPEC-630presetSpawnFault02"
type: "spec"
status: "approved"
title: "Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — spec"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-preset-spawn-fault.md"
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/specs/issue-630-test-fault.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-630-preset-spawn-fault.md"
  implements:
    - "docs/discovery/issue-630-preset-spawn-fault.md"
  implemented-by:
    - "docs/designs/issue-630-preset-spawn-fault.md"
  depends-on:
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/specs/issue-630-test-fault.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — spec

## 目的

Issue #630 の test-only VST3/CLAP fixture を拡張し、production host
が実 plugin ABI binary に対して (a) preset save/load/restore の
往復、(b) spawn/pre-Hello phase の fault を headless L1 child
process で検証できるようにする。production host bridge と
`ruviform_plugin_export::run` を実実行経路とし、test-only の
host shortcut は作らない。

## 要求

- `test_stateful` に VST3 preset surface（controller:
  `IUnitInfo`/`IUnitHandler`/`IUnitHandler2`、component:
  `IProgramListData`/`IUnitData`）を実装し、preset 選択が live
  gain に反映され state codec で往復すること。
- `test_stateful` に CLAP preset surface（`preset-discovery`
  factory extension + `preset-load` plugin extension）を実装し、
  catalog が VST3 と同一であること。
- 両 l1-runner bin が `--ruviform-test-fault=MODE` を受理し、
  fixture load 後・`run()` 前に `exit:N`/`abort`/`hang`/`stderr:N`/
  `bad-fixture` を実行すること。`run()` の no-argv 契約は不変。
- 両 suite に列挙されるが生成拒否する `test_spawn_fail`
  descriptor/class を追加し、scan 成功・create 拒否を検証すること。
- 上記を headless L1 e2e（`l1-runner` feature + `#[ignore]`）で
  実 cdylib + 実 runner binary 経由で検証すること。

## 受入条件（本 slice）

- [ ] VST3/CLAP の preset catalog・preset 選択・state 復元が
  in-process ABI test で pass する。
- [ ] spawn fault 各 mode が pre-Hello に発火し、親が定義済み
  `FailureCause`/`HostError` に収束する（deadlock なし）。
- [ ] `test_spawn_fail` が scan を通過し create 拒否が
  `Initialize → PluginFailed` として観測される。
- [ ] `run()` 無変更・no-argv 契約維持・未認識 argv は usage
  rejection（fail-closed）。
- [ ] `mise run test-l1-real-plugin` 全緑、fmt/clippy/
  `test-rt`/prek pass。
- [ ] 逸脱（専用 bin ではなく runner argv を採用）の根拠と、
  validator opt-in NotRun・mise/cargo・local/統合の検証区別を
  design doc・evidence・PR 本文に記録する。

## 非目標

- production crate（`ruviform-plugin-{vst3,clap,host,export,core}`）
  の変更なし。
- preset/bank 管理 UI、foreign plugin state format の移行なし。
- Hello 後 handshake 中の死亡注入（fixture から到達不能）。
- wire `CreateInstance` verb 自体の拒否注入（dormant・plugin code
  を実行しない）。
- `InstanceLimit`（host pooling により到達不能）。
- 外部 ABI validator の preset surface 検証（別 slice・NotRun）。
- PR は `Refs #630` — Issue は close しない。
