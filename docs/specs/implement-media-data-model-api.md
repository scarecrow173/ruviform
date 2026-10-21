---
id: "SPEC-034J5FYaYfeL5zkzb4sIV4"
type: "spec"
status: "implemented"
title: "メディアデータモデル API の実装（media-core / audio-core / video-core）"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/18"
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
    - "establish-rust-dev-foundation.md"
  blocks:
    - "implement-plugin-foundation.md"
    - "issue-18-followup-enum-and-time-fix.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../discovery/evaluate-plugin-binding-options.md"
  refines: []
  refined-by:
    - "../adr/public-enum-non-exhaustive-policy.md"
  derives-from:
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
  derived-by:
    - "../plans/implement-issue-18-data-model.md"
  verifies: []
  verified-by:
    - "../test-specs/media-data-model-api.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# メディアデータモデル API の実装（media-core / audio-core / video-core）

> **記録注記**: 本 spec は Issue #18 の実装（PR #39・merge commit `d041ba9`・feature commit `bc81838`）が完了・マージ済みの後に、doc-driven-dev-graph の正規チェーンを遡及整備するために作成した事後記録である。status `implemented` は実態（実装・検証・マージ済み）を反映する。

## What Should Be Built

GitHub Issue #18 で定義されたデータモデル実装の成果物として、`ruviform-media-core` / `ruviform-audio-core` / `ruviform-video-core` の 3 クレートに、設計ベースライン「データモデル」節の決定事項を公開 API として確定する。

- **時間モデル**（media-core）: 有理数 `TimeBase`（1 秒の有理数分割・gcd 正規化）＋符号あり整数カウントの `MediaTime` / `MediaDuration` / `MediaRange`。異なる timebase 間の変換は `i128` の正確な有理数演算で行い、表現不可能な場合は呼び出し側指定の `Rounding`（既定 `NearestEven` = round-half-to-even）を適用する。`StreamId` / `StreamKind` / `StreamDescriptor`、SMPTE ドロップフレームを含む `Timecode` 表示変換、将来の音楽時間に備えた opaque 予約フィールド `[u64; 2]` を持つ `TransportStatus` / `TransportSnapshot`、共通 `Error` 型を含む。
- **音声データモデル**（audio-core）: 処理用バッファの正準を planar f32（チャンネルメジャー連続）とし、interleaved はデバイス・ファイル境界の入出力形式として境界で変換する。sealed `Sample` trait（f32 が RT 正準、f64 は型パラメータのみ）、`SampleRate` / `SampleIndex` の音声時間契約（timebase `1/rate`。音声↔動画変換は音声サンプル時刻を優先）、`SpeakerMask` / `ChannelLayout`、`SampleFormat` / `AudioFormat`、`AudioBuffer` + 借用ビュー（`AudioBufferView` / `AudioBufferViewMut`）。
- **動画データモデル**（video-core）: `Plane` / `PlaneMut` / `SharedPlane`（plane 単位の stride は `width × bytes` と独立・パディング明示）、`#[non_exhaustive]` な `PixelFormat`（yuv420p/422p/444p、rgba8/rgbaf16）、色空間・HDR メタデータ（primaries / transfer / matrix / range + mastering display・content light level）、`Arc` バッキングによる zero-copy 供給を許容する `VideoFrame`（`MediaTime` PTS + `Option<MediaDuration>` duration）。
- f64 秒は中核表現としない。`to_seconds_f64()` 相当は文書化された表示用変換に限定する。
- RT 経路の値型は copyable / 確保不要アクセサ / 純粋関数変換とし、公開 API ごとに `**RT contract:**` ラベル（`RT-suitable` / `non-RT only`）を付す（RT ADR「保証範囲は API 単位で定義・明示」に準拠）。
- 依存配線: audio-core → media-core、video-core → media-core の `path` 依存。media-core は依存ゼロ、audio-core ↔ video-core の相互依存は作らない。

## Why Now

- 設計ベースライン「Phase 1: math＋基礎データ型」の完了目安（L504: 「データモデル（時刻・バッファ・フレーム・レイアウト）と RT 契約型・プリミティブが API として成立し、単体テストが通る」）を構成する中核項目であり、「後続実装 Issue の提案一覧」（L558 以降）の「データモデル実装 Issue（media-core/audio-core/video-core の API 確定）」として起票済み。
- Issue #17 で Phase 1 基盤 6 クレートのスケルトンが配置済みであり、次の工程は最下位の共有型を API として確定すること。全上位層（DSP・media-io・engines）がこれらの型に依存するため、API 未確定のままでは後続実装 Issue が着手できない。
- f64 秒を中核型にしない・planar f32 正準・stride 独立など、長時間ドリフトや RT 受け渡しに関わる設計判断はベースラインで決定済みであり、先送りすると後から API を破壊するリスクが大きい。

## Users and Value

- **engines / media-io の実装者**: `MediaTime`・`AudioBuffer`・`VideoFrame` 等の共有契約型が確定し、demux/decode パイプラインやエンジン実装の入力・出力型としてそのまま利用できる。
- **DSP・基盤の利用者**: RT パスで確保・ブロッキングを伴わないことが API 単位で明示された値型・ビュー型を使える。確保が必要な構築 API は `non-RT only` とラベル済みで、混用を設計時に回避できる。
- **レビュアー・後続 Issue の作業者**: API 単位の RT 契約ラベルと受け入れ基準により、何が RT-suitable か・何が非 RT 専用かをコードと文書の両面で検証できる。

## Scope

### In Scope

- `ruviform-media-core` の公開 API（`time` / `stream` / `error` / `timecode` / `transport` モジュール）
- `ruviform-audio-core` の公開 API（`sample` / `time` / `layout` / `buffer` / `format` モジュール）
- `ruviform-video-core` の公開 API（`pixel` / `color` / `plane` / `frame` モジュール）
- audio-core・video-core → media-core の `path` 依存配線と、README / AGENTS / ルート `Cargo.toml` の記述同期
- 公開 API の doctest / 単体テスト、`rt_` プレフィックス命名の RT 契約テスト

### Out of Scope

- `Processor` / `RtContext` / `MasterClock`（`ruviform-rt` の責務。`RtContext` 契約型は既存、残りは RT 基盤 Issue）
- `Packet` / demux / decode のポート抽象（`ruviform-media-io` の後続実装 Issue）
- 音楽時間（小節 / 拍 / PPQN・テンポマップ）— ベースラインで保留。`TransportSnapshot` の opaque 予約フィールドのみ措置
- f64 を中核・保存表現に使うこと（表示用変換のみ許可）、f64 サンプル対応の深さ（型パラメータのみ。需要ベースの後続評価）
- 色変換・色管理の実装（`ruviform-video-proc` / 第 2 ターゲット設計の責務）
- フレームプール・確保済みバッファプールの所有（非 RT 層の後続設計）
- オーディオデバイス境界 trait（アダプタポート契約でありデータモデルではない。別 Issue）
- SIMD / 音声処理・デコードの具体的実装、外部クレートの採用

## Requirements

- Rust edition 2024・MSRV 1.85・stable のみ。`publish = false`（workspace 継承）、`[lints] workspace = true`。`unsafe` は使用しない（局所許可なしで成立）
- 外部クレート依存ゼロ（std のみ。`Arc` 等の std 型は可。サードパーティ / バックエンド型を公開 API に露出しない）
- 全公開 API に doctest または単体テストのカバレッジを持たせる。RT-suitable な操作を検証するテストは `rt_` プレフィックスで命名し `mise run test-rt` で選択可能にする
- `MediaTime` / `MediaDuration` は異なる timebase 間でも一貫した `Eq` / `Ord` / `Hash`（正確な有理数比較・最低項正規化ハッシュ）を提供する
- ドロップフレームは表示・ラベリング規約として整数演算のみで実装し、単調な媒体時間（`MediaTime`）と分離する
- 値型の変換・演算でサイレントな f64 変換やパニックを生じさせない（`Result` / `Option` または文書化された飽和・panic 条件）

## Acceptance Criteria

Issue #18・API brief・設計ベースラインの決定事項に基づく。すべて実施・検証済み（PR #39、2026-10-20 マージ）。

- [x] 有理数 `TimeBase` + 整数カウント `MediaTime` / `MediaDuration` / `MediaRange` と、呼び出し側指定の丸め（5 モード・既定 `NearestEven`）を持つ timebase 変換 API（`convert` / `convert_exact` / `checked_add` / `checked_sub`）が存在する
- [x] planar f32 を正準とする `AudioBuffer`（チャンネルメジャー連続）と、境界専用の interleaved 変換（`from_interleaved` / `copy_to_interleaved` / `copy_from_interleaved`）が存在する
- [x] plane / stride / pixel format / 色・HDR メタデータ / `MediaTime` PTS を持つ `VideoFrame` モデルと、`Arc` 共有 backing を受け取る `from_shared_planes` zero-copy 供給経路が存在する
- [x] 公開 API ごとに `**RT contract:**` ラベル（RT-suitable / non-RT only）が付いている
- [x] audio-core・video-core → media-core の `path` 依存が配線され、media-core は依存ゼロ、audio ↔ video の相互依存はない（`cargo metadata` で確認済み）
- [x] doctest + 単体テストが整備され、`rt_` プレフィックスの RT 契約テストが 3 クレートに存在する
- [x] `mise run check` / `lint` / `test` / `test-rt` がグリーン（80 単体テスト + 50 doctest、rt_ レーンは意図した 10 テストのみ選択）
- [x] f64 を保存・中核表現に使っていない（`to_seconds_f64` は文書化済み表示用変換のみ）
- [x] 外部依存なし・`unsafe` なし（clippy `-D warnings` 下で 0 warnings）
- [x] README / AGENTS / ルート `Cargo.toml` の記述が実装済み状態と一致している

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 判断・移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| 音楽時間（小節 / 拍 / PPQN・テンポマップ） | `TransportSnapshot` に opaque `reserved: [u64; 2]` のみ残し型は持たない | 第 2 期・テンポマップ設計 Issue（ベースライン保留） | 後付けで公開 API を破壊する |
| f64 サンプル対応の深さ | `Sample` 型パラメータのみ（f32 が RT 正準） | 需要ベースの後続評価 | f64 機構を早計に広げると RT 正準が曖昧になる |
| 色変換・HDR 対応範囲 | メタデータ型のみ確定し変換処理は持たない | `ruviform-video-proc` 実装時の要 PoC 項目 | 変換仕様未確定のまま型を過剰設計する |
| デコーダ zero-copy バッキングの具体方式 | `SharedPlane` / `from_shared_planes` で許容する型契約のみ確定 | media-io 実装時の要 PoC 項目 | 機構先行で型が実需とずれる |
| フレームプール / バッファプール | 所有は持たず `Arc` 参照カウントまで | 非 RT 層の後続設計 | ライフタイム管理の責務境界が曖昧になる |
| `#[non_exhaustive]` 適用範囲 | `PixelFormat` / `SampleFormat` のみ適用（`publish = false` の内部クレート方針。残りの列挙型は exhaustive） | クレートを公開する場合のポリシー再評価 | 公開時に exhaustive 列挙型の拡張が breaking change 化する |

## Sources and Relations

- 一次ソース: GitHub Issue [#18 [impl] データモデル実装（media-core / audio-core / video-core の API 確定）](https://github.com/scarecrow173/ruviform/issues/18)（`relations.source`）。Issue #1 設計ベースライン「後続実装 Issue の提案一覧」から起票された。
- 上流文書（`relations.derives-from`）: [設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（「データモデル」節の決定事項・Phase 1 完了目安・後続実装 Issue 一覧）と [RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)（API 単位の RT 保証ラベル付け規則）。
- 前提（`relations.depends-on`）: spec [establish-rust-dev-foundation.md](establish-rust-dev-foundation.md)（`SPEC-034IcwhwIFSNUNKtX7KK0Q`）が確立した Rust 開発基盤（workspace・lint・mise 検証タスク）を本 spec の検証基盤として利用する。
- 派生（`relations.derived-by`）: [実装 plan](../plans/implement-issue-18-data-model.md)。検証意図は [test spec](../test-specs/media-data-model-api.md)（`relations.verified-by` 相互リンク）が保持する。
- 関連（`relations.related`）: リポジトリルートの [`AGENTS.md`](../../AGENTS.md)（暫定開発規約・unsafe 原則禁止・lint ゲート）。
- 事後注記（`relations.refined-by`）: 上記 Deferred Design Concerns の「`#[non_exhaustive]` 適用範囲」行（公開時に再評価と記録した延期判断）は、[ADR-034J6OnZouPI8HQy6cIg9D](../adr/public-enum-non-exhaustive-policy.md)（全公開 enum への enum レベル `#[non_exhaustive]` 統一方針）として前倒しで確定・適用された。
- 参考（`relations.references`）: 親 Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（設計ベースラインの一次ソース）。
