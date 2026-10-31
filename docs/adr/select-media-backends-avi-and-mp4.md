---
id: "ADR-154-media-backends"
type: "adr"
status: "accepted"
title: "AVI/MP4/AAC 入力バックエンドの配置: AVI は専用 pure-Rust クレート、MP4/AAC は symphonia opt-in feature"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/154"
    - "https://github.com/scarecrow173/ruviform/issues/151"
  derives-from:
    - "../discovery/al-media-support-matrix.md"
    - "media-io-backend-abstraction.md"
    - "media-observation-non-destructive-pcm.md"
    - "licensing-and-distribution-policy.md"
  related:
    - "platform-and-toolchain-targets.md"
---

# AVI/MP4/AAC 入力バックエンドの配置

## Context and Problem Statement

Issue #154 は AudioLens が現に消費する入力（WAV/PCM、AVI+PCM+DIB、
MP4+PCM/AAC+video track）について、Ruviform 側に demux/decode の
実動経路を提供する。決定が必要なのは各入力を「どのバックエンド単位に、
どの feature・依存範囲で」配置するか — 既存 `ruviform-io-symphonia` の
拡張か、新 adapter クレートか。

## Decision Drivers

- ライセンス ADR: 特許係属 codec（AAC）は配布者判断で切替可能な opt-in
  とし、default feature はロイヤリティフリー集合を維持する。
- 抽象境界 ADR: バックエンドは opt-in 分離クレート。境界 crate への
  codec 固有ロジック混入を避ける。
- al-media マトリクス正本: AVI の timestamp 派生規則・DIB 変換・
  `rec `/OpenDML 制限は AudioLens 実装と同一であること。
- unsafe 原則禁止・pure-Rust 優先・外部メディアフレームワーク
  （FFmpeg 等）の追加は依存木・ライセンス・MSRV 影響が大きい。

## Considered Options

### AVI

- **A. symphonia の RIFF demuxer で AVI を読む** — symphonia-format-riff
  は WAV 系であり AVI の `strl`/`movi` 複数 stream chunk を扱わない。
  video decode も存在しない。棄却。
- **B. 新規 pure-Rust クレート `ruviform-io-avi`（採用）** — RIFF/AVI
  demux + WAVEFORMATEX PCM + DIB decode を自前実装。外部依存ゼロ、
  al-media の timestamp 規則を忠実に再現でき、unsafe 不使用。
  MJPEG 等の圧縮 fourCC は demux facts のみで `Unsupported` を明示。
- C. FFmpeg 系バックエンド — ライセンス/配布/依存木コストが本 Issue の
  ゲート（現行入力同等）に対して過大。将来候補として設計上の余地は残る。

### MP4 / AAC

- **D. 新規 pure-Rust MP4 demuxer クレート** — al-media の MP4 reader を
  移植すると二重所有になる。sample table 解析は isomp4 が既に提供する。
  棄却。
- **E. `ruviform-io-symphonia` の opt-in feature（採用）** —
  `mp4 = symphonia/isomp4`（container）、`aac = mp4 + symphonia/aac`
  （patent codec、mp4 単体では有効化しない）。default ビルドは不変で
  feature なしの MP4 probe は `ErrorKind::Unsupported` を返す。
  container と codec の feature を分離し、ライセンス ADR の
  「codec/container/機能フラグ分離」をそのまま Cargo feature に写像した。

## Decision

- **AVI** → 新規クレート `ruviform-io-avi`（Option B）。
- **MP4** → `ruviform-io-symphonia` feature `mp4`（Option E）。
- **AAC** → `ruviform-io-symphonia` feature `aac`（`mp4` 含意、単独不可）。
- 動画 track（MP4/AVI 圧縮 codec）は packet demux と宣言 facts
  （`StreamFacts.video`、time base、trim）まで。decode は持たず
  `decodable: Fact::Unsupported` と decoder 構築時 `ErrorKind::Unsupported`。
- `CodecId` に `Uncompressed`（生 video、AVI `BI_RGB` DIB）と `Mjpeg`
  （AVI `MJPG` fourCC の正直な命名）を追加。

## Consequences

- default 依存木は不変（symphonia default features のまま）。`aac` を
  有効化する配布物は特許ライセンス判断を要する — README と本 ADR で
  配布者向けに明記。
- MP4 video track は「I/O 成功 + decode unsupported」の中間状態を
  #151 の `Fact` 契約で表現でき、AudioLens の video-frame 用途は
  AVI 経路でカバーされる。
- AAC 実ビットストリーム decode は generated fixture では検証不可
  （有効な AAC/ADTS サンプル生成はスコープ外）。capability/demux 経路と
  `CodecId::Aac` 申告は検証済み、実 decode 検証は AL#100 側の
  fixture 移行時に行う。
- MSRV・OS 制約の追加なし（workspace `rust-version = "1.89"`、
  pure-Rust のみ・OS 非依存）。FFmpeg 導入時に必要になる再検討は
  将来の ADR で行う。
