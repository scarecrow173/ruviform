# ruviform-media-reader

Ruviform メディア基盤の汎用 Reader・Decoder/Prefetch・RT 消費境界を担うクレート（Issue #197）。

## 責務

- `SourceReader` — `Demuxer`/`Decoder` 対を駆動し、source frame（デコード済みサンプル index）順に
  PCM を取り出す逐次リーダー。`seek_source_frame` は demuxer seek → decoder flush →
  decode-discard によるフレーム精度 seek を構成する（boundary は seek 要求ポートのみ提供）。
- `SourceMetadata` — Container/Backend/Stream facts・canonical `AudioFormat`・`frame_count`・
  `seekable` の capability 投影（classified `Fact`、捏造しない）。
- `FrameMap` / `SourceFrame` / `EngineFrame` / `RequestedRange` — caller が渡す
  EngineFrame→SourceFrame 写像の入力契約（clip offset・placement・rate ratio。
  loop/stretch 意味論は caller 側）。
- `Prefetch` — 非 RT producer（`PrefetchProducer` pump / spawn thread）→ 固定 `PcmBlock`
  pool → 2 本の bounded SPSC ring（data / free-return）→ generation 付き `PrefetchItem`
  列（`Block`/`Boundary`/`Fault`/`End`）→ RT `PrefetchConsumer::fill`。
  - `fill` の `expected_first` は source frame 基準の *ストリーム位置 assertion* —
    caller が想定する `next_frame` を宣言し、不一致なら何も消費せず `mismatch` を
    返す。engine frame の期待値ではなく、engine 側の写像は caller 責務。
  - `pump()` は呼び出しごとに queued command を全て処理したうえで、sequenced な
    item を高々 1 つだけ進める（`Pushed`）。`Backpressured` は ring 空き待ちの
    retry シグナルで、marker は producer `pending` に残る — manual driver は
    `Idle`/`Ended` に到達するまで `pump()` を反復する契約。
  - `PrefetchHandle::seek` は play-through を既定とし、`seek_discard_pending` が
    opt-in の producer pending purge を追加する。consumer 側の `discard_pending` は
    RT-safe な有界 drain で、次の `Boundary` で generation を adopt して停止する。
  - `PrefetchHandle::reopen` は frame 0 anchor を維持し、`reopen_at` が
    reopen→seek の fused 版を追加する（boundary は実際に landed した位置を報告）。
  - 各 generation の `Boundary` は常にその generation 先頭 item として emit される。
    reject された seek / 失敗した reopen でも `Boundary` が先行するため、全ての
    `Fault`/`End` は adopt 後に観測され stale drop にならない。reopen 失敗の
    dead generation は `Boundary{gen,0}` → `Fault{gen,of_kind}` → `End{gen}` で
    observably に終端し、`eof` を返す。`End` は合成時点で commit され、ring full で
    `pending` へ requeue されても generation あたり 1 回だけ emit される。
- Fault 契約 — `SourceFault`（truncated/corrupt/io/invalid-sample/unsupported/unseekable）と
  `FillReport`（written/underrun/faulted/eof/mismatch）で、真の無音・short-read・corrupt・
  EOF・prefetch underrun・invalid sample・unsupported layout を識別する。欠落を
  「verified content」として報告しない。zero-fill は宣言済みの有界 fallback であり、
  必ず report/telemetry に記録される。

## 層・許可依存

- 層: Domain foundations（設計ベースラインの foundations 段。media-io と engine/adapter の間）
- 許可依存: `ruviform-media-io`・`ruviform-media-core`・`ruviform-audio-core`・`ruviform-rt` のみ。
  公開 API には backend 型・`File`/`Path`・provider 概念を露出しない（provider interchange は
  caller 側の `Box<dyn MediaSource>`/`Box<dyn Demuxer>`/open closure）。
- dev 依存: `ruviform-io-symphonia`（テスト fixture）、`assert_no_alloc`（RT 検証）。

## RT 契約

mixed-RT クレート — API ごとの `**RT contract:**` ラベルが正本。

- **non-RT only**: `SourceReader` 全メソッド、`PrefetchHandle`、`PrefetchProducer`、
  `Prefetch::manual`/`spawn`、`PrefetchConsumer::close`、metadata — alloc・I/O・unbounded codec work を行い得る。
- **RT-suitable**: `PrefetchConsumer::fill`/`discard_pending`/`position`/`generation` — 事前確保された
  pool と bounded ring の上で no-alloc・no-lock・no-blocking・no-OS-I/O。

## 関連

- Issue: https://github.com/scarecrow173/ruviform/issues/197
- 設計: docs/designs/issue-197-media-reader-prefetch-rt-consumer.md
- ADR: docs/adr/issue-197-media-reader-boundary-additions.md
- spec: docs/specs/issue-197-media-reader-prefetch-rt-consumer.md
