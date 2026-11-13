---
id: "TESTSPEC-034WXSuI7vNQqJ4kL2mB8"
type: "test-spec"
status: "approved"
title: "Issue #389 Parent Lease 移行 test-spec"
created: "2026-11-13"
updated: "2026-11-13"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-389-parent-lease.md"
    - "docs/designs/issue-389-parent-lease-design.md"
  verifies:
    - "docs/specs/issue-389-parent-lease.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 Parent Lease 移行 test-spec

正本 spec: `docs/specs/issue-389-parent-lease.md`。
設計: `docs/designs/issue-389-parent-lease-design.md`。

## Feature: ParentLease wire/境界契約

### Rule PL-01: `ParentLease` は generation+payload を保持し opaque に運ばれる

- `ParentLease::new(generation, payload)` で生成し、
  `generation()`/`payload()` が同値を返す。
- `Copy`/`Eq`/`Hash` を持ち、session/plugin 境界で再解釈されない。

#### Example PL-01-a

`ParentLease::new(7, 0xDEAD_BEEF)` → `generation()==7`,
`payload()==0xDEAD_BEEF`。

### Rule PL-02: wire `OpenGui` が `ParentLease` を roundtrip する

- `encode_parent_lease`/`decode_parent_lease` が generation(u32)+
  payload(u64) を符号化し、decode で同値復元。
- `Command::OpenGui` の encode→decode で `parent` が一致。
- 境界値: `generation=u32::MAX`, `payload=u64::MAX`。

#### Example PL-02-a

`ParentLease::new(u32::MAX, u64::MAX)` の field roundtrip が一致。

## Feature: session/host 非回帰

### Rule PL-03: `LeaseRedeemer` は `ParentLease` を返し検証済み lease のみ通す

- redeemer が `Err` を返すと `open` は `EditorOpError` で失敗し
  `HostCommand::OpenGui` は発行されない。
- redeemer が `ParentLease` を返すと `HostCommand::OpenGui{parent}`
  にそのまま載る（型レベルで保証、runtime 追加検査なし）。

#### Example PL-03-a

既存 `tests/editor_session.rs` の redeemer fixture を `ParentLease`
返却に更新し、open 成功/拒否の双方が現行どおり動く。

### Rule PL-04: `GuiParentToken` は公開 API に存在しない

- `crates/` 内で `GuiParentToken` への参照が 0 件
  （doc コメント・README を含む）。
- `ParentSlotLease`/`PluginEditorSessionId` の session-facing 契約は不変。

#### Example PL-04-a

`grep -r GuiParentToken crates/` が 0 件。

## Feature: adapter 契約の維持

### Rule PL-05: VST3 adapter は `payload()` のみを OS handle として使う

- `open_gui` は `parent.payload()` を `*mut c_void` に narrow し、
  `generation` を OS 層に渡さない。
- 非 Windows/未対応 platform/二重 open/未初期化の既存
  `Unsupported`/`PluginFailed` 契約は不変。

#### Example PL-05-a

Windows host での実 attach は NotRun（外部 gate）。
headless では型追従のみ検証。

### Rule PL-06: CLAP/L1 GUI は Unsupported のまま

- `L1EditorOps` 全操作 `Unsupported`、clap `open_gui` default
  `Unsupported`、`l1/mod.rs` の GUI verb 拒否は不変。

#### Example PL-06-a

既存 L1 refusal test が非回帰で green。

## 実行状態の正直な記録

- 実 VST3/CLAP GUI attach、Native Dock 実成立、外部 plugin 受入:
  **NotRun**（#388/#532/SV#176 外部 gate）。
- Session headless Pass を実 Plugin Pass と主張しない。
