---
id: "TASK-034JNwTz163N6eT3EJfdaD"
type: "task"
status: "todo"
title: "FrameRate 型の実装（TimeBase newtype・検証付きコンストラクタ・nominal_fps）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/58"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-58-time-type-extension.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-58-time-type-extension.md"
  blocks:
    - "migrate-timecode-to-framerate.md"
    - "add-time-precision-tests.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-58-time-type-extension.md"
    - "../designs/time-model-extension.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/time-type-extension-contracts.md"
  references: []
  defers: []
  deferred-by: []
---

# FrameRate 型の実装（TimeBase newtype・検証付きコンストラクタ・nominal_fps）

## Work

plan `PLAN-034JNstCXHhzzwOvo9iiCv` の Phase T1。`crates/ruviform-media-core/src/time.rs` に `FrameRate`（`TimeBase` の newtype・fps 向き有理数を保持）を追加する。Issue #42 設計文書の提案シグネチャどおりに実装し、`Timecode` 移行（T2）と精度テスト（T3）の前提となる型を確定させる。

- [ ] `pub struct FrameRate(TimeBase)` を `time.rs` に追加（`#[derive(Copy, Clone, PartialEq, Eq, Hash)]`。`Ord`/`PartialOrd` は付けない — `TimeBase` の意図的 `Ord` 未実装方針に倣う。`#[non_exhaustive]` 対象は公開 enum のみで struct は対象外 — 不変条件は ctor で強制）
- [ ] `pub const fn new(numerator: u32, denominator: u32) -> Result<Self>` — `TimeBase::new(numerator, denominator)` に委譲し gcd 正規化を再利用。失敗は `Err(Error::InvalidValue("frame_rate"))` に正規化（`InvalidTimeBase` をそのまま返さず引数名レベルの文脈に寄せる — `SampleRate::new` の `"sample_rate"` 先例）
- [ ] `pub const fn from_fps(fps: u32) -> Result<Self>` — `new(fps, 1)` 相当の shorthand。`fps == 0` で `Err(Error::InvalidValue("frame_rate"))`
- [ ] `pub const fn numerator(self) -> u32` / `denominator(self) -> u32` — fps 有理数の成分アクセサ
- [ ] `pub const fn frame_duration(self) -> TimeBase` — 逆数 `TimeBase::new(denominator, numerator)`。保持値は構築時に非ゼロ・正規化済みのため失敗し得ない（`match` で `Ok` 値を取り出し const を維持）。**唯一の `TimeBase` 返却アクセサ**。`to_time_base()`・`From<FrameRate> for TimeBase`・`From<TimeBase> for FrameRate` は設けない（保持する fps 有理数は `TimeBase` の「1 tick の長」契約を満たさず、露出すると `MediaTime::new(v, fr.into())` 型の混同がコンパイルを通る）
- [ ] `pub const fn nominal_fps(self) -> u32` — round-half-up `(2*num + den) / (2*den)` を **`u64` 中間演算**で計算（`u32` 中間では `num > ~2^31` で `2*num` が溢れる）。`u32` への縮小は結果が `u32::MAX` を超えない（`num ≤ u32::MAX`・`den ≥ 1` より）ので `as u32`/`try_from` で安全
- [ ] `fmt::Debug` を `FrameRate(30000/1001)` 型の簡潔表示に（`SampleRate(48000)` 先例）
- [ ] 全公開 API に `**RT contract:**` ラベル（RT-suitable — `Copy`・`const`・純粋整数演算・alloc なし）
- [ ] 単体テスト: gcd 正規化（`new(60_000, 2_002)` ≡ `new(30_000, 1_001)`）、ゼロ拒否 3 経路、`frame_duration()` 逆数（`30000/1001` fps → `TimeBase(1001, 30000)`）、`nominal_fps()` の round-half-up（`from_fps(24)`→24・`new(30_000,1_001)`→30・`new(60_000,1_001)`→60・`new(24_000,1_001)`→24）・`u32::MAX` 級境界（`new(4_294_967_295, 1)`→`u32::MAX`、`new(1, 4_294_967_295)`→0）
- [ ] `lib.rs` の `pub use time::{…, FrameRate}` 追加と crate-level doc の型列挙への追記

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-58-time-type-extension.md](../plans/implement-issue-58-time-type-extension.md) — `PLAN-034JNstCXHhzzwOvo9iiCv`（approved）。Phase T1 の正本
- [../designs/time-model-extension.md](../designs/time-model-extension.md) — `DESIGN-034JcePtzKFix4IuxyE5mD`（approved）。`FrameRate` API 形状・変換規約の正本
- [../adr/framerate-timebase-separation-and-timecode-extension.md](../adr/framerate-timebase-separation-and-timecode-extension.md) — Option A（newtype）・struct 形状決定の根拠

## Dependencies

- 前提: なし（plan 承認済み。`TimeBase` 既存実装の上に載る）
- 後続をブロック: `migrate-timecode-to-framerate.md`（`FrameRate` 型が前提）、`add-time-precision-tests.md`（`frame_duration()` が前提）
- 外部クレート依存・`unsafe` なし

## Affected Files

- `crates/ruviform-media-core/src/time.rs` — `FrameRate` 型と単体テストを追加（`mod tests` 内に `frame_rate_*` テスト群）
- `crates/ruviform-media-core/src/lib.rs` — `pub use` 追加・crate-level doc 更新

## Done When

- [ ] `FrameRate` が提案シグネチャどおり `time.rs` に存在し、`new()`/`from_fps()` がゼロ入力を `Err(InvalidValue("frame_rate"))` で拒否する
- [ ] `frame_duration()` が唯一の `TimeBase` 返却アクセサで、`to_time_base()`・`From` impl が存在しない
- [ ] `nominal_fps()` が `u64` 中間演算で実装され、`u32::MAX` 級境界テスト（`4294967295/1`→`u32::MAX`、`1/4294967295`→0）を含む全テストが合格する
- [ ] `lib.rs` から `FrameRate` が公開され、全公開 API に RT ラベルがある

## Verification

- [ ] `mise exec -- cargo test -p ruviform-media-core` — `FrameRate` テスト群を含め合格
- [ ] `mise exec -- cargo clippy -p ruviform-media-core --all-targets -- -D warnings` — 0 warnings
- [ ] `mise run fmt-check` — 差分なし
