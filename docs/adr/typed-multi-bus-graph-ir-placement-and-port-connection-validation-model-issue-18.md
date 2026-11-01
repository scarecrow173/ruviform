---
id: "ADR-034QdPgLCqwTPQ4FHtjLsj"
type: "adr"
status: "accepted"
title: "Typed multi-bus Graph IR placement and Port/Connection validation model (Issue #184 M1)"
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
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/specs/issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md"
    - "docs/specs/engine-plugin-host-integration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-184-bus-port-graph-ir-crate.md"
    - "docs/specs/issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/185"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Typed multi-bus Graph IR placement and Port/Connection validation model (Issue #184 M1)

## Context and Problem Statement

Issue #184（Migration M1）は Synvelis Signal Flow/Routing Editor が要求する Main/Aux/Sidechain・複数 Audio Bus・独立補助入力・Channel Mapping・安全な Graph 更新を支える**型付き Port/Bus/Connection の汎用 Graph IR と非 RT Validator**の契約を定義する。棚卸し（discovery）で確定したとおり、現行 `ruviform-audio` の `Graph`/`AudioNode::pull`/`PluginEndpoint::process_audio` は末尾まで単一 Bus であり、Port/Domain/Role の語彙がない。一方 `plugin-core` の `process(&[I], &mut [O])` は複数 Bus 形状済みで、IR を先に確定すれば M2 (#195) の Audio Compiler/Buffer/PDC/Prepared Plan と M4 (#196) の Plugin adapter がその上に構築できる。

本 ADR で確定するのは **(a) IR の crate 配置、(b) Port/Connection の型モデル、(c) 検証・変換・cycle 方針、(d) stable ID/runtime handle/revision 境界、(e) 互換方針**である。実行時 Compiler・Buffer 割当・PDC・Prepared Plan・stateful processor 所有は M2、plugin 実接続は M4、既存 Accepted ADR（Snapshot Swap / Plugin RT Ownership / RT Contract / non-exhaustive enum）の改変・supersede は本 ADR の対象外とし、衝突が生じた場合のみ別途後継 ADR を起こす。

## Decision Drivers

* Issue #184 G1 判定: (1) Domain/Direction/Channel Layout/Multiplicity/Capability の型付き表現、(2) invalid direction/domain/layout・missing node・zero-delay cycle・暗黙変換の非 RT 拒否、(3) Stable ID→runtime handle mapping と version、(4) 単一 Bus legacy goldens の共存、(5) EX-001〜006 受入と実 exit code。
* `synvelis-driven-generic-execution-boundaries` ADR Decision 3: Graph IR と Audio Compiler の責務分離、IR が Host/Device を instantiate せず単独検証可能であること。独立 `ruviform-graph` crate 採否は PoC に委ねる、とされており本 ADR がその判断を行う。
* RT 契約: 構築・検証・準備は全て非 RT。IR 層は `ruviform-rt` の RT callback 型に依存しない。
* 公開 API 規約: `#[non_exhaustive]` enum・`ruviform_media_core::Error` 系統・crate 層 doc（許可依存・RT 契約記述）・`publish.workspace`/`lints.workspace` 継承。
* 依存 DAG: 通常依存に循環を作らず、`media-core`/`rt` 等 foundation の変更影響範囲を拡大しない。
* Synvelis #4: Sidechain は独立 domain ではなく Audio の補助入力として表現できる必要がある。

## Considered Options

### Option A: 独立 `ruviform-graph` crate

* 新 crate `crates/ruviform-graph` に IR（Node/Port/Connection/Validator/Revision/mapping）を置く。通常依存は `ruviform-audio-core`（`ChannelLayout` 等の audio domain 値型）と `ruviform-media-core`（`Error` 系統）のみ。`ruviform-rt`/`ruviform-audio`/`ruviform-plugin-*`/device 系には依存しない。
* Good: ADR の責務分離を Cargo 境界で強制できる。Host/Device 非依存が `cargo tree` で構造証明できる。`cargo check -p ruviform-graph` で IR 単体を高速検証できる。M2 Compiler が `ruviform-audio` 側から IR を参照する自然な依存方向（audio → graph）を作れる。Video/CLI 等他 Consumer が audio engine を引かずに IR を利用できる。
* Bad: workspace に crate が 1 つ増える管理コスト。PoC 規模では crate 粒度が細かく見える可能性。

### Option B: `ruviform-audio` 内モジュール

* `ruviform_audio::typed_graph` 等として既存 engine crate 内に IR を置く。
* Good: crate 追加なし。既存 `Graph` と近い。
* Bad: IR が engine crate に閉じ、汎用 IR を使う全 Consumer が audio engine 全体を引く。engine 側から host/device への不要結合を Cargo で防げない。M2 の「IR と Compiler は別責務」を構造上保証できない。

### Option C: `ruviform-media-core`/`ruviform-audio-core` に併設

* 値型 foundation crate に IR を併設する。
* Good: foundation 層から広く参照できる。
* Bad: 時間/形式値型の foundation に「構造グラフの編集・検証」という別責務を混在させる。foundation の変更影響範囲を不用意に拡大する。

### Option D: `ruviform-plugin-core`/`ruviform-plugin-host` 側に配置

* plugin 接合面に近い側へ IR を置く。
* Good: endpoint と近い。
* Bad: Host は IR の Consumer 側であり依存方向が逆になる。engine/host 双方向禁止の既存 ADR に抵触する。

## Decision Outcome

Chosen option: **Option A — 独立 `ruviform-graph` crate**。

理由: 責務分離（IR ≠ Audio Compiler）を Cargo 境界で強制でき、Host/Device 非依存を依存グラフで証明できる唯一の配置であり、M2/M4 が参照する自然な依存方向と整合する。`ruviform-audio` 等 Consumer 側からの参照は `audio → graph` の下向き依存として追加し、逆方向は作らない。比較の詳細（依存方向・public API・他 Consumer・compile-time の4軸）は discovery の Alternatives 表に記録済み。

併せて以下を決定する（本 ADR の構造決定）:

1. **Port モデル:** Port descriptor は direction（`Input`/`Output`）・signal domain（`Audio`/`TimedEvent`/`Control` を初期値 — `PortFormat` から導出する独立に参照可能な属性で、format と domain の二重持ちはしない）・role（`Main`/`Aux`/`Sidechain`/`Send`/`Return` を初期値）・format（audio は `ruviform_audio_core` の `ChannelLayout`/`AudioFormat` を再利用、event/control は初期 payload なし marker）・multiplicity（`min`/`max`、fan-in/fan-out 容量と必須接続数を担う）・capability・availability・activation を**独立属性**として持つ。Sidechain は role であり domain ではない。全ての公開 enum は `#[non_exhaustive]`。
2. **検証方針:** Validator は (a) connection direction 不整合、(b) domain 不整合、(c) format/layout 不整合、(d) capacity/multiplicity 違反、(e) fan-in/fan-out ポリシー違反、(f) missing node/port、(g) zero-delay cycle を**個別に識別可能な定義済み失敗**として非 RT で拒否する。暗黙の downmix/upmix・aux 自動生成・未承認変換はしない。対応変換は graph 上の明示要素（`ChannelMap`/変換宣言）としてのみ受理する。未対応の format/layout 組合せは `Unsupported(reason)` で理由つき明示する。
3. **Cycle 方針:** zero-delay cycle は常に reject。`delay` を宣言した edge は IR で表現可能とするが、M1 の validator は delayed cycle を `Unsupported(reason)`（実行未対応）として明示的に扱い、全許可/全拒否を既成事実化しない。実行対応は M2 Compiler が決める。
4. **ID/Revision 境界:** IR は `NodeId`/`PortId`/`ConnectionId` の stable logical ID を発行し、compile/publish で不変とする。runtime handle（compile 産物の index/handle）は別物で、stable ID → runtime handle の対応表に `GraphRevision` を持たせ、project 要求 revision と公開済み revision を区別して照会可能にする。
5. **互換方針:** 既存 `Graph`/`AudioNode`/`PluginEndpoint`/`GraphPlan`/`PlanSwap`/`Session` とその `rt_*` 回帰は不変で共存（additive）。M1 は実行 Plan の publish/swap 機構を新設せず、IR revision/検証済みスナップショットの公開のみを扱う。RT endpoint 所有・退役は既存 #113/#25 契約に従い、新しい同期方式は導入しない。

### Consequences

* Good, because IR が Host/Device/Device adapter なしに単独で構築・検証・テスト可能になり、TA-003 を `cargo tree` で構造証明できる。
* Good, because `plugin-core` の multi-bus 形状と整合し、M4 の IR Port→`HostInput`/`HostOutput` adapter が IR 側契約の変更不要で設計できる。
* Good, because 単一 Bus 経路を破壊せず段階移行でき、Issue #184 の「既存互換維持」と「新契約追加」を同時に満たす。
* Bad, because crate が 1 つ増え workspace 管理対象が増える（許容: DAG 概念図と整合する既知のコスト）。
* Bad, because `ruviform-graph → ruviform-audio-core` の依存により audio 型（`ChannelLayout`）の変更が IR に伝播する（許容: audio-core は検証済み値型 foundation で、非 audio domain は marker 型のため伝播しない）。

### Confirmation

* `cargo tree --locked -p ruviform-graph` で通常依存が `ruviform-audio-core`+`ruviform-media-core`（+外部 crate）のみ、engine/host/device への依存 0、workspace 循環 0 を実測する。
* `mise run check`/`lint`/`test`/`test-rt` 緑（既存スイート不変共存）+ PoC 受入テスト（EX-001〜006 相当）の実行コマンドと実 exit code を記録する。
* design 文書が本 ADR に `derives-from` で連結され、spec/ADR/design 鎖が graph gate を通過する。

## Implementation Plan

* Affected paths: `crates/ruviform-graph/`（新規: `Cargo.toml`、`src/lib.rs`+IR モジュール）、workspace `Cargo.toml` `members`、必要なら `crates/README.md` 索引。既存 `crates/ruviform-audio`/`ruviform-plugin-*` は M1 では変更しない。
* Patterns to follow: 各 crate の crate-level doc に層・許可依存・RT 契約を記述。公開 enum は `#[non_exhaustive]`。エラーは crate-local domain error enum（`ValidationError`/`PrepareError` 等）で `Display`+`core::error::Error` を実装する — `ruviform_media_core::Error` は `Copy`/`&'static str` payload のみで `ConnectionId`/`Vec<NodeId>` を保持できないため、`SessionError`/`HostError` と同じ per-crate enum 規約に従う。`[lints] workspace = true`、`publish.workspace = true` 継承。ID は deterministic な発生器（builder が採番）で数値 newtype。
* Patterns to avoid: IR 層から `ruviform-rt`/`ruviform-audio`/plugin/device crate への依存追加。IR 型への `Box<dyn AudioNode>` 埋め込み。暗黙変換や aux 自動生成の hidden 副作用。DAW 用語（Track/Clip/Send Level/Mixer/Undo）の命名流入。`unsafe`。
* Dependencies or configuration: `ruviform-audio-core`（`ChannelLayout`/`AudioFormat`/`SampleFormat`/`SampleRate`）、`ruviform-media-core`（`TimeBase`/`Fact` 等 — `SampleRate` が `TimeBase` を包含するため直接依存として必要）。追加外部依存は原則なし（必要になれば ADR 追記）。
* Migration: additive のみ。既存 API・テスト・Cargo.lock への影響は新 crate 追加分のみ。`Cargo.lock` は `cargo generate-lockfile`（mise 経由）で再生成。

## Verification

* [ ] `cargo tree --locked -p ruviform-graph --edges normal` で ruviform 内依存が `audio-core`+`media-core` のみ、workspace 全体で循環 0。
* [ ] `mise run check` / `mise run lint` / `mise run test` / `mise run test-rt` を実行し結果と exit code を記録（既存スイートの不変共存を実測）。
* [ ] PoC テストで: 型表現（3 domain×direction×role×layout×multiplicity×capability）、模擬構成（Main→Processor→Output・Send 分岐・Sidechain・合流）、invalid 系拒否（direction/domain/layout/missing/zero-delay cycle/暗黙変換）、ID→handle mapping+revision、失敗時旧 state 維持を実証。
* [ ] Validator の失敗が全て非 RT で確定し、IR 構築・検証コード経路に `RtContext`/alloc-required RT API を要求しないことをコードレビューで確認。
* [ ] design 文書の `relations.derives-from` に本 ADR と spec が記録され、`route_graph.js` の briefing/design gate が pass する。

## Pros and Cons of the Options

### Option A: 独立 `ruviform-graph` crate

* Good, because 責務境界・依存方向・単独検証可能性を Cargo で強制できる最も強い形。
* Good, because M2 以降の Consumer（audio compiler、plugin adapter、video/CLI graph 利用）が必要最小依存で IR を利用できる。
* Bad, because workspace 管理コストと小規模期の crate 粒度の細かさ。

### Option B: `ruviform-audio` 内モジュール

* Good, because 新 crate なしで最小の diff。
* Bad, because IR の汎用利用者が audio engine に引きずられ、責務分離が曖昧になる。

### Option C: foundation crate 併設

* Good, because 依存の浅い位置に置ける。
* Bad, because 値型 foundation に構造編集・検証の責務が混在する。

### Option D: plugin 側配置

* Good, because endpoint との距離が近い。
* Bad, because 依存方向が逆で双方向禁止 ADR に抵触。

## More Information

* 棚卸し正本: [discovery: issue-184-bus-port-graph-ir-crate](../discovery/issue-184-bus-port-graph-ir-crate.md)。
* 要求仕様: [spec: issue-184 typed multi-bus graph IR contract](../specs/issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md)。
* 上位境界: [synvelis-driven-generic-execution-boundaries](synvelis-driven-generic-execution-boundaries.md) Decision 3/5/7・[target architecture](../designs/synvelis-driven-target-architecture-and-migration.md) §4/§5.1。
* 維持対象の既存 ADR: [audio-engine-snapshot-swap](audio-engine-snapshot-swap.md)・[plugin-rt-ownership-split](plugin-rt-ownership-split.md)・[realtime-contract-and-verification-policy](realtime-contract-and-verification-policy.md)・[public-enum-non-exhaustive-policy](public-enum-non-exhaustive-policy.md)。本 ADR はこれらを supersede しない。
