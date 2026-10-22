---
id: "TSPEC-034KoNprsmdPREx6TtcMB6"
type: "test-spec"
status: "approved"
title: "音楽時間モデル（media-core::music）テスト仕様"
created: "2026-10-22"
updated: "2026-10-22"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/music-time-model.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by: []
  verifies:
    - "docs/designs/music-time-model.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 音楽時間モデル（media-core::music）テスト仕様

## Feature

`TempoMapBuilder` → `TempoMap` の構築検証と、tick ↔ `MediaTime`・小節/拍分解・ABI スナップショットの変換 API。

## Rules

- 構築は非 RT で検証済み（ソート・範囲・重複排除）: `bpm_milli` 1–999,999,999・拍子 denominator は 2 の冪・テンポイベント ≥1
- 変換は全て allocation なし・O(log n)・丸めは呼出しごと最大 1 回（テンポ変更点は tick/秒で厳密一致するアンカー）
- 往復変換 `media_to_tick ∘ tick_to_media` の誤差は ≤1 tick（端数は切捨て）
- 拍子イベント非適用域では `musical_position` = `None`・`time_info` の sig 系は 0/0 + `bar_start_tick` 0（未設定表現。4/4 暗黙仮定なし — ADR 決定 4）
- 負 tick（pre-roll）は先頭セグメントを `-∞` 方向へ外挿して変換
- `MusicalTimeInfo` は `Copy`・24 bytes

## Examples

- `builder_sorts_and_validates`: 非ソート投入で昇順化・`bpm_milli` 範囲外/拍子 denominator 非 2 冪/テンポイベント 0 件で `Err`
- `tick_to_media_constant_tempo`: 120.000 BPM 単一イベント・tick 960 → 0.5 s・tick 1920 → 1.0 s（`TimeBase` 厳密）
- `tick_to_media_anchor_exact`: テンポ変更点 tick が累積秒に厳密一致（往復の安定アンカー）
- `media_to_tick_roundtrip`: 代表 tick 列で往復誤差 ≤1 tick・セグメント境界で厳密
- `tick_to_media_milli_bpm`: 120.500 BPM の milliBPM 固定小数点経路が厳密有理数のまま変換
- `preroll_negative_tick`: 先頭イベントより前の負 tick が先頭セグメント tempo で外挿（負の `MediaTime`）
- `musical_position_decomposes`: 4/4 単一・3/4 変更ありの両マップで bar/beat/tick_offset 分解（例: 4/4 で tick 3840 = 小節 1 拍 0）
- `musical_position_none_without_signature`: 拍子イベントなし/適用域外で `None`
- `time_info_snapshot`: 現在テンポ・現在拍子・当該小節開始 tick を `MusicalTimeInfo` として返す。拍子なし時 sig=0/0・`bar_start_tick`=0
- `musical_time_info_is_copy_24b`: `size_of::<MusicalTimeInfo>() == 24`・`Copy` 実装
- `rt_conversion_no_alloc`: assert_no_alloc 窓内で `tick_to_media`/`media_to_tick`/`musical_position`/`time_info` を実行し 0 alloc 実測

## Non-goals

- `TempoMap` の `Arc` 共有/スナップショット交換配線（エンジン統合時の契約 — #90 スコープ外）
- `TransportInfo`/`TransportSnapshot` への `musical` フィールド追加と CLAP/VST3 写像（#91）
- イベント内位置情報（ノート等の tick タイムスタンプ）

## Risk

- 丸めポリシー逸脱（複数回丸め・中間 `f64`）は往復可逆性とプラグイン ABI の ±1 beattime 許容を崩すため、i128/u128 厳密有理数経路を維持する
