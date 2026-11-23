---
id: "SPEC-Rf895DrpSbtCse3mQ8xKw2Y7"
type: "spec"
status: "approved"
title: "Issue #895 dropped_submits の条件別分離 — 原因別 counter の永続的帰属"
created: "2026-11-23"
updated: "2026-11-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/895"
    - "docs/discovery/issue-895-dropped-submits.md"
    - "docs/specs/issue-767-submit-drop.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-895-dropped-submits.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/discovery/issue-895-dropped-submits.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-767-submit-drop.md"
    - "docs/specs/issue-279-plugin-host-counter-source-registration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-767-submit-drop.md"
    - "docs/specs/issue-279-plugin-host-counter-source-registration.md"
    - "docs/designs/issue-121-l1-diagnostics.md"
  refines:
    - "docs/specs/issue-767-submit-drop.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-895-dropped-submits.md"
---

# Issue #895 — dropped_submits の条件別分離(原因別 counter の永続的帰属)

## What Should Be Built

L1 host 側 submit 経路の `dropped_submits` counter を、2 つの drop 原因
(`docs/specs/issue-767-submit-drop.md` C-2 の slot still busy / submit channel
full)ごとに永続的に観測できるよう分割する。集約 `dropped_submits` は維持し、
原因別 counter `dropped_submits_slot_busy` / `dropped_submits_chan_full` を
`RtLegCounters`(writer)→ `AudioCounterReaders`(reader)→
`AudioCounterSnapshot`(公開 query)→ Observation Source(key
`l1.dropped_submits_slot_busy` / `l1.dropped_submits_chan_full`)の既存経路に
追加する。`results_dropped` / `results_failed_observed` は既に独立 counter で
あり、合わせて AC の 4 分類が永続的に観測できる。

## Why Now

- `ruviform#767` は一時 probe で drop 帰属を取った(60 run 全件 slot-busy)。
  単一 counter のままでは、同種の回帰が起きたとき原因判別に probe 再挿入が
  必要になる。
- #873(pull 型 plugin→host report seam)は着地済みだが、drop 判定は host 側
  leg の決定であり child を介さない — seam 輸送は不要で、既存 counter 経路の
  分割が最小の整合的変更(調査: `docs/discovery/issue-895-dropped-submits.md`)。

## Users and Value

- **運用・診断**: `PluginHost::audio_counters` / OBS source 経由で drop 原因を
  常時読める。chan-full の出現(capacity/slot 不変条件の破損兆候)を probe なしに
  検出できる。
- **E2E oracle**: `l1_real_test_plugin_e2e` 系が分離値を assert でき、
  「drop = defined 経路」の oracle 分類を原因粒度で記述できる。

## Scope

### In Scope

- `RtLegCounters` への `dropped_submits_slot_busy` / `dropped_submits_chan_full`
  cell 追加と、対応する 2 分岐での increment(集約 `dropped_submits` は両分岐で
  従来どおり increment)。
- `AudioCounterReaders`・`AudioCounters`・`AudioCounterSnapshot`・
  `descriptors()`/`AudioSourceLeases`(4 → 6 source)への同値の配線。
- 既存の不変条件 pin(`data.rs` の counter 試験)の分離値 assert への更新と、
  chan-full 分岐の unit 試験追加。
- `l1_real_test_plugin_e2e`(clap/vst3)への分離値 assert pin 追加。
- `docs/specs/issue-767-submit-drop.md` C-2・対象外の現行化、および
  本 spec・test-spec・discovery の新規作成。

### Out of Scope

- diag 面(`l1/diag.rs`)・report seam/wire transport(`wire.rs`/`plugin.rs`/
  `processor.rs`/`event.rs`/`ruviform-plugin-export`)への分類値搭載。
- `dropped_submits` 集約 counter・snapshot フィールド・source key
  `l1.dropped_submits` の削除・意味変更(既存参照を保持するため維持)。
- `ruviform-plugin-clap`/`ruviform-plugin-vst3` adapter 側の変更。
- `docs/designs/issue-121-l1-diagnostics.md` 等の「4 counter」時代の設計文書の
  書き換え(参照先の更新は別 slice)。

## Requirements

- R-1: drop 原因が `dropped_submits_slot_busy` / `dropped_submits_chan_full` /
  `results_dropped` / `results_failed_observed` の 4 分類で永続的に観測できる
  (counter・`audio_counters` snapshot・OBS source key の三面)。
- R-2: `dropped_submits == dropped_submits_slot_busy + dropped_submits_chan_full`
  が構築により常に成立する(各分岐が集約と自身の原因 counter を increment)。
- R-3: 767 spec の契約を維持する — 各 counter は 1 block 高々 +1、drop 時の
  出力は defined pass-through、seq 消費・completion 会計は不変。drop
  semantics(shedding としての設計意図)を変えない。
- R-4: RT 契約を維持する — 追加は `Relaxed` increment のみ。lock・alloc・
  shm・IO を drop 経路に持ち込まない(drop 経路は anomaly で高々 +1/block)。
- R-5: `AudioCounterSnapshot` は `#[non_exhaustive]` のフィールド追加で後方互換。
  source 登録は OBS-04 の per-source degradation に従い、新規 2 key の refusal は
  他 4 slot と mint 成功を壊さない。
- R-6: 語彙は `pub(crate)` counter 名・公開 snapshot フィールド・`l1.` source
  key に留め、`ruviform-plugin-core` の公開型を追加しない。

## Acceptance Criteria

- [x] AC-1: 4 分類が `audio_counters` snapshot フィールドと `l1.` key で観測
  できる(R-1)。
- [x] AC-2: `l1_real_test_plugin_e2e`(clap・vst3 両方)で分離値を assert する
  pin が追加される — burst による slot-busy 帰属・集約=原因和・
  `chan_full == 0` を公開面で検証。
- [x] AC-3: 既存の drop semantics が不変 — 767 spec の C-1〜C-6・既存 counter
  試験が分離値 assert 付きで pass、`results_dropped`/`results_failed_observed`
  の意味・増分点は非回帰。
- [x] AC-4: `mise run test-rt` の RT レーンが回帰しない(新規 increment は
  `Relaxed` fetch_add、drop 経路のみ)。

## Sources and Relations

- Issue #895(本 spec の正本・完了条件)、drop 契約の正本
  `docs/specs/issue-767-submit-drop.md`(C-2 改訂済み)、帰属調査
  `docs/discovery/issue-767-submit-drop.md`・carrier 選定
  `docs/discovery/issue-895-dropped-submits.md`。
- counter 所有・登録経路: `docs/specs/issue-279-plugin-host-counter-source-registration.md`、
  意味表の Issue #121 系 `docs/designs/issue-121-l1-diagnostics.md`。
- `ruviform#891`: pull 型 seam 語彙の並行拡張 — 本 spec は seam 非依存だが、
  将来 wire 越しの分類値が必要になった場合は `boundary.rs` の中立 enum 先例に
  揃える。
