---
id: "DESIGN-mEXH6f0uKRaa2mwCWE8mxC"
type: "design"
status: "approved"
title: "ruviform-plugin-host 設計（L0 インプロセス・ハンドル/コマンド境界）"
created: "2026-11-03"
updated: "2026-11-03"
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
    - "docs/specs/plugin-host-l0.md"
  implemented-by: []
  depends-on:
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/plugin-host-l0.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/plans/implement-issue-74-plugin-host-l0.md"
    - "docs/test-specs/plugin-host-l0-tests.md"
    - "docs/discovery/plugin-l1-out-of-process.md"
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/discovery/engine-plugin-host-integration.md"
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-host-l0-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-plugin-host 設計（L0）

## Overview

#31 の境界契約を実装する host 層。公開面は **`PluginHandle` + `HostCommand`/`HostReply` の純データ型**であり、L0 の `InProcessHost` は同一コマンド経路をインプロセスで実行する。呼び出し側は分離レベルを意識しない — L1 は同じコマンド/リプライを別バックエンドが処理する形で追加される。

## 型設計

| 型 | 形状 | 責務 |
| --- | --- | --- |
| `PluginHandle` | `{slot: u32, generation: u16}` の `Copy` 不透明ハンドル | インスタンス参照。generation で破棄済みスロットの再利用を検出 |
| `HostCommand` | `#[non_exhaustive]` enum・純データ（`String`/`Vec<u8>`/値型のみ） | scan・instantiate・ライフサイクル・state・gui の非 RT オペレーション。**`process` は含まない**（#31 設計のコマンド分岐 — L0 直接呼出し / L1 共有メモリ） |
| `HostReply` | `#[non_exhaustive]` enum・純データ | `Scanned(Vec<PluginDescriptor>)`・`Instantiated(PluginHandle)`・`State(Vec<u8>)`・`Opened(GuiHandle)`・`Ok` |
| `HostError` | `#[non_exhaustive]` enum・`Copy` | `UnknownPluginId`・`InvalidHandle`・`Lifecycle(LifecycleError)`・`Prepare(PrepareError)`・`PluginFailed`・`Unsupported` |
| `Plugin` | trait（`PluginProcessor` を継承） | host が駆動するインスタンス契約: `descriptor()`・`params()`・`init()`・`start_processing()`/`stop_processing()`（既定 no-op）・`save_state()`/`restore_state()`・`open_gui()`/`close_gui()`（既定 `Unsupported`） |
| `PluginFactory` | trait | `descriptor()` + `create() -> Box<dyn Plugin>` — アダプタが登録する生成面 |
| `GuiHandle`/`GuiParentToken` | `u32`/`u64` newtype | GUI は規格固有のため不透明トークンのみ（#30 の規格準拠 attach と整合） |
| `InProcessHost` | `factories: Vec<Box<dyn PluginFactory>>` + スロット配列 | L0 実行体。`execute(command)` で全非 RT コマンドを処理 |

## 振る舞い

- `Instantiate { plugin_id }`: `plugin_id`（`PluginDescriptor.id`）で factory を検索し `create()`。インスタンスは `Lifecycle::new()`（`Created`）から開始
- `Initialize`/`Activate`/`StartProcessing`/`StopProcessing`/`Deactivate`/`Destroy`: コマンド → `LifecycleEvent` に写像し `transition` を先に評価。`Activate` は `Plugin::prepare`（`PluginProcessor` 継承）を `ProcessConfig` で呼び、失敗時は `ActivateFailed` 遷移 + `Prepare` エラー。`Destroy` は状態機械が許可する場合のみスロットを解放
- `SaveState`/`RestoreState`: `Plugin` の state フックを呼ぶ（`Initialized` 以降のみ）
- `OpenGui`/`CloseGui`: `Plugin` 既定実装は `Unsupported` — GUI 対応プラグインのみ実装
- `process`（host メソッド・コマンド外）: `lifecycle.transition(Process)` で `Processing` 状態を検査してから `plugin.process` を直接呼ぶ。RT 適合 — 遷移は const テーブル参照のみ・allocation なし・戻り値は `Option<ProcessResult>` 相当で `InvalidHandle`/非 `Processing` を reject

## RT 面制約

- コマンド経路（`execute`）は**非 RT 専用** — Vec 操作・plugin の init/prepare/state を含むため
- RT 経路は `process`/`processor(handle)` アクセサのみ — 直接 `&mut dyn PluginProcessor` 呼出しで IPC・ブロッキングを持ち込まない（#31 設計の L0 分岐そのもの）
- `InProcessHost` 自体は `Send` 不要求（L0 は同一スレッド駆動想定。並行アクセスは呼出し側責務）

## 実装単位

- `crates/ruviform-plugin-host` 新設（workspace 追加・`publish.workspace = true` 継承）
- ファイル分割: `lib.rs`（crate doc + re-export）・`command.rs`・`plugin.rs`・`host.rs`

## Verification

- [x] #31 境界契約（ハンドル + 直列化可能コマンド・process 分岐）を実装形に具体化 — `HostCommand` に `process` を含まない設計で確認
- [x] RT 契約: `process` 経路は const 遷移 + 直接呼出しのみ（`rt_` テストで 0 alloc 実測）
- [x] 依存方向: host → plugin-core・audio-core・media-core のみ（アダプタ逆依存なし）
