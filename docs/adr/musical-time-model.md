---
id: "ADR-2iNuZm3ymoyzRUmpgnKr0x"
type: "adr"
status: "superseded"
title: "音楽時間モデル（PPQN 960 tick 中核・piecewise-constant テンポマップ）"
created: "2026-10-22"
updated: "2026-11-01"
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
  depends-on:
    - "docs/specs/music-time-design.md"
  blocks: []
  supersedes: []
  superseded-by:
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/music-time-design.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/designs/music-time-model.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 音楽時間モデル

> **Erratum（旧規格記述の訂正、2026-11-01）:** 本文のCLAP `clap_beattime`を「1/65536拍」とする2箇所は**誤り**。正式な[CLAP fixedpoint.h](https://github.com/free-audio/clap/blob/main/include/clap/fixedpoint.h)では`CLAP_BEATTIME_FACTOR = 1LL << 31`（**2³¹単位/拍**）。対応する[旧詳細設計](../designs/music-time-model.md)はすでに2³¹を使用している。歴史的Decision本文は改変せず、今後の実装・テストは[後継可変PPQN ADR](configurable-ppqn-resolution-independent-musical-time.md)と[Ruviform #189](https://github.com/scarecrow173/ruviform/issues/189)に従う。日付メタデータのcreatedは初版Commit（2026-10-22）で修正。

> **Superseded（設計上）:** 2026-11-01に[可変PPQN・分解能非依存音楽時間の後継ADR](configurable-ppqn-resolution-independent-musical-time.md)を採用。本文は固定960が採用された時点の**歴史的判断**を保存している。既存コード（`Ppqn`/`TempoMap`等）は移行が完了するまで依然として固定960で動作する。[移行Issue #189](https://github.com/scarecrow173/ruviform/issues/189)で追跡。

## Context

ベースライン保留項「音楽時間（小節/拍/PPQN）設計」の確定。プラグイン規格は音楽時間を要求する: CLAP `clap_event_transport` は `clap_beattime`（拍の 1/65536 固定小数点）・テンポ（BPM double）・拍子を持ち、VST3 `ProcessContext` は PPQ double・テンポ・拍子を持つ。Ruviform 中核の時間モデルは有理数（`TimeBase` + 整数カウント）であり、f64 を中核に置かない方針が決定済み。テンポ変更を含む媒体↔音楽変換は RT パス上で allocation なし・有界コストである必要がある。

## Decision

1. **tick 分解能は固定 PPQN=960** とし、内部位置は整数 tick（`i64` newtype）で表す。960 = 2⁶×3×5 で全ての一般的音符分割（2/3/4/5/6/8 連符）を整数で表せ、主要 DAW（Logic 960・REAPER 960・Cubase 480 の整数倍）と整合する。媒体時間モデルと同じ「有理数中核・f64 非中核」方針を継承する。
2. **テンポマップは piecewise-constant**（テンポ/拍子はセグメント内一定）とし、ソート済みイベント列（`{tick, milliBPM}`・`{tick, num, den}`）を非 RT でコンパイルした不変スナップショットとする。媒体↔tick 変換は累積有理時間の二分探索で **allocation なし・O(log n) 有界**。秒/tick が厳密な有理数（`60 / (bpm×ppqn)`）のため変換は可逆かつ丸めは境界呼出しごと最大 1 回（時間モデルの丸めポリシーと同一）。
3. **ABI 境界は専用値型 `MusicalTimeInfo`**: `{position_ticks: i64, bpm_milli: u32, sig_num: u8, sig_den: u8, bar_start_tick: i64, ppqn: u16}` の `Copy` スナップショットを `TransportInfo` に `Option` 付きフィールドとして追加する（`reserved: [u64; 2]` は他の将来拡張のため温存 — 拡張予約の「ABI 形状を変えずに吸収」の意図に反しない `#[non_exhaustive]`/Option 追加）。規格写像はアダプタ側: VST3 PPQ double は `tick / 960`、CLAP beattime は `tick × 65536 / 960` の丸め（端数は round-half-up・変換損失は CLAP 側のみ）。
4. **所有は `ruviform-media-core`**（時間モデルの家。plugin-core は media-core に既依存のため依存方向を乱さない）。テンポマップ編集・可変状態は非 RT 側の `TempoMapBuilder`（仮名）とし、RT 参照は不変 `TempoMap` スナップショット経由。

## Considered Options

- **A. PPQN=960 整数 tick（採用）**: 上記決定。利点: 媒体時間モデルと同じ有理数中核・可逆変換・業界標準分解能。
- **B. CLAP 準拠の 1/65536 拍固定小数点**: CLAP との相互変換は丸め不要になるが、VST3・DAW 慣例（PPQN 整数）と非整合で内部表現としては不自然。棄却 — 規格特異表現はアダプタ層に押し出す既存方針と一致しない。
- **C. BPM/tick を f64 double で保持**: 変換が単純だが、累積誤差・丸め位置不定・「f64 非中核」決定と衝突。棄却。
- **D. 連続テンポカーブ（ramp）対応**: 一部 DAW は tempo ramp を持つが、piecewise-constant で近似可能（セグメント分割）。初期対象外 — 必要時にセグメント種別を追加する拡張として設計に記録。
- **E. 音楽時間を `MediaTime`/`TimeBase` へ直接統合**（tick を一種の TimeBase として扱う）: tick はテンポ依存で媒体秒への変換が文脈依存のため、文脈非依存の `TimeBase` 公理を破る。棄却 — 別型として保持。

## Consequences

- `ruviform-media-core` に `music` 系型（`Tick`・`TempoMap`・`MusicalTimeInfo` 等）が追加される（後続実装 Issue）。
- `TransportInfo` に `Option<MusicalTimeInfo>` フィールド追加 — `publish = false` のため破壊的変更可だが、拡張予約方針どおり既存フィールドは不変で `reserved` は温存。
- RT パス上の変換コストは O(log n)（セグメント数 n）で有界・allocation なし — `rt_` 検証対象。
- 拍子記号が未定の場合（テンポのみのテンポマップ）、bar/beat 分解は行わず tick 位置のみ提供する（後方互換のデフォルト 4/4 は仮定しない）。
- CLAP 写像は tick→beattime で有理丸めを伴う（960 と 65536 は互いに素でないため）。往復変換で最大 ±1 beattime の誤差 — ADR で許容を記録。
