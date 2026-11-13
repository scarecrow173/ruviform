---
id: "IMPL-czhYl79NIIqQrrZo0gznNl"
type: "impl"
status: "completed"
title: "Issue #465 WebView 移行台帳現行化 — 実装記録"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/465"
  changes:
    added:
      - type: "file"
        path: "evidence/rf465/inventory.json"
      - type: "file"
        path: "evidence/rf465/README.md"
      - type: "file"
        path: "docs/specs/wv-removal-inventory.md"
      - type: "file"
        path: "docs/designs/wv-removal-inventory-design.md"
      - type: "file"
        path: "docs/plans/wv-removal-inventory-plan.md"
      - type: "file"
        path: "docs/tasks/wv-removal-inventory-task.md"
      - type: "file"
        path: "docs/test-specs/wv-removal-inventory.md"
      - type: "file"
        path: "docs/impl/ir/wv-removal-inventory.md"
      - type: "file"
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
      - type: "file"
        path: "docs/impl/ir/README.md"
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
  implements:
    - "docs/tasks/wv-removal-inventory-task.md"
  implemented-by: []
  depends-on:
    - "docs/plans/wv-removal-inventory-plan.md"
    - "docs/specs/wv-removal-inventory.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/wv-removal-inventory.md"
    - "docs/designs/wv-removal-inventory-design.md"
    - "docs/plans/wv-removal-inventory-plan.md"
    - "docs/tasks/wv-removal-inventory-task.md"
    - "docs/test-specs/wv-removal-inventory.md"
    - "docs/discovery/issue-465-legacy-webview-inventory.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/wv-removal-inventory-plan.md"
  derived-by: []
  verifies:
    - "docs/specs/wv-removal-inventory.md"
  verified-by: []
  references:
    - "evidence/rf465/inventory.json"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/wv-removal-inventory.md"
    rejected: []
---

# Issue #465 WebView 移行台帳現行化 — 実装記録

## Summary

現行 main 基準で WebView 関連資産を再棚卸しし、4 分類
（`remove_candidate` / `reuse` / `provisional` / `exception`）の機械
可読台帳 `evidence/rf465/inventory.json`（52 entries）と根拠 README、
P0/P1 棚卸しとの差分、AL/SV consumer 参照列挙、Owner/退役 Trigger を
確定した。`crates/ruviform-ui-webview` には doc コメント/README の
legacy/preview 注記のみを追加し、API・feature・example・挙動は
変更していない。P2 物理撤去は未実施。

## Context

- Issue: <https://github.com/scarecrow173/ruviform/issues/465>
- task: [../../tasks/wv-removal-inventory-task.md](../../tasks/wv-removal-inventory-task.md)
- plan: [../../plans/wv-removal-inventory-plan.md](../../plans/wv-removal-inventory-plan.md)
- spec: [../../specs/wv-removal-inventory.md](../../specs/wv-removal-inventory.md)
- design: [../../designs/wv-removal-inventory-design.md](../../designs/wv-removal-inventory-design.md)
- test-spec: [../../test-specs/wv-removal-inventory.md](../../test-specs/wv-removal-inventory.md)
- exp: [../exp/wv-removal-inventory.md](../exp/wv-removal-inventory.md)
- 台帳: [../../../evidence/rf465/inventory.json](../../../evidence/rf465/inventory.json)
- 前提: PR #475（merge `4e59494a`）の P0/P1 棚卸し済み。本スライスは
  その後の現行 main 差分を対象とする有限残件。

## Implementation

### Changes

- `evidence/rf465/inventory.json` + `README.md` を新設。entry 単位で
  分類・根拠・Owner/退役 Trigger・`diff_vs_p0p1` を記録。
- doc chain 7 文書を `wv-removal-inventory*` 命名で新設。
- `crates/ruviform-ui-webview/src/lib.rs` に Legacy status 節、
  `README.md` に legacy/preview 注記を追加（doc のみ）。
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md` を
  `doc-maintenance` `index apply` で再生成。

### Decisions During Implementation

- 分類単位を package ではなく「削除で壊れる契約」へ変更:
  `ui/guitest/fixtures`（`reuse`）と `ui/guitest/src`（`provisional`）、
  `poc/` コード（`remove_candidate`）と証拠文書（`exception`）を分割。
  根拠は [exp](../exp/wv-removal-inventory.md) 試行 3。
- consumer 列挙は `gh api search/code` が `incomplete_results` を返し
  信頼できないため、origin/main HEAD の clean checkout を正本とした
  （read-only、変更なし）。根拠は exp 試行 1。
- `#[deprecated]` は付与しない — crate は現行 consumer が使用中であり、
  API 属性の変更は「挙動変更」に近くスコープ外。doc 注記で legacy
  位置づけを明示するに留めた。

## Related Experiments

- [docs/impl/exp/wv-removal-inventory.md](../exp/wv-removal-inventory.md)
  — consumer 列挙手法、P0/P1 差分抽出、分類単位の決定、webview lane
  存続状況の 4 試行を記録（adopted）。

## Validation

- `node` による `inventory.json` 構文・集計検証: pass（52 entries、
  分類内訳 ruviform 10/16/11/2・AL 1/1/5・SV 2/1/3 =
  remove/reuse/provisional/exception）。
- `gh issue view`: AL #189 / SV #232 / RF #419 / #262 すべて OPEN
  （P2 gate 未達を確認）。
- `git log 4e59494a..HEAD`: `crates/ruviform-ui-webview`・`poc/` に
  変更なし。`ui/` は契約修正 3 件のみで package 構造不変。
- `git diff`: crate 変更は doc コメント/README のみ、削除・rename・
  Cargo.toml/mise.toml/CI workflow の変更なし。
- mise 検証（`fmt-check`、`check`/`lint`/`test ruviform-ui-webview`、
  `check-ui-webview`/`lint-ui-webview`/`test-ui-webview`）と doc audit
  （`doc_maintenance.js index plan/apply` + `audit_impl_record.js`）の
  結果は PR 本文「実行した検証」に記録。

## Risks

- `gh api search/code` の `incomplete_results` により consumer 列挙は
  checkout 正本。GitHub code search 復旧後の再列挙で差分が出る可能性は
  残る（台帳に手法を明記済み）。
- `crates/ruviform-ui/tests/ipc_conformance.rs` は schema 部（reuse
  相当）と `rvf://data` 部（legacy）が同居しており、P2 時に検証対象
  単位での分割が必要 — 台帳 notes に記録済み。

## Follow-ups

- P2 物理撤去は #419（GPUI adoption 決定）+ AL #189 / SV #232
  （consumer 退役 owner）の受入待ち。台帳の `remove_candidate` entries
  はその入力として使用する。
- `synvelis-workspace` の `WindowHandle` 依存は RF 旧 API 撤去の
  ブロッカー — provisional entry として Owner/Trigger を記録済み。
- SV webview bin は `run_multi` 系 API へ移行済みだが CI に webview
  lane がなく（mise task のみ）、撤去判断時は lane 整備状況を再確認。
