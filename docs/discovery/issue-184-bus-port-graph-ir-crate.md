---
id: "DISC-034QdFl4ZIWzDfKWh0Cg90"
type: "discovery"
status: "active"
title: "Issue #184 型付き複数Bus/Port・Graph IRの現行実装棚卸しとcrate配置比較"
created: "2026-11-01"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/synvelis/issues/4"
    - "https://github.com/scarecrow173/synvelis/issues/18"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
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
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/designs/audio-engine.md"
    - "docs/specs/audio-engine.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md"
    - "docs/adr/typed-multi-bus-graph-ir-placement-and-port-connection-validation-model-issue-18.md"
  verifies: []
  verified-by: []
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/185"
  deferred-by: []
---

# Issue #184 型付き複数Bus/Port・Graph IRの現行実装棚卸しとcrate配置比較

## Summary

Issue #184（Migration M1）の G1 判定に向け、現行 Graph/AudioNode/PluginEndpoint/規格アダプタの複数 Bus・Sidechain 能力をコード・テスト・M0 監査に基づき棚卸しした。**確認結果: エンジン層は末尾まで単一 Bus（`Graph`=順序リスト・`process_audio`=1in/1out変換）であり、複数 Bus 形状は `ruviform-plugin-core` の `process(&[I], &mut [O])` と `HostInput`/`HostOutput` にのみ存在する。** Typed Graph IR は Device/Host 非依存である必要から、独立 `ruviform-graph` crate（依存: `ruviform-audio-core` + `ruviform-media-core` のみ）が支持される — 最終判断は spec/ADR と G1 PoC で確定する。

## Exploration Goal

Issue #184 の第1完了条件「現行 Graph/PluginHost/規格アダプタの複数 Bus/Sidechain 能力と不足をコード/テストに基づき棚卸しする」と、G0-H が要求する「独立 `ruviform-graph` crate vs 既存 crate 内配置の採否を依存方向・公開 API・他 Consumer・compile-time で比較し採否理由を記録」を、M0 baseline（#194・PR #200）確定後の本 worktree HEAD で確認すること。設計・実装の確定は本稿の範囲外で、spec/ADR/PoC に引き継ぐ。

## Key Issues and Assumptions

実コード確認（本 worktree `scarecrow173/rf184-typed-graph` @ HEAD、根拠は `## Evidence`）:

- `ruviform-audio::graph::Graph` は `Vec<Box<dyn AudioNode>>` の順序実行リスト。`AudioNode::pull(&mut self, out: &mut AudioBufferViewMut<f32>, ctx: &RtContext)` は**単一の mutable 出力 view のみ**を取り、Port/Bus/方向・複数入力・aux/sidechain の語彙を一切持たない。
- `Mixer` は「呼出し側が事前に充填した `inputs: Vec<Vec<f32>>` を `out` へ和算する」末端ノードであり、複数入力合流の構造的 Port 表現ではない（入力 plane は Port ではなく内部 scratch）。
- `GraphPlan`（`Mutex<Graph>`）/`PlanSwap`（`RwLock<Arc<GraphPlan>>` + RT `try_read` + last-good cache + `retire_channel` 非 RT 退役）は Issue #113/#25 契約どおり動作し、`rt_` 系 no-alloc テストが存在する。**この公開・退役機構は Typed Graph にも再利用可能な既存資産**であり、M1 では改変しない。
- `PluginEndpoint::process_audio(out, events, transport, ctx)` は単一 Bus の変換 shell（`out`→事前確保 `in_scratch`→1 `HostInput`/1 `HostOutput`）。bound clip は `out.channels()==bound.channels && out.frames()<=bound.max_frames` で、`Err` 時 `out` 不変。
- 対照的に `plugin-core` の `PluginProcessor::process<I,O>(inputs: &[I], outputs: &mut [O], events, transport, context)` と host 側 `process(&[HostInput], &mut [HostOutput], ...)` は**既に複数 Bus 形状**（bus 順序の erased view 列）。「複数 Bus・plugin chain routing」は Issue #113 design で明示的に deferred と記録済み。
- `PluginEndpoint` の `EndpointBounds`/`channels`/`max_frames`/`latency_frames`/`EventList`/`TransportInfo` は存在するが、**バス名・ロール・方向の宣言的ポート記述はない**。`prepare_endpoint` は 1 インスタンス 1 endpoint・epoch CAS、退役は非 RT 契約済み。
- `EventList`（plugin-core）は frame offset 昇順の検証済みイベント列だが `EventKind` は `ParamValue` 中心 — Timed MIDI/Note Ledger は #188/#190 領域で M1 範囲外。
- 規格アダプタ（`ruviform-plugin-clap`/`vst3`/`ofx`）は `PluginProcessor` を implement する側であり、CLAP audio-ports の multi-bus 宣言や VST3 aux/sidechain の実配線は M4 (#196) の棚卸し対象。M1 が必要とするのは「アダプタが既に erased multi-bus view を受ける口を持つ」ことの確認であり、これは `HostInput`/`HostOutput` で確認済み。
- DAG（本 worktree で `cargo tree --locked` 実測）: 通常依存は循環 0。`ruviform-audio → {rt, audio-core, media-core}`、`plugin-host → {audio-core, media-core, plugin-core, observability, rt, video-core}` で `ruviform-audio` への通常依存はなく dev-only（M0 と一致）。`media-core`・`rt` は ruviform 内依存 0 の foundation。
- 規約: 公開 enum は `#[non_exhaustive]`（ADR `public-enum-non-exhaustive-policy`）、エラーは `media_core::Error` 系統、crate は `publish.workspace`/`lints.workspace` 継承、crate-level doc に層・許可依存・RT 契約を記す。

## Alternatives and Comparison

G0-H が要求する配置比較（依存方向・公開 API・他 Consumer・compile-time の4軸）:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **A: 独立 `ruviform-graph` crate**（依存 `audio-core`+`media-core` のみ） | ADR 責務分離（IR≠Audio Compiler）を Cargo 境界で強制・Host/Device 非依存を構造証明・`cargo check -p` で IR 単体検証可・M2 Compiler が audio 側から IR を参照する自然な依存方向・Video/CLI 等他 Consumer が engine を引かず利用可 | workspace+1 crate の管理コスト・小規模時は crate 粒度が細かい | **有力**（target design§4「抽出有力」・DAG 概念図と整合） |
| B: `ruviform-audio` 内モジュール | crate 追加なし・既存 Graph と近接 | IR が engine crate に閉じる=「Device/Host 非依存の汎用 IR」を利用する全 Consumer が audio engine を引く・engine 側からの不要結合を Cargo で防げない・M2 の「IR と Compiler の責務分離」が曖昧化 | 不採用寄り |
| C: `ruvimedia-core`/`audio-core` に併設 | foundation 層で広く参照可 | 時間/形式値型の crate に構造グラフ編集・検証の責務を混在させる・foundation の変更影響範囲を不用意に拡大 | 不採用 |
| D: `ruviform-plugin-host`/`plugin-core` 側 | endpoint と近い | Host は IR の Consumer 側であり依存方向が逆・engine/host 双方向禁止 ADR に抵触 | 不採用 |

IR 表現の粒度（spec 論点の事前比較）:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Domain=`Audio|TimedEvent|Control` + Role=`Main|Aux|Sidechain|…` を独立直交属性 | Synvelis #4「Sidechain は独立 Domain ではなく Audio の補助入力」に一致・domain 拡張を non_exhaustive enum で段階追加可 | 直交属性の組合せ正当性を validator が担う | 有力 |
| 単一 enum で `AudioMain|AudioAux|Sidechain|Midi|Control` を平坦列挙 | 実装最小 | domain/role 混同で将来の domain 追加が破壊的・#4 の要求と不一致 | 不採用 |
| Audio のみ実装・他 domain は型上の予約のみ | M1 PoC を Audio に絞れる（Issue 範囲外の許容） | `TimedEvent`/`Control` の検証経路を M1 で全く通せない | **G1 要件(1)が Domain 表現を要求するため非選択**（表現は3 domain、検証強度は Audio 中心で段階化） |

公開・退役経路（M1 が「Graph 構築失敗時に旧 Plan 継続」をどう証明するか）:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 既存 `PlanSwap`/`GraphPlan`/`RtPlan` をそのまま公開機構として再利用 | 既存 Accepted ADR・`rt_` no-alloc 証拠・退役経路を全て継承・改変ゼロ | Typed IR 自体は RT 実行されないため「公開される物」は M1 では IR revision/検証済みスナップショット（実行 Plan は M2） | **M1 ではこれ**（実行 Plan の swap は M2 #195） |
| M1 で新しい publish/retire 機構を並行新設 | 型付き Plan を直接公開可 | Snapshot Swap ADR の後継 ADR なしに新同期方式を導入することになり禁止・M2 との重複 | 不採用 |

## Tentative Conclusions and Hypotheses

- **Hypothesis (有力):** Typed Graph IR を新規 `ruviform-graph` crate に配置。依存は `ruviform-audio-core`（`ChannelLayout` 等の実証済み audio domain 型を再利用）+ `ruviform-media-core`（`Error` 統一）。`ruviform-rt`/`audio`/`plugin-*`/`device` への依存を持たない → DAG 循環なし・TA-003「Device/Host なしで検証可能」を Cargo で証明。
- **Confirmed:** `plugin-core` process 契約と host `HostInput`/`HostOutput` は複数 Bus 対応形状済み。M1→M4 で必要なのは IR Port→`HostInput`/`HostOutput` 写像の Adapter であり、plugin ABI 契約の破壊は不要。
- **Confirmed:** エンジン層の複数 Bus 実処理に必要な Buffer/Endpoint 契約は存在しない（AudioNode が単一 `out` のみ）。M1 では「Node が型付き Port 群を宣言する IR 契約」を設計し、実 Buffer 割当/スケジュールは M2 に defer と明記。
- **Hypothesis:** IR は `SignalDomain`(Audio/TimedEvent/Control)・`Direction`・`PortRole`・`ChannelLayout/format`・`Multiplicity`・`Capability`/`Availability` を直交属性として持ち、Validation は direction/domain/format/capacity/fan-in/fan-out/zero-delay cycle を非 RT で定義済み失敗として拒否。暗黙の Downmix/Upmix・aux 自動生成・未承認変換はしない（明示変換 = 宣言済み Conversion/ChannelMap のみ）。
- **Hypothesis:** Stable ID → runtime handle の対応は「IR の `NodeId`/`PortId`/`ConnectionId`（論理・安定）」と「compile 産物の実行 index/Handle + `GraphRevision`」を分離し、revision 照会（Project/Compiled/Active 区別の基盤）は mapping テーブルに `revision` を持たせることで実現（Source Mapping 本体は Consumer 側・Synvelis #18 項目1/11 に整合）。
- **Confirmed:** 「Graph 構築失敗/更新失敗時に旧 Plan を継続し RT に alloc/block/OS I/O を持ち込まない」は、M1 では「validate/prepare は全て非 RT・公開は成功時のみ」という IR 側契約 + 既存 PlanSwap の非改変継続で証明可能。新しい RT 実行物は M1 で公開しない（M2 領域）。
- **Confirmed:** 単一 Bus レガシー経路（`Graph`/`GraphPlan`/`PlanSwap`/`Session`/`process_audio` shell とその `rt_` 回帰）は不変で共存可能 — IR は追加 API であり既存 API の破壊を伴わない（RB0: Additive）。

## Open Questions

- IR の `PortFormat`/`Capability`/`Multiplicity` の最終フィールド構成と命名（spec/design で確定）。
- Channel mapping の明示表現（`ChannelMap` の型: 順序置換のみか任意 N→M 写像か）— 初期 PoC は同一 layout or 明示 map 宣言に限定し、任意行列変換は `Unsupported` 明記か。
- Delayed edge を持つ cycle を IR で「宣言可能・検証で受理・実行 compile は Unsupported」とするか「宣言そのものを初期は拒否」とするか — Issue 本文は「安易に全許可/全拒否を既成事実化しない」、Synvelis #18 は「初期 zero-delay feedback は Reject」。両者に整合するのは「cycle は常に reject、ただし edge に delay 宣言を持つ経路は IR で表現可能とし実行対応を `Unsupported(reason)` として明示」か — spec で確定。
- `TimedEvent`/`Control` port の PoC 検証深度（Audio domain 完備 + Event/Control は型表現と validator 経路の通しまでか）。
- probe 境界（#185）への参照面: IR の `PortId`/runtime handle が probe の観測点 id と一致するための最小契約（ID の安定性保証のみか、probe capability フラグまで持つか）。

## Promotion Candidates

- [x] spec-doc needed: 型付き Port/Bus/Connection・GraphSpec・Validator・prepare/publish・Stable ID mapping・revision・互換方針の要求仕様（Issue #184 の設計検討範囲 1-7 と最小 PoC/完了条件の製品側受入を formalize）。
- [x] adr-doc needed: `ruviform-graph` 独立 crate 採否の記録 + Port モデル（直交 Domain/Role・暗黙変換禁止・cycle/delay 方針・Stable ID/Revision 境界）の構造決定。既存 ADR の改変なし・新規 ADR として追加。

## Sources and Relations

- 主 Issue: [ruviform#184](https://github.com/scarecrow173/ruviform/issues/184)。上位: [#192](https://github.com/scarecrow173/ruviform/issues/192)（target arch）・[#193](https://github.com/scarecrow173/ruviform/issues/193)（migration plan・G1 判定基準）。実行境界: [#195](https://github.com/scarecrow173/ruviform/issues/195)（M2 Compiler/RT ownership）・[#196](https://github.com/scarecrow173/ruviform/issues/196)（M4 plugin adapter）・[#185](https://github.com/scarecrow173/ruviform/issues/185)（Probe）。
- 要件元: [synvelis#4](https://github.com/scarecrow173/synvelis/issues/4)（Signal Flow Port/Connection 概念）・[synvelis#18](https://github.com/scarecrow173/synvelis/issues/18)（EX-001〜006・12 項目）。
- Baseline 証拠: [issue-194-m0-baseline-audit](issue-194-m0-baseline-audit.md)（DAG 67 edges/循環0・dev 5・plugin-host→audio dev-only・テスト 1784）。
- 設計正本: [synvelis-driven-target-architecture-and-migration](../designs/synvelis-driven-target-architecture-and-migration.md) §4（crate 候補表）・§5.1（NonRT Graph）・[synvelis-driven-generic-execution-boundaries](../adr/synvelis-driven-generic-execution-boundaries.md) Decision 3（Graph separation）・5（Plugin composition）・7（Incremental migration）。

## Evidence

### 2026-11-02: 現行実装棚卸し（worktree `scarecrow173/rf184-typed-graph`）

**環境:** Windows 11 / Git Bash / mise 管理 rust 1.99.0（`mise exec --` 経由）。

#### エンジン層（単一 Bus 確定）

- `crates/ruviform-audio/src/graph.rs:16-19` — `trait AudioNode: Send { fn pull(&mut self, out: &mut AudioBufferViewMut<'_, f32>, ctx: &RtContext); }`。**唯一の mutable 出力 view**、入力 bus/ports/direction/role の語彙なし。`:30-37` — `FnMut(&mut AudioBufferViewMut, &RtContext) + Send` blanket impl（closure=node の既存経路）。`:43-79` — `Graph { nodes: Vec<Box<dyn AudioNode>> }`、`push`/`pull` は順序実行のみ。
- `crates/ruviform-audio/src/mixer.rs:16-64` — `Mixer { inputs: Vec<Vec<f32>> }`、caller が事前充填した内部 plane を `out` へ加算。Port 表現ではない。
- `crates/ruviform-audio/src/plan.rs` — `GraphPlan{Mutex<Graph>}`、`PlanSwap{RwLock<Arc<GraphPlan>> + DropCollector}`、`RtPlan{try_read + cached + Retirer}`、leak-on-full + `retired_overflows` 計数、非 RT drain。Issue #113 T5 の Send/Sync static_assertions と `rt_*_no_alloc` テストが `lib.rs` tests に実在。
- `crates/ruviform-audio/src/session.rs` — `Session` が `OutputStream` 所有・callback 内 `RtContext::mint()`+`plan.drive()`。Session 自身は port/bus を知らない。

#### Plugin 層（複数 Bus 形状は契約済み、単一 Bus shell が現行 graph 接合面）

- `crates/ruviform-plugin-core/src/processor.rs:459-504` — `PluginProcessor::process<I,O>(inputs: &[I], outputs: &mut [O], events, transport, context)`、doc: 「`inputs`/`outputs` hold one borrowed view per enabled bus, in bus order」。`BufferView`/`BufferViewMut` trait は 1 view=1 bus。
- `crates/ruviform-plugin-core/src/plugin.rs:25-61` — `HostInput(&dyn BufferView)`/`HostOutput(&mut dyn BufferViewMut)` erased bus view。`:101-110` — `process_host(&[HostInput], &mut [HostOutput], ...)`。
- `crates/ruviform-plugin-host/src/endpoint.rs` — `PluginEndpoint::process_audio(&mut AudioBufferViewMut<f32>, EventList, Option<TransportInfo>, &RtContext)` が graph 接合面: bound clip（`channels` 一致 + `frames<=max_frames`、Err で `out` 不変）→ `in_scratch` へ channel-major copy → `inner.process(bound, &inputs[1], &mut outputs[1], ...)`。sidechain/aux/multi-bus の語彙なし（grep 一致 0）。
- `docs/designs/engine-plugin-host-integration.md` — deferred 行「multiple buses・plugin chain routing/auto-connect | v1 graph stays an ordered list · process_audio is a 1-bus mapping」、spec 範囲外「plugin チェーンの複雑なルーティング/自動接続」。
- `tests/plugin_graph_e2e.rs` — `Send` `PluginEndpoint` を closure 経由 `Graph::push`、endpoint Send 証拠・`rt_*_no_alloc`・retire 非 RT の実在テスト。

#### DAG・規約（実測）

- `cargo tree --locked --workspace --edges normal`（本 worktree）: `ruviform-audio → {ruviform-rt, ruviform-audio-core, ruviform-media-core}`。`ruviform-plugin-host → {audio-core, media-core, plugin-core, observability, rt, video-core}`（`ruviform-audio` への通常依存なし = dev-only、M0 記録と一致）。`ruviform-media-core`・`ruviform-rt` は ruviform 内依存 0。通常依存全体: M0 記録の 67 edges/循環 0 維持（plugin-host→audio は `[dev-dependencies]` のみ）。
- `audio-core` 型: `ChannelLayout`（Mono/Stereo/Quad/5.1/7.1/Custom(SpeakerMaskSet)、`#[non_exhaustive]`、`channels()` は mask から導出）・`AudioFormat{sample_rate, layout, sample_format}`・`SampleFormat`（F32 default、`#[non_exhaustive]`）・`AudioBufferView(Mut)`/ `PlanarBufferView(Mut)`。
- `plugin-core`: `ProcessConfig{sample_rate, min_frames, max_frames, reserved}`・`ProcessResult{status, tail_frames}`・`EventList`/`TransportInfo`。
- 規約: 全 crate `[lints] workspace = true`、`publish.workspace = true`（`publish=false` 継承）、crate-level doc に層/許可依存/RT 契約記述、`#[must_use]`・`#[non_exhaustive]` 公開 enum、エラー `ruviform_media_core::Error` 系統。
- `docs/discovery/issue-194-m0-baseline-audit.md`: M0 実測確定値（test 1784 全緑・rt_ 実在 109・全 feature lane・MSRV 1.89・G0 全項合格・G1 対象 Ready 判定）。
