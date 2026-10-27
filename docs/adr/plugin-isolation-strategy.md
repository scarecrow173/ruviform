---
id: "ADR-034JZLISO0000000000002"
type: "adr"
status: "accepted"
title: "プラグイン分離戦略（インプロセス既定 + opt-in アウトプロセス）"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/plugin-isolation-design.md"
    - "docs/adr/plugin-architecture-clap-vst3.md"
  derived-by:
    - "docs/designs/plugin-isolation.md"
    - "docs/designs/plugin-host-l0.md"
    - "docs/adr/ofx-video-plugin-scope.md"
    - "docs/discovery/plugin-l1-out-of-process.md"
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/discovery/engine-plugin-host-integration.md"
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/discovery/issue-114-real-adapter-l1-e2e.md"
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# プラグイン分離戦略

## Context

プラグイン（特に外部 VST3/CLAP）は未検証コードであり、クラッシュはホスト全体を巻き込む。一方で音声処理の `process` は RT スレッド上で呼ばれ、プロセス境界越えはレイテンシ・スケジューリング上の重大制約となる。ベースラインは「インプロセス既定 + 分離を阻害しない境界」を確定済み。

## Decision

**階層化分離**: L0 = インプロセス（既定・RT フル対応）、L1 = アウトプロセス（opt-in・非 RT 処理面のみ）。L1 はプラグインを子プロセスで実行し、`process` 呼出しは共有メモリ SPSC リング経由。RT スレッドは `try` ベースで応答を取得し、未完了/クラッシュ時は pass-through（無音）で継続 — RT 契約を維持する。

## Considered Options

- **Option A — 階層化（インプロセス既定 + opt-in アウトプロセス）採用**: 信頼プラグインは L0 で RT フル性能、未信頼は L1 でクラッシュ隔離。RT 面の制約は API 契約で明示
- **Option B — 全プラグイン アウトプロセス**: 完全なクラッシュ隔離だが RT `process` をプロセス境界越しに毎ブロック呼ぶため、共有メモリ設計でもレイテンシ・スケジューリング上の固有コストが残り、既定として不適
- **Option C — インプロセスのみ（分離なし）**: 実装最小だが未検証プラグインのクラッシュでホスト巻き添え。要件のクラッシュ隔離を満たさない

## Consequences

- `ruviform-plugin-host` のインスタンス境界は L0/L1 共通のハンドル + コマンド直列化面とする（既存 API 面を変えない）
- L1 の `process` 等価経路は「非 RT 準同期」— RT パスには直接接続せず、エンジン側は非 RT スレッドから駆動（ベースラインの非 RT 制御スレッド層）
- L1 実装は後続実装 Issue。本 Issue では境界設計のみ確定

## Verification

- [x] 既存 `ruviform-plugin-host` API 面との整合（ハンドル + コマンド境界の前提を壊さない）
- [x] RT 契約文書（realtime-contract）と整合（アウトプロセス呼出しを RT スレッドから直接行わない）
