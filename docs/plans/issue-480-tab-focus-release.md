---
id: "PLAN-034UiTo1gkl570hZTb2iU7"
type: "plan"
status: "approved"
title: "Issue #480 実装計画 — tab_focus の op 駆動解放"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-480-tab-focus-release.md"
    - "docs/designs/issue-480-tab-focus-release.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-480-tab-focus-release.md"
    - "docs/designs/issue-480-tab-focus-release.md"
  implemented-by:
    - "docs/tasks/issue-480-tab-focus-release.md"
  depends-on:
    - "docs/designs/issue-480-tab-focus-release.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-480-tab-focus-release.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-480-tab-focus-release.md"
    - "docs/designs/issue-480-tab-focus-release.md"
  derived-by:
    - "docs/tasks/issue-480-tab-focus-release.md"
  verifies:
    - "docs/test-specs/issue-480-tab-focus-release.md"
  verified-by:
    - "docs/test-specs/issue-480-tab-focus-release.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/480"
  defers: []
  deferred-by: []
---

# Issue #480 実装計画 — tab_focus の op 駆動解放 Implementation Plan

## Goal

`RvfView::flush_events` で `Removed`/`Unmounted` op を受けたとき
`tab_focus` の対応 entry を解放し、strip の FocusHandle が spec の
存続期間を超えて残らないようにする。

## Scope

単一実装ストリーム。`crates/ruviform-ui-gpui/src/render.rs` のみを変更し、
契約層（`ruviform-ui`）・`ui/`・`ruviform-ui-test` には触れない。

## Upstream Documents

- `docs/specs/issue-480-tab-focus-release.md`

## Design Inputs

- `docs/designs/issue-480-tab-focus-release.md`（approved）

## File Map

- `crates/ruviform-ui-gpui/src/render.rs` — `flush_events` に op 処理の
  match arm を追加、`ProjectionOp` を import、`tab_focus` の doc
  comment 更新、gpui-test に恒久テスト追加。

## Implementation Sequence

- [x] P1: gpui-test に解放検証テストを追加（`Removed`・kind-swap・
      `Unmounted`・sibling 保持・`Updated` 保持）— RED を確認
      （3 テスト全て release assertion で失敗）。
- [x] P2: `flush_events` に `ViewEvent::Op(ProjectionOp::Removed{key} |
      Unmounted{key}) → tab_focus.remove(&key)` を実装 — GREEN を確認。
- [x] P3: `origin/main` 取込・全 lane 検証・review-protocol・commit →
      push → PR。

## Dependencies and Risks

- `ProjectionOp`/`ViewEvent` は `#[non_exhaustive]` — match は wildcard
  arm を維持する。
- op が flush まで滞留する経路は production に存在しない
  （全 mutation path が同期 flush）。test が `host.update` を直接呼ぶ
  場合のみ手動 flush が必要 — 設計どおり。

## Task Breakdown

- `docs/tasks/issue-480-tab-focus-release.md`（単一タスク）。

## Verification

- [x] `cargo test -p ruviform-ui-gpui --features gpui-test -- focus_handle`
      （補助証拠）: RED 3 fail → GREEN 3 pass。
- [x] `mise run test-ui-gpui` / `lint-ui-gpui` /
      `check-ui-driver-boundary` / `check` / `lint` / `fmt-check` green。
      （`origin/main` 取込後に実施。全て exit 0。追加で `mise run test`
      workspace 全体も green。）

## Open Questions / Gaps

- `control_focus`（#479 FU-c）の解放は本 Issue の範囲外。design の
  Deferred Design Concerns に記録済み。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker dispatch）。
- Independent work streams: none。
- User approval required before dispatch: no（是正タスク、受入条件確定済み）。

## Review Handoff

- reviewer focus: `flush_events` の match が `#[non_exhaustive]` enum を
  正しく扱うか、`Updated`/`Mounted`/`Gesture`/`Unsupported` の drop が
  従来どおりか、sibling のフォーカス保持がテストで固定されているか。
