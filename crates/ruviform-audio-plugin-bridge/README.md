# ruviform-audio-plugin-bridge

Issue #196 (Migration M4) — hosted-plugin → typed multi-bus graph adapter.

## 責務

非 RT `PluginHost` 制御面と RT `PluginEndpoint` 処理面を typed multi-bus
graph（`ruviform-audio`/`ruviform-graph`）へ接続する。3 つの継ぎ目だけを所有する:

- **bus → port**: 宣言済み `BusDescriptor` 表を typed `PortDef` へ射影
  (`bus_to_port_def`) し、host の bus report が得られる層では突合する
  (`validate_buses`)。`Unavailable` は失敗ではなく「未検証」として記録する。
- **logical span → physical call**: `PluginNode` が
  `TransportScheduler<EventKind>` を内蔵し、deliverable span を
  `BlockContract` (`min_frames`/`max_frames`) の合法分割へ写像する。
  `EventList` の `frame_offset` は各 physical call 開始相対。
  undeliverable span は silent 化 + `NodeReport::UndeliverableSpan` で監査。
- **lifecycle ownership**: nonRT verb は `PluginHost`、RT 脚は node が
  単独所有。transport 制御は bounded SPSC (`PluginNodeControl`)、
  `Arc` plan は `PlanVault` が寿命を保持し RT 側で destructor が走らない。

## 層・依存

利用側層（consumer）— このクレートに依存するものはない。
`ruviform-plugin-host → ruviform-audio` は dev-dependency のまま (TA-005)。

## RT 制約

`PluginNode::process` は確保/ブロック/`unsafe` なし。per-bus view は
`MAX_BUSES`/`MAX_CHANNELS` の固定配列、event scratch・silent slab は
attach 時の `ScheduleBudget`/`EndpointBounds` で事前確保。

## Capability matrix

`capability_for(Backend)` が L0/L1/CLAP/VST3 各行の
Verified/Declared/Unsupported/Unavailable を宣言する。L1 の bus
enumeration は wire verb 未実装のため `Unavailable` — 推測しない。
