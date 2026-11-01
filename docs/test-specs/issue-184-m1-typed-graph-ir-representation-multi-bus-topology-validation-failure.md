---
id: "TSPEC-034QeeLKxXQyMBzA6HNOW6"
type: "test-spec"
status: "approved"
title: "Issue #184 M1 typed graph IR: representation, multi-bus topology, validation failures, revisions, legacy coexistence"
created: "2026-11-01"
updated: "2026-11-02"
owners: ["scarecrow173"]
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
    - "docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md"
    - "docs/adr/typed-multi-bus-graph-ir-placement-and-port-connection-validation-model-issue-18.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #184 M1 typed graph IR: representation, multi-bus topology, validation failures, revisions, legacy coexistence

## Purpose

Issue #184 G1 判定の受入を、`ruviform-graph` PoC テスト `tests/poc_ex.rs` の ex_001〜ex_006 が保証することを記録する。M2 で IR が Compiler の入力になるまで本保証を維持する。IR を廃止・再設計する場合に退役可能。

## Feature

Typed multi-bus Graph IR の表現・構築・検証・revision/mapping 公開契約（全て非 RT）。

## Rules

- R-REP: Port は direction/domain(format 導出)/role/format/multiplicity/capability/availability/activation を独立属性として保持できる（Audio/TimedEvent/Control・Main/Aux/Sidechain/Send/Return を含む）。
- R-TOPO: Main→Processor→Output・Send 分岐・別入力 Sidechain・fan-in 合流を持つグラフは構築・検証を通過する。
- R-REJ: InvalidDirection/DomainMismatch/FormatMismatch/ImplicitConversionRequired/InvalidChannelMap/MissingNode/MissingPort/PortNotOnNode/CapacityExceeded/RequiredConnections/ZeroDelayCycle/DelayedCycleUnsupported は個別に識別可能な失敗として非 RT で拒否される。
- R-CONV: layout 相違は `ChannelMap`（dst 完全カバー・index 範囲内）宣言時のみ受理。未宣言は `ImplicitConversionRequired`。Audio 以外の domain で map 指定は `InvalidChannelMap`。
- R-REV: `GraphProject` が revision を採番し、requested（最終試行）と published（最終成功）を区別して照会できる。失敗時 published は従前を維持。
- R-MAP: `StableHandleMap` が NodeId/PortId/ConnectionId → deterministic runtime index（Kahn+BTreeMap tie-break topo）を双方向に引ける。
- R-COMPAT: 単一 Bus トポロジ（Main-only chain）が IR で表現・検証可能で、固定 expected-structure と一致する。既存スイートは不変で緑。

## Examples

- ex_001: 全属性の組合せで Port を宣言し `PortSpec` の属性が保持される。
- ex_002: Source(Main out)→Processor(Main in, Sidechain in, Main out, Send out)→Sink(Main in, Aux in) + Sidechain Source→Processor Sidechain、合流 Sink への fan-in 2 本で validate 成功。
- ex_003: Input→Input 接続、Audio→TimedEvent 接続、SR 不一致接続、layout 相違+map 無し接続、範囲外 index map の各拒否。
- ex_004: 不存在 node/port/owner 不整合の接続、zero-delay A→B→A、delayed edge 含む cycle、fan-in 超過、Always+min 未達の各拒否。
- ex_005: `prepare` 成功後 `mapping().node_index(id)` で runtime index を照会、`requested_revision()==published_revision()`。失敗 prepare 後は requested>published。
- ex_006: 失敗 spec の prepare で旧 published 維持。Main-only chain を構築し node/port/connection 数・端点・RuntimeNodeIndex 順が fixture と一致。

## Guarantees

- 上記 Examples が `mise exec -- cargo test -p ruviform-graph --test poc_ex` exit 0 で再現可能。
- `mise run test`/`test-rt` 緑（既存不変）。
- `cargo tree` で graph の通常依存が `audio-core`+`media-core` のみ。

## Non-goals

- 実 Buffer/スケジュール/PDC/RT 実行（M2）。plugin adapter 接続（M4）。Event/Control payload 検証（M3）。Probe 観測（#185）。serde/永続化。

## Risk

- 本保証を落とすと Issue #184 G1 (1)-(5) の受入証拠が消失し、M2 が検証済みでない IR 契約の上に実装を始める。
