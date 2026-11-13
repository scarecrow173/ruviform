---
id: "PLAN-034WXSRQARP7ZVwXSyqHsA"
type: "plan"
status: "approved"
title: "Issue #389 Parent Lease 移行 plan"
created: "2026-11-13"
updated: "2026-11-13"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-389-parent-lease.md"
    - "docs/designs/issue-389-parent-lease-design.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-parent-lease.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-parent-lease-design.md"
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

# Issue #389 Parent Lease 移行 plan

設計: `docs/designs/issue-389-parent-lease-design.md`（D1–D6）。
検証仕様: `docs/test-specs/issue-389-parent-lease.md`。

## ファイルマップ

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-plugin-core/src/boundary.rs` | `GuiParentToken` 削除、`ParentLease` 追加 |
| `crates/ruviform-plugin-core/src/plugin.rs` | `open_gui` シグネチャ `ParentLease` 化（default/`ErasedPlugin`） |
| `crates/ruviform-plugin-core/src/wire.rs` | `Command::OpenGui.parent` 型変更、`encode/decode_parent_lease` 新設、テスト更新 |
| `crates/ruviform-plugin-core/src/lib.rs` | export 差し替え（`GuiParentToken` → `ParentLease`） |
| `crates/ruviform-plugin-core/README.md` | 語彙記述更新 |
| `crates/ruviform-plugin-host/src/command.rs` | `HostCommand::OpenGui.parent` 型変更、re-export 更新 |
| `crates/ruviform-plugin-host/src/host.rs` | `open_gui` 転送の型追従 |
| `crates/ruviform-plugin-host/src/composite.rs` | wire 変換の型追従 |
| `crates/ruviform-plugin-host/src/editor_session.rs` | `LeaseRedeemer` 戻り値 `ParentLease` 化 |
| `crates/ruviform-plugin-host/src/lib.rs` | re-export 更新 |
| `crates/ruviform-plugin-host/tests/*.rs` | fixture/テスト追従 |
| `crates/ruviform-plugin-vst3/src/host.rs` | `parent.payload()` 使用 |
| `crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs` | 型追従 |
| `crates/ruviform-plugin-clap/src/host.rs` + tests | 型追従（実装は Unsupported のまま） |
| `crates/ruviform-ui-gpui/src/gpui_driver/surface.rs` | doc コメント語彙更新のみ |
| `crates/ruviform-poc-native-dock/` | redeemer closure を `ParentLease` 生成に、README/main doc 更新 |

## タスク分解

- T-1 plugin-core: `ParentLease` 新設 + `GuiParentToken` 除去 +
  plugin/wire/lib 追従 + wire テスト更新。
- T-2 plugin-host: command/host/composite/editor_session/lib +
  tests 追従。
- T-3 adapter/poc: vst3 host.rs、clap、ui-gpui docs、poc-native-dock
  redeemer/README/main doc。
- T-4 test-spec PL-01.. に対応する contract test 追加/更新、
  evidence/impl 記録。
- T-5 fmt-check + check/lint/test（変更クレート + 直接下流
  plugin-export/ofx/audio-plugin-bridge）+ review-protocol + PR。

## リスクと rollback

- wire `OpenGui` 符号化変更は L1 GUI 未実装のため実 consumer なし。
  `plugin-export` は opaque 転送で非依存。
- `ParentSlotLease`/`PluginEditorSessionId`/session 状態機械は不変 —
  既存 contract test が非回帰を担保。
- 戻しは型置換の revert のみ（状態機械への影響なし）。

## 検証マトリクス

| 層 | 手段 |
| --- | --- |
| wire roundtrip | `mise run test ruviform-plugin-core` |
| host/session contract | `mise run test ruviform-plugin-host` |
| adapter build/link | `mise run check/lint/test ruviform-plugin-vst3` `...-clap`（`--features host` 必要時） |
| poc compile | `mise run check/lint ruviform-poc-native-dock` |
| ui-gpui docs | `mise run check/lint ruviform-ui-gpui` |
| 下流非回帰 | `mise run check ruviform-plugin-export ruviform-plugin-ofx ruviform-audio-plugin-bridge` |
