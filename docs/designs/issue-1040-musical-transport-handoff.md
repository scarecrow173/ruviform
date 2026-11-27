---
id: "DESIGN-Rf1040Musical01"
type: "design"
status: "approved"
title: "Issue #1040 musical transport handoff design"
created: "2026-11-27"
updated: "2026-11-27"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-1040-musical-transport-handoff.md"
    - "docs/adr/issue-1040-plan-owned-musical-transport.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-1040-musical-transport-handoff.md"
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/transport_schedule.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/transport_bridge.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/node.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-1040-musical-transport-handoff.md"
  implemented-by: []
  depends-on:
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-1040-musical-transport-handoff.md"
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

# Issue #1040 musical transport handoff design

## Ownership boundary

tempo map は schedule/composition の一部、PPQN はその tempo map を plugin-facing tick grid へ投影する project policy である。両方を `TransportPlan` と同じ generation/apply boundary に置く。`AttachConfig`、control message、fixture、ABI adapter へ PPQN policy を複製しない。

`TransportPlan` は `tempo_map` と `musical_ppqn` を independent optional として保持する。両方そろった場合だけ musical source が有効であり、片方だけの状態はエラーではなく `musical=None` を意味する honest negative state である。

## Data flow

```text
TransportPlanBuilder
  └─ tempo map + revision + PPQN (same plan generation)
TransportScheduler::emit
  └─ active plan for this emitted span
      └─ MusicalTimeSource<'_> { tempo_map, ppqn }
          └─ SpanSink::span(..., musical)
              └─ PluginNode::CallSink
                  └─ call_transport_info(ctx, rel, musical.tempo_map, musical.ppqn)
                      └─ TempoMap::media_to_beat + time_info (exact rational)
```

`MusicalTimeSource` は small `Copy` struct で、`Option<&TempoMap>` と `Option<Ppqn>` だけを持つ。scheduler が sink call の間だけ reference を貸し、`Arc` clone や whole plan の公開を避ける。mid-callback install では span ごとに emit 時点の active plan を見るため、境界前後で source が混ざらない。

## Playing gate

`call_transport_info` は transport status が `Playing` の場合だけ musical conversion を行う。stopped/paused では media position と loop range を保持したまま `musical=None` とする。これは fixture click law へ依存した downstream workaround ではなく、host 側 snapshot の defined contract である。

## RT contract

追加処理は active plan の field read、reference/Copy 値の受け渡し、既存 exact rational conversionだけである。新規 allocation、blocking、I/O、unbounded scan、fixed 960 fallback はない。media-core の `rt_run_callback_no_alloc` は musical source 付き plan を install した状態で測定する。

## ABI projection boundary

VST3/CLAP adapter は format 由来の quarter-note 表現を plugin-side PPQN 960 へ再投影する。real ABI PCM oracle は plugin が実際に見る PPQN 960 を期待値に使う。host internal handoff が非 960 であることは typed graph Event oracle で直接証明し、両者を混同しない。
