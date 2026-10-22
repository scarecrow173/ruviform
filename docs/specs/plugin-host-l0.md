---
id: "SPEC-aNDyfHu1ZkBojTHEltDl94"
type: "spec"
status: "approved"
title: "ruviform-plugin-host 新設（L0 インプロセス・ハンドル/コマンド境界）"
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
  implements: []
  implemented-by:
    - "docs/designs/plugin-host-l0.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/plugin-isolation-design.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/designs/plugin-host-l0.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform-plugin-host（L0）spec

Issue #74 — Issue #31 のプラグイン分離設計（ADR `docs/adr/plugin-isolation-strategy.md`・design `docs/designs/plugin-isolation.md`）が規定した境界契約に基づき、`ruviform-plugin-host` クレートを新設し L0 インプロセス実行を既定パスとして実装する。

## 背景

ベースラインの拡張層は `ruviform-plugin-core`（規格中立値型・`PluginProcessor` 契約）→ `ruviform-plugin-host` → `ruviform-plugin-export` → CLAP/VST3 アダプタの構成を定めるが、host 層は未実装。#31 で L0（インプロセス既定）/L1（アウトプロセス opt-in）の階層化分離が採択され、「`PluginInstance` 不透明ハンドル + 直列化可能コマンド境界」を host の公開面とすることが境界契約として規定された。呼び出し側は分離レベルを意識せず、L1 導入時も同一境界を維持する。

## 要件

1. `ruviform-plugin-host` クレートを新設する（拡張層。許可依存: `ruviform-plugin-core` + foundations/Primitives のみ）
2. **境界契約**: `PluginHandle`（不透明・`Copy` 値）と `HostCommand`/`HostReply`（直列化可能な純データ型 — trait object・ポインタを含まない）を公開面とする。`process` はコマンド境界に含めない（#31 設計: L0 では直接 `PluginProcessor::process`、L1 では共有メモリ経路 — コマンド経路は非 RT オペレーション専用）
3. **L0 実行**: `InProcessHost` が登録済み `PluginFactory` 群を scan・instantiate し、ライフサイクル（plugin-core `Lifecycle` 機械駆動）・state 保存/復元・GUI 管理コマンドを同一プロセス内で実行する
4. **RT パス**: `processor(handle)`/`process(...)` が L0 直接呼出しを提供し、ブロッキング・IPC・allocation を持ち込まない（lifecycle `Process` 遷移は const テーブル参照のみ）
5. **プラグイン側契約**: 規格中立の `Plugin` trait（`PluginProcessor` + descriptor/params/state/gui フック）と `PluginFactory` trait を定義。アダプタ（clap/vst3）は本 trait を実装して host に登録する（アダプタ側の組込は本 Issue 外）
6. L1 アウトプロセス・共有メモリ・子プロセス監視・`ruviform-plugin-export` はスコープ外（#76 等の後続 Issue）

## Acceptance Criteria

- [x] `HostCommand`/`HostReply`/`HostError` が純データ型で、scan・instantiate・initialize・activate・start/stop processing・deactivate・destroy・save/restore state・open/close gui を表現できる
- [x] `InProcessHost` がコマンドを `Lifecycle` 状態機械と整合して駆動し、違法遷移は定義済みエラーとして reject される
- [x] RT 経路（`process`）が `Processing` 状態でのみプラグインへ到達し、allocation/ブロッキングなし（`rt_` テストで実測）
- [x] GUI 非対応プラグインでは open gui が `Unsupported` として定義済み拒否される
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス・doc 監査で本チェーン起因 findings 0・graph complete
