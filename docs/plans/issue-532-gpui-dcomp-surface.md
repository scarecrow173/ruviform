---
id: "PLAN-034XB2HkzXlDdkYzRd4gWv"
type: "plan"
status: "approved"
title: "Issue #532 GPUI DComp public-surface 監査計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-532-gpui-dcomp-surface.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
  implemented-by:
    - "docs/tasks/issue-532-gpui-dcomp-surface.md"
  depends-on:
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
    - "docs/tasks/issue-532-gpui-dcomp-surface.md"
    - "docs/test-specs/issue-532-gpui-dcomp-surface.md"
    - "docs/impl/ir/issue-532-gpui-dcomp-surface.md"
    - "docs/impl/exp/issue-532-gpui-dcomp-surface.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
  derived-by:
    - "docs/tasks/issue-532-gpui-dcomp-surface.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-gpui-dcomp-surface.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 GPUI DComp public-surface 監査計画 Implementation Plan

## Goal

vendored gpui 0.2.2 の Windows platform 層を読み取り専用で監査し、
方式 A/B/C の技術入力を正本 discovery・証拠ノート・設計文書として
固定する docs-only slice。

## Scope

Single implementation stream。docs-only でありコード変更・実機
検証を含まない。実 OS 依存の検証は本 plan の scope 外で、L2/L3
再開時の外部 gate として残す。

## Upstream Documents

- `docs/specs/issue-532-gpui-dcomp-surface.md`（approved）
- `docs/discovery/issue-532-gpui-dcomp-surface.md`（正本監査記録）

## Design Inputs

- `docs/designs/issue-532-gpui-dcomp-surface.md`（approved）— 監査
  事実の方式 A/B/C 写像と A'（env var）区分。

## File Map

- `docs/discovery/issue-532-gpui-dcomp-surface.md` - 正本監査記録
  （F1-F7・Open Questions 更新）。
- `docs/designs/issue-532-gpui-dcomp-surface.md` - 方式 A/B/C 技術
  入力設計。
- `evidence/rf532/gpui-public-surface-audit.md` - 監査証拠ノート
  （コードパス・行番号・判定根拠）。
- `docs/discovery/issue-532-dcomp-native-dock.md` - Open Questions
  更新対象（status resolved 維持）。
- `docs/designs/issue-532-dcomp-native-dock.md` - upstream 調査
  リスク記述の更新対象。
- `docs/plans|tasks|test-specs|impl/ir|impl/exp/issue-532-gpui-dcomp-surface.md`
  - 文書チェーン。

## Implementation Sequence

- [x] vendored `gpui-0.2.2` source の 7 項目監査（DComp グラフ・env
  var・公開面・patch 面積・Option B 要件・version 情勢・exit-127
  仮説）を実施。
- [x] `evidence/rf532/gpui-public-surface-audit.md` を作成。
- [x] `docs/discovery/issue-532-gpui-dcomp-surface.md` を作成。
- [x] `docs/specs/issue-532-gpui-dcomp-surface.md` を作成・approved。
- [x] `docs/designs/issue-532-gpui-dcomp-surface.md` を作成・
  approved。
- [x] `docs/test-specs/issue-532-gpui-dcomp-surface.md` を作成・
  approved。
- [ ] `docs/plans/issue-532-gpui-dcomp-surface.md`（本書）を確定。
- [ ] `docs/tasks/issue-532-gpui-dcomp-surface.md` を作成。
- [ ] `docs/impl/ir/issue-532-gpui-dcomp-surface.md`・
  `docs/impl/exp/issue-532-gpui-dcomp-surface.md` を作成。
- [ ] `docs/discovery/issue-532-dcomp-native-dock.md` の Open
  Questions を更新。
- [ ] `docs/designs/issue-532-dcomp-native-dock.md` の upstream
  調査リスク記述を更新。
- [ ] `doc-maintenance` で index 再生成 + `doc-status` で relations
  検証。
- [ ] `mise exec -- prek run --files <変更ファイル>`。
- [ ] commit → push → PR。

## Dependencies and Risks

- 監査対象の行番号は本 slice で独立再検証済みだが、gpui 0.2.2 は
  固定 pin であり upstream 変動はない。
- exit-127 の原因は仮説のみ（PENDING）。実 OS 再現は L2/L3 再開時。
- `topmost=false`/env var/`SetWindowRgn` の実画素効果は未検証。

## Task Breakdown

- 監査・証拠ノート・discovery/spec/design/test-spec 作成（完了 —
  `docs/tasks/issue-532-gpui-dcomp-surface.md` の前段）。
- plan/task/impl-ir/impl-exp 作成 + 先行 discovery/design 更新 +
  index 再生成 + validation（本 plan の残り — 同 task doc が追跡）。

## Verification

- [ ] `doc-maintenance` `index apply` + `doc-status` で全 doc の
  relations が解決。
- [ ] `mise exec -- prek run --files <変更ファイル>` が exit 0。
- [ ] 実機依存事項が PENDING/未検証として明記されている。

## Open Questions / Gaps

- exit-127 の原因（H1-H5 仮説、実 OS 再現待ち）。
- `topmost=false` の実画素・z-order 効果（fixture 実測待ち）。
- env var 経路（A'）の実画素での child 合成可否（実機待ち）。
- 実 plugin の offscreen surface 普遍性（先行 OQ のまま）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
  consistent.

## Implementation Handoff

- Recommended mode: inline execution（docs-only slice）。
- Independent work streams: none。
- User approval required before dispatch: yes（review-protocol で
  blocker なしを承認として扱う）。

## Review Handoff

- 監査行番号の正確性・公開面の到達可能性判定・exit-127 仮説の
  過剰断定がないか・未検証事項の Pass 偽装がないかを重点確認。
