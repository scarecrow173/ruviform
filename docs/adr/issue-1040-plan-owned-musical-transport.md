---
id: "ADR-20261127-plan-owned-musical-transport"
type: "adr"
status: "accepted"
title: "Issue #1040 plan-owned musical transport source"
created: "2026-11-27"
updated: "2026-11-27"
owners:
  - "scarecrow173"
metadata:
  adr:
    decision-makers: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/1040"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
  changes:
    added:
      - type: "file"
        path: "docs/adr/issue-1040-plan-owned-musical-transport.md"
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
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-1040-musical-transport-handoff.md"
    - "docs/test-specs/issue-1040-musical-transport-handoff.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/specs/issue-630-test-instrument.md"
  defers: []
  deferred-by: []
---

# Issue #1040 plan-owned musical transport source

## 状態

accepted — 2026-11-27

## 文脈

production graph node 経路の `PluginNode` は `call_transport_info(&span.ctx, rel, None, None)` を呼び、`TransportInfo::musical()` が実プラグインへ届かなかった。`TransportPlan` は既に optional な `TempoMap` を保持していたが、plugin-facing PPQN を同じ apply boundary で供給する方法がなかった。

PPQN 960 は慣例 default であり固定値ではない、という既存 ADR の決定を維持する必要がある。したがって host が missing PPQN を 960 で補ったり、fixture の ABI projection に合わせて host internal resolution を固定したりしてはならない。

## 決定

1. `TransportPlan` が `tempo_map: Option<Arc<TempoMap>>` と `musical_ppqn: Option<Ppqn>` を独立に保持する。
2. `TransportPlanBuilder::with_musical_transport` は tempo map・tempo revision・PPQN を一度に供給する入口とする。`with_tempo_map` 単独または `with_musical_ppqn` 単独は、musical timing を供給しない honest negative state とする。
3. `TransportScheduler::emit` は emit した span に対応する active plan から `MusicalTimeSource<'_>` を構築し、`SpanSink::span` へ reference として貸す。plan 全体や `Arc` の新規 clone は sink へ公開しない。
4. `PluginNode` は scheduler が渡した per-span source を `call_transport_info` へそのまま forward する。mid-callback install 後の span は新しい plan の source を使い、前 plan の sourceを引きずらない。
5. musical timing は `Playing` かつ tempo map/PPQN が両方存在する場合だけ付与する。plan-less、tempo map 不在、PPQN 未指定、停止、一時停止では `musical=None` とし、zero/invalid tempo map は production input として builder validation が拒否する。
6. VST3/CLAP adapter が quarter-note 表現から plugin-side PPQN 960 へ再投影する現行 format adaptation 契約は変更しない。これは host internal PPQN の固定化ではない。

## 却下した選択肢

- `AttachConfig` や `PluginNodeControl` に独立した PPQN policy を追加する案: plan generation と別の apply boundary を作り、tempo map と PPQN が混在するため却下。
- fixture に合わせた fixed 960 fallback: 可変 PPQN ADR に反し、missing PPQN を捏造するため却下。
- `SpanSink` へ whole plan を渡す案: retired plan chase 用 snapshot と per-span musical source の責務が混ざるため却下。
- 旧 signature の compat shim を残す案: 二重契約を恒久化するため却下。全 implementor を coherent に更新する。

## 影響

`SpanSink` は workspace internal public trait の signature 変更を受ける。全 implementor は同一生成物内で更新し、`check-full` を compile sweep とする。consumer は plan build 時に tempo map/PPQN を両方供給しなければ musical transport を得られない。plan-less playback で musical を供給しないことは defined contract とする。
