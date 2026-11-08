---
id: "PLAN-f7s6LZ2O4ASTSi0kJoiojB"
type: "plan"
status: "approved"
test-spec-skip: "挙動変更なし。文書の歴史化・分類注記と doc コメントのみの変更であり、新規 test-spec は作成しない。既存の driver 境界 lane（check-ui-driver-boundary / test-ui-driver-contract）と cargo test が回帰を担保する"
title: "Issue #465 Legacy WebView P0/P1 実装計画"
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
        path: "docs/tasks/implement-issue-465-legacy-webview-p0p1.md"
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
    generated:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/discovery/README.md"
      - type: "doc"
        path: "docs/adr/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
  implements:
    - "../specs/issue-465-legacy-webview-p0p1.md"
  implemented-by:
    - "../tasks/implement-issue-465-legacy-webview-p0p1.md"
  depends-on:
    - "../designs/issue-465-legacy-webview-p0p1.md"
    - "../discovery/issue-465-legacy-webview-inventory.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-465-legacy-webview-p0p1.md"
    - "../designs/issue-465-legacy-webview-p0p1.md"
    - "../adr/issue-465-legacy-webview-retirement-status.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-465-legacy-webview-p0p1.md"
    - "../designs/issue-465-legacy-webview-p0p1.md"
  derived-by:
    - "../tasks/implement-issue-465-legacy-webview-p0p1.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #465 Legacy WebView P0/P1 実装計画

## Goal

旧 WebView-first 前提の文書・API 記述を legacy/preview に分類し、
棚卸し・移行先・consumer 影響を記録する。物理撤去は行わない。

## Scope

- docs チェーン新設（discovery/spec/design/adr/plan/task）
- Issue #310/#311/#312 系 spec/design/test-spec への legacy banner
- `docs/development-environment.md` の lane 注記
- `crates/ruviform-ui` の README + module doc コメント整理
- 生成 index の再生成

## Upstream Documents

- [spec](../specs/issue-465-legacy-webview-p0p1.md)
- [design](../designs/issue-465-legacy-webview-p0p1.md)
- [ADR](../adr/issue-465-legacy-webview-retirement-status.md)
- [inventory](../discovery/issue-465-legacy-webview-inventory.md)

## Task

- [task](../tasks/implement-issue-465-legacy-webview-p0p1.md)

## Implementation Sequence

1. 棚卸し doc を正本として作成（本計画に先行して実施済み）。
2. spec → design → ADR → plan → task の front matter relations を
   双方向に揃える。
3. 対象 docs に legacy banner を挿入（本文は改訂しない）。
4. `crates/ruviform-ui` の doc コメントに legacy 分類と移行先を明記。
5. `doc_maintenance.js` で index を再生成し audit を実行。
6. `git fetch origin main && git merge origin/main` の後に最終検証。

## Verification

- `mise run fmt-check` / `check` / `lint` / `test`
- `mise run check-ui-driver-boundary` / `test-ui-driver-contract`
- `doc_maintenance.js plan --json` で broken relation なし

## Rollback

文書・コメントのみの変更のため、コミット単位で revert 可能。
コード挙動への影響はない。
