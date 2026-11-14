---
id: "TASK-034XB27D04PGJexpQv3Eee"
type: "task"
status: "done"
title: "Issue #389 CLAP/L0 実外部 Editor attach タスク（adapter open_gui/close_gui + PoC clap lane）"
created: "2026-11-14"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-389-clap-l0-editor-attach.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-389-clap-l0-editor-attach.md"
    - "docs/test-specs/issue-389-clap-l0-editor-attach.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-clap-l0-editor-attach.md"
    - "docs/tasks/issue-389-vst3-l0-editor-attach.md"
    - "docs/tasks/issue-388-389-native-dock-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-389-clap-l0-editor-attach.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-clap-l0-editor-attach.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 CLAP/L0 実外部 Editor attach タスク（adapter open_gui/close_gui + PoC clap lane）

正本: `docs/plans/implement-issue-389-clap-l0-editor-attach.md`。

## チェックリスト

- [x] T-1 `crates/ruviform-plugin-clap/Cargo.toml` — `host`
  feature 配下に `clack-extensions/gui` を追加。
- [x] T-2 `crates/ruviform-plugin-clap/src/host.rs` —
  `ClapPlugin` に `gui`/`gui_seq` フィールド + `Drop` 先頭
  GUI teardown（`hide` best-effort + `destroy`）+ `open_gui`/
  `close_gui` override（design D1–D6）+ module docs 更新。
- [x] T-3 adapter pin test 更新 — CA-01..CA-05（dormant
  `open_gui`→`PluginFailed`、init 済 fixture→`Unsupported`、
  close 未 open→`Unsupported`、既存 descent/DSP 非回帰）。
- [x] T-4 `ruviform-poc-native-dock` `clap` feature —
  `Cargo.toml` optional dep + `poc.rs` 内に
  `RVF_POC_CLAP_PATH`/`RVF_POC_CLAP_PLUGIN_ID` 駆動の実 module
  mode + `vst3_mode`→`external_mode` 改名（probe mode 既定維持）。
- [x] T-5 `mise.toml` manual lane `poc-clap-editor` +
  `docs/development-environment.md` entry。
- [x] T-6 canonical 検証一式（task 固有の正規検証コマンド）+
  manual lane 実行可否の honest 記録（`evidence/rf389-clap-editor/`）。
- [x] T-7 review-protocol + IR/EXP 記録 + docs index 再生成 +
  PR 作成。

## 完了条件

- test-spec の headless 全 case（CA-01..05）green。
- `open_gui`/`close_gui` が CLAP `clap.ext.gui` extension を
  `PluginGui` host API 経由で呼び、`ParentLease.payload()` の
  HWND を `Window::from_win32_hwnd` + `set_parent` で embed する
  実装（`PluginEditorSession`/`ParentSlotLease` 境界経由、
  OS handle 非露出）。
- 実現不能経路（L1/他 OS/floating/size negotiation/keyboard-IME/
  state verbs）は honest Unsupported/NotRun のまま。
- manual lane の結果が PASS/Fail/NotRun/Unsupported の個別分類で
  evidence 化される（実 GUI attach は fixture 非搭載のため
  本 slice では実走証拠まで、attach Pass は後続 slice）。

## Verification

- [x] `mise run fmt-check` / `mise run check
  ruviform-plugin-clap` / `mise run lint ruviform-plugin-clap` /
  `mise run test ruviform-plugin-clap` /
  `mise run test ruviform-plugin-clap --features host`
- [x] `mise run check-l1-real-plugin` / `mise run
  lint-l1-real-plugin` / `mise run test-l1-real-plugin`
- [x] `mise run test ruviform-plugin-host --test editor_session
  --test editor_session_driver`
- [x] `mise run check ruviform-poc-native-dock` /
  `mise run check-ui-driver-boundary` / `mise run test-rt`
- [x] manual lane `mise run poc-clap-editor`（`RVF_POC_CLAP_PATH`
  必須、real-OS-only — 実 binary があれば実行・証拠化、なければ
  NotRun 記録）
