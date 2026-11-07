---
id: "TASK-034T6RWXn4Bv9Hg8sM2eR5"
type: "task"
status: "proposed"
title: "T4 GPUI Controls/Viz renderer(ruviform-ui-gpui::render)"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "../plans/implement-issue-387-component-renderer.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/component_render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/component_showcase.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-387-component-renderer.md"
  implemented-by: []
  depends-on:
    - "../tasks/t1-component-view-spec-issue-387.md"
    - "../tasks/t2-headless-behavior-issue-387.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/issue-387-component-renderer.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-387-component-view-specs.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T4: GPUI Controls/Viz renderer

## Work

`ruviform-ui-gpui::render` 新設。`ThemeMap`(ResolvedTheme→Palette,
canonical semantic token + literal fallback)、`RvfView`(`ViewHost`
保持の `Render` entity):Panel/Tabs/Fader/Seek/Waveform の spec→
element 写像、mouse/scroll/keyboard→`ControlInput` normalize、
intent→`IntentSink`。Waveform は `gpui::canvas` で `ViewBin` 描画。
未対応 kind は typed `UnsupportedKind` を sink へ。example
`component_showcase.rs` で実 window fixture、mise lane 追加。
`tests/component_render.rs` で `TestAppContext` headless parity を
試行(不可なら gpui binding `not_run`+rerun 維持)。

## Acceptance

- `mise run check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` pass。
- `mise run example-ui-gpui-components` が build し manual 起動可。
- ThemeMap unit test: 2 theme → 異なる palette。
