---
id: "DESIGN-034QdReVJaFxlQTjmr5oy5"
type: "design"
status: "approved"
title: "Issue #184 Typed Graph IR: Ports, Buses, Connections, Validation, Revisions (M1)"
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
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/discovery/issue-184-bus-port-graph-ir-crate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/designs/audio-engine.md"
    - "docs/specs/audio-engine.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md"
    - "docs/adr/typed-multi-bus-graph-ir-placement-and-port-connection-validation-model-issue-18.md"
    - "docs/discovery/issue-184-bus-port-graph-ir-crate.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/185"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/190"
  deferred-by: []
---

# Issue #184 Typed Graph IR: Ports, Buses, Connections, Validation, Revisions (M1)

## Context

Issue #184（Migration M1）の設計正本。spec は「型付き Port/Bus/Connection + 非 RT 構築・検証・準備・公開契約 + stable ID/revision + additive 互換」を要求し、ADR は「独立 `ruviform-graph` crate・直交 Port 属性・定義済み失敗・暗黙変換禁止・zero-delay cycle 常時拒否・delayed edge は IR 表現可/実行 Unsupported・stable ID と runtime handle の分離・既存 PlanSwap 不変維持」を決定した。本稿はそれを実装可能な API 形状・型・モジュール境界・検証アルゴリズム・エラー分類・PoC 構成へ落とす。

問題境界: 現行 engine は `Vec<Box<dyn AudioNode>>` の順序実行・`AudioNode::pull(out, ctx)` の単一出力契約で、Port/Domain/Role の語彙がない。複数 Bus 形状は `plugin-core::process(&[I], &mut [O])` と `HostInput`/`HostOutput` にのみ存在する（discovery 確定済み）。IR はこの多 bus 形状が**最終的に接続される構造記述**であり、M1 では実 Buffer/スケジュール/実行物を持たない。

## Scope

### In Scope

- 新 crate `crates/ruviform-graph`: 通常依存 `ruviform-audio-core` + `ruviform-media-core` のみ。
- 型: `NodeId`/`PortId`/`ConnectionId`/`GraphRevision`/`RuntimeNodeIndex`/`RuntimePortIndex`/`RuntimeConnectionIndex`（stable logical ↔ runtime handle 分離）、`SignalDomain`/`Direction`/`PortRole`/`PortFormat`/`Multiplicity`/`PortCapability`/`Availability`/`Activation`、`NodeDef`/`PortDef`/`NodeSpec`/`PortSpec`/`Connection`/`EdgeDelay`/`ChannelMap`、`GraphSpec`（builder 経由のみ構築可能）、`Validator`→`ValidatedGraph`、`GraphProject`（revision 発行権・requested/published 区別・prepare 失敗時の旧 published 維持）、`StableHandleMap`。
- Validator: direction/domain/layout/capacity/fan-in-fan-out/missing/zero-delay cycle/delayed cycle/implicit-conversion/channel-map 妥当性/owner 不整合の定義済み失敗。
- 単一 Bus トポロジの IR 表現（固定 expected-structure fixture による検証）。
- PoC テスト `ex_001`〜`ex_006`（Issue #184 G1 (1)-(5) の写像）。
- workspace `Cargo.toml` の `members` 追記、`crates/README.md` 索引（計 28→29）と crate 個別 `README.md`、`Cargo.lock` 再生成。

### Out of Scope

- Audio Compiler・Buffer 割当/alias/lifetime・DAG scheduling・PDC/delay 実装・Prepared Plan・RT 実行・stateful processor 所有（M2 #195）。
- plugin 規格アダプタの実 multi-bus 宣言・sidechain 配線・Process Frame 精度（M4 #196）。
- Timed Event/Control の payload・配送・Clock/Transport（#188/#190）。
- Probe 実装（#185）、Offline（#191）。
- 既存メンバークレートの manifest・API・テストの変更（root `Cargo.toml` の `members` 追記は除く）。
- `ruviform-architecture-design-baseline.md`（approved baseline）の改変 — 構造層の層位置は本稿と target-arch 文書に記録する。

## Components and Boundaries

### crate `ruviform-graph`（新規・engine/host/device 非依存）

```
crates/ruviform-graph/
  Cargo.toml            # deps: ruviform-audio-core, ruviform-media-core; lints/publish workspace 継承
  README.md             # 責務/層・許可依存/RT契約（クレート規約）
  src/lib.rs            # crate doc(層/許可依存/RT契約) + re-export
  src/ids.rs            # stable ID + runtime handle + revision
  src/port.rs           # Port descriptor 属性群
  src/graph.rs          # NodeDef/PortDef/NodeSpec/PortSpec/Connection/GraphSpec/GraphBuilder
  src/validate.rs       # ValidationError / ValidationErrors / Validator / ValidatedGraph
  src/project.rs        # GraphProject (revision 発行, requested/published, prepare)
  src/mapping.rs        # StableHandleMap
  tests/poc_ex.rs       # ex_001..ex_006 受入
```

層位置: foundations（`audio-core`/`media-core`）と engine（`ruviform-audio`）の間の**構造層**。`ruviform-rt`/`ruviform-audio`/`ruviform-plugin-*`/device crate への依存を持たない（TA-003 の構造証明）。M2 は `ruviform-audio`（または同等 consumer 側）が `graph` を下向き参照して IR→実行 Plan を compile する。

### ID と revision の発行権

- `GraphBuilder` が `NodeId`/`PortId`/`ConnectionId` を内部 counter で採番して返す（deterministic、`u64` 1 起点）。caller は `NodeDef`/`PortDef`（ID 無しの宣言型）を渡し、採番済み ID を受け取る。`GraphSpec`/`NodeSpec`/`PortSpec`/`Connection` のフィールドは crate 内構築（builder のみが完成形を作る）とし、caller が任意 ID を混入できない形にする（duplicate ID は構築不能 → 検証 variant 不要）。
- `GraphSpec` は revision を持たない。**`GraphProject` が唯一の revision 発行権**を持ち、`prepare()` 呼出し毎に内部 counter を +1 して `GraphRevision` を採番する（同一 project 内で単調増加、1 起点）。builder 間・project 間の revision 比較は契約外。

```rust
// ids.rs
pub struct NodeId(pub u64);
pub struct PortId(pub u64);           // graph 内で一意（builder 採番）
pub struct ConnectionId(pub u64);
pub struct GraphRevision(pub u64);    // GraphProject が採番、project 内単調増加
// runtime handle（ValidatedGraph 内の deterministic index。M1 では実行 slot ではなく写像 placeholder）
pub struct RuntimeNodeIndex(pub u32);
pub struct RuntimePortIndex(pub u32);        // graph 内 global index
pub struct RuntimeConnectionIndex(pub u32);
```

### Port モデル（直交属性、全公開 enum `#[non_exhaustive]`）

```rust
// port.rs
pub enum Direction { Input, Output }
pub enum SignalDomain { Audio, TimedEvent, Control }
pub enum PortRole { Main, Aux, Sidechain, Send, Return }
pub enum PortFormat {
    Audio(AudioFormat),    // ruviform_audio_core::AudioFormat を再利用 { sample_rate, layout: ChannelLayout, sample_format }
    TimedEvent,            // marker — payload は M3/#188 で確定
    Control,               // marker
}
impl PortFormat { pub fn domain(&self) -> SignalDomain { .. } }  // domain は format から導出（二重持ち禁止）
pub struct Multiplicity { pub min: u16, pub max: u16 }
//   規約: min <= max 必須（`Multiplicity::new` が `Err` で検証）。
//   `Multiplicity::UNBOUNDED_MAX == u16::MAX` = 無制限。`max` = port が受け付ける接続数の上限
//   （input なら fan-in、output なら fan-out の容量 — fan-in/fan-out ポリシーは port ごとの
//   Multiplicity が担う）。`min` = `Availability::Always` の port が validate 時に要求する
//   最小接続数（Optional は未達可）。
//   presets: `ONE` = {min:1,max:1}, `ONE_OR_NONE` = {0,1}, `UNBOUNDED` = {0,UNBOUNDED_MAX}。
pub enum PortCapability { LatencyReporting, DelayCompensation, ProbePoint }
pub enum Availability { Always, Optional }   // Always: min 接続数を要求。Optional: min 未達を許容
pub enum Activation { Enabled, Disabled }    // Disabled: 宣言は残すが接続要求を免除（runtime active 状態は consumer 側）
pub struct PortDef {                          // builder への入力（ID 無し）
    pub direction: Direction, pub role: PortRole, pub format: PortFormat,
    pub multiplicity: Multiplicity, pub capabilities: BTreeSet<PortCapability>,
    pub availability: Availability, pub activation: Activation, pub label: Option<String>,
}
pub struct PortSpec { pub id: PortId, /* + PortDef の全属性（domain() は format から導出 getter） */ }
// 明示変換のみ。audio→audio で layout が異なる接続に必須
pub struct ChannelMap { pub entries: Vec<(u16,u16)> }  // (src_ch, dst_ch)。全 dst を一意に覆う完全写像
```

`PortRole` の意味規約（generic、DAW 意味なし）: `Main`=主要信号経路、`Aux`=補助入力、`Sidechain`=別経路参照入力、`Send`=分岐 tap、`Return`=分岐戻り。`Send`/`Return` は role 名であり DAW の Send Level/Pre-Post-Fader 意味を持たない。

```rust
// graph.rs
pub struct NodeDef { pub kind: NodeKind, pub label: Option<String> }   // builder への入力
pub enum NodeKind { Processor, Source, Sink }
//   NodeKind と port direction の整合は validator が検査しない（Source が input を持つ等は IR 上許容。
//   kind は分類情報であり構造制約を持たない — consumer 側の解釈に委ねる）。
pub struct NodeSpec { pub id: NodeId, pub kind: NodeKind, pub ports: Vec<PortSpec>, pub label: Option<String> }
pub struct PortRef { pub node: NodeId, pub port: PortId }  // 両方必須。owner 不整合は PortNotOnNode で拒否
pub enum EdgeDelay { Zero, Declared }   // Declared: 遅延要素の存在宣言（量は M2 契約。variant 形状の将来拡張余地を残す）
pub struct Connection {
    pub id: ConnectionId, pub from: PortRef, pub to: PortRef,
    pub channel_map: Option<ChannelMap>, pub delay: EdgeDelay,
}
pub struct GraphSpec { /* private fields: nodes, connections */ }
pub struct GraphBuilder {
    // add_node(NodeDef) -> NodeId; add_port(NodeId, PortDef) -> Result<PortId, BuildError>;
    // connect(PortRef, PortRef, {channel_map, delay}) -> Result<ConnectionId, BuildError>;
    // finish() -> GraphSpec
}
```

### Validator（`validate.rs` — 全て非 RT）

`Validator::validate(&GraphSpec) -> Result<ValidatedGraph, ValidationErrors>`。全件収集（件数は edge/node 数に線形で bounded）。

```rust
#[non_exhaustive]
pub enum ValidationError {
    MissingNode { connection: ConnectionId, node: NodeId },
    MissingPort { connection: ConnectionId, port: PortId },
    PortNotOnNode { connection: ConnectionId, node: NodeId, port: PortId },
    InvalidDirection { connection: ConnectionId, from: Direction, to: Direction },
    DomainMismatch { connection: ConnectionId, from: SignalDomain, to: SignalDomain },
    FormatMismatch { connection: ConnectionId, reason: &'static str },
    ImplicitConversionRequired { connection: ConnectionId, reason: &'static str },
    InvalidChannelMap { connection: ConnectionId, reason: &'static str },
    CapacityExceeded { port: PortId, direction: Direction, have: u16, max: u16 },
    RequiredConnections { port: PortId, have: u16, min: u16 },
    ZeroDelayCycle { nodes: Vec<NodeId> },
    DelayedCycleUnsupported { nodes: Vec<NodeId>, reason: &'static str },
}
```

検査規則（順序: 1→7。同一接続で複数ルールが落ちる場合は各 variant ごとに記録し、重複報告は許容）:

1. **端点存在・owner 一致:** `from`/`to` の NodeId が spec 内に存在しなければ `MissingNode`。PortId が存在しなければ `MissingPort`。PortId は存在するが `PortRef.node` に宣言されていなければ `PortNotOnNode`。
2. **方向:** `from` の Port が `Direction::Output`、`to` が `Direction::Input` → 違反は `InvalidDirection`。
3. **domain 一致:** 両端の `format.domain()` 不一致 → `DomainMismatch`。
4. **format/layout（Audio-Audio のみ。Event/Control は marker 同士で構造 OK）:** `sample_rate`/`sample_format` 相違 → `FormatMismatch`（M1 は変換要素を提供しない=Unsupported の M1 表面）。SR/sample 相違がある接続は以降の layout/map 検査を行わない（precedence: SR/sample > layout > map）。SR/sample 一致で `layout` 相違 + `channel_map` 無し → `ImplicitConversionRequired`。`channel_map` が存在する接続は layout 一致の有無に関わらず 5 へ。
5. **ChannelMap 妥当性:** (a) Audio 以外の domain で `channel_map` 指定 → `InvalidChannelMap`。(b) 各 `src_ch < from.layout.channels()`、`dst_ch < to.layout.channels()`。(c) `dst_ch` は重複不可・`0..dst.channels()` を完全に覆う（部分 map・空 map は拒否 — unmapped channel の暗黙 silence を防ぐ）。(d) src の重複は明示的複製として許容。(e) layout が等しい場合の `channel_map` も同規則で受理（明示 permutation）。
6. **capacity/fan-in-fan-out:** port への接続数（input なら fan-in、output なら fan-out）が `Multiplicity.max` 超過 → `CapacityExceeded`。`Availability::Always` かつ `Activation::Enabled` の port で接続数 < `min` → `RequiredConnections`。
7. **cycle（全グラフ対象の SCC — 全 validated graph は DAG になる）:** `Connection` の `delay` 属性に関係なく強連結成分を算出。(a) SCC 内の全 edge が `EdgeDelay::Zero` の非自明 SCC（自己 loop 含む）→ `ZeroDelayCycle`。(b) それ以外の非自明 SCC（`Declared` edge を 1 本以上含む cycle）→ `DelayedCycleUnsupported{reason: "delayed feedback cycle requires M2 execution semantics"}`。IR 上は delayed edge を表現できるが、cycle を閉じる構造は M1 では実行未対応として定義済み失敗で拒否する（安易な全許可/全拒否を取らない ADR 方針の実装）。

`ValidatedGraph`: 検証通過 spec の immutable snapshot（`Arc`）+ `StableHandleMap` + capability 境界 API。

### capability 境界 API（`ValidatedGraph`）

- `port_latency(&self, port: PortId) -> LatencyStatus`: `enum LatencyStatus { Unsupported(&'static str) }`。M1 は常に `Unsupported("latency computation is an M2 compiler responsibility")` — latency/合流整列/PDC を「能力/未対応」として明示し、黙って 0 を仮定しない（spec R9）。
- `has_capability(port, capability) -> bool`: 宣言の有無（`ProbePoint` 等、#185 が逆引きする情報の保持）。

### StableHandleMap（`mapping.rs`）

`NodeId→RuntimeNodeIndex`、`PortId→RuntimePortIndex`、`ConnectionId→RuntimeConnectionIndex` の対応表。`RuntimeNodeIndex` は **Kahn 法（`BTreeMap<NodeId>` 順の tie-break）による topological order の位置**とする。ValidatedGraph は常に DAG（rule 7）なので topo は必ず存在し決定的。M2 はこの index を Buffer/Plan slot に延長する（M1 では実行 slot ではなく mapping placeholder と明記 — M2 への非拘束的意図）。accessor: `node_index(NodeId) -> Option<RuntimeNodeIndex>`、`port_index`、`connection_index`、逆引き `node_at(RuntimeNodeIndex) -> Option<NodeId>`。

### GraphProject（`project.rs` — prepare/publish 契約）

```rust
pub struct GraphProject {
    requested: Option<GraphRevision>,          // 最後の prepare 試行に採番した revision
    published: Option<PublishedGraph>,          // { revision, snapshot: Arc<ValidatedGraph>, map: StableHandleMap }
    next_revision: u64,                         // 0 起点、prepare 毎に +1 → GraphRevision
}
impl GraphProject {
    pub fn requested_revision(&self) -> Option<GraphRevision>;
    pub fn published_revision(&self) -> Option<GraphRevision>;
    pub fn published(&self) -> Option<&ValidatedGraph>;
    pub fn mapping(&self) -> Option<&StableHandleMap>;
    pub fn prepare(&mut self, spec: GraphSpec) -> Result<GraphRevision, PrepareError>;
}
#[non_exhaustive] pub enum PrepareError { Validation(ValidationErrors) }
```

- `prepare` は内部 counter を +1 して `revision` を採番し `requested` に記録してから `Validator::validate` を実行。**成功時のみ** `published` を `{revision, snapshot, map}` で差替え、`revision` を返す。失敗時は `requested` のみ進み `published` は従前を維持（AC-5 区別・AC-8 維持の実装）。spec に revision は無く、古い spec の後出し publish という問題は存在しない（prepare 呼出し順がそのまま revision 順）。
- discovery で不採用とした「新しい publish/retire 機構の並行新設」との関係を明記: 禁止対象は **RT 面向けの新同期方式**（`RwLock`+`try_read`+retire の代替）であり、`GraphProject` は `&mut self` の non-RT コンテナで lock・retire・RT reader を持たない同期機構ではない。`PlanSwap` の再利用は `ruviform-graph → ruviform-audio` 依存を要し依存方向違反のため不可 — これが独立容器を置く理由。M1 で「公開」するのは検証済み IR snapshot であり実行 Plan ではない。typed plan の RT 適用と退役は M2 が後継 ADR と共に設計する。

## Data and Control Flow

構築→検証→公開（全て非 RT）:

```
Consumer(Synvelis Project lowering 等)
  → GraphBuilder::add_node(NodeDef{kind,label}) -> NodeId
  → GraphBuilder::add_port(node, PortDef{direction,role,format,multiplicity,capabilities,availability,activation,label}) -> Result<PortId, BuildError>
  → GraphBuilder::connect(PortRef{node,port}, PortRef{node,port}, ConnDef{channel_map?,delay}) -> Result<ConnectionId, BuildError>
  → GraphBuilder::finish() -> GraphSpec
  → GraphProject::prepare(spec)
      ├─ next_revision += 1 → requested = Some(rev)
      ├─ Validator::validate(&spec)
      │    ├─ Err(ValidationErrors) → published 不変、Err(PrepareError::Validation)
      │    └─ Ok(ValidatedGraph{snapshot, StableHandleMap})
      └─ published ← Some(PublishedGraph{rev, snapshot, map}) → Ok(rev)
  → Query: requested_revision() / published_revision() / published() / mapping()
  → (M2 以降) ruviform-audio 側 compiler が ValidatedGraph を入力に Buffer/PDC/Prepared Plan を生成
```

## Non-Functional Constraints

- **RT 契約:** 構築・検証・`prepare` は alloc 可能な non-RT API。`RtContext`・RT callback・`ruviform-rt` 型への依存なし。crate doc に「本 crate は non-RT 専用。RT thread からの呼出しは契約外」を明記。
- **依存:** `ruviform-audio-core`+`ruviform-media-core` のみ（`cargo tree` 実測で循環 0・engine/host/device 依存 0）。追加外部依存なし。
- **API 規約:** 全公開 enum `#[non_exhaustive]`（exhaustive_enums lint 方針の例外として public-enum-non-exhaustive ADR に従う）。エラーは crate-local domain error enum（`ValidationError`/`ValidationErrors`/`PrepareError`/`BuildError`）で `#[non_exhaustive]` + `Display` + `core::error::Error` 実装 — `media_core::Error` は `Copy`/`&'static str` payload で `ConnectionId`/`Vec<NodeId>` を保持できないため採用せず、`SessionError`/`HostError` と同じ per-crate enum 規約に従う。`[lints] workspace = true` 準拠（`unsafe_code` 等）。`unsafe` なし。
- **決定性:** ID 採番・runtime index 割当（Kahn + `BTreeMap` tie-break）・validation エラー収集順（edge 宣言順→node 順）・`BTreeMap`/`BTreeSet` のみ使用。時刻/乱数/不安定順序に依存しない。
- **互換:** 既存メンバークレートの manifest・API・テスト不変。root `Cargo.toml` `members` 追記と `Cargo.lock` 再生成のみ。
- **`Unsupported(reason)` との対応:** spec R4/ADR 句 2 の「`Unsupported(reason)`」は、M1 では **`FormatMismatch`/`ImplicitConversionRequired`/`InvalidChannelMap`/`DelayedCycleUnsupported` の各 variant が `reason` payload を持つ**形で表面化する（variant 名で分岐可能 + 理由文字列）。capability 系は `LatencyStatus::Unsupported(&'static str)` が担う。

## Risks and Trade-offs

- **IR の未使用化リスク:** M2 が IR を採用しないと crate が宙に浮く → ADR で採用確定 + M2 #195 が IR を入力と明記。consumer 接続を急がないのは責務分離の意図的維持。
- **role 意味の DAW 化:** `Send`/`Return` 命名 → role は「経路構造」のみを表し Level/Fader 意味を持たないことを crate doc と spec で明記。
- **TimedEvent/Control の marker 化:** payload 未確定 domain を構造一致のみで通すのは弱い検証 → marker と宣言し M3 で payload/format 拡張する経路を `#[non_exhaustive]` で確保。
- **delayed cycle の扱い:** IR 表現を許しつつ `DelayedCycleUnsupported` で拒否 — Issue の「全許可/全拒否を既成事実化しない」要求への実装。全グラフが DAG になるため runtime index は常に topo 可能。
- **ValidationErrors の全件収集:** edge/node 数に線形で bounded。件数 cap は設けない（YAGNI）。
- **PortRef{node,port} の冗長性:** `PortId` は global 一意なので `node` は冗長だが、明示的 owner 宣言により `PortNotOnNode` で早期誤り検出が可能（曖昧さより明示を取る選択）。
- **activation の M1 意味:** `Disabled` は「接続要求を免除する宣言済み port」のみを意味し、runtime enable/disable の動的制御は consumer 側状態として deferred。

## Deferred Design Concerns

- IR→Prepared Plan の compile・Buffer 割当/alias・PDC・topo schedule・stateful owner | M2 #195 責務 | M2 着手 | 実行意味論が未検証のまま残る
- IR Port→`HostInput`/`HostOutput` adapter・規格 bus 宣言写像 | M4 #196 責務 | M4 着手 | plugin 実接続が未検証
- TimedEvent/Control payload・event list・clock | M3/#188/#190 | M3 着手 | marker format が暫定のまま残る
- Probe tap/観測点・逆引き API | #185 | #185 着手 | mapping の観測利用が未検証
- `ChannelMap` の N→M 任意行列変換・重み付け | PoC は dst 完全カバーの index 写像のみ | Consumer 要求時 | 複雑変換を早期確定すると未検証仕様が残る
- GraphSpec の serde/永続化・Project Snapshot 復元 | 製品側 Project Model 責務 | 要求発生時 | IR に persistence 設計を混入させる
- Fan-in merge semantics（合流時の mix 規約） | M2 buffer/processing 契約 | M2 着手 | 合流が構造的に許されても mix 意味が未確定
- runtime activation（動的 enable/disable） | consumer 側状態・M2 以降 | 要求発生時 | IR が製品状態機械を所有すると境界違反

## Verification Notes

- [ ] `mise exec -- cargo tree --locked -p ruviform-graph --edges normal` で依存が `audio-core`+`media-core` のみ・循環 0（実測を記録）。
- [ ] `mise run check`/`lint`/`test`/`test-rt` 緑・既存スイート不変（exit code 記録）。
- [ ] `mise exec -- cargo test -p ruviform-graph --test poc_ex` で ex_001〜ex_006 緑（exit code 記録）:
  - ex_001: 3 domain × direction × role(Main/Aux/Sidechain/Send/Return) × layout × multiplicity × capability × availability/activation の型表現
  - ex_002: Main→Processor→Output + Send 分岐 + Sidechain 別入力 + 合流（fan-in multiplicity）の構築・検証通過
  - ex_003: InvalidDirection/DomainMismatch/FormatMismatch/ImplicitConversionRequired/InvalidChannelMap の個別拒否
  - ex_004: MissingNode/MissingPort/PortNotOnNode・ZeroDelayCycle・DelayedCycleUnsupported・CapacityExceeded/RequiredConnections の拒否
  - ex_005: stable ID→runtime index mapping・requested vs published revision 区別
  - ex_006: (a) 検証失敗時に旧 published 維持、(b) 単一 Bus トポロジ（Main-only chain Source→Processor→Sink）の IR 構築・検証通過 + 固定 expected-structure（node/port/connection 数・端点 PortId・RuntimeNodeIndex 順）との一致を assert
- [ ] Validator 実装に `RtContext`/`ruviform-rt` 参照・unsafe がないことを `rg` で確認。
- [ ] design gate: `route_graph.js --current design` が design→planning edge を返す。
