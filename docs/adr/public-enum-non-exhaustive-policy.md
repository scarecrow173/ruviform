---
id: "ADR-034J6OnZouPI8HQy6cIg9D"
type: "adr"
status: "accepted"
title: "公開 enum への `#[non_exhaustive]` 統一方針"
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
    - "../specs/issue-18-followup-enum-and-time-fix.md"
    - "../plans/issue-18-followup-enum-and-time-fix.md"
    - "../../AGENTS.md"
  refines:
    - "../specs/implement-media-data-model-api.md"
    - "../specs/implement-media-io-abstraction-and-first-backend-poc.md"
  refined-by: []
  derives-from:
    - "../designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "../specs/issue-18-followup-enum-and-time-fix.md"
    - "../plans/issue-18-followup-enum-and-time-fix.md"
    - "../specs/implement-video-foundation-api.md"
    - "video-proc-kernel-api-and-color-hdr-scope.md"
    - "../plans/implement-issue-26-video-foundation.md"
  verifies: []
  verified-by: []
  references:
    - "https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute"
    - "https://doc.rust-lang.org/cargo/reference/semver.html#enum-new"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 公開 enum への `#[non_exhaustive]` 統一方針

## Context and Problem Statement

Issue #18（データモデル実装・PR #39）のレビューで、3 クレートの公開 enum の大部分が blanket `#![allow(clippy::exhaustive_enums)]` によって exhaustive のまま残っていることが指摘された（finding C5）。`PixelFormat` と `SampleFormat` のみが `#[non_exhaustive]` を明示付与済みであり、`Error`・`Rounding`・`StreamKind`・`TransportStatus`・`ChannelLayout`・色 enum 群は exhaustive のままである。Issue #18 の Judge は C5 を「`publish = false` の内部クレート方針として文書化済み・公開時に再評価」の non-blocking ノートとして扱ったが、レビュー後の再裁定として、メンテナが公開 enum に対する統一 API ポリシーを早期に確定することを決めた（Issue #41）。公開 enum を exhaustive のまま放置すると、将来の variant 追加が外部利用者の exhaustive `match` を破壊する breaking change となる。方針を「enum ごとの個別判断」に委ねると、新規 enum が既定で exhaustive に倒れ、ポリシーが静かに劣化する。ここで決めるのは「`#[non_exhaustive]` をどの enum に・どの粒度で適用するか」の統一ルールである。

## Decision Drivers

* semver 互換性: exhaustive enum への variant 追加は、downstream の exhaustive `match` を定義側の変更でコンパイルエラーにする breaking change（RFC 2008 / Cargo semver compatibility guide）。`#[non_exhaustive]` を付ければ variant 追加は外部利用者に対して non-breaking になる
* クレート境界の実態: `publish = false` であっても、同一 workspace の兄弟クレートは `path` 依存を通じた外部利用者であり、同じ semver 互換性の問題構造を持つ
* 既定方向の安全性: blanket `#![allow]` は、今後追加される全 enum を「個別に判断されないまま exhaustive」にする。ポリシーが明示的な例外管理ではなく既定値の惰性に支配される
* `unreachable!()` wildcard arm の反パターン排除: exhaustive enum に対して外部利用者が後付けで wildcard arm を `unreachable!()` にすると、variant 追加が未定義の panic を生む。未知 variant は graceful に扱うのが正しい
* 一貫性: `PixelFormat` / `SampleFormat` にのみ `#[non_exhaustive]` が付いている現状は半端なポリシーであり、「なぜこの enum だけか」の説明コストと読み違えを生む

## Considered Options

* **Option 1 — enum 単位の `#[non_exhaustive]` を全公開 enum に統一適用する**: クレート外から到達可能な全公開 enum に enum レベルの `#[non_exhaustive]` を付与する。`Rounding`・`ColorRange` のような閉集合と見なされる enum も対象に含め、同一 workspace の兄弟クレートも外部利用者として扱う。クレート内部限定の enum は対象外
* **Option 2 — enum ごとの個別判断を継続する**: 拡張見込みのある enum（`PixelFormat`・`SampleFormat`）にのみ `#[non_exhaustive]` を付け、`Error`・`Rounding` 等の閉集合は exhaustive のままとする。現状の延長
* **Option 3 — 全 enum を exhaustive のまま維持する**: Issue #18 で文書化された `publish = false` 内部クレート方針を継続し、公開時に再評価する

## Decision Outcome

**Option 1 を採用する。** メンテナが Issue #18 レビュー後の再裁定として決定した統一 API ポリシーであり、Issue #41 のスコープ 1 として確定済み（再裁定不要）。

具体的な規則:

* クレート外から到達可能な全公開 enum に、**enum レベル**の `#[non_exhaustive]` を付与する
* 閉集合と見なされる enum（`Rounding`・`ColorRange` 等）も対象に含める。「将来絶対に増えない」判断は将来の自分ではなく lint が監視する形にしない
* 同一 workspace の兄弟クレートは外部利用者として扱う（`path` 依存はクレート境界を越える）
* クレート内部限定の enum（`pub(crate)`・非公開）は対象外とし、`#[non_exhaustive]` を強制しない
* 既存の `#[non_exhaustive]` 付与（`PixelFormat`・`SampleFormat`）はそのまま維持する
* enum レベルと variant レベルの属性は区別する。本ポリシーが定めるのは enum レベルの適用であり、variant 個別の `#[non_exhaustive]`（その variant の構築を外部に閉じる別機構）は別途判断する

### Consequences

* 良い点: variant 追加が外部利用者に対して non-breaking になり、semver 面の将来の制約が消える。`#![allow(clippy::exhaustive_enums)]` を 3 クレートの `lib.rs` から撤去することで、以後新規公開 enum は clippy が exhaustive を自動検出し、ポリシーが lint で強制される（人のレビューに依存しない）。`PixelFormat`・`SampleFormat` のみ特別扱いという不整合が解消される
* 悪い点: 外部利用者（兄弟クレートを含む）はこれら enum に対して exhaustive `match` を書けず、`_` arm が必須になる。`_` arm の実装を誤ると未知 variant の処理が暗黙の握り潰しになるリスクは残るが、`unreachable!()` を置くよりは遥かに健全である。閉集合 enum にも wildcard を要求する分、match の見た目がわずかに冗長になる

### Confirmation

下記 Verification のチェックリストで確認する。enum 監査リスト（Issue #41 で列挙済みの 11 enum）に対する付与状況は clippy `exhaustive_enums` lint（allow 撤去後に `-D warnings` で実質 deny）が機械的に強制し、兄弟クレート（audio-core / video-core）が media-core の non_exhaustive enum に依存したままコンパイルできることがクレート間互換性の実証となる。

## Implementation Plan

* Affected paths:
  * `crates/ruviform-media-core/src/error.rs`（`Error`）、`time.rs`（`Rounding`）、`stream.rs`（`StreamKind`）、`transport.rs`（`TransportStatus`）
  * `crates/ruviform-audio-core/src/layout.rs`（`ChannelLayout`）、`format.rs`（`SampleFormat` — 付与済み・維持）
  * `crates/ruviform-video-core/src/color.rs`（`ColorPrimaries`・`TransferCharacteristics`・`MatrixCoefficients`・`ColorRange`）、`pixel.rs`（`PixelFormat` — 付与済み・維持）
  * `crates/ruviform-media-core/src/lib.rs`、`crates/ruviform-audio-core/src/lib.rs`、`crates/ruviform-video-core/src/lib.rs` — `#![allow(clippy::exhaustive_enums)]` 撤去と隣接ポリシーコメントの書き換え
  * `AGENTS.md` — 「暫定規約」節に本ポリシー 1 行を追記
* Patterns to follow: `#[non_exhaustive]` は `pub enum` の直上に属性として付与する（`PixelFormat`: `crates/ruviform-video-core/src/pixel.rs:32`、`SampleFormat`: `crates/ruviform-audio-core/src/format.rs:23` と同じ形）。外部側の `match` には `_` arm を置き、未知 variant は無視・フォールバック等の graceful な処理にする
* Patterns to avoid: variant レベルの `#[non_exhaustive]`（enum 構築を塞ぐ別機構。本ポリシーの対象外）、外部側 wildcard arm への `unreachable!()`（variant 追加時に panic を生む反パターン）、「この enum は閉集合だから exhaustive のまま」の個別例外、allow の部分温存
* Dependencies or configuration: 依存追加なし。`#![allow(clippy::exhaustive_enums)]` 撤去により、workspace lints + `lint` / `lint-ci` の `-D warnings` ゲートがそのまま強制機構になる

## Verification

* [x] 監査リストの全 11 enum（`Error`・`Rounding`・`StreamKind`・`TransportStatus`・`ChannelLayout`・`ColorPrimaries`・`TransferCharacteristics`・`MatrixCoefficients`・`ColorRange`・`PixelFormat`・`SampleFormat`）に enum レベルの `#[non_exhaustive]` が付いている（`PixelFormat`・`SampleFormat` は既存を維持）
* [x] 3 クレートの `lib.rs` から `#![allow(clippy::exhaustive_enums)]` が撤去され、隣接コメントが本ポリシー（本 ADR）を指す内容に更新されている
* [x] クレート内部限定の enum に `#[non_exhaustive]` が強制されていない
* [x] `mise run lint`（`cargo clippy --workspace --all-targets -- -D warnings`）がグリーン — allow 撤去後は lint が enum の exhaustive 化を検出する
* [x] `mise run check` / `mise run test` がグリーン — audio-core / video-core が media-core の `#[non_exhaustive]` enum に依存したままビルド・テスト通過する（クレート間で exhaustive `match` が存在しないことの実証）
* [x] `AGENTS.md` の暫定規約に本ポリシー（本 ADR へのリンクを含む）が記録されている

## Pros and Cons of the Options

### Option 1 — 全公開 enum に enum 単位で統一適用（採用）

* Good, because variant 追加が non-breaking になり、lint が既定で強制するためポリシーの劣化余地がない。ルールが一律で説明コストが最小。
* Bad, because 閉集合 enum にも `_` arm を要求するため、外部側の `match` がわずかに冗長になる。

### Option 2 — enum ごとの個別判断

* Good, because 閉集合 enum の外部側で exhaustive `match` が書け、match の網羅性チェックをコンパイラに委ねられる。
* Bad, because 「拡張見込みがあるか」の判断が各 enum の追加時判断に委ねられ、blanket allow の既定値によって新規 enum が気づかず exhaustive 化する。`Error` のような「今は閉じて見える」enum の variant 追加が breaking になるリスクを温存する。

### Option 3 — 全 enum exhaustive 維持（現状）

* Good, because 変更コストがゼロ。内部クレートでは exhaustive `match` の網羅性チェックが常に効く。
* Bad, because `publish = false` の間は workspace 内という限定空間でも breaking 影響が実在し（兄弟クレートの match が壊れる）、将来クレートを公開する場合に exhaustive enum の拡張ができない。C5 が指摘した「一括 allow で個別判断していない」問題が残る。

## More Information

* 一次ソース: GitHub Issue [#41 [impl] Issue #18 フォローアップ: 公開 enum non_exhaustive 方針の適用と時間演算修正](https://github.com/scarecrow173/ruviform/issues/41)（スコープ 1「公開 enum `#[non_exhaustive]` 統一方針」は決定済み・再裁定不要と明記）
* 経緯: Issue #18 / PR #39 レビュー finding C5（`tmp/issue-18/review-b.md`・`tmp/issue-18/judgment.md`）。当時は `publish = false` 内部クレート方針として文書化済み判断と裁定されたが、本 ADR で統一ポリシーとして上書き確定した
* 上流ドキュメント: [設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（`relations.derives-from`。データモデルの公開 API 方針の正本）
* 派生ドキュメント（`relations.derived-by`）: [spec](../specs/issue-18-followup-enum-and-time-fix.md)・[plan](../plans/issue-18-followup-enum-and-time-fix.md) — 本ポリシーの適用を含むフォローアップ実装
* 外部参照（`relations.references`）: [Rust Reference — the `non_exhaustive` attribute](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute)、[Cargo SemVer compatibility — enum variants](https://doc.rust-lang.org/cargo/reference/semver.html#enum-new)
* 関連（`relations.related`）: [`AGENTS.md`](../../AGENTS.md)（暫定規約に本ポリシーが転記される）
