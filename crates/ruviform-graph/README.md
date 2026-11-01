# ruviform-graph

型付き複数 Bus・Port 接続を表現する **Graph IR** と、その **非 RT Validator** を提供する構造層クレート（Issue #184 / Migration M1）。

## 責務

- `NodeId`/`PortId`/`ConnectionId` 等の stable logical ID と `NodeDef`/`PortDef`/`ConnDef` から `GraphSpec` を構築する `GraphBuilder`
- direction/domain(format 導出)/role/format/multiplicity/capability/availability/activation を持つ Port 属性モデル
- direction・domain・format 階層・layout+`ChannelMap`・capacity・required connection・SCC ベースの cycle 判定を行う `Validator`
- 検証済み不変 snapshot `ValidatedGraph` と stable↔runtime 双方向 `StableHandleMap`
- requested/published revision を区別する非 RT コンテナ `GraphProject`

## 許可依存層

- 通常依存: `ruviform-audio-core` のみ（`PortFormat::Audio` が `AudioFormat` を再利用）。
- `ruviform-rt`・engine(`ruviform-audio`)・plugin 系・adapter 系への依存は禁止。engine/plugin 側が本 crate に下向き依存する方向のみ許可。

## RT 契約

- 本 crate の API は **全て非 RT**（構築・検証・revision 発行は `&mut`/所有権ベース、RT コールバック外で行う）。RT スレッドからの読み取り機構（`PlanSwap` 等）は M2 以降の consumer 側の責務。
- `unsafe` 不使用（workspace lint `unsafe_code = warn` でゲート済み）。
- Audio domain の payload format として `AudioFormat`（RT-suitable `Copy` 値型）を参照するが、IR 自体はバッファを扱わない。
