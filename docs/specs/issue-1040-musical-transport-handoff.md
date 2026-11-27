---
id: "SPEC-Rf1040Musical01"
type: "spec"
status: "approved"
title: "Issue #1040 production PluginNode musical transport handoff"
created: "2026-11-27"
updated: "2026-11-27"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/1040"
    - "docs/adr/issue-1040-plan-owned-musical-transport.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-1040-musical-transport-handoff.md"
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/transport_schedule.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/transport_bridge.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/node.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-1040-musical-transport-handoff.md"
  depends-on:
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/specs/issue-630-test-instrument.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-1040-musical-transport-handoff.md"
    - "evidence/rf1040/"
  refines: []
  refined-by: []
  derives-from:
    - "docs/adr/issue-1040-plan-owned-musical-transport.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-1040-musical-transport-handoff.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #1040 production PluginNode musical transport handoff

## Purpose

production graph node 経路で、`TransportPlan` に供給された tempo map と PPQN を plugin process の `TransportInfo::musical()` へ届ける。direct endpoint API に caller supplied `TransportInfo` を渡す経路は対象外とし、`attach()` → `PluginNode` → typed graph の production route を正本として検証する。

## Requirements

1. `TransportPlan` は optional tempo map と optional PPQN を独立に保持できる。consumer は plan build 時に両方を atomic に供給できる。
2. `TransportScheduler` は emit した span に対応する active plan の musical source を `SpanSink` へ渡す。mid-callback install 後の span は新 plan の sourceを使う。
3. `PluginNode` は physical call ごとの span に対応する tempo map/PPQN を `call_transport_info` へ渡す。`None, None` 固定を残さない。
4. plan-less、tempo map 不在、PPQN 未指定、stopped、paused では musical timing を捏造しない。zero/invalid tempo は tempo map validation で production input として成立しない。
5. RT path は reference read・Copy 値・bounded loop に限定し、新規 allocation・blocking・I/O を行わない。
6. host internal PPQN は固定しない。VST3/CLAP adapter の wire/plugin-side PPQN 960 projection は format adaptation として区別する。

## Acceptance criteria

- typed graph Event oracle で、非 default PPQN（480）と 150 BPM が plugin process snapshot に届くことを確認する。
- missing tempo map、missing PPQN、stopped、paused で `musical=None` を確認する。
- VST3/CLAP real ABI `test_instrument` を production graph node 経路で実行し、150 BPM の click frame が独立計算 `ceil((boundary_tick - position_ticks) * sample_rate * 60000 / (bpm_milli * ppqn))` に一致することを確認する。
- loop/seek/position jump で click が前 block state に引きずられず、各 block-start snapshot から再導出されることを確認する。
- pass-through/substituted/stale block は fresh evidence に数えず、lifecycle は `Processing` を維持する。

## Non-goals

rf#630 fixture click law、VST3/CLAP adapter reprojection、L1 wire protocol、direct endpoint API、consumer production plan builder、dynamic tempo edit matrix、全 frame/block boundary matrix、GUI/Native Dock、3OS 最終受入は変更しない。
