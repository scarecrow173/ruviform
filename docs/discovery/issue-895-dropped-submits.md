---
id: "DISC-Rf895DrpSbtCse7nQ2xKw4Y9"
type: "discovery"
status: "resolved"
title: "Issue #895 dropped_submits の条件別分離 — carrier 選定と集約 counter 扱いの調査"
created: "2026-11-23"
updated: "2026-11-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/895"
    - "docs/specs/issue-767-submit-drop.md"
    - "docs/discovery/issue-767-submit-drop.md"
    - "crates/ruviform-plugin-host/src/l1/data.rs"
    - "crates/ruviform-plugin-host/src/l1/mod.rs"
    - "crates/ruviform-plugin-host/src/l1/sources.rs"
    - "crates/ruviform-plugin-host/src/composite.rs"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-895-dropped-submits.md"
    modified:
      - type: "file"
        path: "docs/specs/issue-767-submit-drop.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/specs/issue-895-dropped-submits.md"
  depends-on:
    - "docs/specs/issue-767-submit-drop.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-767-submit-drop.md"
    - "docs/specs/issue-279-plugin-host-counter-source-registration.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
---

# Issue #895 — dropped_submits の条件別分離: carrier 選定調査

## 結論

**host 側 counter 分割で完結する設計を採用。** `dropped_submits` 集約は残し、原因別 counter `dropped_submits_slot_busy` / `dropped_submits_chan_full` を `RtLegCounters` → `AudioCounterReaders` → `AudioCounterSnapshot` → Observation Source(`l1.dropped_submits_slot_busy` / `l1.dropped_submits_chan_full`)の既存経路に追加する。diag 面・report seam・wire transport への分類値搭載は行わない。

## 背景

`ruviform#767` の帰属調査は一時 probe(`RUVIFORM_L1_PROBE`)で実施した。`dropped_submits` が単一 counter のままでは、同種の回帰(chan-full 系 drop の出現等)を判別するのに probe 再挿入が必要になる。Issue #895 は永続的帰属を要求する。

## drop 条件の構造(再確認)

`RtLeg::process` の `dropped_submits` increment site はちょうど 2 分岐(`docs/specs/issue-767-submit-drop.md` C-2):

1. **slot still busy** — `cleared[slot] < last_pushed[slot]` で strict-alternation gate が落とす。`try_push` に到達しない。
2. **submit channel full** — gate 成立後の `submit.try_push` 失敗。

構造的事実として **chan-full 分岐は in-protocol 到達不能**: submit channel capacity = `depth` であり、channel が満杯になるには `depth` 個の未 drain submit が必要だが、それらは全 slot の `last_pushed` を占有するため全 slot が busy となり gate が先に drop する。`results_dropped`(reader 側 `push_result` 失敗)と同じく、不変条件が破れたときだけ非ゼロになる defensive arm として扱うのが正確な意味付け。

## carrier 比較

| 案 | 内容 | 判定 |
| --- | --- | --- |
| **host counter 分割(採用)** | `RtLegCounters` に 2 cell 追加し、既存の readers/snapshot/source 経路で観測 | 分岐点と観測面が同一側(host)のため輸送不要。`#[non_exhaustive]` の `AudioCounterSnapshot` へのフィールド追加は後方互換。source key の追加は OBS-04 の per-source degradation で収まる |
| diag 面分類値(`l1/diag.rs`) | drop 発生時に分類イベントを diagnostics registry へ記録 | diag は失敗・異常の記録面であり、定常発生し得る bounded shedding の会計面ではない。既存 counter 経路で必要十分なため不採用 |
| report seam(#873 型)経由 | `boundary.rs` に分類 enum を置き plugin→host pull 輸送に載せる | drop 判定は host 側 leg の決定であり child は関与しない。plugin→host 輸送は解決する問題が存在せず、`wire.rs`/`processor.rs`/`event.rs`/plugin-export codec の変更を伴うため不採用 |

## `dropped_submits` 集約 counter の扱い

**残して原因 counter を追加する**(置き換えない)。理由:

- `AudioCounterReaders.dropped_submits` は `CounterReader` cell であり、2 cell の和として派生できない — 集約を残すには独立 cell が必須。
- 集約 counter・snapshot フィールド・source key `l1.dropped_submits` は複数の既存参照(`endpoint.rs`、proxy/scale probe E2E、fault/stateful E2E、`docs/designs/issue-121-l1-diagnostics.md` 等の設計文書)から読まれる。置き換えると後方互換を破り既存 pin を壊す。
- 集約を両分岐で increment し続ける設計では `dropped_submits == slot_busy + chan_full` が構築により成立し、集約値の意味(`高々 +1/block`)は不変。

writer コスト: drop 分岐での `Relaxed` increment が 1→2 個。drop 経路は anomaly 経路(正常系では発火しない)であり高々 +1/block のため、RT 契約(wait-free・no-alloc)に影響しない。

## 配置と命名

- `RtLegCounters` / `AudioCounterReaders` / `AudioCounterSnapshot` / `descriptors()` では `dropped_submits` の直後に `dropped_submits_slot_busy`・`dropped_submits_chan_full` を並べ、drop 系 counter を隣接させる。
- source key は `l1.` namespace に倣い `l1.dropped_submits_slot_busy` / `l1.dropped_submits_chan_full`。
- 両方とも `FAULT_REPORTING`(`OnIncrease` + High) — 親 counter と同じく「正常動作では増えない」宣言。`passthrough_submits` の `SnapshotOnly` は維持。
- 新規語彙は `pub(crate)` counter 名と公開 snapshot フィールドに留め、`ruviform-plugin-core` の公開型(`boundary.rs` 等)は追加しない。

## 関連・残件

- `ruviform#891`: 同じ pull 型 seam 語彙を拡張し得る並行作業 — 本 slice は seam 非依存のため競合しないが、将来 drop 原因を wire 越しに観測したい要求が出た場合は語彙の置き場所を `boundary.rs` 先例に揃える。
- `docs/designs/issue-121-l1-diagnostics.md` 等の設計文書は「4 counter」時代の記述のまま残る(本 slice の文書更新範囲外 — 767 spec C-2 が現行正本)。
- consumer(AudioLens/Synvelis)は `audio_counters`/`AudioCounterSnapshot` を未参照 — 追加フィールドは読まれずとも破壊的変更なし。
