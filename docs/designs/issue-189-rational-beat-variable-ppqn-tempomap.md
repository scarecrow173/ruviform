---
id: "DESIGN-034QfaKklvNoBLtP5JVkOj"
type: "design"
status: "approved"
title: "Issue #189 M3 Rational Beat / Variable PPQN media-core design"
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
  implements:
    - "docs/specs/issue-189-rational-beat-variable-ppqn-m3.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/music-time-model.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-189-rational-beat-variable-ppqn-m3.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #189 M3 Rational Beat / Variable PPQN media-core design

## Context

現行 `crates/ruviform-media-core/src/music.rs` は PPQN=960 固定:
`Ppqn(u16)` は `pub const PPQN = Ppqn(960)` でしか構成できず、`Tick(i64)`
が暗黙に 960 分解能を持ち、`TempoMap` 内部に `960`/`3840` リテラルが残る。
後継 ADR は「正規化有理数 Beat Position を正本・可変 PPQN・Tick+分解能
文脈・TempoMap の Beat 基準化」を採択済み。本書は M3 の media-core 内
実装設計を確定する。**旧固定960 API の本文・設計書を無言で置換しない**
—— 旧型は互換経路として残り、新しい正本型が並立する。

前提測定（M0 #194、`docs/discovery/issue-194-m0-baseline-audit.md`）:
`TempoMap`/`TempoEvent`/`MusicalPosition`/`Tick`/`TickDuration` の外部
consumer は存在しない（media-core 内部のみ）。`MusicalTimeInfo` は
plugin-core wire / plugin-clap / plugin-vst3 / transport が使う 24-byte
`Copy` ABI で、`ppqn: u16` フィールドが既に存在する。

## Design Decisions

### D1. `BeatPosition` — 正規化有理数の拍位置（正本）

```rust
pub struct BeatPosition { num: i64, den: u32 }
```

- 不変条件: `den >= 1`、`gcd(|num|, den) == 1`（構築時に必ず正規化）、
  符号は `num` のみが持つ。負位置（Pre-roll）を許す。
- **幅の選定根拠:** `num: i64` は `Tick`/`MediaTime` と同じ 64bit 位置幅。
  `den: u32` を選ぶ理由は比較の全域性: `|num * den_other| <= 2^63 * 2^32
  < 2^127` で `i128` 中間値が**常に**収まり、`Ord` を Result なしの全
  関数として実装できる（`u64` den では境界で `i128` を溢れ得る）。
- 分母上限: `u32` 型幅そのもの（`<= 4_294_967_295`）。加減算などで正規化
  後も `u32`/`i64` に収まらない結果は `Error::TimeOverflow` で明示拒否。
  無制限分母を保証しない（ADR の有限計算能力の明示）。
- API: `new(num, den) -> Result<Self>`（`den == 0` は
  `InvalidValue("beat_denominator")`）、`from_int`、`num`/`den`、
  `checked_add`/`checked_sub`/`checked_neg`、`cmp`(Ord)、`to_f64`（明示
  損失変換）、`Display`（`num/den`、`den==1` なら整数表示）。
- `Copy` 値型・`f64` を正本表現に使わない。等値は正規化後の `(num,den)`
  一致＝有理数等値。

### D2. `Ppqn` — 検証済み可変分解能

- `Ppqn(u16)` を維持し、`pub const fn new(value: u16) -> Result<Self>` で
  `1..=u16::MAX` を許可（`0` は `InvalidValue("ppqn")`）。
- **範囲根拠:** `MusicalTimeInfo.ppqn: u16` と wire codec（u16）の既存 ABI
  が上限 `65535` を既に規定しており、SMF division（15bit, 32767）と実用
  PPQN（〜15360）を十分にカバーする。型幅を広げる利益はなく ABI を壊す。
- `pub const PPQN: Ppqn = Ppqn(960)` は**旧固定960の legacy resolution 定数**
  として維持（adapter が 960 投影を明示指定するための名前）。
  `Ppqn::default()` は新設しない —— 既定値 960 は Synvelis Project
  Policy であり Core の不変条件ではない。

### D3. `TickPosition` — Tick + 分解能文脈の束ね

```rust
pub struct TickPosition { tick: Tick, ppqn: Ppqn }
```

- `new(tick, ppqn)`、`tick()`、`ppqn()`、`to_beat() -> BeatPosition`
  （`tick/ppqn`、正確）、`from_beat(beat, ppqn, rounding) -> Result<Self>`
  （単回丸め）、`rescale(target, rounding)`。
- `PartialEq`/`Eq` は `(tick, ppqn)` の**表現等値**: 異なる PPQN の同じ
  Tick 値は等しくない（誤混合を等値で黙認しない）。`Ord` は実装しない
  —— 音楽的比較は `to_beat()` 経由を必須とする。`Add`/`Sub` も提供
  しない（裸 `Tick` 同士の算術は同一分解能文脈内の既存契約として残る）。
- 名前は `MediaTime{value, base}` と同型の「値+文脈」対に倣う。

### D4. `TempoMap` — Beat 基準化（960 リテラル除去）

- 内部 `TempoSegment.start_tick: i64` → `start: BeatPosition`。
  累積秒 `start_num/start_den: i128/u128` はそのまま（既に有理数）。
  `SignatureSpan` も Beat 基準にし、`bar_len_ticks: u64` → 有理数の
  `bar_len`（beats = `numerator * 4 / denominator`、正規化保持）。
- `TempoEvent`/`TimeSignatureEvent` の `tick: Tick` フィールドを
  `position: BeatPosition` へ変更（**コンパイルを壊す明示的移行** ——
  黙った意味変化を避けるため型・名の両方を変える）。Tick 起点の構築は
  `at_tick(TickPosition, ...)` 補助コンストラクタで明示文脈つきに限定。
- 変換式: `seconds_per_beat = 60_000 / bpm_milli`（旧式の
  `60_000/(bpm*960)` の `960` は tick→beat 換算因子であり、Beat 基準では
  消える）。`beat_to_media` は有理秒→最終 base で `Rounding::NearestEven`
  単回丸め（旧 API と同一ポリシー）。
- 公開 API:
  - `beat_to_media(BeatPosition, TimeBase) -> Result<MediaTime>`
  - `media_to_beat(MediaTime) -> Result<BeatPosition>`（有理数で**正確**、
    結果分母が `u32` に収まらない場合のみ `TimeOverflow` で明示拒否）
  - `tick_to_media(TickPosition, TimeBase)` / `media_to_tick(MediaTime,
    Ppqn) -> Result<TickPosition>`（Beat 経由の明示投影、media→tick は
    絶対拍位置への `Floor` 単回丸め —— 旧実装と同一ポリシー）
  - `musical_position(BeatPosition) -> Result<Option<MusicalPosition>>`
    （`None` = signature 未適用；`beat_offset` の正確分母が `u32` を
    超える場合は `TimeOverflow` で明示拒否）
  - `time_info(BeatPosition, Ppqn) -> Result<MusicalTimeInfo>`（`NearestEven`
    投影、`ppqn` フィールドは渡された実分解能を正直に報告）
- Piecewise-constant tempo、非 RT compile、immutable `Box<[T]>`、O(log n)
  無割当バイナリサーチの RT 契約は全て維持。

### D5. `MusicalPosition` — 分解能非依存化

- `tick_offset: u16` → `beat_offset: BeatPosition`（現 beat 内の正確な
  有理数オフセット、`[0, 4/denominator)` beats）。`bar`/`beat` の意味は
  不変。media-core 内部専用のため破壊的変更を許容し、Tick 投影は呼び出し
  側の `rescale` に委ねる。

### D6. `MusicalTimeInfo` — ABI 不変・意味の正直化

- 24-byte 構造・wire 形式は**不変**。`position_ticks`/`bar_start_tick` は
  `time_info` に渡された `ppqn` での投影値、`ppqn` フィールドはその実
  分解能を報告する。`ppqn == 960` で旧来と同一の意味・値（Golden で
  検証）。beat-native snapshot は #190 の契約で必要に応じて別設計とし、
  本 M3 では ABI を壊さない。

### D7. Crate 配置 — 独立 Time crate 不抽出

- Target Architecture §4 の「`ruviform-time` 当面新設しない —— media-core
  内で型/Ownerを先に安定」に従い、M3 では media-core 内 `music.rs` に実装
  する。**抽出条件**: media-core の非時間型（stream/observe/fact/
  timecode）を要しない Consumer が実在し、依存分離の利益が公開パス変更
  コストを上回る場合。現時点でその Consumer は存在しない（plugin 系は
  `MusicalTimeInfo` 経由で既に media-core 全体に依存）。
- music.rs 内の構成分割: 型ブロック（Beat/Ppqn/Tick 系）→ イベント型 →
  TempoMap → tests。独立 module 分割はファイル肥大化が実害になった時点で
  判断する。

### D8. 丸めポリシーと誤差上界（出力契約の根拠）

- Beat→MediaTime: `NearestEven`、誤差 ≤ 0.5 目標 base tick。TempoMap 内部は
  正確有理数で誤差を累積しない。
- MediaTime→Beat: **正確**（丸めなし）。分母が `u32` を超える場合のみ
  `TimeOverflow` 明示拒否。
- Beat→Tick(ppqn): 明示 `Rounding`（`NearestEven`/`Floor` 等）、誤差 ≤
  選択丸め 1 tick。`NearestEven` は unbiased で往復安定。
- MediaTime→Tick: `Floor`（旧来互換）。往復ドリフト ≤ 1 tick。
- 旧→新 960 Golden: 全ての旧テストケースを `Ppqn(960)` 経由で再現し同一
  結果を要求。

## Adapter / 規格写像の契約（#190/#188/#186/#196 への出力）

汎用時間型は media-core、規格写像は各 Adapter 側（ADR Decision 6）。
M3 が提供・文書化する変換仕様:

| 変換 | 式 | 丸め | 誤差上界 |
| --- | --- | --- | --- |
| Beat→Frame(#190) | `beat_to_media(beat, TimeBase::sample_rate(rate))` | `NearestEven` 単回 | ≤ 0.5 frame |
| Beat→CLAP `clap_beattime` | `bits = round_half_away(num * 2^31 / den)` | half away from zero | ≤ 0.5/2^31 beat |
| CLAP `clap_beattime`→Tick(ppqn) | `ticks = round_half_away(bits * ppqn / 2^31)` | half away | adapter 既存 `beats_to_ticks` |
| Beat→VST3 `TQuarterNotes` | `beat.to_f64()` | f64 最近接 | ≤ 0.5 ulp（~2^-53 相対） |
| VST3 PPQ→Tick(ppqn) | `round(ppq * ppqn)` | f64 nearest | adapter 既存 `ppq_to_ticks` |
| Beat→`MusicalTimeInfo` | `time_info(beat, ppqn)` | `NearestEven` | ≤ 0.5 tick @ppqn |
| Native Timestamp 保持 | `TickPosition` は `(tick,ppqn)` 非破壊保持、`rescale` は明示のみ | — | 無変換時 0 |

- CLAP factor は `1 << 31`（公式 fixedpoint.h。旧 ADR の「1/65536」記述は
  横断監査で Erratum 済み —— **文書修正ではなく実コード照合を行う**）。
- `MusicalTimeInfo` は自己記述（`ppqn` フィールド）のため、adapter が
  960 で投影しても正確 —— M3 では adapter コードを変更せず Golden で
  既存式の正当性を固定する。

## Failure / Reject 条件（PoC 判定基準）

- `BeatPosition::new(_, 0)` → `InvalidValue("beat_denominator")`。
- `Ppqn::new(0)` → `InvalidValue("ppqn")`。
- 加減算・変換の中間/結果が `i64`/`u32`/`i128`/`u128` 境界超過 →
  `Error::TimeOverflow`（暗黙丸め・飽和なし）。
- `media_to_beat` の結果分母が `u32` 超過 → `TimeOverflow`。
- TempoMap: tempo 空・`bpm_milli` 範囲外・signature `numerator==0`・
  `denominator` 非2冪・重複 BeatPosition → 既存 `InvalidValue` を維持。
- RT: 全クエリ API を `assert_no_alloc` 窓内で無割当・有界（`rt_` テスト）。

## Alternatives Considered

| Option | Decision |
| --- | --- |
| `num: i128, den: u128` BeatPosition | 32B・比較が Result 化し得るため不採用 |
| `den: u64` | 比較の `i128` 中間値が境界で溢れ得るため不採用 |
| `num: i64, den: u32` | **採用** — 比較が全域で `i128` に収まり `Ord` が全関数 |
| TempoEvent.tick を `Tick` のまま残し暗黙960継続 | 黙った意味継続は ADR の「暗黙正本としない」に反するため不採用 |
| 新 `BeatTempoMap` 並設（旧 TempoMap 温存） | 同一ドメイン規則の二重実装を生むため不採用（互換は API 経路で担保） |
| `ruviform-time` 独立 crate | 条件未成立・当面 media-core（D7） |
| MusicalTimeInfo を beat-native 化 | 24B ABI/wire 破壊。段階移行のため本 M3 では不採用 |

## Verification Plan（Golden 対応表）

- PPQN-03/04/05: `TickPosition`/`BeatPosition` の単体・往復・誤混合防止。
- PPQN-07: tempo boundary anchor exact・負 pre-roll 外挿・丸め一貫。
- PPQN-09: CLAP `2^31` golden（spec 式 vs adapter 式）、VST3 PPQ。
- PPQN-11: 旧960 テスト群の新 API 再現（同一結果）。
- PPQN-12: overflow/den bound/範囲/`rt_` no-alloc/既存全スイート回帰。
- PPQN-01/02/06/08/10: Synvelis 所有（Project Settings/Undo/Save/Loop
  Note配送/Export Loss UI）—— Ruviform 側は前提契約のみ提供し範囲外と
  明示。
