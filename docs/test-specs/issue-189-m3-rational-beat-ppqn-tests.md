---
id: "TSPEC-034QfbY0KiEUPgYkq50kJl"
type: "test-spec"
status: "approved"
title: "Issue #189 M3 Rational Beat/PPQN golden + RT tests"
created: "2026-11-01"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/189"
    - "https://github.com/scarecrow173/synvelis/issues/19"
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
  related:
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md"
    - "docs/test-specs/music-time-media-core-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-189-rational-beat-variable-ppqn-m3.md"
  derived-by: []
  verifies:
    - "docs/plans/implement-issue-189-m3-rational-beat-variable-ppqn.md"
    - "docs/specs/issue-189-rational-beat-variable-ppqn-m3.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #189 M3 Rational Beat/PPQN golden + RT tests

## Purpose

Synvelis PPQN-01〜12 のうち Ruviform #189 M3 が所有する項目を、
`ruviform-media-core` の単体・Golden・`rt_` テストで固定する。
旧固定960実装の意味的互換を「同一 PPQN で同一結果」の Golden として
後退させないことが目的。旧固定960を暗黙の正本とした実装へ退行した場合に
検出できるよう、正本は常に有理数 Beat Position であることを試験する。

## Feature

Resolution-independent musical time: `BeatPosition` 正本、検証済み可変
`Ppqn`、`Tick`+分解能文脈 `TickPosition`、Beat 基準 `TempoMap`、および
規格写像（CLAP `clap_beattime` = `1<<31` 固定小数点、VST3 PPQ f64）。

## Rules

- Rule 1: `BeatPosition` は常に正規化（`gcd(|num|,den)==1`・`den>=1`・符号は num）され、等値は有理数等値と一致する。
- Rule 2: `den == 0`、演算結果の `i64`/`u32`/`i128`/`u128` 境界超過、および `media_to_beat` の分母 `u32` 超過は、暗黙丸め・飽和なしに定義済みエラーで拒否する。
- Rule 3: `Ppqn` は `1..=u16::MAX` で構成でき、`0` は `InvalidValue("ppqn")`。
- Rule 4: `TickPosition` の等値は `(tick, ppqn)` 表現等値であり、異 PPQN の同 Tick 値は等しくない。音楽的順序は `to_beat()` 経由でのみ比較する。
- Rule 5: TempoMap 変換は Tempo セグメント境界が両方向で正確（ロスレス anchor）であり、丸めは最終投影での単回適用のみ。
- Rule 6: `media_to_beat` は正確な有理数を返し、`media_to_tick` は絶対拍位置への `Floor` 単回丸め（旧 960 実装と同一ポリシー）。
- Rule 7: 旧固定960 API（`PPQN` 投影）と新 API で、同一入力の TempoMap 変換・Bar/Beat 分解・`MusicalTimeInfo` が一致する。
- Rule 8: 全ての `TempoMap`/`BeatPosition` クエリ API は `assert_no_alloc` 窓内で無割当・有界である。
- Rule 9: CLAP `clap_beattime` 変換は `1<<31` 固定小数点を用い、往復・負位置・丸め境界が spec 式と一致する。
- Rule 10: `MusicalTimeInfo` の `ppqn` フィールドは渡した実分解能を報告し、960 投影時は旧来と同一値となる。

## Examples

- `BeatPosition::new(1, 7)` は正確に保持され、`TickPosition::from_beat` で PPQN=960 へ投影すると `NearestEven` で `960/7 ≈ 137.14 → 137` の単回丸め（PPQN-04）。
- `TickPosition(960t @960)` と `TickPosition(960t @480)` は等しくなく、`to_beat()` で `1 beat` vs `2 beats`（PPQN-05）。
- PPQN 480/960/1000/1920 の `Tick` → `BeatPosition` → `MediaTime` → `BeatPosition` → `Tick` 往復が ±1 tick（PPQN-03/07）。
- tempo 変更点の `beat_to_media`/`media_to_beat` 往復がロスレスで、第1 tempo イベントより前の負位置は外挿で定義済み結果（PPQN-07）。
- 旧テストケース（`tick_to_media`/`musical_position`/`time_info`/`media_to_tick`）を `Ppqn(960)` 経路で再実行し同一結果（PPQN-11）。
- `BeatPosition::new(i64::MAX, 1)` 加算の `i128` 中間値超過、`media_to_beat` の大分母、及び `assert_no_alloc` 窓（PPQN-12）。
- CLAP: `beats_to_ticks(BeatTime::from_float(2.5)) == 2400`（960）、`BeatTime::from_bits` 半値境界の half-away 丸め、factor が `1<<31` であること（PPQN-09）。

## Guarantees

- 1/7 Beat 等の旧960 Tick に乗らない位置が正本として保持される。
- 異なる分解能の Tick が暗黙に比較・混合されない API 形態。
- 全ての変換失敗が定義済みエラー（`InvalidValue`/`TimeOverflow`）であり、未定義動作・パニック・f64 正本への退行がない。
- `time_info` の `ppqn` フィールドが投影に使った実分解能を正直に報告する。

## Non-goals

- Synvelis 側の Project PPQN 既定値・Undo・Save/Reload・Loop Note 配送・
  Export Loss UI（PPQN-01/02/06/08/10）— 本 spec の検証対象外。
- SMF SMPTE Division、Tempo Ramp、beat-native ABI snapshot（#190 以降）。
- Plugin 実機 E2E（L1 lane 対象外。adapter の純粋変換式のみ golden）。

## Risk

- 新正本が正規化・文脈付き投影を守らなければ、960 への暗黙量子化という
  旧来の情報損失が復活する。等値/順序の緩い実装は異 PPQN 混同を再導入
  する。
- CLAP factor を `65536`（旧 ADR 誤記）で固定すると plugin transport
  位置が 32768 倍ずれる。
