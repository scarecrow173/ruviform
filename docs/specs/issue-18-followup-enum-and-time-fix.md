---
id: "SPEC-034J6OCXHiP1izKRzbMmgz"
type: "spec"
status: "implemented"
title: "Issue #18 フォローアップ（公開 enum 方針適用・時間演算修正・改善トリアージ）"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/41"
  changes:
    added: []
    modified:
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/ruviform-media-core/src/error.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/time.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/transport.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/buffer.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/layout.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/color.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../plans/issue-18-followup-enum-and-time-fix.md"
  depends-on:
    - "implement-media-data-model-api.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
  derived-by:
    - "../plans/issue-18-followup-enum-and-time-fix.md"
  verifies: []
  verified-by:
    - "../test-specs/followup-enum-compat-and-time-arithmetic.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/18"
    - "https://github.com/scarecrow173/ruviform/pull/39"
  defers: []
  deferred-by: []
---

# Issue #18 フォローアップ（公開 enum 方針適用・時間演算修正・改善トリアージ）

## What Should Be Built

GitHub Issue #41 で定義された Issue #18 フォローアップの成果物として、3 系統の変更を data-model 3 クレートに適用する。

- **公開 enum `#[non_exhaustive]` 統一方針の適用**（[enum ADR](../adr/public-enum-non-exhaustive-policy.md) の実装）: クレート外から到達可能な全公開 enum に enum レベルの `#[non_exhaustive]` を付与する。対象監査リストは `Error`・`Rounding`・`StreamKind`・`TransportStatus`・`ChannelLayout`・`ColorPrimaries`・`TransferCharacteristics`・`MatrixCoefficients`・`ColorRange`・`PixelFormat`・`SampleFormat` の 11 enum（うち `PixelFormat`・`SampleFormat` は付与済みを維持）。あわせて 3 クレートの `lib.rs` から `#![allow(clippy::exhaustive_enums)]` を撤去し lint に強制させ、`AGENTS.md` 暫定規約にポリシー 1 行を追記する。
- **時間演算の修正**: `MediaRange::end()`（および同型の `MediaTime::checked_add` / `checked_sub`）が、duration の自 base → start base 変換が `i64` に収まらない時点で早期飽和 / `Err` を返し、「最終的な和が `i64` で表現可能」でも誤値・過剰 `Err` になるバグを修正する。delta を `i128` の正確な有理数として計算し、丸めは最後に 1 回だけ適用し、`i64` 範囲検査は最終値（`start + delta`）にのみ行う形に改める。あわせて `MediaRange::checked_end(rounding) -> Result<MediaTime>` を新設し、負 duration・丸めモードをカバーするテストと `# Panics` / `# Errors` ドキュメント・RT ラベルを更新する。
- **小改善トリアージ**: `AudioBuffer::try_new() -> Result<Self>` の追加（`from_interleaved` と検証を共有。`new()` は従来どおり異常入力で panic する契約を維持し doc 相互参照）、`ChannelLayout::Custom` を空 mask で構築できない構造的不変条件の強制（非空 mask newtype による payload 変更。`SpeakerMask` 自体は不変）、時間変換における呼び出しごとの丸め（単一丸め/回）と誤差累積への案内を doc 注記する。
- **延期項目の Issue 記録**: 本 spec の Out of Scope 項目を、Issue #41 が規定する既存 Issue 対応表どおりに記録・起票する。

## Why Now

- Issue #18 / PR #39 の追加レビューに基づくフォローアップ（Issue #41）。レビューで enum 方針（C5）・時間演算バグ・小改善候補が洗い出され、enum 方針はメンテナ再裁定で「統一ポリシー適用」が確定済み。データモデル API が上位層の基盤となる前の修正コストが最も低い時期である。
- 時間演算バグは正確性の欠陥であり、有理数時間を「正確」と謳う中核契約（f64 を介さない整数演算）を裏切る。極端な base（`~2^32/1` 等）や `i64::MIN` 起点の範囲で誤値・過剰 `Err` が実在し、編集計算や範囲クエリに静かに混入し得る。
- exhaustive enum のまま利用クレートが増えると、あとから `#[non_exhaustive]` を付ける変更自体が「match を壊す」修正になる。利用者がまだ少ない今の適用が最も安い。
- `ChannelLayout::Custom(SpeakerMask)` を空 mask で直接構築するとゼロチャンネルの縮退レイアウトが作れてしまう現状は、不変条件が doc 文言（「contract violation」と注記）にのみ依存する構造的欠陥であり、型で閉じるのは小さい修正のうちに限る。

## Users and Value

- **兄弟クレート・将来の外部利用者**: 全公開 enum で variant 追加が non-breaking になり、downstream の `match` が定義側の拡張で壊れない。`_` arm による未知 variant の graceful 処理が前提化する。
- **engines / media-io の実装者**: `MediaRange::end()`・`checked_add`/`checked_sub` が「最終値が表現可能か」だけで判定する一貫した契約になり、極端な base・負 duration を含む時間計算を信用できる。`MediaRange::checked_end` で失敗を `Result` として扱える。
- **API 利用者**: `AudioBuffer::try_new()` で panic を避けたい構築経路（フォーマット交渉等の回復可能な失敗）が `Result` で書ける。`ChannelLayout::Custom` のゼロチャンネル構築が型で塞がれる。
- **レビュアー・後続 Issue の作業者**: enum 方針が lint で強制されレビュー負荷が下がる。延期項目が Issue に対応付けられ、二重起票や取りこぼしを防げる。

## Scope

### In Scope

- `ruviform-media-core` / `ruviform-audio-core` / `ruviform-video-core` の公開 enum 11 件への enum レベル `#[non_exhaustive]` 付与（9 件新規 + 2 件維持確認）と、3 `lib.rs` の `#![allow(clippy::exhaustive_enums)]` 撤去・隣接ポリシーコメント更新
- `AGENTS.md` 暫定規約への enum ポリシー 1 行追記（enum ADR への参照付き）
- `crates/ruviform-media-core/src/time.rs` の `MediaRange::end()` 修正、`MediaRange::checked_end(rounding)` 新設、`MediaTime::checked_add`/`checked_sub` の同型修正（`i128` 中間演算・丸めは最終 1 回・最終値のみ `i64` 範囲検査）、回帰テストと `# Panics`/`# Errors` ドキュメント・RT ラベル更新。`convert` の契約は変更しない
- `crates/ruviform-audio-core/src/buffer.rs` の `AudioBuffer::try_new(spec) -> Result<Self>`（`from_interleaved` と検証共有）と `new()` の panic 契約 doc 相互参照
- `crates/ruviform-audio-core/src/layout.rs` の `ChannelLayout::Custom` 構造的不変条件（非空 mask newtype を payload にする方式）と、付随するコンストラクタ・`channels()`・`speaker_mask()`・テスト・doctest の更新
- `MediaTime::convert` / `checked_add` の「呼び出しごとに 1 回の丸め」「境界で変換して誤差累積を避ける」doc 注記（doc-only）
- 延期項目の Issue 記録（Issue #41 の既存 Issue 対応表どおり）

### Out of Scope

- 借用 planar ビュー（CLAP/VST3 ゼロコピー）の新規抽象 — Issue #29 / #23 / #25 のスコープ
- RT テスト選択方式の改善・alloc/dealloc 検証 — Issue #22 のスコープ
- `VideoFrame` 所有権・フレームプール・zero-copy 方式 — Issue #26 のスコープ
- `FrameRate` 型・`TimeBase` 意味分離と `Timecode` 拡張、丸め誤差累積のポリシー化 — 新規 Issue（起票対象）
- `MediaTime::convert` / `MediaDuration::convert` の変換契約自体の変更（丸めモード・`Err` 条件は現行維持）
- variant レベルの `#[non_exhaustive]`、struct の `#[non_exhaustive]` 検討（本 spec の enum ポリシーは enum レベルのみ）
- クレート内部限定 enum への `#[non_exhaustive]` 強制

## Requirements

- Rust edition 2024・MSRV 1.85・stable のみ。`unsafe` 不使用・外部依存追加なし（現行方針維持）
- `#[non_exhaustive]` は enum レベルで付与する。variant レベル属性は本ポリシー対象外。閉集合 enum も含め一律とし、クレート内部限定 enum は対象外（enum ADR の規則どおり）
- 時間演算の修正は「delta を `i128` の正確な有理数（`value * num * other_den / (den * other_num)`）として保持し、丸めは最後に 1 回だけ適用、`i64` 範囲検査は最終値のみ」に統一する。`MediaRange::end()` はトータル関数を維持し飽和は「真の最終オーバーフロー時のみ」、`checked_end` / `checked_add` / `checked_sub` は最終値オーバーフローのみ `Err(Error::TimeOverflow)` とする
- `MediaTime::convert` の契約（表現可能なら厳密・表現不可能なら指定丸め・変換値自体が `i64` を超えるなら `Err`）は変更しない
- `AudioBuffer::new` の `-> Self` + panic 契約は維持し、`try_new` を `Result` を返す並行経路として追加する（`from_interleaved` と形状検証を共有）
- `SpeakerMask` 自体は変更しない（空 mask は値として引き続き合法）。空 mask を `Custom` に入れられないのは `ChannelLayout` 側の構造的不変条件として実現する
- 変更対象 API の `**RT contract:**` ラベル・`# Panics`/`# Errors` ドキュメントを実態に合わせて更新する

## Acceptance Criteria

- [x] **AC1**: 監査リストの全 11 enum が enum レベルの `#[non_exhaustive]` を持つ（`Error`・`Rounding`・`StreamKind`・`TransportStatus`・`ChannelLayout`・`ColorPrimaries`・`TransferCharacteristics`・`MatrixCoefficients`・`ColorRange`・`PixelFormat`・`SampleFormat`）。クレート内部限定 enum に強制は及んでいない。3 `lib.rs` の `#![allow(clippy::exhaustive_enums)]` が撤去され、`mise run lint` がポリシーを強制する
- [x] **AC2**: `MediaRange::end()` が「duration の中間変換が `i64` を超えるが最終和は表現可能」な入力で正しい値を返す（回帰テストあり）。`MediaTime::checked_add`/`checked_sub` が同型の早期 `Err` を返さず最終値のみで判定する。`MediaRange::checked_end(rounding) -> Result<MediaTime>` が存在する。負 duration と全 `Rounding` モードの境界がテストで覆われる
- [x] **AC3**: `AudioBuffer::try_new()` が `Result` を返す構築経路として存在する。`ChannelLayout::Custom` が空 mask で構築不能（構造的不変条件）。時間変換の丸め誤差累積に関する doc 注記がある
- [x] **AC4**: 延期項目が Issue #41 の既存 Issue 対応表（借用 planar ビュー → #29/#23/#25、RT テスト選択・alloc 検証 → #22、`VideoFrame` 所有権・pool → #26、`FrameRate`/`TimeBase` 分離・`Timecode` 拡張・丸め方針 → 新規 Issue）どおりに記録されている

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| 借用 planar ビュー（ゼロコピー） | 新規抽象は作らない（`AudioBufferView` 現行のまま） | Issue #29 / #23 / #25 | CLAP/VST3 境界でコピーが残り性能契約が成立しない |
| RT テスト選択方式・alloc/dealloc 検証 | `rt_` substring フィルタの既知精度限界は据え置き | Issue #22 | RT レーンの汚染・RT 契約違反の見逃し |
| `VideoFrame` 所有権・プール・zero-copy | `Arc` backing 現行維持 | Issue #26 | フレーム確保が RT パスに混入し得る責務境界の曖昧さ |
| `FrameRate`/`TimeBase` 意味分離・`Timecode` 拡張・丸め誤差累積ポリシー | 本 spec では doc 注記のみ（ポリシー化しない） | 新規 Issue（起票対象） | フレームレートと timebase の混同・累積丸め誤差の取り扱いが未定義のまま残る |

## Sources and Relations

- 一次ソース: GitHub Issue [#41 [impl] Issue #18 フォローアップ: 公開 enum non_exhaustive 方針の適用と時間演算修正](https://github.com/scarecrow173/ruviform/issues/41)（`relations.source`）。Issue #18 / PR #39 の追加レビューに基づくフォローアップとしてスコープ・既存 Issue 対応表を規定。
- 上流文書（`relations.derives-from`）: [設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（時間モデル・データモデルの決定事項）、[RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)（API 単位の RT 保証ラベル規則）、[公開 enum ADR](../adr/public-enum-non-exhaustive-policy.md)（本 spec が適用する統一ポリシーの正本）。
- 前提（`relations.depends-on`）: [implement-media-data-model-api.md](implement-media-data-model-api.md)（`SPEC-034J5FYaYfeL5zkzb4sIV4`・implemented。本 spec が修正対象とする API 表面を確定した先行 spec）。
- 派生（`relations.derived-by`）: [実装 plan](../plans/issue-18-followup-enum-and-time-fix.md)。検証意図は [test spec](../test-specs/followup-enum-compat-and-time-arithmetic.md)（`relations.verified-by` 相互リンク）が保持する。
- 関連（`relations.related`）: [`AGENTS.md`](../../AGENTS.md)（暫定規約 — enum ポリシーの転記先）。
- 参考（`relations.references`）: [Issue #18](https://github.com/scarecrow173/ruviform/issues/18)（データモデル実装の親 Issue）、[PR #39](https://github.com/scarecrow173/ruviform/pull/39)（マージ済み実装）。
