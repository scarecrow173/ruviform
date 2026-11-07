---
id: "PLAN-5rBdUXHzqmuHrA1h4sweAj"
type: "plan"
status: "approved"
title: "Issue #389 VST3/L0 実外部 Editor attach の実装計画（adapter open_gui + PoC vst3 lane）"
created: "2026-11-07"
updated: "2026-11-07"
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
    - "docs/specs/issue-389-plugin-editor-session.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-389-vst3-l0-editor-attach.md"
    - "docs/tasks/issue-388-389-native-dock-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-vst3-l0-editor-attach.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 VST3/L0 実外部 Editor attach 実装計画

## 目的

`docs/designs/issue-389-vst3-l0-editor-attach.md`（D1–D7）を実装し、
**既存 VST3 adapter が実外部 module の native editor を
open/close・attach/detach して EditorSession/Slot lease 境界へ
到達できる**ことを、headless fixture 契約（決定的）と
Windows 実機 manual lane（実 Vinyl.vst3 binary）で検証する。

## 依存・前提

- PR399 統合済み: `PluginEditorSession`/`InProcessEditorOps`/
  `ParentSlotLease`/`GpuiSurfaceAdapter`/`ruviform-poc-native-dock`
  probe PoC。これらの契約・実装は再利用し、重複実装しない。
- `vst3 = "0.3.0"` binding crate 既存依存のみ使用（新規依存は
  `ruviform-poc-native-dock` → `ruviform-plugin-vst3` の
  optional path dep のみ。windows-sys は poc 側既存）。
- 対象 binary: `C:\Program Files\Common Files\VST3\iZotope\
  Vinyl.vst3`（既インストール品・勝手に installer/SDK を入れない）。
- Slot/GPUI adapter/Session 契約は他 owner — 本計画では変更しない。

## 段階

| Stage | 内容 | 検証 |
| --- | --- | --- |
| T-1 | `host.rs`: `BridgePlugFrame` COM host object（`Class`/`ComWrapper` パターン）+ `Vst3Plugin` に `view`/`plug_frame`/`gui_seq` フィールド + `close_view` helper + `Drop` 先頭で view teardown + `open_gui`/`close_gui` override（D1–D4・D6） | `cargo test -p ruviform-plugin-vst3 --features host`（fixture 契約 + 新 RED→GREEN） |
| T-2 | adapter unit test 追加/更新: dormant open_gui→`PluginFailed`、init 済 fixture open_gui→`Unsupported`（null view）、close 未 open→`Unsupported`、close→`Ok` の既存 fixture 経路維持 | 同上 |
| T-3 | `ruviform-poc-native-dock`: `vst3` feature + `poc.rs` 内 `register_plugin`（`Vst3Library::load`→register→instantiate→session open→`EnumChildWindows` 証跡→geometry/hide/focus/close/release）+ `RVF_POC_VST3_PATH` env dispatch（probe mode は既定維持） | `cargo check/clippy -p ruviform-poc-native-dock --features vst3` |
| T-4 | `mise.toml` に manual lane task `poc-vst3-editor`（`CARGO_BUILD_JOBS=2` 上限・`RVF_POC_VST3_PATH` 環境注入・証跡 tee）+ `cargo run` 手動実行 → `evidence/rf389-vst3-editor/` 保存 | 実機 run log + report tuple |
| T-5 | 正規検証 `mise run fmt-check`/`check`/`lint`/`test` + vst3 host lane + poc vst3 lane + 独立 review（A/B→Validator→Judge）+ IR/report/PR | 全 green + report |

## 検証区分

- **決定的 headless lane**: workspace fixture（`createView` が null
  を返す既存 gain fixture）で Unsupported 分類と寿命順序を固定。
  実 plugin binary は headless CI に混ぜない。
- **manual 実機 lane**: `Vinyl.vst3` のみ対象。plugin 未入手・
  load 失敗・editor 非搭載は `Fail`/`NotRun`/`Unsupported` を
  個別記録し成功偽装しない。

## リスク

| リスク | 対処 |
| --- | --- |
| `Vinyl.vst3` が `iZVinyl.dll` 依存で LoadLibrary 失敗 | 依存 DLL 探索を記録（同 dir 自動解決前提だが失敗時は `Fail` 記録。PATH/設定改変でごまかさない） |
| plugin が heavy init で load 時 timeout | load 段階と open 段階を分けて evidence 化。中断時は `Blocked` + exit code 記録 |
| `attached` 失敗（HWND 非対応等） | `Unsupported`/`PluginFailed` の honest 分類で記録 — editor 非対応でも load+createView 到達証跡は残る |
| editor が preferred size > container で clip | slot `set_geometry` で container を plugin size へ resize — 観測結果のみ記録（`onSize` 未通知のため子 rect 追従は plugin 挙動次第） |
| COM/スレッド affinity 違反で crash | GPUI window 所有スレッド上の tick 駆動のみで GUI 呼出。OpenGui は既存 L0 経路（同一スレッド） |

## Scope boundary

- `InProcessEditorOps`/`PluginEditorSession`/`GpuiSurfaceAdapter`/
  slot 契約本体は変更しない（他 owner の境界）。
- `Vst3Plugin` の DSP/process/params/状態遷移は変更しない。
- `create_sendable`、wire、L1 契約、CLAP/AU、Dock widget 本体は
  変更しない。
- 共有 `Cargo.lock`/`crates/README.md`/docs index は最小変更。
  `AGENTS.md` クレート数は変更しない。

## 完了条件

- test-spec 全 headless case green + workspace 検証 green
- 実機 lane の実行 log/report が `evidence/rf389-vst3-editor/`
  へ実値保存（PASS/Fail/NotRun/Unsupported を tuple 付きで記録）
- review-protocol で blocker なし
- PR 作成（Refs #389、AC 残 Gate を明記）
