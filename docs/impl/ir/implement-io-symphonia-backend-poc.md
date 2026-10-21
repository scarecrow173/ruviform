---
id: "IMPL-034K7dWqYmSaPhOnIaPc02"
type: "impl"
status: "completed"
title: "Implement io-symphonia backend PoC"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/20"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-symphonia/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/demuxer.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/decoder.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/map.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/source.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/tests/wav_decode_poc.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/tasks/implement-io-symphonia-backend-poc.md"
      - type: "file"
        path: "crates/ruviform-media-io/src/capability.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/encode.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/error.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/mux.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/implement-io-symphonia-backend-poc.md"
  implemented-by: []
  depends-on:
    - "../../plans/issue-20-i-o-poc.md"
    - "implement-media-io-abstraction.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/issue-20-i-o-poc.md"
    - "../../specs/implement-media-io-abstraction-and-first-backend-poc.md"
    - "../../adr/media-io-backend-abstraction.md"
    - "../../adr/licensing-and-distribution-policy.md"
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
  references:
    - "https://github.com/pdeljanov/Symphonia"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement io-symphonia backend PoC

## Summary

task `TASK-034Jgi3OFauA1SWRPdvnnb`（plan `PLAN-034JfKRCFCu8oJr5iCe6AV` Phase 2・Issue #20 タスク2）として、symphonia 0.6（default features のみ）を用いる第一バックエンド PoC `ruviform-io-symphonia` を分離クレート + opt-in 依存として新設した。demux + decode のみを提供し、`Muxer` / `Encoder` 非提供を能力交渉「非対応」表現（照会 `capabilities()` と要求 `make_muxer()` / `make_encoder()` の両機構で `ErrorKind::Unsupported`）の実例とした。in-memory 合成 WAV の demux → decode → `AudioFrame` 変換を end-to-end 検証し、公開面に symphonia 型・エラー・feature 名を露出しない型純度を維持した。

## Context

- 実施タスク: [../../tasks/implement-io-symphonia-backend-poc.md](../../tasks/implement-io-symphonia-backend-poc.md)（`TASK-034Jgi3OFauA1SWRPdvnnb`）
- 上流: [plan `PLAN-034JfKRCFCu8oJr5iCe6AV` Phase 2](../../plans/issue-20-i-o-poc.md)、[spec `SPEC-034JcaMXubILS7j1FX5qob`](../../specs/implement-media-io-abstraction-and-first-backend-poc.md)、[media-io ADR](../../adr/media-io-backend-abstraction.md)、[licensing ADR](../../adr/licensing-and-distribution-policy.md)
- 前提: [implement-media-io-abstraction.md](implement-media-io-abstraction.md)（`IMPL-034JxNThv4gsW2IpbkYmK4`）が確定した `ruviform-media-io` の trait・型・エラーモデル
- 正本要件: spec Requirements（symphonia PoC・型純度・能力交渉両機構・エラー正規化）・AC1/AC4/AC5/AC6/AC9

## Implementation

### Changes

- `crates/ruviform-io-symphonia/Cargo.toml` — workspace 継承 + `[lints] workspace = true` + `ruviform-media-io` / `ruviform-media-core` / `ruviform-audio-core` への `path` 依存 + `symphonia = "0.6"`（default features のみ）。依存追加理由・MPL-2.0（弱コピーレフト・ファイル単位）・MSRV 1.85 整合・feature 制限を Cargo.toml コメントに記録（暫定規約の依存追加規約・licensing ADR）。ルート `Cargo.toml` の `members` へ追加・コメント同期
- `src/source.rs` — `SourceAdapter`（media-io `MediaSource` → バックエンド `MediaSource` trait のアダプタ。`Sync` ギャップは `Mutex` + `get_mut` の非ロック経路で閉じる。crate 内部・公開非露出）。レビュー修正 wave（[T3 IR](sync-docs-and-verify-issue-20.md)）で `is_seekable` / `byte_len` を構築時 `Seek` probe のキャッシュ値から返す方式へ変更 — 真に非シーク可能なソースは `SeekError::Unseekable` → `Unsupported` に正規化され、Ogg の末尾走査・二分シークに必要な実長も供給される
- `src/error.rs` — `map_error`: backend `Error` → `ErrorKind` 正規化写像表（`IoError(UnexpectedEof)`→`UnexpectedEos`、`IoError`→`Io`、`DecodeError`/`LimitError`/`ResetRequired`→`InvalidData`、`SeekError::{Unseekable,ForwardOnly}`→`Unsupported`、`SeekError` 他→`InvalidValue`、`Unsupported`→`Unsupported`）+ 原本を型消去 `source` 診断へ封入
- `src/map.rs` — backend → 境界値型の純粋写像（`to_time_base` / `stream_kind` / `codec_id`（音声: PCM 系譜→`Pcm`、Vorbis/Opus/Mp3/Aac/Flac/Alac、動画: H264/Hevc/Vp9/Av1、他→`Other`）/ `audio_format` / `channel_layout`）。写像不能は `CodecFormat::Data` / `None` で正直に降格
- `src/demuxer.rs` — `SymphoniaDemuxer`（`FormatReader` wrap。`streams` は probe 時スナップショット、`read_packet` は `Packet` 変換＋正常 EOS=`Ok(None)`、`seek` は時刻指定要求を `SeekMode::Accurate` へ）+ `make_decoder(StreamId)`（codec 非対応・非音声は `ErrorKind::Unsupported`/`InvalidValue`）
- `src/decoder.rs` — `SymphoniaDecoder`（backend `AudioDecoder` wrap。`send_packet` で eager decode → planar f32 `AudioBuffer` → `AudioFrame`（PTS/duration 写像・symphonia timebase → `MediaTime`/`TimeBase`）を pending queue へ。`receive_frame` は queue 排出・`flush` は decoder reset + queue/TS anchor 破棄）
- `src/lib.rs` — crate doc（PoC・型純度・非 RT・backend feature scope）+ `open_demuxer` / `make_decoder` 経由の構築入口 + `capabilities` / `make_muxer` / `make_encoder`（能力交渉の両機構）+ `SymphoniaDemuxer` / `SymphoniaDecoder` の公開（自クレート型・外部型の `pub use` 再エクスポートなし）
- `tests/wav_decode_poc.rs` — in-memory RIFF/WAVE 合成（RIFF ヘッダをテスト内で組み立て・LFS 不使用）→ demux → decode → `AudioFrame` の end-to-end。フレーム数・PTS 単調進行・planar f32 サンプル値完全一致を検証
- `crates/ruviform-media-io/src/{capability,encode,error,mux}.rs` — doc コメント中のバックエンド名（symphonia / FFmpeg）言及を汎用表現へ修正（AC1 監査 `grep -rn "symphonia" crates/ruviform-media-io/src` を 0 件に。抽象層が特定バックエンド名を名指ししない純度維持）

### Decisions During Implementation

- **構築入口は free function**: `open_demuxer(MediaSource)` / `SymphoniaDemuxer::make_decoder(StreamId)`。境界 trait にコンストラクタ要件を持たせないタスク 1 決定を踏襲し、codec パラメータは `Track` が保持する内部経路で受け渡し（`StreamInfo` はバックエンド非依存データのみ運搬）
- **decoder は send/receive ポンプの内部 eager 化**: backend は 1 packet → 全 frame を同期 decode するため、`send_packet` で decode して小さい出力 queue に保持し `receive_frame` で排出。`Ok(None)` = 「さらに入力が必要」の pump 契約を偽装せず実装
- **PTS/duration 写像**: frame PTS は packet PTS の stream timebase 変換（`Rounding::NearestEven`）、PTS 欠落時は `next_ts` anchor で補外。duration は decoded frames × sample-rate base を stream base へ変換し、表現不能は `None` 降格（フィールドが `Option` の契約どおり）
- **能力照会は under-claim**: `capabilities()` は `Features::DEMUX|DECODE` + registry 既知名の decode 可能 codec（`Pcm`/`Flac`/`Vorbis`）のみ報告。decode は可能でも `CodecId` レジストリに名前を持たない codec（ADPCM 等）は `Other` 以上に主張しない — 嘘のない能力表現を優先
- **packet フラグは `NONE`**: バックエンドの `Packet` が keyframe/corruption 属性を提供しないため、捏造せず `PacketFlags::NONE` を付す（doc 明記）
- **seek は request-only**: `SeekTo::Time{track_id:None}` + `Accurate` へ写像し、実着地点は境界契約どおり返さない。時刻変換は i128 nanoseconds 経由で任意 `TimeBase` を受理
- **非音声・未識別の降格規則**: 写像不能な codec/format は `CodecFormat::Data`・`CodecId::Other`・`None` duration で全射を保ち、列挙を失敗させない（hostile input への防御は分類済み `Error` 側の責務）

## Related Experiments

### Adopted

なし（実験ログ運用なし・直接実装）

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `mise exec -- cargo test -p ruviform-io-symphonia` — 11 unit + 7 e2e（`tests/wav_decode_poc.rs`）+ 5 doctest、全 pass（AC5/AC6）
- `mise exec -- cargo clippy -p ruviform-io-symphonia --all-targets -- -D warnings` — 0 warnings
- `grep -rni "symphonia" crates/ruviform-media-io/src` — 0 件（doc 言及の整理後。AC1）
- `grep -rn "pub use" crates/ruviform-io-symphonia/src` — `SymphoniaDecoder` / `SymphoniaDemuxer` のみ（自クレート型・外部型再エクスポートなし。AC1 型純度）
- `grep -rn "unimplemented!\|panic!" crates/ruviform-io-symphonia/src` — 0 件（panic 経路なし。AC6）
- `grep -rn "unsafe" crates/ruviform-io-symphonia` — 0 件（AC8 関連）
- `mise exec -- cargo tree -p ruviform-media-io` — 依存辺は domain 3 クレートのみ（symphonia / `ruviform-io-*` 辺なし。AC4）
- `mise run fmt-check` / `check` / `lint` / `test` — ワークスペース全体グリーン
- `grep -rni "symphonia" crates/ruviform-io-symphonia/src/lib.rs` — `use symphonia::…` の private import と `SymphoniaDemuxer` / `SymphoniaDecoder` の自クレート型名のみ（公開署名へのバックエンド型・feature 名の露出なし）

## Risks

- `Muxer` / `Encoder` は実装者ゼロの未検証契約のまま（spec Deferred。`publish = false` 下で第一 encode 対応バックエンド実装 Issue で実証・改訂可）
- `next_ts` anchor による PTS 欠落補外は単一ストリーム順序を前提。異常な PTS 系列での補外誤差は engines 責務の再同期で吸収される設計
- チェーン Ogg（連結物理ストリーム）は第一リンク末尾で `ResetRequired` → `InvalidData` に正規化され打ち切り。`streams()` スナップショットモデルとの構造的衝突のため PoC では制限として crate doc に明記（レビュー候補 B5・後続課題）
- 能力照会の under-claim 方針は codec 採否の正式判断を含まないため、decode 可能だが未広告の codec（ADPCM 等）の扱いは後続の codec 採否 Issue で再評価が必要

## Follow-ups

- 個別 codec/container の正式採否・対応範囲（spec Deferred → 後続 Issue）
- Mux/Encode 対応バックエンド実装（`Muxer` / `Encoder` 契約の実証）
- 動画 decode バックエンド（`Frame::Video` 出力経路の実証）
- `Packet` の借用/zero-copy backing 評価（非 RT 層の後続設計・Issue #26 系）
- gapless 情報（encoder delay / padding trim）の運搬先決定
