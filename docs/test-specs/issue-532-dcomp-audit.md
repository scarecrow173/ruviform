---
id: "TSPEC-034WZEsNYIpYAIB3snCErj"
type: "test-spec"
status: "approved"
title: "Issue #532 DComp 監査文書チェーン検証"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-audit.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/issue-532-dcomp-audit-plan.md"
    - "docs/specs/issue-532-dcomp-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-dcomp-native-dock.md"
    - "docs/specs/issue-532-dcomp-audit.md"
    - "docs/designs/issue-532-dcomp-audit-design.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/tasks/issue-532-dcomp-audit-task.md"
    - "docs/impl/ir/issue-532-dcomp-audit.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-532-dcomp-audit-design.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-532-dcomp-audit.md"
    - "docs/designs/issue-532-dcomp-audit-design.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
---

# Issue #532 DComp 監査文書チェーン検証

## Purpose

Issue #532 の文書監査スライスが、正本監査記録・実機調査設計・文書チェーンを正しく作成し、L1 証拠の CI 対応付け・再開条件・方式 A/B/C 手順・Blocked by 除外提案を固定したことを検証する。

## Feature

- `docs/discovery/issue-532-dcomp-native-dock.md` — 正本監査記録。
- `docs/designs/issue-532-dcomp-native-dock.md` — 実機調査設計。
- 文書チェーン（spec/design/plan/task/test-spec/impl-ir/impl-exp）。

## Rules

- R-DSC-01: 正本監査記録は F1（制約）・F2（L1 被覆棚卸し）・F3（CI lane 対応付け）・F4（依存状態）・再開条件・方式 A/B/C 手順・Blocked by 除外提案を含む。
- R-DSC-02: 実機調査設計は L2/L3 再開条件・Step 1〜4 手順・方式 A/B/C 採否条件・契約維持条件を含む。
- R-DSC-03: 文書チェーンの front matter relations が相互に解決可能（spec→design→plan→task→test-spec→impl-ir/impl-exp、discovery を derives-from の起点）。
- R-DSC-04: 触った `docs/<type>/` dir の index が `doc-maintenance` で再生成される（impl/ir は `index apply` で再生成、impl/exp の .md 一覧は手動追記）。
- R-DSC-05: `mise run fmt-check` + `mise exec -- prek run --files` が exit 0。
- R-DSC-06: 実 Native Dock 未成立が「未確証」として明記され、Pass/Unsupported への偽装がない。
- R-DSC-07: fake/headless の成功を実 plugin 画素・実 Native Dock の証拠に読み替えない。

## Examples

- EX-DSC-01: `docs/discovery/issue-532-dcomp-native-dock.md` の F1 が DComp topmost 制約を「観測済み・未解消」として記録し、HWND 機構 Pass と画素未達を分離している。
- EX-DSC-02: F3 が既存 CI lane（`ui-gpui-verify` 3 OS・`ui-gpui-offscreen-verify`・`msrv-verify`・`rt-verify`・`l1-real-verify`）と run `43428172820` の証拠を対応付け、同一 SHA の新規 run 未取得を明記している。
- EX-DSC-03: 再開条件表が対象 OS・GPU/driver・GPUI version・feature tuple・実 Plugin・Window/DPI・測定環境・証拠・Owner を含む。
- EX-DSC-04: 方式 A/B/C の採否条件が Issue 本文の「終了条件」「比較・選択」に基づき、Floating/overlay/Generic を Dock PASS としない旨を明記している。

## Guarantees

- 本 slice は文書のみの変更であり、`crates/`・CI workflow・Cargo manifest/lockfile を変更しない。
- 実機 L2/L3 調査・方式採否・実装は対象外（Deferred）。
- GitHub blocked-by の正式な登録・削除は行わない（Coordinator 責務）。

## Non-goals

- 実画面・実 plugin・実 GUI の検証。
- 方式 A/B/C の技術的採否判定。
- Issue #532 の Close。

## Risk

- SHA 不一致（CI 証拠 `e93c438a` vs 監査 HEAD `8944a360`）の誤読 → `git diff` で契約層無変更を確認済み・discovery に明記。
- fake/headless の過大評価 → 各所で「実 Dock PASS ではない」「未確証」を明記。
