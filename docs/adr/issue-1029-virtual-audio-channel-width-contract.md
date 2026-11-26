# ADR: virtual audio endpoint の channel-width contract（Issue #1029）

## 状態

Accepted（provisioned Windows runtime evidence は本変更の CI / manual lane 結果で確定する）

## コンテキスト

Ruviform の virtual audio endpoint pair は、render 側 `Ruviform Cable Input (Ruviform Virtual Audio Device)` に書かれた音声を、capture 側 `Ruviform Cable Output (Ruviform Virtual Audio Device)` から読めるようにする Windows 仮想オーディオケーブルである。Synvelis の provisioned consumer run `45797309944` attempt `1` では、render endpoint は Stereo、capture endpoint は Mono に negotiate され、consumer は `multi-track stereo split requires >=2 channels (in=1 out=2)` で Fail になった。このときの Ruviform revision は `87a7ba8ff3fbe6d4502bea27d152d7f74c105899` であり、provisioning・readiness・artifact upload・cleanup は成立していた。したがってこれは環境不足や artifact 欠損ではなく、Ruviform virtual capture endpoint の channel-width capability に関する consumer Fail である。

既存の Ruviform round-trip harness は左右同一 stimulus を render し、capture oracle は channel 0 のみを見ていた。この構成では capture が mono、mono copy、downmix のいずれでも Pass し得るため、stereo preserve の証拠にはならない。また endpoint probe で `IsFormatSupported(shared): f32/2ch/48k: supported` と表示されていても、WASAPI shared mode の client format conversion が成功するという意味しか持たず、driver/engine が実際に stereo content を保持することを示さない。

## 観測事実と層別診断

| 層 | 観測事実 | 判定 | 根拠 |
| --- | --- | --- | --- |
| driver format range | 変更前の `micinwavtable.h` では `MICIN_DEVICE_MAX_CHANNELS` が 1、`MicInPinSupportedDeviceFormats` がすべて `nChannels=1` / `KSAUDIO_SPEAKER_MONO`、streaming pin data range も mono だけだった。`CMiniportWaveRT::IsFormatSupported` は pin の supported device format と channel 数・sample rate・channel mask を exact-match で照合する。`DataRangeIntersection` も channel count exact-match を要求し、複数 channel count をサポートする pin は channel count ごとの data range を必要とする。 | **直接原因** | capture miniport capability が mono だけを公開していたため、driver/engine が stereo capture format を真正の endpoint format として negotiate できなかった。 |
| topology | `micintoptable.h` の `MicInJackDesc.ChannelConfig` は `KSAUDIO_SPEAKER_MONO` だった。topology は endpoint の物理/論理チャネル構成を OS へ示す metadata であり、capture capability と矛盾していた。 | **寄与** | jack channel mask だけでは stream format を決定しないが、mono capability を OS/consumer へ示す一貫しない metadata であり、stereo contract と両立しない。 |
| streaming copy | `RvfLoopbackCopyToCapture` は capture format が mono かつ render source が multi-channel のとき arithmetic mean downmix を行う。一方、そうでなければ capture channel `j` に render channel `j` を割り当て、source が不足するときだけ source channel 0 を複写する。stereo→stereo の channel-mapped copy は既に存在した。 | **直接原因ではない** | mono capture format が選ばれた結果として downmix path が使われていた。stereo capture format を実際に negotiate できれば、copy 層には stereo channel mapping が既にあり、この層が mono 化の起点ではない。 |
| endpoint negotiation | 既存 harness/adapter は default F32 format を優先し、stereo range は fallback 時だけ選んでいた。capture 側 default/mix format は mono だった。`IsFormatSupported(shared)` の stereo success は OS audio engine が client-visible format conversion を受け入れることを示すにすぎず、open した stream が stereo content を保持した証拠ではない。 | **症状を固定する寄与** | driver capability が mono のままでは default negotiation は mono に落ち、stereo client format が conversion/upmix で開けても独立 channel content は保証されない。harness が stereo を明示要求しない限り、この差を検出できない。 |

結論として、直接の根源は driver format range（capture pin capability）である。topology は同じ mono 契約を表示する寄与層、endpoint negotiation は mono capability を default の mono stream として表面化する寄与層、streaming copy は結果を実行する層である。

## 決定

`Ruviform Cable Input` stereo render → `Ruviform Cable Output` capture の channel-width contract を **stereo preserve** とする。

具体的には次を意味する。

1. capture endpoint は stereo 16-bit PCM format を driver supported device format として公開し、streaming pin data range に stereo channel count を持つ。
2. `RAW` / `DEFAULT` mode の default capture format は stereo 48 kHz とする。speech / communications / far-field speech mode の default format も対応する stereo rate とする。
3. mono 16-bit PCM format は後方互換の明示的 mono capture として残す。mono open は stereo preserve contract の成功ではなく、stereo を必要とする consumer とは非互換である。
4. stereo render source を stereo capture stream が読むとき、left/right の独立内容を保持しなければならない。平均化、mono copy、channel swap はすべて contract Fail である。
5. stereo render source を mono capture stream が明示的に開いたときだけ、既存 loopback copy の arithmetic mean downmix を許す。これは stereo preserve contract の代替ではなく、mono consumer が明示選択した変換である。
6. render source がない、render stream が RUN でない、format 変換不能な場合は従来どおり silence を返す。virtual cable の観測路を bit-exact pipe とみなさない。

## 却下した選択肢

### mono/downmix を正式契約にする案

却下する。実装が簡単だからでは、正式契約の理由にならない。Synvelis consumer は stereo input を要求しており、virtual endpoint の render 側は既に stereo capable である。driver 側でも stereo preserve は、format range/default format/topology metadata を整合させれば既存 channel-mapped loopback copy で実現できる。OS/driver/RT 制約によって stereo が不可能という根拠は観測されていない。

### shared-mode stereo `IsFormatSupported` を stereo preserve evidence とみなす案

却下する。WASAPI shared mode では client format と engine/device format が異なっても OS audio engine が変換する。stereo client format を open/start できただけでは、driver/engine が独立 stereo channel content を保持したことにならない。channel count と独立 channel content の両方を oracle で確認する必要がある。

### mono format を完全に削除する案

現時点では採用しない。mono capture は stereo contract の証拠にならないが、明示的 mono consumer との後方互換性を直ちに壊す必要はない。ただし stereo fixture を要求する consumer が mono formatしか選択できない環境は contract 非互換であり、Fail として扱う。silent Pass や stereo preserve evidence への読み替えはしない。

## Harness / oracle contract

`virtual_endpoint.rs` の channel-width contract case は次を満たす。

- `Ruviform Cable Input` / `Ruviform Cable Output` の exact pair を選択し、generic virtual cable の先頭候補を使わない。
- render/capture 双方に stereo F32 client format を明示要求する。
- render stimulus は channel 0 に marker + 440 Hz tone、channel 1 に marker-free 880 Hz tone を書き、左右を独立に識別できるようにする。
- capture は channel 0 だけでなく negotiated channels を bounded planar buffer にコピーする。
- oracle は channel count、440 Hz / 880 Hz tone energy ratio、left/right separation を検査する。
- stereo preserve、mono capture、mono copy、downmix、channel swap、unsupported width、indeterminate observation を区別して machine-readable JSON に出力する。
- mono capture が stereo contract test で Pass になることはない。
- endpoint がない host では runtime case を NotRun とする。endpoint なしで実行できる deterministic oracle/schema test だけを非 ignored test とし、runtime evidence と混同しない。
- rf#1009 xrun case、fault case、既存 generic round-trip の Pass/Fail/NotRun 条件は変更しない。

## Pass / Fail / NotRun / Unsupported の語彙

- **Pass**: provisioned Windows endpoint で stereo format が実際に使われ、独立 left/right content が capture 側対応 channel で保持された。
- **Fail**: endpoint pair や stereo format が見つかった、または stereo stream を開始できたのに、mono capture、mono copy、downmix、channel swap、content loss、stream contract violation が観測された。
- **NotRun**: endpoint pair が provisioning されていない、host backend がない、manual workflow を実行できないなど、測定自体を開始できない環境理由。NotRun を Pass とみなさない。
- **Unsupported**: stereo contract oracle が意味を持たない channel width（例: stereo/mono 以外の capture width）や非有限な測定値など、契約上この oracle で分類できない観測形状。環境不足を Unsupported と呼ばない。

## Consumer 非互換条件

stereo fixture または independent channel input を要求する consumer は、capture endpoint が mono negotiation しかできない Ruviform revisionとは非互換である。Synvelis run `45797309944` の `in=1 out=2` はこの非互換を検出した正当な consumer Fail である。本 ADR の修正後は、Ruviform 側が stereo preserve evidence を示せれば Synvelis#115 を再評価する意味がある。ただし Synvelis consumer Pass は本 Issue の成果ではなく、Synvelis 側の consumer oracle/harness/workflow は変更しない。

## Driver 実装上の制約

- realtime path では allocation-free・bounded・integer math を維持し、DISPATCH_LEVEL 以下で floating point を使わない。
- stream position lock → adapter loopback lock の lock ordering、DMA buffer lifetime、RUN state publish、play cursor continuity を変更しない。
- xrun injection semantic、MIDI path、physical device compatibility は変更しない。
- stereo/mono/rate mismatch/no-render-source の意味は loopback copy の既存 contract に従う。stereo→stereo は channel-mapped copy、stereo→mono のみ arithmetic mean downmix、rate mismatch は nearest-neighbor map、render source 不在は silence。

## Verification evidence

本 ADR に対応する実行証拠は `evidence/rf1029/stereo-capture.md` に記録する。local deterministic oracle、canonical Rust lanes、driver build lane、provisioned Windows workflow lane は意味が異なるため、同じ evidence の中で区別する。local deterministic test は endpoint runtime evidence ではない。provisioned workflow が未実行なら NotRun とし、後続 gate として残す。

## 残件

- provisioned Windows virtual endpoint での stereo preserve runtime evidence は、manual `virtual-audio-driver-probe.yml` lane の実走結果で確定する。未実行なら Issue 完了の証拠にはならない。
- WDK/MSBuild 未装備 host では driver build lane を NotRun とし、WDK 装備環境で再確認する。
- stereo preserve 実装後の Synvelis#115 canonical EXT-L3 consumer run 再評価は後続 gate であり、本 Issue の成果には含めない。
- docs index（`docs/**/README.md`）は本変更では更新せず、Coordinator が統合時に整備する。
