---
id: "TSPEC-Rf1040Musical01"
type: "test-spec"
status: "approved"
title: "Issue #1040 musical transport handoff — test specification"
created: "2026-11-27"
updated: "2026-11-27"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-1040-musical-transport-handoff.md"
    - "docs/designs/issue-1040-musical-transport-handoff.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-1040-musical-transport-handoff.md"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/tests/e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_real_test_plugin_e2e.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-1040-musical-transport-handoff.md"
    - "docs/designs/issue-1040-musical-transport-handoff.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-test-instrument.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "evidence/rf1040/"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-1040-musical-transport-handoff.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-1040-musical-transport-handoff.md"
    - "docs/designs/issue-1040-musical-transport-handoff.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #1040 musical transport handoff — test specification

## Test matrix

| Level | Test | Oracle |
| --- | --- | --- |
| media-core unit | `plan_builder_carries_musical_halves_independently` | tempo map only / PPQN only / complete plan が independent optional として保持される |
| media-core unit | `spans_receive_their_own_active_plan_musical_source` | mid-callback install 前後の span が各自の active plan sourceを受け取る |
| media-core RT | `rt_run_callback_no_alloc` | musical source 付き plan install 後も callback が allocation-free |
| plugin-core unit | `call_transport_musical_requires_playing_and_both_sources` | Playing + 両 source の場合だけ musical が付く |
| typed graph Event | `musical_transport_reaches_plugin_with_plan_ppqn_and_tempo` | 150 BPM・host PPQN 480 が production `PluginNode` 経路で plugin process snapshot に届く |
| typed graph Event negative | `musical_transport_negative_controls_do_not_invent_musical_time` | missing map / missing PPQN / stopped / paused で `musical=None` |
| typed graph Event loop | `musical_transport_loop_wrap_is_rederived_per_span` | loop wrap 後の position/tick が前 span の値を引きずらない |
| VST3 real ABI PCM | `attach_transport_clicks_follow_production_plan_musical_handoff` | production graph node 経路で click frame・amplitude・stereo一致・negative control を検証 |
| CLAP real ABI PCM | 同名 | VST3 と同じ oracle を CLAP で検証 |

## Typed graph Event oracle

150 BPM / 48 kHz / PPQN 480 は 1 tick = 40 frame である。40-frame physical call の block-start position 0/40/80 が tick 0/1/2 に対応することを、plugin が記録した `TransportInfo` で直接 assert する。PPQN 480 は fixed 960 fallback があれば失敗する非 default resolutionである。

negative control は plugin process が観測した snapshot 自体を見る。stopped/paused については click が出ないことではなく、`musical=None` が host snapshot contract として届くことを確認する。

## Real ABI PCM oracle

production route は out-of-process L1 `test_instrument` → `attach()` → `PluginNode` → typed `GraphRuntime` → sink PCM とする。caller supplied `TransportInfo` は使わない。

host plan は 150 BPM / PPQN 480 とする。ABI adapter の現行 format adaptation により fixture-visible musical data は PPQN 960 へ再投影されるため、click law の期待値は PPQN 960 で独立計算する。480-frame block は projected tick 24 分に相当し、scripted position は rounding を検証対象にしない exact positionである。

独立計算は次式である。

```text
ceil((boundary_tick - position_ticks) * sample_rate * 60000 / (bpm_milli * ppqn))
```

150 BPM / 48 kHz / PPQN 960 では 1 tick = 20 frame である。position tick 954 は frame 120、position tick 958 は frame 40 に click を置く。click amplitude は `0.5`、click 以外の sample は IEEE zero、stereo channel は bit-identical とする。

pass-through/substituted block は replay であり fresh evidence に数えない。matched block は source block `call - L1_LOOKAHEAD` に帰属させ、phase ごとに少なくとも 2 つの fresh click sampleを要求する。lifecycle が `Processing` でない場合は pass にしない。

## Loop/seek oracle

seek phase は毎 block exact position へ jumpし、click frame が新しい block-start snapshotだけから再導出されることを見る。loop phase は 2-block loop で projected tick 930/954 を往復し、tick 954 の blockだけ frame 120 に click が出ることを確認する。前 block の残り時間で click を schedule する実装ではこの配置を再現できない。

## Canonical lanes

- `mise run test ruviform-media-core`
- `mise run test ruviform-plugin-core`
- `mise run test ruviform-audio-plugin-bridge`
- `mise run test-l1-real-plugin`
- `mise run test-rt`
- `mise run check-full`

`test-l1-real-plugin` と `test-rt` は zero-match pass を防ぐため、実行された test 名/件数を evidence に残す。
