---
id: "DESIGN-iwcBlHQ0lcdHJGucocxHOo"
type: "design"
status: "approved"
title: "音楽時間モデル設計（小節/拍/PPQN・テンポマップ・プラグイン ABI 整合）"
created: "2026-11-03"
updated: "2026-10-23"
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
  implements:
    - "docs/specs/music-time-design.md"
  implemented-by: []
  depends-on:
    - "docs/adr/musical-time-model.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/time-model-extension.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/music-time-design.md"
    - "docs/adr/musical-time-model.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/plans/implement-issue-33-music-time.md"
    - "docs/plans/implement-issue-90-music-time-media-core.md"
    - "docs/plans/implement-issue-91-musical-transport-abi.md"
    - "docs/test-specs/music-time-media-core-tests.md"
    - "docs/test-specs/musical-transport-abi-tests.md"
  verifies: []
  verified-by:
    - "docs/test-specs/music-time-media-core-tests.md"
    - "docs/test-specs/musical-transport-abi-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# 音楽時間モデル設計

Issue #33 — 設計ベースライン保留項（音楽時間設計）の再着手。ADR `docs/adr/musical-time-model.md`（accepted）の決定を実装可能な型・アルゴリズム・ABI 写像規則へ具体化する。**本 Issue は設計のみ — コード実装は後続 Issue。**

## Overview

音楽時間は媒体時間（`MediaTime`）とは別系統の座標系であり、テンポマップを介したみ相互変換される。中核表現は **PPQN=960 の整数 tick**（有理数中核方針の継承）。テンポマップは piecewise-constant のソート済みイベント列を非 RT でコンパイルした不変スナップショットとし、RT 変換は二分探索による allocation なし・有界コストとする。所有は `ruviform-media-core` の `music` モジュール（仮名）。

## 型設計（`ruviform-media-core::music`）

| 型 | 形状 | 責務 |
| --- | --- | --- |
| `Ppqn` | `u16` newtype・唯一の値 `960` を持つ定数 `PPQN` | 分解能の単一正本。将来の可変分解能を型レベルで排除 |
| `Tick` | `i64` newtype | tick 位置（負値 = pre-roll/先行小節を許容）。`Add/Sub`・`Ord` |
| `TickDuration` | `i64` newtype（tick 差分） | `Tick` の区間長。`MediaDuration` に相当する tick 版 |
| `TempoEvent` | `{tick: Tick, bpm_milli: u32}` | tempo 変更点。bpm は milliBPM 固定小数点（120.000 → `120_000`、範囲 1–999,999,999） |
| `TimeSignatureEvent` | `{tick: Tick, numerator: u8, denominator: u8}` | 拍子変更点。denominator は 2 の冪のみ（構築時検証） |
| `TempoMapBuilder` | 可変イベント列 + `build()` | 非 RT 構築。イベントのソート・検証・セグメント事前計算を行う |
| `TempoMap` | 不変コンパイル済みスナップショット（`Arc` 共有前提） | 変換クエリのみ公開。内部は累積有理時間テーブル |
| `MusicalPosition` | `{bar: u32, beat: u8, tick_offset: u16}` | tick の小節/拍分解（拍子マップ依存の表示層） |
| `MusicalTimeInfo` | `{position_ticks: i64, bpm_milli: u32, sig_num: u8, sig_den: u8, bar_start_tick: i64, ppqn: u16}` | **プラグイン ABI 境界の `Copy` スナップショット**（RT 適合） |

### `TempoMap` 内部表現

```text
sorted TempoSegment[] = {
  start_tick: i64,            // セグメント開始（昇順・非重複 — build 時に検証）
  bpm_milli: u32,
  start_seconds: Ratio,       // セグメント先頭の累積秒（有理数 — TimeBase 無しの内部表現）
}
sorted TimeSignatureEvent[] = {tick, num, den}  // 拍子列（独立マップ）
```

`start_seconds` は構築時に前方累積で確定: `seg[i+1].start_seconds = seg[i].start_seconds + (seg[i+1].tick − seg[i].tick) × 60 / (bpm_milli[i]/1000 × 960)`。全て有理数演算のため往復変換は可逆。

## 変換 API（全て純粋・allocation なし・O(log n)）

| API | 変換 | コスト |
| --- | --- | --- |
| `TempoMap::tick_to_media(tick, base: TimeBase) -> MediaTime` | tick → 累積有理秒 → `TimeBase` へ 1 回丸め | `log2(n)` 二分探索 + 有理数乗除 |
| `TempoMap::media_to_tick(mt: MediaTime) -> Tick` | 秒 → セグメント探索 → 線形補間（端数は切捨て・往復誤差 ≤1 tick） | 同上 |
| `TempoMap::musical_position(tick) -> MusicalPosition` | 拍子マップで小節/拍分解 | `log2(m)` |
| `TempoMap::time_info(tick) -> MusicalTimeInfo` | ABI スナップショット生成（現在拍子・現在テンポ・小節開始 tick） | `log2(n+m)` |

- セグメント探索は「`start_tick ≤ t` を満たす最大のセグメント」の二分探索 — 有界・分岐数固定。
- 変換の丸めポリシーは時間モデルと同一: 呼出しごと最大 1 回・安定アンカー（テンポ変更点は tick と秒の両方で厳密に一致するアンカー）。
- 拍子イベントが無いテンポマップでは `musical_position`/`bar_start_tick` は `None`/未設定とする（4/4 を暗黙仮定しない — ADR 決定 4）。

## RT 制約

- `TempoMap` は構築後に不変 — RT 側は `arc-swap`/`RtPlan` 系の既存スナップショット交換で受け取る（エンジン統合時の契約）。RT スレッド上で `TempoMapBuilder`・イベント列の変更は行わない。
- 全変換 API は `const`-不可（二分探索）だが **allocation なし・上限有界** — `rt_` 検証対象（`assert_no_alloc` 適用）。
- `MusicalTimeInfo` は `Copy` 値型（24 bytes）— イベント/トランスポートの `Copy` 契約と整合。

## プラグイン ABI 整合（Issue #33 の ABI スコープ）

`ruviform-plugin-core::TransportInfo` に **`Option<MusicalTimeInfo>` フィールドを追加**する形で拡張予約（`reserved: [u64; 2]`）の意図を吸収する。`reserved` 自体は他の将来拡張のため温存し、本変更はフィールド追加のみ（`publish = false` のため破壊的変更可）。`TransportSnapshot`（media-core）側も同一方針。

| 中立 ABI (`MusicalTimeInfo`) | CLAP `clap_event_transport` | VST3 `ProcessContext` |
| --- | --- | --- |
| `position_ticks` (PPQN=960) | `song_pos_beats`（`clap_beattime` = 拍の 1/2³¹, `CLAP_BEATTIME_FACTOR`）: `tick × 2³¹ / 960`、round-half-up（往復 ±1 beattime 許容 — ADR 記録） | `project_time_music`（PPQ double）: `tick / 960` |
| `bpm_milli` | `tempo`（double）: `bpm_milli / 1000.0` | `tempo`（double）: 同左 |
| `sig_num`/`sig_den` | `tsig_num`/`tsig_denom` | `time_sig_numerator`/`time_sig_denominator` |
| `bar_start_tick` | `bar_start`（beattime 変換同上） | `bar_position_music`（PPQ 変換同上） |
| `ppqn` (=960) | 変換定数（ABI に露出しない） | 変換定数 |

- 音楽時間を持たない呼出し（ホストが tempo map 非提供）は `musical = None` — プラグイン規格側では transport の該当フラグ（CLAP `has_tsig`/`has_tempo` 等）を落とす写像。
- イベント内の位置情報（ノートイベント等の tick ベースタイムスタンプ）は本スコープ外 — プラグインイベント型の拡張時に同じ `Tick`/PPQN 規約を再利用する（設計に予約として記録）。

## 実装単位（後続 Issue の分割）

1. `ruviform-media-core::music` モジュール新設: 値型（`Tick`/`TempoEvent`/`TimeSignatureEvent`/`MusicalPosition`/`MusicalTimeInfo`）+ `TempoMapBuilder`/`TempoMap` + 変換 API + `rt_` 検証
2. `ruviform-plugin-core`: `TransportInfo` への `musical` フィールド追加 + CLAP/VST3 アダプタの写像実装

## Verification

- [x] ADR で時間表現・テンポマップ方式が決定（選択肢比較付き）
- [x] 型設計・変換アルゴリズム・RT 制約が本書に記述（spec 要件 1–4 をカバー）
- [x] ABI スコープ整理: `TransportInfo` の拡張形と CLAP/VST3 写像規則・丸め損失の許容範囲を規定（spec 要件 4・5、`reserved` 整合を確認済み — `crates/ruviform-plugin-core/src/transport.rs`・`crates/ruviform-media-core/src/transport.rs`）
