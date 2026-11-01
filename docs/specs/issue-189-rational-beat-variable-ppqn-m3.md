---
id: "SPEC-034Qf9Ze8jsyjHppvPkQup"
type: "spec"
status: "approved"
title: "Issue #189 Rational Beat Position + Variable PPQN (Migration M3)"
created: "2026-11-01"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/189"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/synvelis/issues/19"
    - "https://github.com/scarecrow173/synvelis/issues/20"
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
    - "docs/adr/musical-time-model.md"
    - "docs/designs/music-time-model.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/186"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/219"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/186"
  deferred-by: []
---

# Issue #189 Rational Beat Position + Variable PPQN (Migration M3)

## What Should Be Built

`ruviform-media-core` 内に、後継 Accepted ADR
`docs/adr/configurable-ppqn-resolution-independent-musical-time.md` が定める
**分解能非依存の正確な音楽位置（正規化有理数 Beat Position）**、
**検証済み可変 `Ppqn`**、**Tick と分解能文脈を束ねる型**、および
**Beat Position 基準の `TempoMap` / 変換 API** を実装する。
旧固定960実装（`Ppqn` が 960 のみ構成可能・`TempoMap` 内部の `960`/`3840`
リテラル）を、互換 Golden 比較つきで新しい正本へ移行する。

独立 Time crate への抽出は条件付き判断とし、本 M3 では media-core 内実装を
行い、抽出条件の評価結果を記録する。

## Why Now

- Synvelis #19（採用済み設計）が Project 可変 PPQN・分解能非依存の正確な
  音楽時刻・MIDI Source-native 時刻保持を要求しており、Ruviform 側の実装
  移行が未着手だった。
- Migration #192/#193 の M3 は #184（M1 Graph IR）の後続段であり、
  #190（Transport Span）・#188（Timed MIDI）・#186（Control）・#196
  （Plugin Adapter）が Beat→Frame / Plugin Position 変換契約を必要とする。
- 現行 `media-core/src/music.rs` は PPQN=960 固定であり、1/7 Beat のような
  960 整数 Tick に乗らない位置を正本として保持できない。

## Users and Value

- **Synvelis（DAW Consumer）:** Project PPQN 可変・1/7 Beat 等の正確な位置・
  Source-native MIDI 時刻保持・960 既定はアプリ Policy として利用できる。
- **AudioLens（非音楽 Consumer）:** source-native media time（raw PTS/DTS/
  timebase/source frame）への暗黙変換・丸め・情報破棄が生じないことの保証。
- **#190/#188/#186/#196 の実装者:** Beat→Frame / Native Timestamp /
  Plugin Position の変換式と丸め誤差上界の定義済み仕様を得られる。

## Scope

### In Scope

- `BeatPosition`：正規化有理数の拍位置（負位置・Overflow・分母上限を定義済み
  Reject とする `Copy` 値型・RT-suitable）。
- `Ppqn`：960 固定制約を解除した検証済み可変分解能型（検証範囲は設計確定）。
- `Tick`＋`Ppqn` を束ねる型：整数 Tick 値だけを異なる PPQN 間で比較・混合
  できない API 形態。
- `TempoMap`：Beat Position 基準の Tempo/Signature セグメントと
  Beat↔MediaTime / Tick↔MediaTime 変換（960 リテラル除去・非 RT compile・
  immutable RT query・境界の単回丸めを維持）。
- `MusicalPosition` / `MusicalTimeInfo` の可変分解能対応と旧 960 との意味的
  互換（同一 PPQN で同一結果を Golden で比較）。
- Golden 検証：PPQN 480/960/1000/1920、負 Pre-roll、Tempo Boundary、
  Overflow/分母上限、旧 960 新 960 互換。CLAP `clap_beattime`（2^31 固定
  小数点）変換を規格/adapter/テストと照合。
- 出力契約文書化：#190/#188/#186/#196 へ提供する Beat→Frame /
  Native Timestamp / Plugin Position 変換と丸め誤差仕様。
- `rt_` 無割当契約テストを含む検証と、PPQN-01〜12 該当分の実測証拠記録。

### Out of Scope

- 独立 `ruviform-time` crate の新設（条件付き判断・本 M3 では不実施を記録）。
- Synvelis 側 Project PPQN 既定値 UI / Undo / Save Schema / Clip/MIDI Source
  意味論・強制 Quantize —— Synvelis 所有。
- #190 の Transport Span Scheduler、#188 の Note Ledger、#186 の Parameter
  Event、#196 の Process Adapter —— 変換契約の提供のみ。
- SMF SMPTE Division 等非 PPQN Division の Import 対応範囲（別 PoC）。
- 既存 `Tick`/`MusicalTimeInfo` ABI/Wire 形式の破壊的変更（段階的移行方針）。
- Tempo 曲線・連続 Ramp 等の TempoMap 表現拡張（Piecewise-constant 維持）。

## Requirements

- R1: 音楽位置の正本は PPQN 整数格子に依存しない正規化有理数とし、
  1/7 Beat 等も丸めずに保持・比較・演算できる。
- R2: `Ppqn` は 1 以上の検証済み範囲で構成可能とし、960 のみを許す制約を
  除去する（960 既定は Synvelis Policy であり Core は他値を拒否しない）。
- R3: Tick 数値を PPQN 文脈なしに異分解能間で比較・合成する API を提供
  しない。変換は明示的な Beat 経由で行う。
- R4: `TempoMap` は非 RT compile・immutable・RT 有界無割当クエリを維持し、
  960 リテラルを除去する。変換は最終段で単回の定義済み丸め。
- R5: 旧固定 960 利用者へ段階的な互換経路を提供し、同一 PPQN では旧来と
  同一の意味・結果を保持する（Golden で証明）。
- R6: Overflow・分母上限・非表現値は暗黙丸めではなく定義済みエラーで
  拒否する。
- R7: 実装は `unsafe` なし・`f64` を正本表現に使わない・既存 lint/RT 規約
  に従う。
- R8: AudioLens の raw PTS/DTS/timebase/source frame 契約に対し、共有
  timestamp/serialization/feature を変えない（非退行）。

## Acceptance Criteria

- [ ] AC-1: `BeatPosition` が 1/7 Beat を正確に保持し、任意 PPQN の整数
  Tick へ暗黙量子化されない（PPQN-04）。
- [ ] AC-2: `Ppqn` が 480/960/1000/1920 等で構成でき、0 は定義済みエラーで
  拒否される。
- [ ] AC-3: 同じ整数 Tick 値で異なる PPQN の位置が誤比較・混合されず、
  明示変換が必要である（PPQN-05）。
- [ ] AC-4: PPQN 480/960/1000/1920 で Tick→Beat→Media→Beat（→Tick）を
  定義済み丸めポリシーで往復し、往復誤差が上界内である（PPQN-03/07）。
- [ ] AC-5: Tempo Change 境界が Beat/Media 両方向で正確に保持され、負
  Pre-roll 位置が外挿で定義済み結果を返す（PPQN-07）。
- [ ] AC-6: 旧固定 960 API と新 API（PPQN=960）で TempoMap 変換・Bar/Beat・
  `MusicalTimeInfo` が Golden 一致する（PPQN-11）。
- [ ] AC-7: Overflow・分母上限・範囲外値が定義済みエラーで拒否され、RT
  クエリが `assert_no_alloc` 窓内で無割当である（PPQN-12）。
- [ ] AC-8: CLAP `clap_beattime` が `1<<31` 固定小数点として adapter 実装
  と仕様で一致し、負位置・丸め・境界の Golden が記録される（PPQN-09）。
- [ ] AC-9: `MusicalTimeInfo`/Wire 形式の ABI・フィールド意味が旧来と
  互換で、`ppqn` フィールドが実分解能を正直に報告する。
- [ ] AC-10: Beat→Frame（MediaTime 変換）/ Native Timestamp / Plugin
  Position の変換式と丸め誤差上界が文書化され、#190/#188/#186/#196 から
  参照可能である。
- [ ] AC-11: PPQN-01〜12 のうち Ruviform 該当分の実測証拠が impl record
  に記録され、Synvelis 所有項目は範囲外として明示される。
- [ ] AC-12: 独立 Time crate 抽出の採否が条件と根拠つきで記録される。

## Deferred Design Concerns

- `ruviform-time` 独立 crate 化 | 理由: media-core 内で型/Owner を先に安定
  させる方針（Target Architecture §4「当面新設しない」）| 再検討条件:
  media-core の非時間型を要しない Consumer が実在した場合 | 放置リスク:
  抽出時の公開パス変更コスト。
- SMF SMPTE Division / MIDI2 timestamp 対応 | 別 PoC（Native MIDI Import 側
  の責務・#188 関連）。
- `MusicalTimeInfo` を超える beat-native ABI snapshot | #190 の Transport
  Span 契約で必要に応じて新設。
- Tempo Ramp/曲線、Bar numbering 意味論 | 現行 Piecewise-constant の範囲を
  維持し、拡張時に別設計。

## Sources and Relations

- 正本 ADR: `docs/adr/configurable-ppqn-resolution-independent-musical-time.md`
  （accepted、旧 `docs/adr/musical-time-model.md` を supersedes）。
- 要件元: Synvelis #19 / #20、`docs/designs/synvelis-driven-target-architecture-and-migration.md`
  （§4 media-core「Rational Beat/Validated PPQN」、§5.4、TA-009）。
- Baseline: `docs/discovery/issue-194-m0-baseline-audit.md`（現行固定 960
  実装・回帰緑の実測 Baseline）。
- 出力契約先: ruviform #190/#188/#186/#196。Consumer 非退行: #219 /
  AudioLens #127 / Synvelis #28。
