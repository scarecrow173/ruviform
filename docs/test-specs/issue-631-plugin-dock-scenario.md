---
id: "TSPEC-034XEbbePtPeJtXIR2bxrL"
type: "test-spec"
status: "approved"
title: "Issue #631 Plugin-dock scenario skeleton のテスト仕様"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-631-plugin-dock-scenario.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/issue-631-plugin-dock-scenario.md"
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "docs/designs/issue-631-plugin-dock-scenario.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-631-plugin-dock-scenario.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-631-plugin-dock-scenario.md"
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "docs/designs/issue-631-plugin-dock-scenario.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "docs/designs/issue-631-plugin-dock-scenario.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Plugin-dock scenario skeleton のテスト仕様

## Feature: plugin-dock scenario skeleton

### Rule: artifact evidence is recorded, never assumed

- **Example**: `test_gain` cdylib が解決できる環境で lane を実行すると、case evidence に path・SHA-256 digest・plugin descriptor（id/name/vendor/version）が記録される。
- **Example**: artifact path が存在しない場合、artifact 依存 case は `NotRun` で記録され、silent skip も `Pass` も起きない。

### Rule: verdict は実走結果のみを反映する

- **Example**: `Vst3Library::load`・descriptor discover・host instantiate/initialize/activate が成功した step は `pass`。
- **Example**: `session.open(Docked)` が `createView` null → `HostError::Unsupported` → `EditorOpError::Unsupported` で失敗した場合、editor-attach case は `unsupported`、dock pixel case は `not_run`。`pass` にはならない。
- **Example**: `ScenarioReport`/journal 行は run_id・scenario・case・driver・capabilities・required・verdict・environment（OS・scale・repo SHA・plugin digest）を含む。

### Rule: driver-neutral contract の再利用

- **Example**: ui-test の plugin-dock spec は `NativeSlot` 入り `ViewSpec` と `required={NativePluginDock, VisualCapture}` を提供し、GPUI 型を signature に含まない。
- **Example**: `view-mount` case は `FakeGuiTest` + `run_scenario` を通り `driver=Fake`・semantic evidence のみを記録する（Dock PASS を主張しない）。

### Rule: test-only 境界

- **Example**: 本番クレートは `ruviform-plugin-dock-test` に依存しない。公開 enum は `#[non_exhaustive]`。
- **Example**: `mise run test-ui-plugin-dock` は manual/opt-in で CI 必須条件にしない。
