---
id: "TSPEC-034Y14780zESXWkqYxDVNj"
type: "test-spec"
status: "approved"
title: "Issue #630 adapter-state-bridge slice — test specification"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/designs/issue-630-adapter-state-bridge.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-adapter-state-bridge.md"
  implements:
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/designs/issue-630-adapter-state-bridge.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-630-adapter-state-bridge.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/designs/issue-630-adapter-state-bridge.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/designs/issue-630-adapter-state-bridge.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 adapter-state-bridge slice — test specification

## Purpose

両 adapter host bridge の標準 ABI state 境界（VST3 `IBStream` /
CLAP `clap.ext.state`）が中立 `save_state`/`restore_state` を
正直に駆動することを pin する。in-process COM double / fixture
defined-decline が本 spec の oracle — real-plugin state oracle
は `test_stateful` fixture slice で初めて到着する。

## Feature: VST3 `BridgeStream` + state bridge

- **Rule**: `BridgeStream` は `IBStream` ABI を正直に実装する —
  read/write は部分転送を実数で報告し、seek/tell は位置を正しく
  追跡し、null buffer・負 size・不正 mode は
  `kInvalidArgument`、負/overflow seek は `kResultFalse`。
  **Example**: `write` 後に `tell` で位置確認、`seek(0)`→`read`
  で先頭から再読、EOF での部分 read は `numBytesRead < numBytes`。
- **Rule**: `save_state` は component/controller の `getState`
  を呼び framing した blob を返す。single-component では
  controller 側を呼ばない（`ctrl_len=0`）。
  **Example**: distributed double で comp/ctrl 両 stream が
  埋まり framing が round-trip する。
- **Rule**: `restore_state` は `component.setState` →
  `component.getState` → `controller.setComponentState` →
  `controller.setState` の順で駆動し、8 byte 以上で parse 不能な
  corrupt frame は `Err(PluginFailed)`。8 byte 未満の blob は
  本 framing が生成し得ない foreign state として plugin へ
  委譲する（plugin の tresult がそのまま結果 — stateless
  fixture の `kNotImplemented` → `Unsupported` を守る
  L1 e2e oracle との整合。design「foreign blob の扱い」）。
  **Example**: save→restore で double の保持 state が復元
  される。trailing byte・len 超過の corrupt frame は拒否、
  `[0xAA,0xBB]` は plugin の `setState` へそのまま届く。
- **Rule**: plugin の state call が `kNotImplemented` →
  `Err(Unsupported)`、その他の失敗 → `Err(PluginFailed)`、
  pre-init → `Err(PluginFailed)`。
  **Example**: workspace fixture（`kNotImplemented` stub）で
  `Err(Unsupported)` — 既存 L1 e2e の pin と一致。

## Feature: CLAP `clap.ext.state` bridge

- **Rule**: `instance=None` → `Err(PluginFailed)`、extension
  非搭載 → `Err(Unsupported)`、`StateError` → `Err(PluginFailed)`。
  **Example**: workspace fixture（`PluginAudioPorts`/`PluginParams`
  のみ宣言）は init 後も `Err(Unsupported)` — L1 e2e pin 維持。
- **Rule**: extension 搭載 plugin では `PluginState::save`/
  `load` が `PluginMainThreadHandle` 経由で駆動され、blob は
  plugin 定義の生 byte stream をそのまま中継する。
  **Example**: in-process fixture では extension 無しのため
  本 rule の positive arm は `test_stateful` slice まで
  NotRun — negative arm（defined decline）のみ本 slice で検証。

## Feature: fixture stub honesty (`controller.rs`)

- **Rule**: `GainController::setState`/`getState` は
  `kNotImplemented` を返し、`kResultOk` の空 no-op を返さない。
  **Example**: `controller.rs` test が `kNotImplemented` を pin
  （`component.rs` stub 検証の mirror）。

## Guarantees

- `save_state`/`restore_state` は non-RT — `BridgeStream`・
  `Cursor`・blob `Vec` の確保は呼び出し内に限定され、
  `process` の alloc/lock/wait-free 不変条件を侵さない
  （`rt_` テスト不変条件として緑を維持）。
- 既存 L1 e2e（lease 外 oracle）の `Err(Unsupported)` pin が
  維持される — adapter unit test の成功を real-plugin Pass と
  読み替えない。

## Non-goals

- real-plugin state round-trip（`test_stateful` fixture slice）。
- `mark_dirty`/dirty 通知、Preset/preset-list、native editor、
  外部 SDK validator、非 Windows OS、`*-msrv`/`*-full` lane。

## Risk

- error mapping が `kNotImplemented`→`Unsupported` を誤ると
  既存 L1 e2e が壊れる — fixture stateless decline を必ず test
  で pin する。
- `BridgeStream` の seek/write 意味論が実プラグインの前提と
  ずれると `test_stateful` slice で発覚 — file-like 意味論を
  design で明文化済み。
