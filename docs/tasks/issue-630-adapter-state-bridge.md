---
id: "TASK-034Y14fuNy9GI2NIIXyz1U"
type: "task"
status: "done"
title: "Issue #630 adapter-state-bridge slice — task"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-630-adapter-state-bridge.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-630-adapter-state-bridge.md"
  implements:
    - "docs/plans/issue-630-adapter-state-bridge.md"
  implemented-by:
    - "docs/impl/ir/issue-630-adapter-state-bridge.md"
  depends-on:
    - "docs/plans/issue-630-adapter-state-bridge.md"
    - "docs/tasks/issue-630-l1-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-adapter-state-bridge.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-630-adapter-state-bridge.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-630-adapter-state-bridge.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 adapter-state-bridge slice — task

- [x] VST3 `host.rs`: `BridgeStream`（`IBStream` ComWrapper —
      read/write/seek/tell、out-ptr null 許容、部分読み書き正直
      報告、非負 seek 許可・負/overflow は `kResultFalse`）+
      blob framing helper（`[u32 LE comp_len][comp][u32 LE
      ctrl_len][ctrl]`）+ state 呼び出し列 free function +
      `Vst3Plugin::save_state`/`restore_state`（phase gate・
      single-component 分岐・`kNotImplemented`→`Unsupported`/
      その他→`PluginFailed` mapping）+ module doc 更新
- [x] VST3 `host.rs` test: stateful COM double（distributed +
      single-component 両形 round-trip、malformed blob、
      seek/tell/partial read、`kNotImplemented`→`Unsupported`、
      pre-init→`PluginFailed`、fixture stateless→`Unsupported`）
- [x] `controller.rs`: `GainController::setState`/`getState` を
      `kNotImplemented` に修正（side finding）+ test pin
- [x] CLAP `Cargo.toml`: `host` feature に
      `"clack-extensions/state"` 追加
- [x] CLAP `host.rs`: `PluginState` import +
      `ClapPlugin::save_state`/`restore_state`（`instance=None`→
      `PluginFailed`、extension 無し→`Unsupported`、`StateError`→
      `PluginFailed`）+ module doc 更新 + test 書き換え
- [x] 検証: fmt-check / check / lint / test（両 crate）/
      check-l1-real-plugin / lint-l1-real-plugin /
      test-l1-real-plugin / test-rt / prek / git diff scope
- [x] `evidence/rf630/adapter-state-bridge-verification.md` +
      docs index 再生成（doc-maintenance `index apply`）
- [x] review-protocol レビュー（blocker 無し）
