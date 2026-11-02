---
id: "TSPEC-7zOboSetinQdiV2b3AD6Yq"
type: "test-spec"
status: "approved"
title: "Issue #196 M4 Typed Graph Plugin Adapter テスト仕様"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/196"
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
    - "docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/adr/issue-196-plugin-adapter-bridge-placement-scheduler-ownership-bus-role.md"
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/plans/implement-issue-196-typed-graph-plugin-adapter.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-190-transport-span-scheduler-tests.md"
    - "docs/test-specs/issue-195-m2-typed-graph-compiler-runtime-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #196 M4 Typed Graph Plugin Adapter テスト仕様

## Scope

`ruviform-audio-plugin-bridge` の bus 写像・`PluginNode`（scheduler +
endpoint 接着）・fault 契約を、typed `RtRuntime` 上の E2E と単体
テストで検証する。L0 のみ実測可能（L1 は `buses()`=Unavailable）。
TT-025〜030 は #190 test-spec の matrix に対応し、plugin adapter 側の
配送精度として再紐付けする。TT-001〜024 は span/event/transport
契約が本 adapter を通じて保持されるケースのみ対象とする。

## Test Matrix

| ID | Feature | Rule | Example | Layer |
| --- | --- | --- | --- | --- |
| M4-01 | capability matrix | `capability_for` は backend 毎に正直な水準を返す | L0: bus enum Verified/multi-bus Verified/in-block Declared; L1: enum Unavailable/in-block Declared; CLAP: multi-bus Declared/in-block Declared; VST3: 同上 | unit |
| M4-02a | bus→port 写像 | `bus_decl_to_port_def` が role/layout/activation を保存 | Main stereo + Aux stereo + Sidechain mono → 3 PortDef | unit |
| M4-02b | 検証 | `validate_bus_decls` が missing/不一致を defined error | declared 4 vs reported 2 → MissingPort | unit |
| M4-02c | multi-bus E2E | typed graph で main+sidechain 入力・main+aux 出力を駆動 | stereo main in + mono sidechain in → stereo main out + stereo aux out | E2E |
| M4-03 | min/max 契約 | `calls_for_span` のみで物理 call 生成・違反 0 | 256F + loop@100 + min=128 → 100F span undeliverable + 156F call 1 回のみ | E2E |
| M4-04a | offset 正本 | `frame_offset` は process call 開始相対 | span [100,156) + event@120 → call offset 20 | unit/E2E |
| M4-04b | 境界 off-by-one | `event@span.end` は次 span/次 callback | event@256 → 次 callback offset 0 | E2E |
| M4-04c | same-frame causal | 同 frame 複数 event は source_sequence 安定順 | 2 events @100 → offset 0,0 で seq 順 | E2E |
| M4-04d | wrap occurrence | 多周 loop で occurrence 毎に event 1 回配送 | 2 wraps → 同一 event 2 回（各 occurrence） | E2E |
| M4-05 | CLAP 証拠 | in-block transport は証拠なし→Declared で不採用 | capability_for(Clap).in_block_transport == Declared | unit |
| M4-06a | graph swap | generation 一致 install/remove で instance 重複なし | plan A→B swap、旧 event 遮断 | E2E |
| M4-06b | state | SaveState→Deactivate→Activate→RestoreState で state 保持 | param 値復元 + RT 継続 | E2E |
| M4-06c | crashed plugin | panic → silence + Failed + RT 継続 | L0 panic plugin → convergence | E2E |
| M4-06d | missing/busy | EndpointUnavailable は defined rejection | busy cell → Err + outputs unchanged | unit |
| M4-06e | retire overflow | retire channel full → leak + counter | `retire_overflows` 増分 | unit |
| M4-07 | TT 紐付け | TT-025〜030 + TT-001〜024 該当ケース | 後述 Mapping 参照 | doc+test |
| M4-08 | 回帰 | fmt/check/lint/test/test-rt + plugin-host/workspace --locked | 全緑 | CI |
| RT | 無割当 | `rt_plugin_node_process_no_alloc` | assert_no_alloc で process パス | rt_ |

## TT Mapping

- **TT-025**（min/max 互換）→ M4-03 E2E（loop=100/min=128）。
- **TT-026**（Degraded 延期）→ `DeliveryPolicy::Degraded` で deferred
  apply + receipt degradation（#190 と同機構を typed 経路で検証）。
- **TT-027/029**（capability 区別・loop 配送）→ M4-01（L0/L1/規格差）
  + M4-04d（occurrence 配送）。
- **TT-028**（generation/occurrence 区別）→ M4-04d + M4-06a。
- **TT-030**（Explicit degraded のみ）→ undeliverable span は
  `UndeliverableSpan` report で監査可能、silent approximation 禁止を検証。
- **TT-001〜024** — adapter を通じて保持されるのは offset/因果順/
  epoch/generation 契約のみ: M4-04a〜d が TT-006/007/010 に相当。
  command lifecycle・musical time 等は #190/#189 層の責務であり、
  adapter 層では個別検証しない（`build_call_event_list`/
  `call_transport_info` の正本化で担保）。未対応名: TT-001〜005
  （command state・apply 系は scheduler 層責務）、TT-011〜024
  （transport 制御・musical・PPQN・plan apply — adapter 層非対象）。

## Notes

- L1 は `buses()`=Unavailable で bus 列挙検証を行わない。endpoint 接着
  自体は同一だが bus 宣言の実測根拠は L0 限定。
- CLAP/VST3 実規格 lane（`test-l1-real-plugin`）は回帰のみ（fixture は
  単一 stereo のまま）。multi-bus は L0 mock plugin で検証。
- `rt_plugin_node_process_no_alloc` は `assert_no_alloc` で process
  パスの割当を検査（Issue #22 lane 方針）。

## Sources

- `docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md`
- `docs/test-specs/issue-190-transport-span-scheduler-tests.md`
