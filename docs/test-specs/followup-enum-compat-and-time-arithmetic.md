---
id: "TSPEC-034J6OIFK83GJIsST60Jck"
type: "test-spec"
status: "approved"
title: "公開 enum 拡張性と時間演算精度の契約保証"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/41"
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
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-architecture-design-baseline.md"
  derived-by: []
  verifies:
    - "../specs/issue-18-followup-enum-and-time-fix.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 公開 enum 拡張性と時間演算精度の契約保証

## Purpose

Issue #41 フォローアップが導入する振る舞い契約——公開 enum の `#[non_exhaustive]` による前方互換性、`i128` 中間精度に基づく時間演算、`ChannelLayout::Custom` の非空不変条件、`AudioBuffer::try_new` の失敗表出——の「なぜテストが存在するか」を保持する。[spec](../specs/issue-18-followup-enum-and-time-fix.md) の受け入れ基準 AC1–AC3 を、実装の改変や後続クレート追加の際に「どの振る舞いを守るべきか」「そのテストはまだ必要か」を判断できる形で固定する。廃止してよいのは、対象契約自体を変更する ADR / spec 改訂が行われたとき（例: enum 方針の見直し、時間演算の丸め規則の変更、`ChannelLayout` モデルの刷新）。

## Feature

公開 enum の拡張性契約と時間演算の精度契約: data-model 3 クレートの全公開 enum が enum レベル `#[non_exhaustive]` で外部 match に wildcard arm を要求し、`MediaRange::end()` / `checked_end` / `MediaTime::checked_add` / `checked_sub` が `i128` 中間演算で「最終値のみ `i64` 範囲検査」を満たし、`ChannelLayout::Custom` が構造的に空 mask を持たず、`AudioBuffer::try_new` が検証失敗を `Err` で表出すること。

## Rules

- **全公開 enum は enum レベル `#[non_exhaustive]` を持つ**。downstream（クレート外）の `match` は `_` arm を必須とし、variant 追加は外部利用者に対して non-breaking。同じクレート内の exhaustive `match` は引き続き合法であり、wildcard arm に `unreachable!()` を置かない（variant 追加時に panic 化する反パターン）。閉集合 enum（`Rounding`・`ColorRange` 等）にも一律適用し、クレート内部限定 enum には強制しない。
- **時間演算の `i64` 範囲検査は最終値にのみ適用する**。`MediaRange::end()` / `checked_end` / `MediaTime::checked_add` / `checked_sub` は、delta を `i128` の正確な有理数（`value * num * other_den / (den * other_num)`）として保持し、丸めは最後に 1 回だけ適用する。「duration の自 base → start base 変換値が `i64` を超えるが `start + delta` は表現可能」な入力で、誤値・過剰 `Err` を返さない。
- **`MediaRange::end()` はトータル関数を維持する**。飽和は「真の最終オーバーフロー（`start + delta` が `i64` を超える）時のみ」とし、変換途中の `i64` 超過では飽和しない。`checked_end(rounding)`・`checked_add`・`checked_sub` は最終値オーバーフローのみ `Err(Error::TimeOverflow)` を返す。
- **負の duration は符号を保った対称な振る舞いとする**。負 duration を持つ range の `end` は `start` より前に来ることがあり、丸めは各 `Rounding` モードで負値にも対称に作用する（既存 `-3/2` 丸め例と同じ規則）。
- **`ChannelLayout::Custom` は構造的に空 mask を持たない**。ゼロチャンネルの縮退レイアウトを直接構築する経路は存在せず、非空 mask の検証は構築時の `Err` で表出する。`SpeakerMask` 自体の空値は引き続き合法（不変条件を課すのは `Custom` payload 側）。
- **`AudioBuffer::try_new` は検証失敗を `Err` で返し panic しない**。形状検証は `from_interleaved` と共有され、`new()` は従来どおり異常入力で panic する契約を維持する（panic 条件は `# Panics` に記載済み）。

## Examples

- Given: `start = MediaTime::new(i64::MIN, TimeBase::new(1, 1))`、`duration = MediaDuration::new(2^63 相当の tick 数, 大きな分子を持つ base)`（duration の start base 変換値が `i64` を超える）。When: `range.end()` / `range.checked_end(Rounding::NearestEven)` を呼ぶ。Then: `start + delta` が `i64` に収まる場合、end は真値（0 または小さい値）を返し、`Err` / 飽和にならない。
- Given: `start + delta` が `i64::MAX` を真に超える入力。When: `range.end()` を呼ぶ。Then: `i64::MAX` に飽和する（`Err` ではない — トータル契約）。`checked_end` なら `Err(Error::TimeOverflow)`。
- Given: 負 duration の range。When: `end()` / `checked_end` を呼ぶ。Then: end が start より前の時刻として正しく計算される。`checked_sub` で `i64::MIN` の duration を扱う際も中間否定が `i128` で行われ最終値のみで判定される。
- Given: 非整数変換（`delta` が start base で表現不可能）。When: 各 `Rounding` モードで `checked_add` / `checked_end` を呼ぶ。Then: 丸めは最終 delta に 1 回だけ適用され、Floor / Ceil / TowardZero / AwayFromZero / NearestEven の各規則どおりになる（`convert` の丸め規則と一貫）。
- Given: 空の `SpeakerMask`。When: `ChannelLayout` の `Custom` 構築経路に渡す。Then: 構築は `Err` で失敗し、ゼロチャンネルの `ChannelLayout` 値は作れない（`ChannelLayout::custom` が引き続き `Err(Error::InvalidValue("mask"))` を返す契約は維持）。
- Given: `channels * frames` が `usize` / `isize::MAX` バイト上限を超える `BufferSpec`。When: `AudioBuffer::try_new(spec)` を呼ぶ。Then: `Err(Error::InvalidValue(..))` が返り panic しない。同じ spec で `new()` を呼べば従来どおり panic する（契約の非対称は意図的）。
- Given: audio-core / video-core のコード・テストが media-core の `#[non_exhaustive]` enum（`Error`・`Rounding` 等）を利用している。When: `mise run check` / `mise run test` を実行する。Then: クレート間で exhaustive `match` なしにビルド・テストが通る（sibling crate が実在する downstream 利用者として互換性を実証する）。

## Guarantees

- variant 追加が外部利用者（兄弟クレート含む）の `match` を壊さず、将来の enum 拡張が non-breaking であることが lint とコンパイルで保たれる。
- 「duration の変換値が `i64` を超える」だけで最終和が表現可能なケースで、誤った飽和値・過剰 `Err` が出ないことが回帰テストで保たれる。
- `end` / `checked_end` / `checked_add` / `checked_sub` の失敗・飽和条件が「最終値のみ」で一貫し、`i128` 中間精度により中間値の表現力不足を利用者が意識しなくてよい。
- `ChannelLayout::Custom` のゼロチャンネル値が型構造上存在せず、doc 上の注記ではなく構築不能性として保たれる。
- `AudioBuffer` 構築で panic を避けたい呼び出し側が `try_new` の `Err` で回復可能な失敗を扱える。

## Non-goals

- 変換 API 自体の契約変更の検証（`MediaTime::convert` / `MediaDuration::convert` の丸め規則・`Err` 条件は現行維持。`media-data-model-api.md` test-spec の保証対象）。
- cross-base `Eq`/`Ord`/`Hash`、ドロップフレーム、planar/interleaved 往復、plane/stride 契約、`plane_mut` の `Arc` ゲート（`media-data-model-api.md` test-spec の既存保証に委譲）。
- 借用 planar ビュー・`FrameRate` 型・フレームプール・alloc 検出 CI（Issue #29/#23/#25・#22・#26・新規 Issue の検証対象）。
- variant レベル `#[non_exhaustive]` や `#[non_exhaustive]` struct の互換性（本ポリシー対象外の別機構）。
- スクラッチ downstream クレートを新設しての match 検証 — 兄弟クレート（audio-core / video-core）が実在する downstream 利用者であり、そのコンパイル成功が十分な実証である。
- パフォーマンス・ベンチマーク（本変更は整数演算の精度修正であり計量対象外）。

## Risk

- enum への `#[non_exhaustive]` 付与が漏れると、blanket allow の惰性で新規 enum が exhaustive のまま増え、将来の variant 追加が breaking change 化する。lint 強制の撤去漏れは同じ劣化を招く。
- 時間演算が早期 `i64` 変換に戻ると、極端 base・`i64::MIN` 起点の範囲計算が静かに誤値を返し、編集・同期・範囲クエリの精度欠陥が発見困難な形で蓄積する。
- 飽和と `Err` の条件が「最終値のみ」からずれると、`end()` のトータル契約と `checked_*` の失敗契約が相互に不整合になり、呼び出し側が誤った失敗経路を前提にコードを書く。
- `Custom` の不変条件が doc 注記に退行すると、ゼロチャンネルレイアウトを持つ `BufferSpec` が下流（バッファ確保・デバイス交渉）まで到達し、意味のない 0 チャンネル処理や検証の重複を生む。
- `try_new` が無い・検証が `from_interleaved` と分岐すると、panic を避けたい経路が手書き検証を複製し、2 つの検証ルールが乖離する。
