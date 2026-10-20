---
id: "SPEC-034JcaMXubILS7j1FX5qob"
type: "spec"
status: "approved"
title: "メディア I/O 抽象境界と第一バックエンド PoC の実装"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/20"
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
    - "implement-media-data-model-api.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../adr/licensing-and-distribution-policy.md"
  refines: []
  refined-by:
    - "../adr/public-enum-non-exhaustive-policy.md"
  derives-from:
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/realtime-contract-and-verification-policy.md"
  derived-by:
    - "../plans/issue-20-i-o-poc.md"
  verifies: []
  verified-by:
    - "../test-specs/media-io-abstraction-and-backend-poc.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
    - "https://github.com/pdeljanov/Symphonia"
  defers: []
  deferred-by: []
---

# メディア I/O 抽象境界と第一バックエンド PoC の実装

## What Should Be Built

GitHub Issue #20 で定義されたメディア I/O 実装の成果物として、抽象境界クレート `ruviform-media-io` と、第一バックエンド PoC `ruviform-io-symphonia` の 2 クレートをワークスペースに追加する。

- **`ruviform-media-io`（新規・抽象境界のみ）**: コンテナの demux/mux と codec の decode/encode を**別の抽象**として分離した 4 系統 trait を定義する（ベースライン L263-L266・media-io ADR）。抽象的な流れは `Demuxer` → `Packet` → `Decoder` → `AudioFrame` / `VideoFrame`、mux/encode は逆方向（`Encoder` → `Packet` → `Muxer`）であり、デマルチプレックスとデコードを一つの「読み出し」抽象に併合しない。本クレートはバックエンド実装を持たない。
- **音声/動画共通の仕組み**（ベースライン L267 の決定どおり）: `Packet`（ストリーム参照・PTS/DTS・フラグを持つ符号化単位・符号化バイト列）、media-core が定義する `StreamDescriptor` / `StreamKind::{Audio,Video,Data}` の再利用、能力交渉 API、共通エラーモデル。
- **codec 記述型**: media-core の `StreamDescriptor` は codec 非依存が意図的（`stream.rs` モジュール doc に明記）であるため、media-io 側に別の記述型を設け、codec 識別（Ruviform 管理の ID/名）・codec 固有パラメータ（codec private data の opaque 保持）・`AudioFormat` / `VideoFormat` への写像を担わせる。
- **`AudioFrame`（音声デコード出力型）**: `AudioBuffer` は PTS を持たない（audio-core は処理用バッファの純粋データ型であり、RT 正準の planar f32 を担う）一方、video-core の `VideoFrame` は `MediaTime` PTS + `Option<MediaDuration>` duration を内蔵しており非対称である。**決定**: audio-core を改変せず、media-io 側に `AudioBuffer` + `MediaTime` PTS + `Option<MediaDuration>` duration を束ねる専用の名前付き型 `AudioFrame` を設ける。タプル返却は採用しない（`VideoFrame` と同形状で API 対称性が得られ、フィールド追加を non-breaking にでき、境界の自己文書性が保てるため）。本決定はベースライン L265 の「`Decoder` → `AudioBuffer` / `VideoFrame`」記述を、PTS/duration 運搬のため `AudioBuffer` を内包する `AudioFrame` へ更新するベースラインとの差異であり、上流文書の同期更新対象とする（In Scope・AC10 参照）。
- **エラーモデル**: media-io 固有の `Error` / `Result`。バックエンド由来の詳細は正規化された分類として内包し、外部エラー型を公開 API に露出しない（ベースライン L270・media-io ADR）。
- **シーク受け口と decoder フラッシュ**: `Demuxer` に時刻指定シークの要求 API、`Decoder` にフラッシュ（内部状態リセット）機構を提供する。シークの実行時振る舞い・decoder フラッシュ後の A/V 再同期は engines の責務（ベースライン L239-L240）であり、media-io は受け口のみを提供する。
- **`ruviform-io-symphonia`（新規・分離クレート）**: 第一バックエンドとして symphonia を採用し、demux + decode のみを実装する PoC。symphonia は encode を持たないため `Muxer` / `Encoder` は提供せず、これを能力交渉の「非対応」表現の実例とする。**PoC の目的は抽象境界の実証であり、個別 codec/container の正式採否は本 spec の Out of Scope（後続 Issue）**である。
- 全公開 API に `**RT contract:** non-RT only` ラベルを付す。`ruviform-media-io` は非 RT 専用クレート（ベースライン L198）であり、RT パスからの呼び出しは契約違反。

## Why Now

- 設計ベースライン「後続実装 Issue の提案一覧」（L570）の「メディア I/O 実装 Issue（抽象境界 + 第一バックエンド PoC）」として起票済みであり、採否分類表（L476-L477）で「抽象境界は決定・個別対応は実装 Issue に委任」「symphonia = オーディオ codec 純 Rust 第一候補」と位置付けられている。
- Issue #18 / #41 でデータモデル API（`MediaTime` / `AudioBuffer` / `VideoFrame` / `StreamDescriptor` / `AudioFormat` / `PixelFormat`）が確定し、抽象境界の入力・出力型としてそのまま利用できる状態になった。これらを前提とする engines・exec の後続実装 Issue が demux/decode の入力契約を必要とするため、境界未確定のままでは着手できない。
- media-io ADR が確定した「抽象境界の背後に全バックエンドを配置・外部型非露出・分離クレート + opt-in」は、実バックエンド 1 件で検証するのが最も安い。利用クレートが増えた後に境界契約の欠陥が判明すると、修正が公開 API の破壊になる。
- symphonia は discovery で「純 Rust decode の最有力・Lean=yes・`forbid(unsafe)`・trait 分離設計が抽象境界の好モデル」（explore-architecture-design-baseline L155）と評価され、MPL-2.0（弱コピーレフト・ファイル単位）と MSRV 1.85 整合が Ruviform 暫定規約と衝突しない。

## Users and Value

- **engines（`ruviform-audio` / `ruviform-video`）実装者**: `Demuxer` / `Decoder` / `AudioFrame` / `VideoFrame` の確定した契約でファイル→フレーム経路を組め、シーク要求・decoder フラッシュの受け口と engines 側責務（再同期・実行時振る舞い）の境界が明確になる。
- **バックエンド実装者**（将来の `ruviform-io-ffmpeg` 等）: 実装すべき trait・能力交渉・エラー正規化の規則が確立され、`ruviform-io-symphonia` が参照実装として機能する。
- **`ruviform-exec` / オフライン処理の実装者**: ブロッキング I/O を含む demux/decode を非 RT の I/O・デコードスレッド（ベースライン L301）から駆動する際の入力契約が得られる。
- **レビュアー・後続 Issue の作業者**: 「外部型非露出」「コアはバックエンドなしでビルド可能」「demux/mux と decode/encode の分離」がクレート構造と受け入れ基準で客観的に検証できる。

## Scope

### In Scope

- `crates/ruviform-media-io` の新設: `Demuxer` / `Muxer` / `Decoder` / `Encoder` trait、`Packet`、codec 記述型、`AudioFrame`、能力交渉 API、共通 `Error` / `Result`、demuxer シーク要求 API・decoder フラッシュ機構
- `crates/ruviform-io-symphonia` の新設: symphonia による demux + decode のみを実装する第一バックエンド PoC（`Muxer` / `Encoder` 非提供 = 「非対応」表現の実例）
- `ruviform-media-io` → `ruviform-media-core` / `ruviform-audio-core` / `ruviform-video-core` の `path` 依存配線、`ruviform-io-symphonia` → `ruviform-media-io`（+ 必要に応じ domain クレートへの直接 `path` 依存。ベースライン L203 の許容範囲）の `path` 依存配線、ルート `Cargo.toml` `members` 追加、README / AGENTS の記述同期、設計ベースライン L265 相当行の同期更新（`AudioBuffer` 出力記述 → `AudioFrame`）
- 公開 API の doctest / 単体テスト。PoC 検証は in-memory で WAV/音声バイト列を合成し demux → decode → `AudioFrame` 変換の一連の実行（end-to-end）を検証するテストとする（LFS フィクスチャ不使用）
- 全公開 API の `**RT contract:**` ラベル（本 spec の対象はすべて `non-RT only`）と、公開 enum への enum レベル `#[non_exhaustive]`（公開 enum ポリシー ADR）

### Out of Scope

- 個別 codec/container の正式採否・対応範囲の拡大（ベースライン L476 のとおり後続 Issue。symphonia PoC は境界実証目的であり codec 採否決定ではない）
- 動画 decode の実装（symphonia は音声専用。`VideoFrame` 出力経路は抽象契約の確定まで）
- Mux / Encode のバックエンド実装（symphonia は encode 非対応。抽象 trait の定義と「非対応」表現まで）
- シーク・A/V 同期・VFR の実行時振る舞い（engines 責務。media-io は受け口のみ）
- フレームプール・デコーダ zero-copy バッキングの具体方式（ベースライン L478 要 PoC の残存項目・非 RT 層の後続設計）
- RT 経路での I/O（本クレートは非 RT 専用。RT パスからの呼び出しは契約違反）
- FFmpeg バックエンド `ruviform-io-ffmpeg`（後続 Issue・ライセンス個別調査を経て採否）
- GUI / engine 統合（アプリケーション・エンジン層の責務）

## Requirements

- Rust edition 2024・MSRV 1.85・stable のみ。`publish = false`（workspace 継承）、`[lints] workspace = true`。`unsafe` は使用しない（局所許可なしで成立）
- **クレート構成**: メディア I/O の概念は単一クレート `ruviform-media-io` に集約する（ベースライン L263/274 の決定。`ruviform-audio-io` / `ruviform-video-io` 分離案は不採用済み）。`ruviform-media-io` の依存は Primitives + `ruviform-audio-core` / `ruviform-video-core`（ベースライン L198）に限定し、`ruviform-media-io` 自身は外部クレート依存を持たない
- **4 系統分離**: `Demuxer`（コンテナ→packet）/ `Muxer`（packet→コンテナ）/ `Decoder`（packet→frame）/ `Encoder`（frame→packet）は別 trait とし、単一抽象への併合を禁止する
- **`Packet`**: ストリーム参照（`StreamId` 等）・PTS/DTS・フラグ（キーフレーム等の符号化単位属性）・符号化バイト列を持つ値型とする
- **共通仕組み**: `StreamDescriptor` / `StreamKind` は media-core の既存型を再利用する。codec 記述型は media-io が新設し、codec 識別（Ruviform 管理の codec ID/名）・codec 固有パラメータ（opaque バイト列等）・`AudioFormat` / `VideoFormat` への写像を担う（`StreamDescriptor` を codec 依存に改変しない）
- **`AudioFrame`**: `AudioBuffer` + `MediaTime` PTS + `Option<MediaDuration>` duration を束ねる専用の名前付き型を media-io に設ける（audio-core は非改変。`VideoFrame` の PTS+duration 形状との対称性を根拠とする）。デコーダの音声出力は境界で planar f32 正準へ変換した `AudioBuffer` を運搬する（audio-core の正準規約どおり）
- **エラーモデル**: media-io 固有の `Error` / `Result` を定義し、バックエンド由来の詳細は正規化された分類（I/O 失敗・非対応・破損/不正データ・EOS 等）として内包する。外部エラー型・外部型名を公開 API に露出しない（外部詳細は型消去済みの診断情報・`std::error::Error::source` 経由までとする）。パケット・フレーム読み出しの終端（EOS）は `Err` ではなく `Option` / 専用終端表現で返す（`Error` の EOS 分類はハード終端・途中切断等の異常系に限定）。media-core / audio-core 等の上流値型 `Error`（`InvalidValue` / `TimeOverflow` 等）は media-io `Error` の対応分類へ正規化写像し、詳細は `source` 経由の型消去診断とする
- **能力交渉**: バックエンドの非対応機能（`Muxer` / `Encoder` 未提供・非対応 codec・codec 個別パラメータ等）は、照会可能な能力情報と `Error` の非対応分類の両方で「非対応」として表現する。panic・暗黙的な失敗・`unimplemented!()` による表現は禁止。全機能を一律汎用化しない（ベースライン L269・ADR Consequences）
- **バックエンド選択**: `ruviform-io-*` 分離クレートへの依存として表現し、実行時の文字列指定等は設けない（media-io ADR）
- **公開 API の型純度**: `ruviform-media-io` の公開 API は Ruviform 固有型と std 型のみで構成し、外部クレートの型・エラー・列挙値・feature 名を `pub` で露出・再エクスポートしない
- **RT 分離**: 非 RT 専用クレートとし、全公開 API に `**RT contract:** non-RT only` ラベルを付す。RT パスからの呼び出しは契約違反と文書化する。`**RT contract:** non-RT only` は API 呼び出しコンテキストの規制であり、`Packet` / `AudioFrame` 等のデータ型はスレッド間（I/O・デコードスレッド→有界キュー）を移動可能とする（`Send` を要求。trait object/所有権をスレッド境界で運ぶ前提）。有界キュー手前での `AudioFrame` unwrap/分解境界は engines 側の設計とする
- **シーク/フラッシュの責務限定**: `Demuxer` の時刻指定シーク要求 API と `Decoder` のフラッシュ機構のみ提供し、再同期・提示順序・VFR 処理等の実行時振る舞いは持たない
- **公開 enum**: クレート外から到達可能な全公開 enum に enum レベル `#[non_exhaustive]` を付与する（公開 enum ポリシー ADR）
- **symphonia PoC**: `ruviform-io-symphonia` は `ruviform-media-io` の trait を実装する差替可能な実装詳細とし、demux + decode のみ提供する。`ruviform-io-symphonia` の依存は `ruviform-media-io` と、必要に応じ `ruviform-media-core` / `ruviform-audio-core` / `ruviform-video-core` の直接 `path` 依存（ベースライン L203 の許容範囲）とし、media-io は domain 型の再エクスポートを行わない。symphonia 依存は `ruviform-io-symphonia` の `[dependencies]` に限定し、default features（royalty-free codec/format のみ）に留めて `all` 等の特許対象 codec を含む feature は有効化しない。依存追加理由（discovery 評価・MPL-2.0・MSRV 整合・`forbid(unsafe)`）を PR に記録する（暫定規約の依存追加規約）
- **検証方法**: テストは in-memory で WAV/音声バイト列を合成して供給する（LFS フィクスチャ不使用）。公開 API に symphonia 型が露出しないことを確認する。非露出の対象は `ruviform-media-io` の公開 API が正本であり、`ruviform-io-symphonia` の公開 API にも symphonia 型を露出しない（ベースライン L214 のバックエンド内部 API 安定性契約対象外とは別に、参照実装として型純度を維持する）。非 RT テスト名に `rt_` 部分文字列を含めない（`cargo test --workspace rt_` の部分文字列フィルタ誤検出防止。`assert_` / `start_` / `convert_` 等も該当）
- 全公開 API に doctest または単体テストのカバレッジを持たせる

## Acceptance Criteria

Issue #20・media-io ADR「Verification」節・設計ベースラインの決定事項に基づく。

- [ ] **AC1**: `ruviform-media-io` と `ruviform-io-symphonia` の両クレートの公開 API（`cargo doc` 出力・公開 trait/型/enum）に外部クレート由来の型・エラー・feature 名が存在しない（media-io ADR Verification 第 1 項。`ruviform-io-symphonia` 側は参照実装としての型純度を対象とする）
- [ ] **AC2**: `cargo check -p ruviform-media-io` がバックエンドクレート非依存で成功し、`cargo tree -p ruviform-media-io`（または `cargo metadata`）で `ruviform-io-*` への依存辺が存在しない（分離クレート構造で強制。ADR Verification 第 2 項に対応）
- [ ] **AC3**: `Demuxer` / `Muxer` / `Decoder` / `Encoder` が別 trait として分離され、demux/mux と decode/encode が別抽象として存在する（ADR Verification 第 4 項）
- [ ] **AC4**: `ruviform-io-symphonia` が分離クレート + opt-in 依存として追加され、`ruviform-media-io` の `[dependencies]` に外部バックエンド依存が直接記述されていない（ADR Verification 第 5 項）
- [ ] **AC5**: `ruviform-io-symphonia` の PoC テストが、in-memory 合成した WAV/音声バイト列の demux → decode → `AudioFrame`（`AudioBuffer` + PTS/duration）変換の一連の実行（end-to-end）を行い、`mise run test` で成功する（LFS フィクスチャ不使用・音声フレーム数・PTS 進行・サンプル値を検証）
- [ ] **AC6**: 能力交渉により、`ruviform-io-symphonia` の能力照会 API が `Muxer` / `Encoder` を非対応と報告し、かつ `Muxer` / `Encoder` の取得要求が正規化された `Error` の非対応分類を返す（両機構で検証）。panic / `unimplemented!()` でないことがテストまたは API レビューで確認できる
- [ ] **AC7**: `AudioFrame` が `MediaTime` PTS と `Option<MediaDuration>` duration を運搬し、`AudioBuffer`（audio-core）が非改変である。`Decoder` がシーク後の残留状態を捨てるフラッシュ機構を持ち、`Demuxer` が時刻指定シーク要求 API を持つ
- [ ] **AC8**: `mise run fmt-check` / `check` / `lint` / `test` と `mise exec -- prek run --all-files` がグリーン。`unsafe` 不使用（clippy `-D warnings` 下で 0 warnings）
- [ ] **AC9**: 両新規クレートの全公開 API に `**RT contract:** non-RT only` ラベルが付き、クレート外到達可能な全公開 enum に enum レベル `#[non_exhaustive]` がある
- [ ] **AC10**: README / AGENTS / ルート `Cargo.toml` / 設計ベースライン L265 相当行（`AudioBuffer` 出力記述 → `AudioFrame`）の記述が実装済み状態（2 クレート追加・依存配線・非 RT 専用の記載）と一致している
- [ ] **AC11**: codec 記述型が codec 識別（Ruviform 管理の codec ID/名）・codec 固有パラメータ（opaque バイト列）保持・`AudioFormat` / `VideoFormat` への写像を備え、単体テストで検証される（PCM PoC では通らない経路のため、合成データ/単体テストでカバーする）

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 判断・移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| 個別 codec/container の正式採否・対応範囲 | symphonia PoC は境界実証目的とし採否決定を含まない | 後続 Issue（ベースライン L476・media-io ADR「個別 codec/container・バックエンド選定は後続 Issue」） | PoC の試験的対応が暗黙の正式採否と混同され、ライセンスレビュー・対応範囲ポリシーが未決のまま固定化する |
| Mux / Encode のバックエンド実装 | 抽象 trait と「非対応」表現のみ確定（symphonia は encode 非対応） | Mux/Encode 対応バックエンドの後続 Issue | 書き出し経路未実装のまま encode 側契約が実需で腐る |
| 動画 decode 実装・`VideoFrame` 出力経路 | 抽象契約（`Decoder` → `VideoFrame`）のみ確定 | 動画 decode 対応バックエンドの後続 Issue（純 Rust は AV1/H.264 限定・広範囲は FFmpeg が現実解 — discovery） | 動画ストリーム経路が抽象だけで未検証のまま残る |
| FFmpeg バックエンド `ruviform-io-ffmpeg` | 本 spec では扱わない | 後続 Issue。licensing ADR の個別調査（LGPL/GPL・配布条件）を経て採否 | 広範 codec/container 非対応のまま実用範囲が限定される |
| シーク・A/V 同期・VFR の実行時振る舞い | 受け口（シーク要求・フラッシュ）のみ提供 | engines 実装 Issue（ベースライン L239-L240） | 責務が media-io に混入し engines 側契約と二重管理になる |
| フレームプール・zero-copy バッキングの具体方式 | PoC は `AudioFrame` への値返却まで（プール・共有バッキング方式は未定） | 非 RT 層の後続設計（ベースライン L478 要 PoC）・Issue #26 系 | 機構先行で型が実需とずれる／フレーム確保の責務境界が曖昧になる |
| デコーダ出力の表現形（enum 分岐 vs ストリーム種別ごとの型付き受け口）・`Data` ストリームの packet 扱い | `AudioFrame` / `VideoFrame` 出力の契約のみ確定し、API 表現形は実装で確定 | 実装 plan / 後続の API レビュー | 抽象肥大化または表現不足のどちらかに過剰適合する |
| RT 経路での I/O・メディア供給 | 非 RT 専用を明確化（RT 呼び出しは契約違反） | RT パスへの供給は I/O・デコードスレッド + 有界キュー経路（ベースライン L301）の engines/exec 側設計 | RT パスからの誤呼び出しが契約違反として検出できない |
| ADR Verification 第 3 項（複数バックエンド間の同一境界・透過切替） | 第一バックエンドのみの本 spec では検証不能 | 第 2 バックエンド実装 Issue で検証 | 複数バックエンドでの透過切替が未検証のまま境界契約が固定化する |
| `Packet` のバイト列表現（所有 vs 借用/zero-copy） | 所有バイト列を持つ値型とする | demux 側の借用/zero-copy backing は将来評価（非 RT 層の後続設計・Issue #26 系と併せて） | コピー前提の契約が性能要件とずれた場合に API 改訂が必要になる |
| `Muxer` / `Encoder` の契約実証 | 実装者ゼロの未検証契約（抽象 trait と「非対応」表現のみ確定） | 第一 encode 対応バックエンド実装 Issue で実証（0.x / `publish=false` 下で改訂可能） | 未検証契約のまま利用者が増えると、契約欠陥の修正が破壊的変更になる |
| gapless 情報（encoder delay / padding trim）の運搬先 | 本 spec では運搬先を定めない | 後続論点（codec 記述型・`AudioFrame`・能力交渉のいずれに載せるかを含めて検討） | 運搬先未確定のまま codec 対応を増やすと gapless 再生契約が後付けになる |

## Sources and Relations

- 一次ソース: GitHub Issue [#20 [impl] ruviform-media-io 抽象境界 + 第一バックエンド PoC](https://github.com/scarecrow173/ruviform/issues/20)（`relations.source`）。Issue #1 設計ベースライン「後続実装 Issue の提案一覧」の「メディア I/O 実装 Issue（抽象境界 + 第一バックエンド PoC）」から起票された。
- 上流文書（`relations.derives-from`）: [設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（「メディア I/O 抽象」節 L263-L274・クレート構成表 L198・シーク責務 L239-L240・採否表 L476-L477）、[media-io ADR](../adr/media-io-backend-abstraction.md)（抽象境界・外部型非露出・分離クレート + opt-in・Verification 節）、[RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)（API 単位の RT 保証ラベル付け規則・非 RT 専用の明示）。
- 前提（`relations.depends-on`）: spec [implement-media-data-model-api.md](implement-media-data-model-api.md)（`SPEC-034J5FYaYfeL5zkzb4sIV4`・implemented）。本 spec の入出力型（`MediaTime` / `StreamDescriptor` / `StreamKind` / `AudioBuffer` / `AudioFormat` / `VideoFrame` / `PixelFormat`）を確定した先行 spec。
- 派生（`relations.derived-by`）: [実装 plan](../plans/issue-20-i-o-poc.md)。検証意図は [test spec](../test-specs/media-io-abstraction-and-backend-poc.md)（`relations.verified-by` 相互リンク）が保持する。
- 適用ポリシー（`relations.refined-by`）: [公開 enum ADR](../adr/public-enum-non-exhaustive-policy.md)。本 spec の新規クレートの全公開 enum に enum レベル `#[non_exhaustive]` を適用する統一ポリシーの正本。
- 関連（`relations.related`）: リポジトリルートの [`AGENTS.md`](../../AGENTS.md)（暫定開発規約・unsafe 原則禁止・依存追加規約・lint ゲート）、[licensing ADR](../adr/licensing-and-distribution-policy.md)（symphonia の MPL-2.0・依存 feature 制限・個別 codec/バックエンドのライセンス個別調査ポリシー）。
- 参考（`relations.references`）: 親 Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（設計ベースラインの一次ソース）、[Symphonia](https://github.com/pdeljanov/Symphonia)（第一バックエンド PoC の対象・discovery 評価の起点）。
