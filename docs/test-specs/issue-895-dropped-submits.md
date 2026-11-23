---
id: "TSPEC-Rf895DrpSbtCse9wQ4xKm5Y3"
type: "test-spec"
status: "approved"
title: "Issue #895 dropped_submits 条件別分離 — 原因別 counter の検証仕様"
created: "2026-11-23"
updated: "2026-11-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/895"
    - "docs/specs/issue-895-dropped-submits.md"
    - "docs/specs/issue-767-submit-drop.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-895-dropped-submits.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-895-dropped-submits.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-895-dropped-submits.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-121-l1-audio-counters-tests.md"
    - "docs/specs/issue-767-submit-drop.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-895-dropped-submits.md"
  verified-by: []
---

# Issue #895 — dropped_submits 条件別分離の検証仕様

## 検証対象と検証入口

| 対象 | AC | 入口 | 検証レベル |
| --- | --- | --- | --- |
| `RtLegCounters` 原因別 increment・1 block 高々 +1・集約=原因和 | AC-1, AC-3 | `mise run test ruviform-plugin-host`(`l1::data` の unit 試験) | OS-L1 |
| `AudioCounterReaders`/`AudioCounters`/`AudioSourceLeases` の 6 counter 配線・source 登録 degradation | AC-1, AC-3 | `mise run test ruviform-plugin-host`(`l1`・`l1::sources` の unit 試験) | OS-L1 |
| `AudioCounterSnapshot` の分離値公開(`audio_counter_snapshot_maps_each_reader_to_its_own_field`) | AC-1 | `mise run test ruviform-plugin-host` | OS-L1 |
| E2E pin(burst → slot-busy 帰属・集約=原因和・chan_full=0) | AC-2 | `mise run test-l1-real-plugin`(`real_test_plugin_l1_dropped_submits_attributes_causes`、clap・vst3 両 ABI) | OS-L1 |
| RT 契約非回帰(drop 経路 `Relaxed` increment のみ) | AC-4 | `mise run test ruviform-plugin-host` + RT レーン(コード上の atomicity は unit 試験で構造確認) | OS-L1 |

## 試験一覧

| ID | 試験名(実体) | 検証する契約 | AC |
| --- | --- | --- | --- |
| T-1 | `slot_busy_drops_count_once_per_block`(既存・分離値 assert 追加) | busy slot drop は集約+1・`slot_busy`+1・`chan_full`+0 — drop 出力は C-3 の defined pass-through(`passthrough_submits` も増える) | AC-1, AC-3 |
| T-2 | `chan_full_drops_count_their_own_counter`(新規) | capacity=depth の submit channel を試験 fixture で pre-fill(slot 帳簿を迂回)し、gate 通過後の `try_push` 失敗が集約+1・`chan_full`+1・`slot_busy`+0 | AC-1 |
| T-3 | `same_block_multiple_failures_count_max_once`(既存・分離値 assert 追加) | 同一 block で 2 原因は併発しない — busy arm は `try_push` 未到達のため `chan_full`+0 で `slot_busy` のみ+1 | AC-3 |
| T-4 | `dropped_submits` 以外の試験群(`result_failed_first_pop_counts`・`held_result_failed_reextraction_adds_nothing`・warmup・remint 等) | 非 drop 経路では 2 原因 counter が不変(+0) — `results_failed_observed`/`results_dropped` の意味は非回帰 | AC-3 |
| T-5 | `register_accepts_six_counter_sources_with_instance_scope`(sources) | 6 descriptor・6 slot・key 集合が拡張通り(`l1.dropped_submits_slot_busy`/`l1.dropped_submits_chan_full` 含む) | AC-1 |
| T-6 | source refusal / remint / abandoned 試験群 | 新規 2 slot も OBS-04 degradation・lease abandon・remint-at-zero が従来どおり | AC-3 |
| T-7 | `real_test_plugin_l1_dropped_submits_attributes_causes`(clap) | 24 call unpaced burst → `dropped_submits > 0` かつ `dropped_submits == slot_busy + chan_full`・`chan_full == 0`・`results_dropped == 0`・`results_failed_observed == 0` | AC-2 |
| T-8 | `real_test_plugin_l1_dropped_submits_attributes_causes`(vst3) | T-7 と同じ assert を VST3 ABI で検証 | AC-2 |

## 主要な assert 不変条件

- 集約: `dropped_submits == dropped_submits_slot_busy + dropped_submits_chan_full`(全試験で構築不変)。
- chan-full の構造的到達不能: 実 child(capacity = `depth`)では `dropped_submits_chan_full == 0` が観測 pin。非ゼロは capacity/slot 不変条件の破損を意味する。
- 1 block 高々 +1: T-1/T-3 が原因 counter 単位で pin(T-3 は busy arm が `try_push` に到達しない順序を pin)。

## 判定

- unit 試験(T-1〜T-6): `mise run test ruviform-plugin-host` で全 pass。
- E2E(T-7・T-8): `mise run test-l1-real-plugin` で両 ABI の対象試験が pass。
  - `dropped_submits > 0` は burst depth による構造的保証(3 slot 占有後の drop)に依存 — 実 child の round-trip(driver poll ~200µs 以上)は µs 級の連続 call の間には完了しないため実質確定的だが、極端に速い pipeline では理論上 submit が間に合い得る。767 evidence(60 run 全件 slot-busy)が実測の裏付け。`chan_full == 0`・集約=原因和は構造的な pin。
- RT 契約(AC-4): drop 分岐の increment が `Relaxed` `fetch_add` のみであることをコード検査 + 既存 RT レーンの非回帰で確認。
