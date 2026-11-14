---
id: "PLAN-034XB0UGHRPYuncbgF7bB6"
type: "plan"
status: "approved"
title: "Issue #389 CLAP/L0 実外部 Editor attach の実装計画（adapter open_gui/close_gui + PoC clap lane）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-389-clap-l0-editor-attach.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc/dock.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc/dock_windowing.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "docs/designs/issue-389-clap-l0-editor-attach.md"
      - type: "file"
        path: "docs/test-specs/issue-389-clap-l0-editor-attach.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/specs/issue-389-plugin-editor-session.md"
  implemented-by:
    - "docs/tasks/issue-389-clap-l0-editor-attach.md"
  depends-on:
    - "docs/designs/issue-389-clap-l0-editor-attach.md"
    - "docs/test-specs/issue-389-clap-l0-editor-attach.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-389-vst3-l0-editor-attach.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-389-clap-l0-editor-attach.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-clap-l0-editor-attach.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 CLAP/L0 実外部 Editor attach の実装計画（adapter open_gui/close_gui + PoC clap lane）

## Goal

`ClapPlugin::open_gui`/`close_gui` を clack-extensions 0.2.0
`gui` module 経由で実装し（design D1–D6）、PoC `clap`
feature lane（`RVF_POC_CLAP_PATH`）と mise task
`poc-clap-editor` を追加して、実 `.clap` binary に対する
L0 embedded attach の manual lane を用意する。

## Scope

単一 stream。adapter 実装（crates/ruviform-plugin-clap）→
PoC lane（crates/ruviform-poc-native-dock + mise.toml +
development-environment.md）→ docs index/証拠記録の順で
1 plan・1 task doc として進める。VST3 L0 slice
（merged）と同一構造・同一順序。

## Upstream Documents

- `docs/specs/issue-389-plugin-editor-session.md`（実装対象 spec）
- `docs/designs/issue-389-clap-l0-editor-attach.md`（本 slice 設計、approved）
- `docs/test-specs/issue-389-clap-l0-editor-attach.md`（本 slice 検証仕様）
- `docs/designs/issue-389-vst3-l0-editor-attach.md`（踏襲対象の確定設計）

## Design Inputs

- `docs/designs/issue-389-clap-l0-editor-attach.md`（D1–D7 全て）

## File Map

- `crates/ruviform-plugin-clap/Cargo.toml` — `clack-extensions` に
  `clack-extensions/gui` feature を `host` feature 配下で有効化
- `crates/ruviform-plugin-clap/src/host.rs` — `ClapPlugin` に
  `gui`/`gui_seq` フィールド追加、`open_gui`/`close_gui` override、
  Drop へ GUI teardown（D3）、platform gate（D4）、module docs 更新、
  CA-01..05 pin test
- `crates/ruviform-poc-native-dock/Cargo.toml` — opt-in `clap`
  feature + optional dep `ruviform-plugin-clap`（host feature）
- `crates/ruviform-poc-native-dock/src/poc.rs` — `#[cfg(feature =
  "clap")]` arm で `ClapLibrary::load` + `RVF_POC_CLAP_PATH`/
  `RVF_POC_CLAP_PLUGIN_ID` lane、`vst3_mode` → `external_mode` 改名
- `crates/ruviform-poc-native-dock/src/poc/dock.rs`・`dock_windowing.rs`
  — `external_mode` 改名の追随
- `mise.toml` — `poc-clap-editor` task（`--features gpui,clap`、
  real-OS-only manual lane）
- `docs/development-environment.md` — lane entry 記載
- `docs/*/README.md`（designs/plans/test-specs）— index 再生成
- `evidence/rf389-clap-editor/` — 検証記録・manual lane 記録
- `docs/impl/ir/issue-389-clap-l0-editor-attach-implementation.md`・
  `docs/impl/exp/issue-389-clap-l0-editor-attach-*.jsonl` — 実装記録

## Implementation Sequence

- [ ] 1. Cargo.toml（plugin-clap）: `host` feature に
  `clack-extensions/gui` を追加し、`cargo check --features host`
  で依存解決を確認（Cargo.lock no-op 想定）
- [ ] 2. host.rs: module docs・import（`clack_extensions::gui::*`、
  `GuiApiType`、`GuiConfiguration`、`PluginGui`、`Window`、
  cfg 分岐 `EDITOR_GUI_API`）を追加
- [ ] 3. host.rs: `ClapPlugin` struct に `gui: Option<PluginGui>`・
  `gui_seq: u64` を追加し `Drop` で GUI teardown → instance
  teardown の順序（D3）を実装
- [ ] 4. host.rs: `open_gui`/`close_gui` を実装（D1/D6）+ pin
  test CA-01..05（既存 `unsupported_state_and_gui_verbs_are_defaults`
  を置換）
- [ ] 5. `mise run fmt-check` / `check` / `lint` / `test
  ruviform-plugin-clap --features host` を実行
- [ ] 6. PoC: Cargo.toml feature + poc.rs arm + `external_mode`
  改名（dock.rs/dock_windowing.rs 追随）
- [ ] 7. mise.toml `poc-clap-editor` task + development-environment.md
  entry
- [ ] 8. canonical 検証一式（task 固有の正規検証コマンド全て）
- [ ] 9. `git fetch origin main && git merge origin/main` →
  index/lock 再生成 → evidence/impl docs 記録
- [ ] 10. review-protocol → commit → push → PR 作成 → worker_done

## Dependencies and Risks

- clack-extensions 0.2.0 `gui` module の host API は registry
  source で確認済み（`PluginGui`、`GuiConfiguration{api_type,
  is_floating}`、`Window::from_win32_hwnd`、unsafe `set_parent`）
- `clack-host` feature 依存: `gui` host method は
  `clack-extensions/clack-host`（`host` feature で既に有効）
  配下 — feature wiring を誤ると `PluginGui` host method が
  未定義になるため Cargo.toml と host.rs の feature 境界を一致させる
- `unsafe_code` warn→deny: `set_parent`/`from_win32_hwnd` は
  VST3 と同じ `#[expect(unsafe_code)]` + SAFETY コメント
- workspace fixture は `clap.ext.gui` 非搭載 → headless では
  `Unsupported` のみ検証可能。実 attach は manual lane で
  実 binary がある場合のみ（本 slice では NotRun 記録）
- PoC lane の `gpui` feature 依存は task 側指定であり feature
  間依存は張らない（vst3 と同じ構造）

## Task Breakdown

単一 task `docs/tasks/issue-389-clap-l0-editor-attach.md`（TDD:
test-spec CA-01..05 を先に pin してから実装）。

## Verification

- headless: `mise run fmt-check` / `mise run check
  ruviform-plugin-clap` / `mise run lint ruviform-plugin-clap` /
  `mise run test ruviform-plugin-clap` /
  `mise run test ruviform-plugin-clap --features host` /
  `mise run check-l1-real-plugin` / `mise run
  lint-l1-real-plugin` / `mise run test-l1-real-plugin` /
  `mise run test ruviform-plugin-host --test editor_session
  --test editor_session_driver` / `mise run check
  ruviform-poc-native-dock` / `mise run check-ui-driver-boundary` /
  `mise run test-rt`
- manual: `mise run poc-clap-editor`（`RVF_POC_CLAP_PATH` 必須、
  real-OS-only。実 binary があれば evidence 記録、なければ
  NotRun/Fail/Unsupported を honest 記録）
