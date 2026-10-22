---
id: "TSPEC-034KpKr9gVqJJNxYSJVaQF"
type: "test-spec"
status: "approved"
title: "音楽時間 ABI 拡張テスト仕様"
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

# 音楽時間 ABI 拡張テスト仕様

## Purpose

Issue #91（`TransportInfo.musical` + CLAP/VST3 inbound 写像）の受入契約を固定する。
`reserved` 温存・未設定表現の慣例（sig 0/0・bar 0）・フラグゲート規則が将来の
リファクタで退行しないことを保証する。

## Feature

規格トランスポート（CLAP `clap_event_transport` / VST3 `ProcessContext`）の
音楽時間フィールドを、ホストの有効フラグに応じて中立 `MusicalTimeInfo`
スナップショットへ写像する。

## Rules

- `musical` は位置（CLAP `HAS_BEATS_TIMELINE` / VST3 `kProjectTimeMusicValid`）
  とテンポ（`HAS_TEMPO` / `kTempoValid`）が**両方**有効かつ `tempo` が
  有限正値のときのみ `Some`。それ以外は `None`（4/4・テンポの暗黙仮定なし）
- CLAP beattime → tick: `bits × 960 / 2³¹`（`CLAP_BEATTIME_FACTOR`）、
  ゼロから離れる方向への半端丸め。VST3 PPQ → tick: `ppq × 960` の最近接丸め
- `bpm_milli` = `tempo × 1000` round、`1..=999_999_999` にクランプ
- sig は CLAP `HAS_TIME_SIGNATURE` / VST3 `kTimeSigValid` 有効時のみ
  u8 写像（0 または範囲外 → `0/0`）。`bar_start_tick` は sig 有効かつ
  （VST3 は `kBarPositionValid` も）満たす場合のみ写像、それ以外 `0`
- `TransportSnapshot`/`TransportInfo` は `Copy`・allocation-free であり、
  `musical` 追加後も `reserved` は `[0; 2]` のまま温存
- 既存の status / media position / loop range 写像は不変

## Examples

- CLAP: `HAS_BEATS_TIMELINE|HAS_TEMPO|HAS_TIME_SIGNATURE`・beats=2.5・
  bar_start=2.0・tempo=125.5・sig 6/8 → `position_ticks=2400`・
  `bar_start_tick=1920`・`bpm_milli=125_500`・`sig=(6,8)`・`ppqn=960`
- CLAP: bits が半 tick 境界 ±1 → 0 / 1（half-away-from-zero の確定）
- VST3: `kProjectTimeMusicValid|kTempoValid|kTimeSigValid|kBarPositionValid`・
  PPQ=2.5 → `position_ticks=2400`、sig 6/8、bar PPQ=2.0 → `bar_start_tick=1920`
- ゲート: 位置/テンポの片方のみ、tempo ∈ {0, 負, NaN, ∞} → `musical=None`
- 未設定表現: sig フラグなし → `sig=(0,0)` かつ `bar_start_tick=0`、
  `bpm_milli` は有効値を保持

## Guarantees

- `musical()`/`with_musical()`/`TransportInfo::new`/`TransportSnapshot::new`
  が `const` で `Copy` セマンティクスを保つ
- アダプタ写像関数は allocation も blocking も行わない（既存 RT 契約踏襲）
- 写像は決定的 — 同じ入力に対し常に同一の `MusicalTimeInfo`

## Non-goals

- outbound（中立→規格）写像 — ホスト側アダプタが存在しないため対象外
- CLAP `bar_number`・VST3 `chord`/`smpte`/`frameRate` の写像
  （中立側に対応フィールドなし）
- VST3 サイクル（`kCycleValid` → `loop_range`）写像 — PPQ→sample 変換に
  テンポマップが要り、この境界には無い（別 Issue 候補）

## Risk

- フラグゲート欠落 → 無効値を正値として写像（暗黙 4/4 仮定の退行）
- 係数誤り（65536 等）→ 位置写像が 2 桁級ずれる（今回設計書誤記を修正済み）
- `reserved` への詰め替え → ABI 契約破壊と検査不能な拡張
