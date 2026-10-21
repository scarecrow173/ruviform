---
id: "ADR-034JceOKDwTeWus9xBVXND"
type: "adr"
status: "accepted"
title: "FrameRate/TimeBase 意味分離と Timecode 拡張"
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
  depends-on:
    - "public-enum-non-exhaustive-policy.md"
    - "realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/time-type-extension-framerate-timecode-rounding.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../specs/time-type-extension-framerate-timecode-rounding.md"
  derived-by:
    - "../designs/time-model-extension.md"
  verifies: []
  verified-by: []
  references:
    - "https://crates.io/crates/vtc"
    - "https://github.com/scarecrow173/ruviform/issues/41"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# FrameRate/TimeBase 意味分離と Timecode 拡張

## Context and Problem Statement

`ruviform-media-core` の現行 `Timecode::from_media_time(t, frame_rate: TimeBase)` / `from_media_time_drop(t, frame_rate: TimeBase)` は、`frame_rate` 引数に `TimeBase` 型を取る。しかし `TimeBase` は「1 tick の有理数長（`numerator/denominator` 秒）」を表す型であり、呼び出し側は fps 向きの有理数（`TimeBase::new(30_000, 1_001)` = 29.97 fps 相当）を渡す設計になっている。つまり同じ `TimeBase` 型が「1 秒の有理数分割（= 1 tick の長）」と「フレーム/秒（fps）」という**逆数関係にある 2 つの意味**を担っており、呼び出し側が内部実装（`floor(t * frame_rate)` で fps として使われる）を知らなければ正しい値を渡せない。

audio-core には `pub struct SampleRate(TimeBase)`（`crates/ruviform-audio-core/src/time.rs`、`new(rate)` が `TimeBase::new(1, rate)` を構築する Hz 意味の newtype）という先例があり、音声側は意味分離済みである。動画側には対応する型がなく、video-core にも frame rate 型は未存在（`VideoFrame.pts: MediaTime` のみ）。Issue #27（動画エンジン）がこの API の消費者となる前に、`FrameRate` と `TimeBase` の責務分離・`Timecode` API の引数型・互換性方針・ドロップフレーム拡張性の分類を決定する必要がある（Issue #42 受入条件 1）。

## Decision Drivers

* **`SampleRate(TimeBase)` 先例との一貫性**: 「TimeBase を wrap して Hz 意味を与える newtype」という既存パターンが audio-core にあり、同じ構成を動画側に適用すれば学習コスト・レビューコストが最小
* **意味論的分離の型による固定**: fps（フレーム/秒）と timebase（1 tick の秒数）は逆数関係にあり、`TimeBase::new(30_000, 1_001)` を「29.97 fps」と「約 30 秒/tick」のどちらにも解釈できる現状は誤用の構造的温床。型で分離すれば誤用がコンパイルエラーになる
* **media-core 依存ゼロ方針**: media-core は外部依存ゼロ（std のみ）・外部型を公開 API に露出しない契約（設計ベースライン・media-io ADR 系の方針）を持ち、`vtc` 等の外部型採用はこれと衝突する
* **既存変換インフラの再利用**: `i128`/`u128` 中間精度・`scaled_fraction`/`round_quotient`/`convert_value_wide` ヘルパを持つ既存の整数演算基盤をそのまま使える形が望ましい
* **`#[non_exhaustive]` ポリシー整合**: 拡張設計は enum ADR（`public-enum-non-exhaustive-policy.md`）に整合し、`Error` の将来 variant 追加余地を残す必要がある
* **互換性コストの現時点の最小性**: ワークスペースは `publish = false`・外部利用者不在。`Timecode` API を破壊的に置き換えられるのは現時点に限られる
* **API 単位の RT 契約**: 新 API には `**RT contract:**` ラベルが必須（realtime ADR）

## Considered Options

* **Option A — `FrameRate` を `TimeBase` の newtype struct として導入する**: `pub struct FrameRate(TimeBase)` で fps 向き有理数（例: `30000/1001`）を private フィールド + 検証付き `new() -> Result<Self>` + `frame_duration()`（逆数 = 真の tick 長）等のアクセサで包む。保持する fps 有理数は `TimeBase` 契約（1 tick の長）を満たさないため `TimeBase` 値としては露出しない。`SampleRate(TimeBase)` 先例に整合
* **Option B — `FrameRate` を独立 struct として導入する**: `u32` numerator/denominator を自前保持し、`TimeBase` 内部表現に非依存の別型とする
* **Option C — 型を増やさず `TimeBase` のまま API 名・doc で区別する**: 現状維持。`from_media_time` の引数名とドキュメントで「fps 向きに渡せ」と示すだけ
* **Option D — `vtc` クレート等の外部型を採用する**: SMPTE タイムコードの成熟クレート（discovery で唯一の成熟例と記録）を依存に追加し、その frame rate 型を使う

## Decision Outcome

**Option A を採用する。** `FrameRate` は `TimeBase` の newtype struct（fps 向き有理数を保持）とし、private フィールド + 検証付き `new()` で不変条件を構築時に強制する。`SampleRate(TimeBase)` の先例と同じ構成であり、意味論的分離を型で固定しつつ、既存の `i128`/`u128` 中間精度の変換インフラをそのまま再利用でき、依存ゼロ方針も維持できる。保持は fps 向きであり、`TimeBase` を返す公開 API は `frame_duration()`（真の tick 長 = 逆数）のみとする — 保持する fps 有理数を `TimeBase` 値として取り出す API（`to_time_base` / `From<FrameRate> for TimeBase`）は設けず、`MediaTime::new(v, fr.into())` 型の混同を型で防ぐ分離目的を保全する。詳細な API 形状（`new` / `from_fps` / `frame_duration` / `nominal_fps` 等の提案シグネチャ）は派生設計文書 [time-model-extension.md](../designs/time-model-extension.md) が正本とする。

`Timecode::from_media_time` / `from_media_time_drop` の `frame_rate` 引数は `FrameRate` を受け取る形に変更し、`floor(t * frame_rate)` の内部計算は現行の厳密整数演算を維持する。

### 副次決定 1: 互換性方針（破壊的変更）

`Timecode::from_media_time(_drop)` の `frame_rate` 引数は **`TimeBase` から `FrameRate` への破壊的置換**とし、旧シグネチャの併設は行わない。根拠:

* ワークスペースは `publish = false` で外部利用者が存在せず、呼び出し側は同一ワークスペース内に限定される（現時点で `Timecode` の呼び出し側は media-core 内部のテスト・doctest のみ）
* 「旧 `TimeBase` 版と新 `FrameRate` 版の新 API 併設」は 2 系統の API を恒久的に維持する重複になり、KISS に反する。破壊的変更が許容できるのは外部利用者不在の現時点に限られる
* 移行（シグネチャ置換・呼び出し側・doctest 更新）は実装 Issue で実施する

### 副次決定 2: ドロップフレーム拡張性の分類

* SMPTE 規格上、ドロップフレームのラベリング規則は 29.97/59.94 系（nominal 30/60）にのみ定義される。現行の対応集合（`30000/1001`・`60000/1001` のみ、他は `Err(UnsupportedDropFrameRate)`）を**確定（現行維持）**とする
* 非 NTSC nominal への一般化（例: 23.976 系への独自 drop 規則）・フィールドレート対応は**保留**とする。再着手トリガ: インターレース素材対応・非 NTSC drop を要求する実需要の発生（編集/NLE 系機能またはメディア I/O 実装で要求された時点）
* 拡張時は `Error` enum が `#[non_exhaustive]`（enum ADR の統一ポリシー適用済み）であるため、新しいエラー variant（例: フィールドレート用の別エラー種）や `Rounding` 相当の新ポリシー追加余地は互換性を壊さず残る

### 副次決定 3: `FrameRate` の struct 形状

* private フィールド + 検証付き `new() -> Result<Self>`（`TimeBase` 型式。`numerator`/`denominator` が 0 の場合は失敗）で、「正の有理数 fps」不変条件をコンストラクタで強制する
* `#[non_exhaustive]` struct ポリシーの対象は公開 enum であり、struct である `FrameRate` は対象外。不変条件は ctor で強制する（enum ADR の適用範囲と整合）

### Consequences

* 良い点: `Timecode` API の引数が意味的に正しい型になり、`TimeBase`（1 tick の有理数長）と `FrameRate`（fps）の逆数混同が型で防げる。`SampleRate` 先例に倣う設計語彙が揃う（`SampleRate` = Hz 意味で真の tick 長 `1/rate` を保持、`FrameRate` = fps 意味で fps 有理数を保持。どちらも `TimeBase` newtype だが保持方向は異なる）。依存ゼロ・`unsafe` 不使用・i128 整数演算の既存方針をすべて維持できる
* 悪い点: 既存の `from_media_time(t, TimeBase)` 呼び出しはコンパイルエラーになる破壊的変更（影響はワークスペース内に限定）。`FrameRate` → `TimeBase` の変換規約（`frame_duration()`）を新たに定義・保守するコストが生じる。`TimeBase` 側に `Ord` が意図的に未実装であるのと同様、`FrameRate` の比較契約も個別に決める必要がある

### Confirmation

下記 Verification のチェックリストで確認する。採用方式（Option A）と互換性方針（破壊的置換）・拡張性分類（現行集合確定 + 非 NTSC 保留）は、派生設計文書 [time-model-extension.md](../designs/time-model-extension.md) の API 設計・分類表と、実装 Issue の `mise run check` / `lint` / `test` 通過で検証する。設計ベースラインの分類表にも同分類が記録される。

## Implementation Plan

* Affected paths:
  * `crates/ruviform-media-core/src/` — `FrameRate` 型の追加位置（新規モジュールまたは `time.rs` 内。設計文書の提案に従う）、`timecode.rs` の `from_media_time` / `from_media_time_drop` シグネチャ置換、`lib.rs` の `pub use` 更新
  * `crates/ruviform-media-core/src/error.rs` — 必要な場合のみ新 variant（`#[non_exhaustive]` で non-breaking 追加可。現行 `InvalidValue`/`UnsupportedDropFrameRate` の再利用を優先）
  * `crates/ruviform-media-core/src/time.rs` — `FrameRate` → `TimeBase` 変換規約（`frame_duration()`）の実装時配置
  * doctest・単体テスト（`timecode.rs` の既存テストは `FrameRate` 構築に書き換え）
* Patterns to follow: `SampleRate` の newtype 構成（`crates/ruviform-audio-core/src/time.rs` の `pub struct SampleRate(TimeBase)`、`new() -> Result<Self>`、`get()`/`time_base()` アクセサ、`Debug` を `SampleRate(48000)` 型に整える先例）。`TimeBase::new` の検証付き const ctor と `Error::InvalidValue("field")` 形式。全公開 API への `**RT contract:**` ラベル（RT-suitable — pure integer math, no allocation を維持）。公開 enum の `#[non_exhaustive]` ポリシー（struct は対象外だが不変条件は ctor で強制）。なお `SampleRate::time_base()` は保持値が真の tick 長 `1/rate` だから意味を持つ先例であり、fps 有理数を保持する `FrameRate` では対応する露出は `frame_duration()`（逆数）のみとする
* Patterns to avoid: `FrameRate` の public フィールド露出（不変条件が破れる）。旧 `frame_rate: TimeBase` シグネチャとの併設（API 二重化）。保持する fps 有理数を `TimeBase` 値として露出するアクセサ・`From` 実装（`to_time_base` / `From<FrameRate> for TimeBase` — `TimeBase` 契約「1 tick の長」を破る値が流通する）。`f64` 経由の変換。外部クレート型（`vtc` 等）の公開 API 露出。`TimeBase` に対する安易な `Ord`/`PartialOrd` 追加（意図的に未実装の現行方針と衝突しないよう、`FrameRate` 側の比較は別途決める）
* Dependencies or configuration: 依存追加なし（media-core 依存ゼロ維持）。丸め累積のプロパティ検証で `proptest` 等を使う場合は dev-dependency として実装 Issue で理由記録のうえ追加、または自前 deterministic 列挙を選択する（設計文書の丸めポリシー節に従う）
* 移行: `from_media_time` / `from_media_time_drop` の引数型を `FrameRate` に置き換え、呼び出し側（現状はクレート内テスト・doctest のみ）を `FrameRate` 構築に書き換える。`StreamDescriptor.time_base` 等の `TimeBase` 正当利用は変更しない

## Verification

* [ ] `FrameRate` 型が `TimeBase` newtype として存在し、private フィールド + 検証付き `new()` + `frame_duration()` 等のアクセサを持つ（`nominal_fps` 等の派生 API は設計文書の提案どおり）。`to_time_base()` / `From<FrameRate> for TimeBase` が存在しない（fps 有理数を `TimeBase` 値として取り出せない）
* [ ] `Timecode::from_media_time` / `from_media_time_drop` が `frame_rate: FrameRate` を受け取り、旧 `TimeBase` シグネチャは残っていない（破壊的置換）
* [ ] ドロップフレーム対応集合が `30000/1001`・`60000/1001` のみのまま維持され、他レートで `Err(UnsupportedDropFrameRate)` を返す既存契約が変わっていない
* [ ] 設計ベースラインの分類表に「FrameRate/TimeBase 意味分離 = 決定」「ドロップフレーム対象レート集合 = 決定（現行維持）」「非 NTSC 一般化・フィールドレート = 保留」が記録されている
* [ ] `Error` は `#[non_exhaustive]` のままで、将来の variant 追加余地が確保されている（enum ADR 整合）
* [ ] 実装後に `mise run check` / `lint` / `test` がグリーンで、`Timecode` 関連 doctest が `FrameRate` で書き換わっている
* [ ] 本 ADR・spec・design・ベースライン・索引のリンクが `audit_docs.js --type all --json` で findings 0 を維持する

## Pros and Cons of the Options

### Option A — `FrameRate` = `TimeBase` の newtype（採用）

* 良い点: `SampleRate(TimeBase)` の newtype 先例に倣い、設計語彙が一貫する（完全な対称ではない: `SampleRate` は真の tick 長 `1/rate` を保持するが、`FrameRate` は fps 向き有理数を保持し、`TimeBase` 露出は逆数の `frame_duration()` のみとする）。`TimeBase` の gcd 正規化・`const fn` コンストラクタ・整数演算インフラをそのまま再利用でき、実装コストが最小。依存ゼロ維持。
* 悪い点: `TimeBase` の内部表現に依存するため、`TimeBase` の表現変更が `FrameRate` に波及する。fps 意味と tick 意味の変換方向（どちらが逆数か）を newtype の doc/アクセサで明示する設計が必要。

### Option B — `FrameRate` を独立 struct（u32 num/den 自前保持）

* 良い点: `TimeBase` の内部実装から完全に独立し、`FrameRate` 専用の不変条件・表現を自由に設計できる。`TimeBase` 側の将来変更に影響されない。
* 悪い点: gcd 正規化・検証ロジック・`i128` 変換ヘルパとの橋渡しを `TimeBase` と重複実装することになり、`SampleRate` 先例とも不整合になる。同じドメインルール（正の有理数レート）を 2 箇所で保守する DRY 違反。

### Option C — 型を増やさず `TimeBase` のまま API 名で区別（現状維持）

* 良い点: 変更コストゼロ。既存呼び出しはそのまま動く。
* 悪い点: `frame_rate: TimeBase` の意味混同（fps 向き有理数を tick 長の型に渡す逆転）が残り、呼び出し側が doc を読まないと `TimeBase::new(1_001, 30_000)`（tick 長解釈）を誤って渡す余地が型で防げない。Issue #42 の受入条件「責務が分離され意味的に正しい型を受け取る」を満たさない。

### Option D — `vtc` 等の外部型採用

* 良い点: SMPTE タイムコード計算の成熟した実装を利用でき、ドロップフレーム規則の検証済みロジックを自前実装しなくて済む。
* 悪い点: media-core の依存ゼロ方針と外部型非露出契約（外部型を公開 API に出さない）に直接衝突する。依存追加は理由記録が要る規約であり、`vtc` の型を公開 API に露出すれば外部クレートの semver に API が引きずられる。自前整数演算継続の既定方向（discovery で `vtc` は用途限定と記録）と矛盾するため不採用。

## More Information

* 一次ソース: GitHub Issue [#42 [design] 時間型拡張（FrameRate/TimeBase 意味分離・Timecode 拡張・丸め誤差累積方針）](https://github.com/scarecrow173/ruviform/issues/42)（受入条件 1 が本 ADR の対象）
* 上流ドキュメント（`relations.derives-from`）: [設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（時間モデルの確定契約: ドロップフレームは表示・ラベリング規約、有理数 MediaTime が同期・編集の基本通貨、音声↔動画変換は音声サンプル時刻優先）と [spec](../specs/time-type-extension-framerate-timecode-rounding.md)（本 ADR が決定する要件の定義）
* 派生ドキュメント（`relations.derived-by`）: [時間モデル拡張の設計](../designs/time-model-extension.md)（`FrameRate` API 形状・変換規約・丸め誤差累積ポリシーの詳細設計）
* 前提（`relations.depends-on`）: [公開 enum ADR](public-enum-non-exhaustive-policy.md)（拡張設計が従う `#[non_exhaustive]` ポリシーの正本）、[RT 契約 ADR](realtime-contract-and-verification-policy.md)（新 API の `**RT contract:**` ラベル規則）
* 関連（`relations.related`）: [`AGENTS.md`](../../AGENTS.md)（暫定規約 — unsafe 原則禁止・依存追加規約）
* 参考（`relations.references`）: [`vtc` クレート](https://crates.io/crates/vtc)（Option D の評価対象。SMPTE タイムコードの唯一の成熟クレートとして discovery に記録）、[Issue #41](https://github.com/scarecrow173/ruviform/issues/41)（本論点の延期元トリアージ）
