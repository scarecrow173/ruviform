---
id: "DISC-rf855popupKeyToggle"
type: "discovery"
status: "resolved"
title: "Issue #855 main ci lane popup_key_toggles_plugin_owned_window fail — 原因分類（非 Windows 決定的 defect）と修正方針"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/855"
    - "crates/ruviform-plugin-test-vst3/src/editor_view.rs"
    - "docs/test-specs/issue-630-test-fault.md"
    - "evidence/rf855/verification.md"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-855-popup-key-ci.md"
      - type: "dir"
        path: "evidence/rf855/"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/editor_view.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/test-specs/issue-630-test-fault.md"
    - "docs/discovery/issue-841-spawn-deadline-flake.md"
    - "evidence/rf855/verification.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #855 `popup_key_toggles_plugin_owned_window` CI fail — 原因分類・機構・修正方針

## Summary

`main` の `ci` lane で唯一 fail した
`editor_view::tests::popup_key_toggles_plugin_owned_window` の原因は、
**flake/環境ではなく非 Windows 決定的 defect（実 regression）** と分類する。

`popup_toggle`（`editor_view.rs`）の open/close 判定 oracle が
`state.popup`（HWND 実体の `Option<usize>`）を見ていた。非 Windows の
`win32::popup` stub は設計どおり常に `None` を返すため `state.popup` が
永久に `None` のままとなり、全 `VK_P` 入力が open 分岐に入って
`po:{seq}` のみが記録され、close 分岐と `pc:{seq}` 記録は dead code
だった。`po:`/`pc:` token を全 OS で event log に記録する（verb dispatch
の記録は HWND 生成可否と独立）という fixture 契約に対して、open 判定が
HWND 実体に依存していたことが直接の原因。

修正方針: `ViewState` に論理 open flag（`popup_open: bool`）を持たせ、
`popup_toggle` の分岐 oracle を HWND `Option` から分離する。
`state.popup` は HWND handle 専用のまま維持し、Windows の実 HWND 挙動は
不変（Windows では flag と `popup.is_some()` が常に一致する）。

## 失敗観測（ci run 44854804270・SHA `dd2df31a`）

- 唯一の FAILED test:
  `editor_view::tests::popup_key_toggles_plugin_owned_window`
- assertion: `state.events.contains(&"pc:1")`（`editor_view.rs:798`）
- 失敗 legs: verify-macos / verify(ubuntu) / l1-real-verify-macos /
  l1-real-verify(ubuntu) — **非 Windows 4 legs で同一 assertion に一致**
- Windows legs は全て success — OS 決定的な失敗であり flake signature
  （再実行で通る・環境 stall 相関）ではない

## 「daily-ci 同 SHA pass」は反証にならない

daily-ci run 44854775125 の job は `decide` / `decide-consumer-verify`
の 2 本のみ。`daily-ci.yml` は workflow_dispatch 判定だけを行う
scheduler で verify leg を 1 本も実行しない。実検証は dispatched 先の
ci run 44854804270 であり、それは失敗した唯一の実走である。したがって
「同 SHA が daily-ci で green」は本 defect の反証にならない。

## 導入元

commit `cbdf6d56`（PR #832 merge `7a4a6cd1`・Issue #630 の popup 拡張）
が `popup_toggle`・`state.popup`・本 test を追加した（+177 行）。
それを含む初の ci 実走が `dd2df31a` の run — 先行 green ci
（`cd145c4f` 等）は全て PR #832 merge より前であり、本 defect を含む
SHA の ci 実走は全て fail している。main `5f22bee6`（本 slice 着手時の
remote main 直系祖先）にも `editor_view.rs` の最終変更は `cbdf6d56` の
まま残存していた。

## 機構（実コード確認）

- `popup_toggle`（修正前 `editor_view.rs:168-186`）の分岐条件は
  `if let Some(popup) = state.popup.take()` — close は HWND 実体の
  存在を条件とする。
- 非 Windows `win32::popup` stub（`:677-679`）は常に `None` を返す
  （"no surface" の正直な stub であり設計意図どおり）。
- したがって非 Windows では `state.popup` が常に `None` → 全 `VK_P` が
  open 分岐 → `po:1`/`po:2`/`po:3` のみ記録、`pc:` は記録されない
  （close 分岐 dead）→ test :798 の `pc:1` assertion が決定的に fail。
- 設計意図との矛盾: 同:160-167 の実装コメントが "the verb dispatch is
  recorded whether or not this build can create HWNDs" と明記する契約
  （`po:`/`pc:` を全 OS で記録）に対し、open 判定 oracle が HWND 実体
  `Option<usize>` を使っていた。
- Windows でも `win32::popup` が `None` を返す経路（`CreateWindowExW`
  失敗・class 未登録）では同じ dead-close が起き得た — 非 Windows で
  決定的に発火した潜在 defect。

## 修正内容（AC2）

- `ViewState.popup_open: bool`（初期値 `false`）を追加。
  `popup_toggle` の open/close oracle を論理 flag に分離し、
  `state.popup` は HWND handle 専用 `Option<usize>` のまま維持
  （test :789 `assert_eq!(state.popup.is_some(), cfg!(windows))` の
  既存契約は不変）。
- close 分岐: flag を折り、`state.popup.take()` が `Some` の実 HWND
  のみ `win32::destroy` し `pc:{seq}` を記録。open 分岐:
  `popup_seq` を進め `win32::popup` を試み（`None` でも）`po:{seq}`
  を記録し flag を立てる。
- `removed()`: 実 HWND があれば従来どおり destroy したうえで flag を
  リセット — detach/再 attach に stale な close cycle を残さない。
- Windows 等価性: `win32::popup` 成功時は flag と
  `popup.is_some()` が常に一致し、実 HWND の create/destroy・
  `set_title` restamp・`popup_seq` 進行・`try_borrow_mut` の
  re-entrant rejection（`kResultFalse`）は全て不変。
- 追加 unit test 2 本（同 file `#[cfg(test)] mod tests`）:
  `popup_verbs_are_recorded_without_popup_hwnd`（no-HWND 経路で
  `po:1,pc:1,po:2,pc:2` の交互記録を要求 — 修正前コードで
  `["po:1","po:2","pc:2","po:3"]` となり Windows 上でも再現 fail
  することを実測）、`removed_resets_popup_open_state`（`removed`
  後の再 open が `po:` に戻る flag reset 契約の固定）。
- 既存 test `popup_key_toggles_plugin_owned_window`（:775-807）の
  assertion は oracle として不変（弱体化・意味変更・`#[ignore]` なし）。

## 検証（`evidence/rf855/verification.md` 詳細）

- 修正前: 新 test が Windows ローカルで `["po:1","po:2","pc:2","po:3"]`
  を返し fail — no-HWND 経路の dead-close を再現実測。
- 修正後: `mise run test ruviform-plugin-test-vst3` PASS（109 件・
  `popup_key_toggles_plugin_owned_window` を含む Windows 非回帰の
  直接証拠）、`--features l1-runner --lib --bins` PASS、
  `check`/`lint`（`-D warnings`）/`fmt-check` PASS、
  `check-l1-real-plugin`/`lint-l1-real-plugin` PASS、
  `prek run --all-files` PASS。
- 補助証拠（canonical = mise）: `cargo check --target
  x86_64-unknown-linux-gnu -p ruviform-plugin-test-vst3 --all-targets`
  PASS — 非 Windows stub・`cfg(test)` module の cross compile 確認。
- 非 Windows lane の実証は merge 後 CI に委譲（ローカルは Windows の
  み・`ci.yml` は workflow_dispatch 専用で push/PR では起動しない
  設計）。

## Residual

- **AC3（merge 後の新 main SHA の `ci` lane green 実証）** — 本 slice
  外の follow-on gate。Coordinator が新 main tip の ci run を確認する
  （daily-ci scheduler が失敗 SHA 履歴から自動再 dispatch、または
  `gh workflow run ci.yml -R scarecrow173/ruviform -f
  expected_sha=<新 main tip>` で早期化 — workflow 変更・dispatch
  実施は Worker の範囲外）。
- docs index（`docs/**/README.md`）は Coordinator が統合時に整備。
