---
id: "SPEC-034JROd7c0jBSZ1hnPEAg5"
type: "spec"
status: "approved"
title: "音声エンジン（pull グラフ・デバイスアダプタ統合・RT パス）"
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
  implemented-by:
    - "docs/designs/audio-engine.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/designs/audio-engine.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/discovery/engine-plugin-host-integration.md"
    - "docs/specs/engine-plugin-host-integration.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 音声エンジン（pull グラフ・デバイスアダプタ統合・RT パス）

## What Should Be Built

`ruviform-audio` エンジンを新設し、デバイスコールバック → プラグインチェーン → ミキサー → 出力の RT パスを実装する（Issue #25）。

- pull ベース処理グラフ（destination が必要分を上流へ要求 — ベースライン L310 採否済み）
- `ruviform-audio-core::device` のポート trait（`DeviceEnumerator`/`Device`/`OutputStream`）経由のデバイス統合 — engine↔adapter 直接依存なし（L193）
- RT パス: `process` 系は allocation/ブロッキングなし・事前確保バッファの `&mut` 借用引き回し（L323-355 データフロー例）
- グラフ再構成は不変スナップショット交換（非 RT 側で構築 → atomic 交換 — ADR `audio-engine-snapshot-swap`）
- `MasterClock` によるトランスポート時刻参照（読み取り専用・ノンブロッキング）

## Why Now

wave-2 順序表 #25。前提の #21（rt 基盤）・#23（デバイス境界）・#19（DSP PoC）は wave-1 で確定済み、#59（chunk timestamp）・#53（dispatch 層）は本 wave で実装済み（PR 待ち）。エンジン層の第一実装。

## Users and Value

- **再生ホスト/DAW 実装者**: デバイスコールバック配線済みの RT 音声パイプラインをそのまま利用できる
- **プラグインエコシステム**: `Processor` 契約に乗るノードをグラフに接続できる

## Scope

### In Scope

- 新クレート `crates/ruviform-audio`（workspace メンバー追加・crate doc に責務/依存層/RT 制約）
- pull グラフ: `Graph`（ノード列・`process` 呼出し規約）+ ミキサーノード（planar f32 和算）
- `AudioEngine::open_output_stream(device, config, graph)` 相当のセッション API — デバイス callback 内で `RtContext::mint` → グラフ駆動 → ミックス → out バッファ書込み
- スナップショット交換によるグラフ差替（非 RT 構築 → atomic swap → 次ブロックから適用）
- RT 契約: callback 内 no-alloc/no-block の静的検証 + `rt_` テスト（#22 レーン: thread-check 照会 + assert_no_alloc 窓）

## Requirements

- 依存: `ruviform-audio-core`（device ポート・バッファ型）・`ruviform-rt`（Processor/RtContext/MasterClock/SPSC）。`ruviform-audio-device` への依存は dev-dep（テスト）まで optional/禁止 — adapter 型を公開 API に露出しない
- callback 内で allocation・ブロッキング・OS 呼出しなし（#22 レーンで実測）
- グラフ差替は `prepare` 済み不変プランの atomic 交換 — RT 側は読み取り専用
- `unsafe` 不使用

## Acceptance Criteria

- [x] `ruviform-audio` クレートがワークスペースに存在し pull グラフ型が公開されている
- [x] モック OutputStream 経由でコールバック駆動の end-to-end 出力が得られる（テスト）
- [x] RT 経路で `assert_no_alloc` 窓内 0 allocation の `rt_` テストがグリーン
- [x] グラフ差替が実行中に panic/断続なく適用されるテストがグリーン
- [x] デバイス層の型が公開 API に露出しない（`ruviform-audio-device` 非依存）
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス

## Deferred Design Concerns

- A/V 同期・動画側エンジン（#27）| 本 Issue は音声パスのみ | #27 着手時 | 同期欠如
- パラメータオートメーション SPSC | 基盤の SPSC 機構は #21 で確定済み・適用規約は後続 | プラグイン GUI（#30）時 | 手動再構築のみ

## Sources and Relations

- 設計ベースライン L215（`ruviform-audio` 責務）・L310（pull 採否）・L323-355（RT データフロー例）・L186（engines 相互依存禁止）・L193（engine↔adapter 分離）
- `docs/adr/realtime-contract-and-verification-policy.md`・`docs/adr/rt-foundation-primitives-implementation.md`（#21 契約）
- `docs/adr/select-audio-device-backend.md`（#23 境界 trait）

## What Should Be Built

`ruviform-audio` エンジンを新設し、デバイスコールバック → プラグインチェーン → ミキサー → 出力の RT パスを実装する（Issue #25）。

- pull ベース処理グラフ（destination が必要分を上流へ要求 — ベースライン L310 採否済み）
- `ruviform-audio-core::device` のポート trait（`DeviceEnumerator`/`Device`/`OutputStream`）経由のデバイス統合 — engine↔adapter 直接依存なし（L193）
- RT パス: `process` 系は allocation/ブロッキングなし・事前確保バッファの `&mut` 借用引き回し（L323-355 データフロー例）
- グラフ再構成は不変スナップショット交換（非 RT 側で構築 → atomic 交換 — ADR `audio-engine-snapshot-swap`）
- `MasterClock` によるトランスポート時刻参照（読み取り専用・ノンブロッキング）

## Why Now

wave-2 順序表 #25。前提の #21（rt 基盤）・#23（デバイス境界）・#19（DSP PoC）は wave-1 で確定済み、#59（chunk timestamp）・#53（dispatch 層）は本 wave で実装済み（PR 待ち）。エンジン層の第一実装。

## Users and Value

- **再生ホスト/DAW 実装者**: デバイスコールバック配線済みの RT 音声パイプラインをそのまま利用できる
- **プラグインエコシステム**: `Processor` 契約に乗るノードをグラフに接続できる

## Scope

### In Scope

- 新クレート `crates/ruviform-audio`（workspace メンバー追加・crate doc に責務/依存層/RT 制約）
- pull グラフ: `Graph`（ノード列・`process` 呼出し規約）+ ミキサーノード（planar f32 和算）
- `AudioEngine::open_output_stream(device, config, graph)` 相当のセッション API — デバイス callback 内で `RtContext::mint` → グラフ駆動 → ミックス → out バッファ書込み
- スナップショット交換によるグラフ差替（非 RT 構築 → atomic swap → 次ブロックから適用）
- RT 契約: callback 内 no-alloc/no-block の静的検証 + `rt_` テスト（#22 レーン: thread-check 照会 + assert_no_alloc 窓）

## Requirements

- <!-- requirement -->

## Acceptance Criteria

- [x] 実装済み検証は plan の検証列と同一 — 全項目を参照

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- <!-- concern | reason | re-engagement trigger | risk if ignored -->

## Sources and Relations

<!-- Summarize source evidence, related ADRs, and upstream discovery artifacts. -->
