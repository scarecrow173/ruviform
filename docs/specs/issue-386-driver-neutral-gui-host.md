---
id: "SPEC-0cG0IEArkb6RGWXH0jtA3e"
type: "spec"
status: "proposed"
title: "Issue #386 Driver-neutral GUI Host 契約・GPUI 標準 Driver adapter・最小 Window fixture"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/384"
  changes:
    added: []
    modified: []
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
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/390"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/issue-386-driver-neutral-gui-host-inventory.md"
  derived-by:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #386 Driver-neutral GUI Host 契約・GPUI 標準 Driver adapter・最小 Window fixture

## Goal

Issue #386 の 2026-11-07 横断レビュー確定仕様 v2 に従い、
Ruviform の GUI Window/Host を **backend-neutral semantic contract**
として再定義し、標準 Driver である **GPUI adapter**（新クレート
`ruviform-ui-gpui`）と、#388 が早期利用する**最小 Window fixture**
を提供する。旧 WebView 経路は新 Native 実証完了まで削除せず、
旧 API → 新 API の移行表を作成する。

## Scope

### In scope（本 Issue が所有）

1. **共通契約**（`ruviform-ui` 内・GPUI/DOM/wry/OS handle 非依存）:
   `WindowSpec`、`WindowLifecycle` event payload、`DriverEvent`
   envelope、`WindowDriver` port、`WindowCapabilities`/`Capability`、
   `UiError::Unsupported`、論理 `LogicalSize`/`LogicalPosition`。
   既存 `WindowId`/`CloseDecision`/`MonitorInfo`/`Intent`/`Event`/
   `DockWorkspace` は再利用。
2. **GPUI Driver**（`ruviform-ui-gpui`・adapter 層）:
   `WindowDriver` 実装、`run` 入口、`gpui` feature ゲート
   （`default = []` で `cargo check --workspace` が GPUI なしで
   完走する headless 経路を維持）。GPUI 依存はこの crate と
   明示的 composition root / driver extension のみ。
3. **最小 Window fixture**: `examples/` デモ + 接続試験。
   Fake Driver（`ruviform-ui-test` 内）と GPUI Driver で
   `WindowSpec`→lifecycle→close veto→`Destroyed` の同一意味
   fixture を実施。2 window・detach/reattach・IME・DPI・Dock
   rollback の実 OS Gate は**後続 Gate として明示的に残る**。
4. **旧 WebView API 移行表**（discovery + design doc 内）と
   adapter 接続試験（Fake/GPUI 双方で同一契約）。
5. GPUI の採用 version/license/OS/MSRV/transitive 依存の
   **確認記録**（#262 採否 Gate への入力）。

### Out of scope（他 Issue が所有）

- `NativeSurfaceSlot` semantic contract・実 OS embed（#388）
- `PluginEditorSession`・L0/L1 GUI adapter（#389）
- `DriverContractFixture`・Fake Driver registry・
  Native Component Lab・golden（#390）
- ResolvedTheme/Design Token（#385）、Component/View Spec（#387）
- 旧 `ruviform-ui` API の削除・物理移動（別明示判断）
- ADR ファイルの新設・supersession（#384）
- CI workflow の新設・変更（#294）

## Acceptance criteria

- [ ] `ruviform-ui` の公開 API が GPUI/DOM/wry/OS handle 型を
  持たないままビルドできる（feature なし `cargo check` 経路）。
- [ ] `WindowSpec`/`WindowLifecycle`/`DriverEvent`/`WindowDriver`/
  `WindowCapabilities`/`UiError::Unsupported` が共通契約として
  定義され、旧 `WindowConfig`/`WindowHandle`/`HostEvent` は
  破壊せず残る。
- [ ] `ruviform-ui-gpui` が `gpui` feature で `WindowDriver` を
  実装し、最小 window fixture が `WindowSpec` から window を
  生成・close veto・`Destroyed` 終端まで扱える。
- [ ] Fake Driver と GPUI Driver が**同一の lifecycle 意味論**
  （create → Ready → close flow → Destroyed）で接続試験を通る。
- [ ] 旧 API → 新 API の移行表（型・実装ファイル・移行手順）が
  文書化されている。
- [ ] GPUI 0.2.2（crates.io）の version/license/transitive/
  Windows 実ビルドが確認・記録されている。
- [ ] 実 OS・実 Plugin Dock・IME/DPI の未実施項目が
  NotRun/Unsupported として明記される。

## 非目標

- 本番用 full Host/全 control/Component Lab の完成。
- GPUI 最終採否の宣言（#262 + #388/#389 の実機証拠待ち）。
- `ruviform-ui` の既存公開 API の削除・リネーム。

## 検証入口

- `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui`
  （feature ゲート lane — 本 Issue で新設。workflow 配線は #294 スコープ）
- `mise run lint` / `test`（workspace の既存 lane。headless 経路を維持）
