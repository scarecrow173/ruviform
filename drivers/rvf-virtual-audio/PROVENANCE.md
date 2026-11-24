# RvfVirtualAudio provenance

Issue #977 Slice B で vendoring した仮想オーディオ endpoint driver の出所記録。

## Upstream

- repo: `microsoft/Windows-driver-samples`（GitHub）
- commit: `d49bb7d72f3651952b2b6e4ad28d14fe2247eb69`
- license: Microsoft Public License (MS-PL) — `LICENSE-MS-PL.txt`（upstream ルート `LICENSE` と同一）
- 参照構成: `audio/sysvad/` の SysVAD サンプル（`TabletAudioSample` + `EndpointsCommon` + 共有部品）

## vendoring 方針

- SysVAD 複数プロジェクト構成（adapter 部品 + EndpointsCommon ライブラリ + 各 endpoint サンプル）を、render(speaker)/capture(mic in) 1 対に必要な翻訳単位だけを単一 vcxproj へ flatten した。
- 派生ファイルにも MS-PL に基づく upstream copyright 表記（`Copyright (c) Microsoft Corporation All Rights Reserved`）を保持する。

## ファイル対応

upstream パス → `drivers/rvf-virtual-audio/` 配下。`＝` は upstream と同一内容
（repo hygiene hook で行末空白・改行コードのみ正規化 — `diff -bB
--strip-trailing-cr` で意味差分 0 を検証済み）、`≠` は ruviform 変更あり、
`new` は新規。

### `audio/sysvad/` → 直下

| upstream | こちら | 差分 |
| --- | --- | --- |
| `adapter.cpp` | `adapter.cpp` | ＝ |
| `basetopo.cpp/.h` | 同名 | ＝ |
| `common.cpp/.h` | 同名 | ＝ |
| `hw.cpp/.h` | 同名 | ＝ |
| `kshelper.cpp/.h` | 同名 | ＝ |
| `savedata.cpp/.h` | 同名 | ＝ |
| `ToneGenerator.cpp/.h` | 同名 | ＝ |
| `sysvad.h` | `sysvad.h` | ＝ |
| `SysVadShared.h` | 同名 | ＝ |
| `UnittestData.h` | 同名 | ＝ |
| `ihvprivatepropertyset.h` | 同名 | ＝ |
| `ContosoKeywordDetector.h` | 同名 | ＝ |
| `TabletAudioSample/minipairs.h` | `minipairs.h` | ≠ speaker+mic in のみ登録 |
| `TabletAudioSample/micintopo.cpp/.h` | 同名 | ＝ |
| `TabletAudioSample/micintoptable.h` | `micintoptable.h` | ≠ pin-name GUID を独自 GUID へ |
| `TabletAudioSample/micinwavtable.h` | 同名 | ＝ |

### `audio/sysvad/EndpointsCommon/` → `EndpointsCommon/`

| upstream | こちら | 差分 |
| --- | --- | --- |
| `AudioModule0/1/2.h`, `AudioModuleHelper.cpp/.h` | 同名 | ＝ |
| `MiniportAudioEngineNode.cpp`, `MiniportStreamAudioEngineNode.cpp` | 同名 | ＝ |
| `NewDelete.cpp/.h` | 同名 | ＝ |
| `mintopo.cpp/.h` | 同名 | ＝ |
| `minwavert.cpp/.h`, `minwavertstream.cpp/.h` | 同名 | ＝ |
| `simple.h` | 同名 | ＝ |
| `speakertopo.cpp/.h` | 同名 | ＝ |
| `speakertoptable.h` | `speakertoptable.h` | ≠ pin-name GUID を独自 GUID へ |
| `speakerwavtable.h` | 同名 | ＝ |

### 除外した upstream ファイル

- `TabletAudioSample/` の他 endpoint（speakerhp/hdmi/spdif/micarray1-3）… `minipairs.h` の書き換えで登録対象外にしたため未使用。
- `sysvad/` 直下の `BthhfpDevice`, `UsbHsDevice`, `A2dpHpDevice`, `SimpleAudioSource`, `KeywordDetectorAdapter` 等 … 参照は全て `SYSVAD_BTH_BYPASS` / `SYSVAD_USB_SIDEBAND` / `SYSVAD_A2DP_SIDEBAND` ifdef 下で compile-time OFF。関連 `*minipairs.h`/`*wavtable.h`/`SidebandAudio*.h` も同様。
- `APO/`, `Package/`, `.inx` 群 … APO は別ドライバ、INF は手書き派生版で代替。

### 新規ファイル

- `RvfVirtualAudio.inf` — `ComponentizedAudioSample.inx` 構成を単一 INF へ簡約（hwid `ROOT\RvfVirtualAudio`、x64 のみ、speaker/micin の 4 interface pair + KMDF service + MediaCategories 登録）。
- `RvfVirtualAudio.vcxproj` — 単一 KMDF driver project（`KmdfVersionNumber=1.33`、INF `KmdfLibraryVersion=1.33` と対）。
- `RvfVirtualAudio.rc` — `TabletAudioSample.rc` 派生の version resource。

## 変更内容（≠ ファイル）

- `minipairs.h`: 登録 endpoint を `SpeakerMiniports` + `MicInMiniports` のみに削減（render: `TopologySpeaker`/`WaveSpeaker`、capture: `TopologyMicIn`/`WaveMicIn`）。
- `EndpointsCommon/speakertoptable.h`: `KSPIN_TOPO_LINEOUT_DEST` bridge pin の `Name` に独自 GUID `RVF_CABLE_INPUT_NAME = {131373ee-e400-4a68-ab5e-9ded0544f2b4}` を設定。INF `MediaCategories` で "Ruviform Cable Input" に結び付く。
- `micintoptable.h`: `MICIN_CUSTOM_NAME` を独自 GUID `RVF_CABLE_OUTPUT_NAME = {46eab9d5-1e22-45f8-a744-1cf7561dc20a}` に置換。INF `MediaCategories` で "Ruviform Cable Output" に結び付く。

## endpoint 名の根拠

MMDevice endpoint 表示名は topology filter の bridge pin の pin-name GUID を `HKR\MediaCategories\{guid}\Name` で引いた文字列になる（VB-CABLE 等と同型）。render 側（アプリが書き込む側）を "Ruviform Cable Input"、capture 側（アプリが読む側）を "Ruviform Cable Output" とする — `endpoint_probe` の `VIRTUAL_NAME_HINTS`（"cable input"/"cable output"）と `DIRECTION_TOKENS`（input/output 除去後 stem = "ruviform cable"）に一致する命名。
