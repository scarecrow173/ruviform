---
id: "SPEC-WXP3Ux78gYSf2Aj16emS6p"
type: "spec"
status: "approved"
title: "Issue #465 WebView 移行台帳の現行化 — 現行 main 基準の分類検証（P2 物理撤去は対象外）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/audio-lens/issues/189"
    - "https://github.com/scarecrow173/synvelis/issues/232"
  changes:
    added:
      - type: "file"
        path: "evidence/rf465/inventory.json"
      - type: "file"
        path: "evidence/rf465/README.md"
      - type: "doc"
        path: "docs/designs/wv-removal-inventory-design.md"
      - type: "doc"
        path: "docs/plans/wv-removal-inventory-plan.md"
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
    generated: []
  implements: []
  implemented-by:
    - "../designs/wv-removal-inventory-design.md"
    - "../plans/wv-removal-inventory-plan.md"
  depends-on:
    - "../discovery/issue-465-legacy-webview-inventory.md"
    - "../specs/issue-465-legacy-webview-p0p1.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/issue-465-legacy-webview-retirement-status.md"
    - "../specs/issue-465-legacy-webview-p0p1.md"
    - "../designs/issue-465-legacy-webview-p0p1.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/issue-465-legacy-webview-inventory.md"
  derived-by:
    - "../designs/wv-removal-inventory-design.md"
    - "../plans/wv-removal-inventory-plan.md"
    - "../tasks/wv-removal-inventory-task.md"
  verifies: []
  verified-by:
    - "../test-specs/wv-removal-inventory.md"
  references:
    - "evidence/rf465/inventory.json"
    - "evidence/rf465/README.md"
  defers: []
  deferred-by: []
---

# Issue #465 WebView 移行台帳の現行化 — 現行 main 基準の分類検証

## 背景

Issue #465 の P0/P1（PR #475,
[discovery 棚卸し](../discovery/issue-465-legacy-webview-inventory.md)）
は完了報告済みだが、Issue の現行受入条件には「現行 main 基準で
WebView 関連ファイル/依存/機能/Consumer 利用者/削除可否/維持対象/
例外 Issue を台帳化し、`rvf.ipc.v1` や独立再利用資産を守る」
「Ruviform/AL/SV に残る WebView 固有 bin/deps/feature/CI/asset と、
共通 IPC・headless/CLI・独立 Viz を分類し破壊的誤削除を防止する」
「例外的な Legacy 維持の OS/機能と Owner/退役 Trigger を記録し、
撤去が完了していない範囲を全面完了扱いしない」という有限残件が
残っている。

P0/P1 以降（2026-11-10 → 2026-11-14）に main は大きく変化した:
Issue #464 Consumer-neutral Scenario Contract、#313 windowed
TestIo/`rvf.uitest.v1` GPUI backend、#314 Pixel Golden/visual
capture、#243 multiwindow、#552 SendInput preflight、#598
`ui-gpui-verify-macos`、#418 theme codegen、#429/#436/#442 consumer
observation、mise package-scoped/`*-full` 再構成、ci.yml の
workflow_dispatch 専用化など。consumer 側も pin 更新
（AL `54439fdc`→`fd1d4d5f`、SV `79d1db0a`→`fd1d4d5f`）と SV webview
bin の `run_multi` 系 API 移行、AL `al-desktop` bin 新設があった。
P0/P1 台帳はこの現状を反映していない。

## 目的

- 現行 main（ruviform `fe69f818`、AL `0e2bfee5`、SV `8119d3e8`）を
  対象に WebView 関連資産を再棚卸しし、**削除対象 /
  再利用対象 / 暫定維持 / 例外** の 4 分類で台帳化する。
- P0/P1 棚卸しとの差分（追加・消失・分類変更）を証拠付きで明記し、
  台帳の現行化を示す。
- AL/SV の WebView 参照を列挙して台帳に記録する（consumer repo の
  変更は行わない）。
- `rvf.ipc.v1`、Semantic/headless Scenario、独立可視化資産
  （`crates/ruviform-viz`）が「維持/再利用」側に正しく分類され、
  誤削除を防ぐことを検証する。
- `crates/ruviform-ui-webview` への変更は deprecation 注記・台帳
  根拠のドキュメントコメントに限定する（API 削除・rename・挙動
  変更は行わない）。
- 「例外・暫定維持」が残る範囲を全面完了扱いせず、Owner/退役
  Trigger を明記する。

## 受入条件

| # | 条件 | 結果の記録先 |
| --- | --- | --- |
| A1 | 現行 main 基準で WebView 関連ファイル/依存/機能/Consumer 利用者/削除可否/維持対象/例外 Issue が台帳化され、`rvf.ipc.v1`・独立再利用資産が保護されている | `evidence/rf465/inventory.json` + `evidence/rf465/README.md` |
| A2 | Ruviform/AL/SV の WebView 固有 bin/deps/feature/CI/asset と共通 IPC・headless/CLI・独立 Viz が 4 分類で区別され、破壊的誤削除を防ぐ分類になっている | 同上（entries の classification/notes） |
| A3 | P0/P1 棚卸しとの差分（追加・消失・分類変更）が明記されている | `evidence/rf465/README.md` 差分節 + 各 entry の `diff_vs_p0p1` |
| A4 | 例外的な Legacy 維持（歴史証拠等）と暫定維持対象の Owner/退役 Trigger が記録され、撤去未完了範囲が全面完了扱いされていない | inventory.json `owner`/`retirement_trigger`/`gates` + 本 spec |
| A5 | `crates/ruviform-ui-webview` の変更が doc コメント/README 注記のみで、API・挙動を変更していない | `git diff` + check/lint/test 結果 |
| A6 | P2 物理撤去（ファイル/依存/CI lane の削除）を一切実施していない | `git diff --stat`（削除なし）+ PR 本文 |

## 非目標

- P2 物理撤去: `crates/ruviform-ui-webview`・`ui/*`・`poc/`・旧 API
  面・Cargo member・CI lane・consumer 側資産の削除/除外（#419 +
  AL #189/SV #232 受入待ちの別判断）。
- consumer repo（audio-lens・synvelis）の編集・Issue 起票
  （読み取り専用の列挙のみ）。
- `rvf.ipc.v1` schema・driver-neutral 契約・Semantic Scenario・
  `ruviform-viz` 等の再利用資産への変更。
- 新規機能・dependency・feature・unsafe の追加。

## 制約

- 編集範囲: `crates/ruviform-ui-webview/`（deprecation 注記・台帳
  根拠の注釈のみ）、`evidence/rf465/`、本 doc chain
  （`docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/wv-removal-inventory*`）、
  および生成 docs index の再生成。
- 検証は `mise.toml` タスクが正本。crate doc 注記を入れたため
  `fmt-check` と `check`/`lint`/`test ruviform-ui-webview`（+ feature
  lane）を実行する。docs/監査のみの他部分は Markdown/リンクの
  軽量チェックが原則。
- F: ドライブに中間生成物を置かない。consumer 列挙は read-only。
