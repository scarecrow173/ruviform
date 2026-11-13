---
id: "PLAN-6MwO2Q1o5z6bCM6mtvvt36"
type: "plan"
status: "approved"
title: "Issue #465 WebView 移行台帳現行化 実装計画"
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
        path: "docs/tasks/wv-removal-inventory-task.md"
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
    - "../specs/wv-removal-inventory.md"
  implemented-by:
    - "../tasks/wv-removal-inventory-task.md"
  depends-on:
    - "../designs/wv-removal-inventory-design.md"
    - "../specs/wv-removal-inventory.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/wv-removal-inventory.md"
    - "../designs/wv-removal-inventory-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/wv-removal-inventory.md"
    - "../designs/wv-removal-inventory-design.md"
  derived-by:
    - "../tasks/wv-removal-inventory-task.md"
  verifies: []
  verified-by: []
  references:
    - "evidence/rf465/inventory.json"
  defers: []
  deferred-by: []
---

# Issue #465 WebView 移行台帳現行化 実装計画

## Goal

現行 main 基準で WebView 関連資産を再棚卸しし、4 分類の機械可読
台帳・P0/P1 差分・consumer 参照列挙・Owner/退役 Trigger を確定
する。物理撤去は行わない。

## Scope

- `evidence/rf465/` 新設（`inventory.json` + `README.md`）
- doc chain 新設（spec/design/plan/task/test-spec/impl ir/exp）
- `crates/ruviform-ui-webview` への legacy/preview doc 注記
  （`src/lib.rs` crate doc + `README.md`）
- 生成 docs index の再生成

## Upstream Documents

- [spec](../specs/wv-removal-inventory.md)
- [design](../designs/wv-removal-inventory-design.md)
- [P0/P1 棚卸し](../discovery/issue-465-legacy-webview-inventory.md)
- [P0/P1 spec](../specs/issue-465-legacy-webview-p0p1.md)
- [退役位置づけ ADR](../adr/issue-465-legacy-webview-retirement-status.md)

## Task

- [task](../tasks/wv-removal-inventory-task.md)

## Implementation Sequence

1. 現行 main の棚卸し: `rg`/`git log 4e59494a..HEAD`/`Cargo.toml`/
   `mise.toml`/`ci.yml`/`consumer-verify.yml`/`boundary.json` を精査
   し、P0/P1 以降の追加・消失・分類変更を確定。
2. AL/SV consumer 列挙: 両 repo を origin/main HEAD に fetch し
   file-level で WebView 参照を列挙（read-only）。pin SHA・
   feature/bin/lane/asset/vendor 構造を記録。Issue 状態を
   `gh issue view` で確認。
3. `evidence/rf465/inventory.json` を作成 — 4 分類・根拠・
   Owner/Trigger・差分を entry 単位で記録。
4. `evidence/rf465/README.md` を作成 — schema・収集方法・差分
   サマリ・保護対象検証・検証コマンド。
5. doc chain 7 文書を作成（front matter relations を双方向に揃える）。
6. `crates/ruviform-ui-webview` に doc 注記を追加。
7. `doc_maintenance.js` で index 再生成 + audit。
8. `git fetch origin main && git merge origin/main` 後に最終検証。
9. review-protocol でレビュー → commit/push/PR。

## Verification

- `mise run fmt-check`
- `mise run check ruviform-ui-webview` / `lint ruviform-ui-webview` /
  `test ruviform-ui-webview`
- `mise run check-ui-webview` / `lint-ui-webview` / `test-ui-webview`
- `doc_maintenance.js` audit（broken relation なし）
- `inventory.json` 構文・集計の機械検証
- `git diff` で削除・rename・API 変更がないことを確認

## Rollback

文書・コメント・evidence のみの変更のため、コミット単位で revert
可能。コード挙動への影響はない。
