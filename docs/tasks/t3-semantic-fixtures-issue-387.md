---
id: "TASK-034T6RWWz8Xa6Lf2tK5yQ7"
type: "task"
status: "proposed"
title: "T3 Semantic fixtures + manifest binding(ruviform-ui-test)"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "../plans/implement-issue-387-component-renderer.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/component_view.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/themes/synvelis-dark.dtcg.json"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/themes/audiolens-dark.dtcg.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/stories.json"
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

# T3: Semantic fixtures + manifest binding

## Work

`ruviform-ui-test/tests/component_view.rs` 新設。UI-DVR-02/03 の
機械 binding 実装と 2 story(SV fader+dock / AL waveform+seek)の
fake layer fixture、2-theme semantic 同値 fixture。fixture DTCG
theme 2 本を `fixtures/themes/` に配置。`cases.json`/`stories.json`
の該当 binding を `cargo-test` mechanism で `pass` へ更新
(`package: ruviform-ui-test`, `target: component_view`, filter)。
gpui binding は T4 結果で更新、real-os/manual は維持。

## Acceptance

- `cargo test -p ruviform-ui-test --test component_view` pass。
- `mise run test-ui-driver-contract` validation pass。
- test-spec の semantic fixtures 全項目を網羅。
