---
id: "TASK-BS44ckVmFBThwWFjdUUaro"
type: "task"
status: "done"
title: "Issue #465 Legacy WebView P0/P1 — 文書歴史化と棚卸し"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/465"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-465-legacy-webview-inventory.md"
      - type: "doc"
        path: "docs/specs/issue-465-legacy-webview-p0p1.md"
      - type: "doc"
        path: "docs/designs/issue-465-legacy-webview-p0p1.md"
      - type: "doc"
        path: "docs/adr/issue-465-legacy-webview-retirement-status.md"
      - type: "doc"
        path: "docs/plans/implement-issue-465-legacy-webview-p0p1.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/README.md"
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/event.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/window.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/ipc.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/bridge.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/detect.rs"
      - type: "doc"
        path: "docs/specs/issue-310-browser-gui-test-harness.md"
      - type: "doc"
        path: "docs/designs/issue-310-browser-gui-test-harness.md"
      - type: "doc"
        path: "docs/test-specs/issue-310-browser-gui-test-harness.md"
      - type: "doc"
        path: "docs/specs/issue-311-ipc-conformance-replay.md"
      - type: "doc"
        path: "docs/designs/issue-311-ipc-conformance-replay.md"
      - type: "doc"
        path: "docs/test-specs/issue-311-ipc-conformance-replay.md"
      - type: "doc"
        path: "docs/specs/issue-312-test-gui-host-lifecycle.md"
      - type: "doc"
        path: "docs/designs/issue-312-test-gui-host-lifecycle.md"
      - type: "doc"
        path: "docs/test-specs/issue-312-test-gui-host-lifecycle.md"
      - type: "doc"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-465-legacy-webview-p0p1.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-465-legacy-webview-p0p1.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-465-legacy-webview-p0p1.md"
    - "../designs/issue-465-legacy-webview-p0p1.md"
    - "../adr/issue-465-legacy-webview-retirement-status.md"
    - "../discovery/issue-465-legacy-webview-inventory.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-465-legacy-webview-p0p1.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #465 Legacy WebView P0/P1 タスク

関連文書: [plan](implement-issue-465-legacy-webview-p0p1.md) /
[spec](../specs/issue-465-legacy-webview-p0p1.md) /
[design](../designs/issue-465-legacy-webview-p0p1.md) /
[ADR](../adr/issue-465-legacy-webview-retirement-status.md) /
[inventory](../discovery/issue-465-legacy-webview-inventory.md)

- [x] T1 棚卸し doc（discovery）作成 — legacy 資産・API 分類・consumer 影響・P2 gate
- [x] T2 spec / design / ADR / plan / task の doc chain 作成
- [x] T3 Issue #310/#311/#312 系 docs への legacy banner 注記
- [x] T4 `docs/development-environment.md` の lane 注記
- [x] T5 `crates/ruviform-ui` README + src doc コメント整理
- [x] T6 生成 index 再生成 + doc audit
- [x] T7 merge origin/main → mise 正規検証 → review-protocol
- [x] T8 commit / push / PR（Refs #465）
