---
id: "TSPEC-Wm6ujGxaqkur53e1zm0Xak"
type: "test-spec"
status: "approved"
title: "Issue #465 WebView 移行台帳現行化 — 分類検証契約"
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
        path: "docs/tasks/wv-removal-inventory-task.md"
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
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/wv-removal-inventory.md"
    - "docs/designs/wv-removal-inventory-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/wv-removal-inventory.md"
    - "docs/designs/wv-removal-inventory-design.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/wv-removal-inventory.md"
  verified-by: []
  references:
    - "evidence/rf465/inventory.json"
  defers: []
  deferred-by: []
---

# Issue #465 WebView 移行台帳現行化 — 分類検証契約

本スライスは監査・台帳化・doc 注記のみで Rust の挙動変更を伴わない
ため、新規テストコードは追加しない。以下は「分類が正しいこと」を
保証する検証契約であり、機械検証可能な確認項目として記録する。

## 検証対象の受入条件 → 検証方法

| 受入条件 | 検証 |
| --- | --- |
| A1 台帳化 + 保護資産 | `evidence/rf465/inventory.json` が valid JSON で `entries[]` を持ち、`rvf.ipc.v1`（RV-023）・Scenario（RV-028）・`ruviform-viz`（RV-031）・`ui/guitest/fixtures`（RV-016）・headless/CLI（RV-032/RV-037）が `reuse` に分類されていることを機械集計で確認 |
| A2 4 分類の完全性 | 全 entry が `classification ∈ {remove_candidate, reuse, provisional, exception}` を持ち、`owner`/`retirement_trigger` が provisional/exception entry に記録されていること |
| A3 P0/P1 差分 | README 差分節の「追加・消失・分類変更・consumer 差分」が `git log 4e59494a..HEAD` および consumer checkout の実測と一致すること |
| A4 未完了範囲の非完了扱い | `gates.p2_physical_removal = NotRun`、AL#189/SV#232/#419 が OPEN であること、全面完了を示す記述がないこと |
| A5 doc 注記のみ | `git diff crates/ruviform-ui-webview/` が doc コメント/README のみで、API・feature・example・コード行を変更していないこと |
| A6 物理撤去なし | `git status`/`git diff` に削除・rename・Cargo.toml/mise.toml/CI workflow の変更がないこと |

## 実行した検証

| 検証 | 方法 | 結果 |
| --- | --- | --- |
| JSON 構文・集計 | `node -e "JSON.parse + classification 集計"` | pass（52 entries、分類内訳一致） |
| consumer Issue 状態 | `gh issue view 189/232/419/262` | すべて OPEN（P2 gate 未達を確認） |
| P0/P1 差分 | `git log --oneline 4e59494a..HEAD -- <ui paths>` | webview crate 変更なし、ui/binding・guitest に契約修正 3 件、driver-neutral 側に大規模追加 |
| crate doc 注記 | `mise run check/lint/test ruviform-ui-webview` + `check/lint/test-ui-webview` | 実施結果は IR/PR 本文に記録 |
| doc 整合 | `doc_maintenance.js` audit | 実施結果は IR/PR 本文に記録 |
| doc 注記の lint 非破壊 | `cargo check/clippy/test`（feature 有無） | doc コメントのみのため API 契約に影響しないことを確認 |

## 非対象

- Rust テスト追加・変更（挙動変更なし）。
- `*-full`・`*-msrv`・3OS 横断・実機 GUI/Plugin 検証（本スライスは
  docs/監査 + doc コメントのみ。Daily Full CI・手動 Full CI に委譲）。
- consumer repo の検証実行（read-only 列挙のみ）。
