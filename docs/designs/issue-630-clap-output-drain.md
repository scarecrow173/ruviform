---
id: "DSN-630clapOutputDrain01"
type: "design"
status: "approved"
title: "Issue #842 CLAP output-drain slice — 設計"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-clap-output-drain.md"
    - "docs/designs/issue-253-hostgraph-reports.md"
    - "docs/designs/issue-630-fault-modes.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-clap-output-drain.md"
  implements:
    - "docs/specs/issue-630-clap-output-drain.md"
  implemented-by:
    - "docs/test-specs/issue-630-clap-output-drain.md"
  depends-on:
    - "docs/specs/issue-630-clap-output-drain.md"
  related:
    - "docs/discovery/issue-630-clap-output-drain.md"
    - "evidence/rf630/clap-output-drain-verification.md"
---

# Issue #842 CLAP output-drain slice — 設計

## 配置

変更は 2 層に収まる — 既存の wire/plugin-core/export/composite
機構は全て zero-diff:

- `crates/ruviform-plugin-clap/src/host.rs`: 収集・蓄積・drain・
  統計（`ClapPlugin` の RT/非 RT 責務面）。
- `crates/ruviform-plugin-clap/src/adapt.rs`: CLAP ABI → 中立型の
  翻訳（`translate_output_event` 追加のみ — coordinator 承認済み
  lease 拡張。既存 helper の可視性・シグネチャは不変）。

`PluginProcessor::drain_output_reports` の override だけで
`ErasedPlugin` → export runner の `DrainOutputReports` →
`WireReply::Reports` チャンク → composite/`PluginHost` の全経路が
機械的に生きる（Issue #253 で確立済みの形状）。

## 構造的バウンド: `BoundedOutputBuffer`

clack `EventBuffer` は `Vec` 裏付けで `try_push` が拒否しない —
capacity 超過で再確保し、プラグイン側コードがホストに RT 確保を
強いられる。そこで `OutputEventBuffer` trait を `EventBuffer` へ
転送する wrapper を挟み、2 つの上界で `Err(TryPushError)` を返す:

- 件数: `MAX_EVENTS_PER_BLOCK`（=256、VST3 側と同スケール）。
- バイト: 単一イベント `size_of::<TransportEvent>` 超
  （`EventBuffer::with_capacity` が index slot ごとに確保する
  "最大標準イベント" 量子 — 巨大 SysEx 系 payload を拒否する
  foreign-space ガード。標準イベント型は全てこの範囲内）。

`with_capacity(N)` は `prepare` 時のみ呼ばれ N slot + N×量子分を
予約するため、受理済み push は全て予約内 — RT パスは再確保しない。
拒否は block ごと `refused` カウンタに積み、`collect` 時に
`ReportStats::refused` へ畳み込む。

## 収集・蓄積・drain（VST3 パリティ）

- `process()` は block 開始時に `output_events.clear()`（capacity
  保持）→ `OutputEvents::from_buffer` を plugin へ → 終了後
  `collect_output_reports()` が全イベントを
  `translate_output_event` へ通す。
- `pending_reports: Vec<Event>` は `prepare` で
  `MAX_EVENTS_PER_BLOCK` 予約。`push_report` は
  `len < capacity` 内のみ push — 満杯は defined drop として
  `skipped` 計数（再確保しない = producer back-pressure、消費者が
  drain しない場合の溢れは仕様上の drop）。
- `collect` は `frame_offset` キーの `sort_unstable` で同 block 内
  順序を canonical 化する（同 offset は安定 — 到着順を保つ）。
- `drain_output_reports(&mut out)` は `out.append(&mut pending)`
  のみ — 非 RT 動詞、caller 側 `Vec` の拡張は消費者の責務。
- `ReportStats{translated, skipped, refused}` は `prepare` で reset、
  それ以外は累積（VST3 `MidiBridgeStats` と同法則）。

## 翻訳ディスパッチ: `translate_output_event`

`UnknownEvent` + `params: &[ParamInfo]` → `Option<Event>`:

- `ParamValue`: `param_id` で `params` 探索 → `ParamInfo::normalize`
  （`ParamEvent`/`Event::new`）— 未知 id / `CLAP_INVALID_ID` /
  非有限値は `None`。
- `Midi`/`Midi2`: 入力パスと同一の `translate_midi_event` に委譲
  — channel-voice payload 保持、非 channel-voice（SysEx 等）は
  定義通り `None`。
- `NoteOn`/`NoteOff`: 構造 note → `EventKind::Midi` の NoteOn/
  NoteOff 写像。`pckn.channel`/`key` が `Match::Specific` のみ受け
  付け（wildcard 対象は canonical 化不能 → `None`）、
  `pckn.port_index` `Specific(p)` は `MidiGroup` へ、`All` は
  `MidiGroup::ZERO`。velocity は VST3 `norm_f32_to_u32` 法則
  （非有限 → `None`、`[0,1]` clamp → `u32` スケール → 丸め）。
- それ以外の core kind（Transport、NoteChoke、ParamGesture*、
  ParamMod 等）と foreign イベント空間は `None` — canonical
  report payload を持たない。

## 変形イベントの法則（faithful-stamp）

- `u32::MAX` スタンプ: `Event::frame_offset` は `u32` で canonical
  表現可能 → そのまま届ける。VST3 collect が block 外 offset を
  drop するのとは意図的に異なる選択 — Issue #253 契約はホストが
  offset を再解釈しない（複数 block 蓄積時も順序のみ正規化）と
  定める上、変形スタンプ自体が fault fixture の観測対象であり、
  drop/clamp すると `void()` シンクと区別不能になる。
- NaN payload: `ParamInfo::normalize` が拒否し `None` — `0.0` の
  捏造はしない（`invalid` leg は「drain が live なのに何も届か
  ない」が観測形態）。
