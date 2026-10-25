---
id: "TASK-034LNY736TLX2w3k4RFFZM"
type: "task"
status: "todo"
title: "plugin-core L1 基盤（契約移設 + Lifecycle Failed + wire codec）"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
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
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# plugin-core L1 基盤（契約移設 + Lifecycle Failed + wire codec）

## Work

approved plan `implement-issue-76-plugin-l1-out-of-process.md` の **actions 1–4 を 1 task の ordered substeps として実行**。plugin-core が L1 の共有基盤を獲得する: (A1) host→core へのプラグイン側契約移設（`pub use` 再エクスポートで既存パス互換）、(A2) `LifecycleState::Failed`/`LifecycleEvent::Failed` 終端準状態、(A3) `wire` モジュール（6 メッセージ・LE codec・固定 wire レイアウト）。

**先行確認**: host `src/plugin.rs`/`src/command.rs` の移設対象型の現行定義・`HostError` の既存 variant 集合・`lifecycle.rs` の遷移表 const 構造を確認 — design と相違なら blocker を記録（fake success なし）。

### A1 — 契約移設（plugin.rs + boundary.rs 新設・host は再エクスポート化）

- `crates/ruviform-plugin-core/src/plugin.rs` 新設: `Plugin`/`PluginFactory`/`ErasedPlugin`/`erase`/`HostInput`/`HostOutput` を host `src/plugin.rs` から移設（`crate::` 参照に調整・doc 互換維持）
- `crates/ruviform-plugin-core/src/boundary.rs` 新設: `PluginHandle`/`GuiHandle`/`GuiParentToken`/`HostError` を host `src/command.rs` から移設 — `PluginHandle::new` を `#[doc(hidden)] pub` 化（host が mint するが型は共有語彙）
- `HostError` に `Failed`/`Ipc`/`DuplicatePluginId` variant + Display arm を追加（`#[non_exhaustive]` で non-breaking）
- `lib.rs`: `mod plugin`/`mod boundary` + ルート re-export — core は外部依存ゼロ維持
- host `src/plugin.rs`・`src/command.rs` は `pub use ruviform_plugin_core::{...}` へ差替え（`HostCommand`/`HostReply` は host に残る）— `ruviform_plugin_host::*` の既存公開パスが無変更でコンパイル継続することを既存テスト全パスで実証

### A2 — `LifecycleState::Failed`/`LifecycleEvent::Failed`

- `crates/ruviform-plugin-core/src/lifecycle.rs`: `LifecycleState::Failed`（終端準状態 — `(Failed, Destroy) → Destroyed` のみ合法）・`LifecycleEvent::Failed`（報告遷移 — host が子プロセス死亡・応答喪失・プロトコル喪失を注入）・`is_failed()` を追加
- 遷移表 const 拡張: `Created|Initialized|Activated|Processing → Failed`・`(Failed, Destroy) → Destroyed`・その他 `(Failed, *) → IllegalTransition`。`accepts_processing()` は `Failed` で `false`
- doc 区別: retryable `InitFailed`/`ActivateFailed` とは意味が異なり terminal である旨を variant/event doc に明記。`#[non_exhaustive]` で非破壊追加
- `lifecycle_at` helper・遷移表テスト更新（`Failed` の到達・脱出・違法遷移・`accepts_processing`/`is_failed` の契約）

### A3 — `wire` モジュール（codec + 型 + 固定レイアウト）

- `crates/ruviform-plugin-core/src/wire.rs` 新設: `pub mod wire`（export/host 双方が参照する共有面 — lib.rs で `pub mod wire` + crate doc 更新）
- 定数: `WIRE_VERSION = 1`・`WIRE_MAX_FRAMES = 8192`・`WIRE_MAX_CHANNELS = 16`・`WIRE_MAX_EVENTS = 1024`
- 型: `Message`（`Hello{version,capabilities}`/`Command{seq,cmd}`/`Reply{seq,result}`/`BlockIn{seq}`/`BlockDone{seq,result}`/`Shutdown`）・`wire::Command`（`Scan`/`Initialize`/`Activate{config}`/`StartProcessing`/`StopProcessing`/`Deactivate`/`SaveState`/`RestoreState{bytes}`/`OpenGui{parent}`/`CloseGui` — `Instantiate`/`Destroy` 動詞は持たない）・`wire::Reply`（`Ok`/`Scanned(Vec<Descriptor>)`/`State{bytes}`/`Opened{gui}`）・`wire::Descriptor`（所有型 `String`/`Vec<String>` + `From<PluginDescriptor>` + フィールド等価比較）— enum は `#[non_exhaustive]`、`Descriptor` は struct
- codec: `encode`/`decode`（`[u32 LE payload_len][u8 kind][payload]`・`WireError` で malformed/truncated/未知 kind/上限超を `Err` 拒否・panic/部分消費なし）
- 固定 wire レイアウト: `ProcessConfig`/`ProcessResult`/`GuiHandle`/`GuiParentToken`/`PluginHandle`/`HostError` は直接 encode/decode・`TransportInfo`/`Event`/`EventList`/`MediaTime` は repr 固定（accessor encode・ctor decode — `repr(Rust)` memcpy 横断なし）
- inline `#[cfg(test)] mod tests`: test-spec `plugin-l1-wire-codec-tests.md` の roundtrip/境界/malformed/version/`Descriptor` 等価/非破壊移設ケース

### A4 — gate

- `mise run check`・`lint`・`test` exit 0 — 移設後の無退行・`Failed` 遷移表・codec roundtrip/malformed 全 pass
- `mise run test-plugin-core`・`lint-plugin-core` exit 0 — core 集中検証の既存レーン
- `cargo tree -p ruviform-plugin-core` に serde/postcard 等の外部依存が出ない（core 外部依存ゼロ維持）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-76-plugin-l1-out-of-process.md](../plans/implement-issue-76-plugin-l1-out-of-process.md)
- [../specs/plugin-l1-out-of-process.md](../specs/plugin-l1-out-of-process.md)
- [../designs/plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)
- [../test-specs/plugin-l1-wire-codec-tests.md](../test-specs/plugin-l1-wire-codec-tests.md)

## Dependencies

- 先行: approved plan のみ（最初の task — 後続の export/proxy 双方の共有前提）

## Non-Goals

- L1 プロキシ・runner・`PluginHost` 本体（後続 task）・wire バージョン進化・bundle 子（handle 修飾）・shm スロットレイアウト（host `l1/shm.rs` が所有）
- `InProcessHost` の挙動変更 — 移設は `pub use` で完全互換（既存パス不変）

## Affected Files

- `crates/ruviform-plugin-core/src/plugin.rs`（新規）
- `crates/ruviform-plugin-core/src/boundary.rs`（新規）
- `crates/ruviform-plugin-core/src/lifecycle.rs`（`Failed`/`is_failed`・遷移表・doc）
- `crates/ruviform-plugin-core/src/wire.rs`（新規）
- `crates/ruviform-plugin-core/src/lib.rs`（mod + re-export + crate doc）
- `crates/ruviform-plugin-host/src/plugin.rs`・`src/command.rs`（`pub use` 再エクスポート化 — `HostCommand`/`HostReply` は残る）

## Completion Gate

4 entry 全 green 実 log（`check`/`lint`/`test` + `test-plugin-core`/`lint-plugin-core`）+ lead review の後にのみ task 完了・buildable scope commit。移設型の既存 `ruviform_plugin_host::*` パスが無変更でコンパイル継続（既存テスト全パス）が lead review の焦点 — fake success なし。

## Verification

- [ ] `mise run check` — exit 0（移設後の全 crate 型チェック）
- [ ] `mise run lint` — exit 0（clippy `--all-targets -D warnings` clean）
- [ ] `mise run test` — exit 0（`Failed` 遷移表・codec roundtrip/malformed/version/Descriptor 等価・既存 L0 テスト全 pass）
- [ ] `mise run test-plugin-core` — exit 0（core 集中レーン）
- [ ] `mise run lint-plugin-core` — exit 0
- [ ] `cargo tree -p ruviform-plugin-core` — 外部依存ゼロ（serde/postcard 不在）
- [ ] `rg -n "pub use ruviform_plugin_core" crates/ruviform-plugin-host/src/plugin.rs crates/ruviform-plugin-host/src/command.rs` — 再エクスポート化を確認
- [ ] `rg -n "Failed" crates/ruviform-plugin-core/src/lifecycle.rs` — `LifecycleState::Failed`/`LifecycleEvent::Failed`/`is_failed` 存在
