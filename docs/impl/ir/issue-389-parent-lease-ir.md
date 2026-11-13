---
id: "IMPL-034WYcEVkJHFgwJtp10GaL"
type: "impl"
status: "completed"
title: "Issue #389 Parent Lease 移行 IR"
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
  implements:
    - "docs/tasks/issue-389-parent-lease-task.md"
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
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [389]
  experiments:
    adopted: []
    rejected: []
---

# Issue #389 Parent Lease 移行 IR

## Summary

raw `GuiParentToken` を公開 API から完全に除去し、generation 付き
不透明 `ParentLease { generation: u32, payload: u64 }` へ移行した。
`Plugin::open_gui`/`ErasedPlugin::open_gui`、`HostCommand::OpenGui`、
`wire::Command::OpenGui`、`LeaseRedeemer` の戻り値が `ParentLease`
を運び、OS handle は引き続き adapter 内部に閉じ込められる。
session-facing API（`ParentSlotLease`/`PluginEditorSessionId`/
`EditorSessionOps`）は不変。`WIRE_VERSION` を 4 → 5 に bump
（`OpenGui` payload が u64 → u32+u64 に変わるため codec 規約どおり）。

## Context

- Task: `docs/tasks/issue-389-parent-lease-task.md`
- Plan: `docs/plans/issue-389-parent-lease-plan.md`
- Design: `docs/designs/issue-389-parent-lease-design.md`（D1–D6）
- Spec: `docs/specs/issue-389-parent-lease.md`
- Test-spec: `docs/test-specs/issue-389-parent-lease.md`（PL-01..06）
- Issue: scarecrow173/ruviform#389（RF-GPUI-06 残件）

## Implementation

### Changes

- `ruviform-plugin-core/src/boundary.rs`: `GuiParentToken` 削除、
  `ParentLease` 新設（`#[doc(hidden)] new`、`generation()`/
  `payload()` accessor、Copy/Eq/Hash、RT-suitable 契約）。
  `ParentSlotLease` doc を redeem seam → `ParentLease` の流れに更新。
- `ruviform-plugin-core/src/plugin.rs`: `Plugin::open_gui`・
  `ErasedPlugin::open_gui` を `ParentLease` 化。
- `ruviform-plugin-core/src/wire.rs`: `Command::OpenGui.parent` を
  `ParentLease` 化、`encode_parent_lease`/`decode_parent_lease`
  新設（generation u32 + payload u64）、`WIRE_VERSION` 5、
  roundtrip/version テスト更新。
- `ruviform-plugin-core/src/lib.rs` + `README.md`: export/語彙更新。
- `ruviform-plugin-host`: `HostCommand::OpenGui.parent`、
  `LeaseRedeemer` 戻り値、`InProcessHost::open_gui` 転送、re-export、
  lib/host/tests を `ParentLease` 化。
  `tests/editor_session.rs` の redeemer 検証を `ParentLease` 全体
  （generation+payload）の一致 assert に強化。
- `ruviform-plugin-vst3/src/host.rs`: `open_gui` が
  `parent.payload()` のみを `*mut c_void` に narrow。`generation`
  は OS 層に渡さない。既存 Unsupported/PluginFailed 契約は不変。
- `ruviform-plugin-clap`: `open_gui` 呼び出し側テストを型追従。
  CLAP GUI 実装は追加せず default `Unsupported` のまま。
- `ruviform-ui-gpui/src/gpui_driver/surface.rs`: doc コメントの
  語彙のみ更新（`redeem` シグネチャ・ledger 検証は不変）。
- `ruviform-poc-native-dock`: 3 箇所の redeemer closure が
  `adapter.redeem(lease.raw())` の token と
  `SlotLease::new(lease.raw()).seq()` の generation から
  `ParentLease::new` を生成。probe `open_gui` は `payload()` を使用。
  README/main doc 更新。

### Decisions During Implementation

- `ParentLease` は認証ではなく opaque vocabulary — stale/revoked/
  forged 拒否は引き続き redeem seam の ledger 検証が権威。
  `generation` は issuer lease epoch を型に束縛する世代情報で、
  wire 越しに運ばれる（将来の L1 検証余地）が、本スライスでは
  plugin 側での追加検査は行わない（設計 D1/D3）。
- `GuiParentToken` は残さず完全削除 — 全使用箇所が編集範囲内で、
  `plugin-export` は `parent` を opaque 転送するため非依存。
- `WIRE_VERSION` bump は codec の「payload field-set changes always
  ride a version bump」規約に従う。L1 GUI 未実装のため実 consumer
  影響なし。host/runner は同一 workspace で同時出荷。
- poc redeemer の generation 復元は `SlotLease::new(lease.raw())
  .seq()` — `ParentSlotLease` payload が `SlotLease` の packed
  `{index,seq}` である既存契約を利用（poc は `SlotLease` を既に
  import 済み）。

## Related Experiments

### Adopted

なし（実験的分岐なし — 設計 D1–D6 をそのまま実装）。

### Rejected

なし。

## Validation

Windows 11 / x86_64、mise 管理 toolchain（rustc は `mise.toml`
pin）。canonical = mise タスク。cargo 直接実行なし。

| コマンド | 結果 |
| --- | --- |
| `mise run fmt-check` | Pass（exit 0、差分なし） |
| `mise run check ruviform-plugin-core` | Pass |
| `mise run check ruviform-plugin-host` | Pass |
| `mise run check ruviform-plugin-vst3 --features host` | Pass |
| `mise run check ruviform-plugin-clap --features host` | Pass |
| `mise run check ruviform-poc-native-dock` | Pass |
| `mise run check ruviform-ui-gpui` | Pass |
| `mise run check ruviform-plugin-export` | Pass（下流非回帰） |
| `mise run check ruviform-plugin-ofx` | Pass（下流非回帰） |
| `mise run check ruviform-audio-plugin-bridge` | Pass（下流非回帰） |
| `mise run test ruviform-plugin-core` | Pass（126 tests + doctests） |
| `mise run test ruviform-plugin-host` | Pass（lib 226 + editor_session 49 + l1_proxy_e2e 187） |
| `mise run test ruviform-plugin-vst3 --features host` | Pass（74+2、real-adapter e2e 3 件は ignored=要外部 plugin） |
| `mise run test ruviform-plugin-clap --features host` | Pass |
| `mise run test ruviform-plugin-export` | Pass（53 tests） |
| `mise run test ruviform-poc-native-dock` | Pass（0 tests — bin crate） |
| `mise run lint ruviform-plugin-core` | Pass（`-D warnings`） |
| `mise run lint ruviform-plugin-host` | Pass |
| `mise run lint ruviform-plugin-vst3 --features host` | Pass |
| `mise run lint ruviform-plugin-clap --features host` | Pass |
| `mise run lint ruviform-poc-native-dock` | Pass |
| `mise run lint ruviform-ui-gpui` | Pass |

備考: `mise run test ruviform-plugin-host` 初回は
`ruviform-plugin-l1-runner` bin 未ビルドで e2e 2 件が失敗
（インフラ前提）。`mise run build ruviform-plugin-export` 後に
再実行して全件 Pass — 変更とは無関係の環境前提。

`grep -r GuiParentToken crates/` → 0 件（PL-04）。

## Risks

- `ParentLease` は同一プロセス trust boundary の opaque 語彙であり
  認証ではない。trust boundary を越える利用には別途認証が必要
  （`SlotLease` 既存契約と同じ制約を doc に明記済み）。
- `WIRE_VERSION` 5 への bump により、v4 の host/runner 混在は
  handshake 失敗になる — 同一 workspace 同時出荷の前提どおり。
- 実 VST3/CLAP GUI attach・Native Dock 実成立・外部 plugin 受入は
  本スライス外（NotRun）。

## Follow-ups

- 実 VST3/CLAP/AU の L2/L3・Native Dock 実成立: #388/#532/SV#176
  の外部 gate。
- cross-repo consumer（audio-lens/synvelis）の `GuiParentToken` →
  `ParentLease` 追従は RF #219 互換契約に基づき別途調整。
- L1 GUI 実装時に `ParentLease.generation` の receiver 側検証を
  設計する余地あり（本スライスでは opaque に運ぶのみ）。
