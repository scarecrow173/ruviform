---
id: "DESIGN-034JcePtzKFix4IuxyE5mD"
type: "design"
status: "approved"
title: "時間モデル拡張の設計（FrameRate/TimeBase 分離・Timecode 拡張・丸め誤差累積方針）"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/42"
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
    - "../../AGENTS.md"
    - "overview.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/time-type-extension-framerate-timecode-rounding.md"
    - "../adr/framerate-timebase-separation-and-timecode-extension.md"
    - "ruviform-architecture-design-baseline.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../discovery/explore-architecture-design-baseline.md"
    - "https://github.com/scarecrow173/ruviform/issues/27"
    - "https://github.com/scarecrow173/ruviform/issues/33"
    - "https://github.com/scarecrow173/ruviform/issues/26"
  defers: []
  deferred-by: []
---

# 時間モデル拡張の設計（FrameRate/TimeBase 分離・Timecode 拡張・丸め誤差累積方針）

## Context

Issue #42 は `ruviform-media-core` の時間モデルに対する 2 系統の拡張を要求する:

1. `FrameRate` / `TimeBase` の意味的分離と `Timecode` 拡張（型の責務分離・互換性方針・ドロップフレーム拡張性の分類）
2. 丸め誤差累積・sample/frame 精度の方針化（「呼び出しごとに最大 1 回の丸め」「安定アンカーからの変換」のユースケースへの適用と検証手段）

前提となる現行実装の事実（`crates/ruviform-media-core/src/`）:

- `TimeBase { numerator: u32, denominator: u32 }`（`time.rs` L31。private フィールド・gcd 正規化・`new() -> Result`・`sample_rate(rate)` → `1/rate`・`COMMON_90K`。`Ord` は意図的に未実装）
- `MediaTime { value: i64, base: TimeBase }`（Copy・負値=pre-roll・`convert`/`convert_exact`/`checked_add`/`checked_sub`/`to_seconds_f64`）
- `MediaDuration` 同形、`MediaRange::end()` はトータル関数（最終値のみ `i64` 飽和）・`checked_end(rounding)` あり
- `Rounding` enum（Floor/Ceil/TowardZero/AwayFromZero/NearestEven 既定、`#[non_exhaustive]`、`time.rs` L576）
- `Timecode { hours/minutes/seconds/frames: u32, drop_frame: bool }`（`timecode.rs` L22。`from_media_time(t, frame_rate: TimeBase)` は内部 `floor(t * rate)` で **fps 向き**の有理数を受ける設計。`from_media_time_drop` の drop 対応は `30000/1001`・`60000/1001` のみで他は `Err(UnsupportedDropFrameRate)`。負時刻は `Err(InvalidValue("negative time"))`、hours は unbounded）
- `Error` enum `#[non_exhaustive]`（`InvalidTimeBase`/`TimeOverflow`/`InvalidValue(&'static str)`/`UnsupportedDropFrameRate`）
- 変換内部は `i128`/`u128` 中間精度（`scaled_fraction`/`round_quotient`/`convert_value_wide` ヘルパ）
- 丸め累積の現行注記: `time.rs` モジュール doc（「at most one rounding — the final step」「stable anchor」「repeated base-hopping is discouraged」）
- 先例: `audio-core/src/time.rs` L33 `pub struct SampleRate(TimeBase)` — `TimeBase` を wrap して Hz 意味を与える newtype（`new(rate)` → `TimeBase::new(1, rate)`、`get()`・`time_base()` アクセサ）
- `stream.rs` の `StreamDescriptor.time_base: TimeBase`（pub フィールド）、video-core に frame rate 型は未存在（`VideoFrame.pts: MediaTime`）
- media-core は外部依存ゼロ（std のみ）

設計ベースライン（`ruviform-architecture-design-baseline.md`、approved）の確定契約:

- 異種 timebase 変換は厳密有理数、丸めは呼び出し側指定（既定 round-half-to-even）。**音声↔動画変換は音声サンプル時刻を優先**
- 動画時間 = コンテナ timebase の PTS + フレームインデックス。VFR は per-frame PTS 正、CFR 前提禁止
- ドロップフレームはタイムコードの表示・ラベリング規約、単調媒体時間と分離、media-core 表示変換層の責務
- 有理数時刻・変換・タイムコード表示・transport 値型 = `ruviform-media-core` / `MasterClock` = `ruviform-rt` / A/V 同期・シーク・VFR 実行時振る舞い = engines
- A/V 同期 = audio master（`MasterClock`）+ 動画 PTS 追従（ffplay 型）

ADR [framerate-timebase-separation-and-timecode-extension.md](../adr/framerate-timebase-separation-and-timecode-extension.md) で Option A（`FrameRate` = `TimeBase` newtype）・破壊的置換の互換性方針・ドロップフレーム分類が決定済み。本書はその API 形状と丸め誤差累積ポリシーを詳細化する。

## Scope

### In Scope

- `FrameRate` 型の API 形状（提案シグネチャ・不変条件・`TimeBase` との変換規約）
- `Timecode::from_media_time` / `from_media_time_drop` の新シグネチャ（`frame_rate: FrameRate`）
- 丸め誤差累積ポリシー（文書契約としての明文化・ユースケース別精度基準・許容閾値定義・検証手段）
- RT 契約ラベル方針・エラー契約
- 本設計論点の決定/保留/要PoC 分類表

### Out of Scope

- コード実装（後続の実装 Issue が `mise run check`/`lint`/`test` で検証する）
- `MediaTime::convert`/`MediaDuration::convert` の変換契約変更（#41 spec どおり現行維持）
- 音楽時間（小節/拍/PPQN・テンポマップ）— Issue #33 スコープ
- `MasterClock` の実装（ruviform-rt・Issue #27 系）
- `Timecode` → `MediaTime` 逆変換 API の確定（Deferred に記録）
- frame index 型の新設（media-core vs video-core の境界問題。Issue #26/#27 境界として Deferred に記録）
- VFR の per-frame PTS 表現自体の拡張（ベースライン決定済みの現行維持）

## Components and Boundaries

### `FrameRate` 型（media-core に配置）

**所有: `ruviform-media-core`**（`Timecode` 表示変換の責務所在であり、`StreamDescriptor` と同じく共有値型の層）。`SampleRate` と同じ newtype 構成を取る:

```rust
/// Nominal frame rate in frames/second (`24`, `30000/1001`, `60000/1001`, …).
///
/// Internally stores its fps rational in a [`TimeBase`] field — the
/// reciprocal of a frame duration — so the `rate > 0` invariant is
/// enforced structurally at construction. The stored value is NOT a tick
/// duration and is never exposed as a `TimeBase`.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FrameRate(TimeBase);

impl FrameRate {
    /// Creates a frame rate from a rational `numerator/denominator` fps.
    /// # Errors: `Error::InvalidValue("frame_rate")` when either is zero.
    pub const fn new(numerator: u32, denominator: u32) -> Result<Self>;

    /// Integer-fps shorthand: `FrameRate::new(fps, 1)`.
    /// # Errors: `Error::InvalidValue("frame_rate")` when `fps == 0`.
    pub const fn from_fps(fps: u32) -> Result<Self>;

    pub const fn numerator(self) -> u32;    // fps numerator
    pub const fn denominator(self) -> u32;  // fps denominator

    /// The frame duration as a [`TimeBase`] `denominator/numerator` — the
    /// reciprocal tick base in which one tick spans one frame.
    /// (e.g. `30000/1001` fps → `TimeBase::new(1001, 30000)`)
    ///
    /// The only `TimeBase`-returning accessor: the stored fps rational is
    /// not a tick duration and is therefore never exposed as `TimeBase`.
    pub const fn frame_duration(self) -> TimeBase;

    /// Nearest integer nominal rate (rounded half-up), used as the timecode
    /// label grid. `24` for `24000/1001`, `30` for `30000/1001`.
    pub const fn nominal_fps(self) -> u32;
}
```

設計上の決め事:

- **保持は fps 向き**（`numerator/denominator` = フレーム/秒）。`Timecode` の `floor(t * rate)` 計算・`nominal_fps` の label grid 決定・drop 判定の `(30_000, 1_001)`/`(60_000, 1_001)` マッチがそのまま `numerator()`/`denominator()` で書ける。`frame_duration()` が逆数（tick 長）を返す明示的な変換点になる
- **保持する fps 有理数を `TimeBase` 値として取り出す API は設けない**（`to_time_base()`・`From<FrameRate> for TimeBase` は提供しない）。保持値は `TimeBase` の契約（「1 tick の有理数長」）を満たさず、そのまま取り出せると `MediaTime::new(v, fr.into())` のような契約違反値の流通がコンパイルを通ってしまう。fps 有理数が必要な内部計算は `numerator()`/`denominator()`、tick 長が必要な用途は `frame_duration()` が担う
- **`frame_duration()` は `pub const fn`**（他アクセサとの const 対称）。保持値は構築時に gcd 正規化済み・非ゼロのため逆数化は失敗し得ず、`TimeBase::new(denominator, numerator)` の `Ok` 分岐を `match` で返す実装で const を維持できる
- **`new()` は `TimeBase::new(numerator, denominator)` に委譲**して gcd 正規化を再利用し、失敗を `Error::InvalidValue("frame_rate")` に正規化する（`SampleRate::new` の `Error::InvalidValue("sample_rate")` 先例と同じマッピング方針。`InvalidTimeBase` をそのまま返さず、引数名レベルの文脈に寄せる）
- **`Ord`/`PartialOrd` は付けない**（`TimeBase` が意図的に `Ord` 未実装である方針に倣う。fps 大小比較が必要になった時点で契約を決める）
- **`Debug` は `FrameRate(30000/1001)` 型の簡潔表示**に寄せる（`SampleRate` の `SampleRate(48000)` 先例）
- **`#[non_exhaustive]` は付けない**（ポリシー対象は公開 enum のみ。struct の不変条件は ctor で強制 — ADR 副次決定 3）

### `Timecode` API の新シグネチャ

```rust
impl Timecode {
    /// Non-drop label on `frame_rate`'s nominal grid.
    ///
    /// # Errors
    /// * `Error::InvalidValue("frame_rate")` — `frame_rate.nominal_fps() == 0`
    ///   (rate が 0 に丸められ nominal grid が成立しない場合)
    /// * `Error::InvalidValue("negative time")` — negative `t` (pre-roll)
    /// * `Error::TimeOverflow` — `i128` intermediate or `hours` range overflow
    pub fn from_media_time(t: MediaTime, frame_rate: FrameRate) -> Result<Self>;

    /// SMPTE drop-frame label; `frame_rate` must be `30000/1001` or
    /// `60000/1001` else `Err(Error::UnsupportedDropFrameRate)`.
    ///
    /// # Errors
    /// * `Error::UnsupportedDropFrameRate` — `frame_rate` is not
    ///   `30000/1001` or `60000/1001`
    /// * `Error::InvalidValue("negative time")` — negative `t` (pre-roll)
    /// * `Error::TimeOverflow` — `i128` intermediate or `hours` range overflow
    pub fn from_media_time_drop(t: MediaTime, frame_rate: FrameRate) -> Result<Self>;
}
```

- **破壊的置換**（ADR 副次決定 1）。旧 `frame_rate: TimeBase` シグネチャは併設しない
- 内部計算（`elapsed_frames` の `floor(t * rate)`・drop のラベルスキップ・`narrow_u32` 分解）は現行の厳密 `i128`/`u128` 整数演算を維持し、`frame_rate.numerator()`/`denominator()` をそのまま使う
- `nominal` label grid（`(2*num + den) / (2*den)` の round-half-up）の計算は `FrameRate::nominal_fps()` に移譲し、`Timecode` 側と `FrameRate` 側で規則を二重実装しない。分子・分母は `u64`（以上）へ拡張してから計算すること — `u32` 中間演算では `2*num` が num > 約 21 億で溢れる（現行 `timecode.rs` 実装も `u64::from` で拡張済み。実装 Issue #58 の受入条件に境界テストを明記）
- **`nominal == 0` の拒否契約は現行維持**: `from_media_time` は `frame_rate.nominal_fps() == 0`（nominal grid が 0 に丸められる rate、例: `1/1000` fps 相当）を `Err(Error::InvalidValue("frame_rate"))` で拒否する。現行 `sub_fps_rate_is_rejected`（timecode.rs テスト）と同契約の継続であり、`% nominal` の %0 panic 防止でもある。`FrameRate::nominal_fps()` 自体は infallible のままとし、検査は呼び出し側の `from_media_time` が行う。`from_media_time_drop` は NTSC 2 レート以外を先に `Err(UnsupportedDropFrameRate)` で拒否するためこの経路には到達しない

### 変換規約

- **`TimeBase` を返す API は `frame_duration()` のみ**とする（`TimeBase::new(denominator, numerator)` 相当の逆数 — 1 フレームを 1 tick とする真の tick 長）。保持する fps 有理数は `TimeBase` の契約（1 tick の有理数長）を満たさないため、`to_time_base()` / `From<FrameRate> for TimeBase` は設けない（`MediaTime::new(v, fr.into())` 型の時刻誤解をコンパイルで防ぐ — 混同を型で防ぐ分離目的の保全）
- fps 有理数そのものが必要な消費者（timecode 内部計算・nominal grid 決定・drop 判定）は `numerator()`/`denominator()` アクセサを使い、`TimeBase` 値を経由しない
- **`From<TimeBase> for FrameRate` は提供しない**（任意の tick 長を fps と解釈する暗黙変換を防ぐ。`FrameRate::new` 経由で意図を明示させる）

## Data and Control Flow

- **コンテナ → 時間型**: demux 層（media-io）がコンテナの stream timebase（`StreamDescriptor.time_base: TimeBase`）と fps 情報を読む。fps は `FrameRate` として `MediaTime`/`Timecode` 系 API に渡る。container timebase と `FrameRate` は別物であり混同しない（PTS は `TimeBase`、ラベル grid は `FrameRate`）
- **MediaTime → Timecode（表示境界）**: `MediaTime` が安定アンカーとして保持され、タイムコード表示・サニティチェックの境界でのみ `from_media_time(_drop)` に渡される。`Timecode` はラベルでありタイミング源ではない（モジュール doc の既存規約維持）
- **FrameRate → TimeBase**: `frame_duration()` が唯一の `TimeBase` 返却 API（真の tick 長）。timecode の内部計算は `numerator()`/`denominator()` を直接使用し、`TimeBase` 値を経由しない。A/V 同期の `MasterClock` との接続は engines 側（Issue #27）の責務で、media-core の値型は変換の起点と grid を提供するだけ

## 丸め誤差累積ポリシー

### ポリシー（文書契約として明文化）

現行の doc 注記（`time.rs` モジュール doc・`convert`/`checked_add` の doc）を以下の 3 原則として**ポリシー化**する:

1. **呼び出しごとに最大 1 回の丸め**: 1 回の変換・演算呼び出しが適用する丸めは最終ステップの 1 回のみ。中間値は `i128`/`u128` の厳密有理数で保持する（実装済みの現行契約。変更しない）
2. **安定アンカーからの変換**: 値はそのネイティブ base（音声なら `1/rate`、コンテナなら `StreamDescriptor.time_base`）で保持し、異種 base が必要な表示・I/O・同期の境界で都度アンカーから直接変換する
3. **base-hopping 禁止**: 変換結果をさらに別 base へ連鎖変換する（アンカーを通さず base を渡り歩く）ことを避ける。連鎖が必要な場合も各ホップはアンカー値から直接行い、前ホップの丸め済み値を次の入力にしない

**型強制しない理由（記録）**: 「安定アンカー」は文脈依存（A/V 同期では音声サンプル時刻、編集タイムラインではクリップのネイティブ base）であり、単一の型で表現できない。「連鎖変換を呼ばない」制約は session type 相当の状態追跡が必要で、値型 `Copy` の純粋関数 API には過剰設計と判断する。すべての変換 API が内部で厳密有理数演算を行うため、ポリシー遵守の責務は呼び出し側（engines・編集層）の設計規約として文書契約で担保し、検証はプロパティテストで行う。

### ユースケース別精度基準

| ユースケース | 要求精度 | 安定アンカー | 変換が許される境界 | 閾値 |
| --- | --- | --- | --- | --- |
| A/V 同期（audio master + video PTS 追従、ffplay 型） | **sample 精度**（音声側） | 音声サンプル時刻（`SampleIndex`/`1/rate` base、`MasterClock`） | 動画 PTS→表示 base 変換は表示・スケジューリング境界のみ。音声↔動画変換は音声サンプル時刻を優先（ベースライン確定） | 動画表示タイミングの誤差は音声マスター基準で評価し、表示 base の 0.5 tick 以下 |
| 編集タイムライン | **ネイティブ base 精度** | 各クリップのネイティブ `TimeBase` | 切り貼り境界（編集点・レンジ計算）のみ。クリップ内位置は変換しない | 編集点での変換誤差は宛先 base の 0.5 tick 以下（`NearestEven` 単回丸め） |
| Timecode 表示 | **frame 精度** | `MediaTime`（コンテナ timebase PTS 等） | 表示境界のみ。`floor(t * frame_rate)` の 1 回 | 表示フレーム番号の ±0（floor 規則で決定論的） |
| UI・メータ・概算表示 | 表示用（精度非保証） | 同上 | `to_seconds_f64()` 等の文書化済み表示変換のみ | 精度要求なし（中核表現に使わないことが前提） |

### 許容閾値の定義方法

- **単回変換**: `MediaTime::convert(to, Rounding::NearestEven)` の誤差は宛先 base の **0.5 tick 以下**に収まる（round-half-to-even は |誤差| ≤ 0.5 tick。exact-half tie は誤差がちょうど 0.5 tick のまま偶数側に落ちる — `round_quotient` の `twice == den` 分岐）。これを単位閾値とする
- **連鎖変換**: 各ホップの誤差は**そのホップの宛先 base の** 0.5 tick 以下。連鎖・往復の累積は中間 base の粒度に依存し、最終/元 base 換算では `N × 0.5` tick や 1 tick を大幅に超え得る（粗い中間 base 経由はその半 tick 分を丸ごと失う。例: 1000 tick @ `1/48000` → `1/24` 変換で 0.5 tick が `NearestEven` で 0 に丸められ、復帰時も 0 = 往復誤差 1000 tick）。ポリシー 3（base-hopping 禁止）はこの増分を排除するためのもので、**アンカーからの直接変換では常に宛先 base の 0.5 tick 以下に収まる**ことが保証の形になる
- **検証可能な記述**: 「変換チェインの累積誤差 ≤ 宛先 base の 0.5 tick」は、「全ホップがアンカーから直接変換される（連鎖しない）」構造的性質として表現し、テストでは往復変換（`t → base B → 元 base`）の誤差が**中間 base B の 0.5 tick を元 base 単位に換算した値 + 元 base の 0.5 tick**以下であること（往復 = 2 ホップであり、復帰ホップ自身の丸め項も計上する）、N 回連鎖の累積上界が **Σ（各ホップ宛先 base の 0.5 tick）を共通単位に換算した値**を超えないことを確認する

### 実装側検証手段

- **連鎖変換プロパティテスト**: 実装 Issue で、media-core の time 変換に対し次を検証するテストを追加する
  - 往復変換の誤差上界（`t.convert(B) → convert(元base)` で |往復誤差| ≤ 中間 base B の 0.5 tick を元 base 単位に換算した値 + 元 base の 0.5 tick。粗い中間 base 経由では元 base 換算で大きくなり得る点に注意 — 例: 中間 base `1/24` 経由の往復は元 base `1/48000` 換算で最大 1000 tick）
  - N 回連鎖の累積上界（規定 base 集合の巡回で |累積誤差| ≤ Σ（各ホップ宛先 base の 0.5 tick）の共通単位換算を確認し、ポリシー遵守時＝アンカーからの直接変換では宛先 base の 0.5 tick 以下に収まる対照実験）
  - 代表 base 集合（`1/48000`・`1/44100`・`1/90000`・`1001/30000`・`1001/60000`・`1/25`・`1/24` 等）× 境界値（0・i64 近傍・負値）の deterministic 列挙、または `proptest` 等のプロパティテスト
- **依存方針**: media-core は依存ゼロ（std のみ）のため、`proptest` を使う場合は **dev-dependency** として実装 Issue で理由記録のうえ追加する。依存を増やさない代替として、固定 seed・固定 base 集合の自前 deterministic 列挙テストも選択肢（依存追加規約は `AGENTS.md` 暫定規約どおり）

## Non-Functional Constraints

- **RT 契約**: `FrameRate`・変換規約・新 `Timecode` シグネチャはすべて `**RT contract:**` ラベルを付け、RT-suitable（`Copy` 値型・`const`/純粋整数演算・alloc なし）を維持する。未確定 API を RT-safe と断定しない（realtime ADR）
- **エラー契約**: `Error` は `#[non_exhaustive]` のまま。`FrameRate::new`/`from_fps` は既存 `Error::InvalidValue("frame_rate")` を再利用し、必要な場合のみ新 variant を追加する（`#[non_exhaustive]` で non-breaking 追加可）。`UnsupportedDropFrameRate` の契約（`30000/1001`・`60000/1001` のみ）は現行維持。`from_media_time` が `frame_rate.nominal_fps() == 0` を `Err(Error::InvalidValue("frame_rate"))` で拒否する現行契約（`sub_fps_rate_is_rejected` テストが担保）も維持する
- **`unsafe` 不使用・外部依存ゼロ**: media-core の現行方針を維持（`proptest` 等は dev-dependency まで）
- **`#[non_exhaustive]` ポリシー整合**: `FrameRate` は struct で対象外だが、将来の `Error` variant・`Rounding` 追加余地は確保される（enum ADR）

## 分類表（決定/保留/要PoC）

| 項目 | 分類 | 根拠 | 次のフォーラム |
| --- | --- | --- | --- |
| `FrameRate`/`TimeBase` 意味分離（newtype Option A） | 決定 | Issue #42 ADR（`SampleRate` 先例整合・意味論的分離・依存ゼロ） | 実装 Issue（media-core） |
| `Timecode` の `frame_rate` 引数の `FrameRate` 置換（破壊的変更） | 決定 | Issue #42 ADR（`publish = false`・外部利用者不在・KISS） | 実装 Issue（media-core） |
| `FrameRate` API 形状（`new`/`from_fps`/`numerator`/`denominator`/`frame_duration`/`nominal_fps`・`TimeBase` 露出は `frame_duration()` のみ） | 決定 | 本書 Components 節の提案シグネチャ | 実装 Issue（media-core） |
| ドロップフレーム対象レート集合（`30000/1001`・`60000/1001` のみ） | 決定（現行維持） | SMPTE 規格の定義域に一致・Issue #42 ADR 副次決定 2 | 実装 Issue（変更は新規 ADR/実装 Issue） |
| 丸め誤差累積ポリシー（単一丸め/回・安定アンカー・base-hopping 禁止の文書契約化） | 決定 | 本書「丸め誤差累積ポリシー」節・ベースライン時間モデル契約 | 実装 Issue（doc・プロパティテスト） |
| ドロップフレーム非 NTSC nominal 一般化・フィールドレート | 保留 | SMPTE 上定義外・現需要なし（ADR 副次決定 2） | インターレース/非 NTSC drop 需要発生時の再検討 |
| `Timecode` → `MediaTime` 逆変換 | 保留 | Issue #42 受入条件外・需要未確定 | 編集/NLE 系需要発生時の後続 Issue |
| frame index 型の所有（media-core vs video-core） | 保留 | Issue #26/#27 境界の設計問題 | 動画基盤/動画エンジン Issue |
| 連鎖変換プロパティテストの harness 選定（`proptest` dev-dep vs deterministic 列挙） | 要PoC | media-core 依存ゼロ方針と検証基盤の兼ね合いは実装評価 | 実装 Issue（media-core） |

※ 上表および本書の「実装 Issue」は Issue #58 を指す（#27 動画エンジンより先に完了させる前提）。

## Risks and Trade-offs

- **破壊的変更のワークスペース内波及**: `frame_rate: TimeBase` → `FrameRate` で既存呼び出し（media-core 内テスト・doctest）がコンパイルエラーになる → 影響はワークスペース内に限定され、実装 Issue で一括更新。`publish = false` の間に行うことが前提
- **`FrameRate` と `TimeBase` の変換方向の混同**: fps 保持と tick 長（逆数）の読み違えリスクは、`TimeBase` 返却 API を `frame_duration()`（真の tick 長）に一本化したことで大幅に緩和される。`numerator()`/`denominator()` は fps 有理数の成分として doc で明示し、`From<TimeBase> for FrameRate` は提供しない
- **ポリシーの文書契約への依存**: 丸め累積ポリシーは型で強制されず、呼び出し側の設計規約に依存する → プロパティテストで検証手段を確保し、違反が実害化する前に検出する。型強制は過剰設計として明示的に却下した判断を本書に記録済み
- **`nominal_fps` の round-half-up 規則**: label grid 決定が fps 近傍整数への丸めに依存する（24.975（`25000/1001`）系は 25 grid 等）→ 現行 `from_media_time` と同じ規則を `FrameRate` に移譲し、規則の二重実装を避ける

## Deferred Design Concerns

| 据え置く項目 | 理由 | 再着手トリガ | 無視した場合のリスク |
| --- | --- | --- | --- |
| `Timecode` → `MediaTime` 逆変換 | Issue #42 受入条件外。ラベル→時刻の逆解釈は需要未確定 | 編集/NLE 系機能で timecode 文字列→媒体時刻の需要発生 | 各呼び出し側が独自逆変換を持ち不整合が生じる |
| ドロップフレーム非 NTSC 一般化・フィールドレート | SMPTE 規格の定義域外・現行 2 レートで需要を満たす（ADR 副次決定 2・保留） | インターレース素材対応・非 NTSC drop の実需要 | 現行限定が暗黙の最終決定として固定化・後付けで拡張が破壊的変更化 |
| `vtc` 等外部クレートの再評価 | media-core 依存ゼロ・外部型非露出方針と衝突（ADR Option D 却下） | 依存追加規約の見直し・外部型露出方針の変更 | 依存ゼロ方針との不整合を暗黙に持ち込む |
| frame index 型の所有（media-core vs video-core） | Issue #26/#27 境界の設計問題であり本設計の決定範囲外 | 動画基盤/動画エンジン Issue での frame index 需要 | フレーム番号型の所在が曖昧なまま実装が進む |
| 音楽時間（小節/拍/PPQN・テンポマップ） | Issue #33 スコープ・ベースライン保留項目 | Issue #33（第 2 期・テンポマップ設計） | メディア時間と音楽時間の混在設計 |
| `FrameRate` の `Ord`/比較契約 | 現需要なし・`TimeBase` の `Ord` 未実装方針と整合 | fps 大小比較の需要発生 | 安易な比較実装が `TimeBase` の設計意図と衝突 |

## Verification Notes

- [ ] `FrameRate` が `TimeBase` newtype として提案シグネチャどおりに存在し、`new()`/`from_fps()` が zero 入力を `Err(InvalidValue("frame_rate"))` で拒否する
- [ ] `Timecode::from_media_time(_drop)` が `frame_rate: FrameRate` を受け取り、旧 `TimeBase` シグネチャが残っていない
- [ ] `from_media_time` が `frame_rate.nominal_fps() == 0` を `Err(InvalidValue("frame_rate"))` で拒否する回帰テスト（現行 `sub_fps_rate_is_rejected` 相当）が存在する
- [ ] `nominal_fps()` が `u64` 以上の中間演算で計算され、`u32::MAX` 級の極端有理数（`4294967295/1`・`1/4294967295`）の境界テストが存在する
- [ ] 時間精度テスト行列（音声 base 44.1/48/90 kHz × 映像 rate 24・25・`30000/1001`・`60000/1001`、負値拒否・境界値）が実装 Issue（#58）に含まれ、アンカー直接変換と連鎖変換の誤差上界を比較するテストが存在する
- [ ] drop 対応が `30000/1001`・`60000/1001` のみで他は `Err(UnsupportedDropFrameRate)` の現行契約が変わっていない
- [ ] `frame_duration()` が唯一の `TimeBase` 返却 API で、`to_time_base()`・`From<FrameRate> for TimeBase`・`From<TimeBase> for FrameRate` が存在しない
- [ ] 丸め累積ポリシー（3 原則・ユースケース別精度基準表・許容閾値定義・検証手段）が本文に記述されている
- [ ] 連鎖変換プロパティテスト（往復誤差上界 = 中間 base 0.5 tick の元 base 換算 + 元 base 0.5 tick・N 回連鎖累積上界 = Σ 各ホップ宛先 0.5 tick の共通単位換算）が実装 Issue で追加される計画になっている
- [ ] 本書の分類表と設計ベースラインの分類表の分類（決定/保留/要PoC）が一致している
- [ ] `audit_docs.js --type all --json` の findings が 0
