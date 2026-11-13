---
id: "SPEC-034WXSykXHx9U87oel8vLy"
type: "spec"
status: "approved"
title: "Issue #389 GuiParentToken → 型付き Parent Lease 公開 API 移行"
created: "2026-11-13"
updated: "2026-11-13"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-parent-lease.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 GuiParentToken → 型付き Parent Lease 公開 API 移行

## 背景

Issue #389 の Plugin Editor Session 系スライス（状態機械・deadline・
Degrade・Fake/ScriptedOps）は PR #503/#530/#544 で統合済み。残件は
「raw `GuiParentToken` の公開面からの除去 → generation 付き不透明
Parent Lease 型への移行」。PR #427 で記録済みの移行表を正本とする。

現状、`GuiParentToken(u64)` は low-level `Plugin::open_gui`・
`HostCommand::OpenGui`・`wire::Command::OpenGui`・adapter 境界で
そのまま公開されており、OS handle が driver-neutral 境界を素通り
し得る。Session 系 API は既に `ParentSlotLease` を受けるが、
`LeaseRedeemer` が `GuiParentToken` を返すため raw token 語彙が
session→host→plugin 経路に残っている。

## 目的

- 公開 API（`Plugin::open_gui`/`ErasedPlugin::open_gui`、
  `wire.rs`、`boundary.rs`、`ruviform-plugin-host`/
  `-vst3`/`-clap`/`ruviform-ui-gpui`/`ruviform-poc-native-dock`
  の呼び出し側）から raw `GuiParentToken`（OS handle 直露出）を除き、
  generation を持つ不透明な Parent Lease 型へ移行する。
- OS handle は adapter 内部に閉じ込める（Driver-neutral boundary を
  破らない）。
- 旧 API → 新 API の移行表と保持方針を文書チェーン内に確定する。

## スコープ外（本スライスで扱わない）

- 実 VST3/CLAP/AU の L2/L3・Native Dock 実成立は #388/#532/SV#176 の
  外部 gate。本スライスは headless に実装可能な公開 API 移行のみ。
- Session 状態機械・deadline・Degrade・Fake/ScriptedOps fixture の
  再構築はしない（統合済み）。
- L1 GUI は引き続き Unsupported。wire GUI 動詞の実装を主張しない。

## 受入条件

- [ ] 公開 API から raw `GuiParentToken` を除去し、generation 付き
      不透明 `ParentLease` 型へ移行する。
- [ ] OS handle は adapter 内部に閉じ込め、Driver-neutral boundary を
      維持する。
- [ ] 旧 API → 新 API の移行表と保持方針を文書チェーンに確定する。
- [ ] Windows/macOS/Linux で利用可能な Backend の Build/Link・型付き
      Host Port・Session/generation/tick/timeout/cleanup/RT 境界を
      既存 CI 内の Headless 試験で検証する。Backend 非実装なら機能の
      欠落を明記する。
- [ ] Native Dock 不成立は Unsupported の確証がない限り NotRun/既知
      Fail のまま保持し、Session headless Pass から実 Plugin Pass を
      主張しない。
