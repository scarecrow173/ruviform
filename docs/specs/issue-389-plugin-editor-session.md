---
id: "SPEC-ynejrtwiz2jIJrf3DwFtAK"
type: "spec"
status: "approved"
title: "Issue #389 Driver-neutral PluginEditorSession（state / typed HostPort / L0-L1 接続設計）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
    - "docs/designs/plugin-l1-out-of-process.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
  derived-by:
    - "docs/designs/issue-389-plugin-editor-session.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/115"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 389
---

# Issue #389 Driver-neutral PluginEditorSession

## 背景

Issue #389（2026-11-07 横断レビュー確定仕様 v2）の契約側。規格別
Gui/View と Ruviform 共通 Window/Dock Presentation を接続する
**型付き Plugin Editor Session / Host Port** を、GPUI 非依存の
Plugin Host/View Session Contract として定義する。Native Surface
Slot の実装詳細を共通 Plugin Host 契約に漏らさず、L0/L1 と GUI
表示方式を独立させる。

## スコープ

### In Scope

- `ruviform-plugin-core::boundary` 追加語彙:
  `PluginEditorSessionId`（`{slot,generation}` 形・`PluginHandle`
  準拠）・`ParentSlotLease`（opaque・generation-bound・
  lifetime-checked な親 slot lease の session 面語彙。`u64` ペイ
  ロードで発行側構造は opaque）。
- `ruviform-plugin-host` 新モジュール `editor_session`:
  `PluginEditorSession`（状態機械・request id/generation・terminal
  event 一度切り）・`PluginEditorSessions` registry（id mint・
  instance ≤1 session・instance destroy での converge）・
  `EditorSessionOps` port（L0/L1 adapter が実装する typed host
  port）・`EditorCapabilities`・`AttachMode`・`EditorSessionState`/
  `EditorSessionEvent`/`EditorSessionError`/`EditorOpError`/
  `EditorSize`。
- 状態遷移規範: `Closed → Creating → Attached(Docked|Floating) →
  Hidden|Detaching|Closing → Closed`・`Failed → Recovering|Closed`。
- `InProcessHost` への L0 ops 写像（`HostCommand::OpenGui`/
  `CloseGui` の既存経路を `EditorSessionOps` 経由で利用）。
- 現行 API・wire → 新 Session/Parent Lease/OS Adapter の移行表
  （追加/廃止/保持）と wire negotiation 設計（bump 前提の語彙案）。
- fake ops fixture による Open/Attach/Hide/Detach/Close・stale
  generation・double-close・timeout・unsupported・crash/host lost
  の terminal-event 検証。

### Out of Scope

- 実プラグインの GUI open（adapter `open_gui` 未実装 → NotRun）。
- L1 GUI の wire 動詞追加・`WIRE_VERSION` bump 実施（issue-115
  解除条件未充足 — 設計のみ記録）。
- GPUI Driver / `ruviform-ui-gpui` 実装（RF386 所有）。
- 旧 `GuiParentToken`/wire GUI 語彙の削除（keep-for-future 維持）。
- `Plugin` trait への破壊的変更（open_gui/close_gui signature 維持）。

## 受入条件

- [ ] `PluginEditorSession` が規範状態機械を型で強制し、二重
  Open/Close・stale callback・request 世代不一致・Window 破棄・
  Host crash の各経路で terminal event をただ 1 回通知する。
- [ ] `ParentSlotLease` が session API の parent 語彙となり、raw
  `GuiParentToken::new(u64)` の呼出が OS adapter 内へ隔離される
  契約を記述・fixture で実演する。
- [ ] `EditorSessionOps` の L0 実装（`InProcessHost` ラップ）が
  session→`HostCommand::OpenGui` の live 経路を通り、plugin 既定
  `Unsupported` が `EditorOpError::Unsupported` へ定義済み写像
  される。
- [ ] L1 ops は `EditorOpError::Unsupported` を返す定義済み拒否を
  維持し、L1 対応を黙って L0 へ退化させない。
- [ ] instance destroy 時に未 close session が `Failed`/`Closed` へ
  converge し、DSP instance の生存・RT と独立に widget teardown が
  扱われる設計である。
- [ ] 移行表（追加/廃止/保持）・状態遷移表・所有責務が design に
  記録されている。
- [ ] `mise run fmt-check`/`check`/`lint`/`test` が green。

## 制約・禁止

- 共通 API に `gpui`/`wry`/`tao`/`HWND`/`NSView`/X11 を露出しない。
- plugin 系 ↔ ui 系の相互依存禁止（session は `ParentSlotLease` を
  opaque に運び、`SurfaceSlotId`/`WindowId`/`ViewInstanceId` の
  具象型は消費側が写像する）。
- RT/Audio thread からの UI 呼出・ブロッキング・alloc・OS GUI API
  禁止。View close/hang で DSP/Graph/Project/RT を暗黙停止しない。
- 既存 Audio/RT wire の互換性を暗黙変更しない。
- fake の green を実 Native Dock PASS にしない。
- `unsafe` 原則禁止・全公開 enum `#[non_exhaustive]`。

## 関連

- 実装入力: `docs/discovery/issue-388-389-native-dock-current-state.md`
- 共同検証: `docs/specs/issue-388-native-surface-slot.md`（lease の
  発行側契約）
- 背景: `docs/designs/issue-115-l1-gui-cross-process-parenting.md`
  （L1 defer 確定・解除条件）
- Gate 接続: Issue #262、Synvelis #176
