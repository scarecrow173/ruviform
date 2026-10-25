---
id: "DESIGN-034JZLISO0000000000003"
type: "design"
status: "approved"
title: "プラグイン分離設計（境界・プロセス監視・RT 面制約）"
created: "2026-10-21"
updated: "2026-10-21"
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
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "shared-foundations-and-modularization-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/plugin-isolation-design.md"
    - "docs/adr/plugin-isolation-strategy.md"
  derived-by:
    - "docs/plans/implement-issue-31-plugin-isolation.md"
    - "docs/designs/plugin-host-l0.md"
    - "docs/adr/ofx-video-plugin-scope.md"
    - "docs/designs/second-target-design.md"
    - "docs/discovery/plugin-l1-out-of-process.md"
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# プラグイン分離設計

## Overview

L0（インプロセス・既定）/L1（アウトプロセス・opt-in）の 2 層。境界は `ruviform-plugin-host` の既存公開面（インスタンスハンドル + 直列化可能コマンド）を維持し、呼び出し側は層を意識しない。

## 境界設計

- **PluginInstance ハンドル**: L0/L1 共通の不透明ハンドル（`ruviform-plugin-host` 新設時に定義 — 本設計が境界契約を規定）。L1 ではプロセス内プロキシ
- **コマンド境界**: パラメータ・ライフサイクル（activate/deactivate/reset）はメッセージ化。`process` のみ層で分岐:
  - L0: 直接 `Processor::process`（RT スレッドから呼出し可）
  - L1: 共有メモリリングで audio block を送受信 — 非 RT スレッドが駆動し、RT スレッドは結果を `try_recv` で読むのみ
- **クラッシュ検出**: L1 は子プロセス監視（wait/exit status）。クラッシュ時はインスタンスを `Failed` 状態へ遷移・ホストへ通知。再起動は呼び出し側判断（自動再起動はしない — 状態復元はプラグイン責務）

## RT 面制約

- L1 の `process` 等価経路は **非 RT**。RT スレッドが直接プロセス間通信を待機しない設計（ベースラインのスレッド構成と整合 — RT/非 RT 間は有界 SPSC のみ）
- L1 へ送ったブロックの遅延は「先行ブロック数 × ブロック長」で既定。クラッシュ・応答なし時は pass-through（直前の既受信データ or 無音）

## 実装単位（後続 Issue の分割）

- `ruviform-plugin-host` に L1 プロキシ実装（子プロセス spawn・監視・共有メモリ）
- L1 子プロセス側エントリは `ruviform-plugin-export` の薄いランナー

## Verification

- [x] ベースライン「インプロセス既定 + proxy 阻害しない境界」・plugin ADR と整合（`ruviform-plugin-host` は未実装のため、新設時の境界契約として本設計を規定）
- [x] RT 契約と整合（RT スレッドのブロッキング待機を持ち込まない）
- [x] 規格非依存（CLAP/VST3 アダプタ層は分岐に影響されない）
