---
id: "TSPEC-034Jg7SJdGz3QogvORe8QT"
type: "test-spec"
status: "approved"
title: "メディア I/O 抽象境界とバックエンド PoC の契約保証"
created: "2026-10-20"
updated: "2026-10-21"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/implement-media-io-abstraction-and-first-backend-poc.md"
  derived-by: []
  verifies:
    - "../specs/implement-media-io-abstraction-and-first-backend-poc.md"
  verified-by: []
  references:
    - "https://github.com/pdeljanov/Symphonia"
  defers: []
  deferred-by: []
---

# メディア I/O 抽象境界とバックエンド PoC の契約保証

## Purpose

Issue #20 で導入するメディア I/O 抽象境界の振る舞い契約——demux/mux と decode/encode の 4 系統分離、`Packet` / codec 記述型 / `AudioFrame` の値型、エラー正規化と EOS=Option 終端、能力交渉の両機構、外部型非露出、非 RT 専用ラベル、symphonia PoC の end-to-end 検証——の「なぜテストが存在するか」を保持する。[spec](../specs/implement-media-io-abstraction-and-first-backend-poc.md) の受け入れ基準 AC1–AC11 と [plan](../plans/issue-20-i-o-poc.md) の検証方針を、実装の改変や第二バックエンド追加の際に「どの振る舞いを守るべきか」「そのテストはまだ必要か」を判断できる形で固定する。廃止してよいのは、対象契約自体を変更する ADR / spec 改訂が行われたとき（例: I/O 抽象境界の再編、外部型露出方針の変更、RT 契約ラベル体系の見直し）。

## Feature

バックエンド非依存のメディア I/O 抽象境界と、その第一参照実装 `ruviform-io-symphonia` の契約: `ruviform-media-io` が定義する trait・値型・エラーモデル・能力交渉が spec の不変条件を満たし、symphonia PoC がその境界を実際に駆動して demux → decode → `AudioFrame` 変換を end-to-end で検証すること。

## Rules

- **demux/mux と decode/encode は別の抽象として分離する**。`Demuxer`（コンテナ→packet）/ `Muxer`（packet→コンテナ）/ `Decoder`（packet→frame）/ `Encoder`（frame→packet）は 4 系統の別 trait であり、単一の「読み出し」「書き出し」抽象への併合を禁止する。抽象的な流れは `Demuxer` → `Packet` → `Decoder` → `AudioFrame` / `VideoFrame`、mux/encode はその逆方向（`Encoder` → `Packet` → `Muxer`）とする。
- **`Packet` はストリーム参照・PTS/DTS・フラグ・所有バイト列を持つ値型**。`StreamId` 等で media-core のストリームを参照し、キーフレーム等の符号化単位属性フラグと符号化バイト列を運ぶ。本契約のバイト列表現は所有型とし、借用 / zero-copy backing は将来評価へ送る。
- **codec 記述型は codec 識別・opaque パラメータ・format 写像の 3 責務を持つ**。media-io が新設する codec 記述型は、Ruviform 管理の codec ID/名による識別、codec private data の opaque バイト列保持、`AudioFormat` / `VideoFormat` への写像を担う。media-core の `StreamDescriptor` / `StreamKind` は codec 非依存の既存型をそのまま再利用し、codec 依存に改変しない。PCM PoC が通らない写像経路は合成データ・単体テストでカバーする。
- **音声デコード出力は `AudioFrame` で PTS/duration を運搬する**。`AudioBuffer` + `MediaTime` PTS + `Option<MediaDuration>` duration を束ねる専用の名前付き型を media-io に置き、`VideoFrame` の PTS+duration 形状と対称にする（タプル返却は採用しない）。audio-core は非改変とし、デコーダの音声出力は境界で planar f32 正準に変換した `AudioBuffer` を運搬する。
- **エラーは media-io 固有 `Error` / `Result` に正規化し、正常終端は `Option` で返す**。バックエンド由来の詳細は正規化分類（I/O 失敗・非対応・破損/不正データ・EOS 等）として内包し、外部エラー型・外部型名を公開 API に露出しない（詳細は `std::error::Error::source` 経由の型消去診断まで）。パケット・フレーム読み出しの正常終端（EOS）は `Err` ではなく `Option` / 専用終端表現で返し、`Error` の EOS 分類はハード終端・途中切断等の異常系に限定する。上流値型 `Error`（`InvalidValue` / `TimeOverflow` 等）は media-io `Error` の対応分類へ正規化写像する。
- **非対応は照会可能な能力情報と `Error` 非対応分類の両機構で表現する**。バックエンドの非対応機能（`Muxer` / `Encoder` 未提供・非対応 codec・codec 個別パラメータ等）は、能力照会 API と正規化 `Error` の非対応分類の両方で「非対応」と報告する。panic・`unimplemented!()`・暗黙的な失敗による表現は禁止し、全機能を一律汎用化しない。
- **公開 API の型純度を保ち外部型を露出しない**。両新規クレートの公開 API は Ruviform 固有型と std 型のみで構成し、外部クレートの型・エラー・列挙値・feature 名を `pub` で露出・再エクスポートしない。`ruviform-io-symphonia` の公開面にも symphonia 型を露出しない（参照実装としての型純度）。`ruviform-media-io` は domain 型の再エクスポートを行わない。
- **抽象境界はバックエンド非依存でビルドできる**。`ruviform-media-io` は外部クレート依存を持たず、`ruviform-io-*` への依存辺を持たない。バックエンド選択は `ruviform-io-*` 分離クレートへの opt-in `path` 依存として表現し、実行時の文字列指定等は設けない。symphonia 依存は `ruviform-io-symphonia` の `[dependencies]` に限定し、default features（royalty-free codec/format のみ）に留める。
- **シーク/フラッシュは受け口のみ提供し、実行時振る舞いは持たない**。`Demuxer` の時刻指定シーク要求 API と `Decoder` のフラッシュ（シーク後の残留状態を捨てる内部状態リセット）機構のみを提供し、A/V 再同期・提示順序・VFR 処理等の実行時振る舞いは engines の責務とする。
- **データ型はスレッド境界を越えられる `Send` を要求する**。`Packet` / `AudioFrame` 等のデータ型は I/O・デコードスレッド→有界キューのスレッド間移動を前提とし、`Send` 契約をコンパイル時に検証する。`**RT contract:** non-RT only` は API 呼び出しコンテキストの規制であり、データ型のスレッド間移動を禁じるものではない。
- **全公開 API に `**RT contract:** non-RT only` ラベルを付す**。両新規クレートは非 RT 専用であり、RT パスからの呼び出しは契約違反と文書化する。`Muxer` / `Encoder` は実装者ゼロの未検証契約として doc に明示する。
- **全公開 enum に enum レベル `#[non_exhaustive]` を付与する**。クレート外から到達可能な全公開 enum が対象であり、`exhaustive_enums` lint と `-D warnings` ゲートで強制する（公開 enum ポリシー ADR）。
- **symphonia PoC は in-memory 合成 WAV の demux → decode → `AudioFrame` を end-to-end で検証する**。LFS フィクスチャは使わず、テスト内で WAV/音声バイト列を合成して供給し、音声フレーム数・PTS 進行・サンプル値を検証する。
- **非 RT テスト名に `rt_` 部分文字列を含めない**。`cargo test --workspace rt_` の substring フィルタが RT レーンを選択するため、`assert_` / `start_` / `convert_` 等を含む `rt_` 部分文字列を持つテスト名・関数名は誤検出を招き禁止する。

## Examples

- Given: `cargo doc` 出力（または公開 trait 定義）。When: `ruviform-media-io` の trait 一覧を確認する。Then: `Demuxer` / `Muxer` / `Decoder` / `Encoder` が別 trait として存在し、demux と decode を併合した単一抽象は存在しない。
- Given: `StreamId`・`MediaTime` の PTS/DTS・キーフレームフラグ・符号化バイト列。When: `Packet` を構築してフィールドを読む。Then: 各値がそのまま保持され、`Send` 境界を越えて受け渡せる（`assert_send` 系の静的検証でコンパイル時に確認）。
- Given: codec private data を持つ codec 記述型（PCM PoC が通らない codec も含む合成データ）。When: `AudioFormat` / `VideoFormat` への写像を呼ぶ。Then: Ruviform 管理の codec ID/名が保持され、codec 固有パラメータが opaque に保持され、format 写像が期待値を返す。
- Given: デコーダが planar f32 の `AudioBuffer` を出力。When: `AudioFrame` として束ねる。Then: `MediaTime` PTS と `Option<MediaDuration>` duration が運搬され、`VideoFrame` と同形状である。audio-core のソース（`git status`）は非改変のまま。
- Given: demuxer がストリームの正常終端に達した。When: 次の packet 読み出しを行う。Then: `Err` ではなく終端表現（`None` 相当）が返る。Given: 途中切断・破損データ。When: 読み出す。Then: `Err` の EOS 分類（ハード終端）または破損/不正データ分類が返り、`source` 経由で型消去診断に到達できる。
- Given: `ruviform-io-symphonia` の能力照会 API。When: `Muxer` / `Encoder` の能力を照会する。Then: 非対応と報告される。When: `Muxer` / `Encoder` の取得を要求する。Then: panic / `unimplemented!()` ではなく、正規化 `Error` の非対応分類が返る（照会否定とエラー分類の両機構が一致）。
- Given: 両新規クレートの `cargo doc` 出力・公開 API 面を `symphonia` / `pub use` で grep する。When: 外部型・エラー・feature 名の露出を検査する。Then: `ruviform-media-io`・`ruviform-io-symphonia` 双方の公開面に外部由来の型・エラー・feature 名が存在しない。
- Given: `cargo check -p ruviform-media-io`。When: バックエンドクレート非依存で実行する。Then: 成功する。`cargo tree -p ruviform-media-io`（または `cargo metadata`）に `ruviform-io-*` への依存辺が存在しない。
- Given: `ruviform-io-symphonia` の `Cargo.toml` とワークスペース依存図。When: symphonia 依存の所在を確認する。Then: symphonia は `ruviform-io-symphonia` の `[dependencies]` にのみ記述され、`ruviform-media-io` には外部バックエンド依存が直接記述されていない。
- Given: demuxer と時刻 `t`。When: 時刻指定シーク要求 API を呼ぶ。Then: 要求が受理される（再同期・提示順序の振る舞いは対象外）。Given: decode 途中の `Decoder`。When: フラッシュを呼ぶ。Then: シーク前等の残留内部状態が捨てられ、以降の出力に持ち越されない。
- Given: 両新規クレートの全公開 API の doc コメントと全公開 enum。When: ラベルと属性を audit する。Then: 全公開 API に `**RT contract:** non-RT only` が付き、全公開 enum に enum レベル `#[non_exhaustive]` がある（`-D warnings` 下で `exhaustive_enums` ゲート通過）。
- Given: in-memory で合成した WAV バイト列。When: `ruviform-io-symphonia` で demux → decode → `AudioFrame` 変換を実行する。Then: 期待フレーム数が得られ、PTS が単調に進行し、デコードされたサンプル値が合成した入力と一致する（`mise run test` でグリーン）。
- Given: `mise run test-rt`（`cargo test --workspace rt_`）。When: 選択されるテスト集合を確認する。Then: `rt_` 部分文字列を持つ非 RT テスト名（`assert_` / `start_` / `convert_` 等）が混在せず、レーンが意図した集合に保たれる。

## Guarantees

- demux/mux と decode/encode の分離が trait 構造として固定され、engines・将来のバックエンド実装者が併合抽象に依存する余地を残さない。
- 外部型非露出により、symphonia の API churn やライセンス・feature 制御が公開面へ伝播せず、コアがバックエンド非依存のままビルドされる。
- 正常終端（`Option`）と異常終端（`Error` の EOS 分類）が区別され、終端判定を誤る呼び出し側実装が発生しない。
- `AudioFrame` により PTS/duration 運搬が型契約として閉じ、呼び出し側でのタプル管理・二重管理が不要になる。
- 非対応表現が照会・エラーの両機構で一貫し、panic 経路の無い参照実装が第二バックエンドの規範として機能する。
- e2e テストが実際の symphonia 経路を通り、PTS 運搬・planar f32 正準変換・`Send` 契約が実挙動として回帰検出される。
- 非 RT 専用の意図が API doc ラベルとして維持され、`rt_` レーンの選択集合が汚染されない。

## Non-goals

- 個別 codec/container の正式採否・対応範囲（symphonia PoC は境界実証目的であり、採否は後続 Issue）。
- 動画 decode の実装と `VideoFrame` 出力経路の実証（抽象契約の確定まで）。
- Mux / Encode のバックエンド実装と `Muxer` / `Encoder` 契約の実証（実装者ゼロの未検証契約。第一 encode 対応バックエンド Issue で実証）。
- シーク後の再同期・A/V 同期・VFR の実行時振る舞い（engines 責務。media-io は受け口のみ）。
- フレームプール・デコーダ zero-copy バッキング・`Packet` の借用表現（非 RT 層の後続設計）。
- RT 経路での I/O・no-alloc 検証（本クレートは非 RT 専用。RT パスへの供給は I/O・デコードスレッド + 有界キュー経路の engines/exec 側設計）。
- FFmpeg バックエンド `ruviform-io-ffmpeg`、および複数バックエンド間の同一境界・透過切替（media-io ADR Verification 第 3 項。第 2 バックエンド実装 Issue で検証）。
- gapless 情報（encoder delay / padding trim）の運搬先（後続論点）。
- パフォーマンス・ベンチマーク、OS リアルタイム保証。

## Risk

- 4 系統分離が併合抽象へ退行すると、engines がコンテナ処理と codec 処理を独立に組み替えられず、バックエンド交換が公開 API の破壊的変更になる。
- 外部型露出が起きると、symphonia の API 変更が利用者へ直接伝播し、分離クレート + opt-in 構造の強制力が失われる。
- EOS を `Err` で表す、または分類が混同されると、正常終端が異常として扱われる・逆に途中切断が静かなデータ欠損として見逃される。
- `AudioFrame` の PTS/duration 形状が揺らぐと、タイミング情報の運搬経路が呼び出し側に分散し、`VideoFrame` との対称性が失われて A/V 取り扱いが非一貫になる。
- 能力交渉が panic / `unimplemented!()` に依存すると、非対応機能の問い合わせ自体がクラッシュを引き起こし、「非対応」の表現規則を持つ境界設計の示範性を失う。
- `Send` 契約が欠けると、I/O・デコードスレッドから有界キューへのデータ移動がコンパイル時に破綻し、非 RT スレッド構成の設計前提が崩れる。
- `rt_` 部分文字列のテスト名が混入すると、RT レーンのフィルタ選択が信頼できなくなり、RT 契約テストの意図が薄れる。
- 受け口のみのはずのシーク/フラッシュに実行時振る舞いが混入すると、engines 側責務と二重管理になり、再同期・VFR の契約所在が曖昧になる。
