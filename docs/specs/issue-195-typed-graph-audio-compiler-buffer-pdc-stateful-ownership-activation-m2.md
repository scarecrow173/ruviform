---
id: "SPEC-034QfjN7Wt7AJBbhXeK60o"
type: "spec"
status: "approved"
title: "Issue #195 Typed Graph Audio Compiler・Prepared Plan・Buffer/PDC・Stateful RT Ownership・Plan Activation (Migration M2)"
created: "2026-11-01"
updated: "2026-11-01"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/synvelis/issues/23"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-195-m2-typed-graph-compiler-runtime-audit.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/185"
  deferred-by: []
---

# Issue #195 M2 Spec: Typed Graph Audio Compiler・Prepared Plan・RT Runtime

## Purpose

M1（Issue #184）で確定した `ruviform-graph` Typed IR を実行意味へ繋ぐ。`Typed Graph IR → 非 RT Audio Compiler → immutable Prepared Schedule → 単一 owner Stateful RT Runtime` の経路を `ruviform-audio` に additive に実装し、Issue #195 の受入 Gate G2-01〜G2-08 を実測証拠で満たす。旧単一 Bus `AudioNode`/`Graph`/`GraphPlan`/`PlanSwap`/`PluginEndpoint` shell は golden fixture 下で維持し、追加 API → 相互検証 → consumer 移行 → 旧 API removal 承認の順序を守る。

## What Should Be Built

- **非 RT Audio Compiler**（`ruviform-audio` 内・`ruviform-graph` を下向き参照）: `ValidatedGraph`+`StableHandleMap` を入力に deterministic schedule（topo 順 = `StableHandleMap` の runtime index 順）、port→buffer slot 割当、fan-out/fan-in binding、per-edge `ChannelMap` 適用、node latency 集計と merge 点 PDC（`D_i = max(L_j) - L_i`）、明示的な最大値と overflow の定義済み失敗を生成する。
- **Prepared Plan**: immutable schedule + slot layout + 境界 port（graph input/output）+ processor install 計画。prepare 時に最大 buffer 数・slot あたり最大 frames・alignment（≥64B、safe な over-allocate+offset）を確保する。
- **RT Runtime**: `Vec<RefCell<Slot>>` arena、alias 拒否（input slot ∩ output slot = ∅ を compile で検証し runtime は `RefCell` で二重 borrow を機械的に防止）、未初期化読取拒否（per-callback stamp 検査 → `RuntimeFault::UninitializedRead`）、可変 frames（`frames <= max_frames`、超過は失敗）、zero-frame は no-op、tail は valid prefix = frames。no-alloc/no-clone-heavy/no-lock(=no blocking lock)/no-block/no-OS-I/O。`&RtContext` capability gating。
- **Stateful Ownership PoC**: Plan-owned と Stable Runtime Slot を同一 Mock stateful DSP で比較（状態継続・移動・Activation/Deactivation・旧 generation event 遮断・error rollback・単一 RT owner）。実測に基づく選定を ADR に記録する。コード先行の既成事実化をしない。
- **Plan Activation**: `ruviform_rt::snapshot::slot` による publish/last-good read、generation 採番、spsc install/remove command、旧 generation event 遮断+計数、retire は `Retirer`→非 RT `DropCollector`、RT 側 destructor 禁止。
- **Fault injection**: rebuild 失敗・activation conflict・retirement queue full・collector stopped・shutdown を注入し、last-good 維持・fault 可観測性・RT destructor 禁止を実証。
- **Legacy golden path**: 旧 API の deterministic fixture 検証 + 既存 suite 不変。
- **Traceability**: EX-001〜EX-020・TA-001〜TA-012 を test→impl→evidence に対応付け、N/A・未検証を明示。

## Non-goals / Boundaries

- Plugin 規格 adapter の実 multi-bus 配線・実 plugin 接続（M4/#196）。TimedEvent/Control payload（M3/#188・#190）。Probe（#185）。Offline driver（#191）。
- Synvelis 固有の Track/Bus/Send Level 等の DAW 意味は generic engine に入れない。
- 旧 API の削除・plugin-host の audio への生産依存化・他リポジトリ変更・`unsafe` 導入・新しい RT 同期方式の並存新設。
- graph PDC と ADC/DAC・録音位置補正の混同（graph PDC は node latency 整列のみを担う）。

## Acceptance Criteria

- **AC-1 (G2-01)**: main+aux/sidechain を含む有効 multi-bus DAG が compile され、deterministic schedule と buffer mapping が生成される。fan-out は aliasing なしに共有 read され、意図しない mutation がない（同入力の 2 回連続 compile で byte 同一の plan dump を検証）。
- **AC-2 (G2-02)**: 無効な domain/direction/layout/multiplicity・未宣言 mix・zero-delay cycle・missing node/port・capability mismatch は非 RT で定義済み失敗として拒否され、incomplete plan は一切 publish されない。
- **AC-3 (G2-03)**: 非対称 branch latency が merge 点で整列される（実測 PCM で位相一致）。graph PDC が ADC/DAC・録音位置補正と二重計上されないこと、遅延量の明示最大値と overflow の定義済み失敗が検証される。
- **AC-4 (G2-04)**: 選定した ownership 契約のもと、公開 plan 更新前後で同一 NodeId の stateful processor の DSP 状態が継続し、同一 instance が新旧 plan から同時に process されず、旧 generation event が遮断されることが同一 Mock DSP で実測される。
- **AC-5 (G2-05)**: rebuild 失敗・activation conflict・retirement queue full・collector stopped・shutdown の各注入で last-good plan が維持され、RT 側 destructor が実行されず、fault が非 RT から観測できる。
- **AC-6 (G2-06)**: callback frames 64/128/256 と可変長列で process/buffer bounds と tail/zero-frame 規則が検証され、RT 区間の allocation/blocking/OS-I/O が `alloc_counter` 監査で 0 である。
- **AC-7 (G2-07)**: 旧 `AudioNode`/`Graph`/`GraphPlan`/`PlanSwap`/`PluginEndpoint` shell が golden fixture で動作し既存 test suite が不変で緑。
- **AC-8 (G2-08)**: EX-001〜EX-020・TA-001〜TA-012 の各 ID が test 名・impl 位置・evidence に対応付けられ、N/A・未検証が明示される。
- **AC-9**: 実行した検証コマンド（`mise run fmt-check`/`check`/`lint`/`test`/`test-rt`、`cargo test -p ruviform-audio --locked`、`cargo test --workspace --locked`）と結果・未検証項目が impl record に区別して記録される。最終検証前に `git fetch origin && git merge origin/main` と生成ファイル再生成を行う。
