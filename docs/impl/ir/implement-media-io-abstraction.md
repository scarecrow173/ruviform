---
id: "IMPL-034JxNThv4gsW2IpbkYmK4"
type: "impl"
status: "completed"
title: "Implement media-io abstraction"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/20"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-media-io/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-media-io/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/error.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/source.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/packet.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/codec.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/frame.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/capability.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/demux.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/mux.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/decode.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/encode.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/tasks/implement-media-io-abstraction.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/implement-media-io-abstraction.md"
  implemented-by: []
  depends-on:
    - "../../plans/issue-20-i-o-poc.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/issue-20-i-o-poc.md"
    - "../../specs/implement-media-io-abstraction-and-first-backend-poc.md"
    - "../../adr/media-io-backend-abstraction.md"
    - "../../adr/realtime-contract-and-verification-policy.md"
    - "../../adr/public-enum-non-exhaustive-policy.md"
    - "../../designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/issue-20-i-o-poc.md"
    - "../../specs/implement-media-io-abstraction-and-first-backend-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/media-io-abstraction-and-backend-poc.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement media-io abstraction

## Summary

task `TASK-034JghH9i5p72LmOoWIPGe`（plan `PLAN-034JfKRCFCu8oJr5iCe6AV` Phase 1・Issue #20 タスク1）として、バックエンド実装を持たない抽象境界クレート `ruviform-media-io` を新設した。`Demuxer` / `Muxer` / `Decoder` / `Encoder` の 4 系統 trait 分離、`Packet` / `PacketFlags`、codec 記述型（`CodecId` / `CodecFormat` / `CodecDescriptor`）、`AudioFrame` / `Frame`、能力交渉照会側（`Capabilities` / `Features`）、正規化エラー（`Error` / `ErrorKind` / `Result`）、入力ポート `MediaSource`、列挙ペア `StreamInfo` を公開 API として定義し、crate root からフラット再エクスポートした。外部クレート依存ゼロ・`unsafe` 不使用・domain 3 クレートへの `path` 依存のみ。

## Context

- 実施タスク: [../../tasks/implement-media-io-abstraction.md](../../tasks/implement-media-io-abstraction.md)（`TASK-034JghH9i5p72LmOoWIPGe`）
- 上流: [plan `PLAN-034JfKRCFCu8oJr5iCe6AV` Phase 1](../../plans/issue-20-i-o-poc.md)、[spec `SPEC-034JcaMXubILS7j1FX5qob`](../../specs/implement-media-io-abstraction-and-first-backend-poc.md)、[media-io ADR](../../adr/media-io-backend-abstraction.md)、[設計ベースライン](../../designs/ruviform-architecture-design-baseline.md)「メディア I/O 抽象」節
- 正本要件: spec Requirements（4 系統 trait 分離・`Packet`・codec 記述型・`AudioFrame`・エラーモデル・能力交渉照会側・seek/flush 受け口・`Send` 契約・型純度）・AC2/AC3/AC7/AC11

## Implementation

### Changes

- `crates/ruviform-media-io/Cargo.toml` — workspace 継承 + `[lints] workspace = true` + domain 3 クレートへの `path` 依存のみ（前セッションで作成済み）。ルート `Cargo.toml` の `members` 追加済み（`Cargo.lock` はビルドで自動更新）
- `src/error.rs` — `Error` / `ErrorKind` / `Result`（前セッション作成済み・今回 `FFmpeg` の doc バックティックのみ修正）。正規化分類（`Io` / `Unsupported` / `InvalidData` / `InvalidValue` / `TimeOverflow` / `UnexpectedEos`）+ 型消去 `source` 診断 + `From<media::Error>` / `From<std::io::Error>` 正規化写像。正常 EOS = `Ok(None)` 規約をモジュール doc に明記
- `src/source.rs` — `MediaSource: Read + Seek + Send`（blanket impl）。error.rs の `crate::MediaSource` 参照を解決し、バックエンド demuxer 構築の入力ポートを供給。当初は `byte_len` デフォルト `None` メソッドを持っていたが、blanket impl との coherence 衝突で手動 impl が構造的に不可能（dead 拡張点）とレビューで判明したため削除 — シーク可否・バイト長はバックエンド側が `Seek` probe で導出する（[T3 IR](sync-docs-and-verify-issue-20.md) 修正 wave 参照）
- `src/packet.rs` — `Packet`（`StreamId`・`Option<MediaTime>` pts/dts・`Option<MediaDuration>`・`PacketFlags`・所有 `Vec<u8>`、リテラル構築）+ `PacketFlags`（`u32` bit set・`KEYFRAME` / `CORRUPT`、audio-core `SpeakerMask` パターン踏襲）
- `src/codec.rs` — `CodecId`（`#[non_exhaustive]`、Ruviform 管理 ID + canonical `name()`。`Other` + PCM/AAC/MP3/Vorbis/Opus/FLAC/ALAC + H.264/HEVC/VP9/AV1）・`CodecFormat`（`Audio(AudioFormat)` / `Video(VideoFormat)` / `Data`）・`CodecDescriptor`（codec + opaque `codec_private: Vec<u8>` + `format`、`audio_format()` / `video_format()` 写像）。AC11 の 3 要件を単体テストで固定
- `src/frame.rs` — `AudioFrame`（`AudioBuffer` + `MediaTime` pts + `Option<MediaDuration>`、private fields + `VideoFrame` 型アクセサ形状。正準 planar f32 のみ）+ `Frame`（`#[non_exhaustive]` enum、`Audio` / `Video`、`pts()` 統一アクセサ）
- `src/capability.rs` — `Features`（4 系統の bit set）+ `Capabilities`（`functions` + `decode_codecs` / `encode_codecs`、`can_demux` / `can_mux` / `can_decode` / `can_encode`）
- `src/demux.rs` — `Demuxer` trait（`streams` 列挙・`stream` id 検索デフォルト実装・`read_packet`・`seek` 時刻指定要求）+ `StreamInfo`（`StreamDescriptor` + `CodecDescriptor` ペア）
- `src/mux.rs` / `src/encode.rs` — `Muxer`（`streams` / `write_packet` / `finish`）・`Encoder`（`send_frame` / `receive_packet` / `drain` デフォルト no-op）。両 trait のモジュール doc に「実装者ゼロの未検証契約」を明記
- `src/decode.rs` — `Decoder` trait（`send_packet` / `receive_frame` / `drain` デフォルト no-op / `flush` 残留状態リセット）。send/receive プロトコル・`Ok(None)` EOS・seek 受け口のみの責務限定を doc 化
- `src/lib.rs` — crate doc（責務・許可依存層・非 RT 専用・RT パスからの呼び出しは契約違反・domain 型の再エクスポートなし）+ 自クレート型のみのフラット再エクスポート + `version()`
- `docs/tasks/implement-media-io-abstraction.md` — `status: "done"`、Work / Done When / Verification チェック更新、`source.rs` を changes / Affected Files に追記

### Decisions During Implementation

- **デコーダ出力表現（spec Deferred）**: `Frame` enum 分岐（`Audio(AudioFrame)` / `Video(VideoFrame)`、`#[non_exhaustive]`）を採用。ストリーム種別ごとの型付き受け口案は 4 trait 分離の構造と不整合なため棄却。`Decoder` / `Encoder` はともに `Frame` を通じるため対称。`Data` ストリームは decoder を通さず packet 直通とし、将来の `Frame` 拡張余地は `#[non_exhaustive]` で確保
- **send/receive ポンプ + `drain`**: error.rs が既に `Decoder::receive_frame` / `Encoder::receive_packet` を doc 参照していたため FFmpeg 型の送受信分離を採用。遅延 codec（AAC のエンコーダ遅延・B-frame 並替）を境界が駆動完了できるよう `drain()`（入力終端シグナル・デフォルト no-op）を追加 — spec の「packet→frame」契約の完全性に必須と判断。spec 必須の `flush`（seek 後の残留状態リセット）は要求実装のまま
- **`Packet` の時刻は `Option`**: コンテナが全 unit に時刻を持つとは限らないため `pts` / `dts` / `duration` を全て `Option<…>` とした（`AV_NOPTS_VALUE` 相当の表現力）。`AudioFrame` 側は `VideoFrame` 対称で pts 非 Option
- **構築はバックエンド側・trait はポートのみ**: `Demuxer` / `Muxer` 等にコンストラクタ要件を持たせず、生成はバックエンド固有エントリポイント（`MediaSource` を受ける）に委譲。trait object の `Send` は supertrait で表明（`Box<dyn Demuxer>` がスレッド境界を越える前提）
- **`MediaSource` 追加（File Map からの差異）**: error.rs の `crate::MediaSource` doc 参照を解決するため `Read + Seek + Send` の blanket impl trait を `source.rs` として新設。sink 側ポートは spec 要求外・`Muxer` 未検証契約のため YAGNI で不採用
- **`Capabilities` は bool → `Features` bit set へ**: 初期案の 4 bool フィールドが `clippy::struct_excessive_bools`（pedantic）に抵触したため、`PacketFlags` と同型の `u32` flags 型 `Features` に再設計（拡張時に const 追加で non-breaking）
- **codec ID 管理**: `CodecId` を `#[non_exhaustive]` enum の Ruviform レジストリとし、canonical `name()` を `const fn` で提供。`Other` バリアントで未識別 codec の列挙を可能に（未知 codec でも demuxer が stream 列挙を失敗させない）。詳細 PCM 系譜（s16le 等）は `codec_private` + decoded format に寄せて variant 細分化を見送り（後から non-breaking で追加可）
- **`Muxer::write_packet` は所有権移転**: `send_packet` / `send_frame` と揃えて値渡し（interleave のためバックエンドが retain する実装でコピー不要）

## Related Experiments

### Adopted

なし（実験ログ運用なし・直接実装）

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `mise exec -- cargo fmt -p ruviform-media-io` — 整形済み（差分なし）
- `mise exec -- cargo test -p ruviform-media-io` — 40 unit + 19 doctest、全 pass
- `mise exec -- cargo clippy -p ruviform-media-io --all-targets -- -D warnings` — 0 warnings（`exhaustive_enums` ゲート含む。初回指摘 `struct_excessive_bools` / doc list indent / `doc_markdown` / `assert_is_empty` は修正済み）
- `mise exec -- cargo check -p ruviform-media-io` — 成功（バックエンド非依存・AC2）
- `mise exec -- cargo tree -p ruviform-media-io` — 依存辺が `ruviform-media-core` / `ruviform-audio-core` / `ruviform-video-core` のみ（外部依存・`ruviform-io-*` 辺なし。AC2/AC4）
- `mise exec -- cargo doc -p ruviform-media-io --no-deps` — 生成成功・警告なし（外部型非露出・4 trait 分離を TOC で確認）
- `grep -rn "pub enum" crates/ruviform-media-io/src` — 4 enum（`CodecId` / `CodecFormat` / `ErrorKind` / `Frame`）全て enum レベル `#[non_exhaustive]`（AC9）
- `grep -rn "unsafe" crates/ruviform-media-io/src` — 0 件（AC8 関連）
- `mise exec -- cargo test -p ruviform-media-io rt_` / `--doc rt_` — 選択 0 件（unit 40 / doctest 19 全てフィルタアウト。`rt_` 部分文字列を含むテスト名なし）
- `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` — ワークスペース全体グリーン
- `mise exec -- prek run --all-files` — 全フック pass

## Risks

- `Muxer` / `Encoder` は実装者ゼロの未検証契約（モジュール doc に明記）。第一 encode 対応バックエンド実装時に契約改訂の可能性あり（`publish = false` で許容）
- `Decoder` の `drain` は spec 明示要求を超えた追加（遅延 codec の完全駆動に必要と判断）。将来 EOS を `send_packet` の特別値で表す設計へ変えたい場合の改訂余地は `publish = false` 下で許容
- `CodecId::Pcm` の粒度は PCM 系譜を単一 ID に畳んでいる。バックエンドが PCM variant を区別する必要が出た場合は variant 追加（non-breaking）で対応
- `MediaSource` は `Seek` 必須のため非シーク可能入力（pipe / network stream 等）は表現できない。必要になった時点で `Read` のみの別ポート追加を検討（現時点は spec 範囲外）

## Follow-ups

- `implement-io-symphonia-backend-poc.md`（task 2）: 本クレートの trait / 型を symphonia が実装し境界契約を実証する。`MediaSource` → symphonia `MediaSource` アダプタ、`CodecId` 写像、`Capabilities`（`Muxer` / `Encoder` 非対応）が接点
- `sync-docs-and-verify-issue-20.md`（task 3）: IR 索引（`docs/impl/ir/README.md`）への行追加・ドキュメント同期・総合検証 V-1〜V-6
- `Data` ストリーム packet の扱い（decode 直通・`Frame` への将来 variant）・gapless 情報の運搬先・`Packet` の借用/zero-copy backing は spec Deferred のまま後続設計へ
