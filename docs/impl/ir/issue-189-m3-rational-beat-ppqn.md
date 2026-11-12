---
id: "IMPL-034QfBqtvLZf9NkDqTJnTy"
type: "impl"
status: "completed"
title: "Issue #189 M3 rational beat + variable PPQN implementation"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-189-m3-rational-beat-ppqn.md"
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/music.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/adapt.rs"
      - type: "file"
        path: "docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-189-t-1-beat-canonical-types.md"
    - "docs/tasks/issue-189-t-2-tempomap-beat-basis.md"
    - "docs/tasks/issue-189-t-3-golden-verification-impl-record.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/specs/issue-189-rational-beat-variable-ppqn-m3.md"
    - "docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md"
    - "docs/plans/implement-issue-189-m3-rational-beat-variable-ppqn.md"
    - "docs/test-specs/issue-189-m3-rational-beat-ppqn-tests.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #189 M3 rational beat + variable PPQN implementation

## Summary

`ruviform-media-core` の音楽時間を固定960整数Tickから、正規化有理数の
`BeatPosition` を正本とする分解能非依存モデルへ移行した（Accepted ADR
`configurable-ppqn-resolution-independent-musical-time.md` の M3 実装）。
`Ppqn` は `1..=u16::MAX` の validated 型、`TickPosition` が tick + 分解能を
束縛し、`TempoMap` は Beat 基準の正確有理数変換を提供する。
`MusicalTimeInfo`（24-byte ABI/wire）と `PPQN` 定数は不変のため、plugin 系
consumer は無変更でコンパイル・テストを通過する。

## Context

正本: `docs/adr/configurable-ppqn-resolution-independent-musical-time.md`
（accepted、旧 `musical-time-model.md` を supersede・旧文は履歴維持）。
計画: `docs/plans/implement-issue-189-m3-rational-beat-variable-ppqn.md`。
設計: `docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md`。

## Implementation

### Changes

- `crates/ruviform-media-core/src/music.rs`:
  - `Ppqn::new(u16) -> Result<Self>`（0 → `InvalidValue("ppqn")`）。
    `PPQN = 960` 定数・`ticks_per_quarter()` は互換維持。
  - `BeatPosition { num: i64, den: u32 }` — 構築時に gcd 正規化（`den>=1`、
    sign は num のみ、負値 = pre-roll）。`new`/`from_int`/`ZERO`/accessors/
    `checked_add`/`checked_sub`/`checked_neg`/`to_f64`/`Display`/`Ord`
    （`i64×u32` クロス積は常に `i128` 内 → 全順序）。表現不能は
    `TimeOverflow` で明示拒否（暗黙量子化なし）。
  - `TickPosition { tick: Tick, ppqn: Ppqn }` — `to_beat()`（正確・
    infallible）、`from_beat(beat, ppqn, Rounding)`（明示丸め）、
    `rescale`、`cmp_same_resolution`（異分解能比較は
    `InvalidValue("tick_ppqn_mismatch")`）。`PartialEq` は (tick,ppqn)
    ペア比較で異分解能の暗黙等価を型で防止。
  - `TempoEvent`/`TimeSignatureEvent` — `tick: Tick` → `position:
    BeatPosition`（破壊的・明示的移行）。`at(BeatPosition,…)` /
    `at_tick(TickPosition,…)` コンストラクタ。
  - `TempoMap` — `TempoSegment.start`/`SignatureSpan.start` を
    `BeatPosition` 化、`bar_len` を正規化有理数 beats 化し、非テスト
    コードから `960`/`3840` リテラルを除去。`seconds_per_beat =
    60_000/bpm_milli` の正確有理数累積。
  - 公開 API: `beat_to_media`（`NearestEven` 単回）/ `media_to_beat`
    （正確・分母 `u32` 超過は `TimeOverflow`）/ `tick_to_media` /
    `media_to_tick(time, ppqn)`（絶対拍への `Floor` 投影・旧ポリシー）/
    `musical_position(beat) -> Result<Option<MusicalPosition>>` /
    `time_info(beat, ppqn) -> Result<MusicalTimeInfo>`。
  - `MusicalPosition.tick_offset: u16` → `beat_offset: BeatPosition`。
- `crates/ruviform-media-core/src/lib.rs`: `BeatPosition`/`TickPosition`
  を re-export。
- `crates/ruviform-plugin-clap/src/adapt.rs`: 既存 `beats_to_ticks`
  （`bits×960/2³¹`・half-away）を golden テストで固定（テストのみ追加、
  変換コード不変更）。

### Decisions During Implementation

- **`musical_position`/`time_info` を `Result` 化**: 正確な `beat_offset`
  の分母が `u32` を超え得る場合と tick 投影の `i64` 溢れを暗黙に捨てず、
  設計 D8 の Failure 条件に従い `Result<Option<…>>`/`Result<…>` とした
  （design doc も同期更新済み）。
- **`media_to_tick` は絶対拍への `Floor` 単回投影**: 旧実装の
  `seg.start + floor(Δ)` と、整数拍アンカーで完全同一。非整数拍アンカー
  では「時刻を超えない」floor が旧来より定義として健全（新旧差が出うる
  のは non-integer-beat segment 開始のみ、PPQN-11 Golden は整数アンカーで
  完全一致）。
- **独立 Time crate 不抽出（D7）**: media-core 非時間型を要しない
  consumer が現時点で存在せず（plugin 系は `MusicalTimeInfo` 経由で
  media-core 全体に依存）、公開パス変更コストに対する利益がないため
  `music.rs` 内実装を維持。抽出条件は設計 D7 に記録済み。
- **CLAP adapter 変換式の不変更**: `beats_to_ticks` は公式
  `CLAP_BEATTIME_FACTOR = 1<<31` に対して `bits×960/2³¹`（half-away）で
  既に正しく、`MusicalTimeInfo.ppqn` が 960 を正直に報告するため M3 では
  コード不変更・Golden で固定した（Erratum は文書側で解消済み）。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

実行コマンドと結果（worktree `rf189-rational-beat`、Rust 1.99.0）:

| コマンド | 結果 |
| --- | --- |
| `mise run fmt-check` | PASS |
| `mise run check` (`cargo check --workspace --all-targets`) | PASS（全29クレート） |
| `mise run lint` (`cargo clippy --workspace --all-targets -- -D warnings`) | PASS |
| `mise run test` (`cargo test --workspace`) | PASS（`music::tests` 20 件 + doctests 含む全緑） |
| `mise run test-rt` (`cargo test --workspace rt_`) | PASS（`music::tests::rt_queries_no_alloc` 含む） |
| `mise run lint-l1-real-plugin` | PASS |
| `mise run test-l1-real-plugin` | PASS（実規格 L1 E2E 3 件 × CLAP/VST3） |
| `cargo test -p ruviform-plugin-clap --lib` | PASS（47 件、`beats_to_ticks_clap_factor_golden` 含む） |

### PPQN-01〜12 証拠（Ruviform 該当分）

| ID | 判定 | 証拠（テスト/根拠） |
| --- | --- | --- |
| PPQN-01（Synvelis: project 既定960） | **範囲外** | アプリ policy。Ruviform は `PPQN` 定数を互換用に残すのみ |
| PPQN-02（Synvelis: PPQN 設定/保存/再読込） | **範囲外** | Synvelis project 所有 |
| PPQN-03 可変 PPQN 構成 | **PASS** | `ppqn_validates`（0 拒否、480/960/1000/1920/u16::MAX 受理）、`variable_ppqn_media_roundtrip`（4 分解能で tick→media→tick ≤1 tick） |
| PPQN-04 1/7 Beat 等の非格子位置保持 | **PASS** | `beat_position_normalizes`（1/7 正規化・順序・Display）、`beat_position_off_grid`（1/7・4/7 beat アンカーの厳密変換）、`tick_position_projection`（PPQN-7 grid で 1 tick 厳密） |
| PPQN-05 異分解能 Tick の誤混合防止 | **PASS** | `tick_position_binds_resolution`（960t@960 ≠ 480t@480 as `TickPosition`、同 beat は `to_beat` で一致、跨り比較は `InvalidValue`）、`TickPosition` は `Ord` 非実装で暗黙順序なし |
| PPQN-06（Synvelis: Source-native import） | **範囲外** | `TickPosition`/`rescale` は保持基盤のみ提供 |
| PPQN-07 Tempo 境界/負 pre-roll/往復 | **PASS** | `tempo_boundary_anchor_exact`（両方向厳密・4 分解能で anchor = `ppqn` tick）、`preroll_negative_position`（-0.5s/-1.75s 外挿・tick 級）、`variable_ppqn_media_roundtrip` |
| PPQN-08（Synvelis: PPQN 変更非破壊） | **範囲外** | アプリ undo/migration 所有 |
| PPQN-09 CLAP 2^31 実装照合 | **PASS** | `beats_to_ticks_clap_factor_golden`（`FACTOR == 1<<31`、整数 beat 厳密、逆変換往復、負 half-away 境界 ±1_118_481/2、i64 全域で wrap なし）+ 既存 `adapt_transport_maps_musical_fields` の 0.5 tick 境界 |
| PPQN-10（Synvelis: undo/UI/migration） | **範囲外** | アプリ所有 |
| PPQN-11 旧960↔新960 Golden 一致 | **PASS** | `ppqn_960_golden_compat`（旧全テストケースを `Ppqn(960)` 経路で再現: 定テンポ/anchor/milli-BPM/pre-roll/bar-beat/`time_info` 同一値）、`musical_position_decomposes`（tick 30@960 → offset 1/32 beat） |
| PPQN-12 Overflow/分母上限/範囲/RT | **PASS** | `beat_position_arithmetic`（i64 超過・分母 `u32` 超過・`i64::MIN` neg → `TimeOverflow`）、`BeatPosition::new(_,0)`/`Ppqn::new(0)` → `InvalidValue`、`conversion_overflow_is_explicit`（media→beat/tick の `i64` 超過 → `TimeOverflow`、飽和なし）、`rt_queries_no_alloc`（全クエリ `assert_no_alloc` 窓内） |

`MusicalTimeInfo` ABI: `musical_time_info_is_copy_24b`（Copy・24-byte）で
不変を固定。`time_info_variable_ppqn`（480 投影で tick 値半減・`ppqn`
フィールドは実分解能報告）。

### 出力契約（#190/#188/#186/#196 向け仕様）

設計 doc「Adapter / 規格写像の契約」表が正本。要約:

| 変換 | API/式 | 丸め | 誤差上界 |
| --- | --- | --- | --- |
| Beat→Frame (#190) | `beat_to_media(beat, TimeBase::sample_rate(rate))` | `NearestEven` 単回 | ≤ 0.5 frame |
| MediaTime→Beat | `media_to_beat(time)` | なし（正確） | 0（分母超過は `TimeOverflow`） |
| Beat→Tick(ppqn) | `TickPosition::from_beat(beat, ppqn, r)` | 明示 `Rounding` | ≤ 選択丸め 1 tick |
| MediaTime→Tick(ppqn) | `media_to_tick(time, ppqn)` | `Floor` | ≤ 1 tick |
| CLAP `clap_beattime`↔Tick | `bits = round_half_away(beat×2³¹)` 逆: `bits×ppqn/2³¹` | half-away | ≤ 0.5 変換先単位 |
| VST3 `TQuarterNotes`↔ | `beat.to_f64()` / `round(ppq×ppqn)` | f64 最近接 | ≤ 0.5 ulp / adapter 側 |
| Beat→`MusicalTimeInfo` | `time_info(beat, ppqn)` | `NearestEven` | ≤ 0.5 tick @ppqn |
| Native Timestamp 保持 | `TickPosition` = (tick,ppqn) 非破壊、`rescale` 明示のみ | — | 無変換時 0 |

AudioLens 非退行: 本変更は `MusicalTimeInfo` の ABI/wire 形式・
`MediaTime`/`TimeBase`/serialization を一切変更していない。
AudioLens が依存する source-native media time（PTS/DTS/timebase/
source-frame）の型・意味は不変であり、音楽時間側の変更は
`music.rs` の Beat/TempoMap 内部に限定されるため影響なし。

## Risks

- `musical_position`/`time_info`/`media_to_tick` 等のシグネチャ変更は
  将来の呼び出し側（Synvelis 側 consumer）で移行が必要 — 意図的な破壊的
  変更であり、`Tick` 値の暗黙 960 解釈を排除するための措置。
- `media_to_beat` は正確だが、極端な base（巨大素数分母）では結果分母が
  `u32` を超え `TimeOverflow` になり得る — 定義済みの明示拒否。
- 旧 `time_info` の `bar_start_tick` 溢れ時 `unwrap_or(0)` は
  `Err(TimeOverflow)` へ変更 — 沈黙の 0 返却を排した意図的な semantic
  tightening。

## Follow-ups

- #190 Transport Scheduler / Span: `beat_to_media` 契約で Beat→Frame を
  接続（本契約表参照）。
- #188/#186/#196: `TickPosition`/`TickPosition::from_beat` と
  `MusicalTimeInfo(ppqn)` 投影で consumer 側変換を実装。
- 独立 Time crate 抽出は条件充足時に別 Issue で再評価（設計 D7）。
