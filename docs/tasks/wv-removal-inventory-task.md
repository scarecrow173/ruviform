---
id: "TASK-kUY7hTjLYAgqQfA46CSw7r"
type: "task"
status: "done"
title: "Issue #465 WebView 移行台帳現行化 — 分類検証と台帳確定"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/465"
  changes:
    added:
      - type: "file"
        path: "evidence/rf465/inventory.json"
      - type: "file"
        path: "evidence/rf465/README.md"
      - type: "doc"
        path: "docs/specs/wv-removal-inventory.md"
      - type: "doc"
        path: "docs/designs/wv-removal-inventory-design.md"
      - type: "doc"
        path: "docs/plans/wv-removal-inventory-plan.md"
      - type: "doc"
        path: "docs/test-specs/wv-removal-inventory.md"
      - type: "doc"
        path: "docs/impl/ir/wv-removal-inventory.md"
      - type: "doc"
        path: "docs/impl/exp/wv-removal-inventory.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-webview/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
  implements:
    - "../plans/wv-removal-inventory-plan.md"
  implemented-by: []
  depends-on:
    - "../plans/wv-removal-inventory-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/wv-removal-inventory.md"
    - "../designs/wv-removal-inventory-design.md"
    - "../test-specs/wv-removal-inventory.md"
    - "../discovery/issue-465-legacy-webview-inventory.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/wv-removal-inventory-plan.md"
    - "../designs/wv-removal-inventory-design.md"
    - "../specs/wv-removal-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "evidence/rf465/inventory.json"
  defers: []
  deferred-by: []
---

# Issue #465 WebView 移行台帳現行化 タスク

関連文書: [plan](../plans/wv-removal-inventory-plan.md) /
[spec](../specs/wv-removal-inventory.md) /
[design](../designs/wv-removal-inventory-design.md) /
[test-spec](../test-specs/wv-removal-inventory.md) /
[IR](../impl/ir/wv-removal-inventory.md) /
[EXP](../impl/exp/wv-removal-inventory.md) /
[台帳](../../evidence/rf465/inventory.json)

- [x] T1 現行 main の WebView 資産再棚卸し（rg/git log/Cargo/mise/CI 精査）
- [x] T2 AL/SV consumer 参照の列挙（read-only・origin/main HEAD）
- [x] T3 `evidence/rf465/inventory.json` + `README.md` 作成（4 分類・差分・Owner/Trigger）
- [x] T4 doc chain 作成（spec/design/plan/task/test-spec/ir/exp）
- [x] T5 `crates/ruviform-ui-webview` doc 注記（lib.rs + README、API 不変）
- [x] T6 生成 index 再生成 + doc audit
- [x] T7 merge origin/main → mise 正規検証 → review-protocol
- [x] T8 commit / push / PR（Refs #465）
