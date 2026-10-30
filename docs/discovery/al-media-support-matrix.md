---
id: "DSC-154-al-media-matrix"
type: "discovery"
status: "final"
title: "AudioLens al-media サポートマトリクス抽出と Ruviform 境界との差分"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/154"
    - "F:/repositry/audio-products/audio-lens/crates/al-media"
  related:
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/media-observation-non-destructive-pcm.md"
    - "../adr/select-media-backends-avi-and-mp4.md"
---

# AudioLens `al-media` サポートマトリクス抽出と Ruviform 境界との差分

Issue #154 の移行ゲートは「現行 AudioLens で利用できる入力の同等機能」。
本書は `al-media`（読み取り専用で参照）が受理する入力の正本マトリクスと、
Ruviform `ruviform-media-io` 境界・各バックエンド能力との差分を記録する。

## al-media が受理する入力（正本）

`al-media` は独自の AVI demuxer/decode（`avi.rs`・`wav.rs`・`video.rs`）と
独自の MP4 demuxer（`mp4.rs`、sample table 直読）+ symphonia decode の
組合せで動作する。

| コンテナ | 音声 | 動画 | AudioLens での用途 |
| --- | --- | --- | --- |
| RIFF/WAVE | PCM int（8/16/24/32bit）・IEEE float — canonical 正規化 + native 尺度保持 | — | リファレンス/比較音声 |
| RIFF/AVI | WAVEFORMATEX `0x0001` PCM・`0x0003` float | 非圧縮 DIB（24bit BGR / 32bit BGRA、top-down/bottom-up）・MJPEG fourCC は demux のみ | キャプチャ映像 + 音声 |
| ISOBMFF (`.mp4`/`.mov`) | `sowt`/`twos`/`lpcm`/`in24` 等 PCM sample entry + `mp4a` AAC | `avc1` 等（demux facts と packet 刻印） | キャプチャ MP4 + AAC 音声 |

AVI のタイムスタンプ規則（al-media 実装由来）: stream ごとの
`dwRate/dwScale` 時計、起点 `dwStart * dwScale`、video chunk は
`dwScale` 進行、`dwSampleSize` 音声は `body_bytes * dwScale / dwSampleSize`
（剰余保持）。`LIST rec `（OpenDML マルチセグメント）・`movi` 先行配置は
未対応で明示失敗。MP4 は `mdhd` timescale を time base とし `stts` で
per-sample tick を運ぶ（VFR 保持）。`stss` sync sample・`ctts` offset・
`delay`(elst) 等の trim 事実は `Packet`/`StreamFacts` へ写像される。

## Ruviform 側の対応と差分

| 入力 | バックエンド | 経路 | 状態 |
| --- | --- | --- | --- |
| WAV/PCM | `ruviform-io-symphonia`（既存） | demux + canonical + native decode | 維持（回帰テスト済） |
| AVI PCM audio | `ruviform-io-avi`（新規） | demux + canonical + native decode | 実装済・generated fixture 検証済 |
| AVI DIB video | `ruviform-io-avi`（新規） | demux + `Rgba8` decode + drop-frame 繰返し | 実装済・検証済 |
| AVI MJPEG 等圧縮 video | `ruviform-io-avi` | demux facts のみ、`decodable: Unsupported` | 明示 unsupported |
| MP4 PCM audio | `ruviform-io-symphonia` feature `mp4` | demux + canonical + native decode | 実装済・generated fixture 検証済 |
| MP4 video track | `ruviform-io-symphonia` feature `mp4` | packet demux + `StreamFacts.video`（寸法） | facts/packet のみ（decode なし、`Unsupported`） |
| MP4 AAC audio | `ruviform-io-symphonia` feature `aac` | `CodecId::Aac` capability + decoder 構築 | demux/capability 経路のみ。実ビットストリーム decode は未検証（実 AAC fixture を要し generated fixture 範囲外） |

## #151 契約との接続確認

- PCM 元尺度・元形式: `NativeAudioFrame`（WAV/AVI/MP4 全経路で
  `SampleFormat` + `PcmTransform` 履歴保持）。
- 時刻: AVI は `Derived` 刻印、MP4/WAV は `Carried` 刻印の生 tick +
  stream `time_base`。`trim_start`/`trim_end`・`delay`/`padding` 写像。
- container/stream facts: `ContainerInfo`（`Wav`/`Avi`/`Isobmff`）、
  `StreamFacts`（`index`/`container_id`/`codec_tag`/`declared_units`/
  `video`/`decodable`）— 未宣言は `NotDeclared`、非対応は `Unsupported`、
  破損は `Malformed` を区別。
- AudioLens 固有処理（scene cut・capture anomaly・source locate）は
  移行対象外で AL#100 の責務。
