# RvfVirtualAudio provenance

Issue #977 Slice B で vendoring した仮想オーディオ endpoint driver、および
Issue #978 Slice C で vendoring した DMus MIDI loopback miniport の出所記録。

## Upstream

- repo: `microsoft/Windows-driver-samples`（GitHub）
- commit: `d49bb7d72f3651952b2b6e4ad28d14fe2247eb69`
- license: Microsoft Public License (MS-PL) — `LICENSE-MS-PL.txt`（upstream ルート `LICENSE` と同一）
- 参照構成: `audio/sysvad/` の SysVAD サンプル（`TabletAudioSample` + `EndpointsCommon` + 共有部品）

## Upstream — MIDI miniport（Issue #978 Slice C 追加分）

- repo: `uri247/wdk81`（GitHub — WDK 8.1 MSDN samples の verbatim mirror、
  single-commit repo。DMusUART は `microsoft/Windows-driver-samples` の
  全履歴にも `microsoft/Windows-classic-samples` にも存在しないため、
  公式 repo からの vendoring 経路は存在せず本 mirror を選定した）
- commit: `5a28cfcafb45fcc7fcbb5bb1975cddfd7202f98f`
- 参照構成: `DirectMusic UART Driver Sample/C++/` の DMusUART サンプル
  （MPU-401 UART hardware miniport — `private.h` / `miniport.cpp` / `MPU.cpp`）
- license: **Microsoft Limited Public License (MS-LPL) v1.1** —
  `LICENSE-MS-LPL.txt`（upstream ルート `license.rtf` の text 化。MS-PL では
  **ない** — §3(F) に platform limitation（Microsoft Windows OS / runtime /
  application platform 上で動く software に限り license が及ぶ）があるが、
  kernel driver は Windows でのみ動くため本用途では実質的制約にならない。
  §3(D) に従い license 全文を同 directory に同梱する）
- upstream copyright 表記（`Copyright (c) 1997-2000 Microsoft Corporation`）は
  派生ファイル先頭に保持（§3(C) の attribution notice 保持義務に対応）

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
| `adapter.cpp` | `adapter.cpp` | ≠ `InstallMidiFilter`（DMus subdevice 登録）追加・`maxObjects` に `g_MaxMidiFilters` 加算 |
| `basetopo.cpp/.h` | 同名 | ＝ |
| `common.cpp/.h` | 同名 | ≠ `IAdapterCommon` に loopback registry API 追加（`RvfLoopback*` メソッド + `RVF_LOOPBACK_RENDER_SOURCE` 保持） |
| `hw.cpp/.h` | 同名 | ＝ |
| `kshelper.cpp/.h` | 同名 | ＝ |
| `savedata.cpp/.h` | 同名 | ＝ |
| `ToneGenerator.cpp/.h` | 同名 | ＝ |
| `sysvad.h` | `sysvad.h` | ＝ |
| `SysVadShared.h` | 同名 | ＝ |
| `UnittestData.h` | 同名 | ＝ |
| `ihvprivatepropertyset.h` | 同名 | ＝ |
| `ContosoKeywordDetector.h` | 同名 | ＝ |
| `TabletAudioSample/minipairs.h` | `minipairs.h` | ≠ speaker+mic in のみ登録 + `g_MaxMidiFilters` 追加 |
| `TabletAudioSample/micintopo.cpp/.h` | 同名 | ＝ |
| `TabletAudioSample/micintoptable.h` | `micintoptable.h` | ≠ pin-name GUID を独自 GUID へ + Category を `KSNODETYPE_MICROPHONE`→`KSNODETYPE_LINE_CONNECTOR` + jack channel mask stereo 化 |
| `TabletAudioSample/micinwavtable.h` | 同名 | ≠ stereo capture format/range 追加 + default mode stereo 化 |

### `DirectMusic UART Driver Sample/C++/` → 直下（Issue #978 Slice C）

| upstream | こちら | 差分 |
| --- | --- | --- |
| `private.h` | `RvfMidi.h` | ≠ class 名 `CMiniportDMusUART*`→`CMiniportRvfMidi*`、UART register 定義・portBase/IRQ 状態・ISR 宣言を削除、FIFO 用 `KSPIN_LOCK` 追加、pin-name GUID を独自 `RVF_MIDI_LOOPBACK_NAME` へ、pool tag 変更、factory `CreateMiniportRvfMidi` 宣言追加、pin-name GUID は initguid 機構を通さず extern 宣言 + Miniport TU で単一定義 |
| `miniport.cpp` | `RvfMidiMiniport.cpp` | ≠ 下記「変更内容（MIDI miniport）」参照 |
| `MPU.cpp` | `RvfMidiMpu.cpp` | ≠ 同上 — UART register I/O を software loopback へ置換 |
| `DMusUART.vcxproj` | — | 未 vendor（`RvfVirtualAudio.vcxproj` に 2 翻訳単位として flatten） |

### `audio/sysvad/EndpointsCommon/` → `EndpointsCommon/`

| upstream | こちら | 差分 |
| --- | --- | --- |
| `AudioModule0/1/2.h`, `AudioModuleHelper.cpp/.h` | 同名 | ＝ |
| `MiniportAudioEngineNode.cpp` | 同名 | ≠ sideband 呼出を `SYSVAD_BTH_BYPASS`/`SYSVAD_USB_SIDEBAND` guard 化 |
| `MiniportStreamAudioEngineNode.cpp` | 同名 | ＝ |
| `NewDelete.cpp/.h` | 同名 | ＝ |
| `mintopo.cpp/.h` | 同名 | ＝ |
| `minwavert.cpp/.h` | 同名 | ＝ |
| `minwavertstream.cpp/.h` | 同名 | ≠ capture 充填を ToneGenerator → loopback 経路へ置換 + render stream の register/unregister/run-state/play-cursor publish + STOP 時の時間基準・carry-forward リセット + Issue #1009 static registry xrun scenario injection |
| `simple.h` | 同名 | ＝ |
| `speakertopo.cpp/.h` | 同名 | ＝ |
| `speakertoptable.h` | `speakertoptable.h` | ≠ pin-name GUID を独自 GUID へ + Category を `KSNODETYPE_SPEAKER`→`KSNODETYPE_LINE_CONNECTOR` |
| `speakerwavtable.h` | 同名 | ＝ |

### 除外した upstream ファイル

- `TabletAudioSample/` の他 endpoint（speakerhp/hdmi/spdif/micarray1-3）… `minipairs.h` の書き換えで登録対象外にしたため未使用。
- `sysvad/` 直下の `BthhfpDevice`, `UsbHsDevice`, `A2dpHpDevice`, `SimpleAudioSource`, `KeywordDetectorAdapter` 等 … 参照は全て `SYSVAD_BTH_BYPASS` / `SYSVAD_USB_SIDEBAND` / `SYSVAD_A2DP_SIDEBAND` ifdef 下で compile-time OFF。関連 `*minipairs.h`/`*wavtable.h`/`SidebandAudio*.h` も同様。
- `APO/`, `Package/`, `.inx` 群 … APO は別ドライバ、INF は手書き派生版で代替。

### 新規ファイル

- `RvfVirtualAudio.inf` — `ComponentizedAudioSample.inx` 構成を単一 INF へ簡約（hwid `ROOT\RvfVirtualAudio`、x64 のみ、speaker/micin の 4 interface pair + `MidiLoopback` DMus interface + KMDF service + MediaCategories 登録）。
- `RvfLoopback.cpp/.h` — render→capture kernel loopback（"virtual cable"）のデータ経路。`CAdapterCommon` 内 registry（`RVF_LOOPBACK_RENDER_SOURCE`）に render stream が DMA buffer・format・run 状態・play cursor を登録し、capture stream の充填（`WriteBytes` 経由）が整数演算のみの変換（PCM16/24/32・f32・downmix・最近傍 rate map）で render buffer を読む。upstream に存在しない新規機構。読み位置は registry 内の連続読みカーソル（`ReadBytePos`/`ReadPosValid`）— 初回 fill で live play cursor へ anchor し、以後は消費 frame 分だけ前進（drift が半 buffer 超の時のみ再 anchor）。fill 毎の re-anchor は両 stream の position クロック jitter を fill 境界の dup/skip として観測させたため（attempt-15 実測: marker 間隔 +10.4%・440Hz share 0.085）。
- `RvfVirtualAudio.vcxproj` — 単一 KMDF driver project（`KmdfVersionNumber=1.33`、INF `KmdfLibraryVersion=1.33` と対）。ClCompile define は upstream `EndpointsCommon.vcxproj`（StaticLibrary）から `_USE_WAVERT_;_NEW_DELETE_OPERATORS_` を引き継ぐ（2026-11-24 attempt-2 実測修正: `_NEW_DELETE_OPERATORS_` 無しでは `NewDelete.h` 側の宣言は PortCls header に残り 3 引数 `operator new` が LNK2019 になる。`SYSVAD_BTH_BYPASS`/`SYSVAD_USB_SIDEBAND` は sideband 実装ファイルを vendor しないため意図的に未定義 — 呼出側は `#if defined(...)` guard で compile 対象外）。
- `RvfVirtualAudio.rc` — `TabletAudioSample.rc` 派生の version resource。
- `LICENSE-MS-LPL.txt` — wdk81 mirror ルート `license.rtf` の plain-text 化
  （MS-LPL §3(D) の license 全文同梱義務に対応）。

## 変更内容（≠ ファイル）

- `minipairs.h`: 登録 endpoint を `SpeakerMiniports` + `MicInMiniports` のみに削減（render: `TopologySpeaker`/`WaveSpeaker`、capture: `TopologyMicIn`/`WaveMicIn`）。
- `EndpointsCommon/speakertoptable.h`: `KSPIN_TOPO_LINEOUT_DEST` bridge pin の `Name` に独自 GUID `RVF_CABLE_INPUT_NAME = {131373ee-e400-4a68-ab5e-9ded0544f2b4}` を設定し、`Category` を `KSNODETYPE_SPEAKER`→`KSNODETYPE_LINE_CONNECTOR` に変更（2026-11-24 attempt-8 実測: KSNODETYPE_SPEAKER 系 Category の endpoint では Windows が pin-name を無視して "Speakers" をハードコードする — OSR/実測。LINE_CONNECTOR 化で `MediaCategories` 経由の "Ruviform Cable Input" が表示される）。仮想 cable の形態としても speaker より line-connector が自然。
- `micintoptable.h`: `MICIN_CUSTOM_NAME` を独自 GUID `RVF_CABLE_OUTPUT_NAME = {46eab9d5-1e22-45f8-a744-1cf7561dc20a}` に置換し（INF `MediaCategories` で "Ruviform Cable Output" に結び付く）、capture bridge pin の `Category` を `KSNODETYPE_MICROPHONE`→`KSNODETYPE_LINE_CONNECTOR` に変更（仮想 cable の形態として正しい — VB-CABLE 等と同型。2026-11-24 attempt-10 実測: capture `IAudioClient::Initialize` が `E_ACCESSDENIED`=`permission_denied` — mic device capability の consent gate による拒否。attempt-11 実測で LINE_CONNECTOR 化は同 gate を解除しないことが確定したため、gate 対策は workflow 側の DeviceAccess consent provisioning に分離 — driver 側の変更は semantic 正当性のみを根拠とする）。
- `micinwavtable.h` / `micintoptable.h`（Issue #1029 stereo capture 変更）: capture endpoint の max channels を 1→2 へ引き上げ、既存 mono 16-bit PCM format 群を保持したまま 8 kHz–48 kHz の stereo 16-bit PCM format 群を追加。`RAW`/`DEFAULT` mode の default format は stereo 48 kHz、speech/communications/far-field mode の default も対応する stereo rate へ変更し、streaming pin data range は channel count exact-match を前提に mono/stereo の 2 range へ分離。topology jack channel mask も `KSAUDIO_SPEAKER_STEREO` へ変更。loopback copy は既存の channel-mapped conversion を使い、mono capture 明示時だけ arithmetic mean downmix になる従来契約を維持する。
- `EndpointsCommon/MiniportAudioEngineNode.cpp`（2026-11-24 attempt-1 実測修正）: upstream は `EndpointsCommon` を `SYSVAD_BTH_BYPASS;SYSVAD_USB_SIDEBAND` 定義の StaticLibrary として build するため本ファイル内の sideband 呼出（`IsSidebandDevice()`/`m_pSidebandDevice`）が未ガードのまま compile する。本ツリーは macro 未定義で単一 vcxproj build するため、他 upstream ファイル（`mintopo.cpp` 等）と同じ `#if defined(SYSVAD_BTH_BYPASS) || defined(SYSVAD_USB_SIDEBAND)` guard を `if (...) {...} else` 8 箇所に適用 — sideband 無効時は else 側（非 sideband path）のみ compile される upstream 互換構造。

- `common.cpp/.h`（2026-11-24 Slice B stage-2 変更）: `IAdapterCommon` に `RvfLoopbackRegisterRender`/`RvfLoopbackUnregisterRender`/`RvfLoopbackRenderRunning`/`RvfLoopbackRenderPosition`/`RvfLoopbackCaptureWrite` を追加。`CAdapterCommon` は両 wave miniport に共有される adapter 単位オブジェクトで、render stream の DMA buffer 参照・format・RUN 状態・play cursor を保持する registry（`m_LoopbackRender` + `m_LoopbackLock`）の置き場所として選定。
- `EndpointsCommon/minwavertstream.cpp/.h`（2026-11-24 Slice B stage-2 変更）:
  - capture の `WriteBytes` を `m_ToneGenerator.GenerateSine` から `pAdapterComm->RvfLoopbackCaptureWrite` へ置換（upstream SYSVAD は capture に合成サインを書くだけで render データを一切運ばない — attempt-13 の `observed_rms=0.3535`/440Hz share 0.000 がその実測）。render 未登録・未 RUN・format 非互換では silence（VB-CABLE 同型の cable semantics）。
  - `AllocateAudioBuffer` で speaker system-render pin のみ `RvfLoopbackRegisterRender`（`m_bLoopbackRegistered` 追加）、`FreeAudioBuffer` 先頭（DMA unmap 前・UAF 防止）と destructor で `RvfLoopbackUnregisterRender`。
  - `SetState` 完了時に `RvfLoopbackRenderRunning`、`UpdatePosition` 末尾で `RvfLoopbackRenderPosition`（linear byte cursor）を publish。
  - `SetState(KSSTATE_STOP)` で `m_ullDmaTimeStamp`/`m_ullLastDPCTimeStamp`/3 種の carry-forward（`m_hnsElapsedTimeCarryForward`/`m_hnsDPCTimeCarryForward`/`m_byteDisplacementCarryForward`）をリセット — upstream はこれらを残すため restart 時に前回 run の残渣が position/notification 計算へ混入する。`SetState(KSSTATE_RUN)` の DMA time base stamp を `m_PositionSpinLock` 下へ移動し reader との atomicity を確保。
  - `SetState(KSSTATE_STOP)` の position reset を render stream では省略し、全 position clock（`m_ullPlayPosition`/`m_ullWritePosition`/`m_ullLinearPosition`/`m_ullPresentationPosition`）と `m_llPacketCounter` を STOP 跨ぎ monotonic に維持 — attempt-13 `output timestamp regressions: 2` の driver 側対応として逐次追加（attempt-16/17/18）。**効果は無かった**（regressions は常にちょうど 2 で不変 — 発生源は audiodg の epoch bookkeeping で driver 不可制御と確定）が、実 HW clock と同じく position が逆流しない方が整合的なため維持する。capture stream は `GetReadPacket` の packet-position 計算が buffer epoch 依存のため従来どおり reset。OS 供給値（`m_ulCurrentWritePosition`/`m_ulLastOsWritePacket`/`m_ulLastOsReadPacket`）は protocol 契約として常に reset。
  - `ToneGenerator.cpp/.h` は vendored 資産として保持（`Init` も既存どおり実行 — capture 経路から呼ばなくなったのみで削除はしない）。
  - Issue #1009 driver injection implementation: driver service `Parameters` key の `RvfXrunScenario`/`RvfXrunStartPacket`/`RvfXrunPacketCount` を stream 作成時に snapshot し、`eMicInDevice` + `KSPIN_WAVEIN_HOST` の Ruviform Cable Output capture stream に限定する。capture-overflow-drop は notification completion の `KeSetEvent` だけを抑止し、capture-content-gap は指定 packet 区間に対応する DMA byte spanだけを零 fill する。registry の再読や stream 中の書き換え経路は持たない。
  - Issue #1009 cause-investigation diagnostics: 上記 injection semantics を変えず、`eMINIPORT_IHV_DEFINED` の bounded diagnostic event `0x1001`-`0x1005` を追加する。内容は scenario snapshot、zero range、notification suppression transition、zero-fill boundary、OS packet gap であり、per-packet 無制限 event ではなく state transition/boundary/16 回 cap に限定する。`m_bXrunNotificationSuppressed`・`m_ulXrunZeroFillState`・`m_ulXrunDroppedPacketEvents` はこの診断のための stream state で、oracle や音響データ経路の意味論は変更しない。

## endpoint 名の根拠

MMDevice endpoint 表示名は topology filter の bridge pin の pin-name GUID を `HKR\MediaCategories\{guid}\Name` で引いた文字列になる（VB-CABLE 等と同型）。render 側（アプリが書き込む側）を "Ruviform Cable Input"、capture 側（アプリが読む側）を "Ruviform Cable Output" とする — `endpoint_probe` の `VIRTUAL_NAME_HINTS`（"cable input"/"cable output"）と `DIRECTION_TOKENS`（input/output 除去後 stem = "ruviform cable"）に一致する命名。

## 変更内容（MIDI miniport — Issue #978 Slice C）

DMusUART は MPU-401 UART hardware miniport で、upstream は IRQ + I/O port
resource を持つ物理 device 前提。ruviform 版は root-enumerated virtual device
上の software render→capture loopback で、hardware 経路は全削除。

- `RvfMidi.h`（`private.h` 由来）: 上述の表どおり。stream `Init` から
  `pPortBase` 引数を削除（virtual device に I/O base address は存在しない）。
- `RvfMidiMiniport.cpp`（`miniport.cpp` 由来）:
  - `Init` — `ProcessResources`/`InitializeHardware`/`PcNewInterruptSync`/
    ISR 登録を全削除。resource list は参照しない（root-enumerated device に
    IRQ/I/O port resource は割り当たらない）。`PcNewServiceGroup` +
    `RegisterServiceGroup` は保持し、FIFO 用 `KSPIN_LOCK` を初期化。
    `m_fInitialized=TRUE` で `SetState(KSSTATE_RUN)` gate を満たす。
  - `NewStream` — `ResetHardware`（UART flush）呼出を削除。
  - `SetState(KSSTATE_STOP)` — capture FIFO flush を spinlock 下で実施。
    upstream は PAGED_CODE 関数内で head/tail を素朴に触っていたが、loopback
    では render stream の `Write`（DISPATCH）が同時に head/tail を動かすため
    raise-to-DISPATCH 版 `KeAcquireSpinLock` で serialize する。
  - pin descriptor — 5-pin 構成は維持（legacy render sink / DirectMusic
    render sink / bridge out / bridge in / capture source）が、render・capture
    両 stream pin の `Name` を同一独自 GUID `RVF_MIDI_LOOPBACK_NAME` に設定。
    WinMM midiIn/midiOut で同名 "Ruviform MIDI Loopback" が列挙される
    （MediaCategories 経由 — audio endpoint と同じ mechanism）。
  - `PropertyHandler_Synth` — `SYNTHCAPS.wszDescOut/In` を "Ruviform MIDI
    Loopback" に改名（`[%03X]` portbase suffix 削除。`caps->Guid` は
    `CLSID_MiniportDriverDMusUART`/`...Capture` のまま — DirectMusic が
    synth 種別識別に使う in-box class id で、独自化する意味がない）。
  - `PowerChangeNotify` — D0 resume 時の hardware 再初期化を削除し state
    bookkeeping のみ。
  - `ConsumeEvents` の package 分岐 — upstream は `PutMessageLocked` 内で
    `!m_TimerQueued` 時に `ConsumeEvents()` を再帰呼出しするが、同関数は
    `m_DpcSpinLock` を取得し直すため spinlock 再取得 deadlock になる。
    enqueue のみとし drain は呼出元 loop に委譲（upstream 潜在 bug の回避）。
- `RvfMidiMpu.cpp`（`MPU.cpp` 由来）:
  - `Write` — upstream は MPU-401 UART TX register へ `WRITE_PORT_UCHAR`
    していたが、ここを `CMiniportRvfMidi::FeedCaptureFifo` へ置換 —
    render byte を内部 128-byte ring FIFO へ積み、`IPortDMus::Notify(
    m_pServiceGroup)` で capture stream の `SourceEvtsToPort` を駆動する
    software loopback。FIFO full 時は partial write を返し呼出元の
    timer retry に委譲（upstream の FIFO-full 扱いと同じ）。capture stream
    未作成・未 RUN 時は「cable 未接続」として全 byte を消費扱い。
  - `SourceEvtsToPort` — FIFO drain → `m_AllocatorMXF->GetMessage` で
    `DMUS_KERNEL_EVENT` 化 → `m_sinkMXF->PutMessage` の構造は upstream の
    ISR 経路と同型。timestamp は upstream の `SnapTimeStamp` と同じく
    `m_InputTimeStamp`/`m_MasterClock->GetTime` 由来（MasterClock NULL
    guard 追加）。head/tail アクセスは `m_InputBufferLock` で serialize。
  - `InterruptServiceRoutine`/`InitMPU`/`ResetHardware`/`ProcessResources`/
    `InitializeHardware`/`WriteMPU`/`ReadBytes`/`ValidateBuffer*` 等の
    hardware 経路は全削除。
