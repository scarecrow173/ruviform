---
id: "PLAN-034QfbOOAnZw32cmX1H46E"
type: "plan"
status: "approved"
title: "Implement Issue #189 M3 Rational Beat + Variable PPQN in media-core"
created: "2026-11-01"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/189"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-189-rational-beat-variable-ppqn-m3.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/musical-time-model.md"
    - "docs/designs/music-time-model.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-189-rational-beat-variable-ppqn-m3.md"
    - "docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-189-m3-rational-beat-ppqn-tests.md"
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/186"
  deferred-by: []
---

# Implement Issue #189 M3 Rational Beat + Variable PPQN in media-core Implementation Plan

## Goal

design `DESIGN-034QfaKklvNoBLtP5JVkOj` どおりに、`ruviform-media-core` の
`music.rs` を Beat Position 正本 + 検証済み可変 `Ppqn` + `TickPosition` +
Beat 基準 `TempoMap` へ移行し、Golden/RT テストと出力契約文書で PPQN 系
受入証拠を揃える。

## Scope

1 本の実装ストリーム。新正本型 → TempoMap 一般化 → Golden/文書化は同一
ファイル群上の依存 critical path であり分割しない。`MusicalTimeInfo` 利用
consumer（plugin-core/clap/vst3/transport）は ABI 不変のためコード変更なし。
独立 `ruviform-time` crate 抽出・#190/#188/#186/#196 の実装・Synvelis 側
Project 設定は範囲外。

## Upstream Documents

- `docs/specs/issue-189-rational-beat-variable-ppqn-m3.md`（approved・AC-1〜12）
- `docs/adr/configurable-ppqn-resolution-independent-musical-time.md`（accepted 正本）

## Design Inputs

- `docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md`（approved・D1〜D8）

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/ruviform-media-core/src/music.rs` | `BeatPosition`/`Ppqn`/`TickPosition` 新設、`TempoEvent`/`TimeSignatureEvent`/`MusicalPosition` の Beat 化、`TempoMap` Beat 基準化、tests | 唯一のコード変更ファイル |
| `crates/ruviform-media-core/src/lib.rs` | re-export 追加（`BeatPosition`/`TickPosition`）と crate doc 更新 | 旧型は残す |
| `docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md` | 出力契約・丸め誤差仕様の正本（作成済み） | #190/#188/#186/#196 参照先 |
| `docs/impl/ir/issue-189-m3-rational-beat-ppqn.md` | 実測証拠・PPQN-01〜12 判定・crate 抽出採否の記録 | T-3 で作成 |

## Implementation Sequence

1. `music.rs`: `Ppqn` に `new(u16)->Result` を追加（960 固定解除）、
   `BeatPosition`（i64/u32 正規化有理数・Ord/checked 演算/Display）、
   `TickPosition`（tick+ppqn 束ね・to_beat/from_beat/rescale）を実装。
2. `TempoEvent`/`TimeSignatureEvent` を `position: BeatPosition` 化
   （`at_tick` 補助付き）、`TempoSegment`/`SignatureSpan` を Beat 基準化、
   `build()` から `960`/`3840` リテラルを除去。
3. `TempoMap` 公開 API を `beat_to_media`/`media_to_beat`/`tick_to_media`/
   `media_to_tick`/`musical_position`/`time_info`（BeatPosition+Ppqn）へ
   移行。`MusicalPosition.beat_offset: BeatPosition` 化。
4. `lib.rs` re-export/crate doc 更新。既存テストを新 API へ移植しつつ
   PPQN-03/04/05/07/09/11/12 の Golden・`rt_` no-alloc を追加。
5. impl record に実測証拠・PPQN-01〜12 判定・Time crate 不抽出の記録。

## Dependencies and Risks

- `TempoEvent{tick}`→`{position}` は公開フィールドの破壊的変更だが、
  consumer は media-core 内のみ（M0 測定済み）。`MusicalTimeInfo` ABI は
  不変のため plugin 系 crate は無変更で回帰可能。
- `BeatPosition` 分母上限 `u32`・`media_to_beat` 結果分母オーバーフローは
  `TimeOverflow` 明示拒否（暗黙丸めなし）。
- CLAP factor は `1<<31`（旧 ADR の 65536 記述は Erratum 済み）。adapter の
  実式を golden で固定する —— 文書 Erratum をもって実変換が直ったと
  みなさない。

## Task Breakdown

- T-1: 新正本型（`BeatPosition`/`Ppqn`/`TickPosition`）実装 + 単体テスト。
- T-2: `TempoMap` Beat 基準化 + イベント/位置/ABI 対応 + `lib.rs` 更新。
- T-3: Golden/RT 検証一式 + impl record + 出力契約参照の確定。

## Verification

- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` / `mise run test-rt`
- [ ] PPQN 480/960/1000/1920 の Tick→Beat→Media→Beat 往復（誤差 ≤ 1 tick）
- [ ] 旧 960 テストケースの新 API 同一結果 Golden
- [ ] CLAP `beats_to_ticks` が `2^31` spec 式と一致（adapter テスト）
- [ ] `assert_no_alloc` 窓内で全 TempoMap/BeatPosition クエリ無割当

## Open Questions / Gaps

- `media_to_beat` の分母上限 `u32` で十分か（u64 にしない根拠は比較の全関数性。実害が出た場合は別設計）。
- beat-native ABI snapshot（`MusicalTimeInfo` 後継）は #190 契約確定まで保留。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none（T-1→T-2→T-3 は同一ファイル依存）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `BeatPosition` の整数幅/分母上限（i64/u32）と `Ppqn` 範囲（1..=65535）の
  PoC 判定根拠は design D1/D2 に記録。レビュー焦点は丸めポリシー単回性と
  旧 960 Golden の同値性。
