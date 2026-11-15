---
id: "SPEC-034Y13Y73IFhhGtTmihC8f"
type: "spec"
status: "approved"
title: "Issue #630 adapter-state-bridge slice — VST3 IBStream + CLAP clap.state host bridge"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-l1-runner.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/controller.rs"
      - type: "file"
        path: "docs/specs/issue-630-adapter-state-bridge.md"
      - type: "file"
        path: "docs/designs/issue-630-adapter-state-bridge.md"
      - type: "file"
        path: "docs/plans/issue-630-adapter-state-bridge.md"
      - type: "file"
        path: "docs/tasks/issue-630-adapter-state-bridge.md"
      - type: "file"
        path: "docs/test-specs/issue-630-adapter-state-bridge.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-adapter-state-bridge.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-adapter-state-bridge.jsonl"
      - type: "file"
        path: "evidence/rf630/adapter-state-bridge-verification.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-l1-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-l1-runner.md"
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-l1-runner.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 adapter-state-bridge slice — VST3 IBStream + CLAP clap.state host bridge

## What Should Be Built

Issue #630 AC-2 (State Save/Reload/Preset) の production 側前提と
して、両 adapter host bridge の標準 ABI state 境界を実装する:

- **VST3 (`crates/ruviform-plugin-vst3/src/host.rs`)** —
  `BridgeStream`（`IBStream` の `ComWrapper` 実装:
  `Vec<u8>` + 位置を `RefCell` で保持、`read`/`write`/`seek`/
  `tell`、out-ptr null 許容・部分読み書きの正直な報告）を新設し、
  `Vst3Plugin::{save_state,restore_state}` を VST3 state 呼び出し
  列へ橋渡しする。blob framing は
  `[u32 LE comp_len][comp bytes][u32 LE ctrl_len][ctrl bytes]`
  （single-component は `ctrl_len=0`）。restore は VST3 spec の
  順序 `component.setState` → `component.getState` →
  `controller.setComponentState` → `controller.setState` に従う。
  plugin が `kNotImplemented` を返した場合は
  `Err(HostError::Unsupported)`、その他の失敗 tresult は
  `Err(HostError::PluginFailed)`、8 byte 以上で parse 不能な
  corrupt frame は `Err(HostError::PluginFailed)`。8 byte 未満の
  blob は本 framing が生成し得ない foreign state として plugin
  へ委譲する（design「foreign blob の扱い」— L1 e2e の
  `Unsupported` pin との整合のための確定事項）。
- **CLAP (`crates/ruviform-plugin-clap/src/host.rs` +
  `Cargo.toml`)** — `host` feature に `"clack-extensions/state"`
  を追加し、`ClapPlugin::{save_state,restore_state}` を
  `clap.ext.state`（`PluginState::save`/`load`、
  `PluginMainThreadHandle` 経由）へ橋渡しする。extension 非搭載
  plugin は `Err(HostError::Unsupported)`、`StateError` は
  `Err(HostError::PluginFailed)`。host 側 `HostState`/`mark_dirty`
  は登録しない（KISS — dirty 通知は後続 slice の判断材料）。
- **side finding 修正 (`controller.rs`)** — fixture
  `GainController::setState`/`getState` が `kResultOk` の空
  no-op を返す不正直 stub を `kNotImplemented` に修正し、
  component 側の正直 stub と対称にする。
- **doc/evidence chain** — 本 slice の spec→design→plan→task→
  test-spec→impl 記録と `evidence/rf630/` 検証記録。

## Why Now

中立 contract（`Plugin::save_state`/`restore_state`）、wire
command（`SaveState`/`RestoreState` + `WIRE_MAX_STATE_BYTES`）、
L1 child dispatch は完成済みだが、両 adapter host bridge が
`Plugin` default（`Err(Unsupported)`）のままで、標準 ABI の
state 境界が欠落している。`test_stateful` fixture slice の前に
production 側の橋渡しを確立し、real-ABI state oracle の土台を
作る。

## Acceptance Criteria (this slice)

- [ ] VST3: `BridgeStream` が `IBStream` ABI を正しく実装し、
  save/restore が component/controller state 呼び出し列を
  framing 付きで駆動する（in-process COM double test で
  round-trip・single/distributed 両形・malformed blob・
  seek/tell/partial read を検証）。
- [ ] VST3: workspace fixture（state 未実装）に対して
  `save_state`/`restore_state` が定義済み decline
  （`Err(Unsupported)`）を返し、既存 L1 e2e の
  `Err(Unsupported)` pin を維持する。
- [ ] CLAP: `clap.ext.state` がある plugin に対し
  `save_state`/`restore_state` が `PluginState` host call を
  駆動し、extension 非搭載の workspace fixture は
  `Err(Unsupported)` を維持する。
- [ ] `controller.rs` の不正直 stub が `kNotImplemented` に
  修正され、test で pin される。
- [ ] `save_state`/`restore_state` は non-RT verb —
  `BridgeStream` の `Vec`/`RefCell` は state 呼び出し時のみ
  生成・破棄され、`process` の alloc/lock/wait-free 不変条件を
  侵さない（`rt_` テストは不変条件として緑を維持）。
- [ ] 正直な境界を記録: real-plugin state evidence は
  `test_stateful` fixture slice まで NotRun（in-process double
  の成功を real-plugin Pass と読み替えない）、外部 SDK
  validator NotRun、Preset/preset-list・`mark_dirty`・
  `test_instrument`/`test_editor`・非 Windows OS・`*-msrv`/
  `*-full` lane は Deferred/NotRun。

## Non-Goals

- `ruviform-plugin-test-*` fixture への一切の変更
  （`test_stateful` は後続 slice）。
- `ruviform-plugin-{core,host,export}`・adapter crate の
  lease 外ファイル、`mise.toml`、`.github/**` の変更。
- host `mark_dirty`/dirty 通知経路（CLAP `HostState`・VST3
  `IComponentHandler` 系）の実装 — 記録のみ。
- Preset/preset-list、native editor、consumer oracle 照合。
- PR は `Refs #630` — Issue は open のまま。
