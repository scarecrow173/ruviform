---
id: "SPEC-034QdNpdBY3gkeQraIPKwP"
type: "spec"
status: "approved"
title: "Issue #184 Typed multi-bus Port/Connection Graph IR contract (Migration M1)"
created: "2026-11-01"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/synvelis/issues/4"
    - "https://github.com/scarecrow173/synvelis/issues/18"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
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
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/specs/audio-engine.md"
    - "docs/designs/audio-engine.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-184-bus-port-graph-ir-crate.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/190"
  deferred-by: []
---

# Issue #184 Typed multi-bus Port/Connection Graph IR contract (Migration M1)

## What Should Be Built

Audio / Timed Event / Control 等の signal domain・direction・channel layout/map・bus role（Main/Aux/Sidechain 等）・multiplicity・capability/availability を独立に表現する**型付き Port / Bus / Connection の汎用 Graph IR**と、その**非 RT の構築・検証（Validator）・準備・公開契約**を設計し、最小 PoC で検証する。stable logical handle（Node/Port/Connection ID）と runtime handle を分離し、revision 区別（Project 要求 revision と公開済み graph revision）を照会可能にする。既存の単一 Bus `Graph` / `AudioNode` / `PluginEndpoint` 経路は additive な互換 Path として維持し、本 Issue では破壊しない。

成果物は「汎用 Graph 構築契約の設計正本（design）+ 構造 ADR + 検証可能な最小 PoC + EX-001〜006 相当の受入証拠」である。**実行時の Audio Compiler・Buffer 割当・PDC・Prepared Plan 実行・stateful processor 所有は M2 (#195)、plugin 実接続/Process Frame 精度は M4 (#196) の責務**であり、本 Issue はそれらを実装しない。

## Why Now

Synvelis Signal Flow/Routing Editor は Main Audio・Send/Return・Sidechain・MIDI/Event・Control を区別するポート接続、複数 Audio Bus、独立補助入力、Channel Mapping、処理順序変更、安全な Graph 更新を要求する。現行 `ruviform-audio` の `Graph` は `Vec<Box<dyn AudioNode>>` の順序実行・`AudioNode::pull` の単一出力 view 契約であり、`PluginEndpoint::process_audio` も単一 Bus shell で、これらを表現する語彙が存在しない（棚卸し: discovery 参照）。一方 `plugin-core` の `process(&[I], &mut [O])` は既に複数 Bus 形状であり、IR 側契約を先に確定すれば M2/M4 が確実な基盤の上で実装できる。目標アーキテクチャ（#192）と Migration 実行計画（#193）は M1 の着手を承認済みで、G0 baseline（#194/PR #200）は確定している。

## Users and Value

- **Synvelis（第一 Consumer）**: Signal Flow/Routing Editor が Track/Send 等の製品意味論を汎用 Port/Connection へ写像するための、domain/role/direction 付き接続契約と検証済み失敗モデルを得る（#4/#18）。
- **Migration M2 (#195) / M4 (#196) の実装 Owner**: IR（構造+検証）と Compiler（Buffer/PDC/Prepared Plan）の責務分離により、M1 の型付き契約を直接の入力として実装できる。
- **Probe (#185) 等の後続 Consumer**: stable logical ID と runtime handle の対応表により、実行中グラフの観測点を論理ポートへ逆引きできる基盤を得る。
- **既存 Ruviform 利用者**: 単一 Bus 経路の API・挙動・テストが不変で共存するため、移行を強制されない。

## Scope

### In Scope

- Port descriptor: direction（Input/Output）、signal domain（`Audio`/`TimedEvent`/`Control` を初期値とする non-exhaustive enum）、port/bus role（`Main`/`Aux`/`Sidechain` 等を初期値とする non-exhaustive enum）、channel layout/map（audio domain は既存 `ChannelLayout` 等を再利用）、multiplicity、capability、availability/activation を独立に表現する型付き契約。
- Node（declared ports を持つ論理単位）・Connection（source port → destination port の型付き edge）・Graph IR の構築 API。stable logical ID（NodeId/PortId/ConnectionId）を発行する境界。
- Connection Validator（全て非 RT）: direction 不整合、domain 不整合、format/layout 不整合、capacity/multiplicity 違反、fan-in/fan-out ポリシー違反、missing node/port、zero-delay cycle を**定義済みの失敗**として拒否する。暗黙の downmix/upmix・aux 自動生成・未承認変換は行わない。対応変換は明示的な graph 要素/宣言としてのみ受理する。
- 構築・検証・準備を全て非 RT で行い、成功した revision のみを公開対象とする契約。失敗時は旧 state を維持する（実行 Plan の publish/swap 機構そのものは既存 `GraphPlan`/`PlanSwap` を不変維持。M1 で公開するのは IR revision/検証済みスナップショットであり実行物ではない）。
- Stable logical ID → runtime handle の mapping 契約と `GraphRevision` の区別（Source Mapping 本体の保持は Consumer 側・Synvelis #18 項目に整合）。
- Project 要求 revision と公開済み graph revision を照会・区別できる条件の定義。
- Latency/merge-path 整列・delay compensation を capability/`Unsupported(reason)` として明示する契約（実装は M2）。
- Feedback/cycle の方針: zero-delay cycle は常に reject。delay 宣言を持つ経路の IR 表現可否と実行対応の `Unsupported` 明示を PoC で検証し記録する（安易な全許可/全拒否を既成事実化しない）。
- 単一 Bus レガシー（`Graph`/`GraphPlan`/`PlanSwap`/`Session`/`PluginEndpoint::process_audio` とその `rt_*` 回帰）の不変共存と移行方針の明文化。
- `ruviform-graph` 独立 crate vs 既存 crate 内配置の採否（依存方向・public API・他 Consumer・compile-time で比較、理由を ADR に記録）。
- 最小 PoC: 模擬信号で Main Audio→Processor→Output、Send 分岐（独立 bus）、別入力からの Sidechain、複数入力の合流を構成・検証する。上記 invalid 系を定義済み失敗として拒否することを含む。
- EX-001〜EX-006 相当の受入テストと実行したコマンド・実際の exit code の記録。

### Out of Scope

- 実行時 Audio Compiler、Buffer 割当/alias/lifetime、DAG scheduling、PDC/delay 実装、Prepared Plan の RT 実行、stateful processor の RT 所有権（全て M2 #195）。
- Plugin 規格アダプタ（CLAP/VST3/OFX）の実 multi-bus 宣言・sidechain 実配線・Process Frame 精度の改修（M4 #196。M1 は能力棚卸しと境界契約のみ）。
- MIDI/Timed Event・Control の実配送・Clock 同期・Note Ledger・Transport/Tempo 統合（#188/#190。IR の domain 表現と validator 経路のみ）。
- Probe 観測処理そのもの（#185。logical ID ↔ runtime handle の対応基盤のみ）。
- Offline render・Media Source/Reader（#191 系）。
- DAW 固有意味論: Track、Clip、Send Level、Pre/Post-Fader、Mixer、Automation Write、UI、Undo/Redo（Synvelis Project Model 側の写像責務）。
- 無条件の自動 channel 変換、全種類の feedback routing の保証。
- RT thread での UI コール・動的確保・OS I/O・待機（既存 RT 契約 ADR の維持対象であり拡張しない）。
- 既存 `Graph`/`AudioNode`/`PluginEndpoint`/`PlanSwap` API の破壊的変更または削除。

## Requirements

- R1: Port は direction・signal domain・role・channel layout/format・multiplicity・capability・availability を**独立した属性**として保持し、Sidechain は独立 domain ではなく Audio 等の補助入力 role として表現できる。
- R2: Node/Port/Connection の stable logical ID は compile/publish で不変とし、runtime handle（compile 産物の index/handle + `GraphRevision`）とは別物として対応表で管理する。
- R3: Validator は direction/domain/format・capacity・fan-in/fan-out・missing endpoint・zero-delay cycle を個別の定義済み失敗として拒否し、失敗は全て非 RT で確定する。暗黙変換・自動 downmix/upmix・aux 自動生成を行わない。
- R4: サポートする変換（例: 明示 `ChannelMap` 宣言）は graph 上の明示要素としてのみ受理し、未対応 format/layout 組合せは `Unsupported(reason)` として理由つきで明示する。
- R5: 構築・検証・準備は alloc/free・blocking・OS I/O・待機を伴う RT thread 外で行い、成功したものだけが公開対象になる。検証/準備失敗は従前状態を不変で維持する。
- R6: 公開 API は `#[non_exhaustive]` enum・`ruviform_media_core::Error` 系統・crate 規約（層 doc・許可依存・RT 契約記述）に従う。
- R7: 新 crate 採用時も workspace 通常依存 DAG に循環を導入せず、engine/host/device への依存を持たない Host/Device 非依存を構造上証明する（`cargo tree` 実測で確認）。
- R8: 単一 Bus レガシー経路の公開 API・`rt_*` テスト・golden 相当の挙動を不変で共存させる（additive）。
- R9: 未検証・未対応の形式・機能を実装済みとして扱わず、`Unsupported`/capability 不在として明示する。
- R10: RT endpoint の所有・退役は既存 #113/#25 契約（`Send` 転送・非 RT retire・leak-on-full 計数）に適合させ、新しい同期方式を後継 ADR なしに導入しない。

## Acceptance Criteria

- [ ] AC-1（棚卸し）: 現行 `Graph`/`AudioNode`/`PluginEndpoint`/plugin host/規格アダプタの複数 Bus・Sidechain 能力の棚卸しが `docs/discovery/issue-184-bus-port-graph-ir-crate.md` に実測根拠つきで記録されている。
- [ ] AC-2（型表現）: PoC の IR が `Audio`/`TimedEvent`/`Control` domain、`Input`/`Output` direction、`Main`/`Aux`/`Sidechain` role、channel layout、multiplicity、capability を型付きで表現し、Main+Aux/Sidechain と独立 port 群を持つノードを宣言できる。
- [ ] AC-3（検証拒否）: invalid direction、invalid domain、format/layout 不整合、missing node/port、zero-delay cycle、未宣言の暗黙変換が、それぞれ識別可能な定義済みエラーとして非 RT で拒否される（PoC テストで実証）。
- [ ] AC-4（模擬構成）: 模擬信号で Main Audio→Processor→Output・Send 分岐・別入力 Sidechain・複数入力合流を持つグラフが構築・検証を通過する（PoC テストで実証）。
- [ ] AC-5（ID/revision）: Node/Port の stable logical ID → runtime handle の mapping と `GraphRevision`（少なくとも project 要求 revision と公開済み revision の区別）が照会可能で、PoC で実証される。
- [ ] AC-6（互換）: 既存単一 Bus 経路の公開 API とテストが不変で全件緑（`mise run test` の既存スイート + `rt_` フィルタを含む）であり、legacy 共存が実測される。
- [ ] AC-7（依存 DAG）: 採用構成の通常依存 DAG が `cargo tree --locked` 実測で循環 0、新 crate 採用時は host/device/engine への依存を含まないことが記録される。
- [ ] AC-8（失敗時継続）: 構築/検証/準備失敗時に旧 state が維持されることが PoC で実証され、IR 構築・検証経路が RT thread の alloc/block/OS I/O を要求しないことがコード上確認できる（RT 実行 Plan 自体は M2 のため未検証として明示）。
- [ ] AC-9（受入記録）: EX-001〜EX-006 相当の受入テスト名・実行コマンド（`mise run ...`）・実際の exit code が design/plan/完了報告に記録される。未実行・未対応項目は理由つきで区別される。
- [ ] AC-10（crate 採否記録）: `ruviform-graph` 独立 crate vs 既存 crate 内配置の比較（依存方向・public API・他 Consumer・compile-time）と採否理由が ADR に記録される。

## Deferred Design Concerns

- NonRT Audio Compiler（DAG scheduling・Buffer alias/lifetime・PDC・Prepared Plan・stateful RT ownership）| 責務分割: IR の入力を実行物に変換するのは別責務 | M2 #195 着手時 | IR と Compiler の責務が混在し検証境界が曖昧化
- Plugin 規格アダプタの実 multi-bus 宣言・sidechain 実配線・Process Frame 精度 | M4 #196 の責務、規格実機検証を要する | M4 着手時 | IR Port→`HostInput`/`HostOutput` 写像の前提が未検証のまま残る
- Timed Event 配送・Clock 同期・Transport/Tempo・Note Ledger | #188/#190 の責務 | M3 着手時 | `TimedEvent`/`Control` domain の実 payload が未確定
- Probe 観測（tap/ring/観測点） | #185 の責務 | #185 着手時 | mapping の観測利用が未検証
- Graph revision の永続化・復元・Undo 連携 | 製品側 Project Model の責務 | Consumer 要求発生時 | 過剰な永続化設計を IR に混入させる
- Latency report/PDC の数値契約・Feedback cycle の実行意味論 | M2 Compiler 検証対象 | M2 着手時 | 能力未実装のまま capability 宣言のみ残る

## Sources and Relations

- 正本 Issue: [ruviform#184](https://github.com/scarecrow173/ruviform/issues/184)（設計検討範囲 1-7・最小 PoC/完了条件・G1 判定・実行境界）。
- 実行計画・Gate: [ruviform#193](https://github.com/scarecrow173/ruviform/issues/193)（M1 位置付け）・[ruviform#194](https://github.com/scarecrow173/ruviform/issues/194) / [M0 baseline audit](../discovery/issue-194-m0-baseline-audit.md)（G0 確定証拠）。
- 上位設計: [ruviform#192](https://github.com/scarecrow173/ruviform/issues/192) / [synvelis-driven-target-architecture-and-migration](../designs/synvelis-driven-target-architecture-and-migration.md)（crate 候補・Graph 分離の方針）。
- 要件元: [synvelis#4](https://github.com/scarecrow173/synvelis/issues/4)（Signal Flow Port/Connection 概念）・[synvelis#18](https://github.com/scarecrow173/synvelis/issues/18)（3 層構造・12 項目・EX-001〜006）。
- 棚卸し正本: [issue-184-bus-port-graph-ir-crate](../discovery/issue-184-bus-port-graph-ir-crate.md)（`derives-from`）。
- 関連境界: [engine-plugin-host-integration](engine-plugin-host-integration.md)（単一 Bus shell・deferred 項目）・[audio-engine](audio-engine.md)・Accepted ADR（Snapshot Swap / Plugin RT Ownership / RT Contract / non-exhaustive enum）。
