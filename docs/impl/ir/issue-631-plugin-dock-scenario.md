---
id: "IMPL-034XEbTj64ladkAkB3T5ww"
type: "impl"
status: "completed"
title: "Plugin-dock scenario skeleton (issue-631 first slice)"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/designs/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/plans/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/tasks/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/test-specs/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/impl/ir/issue-631-plugin-dock-scenario.md"
      - type: "dir"
        path: "crates/ruviform-plugin-dock-test"
      - type: "file"
        path: "evidence/rf631/verification.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "crates/ruviform-audio-test/src/metadata.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/plugin_dock.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-631-plugin-dock-scenario.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-631-plugin-dock-scenario.md"
    - "docs/plans/issue-631-plugin-dock-scenario.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "docs/designs/issue-631-plugin-dock-scenario.md"
    - "docs/test-specs/issue-631-plugin-dock-scenario.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-631-plugin-dock-scenario.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-631-plugin-dock-scenario.md"
    rejected: []
---

# Plugin-dock scenario skeleton (issue-631 first slice)

## Summary

Issue #631 first slice: test-only crate `ruviform-plugin-dock-test` に plugin-dock scenario skeleton を実装 — `test_gain` 実 ABI cdylib の解決/digest/load/discover/host lifecycle を `Pass`、editor attach/dock/pixel を `Unsupported`/`NotRun` として `ScenarioReport` + JSONL evidence に記録する。

## Context

`docs/tasks/issue-631-plugin-dock-scenario.md` / `docs/plans/issue-631-plugin-dock-scenario.md` / `docs/specs/issue-631-plugin-dock-scenario.md` に従う。verdict 正直性が核心: `createView` null → `HostError::Unsupported` の現状で Dock Pass を主張しない。

## Implementation

### Changes

- `crates/ruviform-audio-test`: `EnvironmentKind` に
  `RealAbiTestPlugin` / `RealWindow` を追加（wire:
  `real-abi-test-plugin` / `real-window`）。既存
  `DeterministicBackend` は不変。
- `crates/ruviform-ui-test`: `ScenarioDriver::PluginHostProbe`
  （`plugin-host-probe`）を追加し、`scenario/plugin_dock.rs` に
  plugin-dock spec（`NativeSlot` を含む ViewSpec、
  `required = {NativePluginDock, VisualCapture}`）と
  `PluginArtifactEvidence` を追加。
- `crates/ruviform-plugin-dock-test`（新規 test-only crate）:
  artifact 解決（env var + `current_exe`→`deps/` 自動解決）・
  SHA-256 digest・descriptor 由来の plugin id/name/vendor/version 記録、
  `view-mount`（fake semantic）/ `artifact` / `host-lifecycle` /
  `editor-attach` / `dock-pixels` の 5 case runner、
  `plugin-dock-scenario` bin（JSONL 出力）。
- `mise.toml`: `[tasks.test-ui-plugin-dock]` を追加のみ
  （manual/opt-in。CI 必須 lane・workflow には未接続）。
- `evidence/rf631/`: lane 実走 JSONL と検証記録。

### Decisions During Implementation

- `ScenarioReport` は意図的に非 Serialize のため、JSONL case 行は
  bin の出力境界で public field から `serde_json::Map` 組立。adapter
  内部 ID（`targets`）は evidence 面に出さない。
- `editor-attach` は `FakeNativeSlot` lease を
  `validate_lease`→`ParentLease` で redeem する実 contract 経路を通す。
  `test_gain` は `createView` null → `HostError::Unsupported` →
  `EditorOpError::Unsupported` のため verdict は `Unsupported`。
  仮に将来 Pass になっても fake slot 経由の contract 適合証拠であり、
  実 Dock 証拠は `dock-pixels` が担う。
- artifact 未解決時は `artifact`/`host-lifecycle`/`editor-attach` を
  `NotRun`/`Fail` で fail-closed にし、silent skip しない。
- clippy `too_many_lines` は `#[allow]` ではなく helper 抽出で解消
  （`test_gain_config` / `fake_dock_slot_spec` / `attempt_editor_attach`）。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-631-plugin-dock-scenario.md` —
  実装過程の観察記録（clippy lint 対応、verdict 実測確認など）。
  イベントログ:
  `docs/impl/exp/plugin-dock-scenario-skeleton-probes.jsonl`。

### Rejected

- なし。

## Validation

- `mise run fmt-check`: pass
- `mise run lint ruviform-plugin-dock-test`（`-D warnings`）: pass
- `mise run check|test ruviform-plugin-dock-test`: pass
  （integration 2 test: missing-artifact fail-closed、fixture verdict）
- `mise run check|lint|test ruviform-audio-test` /
  `ruviform-ui-test` / `ruviform-plugin-test-vst3`: pass
- `mise run test-ui-plugin-dock`: pass — 5 case 実走
  （`view-mount=pass`, `artifact=pass`, `host-lifecycle=pass`,
  `editor-attach=unsupported`, `dock-pixels=not_run`）
- scenario contract lanes（`test-ui-scenario-contract{,-fake,-gpui}`,
  `test-ui-native-slot`, `test-ui-golden`, `test-ui-driver-contract`,
  `test-ui-gpui`, `check-ui-driver-boundary`）: pass
- 詳細は `evidence/rf631/verification.md`。

## Risks

- `ScenarioReport` 非 Serialize → JSONL は出力層で public field から組立。
- `editor-attach` Pass（将来）も fake slot 経由の contract 証拠に留まる
  設計 — 実 Dock 判定は `dock-pixels`/後続 slice の責務。

## Follow-ups

- `test_editor` fixture / 実 Window lane / `RealWindow` kind の実走利用は後続 slice。
- 実 Native Dock・画素・Input・DPI/IME の Pass 判定は editor 実物
  （`scarecrow173/ruviform#630` 残存 scope）到着後。
