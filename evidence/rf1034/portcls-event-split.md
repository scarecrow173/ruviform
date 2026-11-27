# rf#1034 PortCls event control/emit/capture split evidence

- Issue: `scarecrow173/ruviform#1034`（closeoutではない。PR linkageは `Refs #1034`）
- original branch: `feat/1034-portcls-event-split`
- correction branch: `docs/1034-portcls-evidence-correction`
- base main: `5b1379f5753b7d87d8b3d74161feab472fbb9ee0`; 最終main取り込み: `9aab551cb4d258896035a7db5a44cbc7222ae3ef`
- PR #1052 final head: `9486618a47f0e3b63e5e80c7cbeab14bb16f2ea9`
- PR #1052 merge commit: `16d448a3c53559d830cecb6e6aec76829ddc0da6`
- original Worker/Dispatch: `orca_session_id:6042f97d-4ac3-4aa7-9718-a79e27efa013` / `task_a2fe58b942a2` / `ctx_151e50ec371b`
- original実施日: 2026-11-27 UTC
- evidence correction実施日: 2026-11-28 UTC

本sliceは製品修正ではなく、control IRP到達・driver emit/発火条件・ETW capture/decodeを分離するdiagnostic-only観測である。timestamp oracle、startup grace、regression row、driver behavior、workflow、cpal、adapter、RunMetadata/Case Schemaは変更していない。本版は PR #1052 merge後の derived evidence correctionであり、製品コード、proof、diagnostic harness、workflow、raw capture artifactは変更していない。

## post-merge correction note

merge済み evidence の次の記述を訂正する。

1. 旧版は最終headを `26bf4fdf8334808c854839321724d24272e01c52` と記載していたが、これは最終headではない。PR #1052 の最終headは `9486618a47f0e3b63e5e80c7cbeab14bb16f2ea9`、merge commitは `16d448a3c53559d830cecb6e6aec76829ddc0da6` である。
2. `9486618a47f0e3b63e5e80c7cbeab14bb16f2ea9` は evidence記録コミットであり、その親は `26bf4fdf8334808c854839321724d24272e01c52` である。`26bf4fdf...` はscanner誤検知修正を入れた intermediate head、`d690cdbdb88a5d0b9c6c9b947a1258bff29aec7b` はその前の intermediate headである。
3. run `46010050377` / `46011572850` のheadは `d690cdbdb88a5d0b9c6c9b947a1258bff29aec7b`、run `46013640129` のheadは `26bf4fdf8334808c854839321724d24272e01c52` である。run `46013640129` は final PR head direct evidenceではない。
4. correction時点のgit照合では、`26bf4fdf...` から `9486618a...` までの `crates/`、`drivers/`、`tools/`、`.github/`、`Cargo.toml`、`Cargo.lock` の差分は空である。したがって最終headで診断harness実体は変わっていないが、commit identityが異なる以上、run `46013640129` を final-head direct evidenceと呼ばない。
5. classic self-test/control witness positive、miniport lifecycle absent、verdict `driver-upstream-or-portcls-emission-unresolved` の意味は変えない。この evidence は driver innocence、root cause確定、product fix、regression-count-zeroを主張しない。

## 読んだ一次資料とnegative分解

Issue #1034、PR #1050、render timestamp contract、`backend-os-observation.md`、`driver-position-observation.md`、run `45992928206`/`45994869878` artifact、現行`os_endpoint.rs`、`MiniportStreamAudioEngineNode.cpp`、`minwavertstream.cpp`、`common.cpp`、`adapter.cpp`、Microsoft Learnのclassic ETW/WMI/PortCls/Kernel-EventTracing資料を確認した。PR #1050ではsession enablementとXML conversionは成立しMicrosoft-Windows-Audio eventもdecodeできたが、rf schema `0x2001`・standard miniport event・lifecycle eventは0件であり、control到達・emit・captureのどこで失われたか未分離だった。本sliceはそのzero resultの繰り返しではなく、positive controlとminiport marker decodeを追加した。

## 観測boundary設計

1. controller enablement: `Microsoft-Windows-Kernel-EventTracing` event id 14/15（Enable/Stop opcode）を同じsessionでdecodeし、対象GUIDとsession名の一致を見る。これはcontroller requestがETW subsystemへ入った証拠であり、WMI control IRPがPortCls driverへ届いた直接証拠ではない。
2. classic self-test: test-only classic provider `{6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2001}`、event class `{6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2002}`、32 byte既知payloadを`RegisterTraceGuidsW`/control callback/`TraceEvent`で1件emitする。positiveでもkernel-mode WMI captureの完全証明ではない。
3. miniport marker: xrun schema `0x1001`/`0x1002`、standard ordinal 1/2/4/5、およびordinal 0のIHV-defined containerをdecode inventoryへ追加した。これらは一律の「successful stream activityで必ず発火するlifecycle marker」ではない。発火条件ごとの分解は下記matrixのとおりであり、conditional markerのabsenceと、source上このharnessで発火が期待できるmarkerのabsenceを混同しない。
4. position comparison: lifecycle candidateと`0x2001`/standard ordinal 5を比較する。event absenceをdriver innocenceやroot cause exclusionに使わない。

provider matrixはKernel-EventTracing `{B675EC37-BDB6-4648-BC92-F3FDC74D3CA2}`、Microsoft-Windows-Audio、KernelStreaming、AudioEngineVerbose、PortCls control `{10EB6007-818C-4DB6-A694-B518E589D07A}`、PortCls event `{97BCD203-6286-4974-801D-A293B921F84B}`、self-test controlをlevel `0xff`・keywords allでenableする。PortCls Position/Root event GUIDはidentityとして記録しenableしない。raw discovery/provider log/session query/start/stop/delete/tracerpt logはrun `46010050377` / `46013640129` のrun dirに保持されている。run `46011572850` ではbackend trace completion/artifact preservationへ到達せず、これらのraw artifactは存在しない。trace setup自体が開始したかどうかは残存artifactからは判別できない。

self-test implementationは`os_endpoint.rs`内のWindows限定small unsafe moduleに閉じ込め、RT callback外でregister/start後enable待ち/1件emit/unregisterする。registration/enable/emit errorはoracleを変えずsidecarへfail-closedに記録する。run `46010050377` の解析でKernel witness message中のGUID文字列をPortCls eventと誤認するlegacy scanner分類を発見し、intermediate head `26bf4fdf...` でprovider/event identityだけで分類する修正を入れた。event split verdict自体はこのlegacy coverage分類に依存しない。

## provisioned serializationとrun matrix

各dispatch前にin-progress/queuedが空であること、headと`origin/feat/1034-portcls-event-split`の一致を確認した。最初のdispatch前にはCoordinatorがOrca正本で競合workerなしを確認した。同時runは1本だけで、各runをforeground `gh run watch --exit-status`でterminalまでwatchし、artifact download/抽出後に次へ進んだ。workflow fileとrunner状態は変更していない。

時刻はUTCである。GitHub API/artifact inventoryはevent=`workflow_dispatch`、created/run_started/updatedを記録するが、client側の`gh workflow run`完了instantはrun artifactとは別項目として保存されていない。ここでは`created_at`をdispatch accepted/creation timestampとして扱い、保存されていないclient invocation instantを捏造しない。

| run | head SHA | run URL | runner OS | dispatch accepted / created / started / updated | provisioning result | ETW session | precheck exit | start exit | stop exit | delete exit | tracerpt exit | artifact preservation status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `46010050377` | `d690cdbdb88a5d0b9c6c9b947a1258bff29aec7b` | <https://github.com/scarecrow173/ruviform/actions/runs/46010050377> | `windows-2025` | `2026-11-27T13:46:49Z` / `2026-11-27T13:46:49Z` / `2026-11-27T13:46:49Z` / `2026-11-27T13:56:16Z` | driver-build ok、devnode-create ok、endpoint-probe ok、provisioning=`CANDIDATES_OBSERVED`、cleanup stage ok。stream-roundtrip/os-endpoint-wiringはproduct verdict failだがprovisioning自体は観測済み | `rvf1034_bt_1340_0` | `-2144337918`（Data Collector Set was not found） | `0` | `0` | `-2144337918`（Data Collector Set was not found） | `0` | backend ETL/XML/event-split/summary/driver-position sidecarあり。ETL 5,578,752 bytes sha256:`16df1178a47bce54`、normalized XML 57,167,235 bytes sha256:`ecf5b92affd42093`、artifact zip sha256:`11b7ea9fc4dc922e6526160ae50b507d01f876f1f4894ca4abd7d6b707aa96ca` |
| `46011572850` | `d690cdbdb88a5d0b9c6c9b947a1258bff29aec7b` | <https://github.com/scarecrow173/ruviform/actions/runs/46011572850> | `windows-2025` | `2026-11-27T13:57:27Z` / `2026-11-27T13:57:27Z` / `2026-11-27T13:57:27Z` / `2026-11-27T14:09:55Z` | driver-build ok、devnode-create ok、endpoint-probe ok、provisioning=`CANDIDATES_OBSERVED`、cleanup stage ok。os-endpoint-wiringはround-trip watchdog timeoutでerror/exit 3 | unknown/not recorded。round-tripがbackend trace completion/artifact preservation前にabort。trace setup開始の有無は残存artifactから不明 | unknown/artifact absent | unknown/artifact absent | unknown/artifact absent | unknown/artifact absent | unknown/artifact absent | partial。backend ETL/XML/event-split/summary/driver-position sidecarなし。stage log、endpoint/vaudio probe、rf1034 baseline metadataなどはあり。artifact zip sha256:`df9d167a64e086b1f5e8c7301766e71f6b4df171ef7b7c4aae8b1205ded58f6b` |
| `46013640129` | `26bf4fdf8334808c854839321724d24272e01c52` | <https://github.com/scarecrow173/ruviform/actions/runs/46013640129> | `windows-2025` | `2026-11-27T14:11:19Z` / `2026-11-27T14:11:19Z` / `2026-11-27T14:11:19Z` / `2026-11-27T14:19:49Z` | driver-build ok、devnode-create ok、endpoint-probe ok、provisioning=`CANDIDATES_OBSERVED`、cleanup stage ok。stream-roundtrip/os-endpoint-wiringはproduct verdict failだがprovisioning自体は観測済み | `rvf1034_bt_5200_0` | `-2144337918`（Data Collector Set was not found） | `0` | `0` | `-2144337918`（Data Collector Set was not found） | `0` | backend ETL/XML/event-split/summary/driver-position sidecarあり。ETL 5,586,944 bytes sha256:`f34d2edf900b5e84`、normalized XML 57,330,608 bytes sha256:`4ab1bbd0a4126425`、artifact zip sha256:`838b2eceb9a0554d389cd23d897a1b464faa02adfebb0ef65190305f1fd87cb8` |

runごとのcase resultとevent split結果は次のとおりである。workflow conclusionは3runともsuccessだが、workflow successとcase resultは別である。

| run | rf1034 render timestamp baseline | rf629 audio render→capture | rf629 midi loopback metadata | split verdict | key counts |
| --- | --- | --- | --- | --- | --- |
| `46010050377` | fail（output timestamp regressions: 1） | pass | pass | `driver-upstream-or-portcls-emission-unresolved` | witness control enable/disable 1/1、selftest payload 1、miniport total 0、0x1001/0x1002 0/0、ordinal1/2/4/5 0/0/0/0、0x2001 0 |
| `46011572850` | pass | metadata missing（watchdog timeout） | metadata missing（watchdog timeoutでos_endpoint側midi leg未到達。別stage midi-loopbackはpass） | `artifact-missing-watchdog-timeout`（driver evidenceとして使わない） | event-split sidecarなし。backend traceなし |
| `46013640129` | fail（output timestamp regressions: 1） | fail（output timestamp regressions: 1） | pass | `driver-upstream-or-portcls-emission-unresolved` | witness control enable/disable 1/1、selftest payload 1、miniport total 0、0x1001/0x1002 0/0、ordinal1/2/4/5 0/0/0/0、0x2001 0 |

run `46011572850`は`audio_render_timestamp_baseline`自体はPassしたが、次のround-trip legが60秒watchdog timeoutとなりbackend trace/event split artifactを出力できなかった。stage logには`os_endpoint: watchdog timeout after 60s — aborting`、test exit `3`、`missing copied metadata: run-metadata-rf629-audio-virtual-endpoint-render-capture.json, run-metadata-rf629-midi-virtual-endpoint-loopback.json`が残っている。これはtechnical blocker/artifact欠損として扱い、driver evidenceには使わない。

## lifecycle marker matrix

PortCls miniport payload decoderは、binary payloadのlittle-endian u64列を`trace_event_type, data1, data2, data3, data4, trailing_zero`として読み、`trace_event_type = 0x60 + ordinal`からordinalを復元する。ordinal 0は`eMINIPORT_IHV_DEFINED`のcontainerであり、data1の下位16 bitをrf diagnostic schema idとして解釈する。以下の「stream成功時の期待」はsourceの発火条件だけから判断したものであり、ETW helperがnullの場合でも`CAdapterCommon::WriteEtwEvent`が`STATUS_SUCCESS`を返す実装であるため、absenceをdriver emit未実行の直接証明には使えない。

| marker | source callsite | trigger condition | decode fields（logical data1-4） | successful stream activityでの期待 | run `46010050377` / `46013640129` observation |
| --- | --- | --- | --- | --- | --- |
| xrun schema `0x1001`（`RVF_XRUN_ETW_SCENARIO_SNAPSHOT`） | `drivers/rvf-virtual-audio/EndpointsCommon/minwavertstream.cpp` `CMiniportWaveRTStream::ReadRegistrySettings`（registry key open failure pathおよび正常読込後path）。`Init`はcapture streamのときだけ`ReadRegistrySettings`を呼ぶ | capture stream object初期化。xrun injection有効である必要はない。render-only streamでは発火しない | data1=`0x1001`、data2=pinFacts（device type/pin/injection target）、data3=scenarioFacts（scenario/start packet/packet count）、data4=NTSTATUS | このrender→capture round-tripではcapture stream作成を伴うため、capture stream初期化が成功していれば発火候補。stream activity成功だけを条件にした全stream共通markerではない | 0 / 0。capture stream callbackを伴うstream activityがあったためunresolved negative候補だが、helper null・control未到達・PortCls drop・kernel WMI capture非互換を分けない |
| xrun schema `0x1002`（`RVF_XRUN_ETW_ZERO_RANGE`） | `minwavertstream.cpp` `CMiniportWaveRTStream::AllocateBuffer`のMDL allocate/map後 | WaveRT buffer allocation path到達。xrun scenario disabledでも発火する | data1=`0x1002`、data2=`(packetBytes << 8) \| NotificationCount`、data3=zero start byte、data4=zero end byte | このdriverのWaveRT streamがbuffer allocateに成功すれば発火候補。successful stream activityは通常buffer allocationを伴うが、sourceだけでは全利用modeを保証しない | 0 / 0。round-trip stream activityがあったためunresolved negative候補だが、単独でdriver emit dropとは断定しない |
| standard ordinal 1（`eMINIPORT_BUFFER_COMPLETE`） | `minwavertstream.cpp` `TimerNotifyRT`内のnotification loop | notification interval経過で`bufferCompleted`、またはEoS last buffer rendered。さらにnotification listが空でないこと | data1=current linear buffer position、data2=previous WaveRtBufferWritePosition、data3=completed data length、data4=0 | 無条件ではない。client notification registration、notification interval、buffer completion/EoSに依存するconditional marker | 0 / 0。conditional markerのabsenceであり、発火条件未成立とcapture/emit failureを区別しない |
| standard ordinal 2（`eMINIPORT_PIN_STATE`） | `minwavertstream.cpp` `CMiniportWaveRTStream::SetState`冒頭 | PortClsからのpin state change requestごと。`KSSTATE_STOP/ACQUIRE/PAUSE/RUN`遷移を含む | data1=current linear buffer position、data2=current/previous WaveRtBufferWritePosition、data3=requested KS state、data4=0 | successful stream activityには少なくともstate遷移が必要なため、このharnessでは発火が期待できる強い候補。ただしWriteEtwEvent helper/capture経路の失敗とは分離できない | 0 / 0。stream activity後のabsenceはunresolved negative候補。driver emit未実行の直接証明ではない |
| standard ordinal 4（`eMINIPORT_SET_WAVERT_BUFFER_WRITE_POSITION`） | `MiniportStreamAudioEngineNode.cpp` `CMiniportWaveRTStream::SetCurrentWritePosition` | PortCls/audio engineがWaveRT buffer write positionを更新したとき。主にrender/event-driven write position更新に依存 | data1=current linear buffer position、data2=previous write position、data3=target write position、data4=0 | 無条件ではない。render write position更新APIが使われるかに依存するconditional marker | 0 / 0。conditional markerのabsenceであり、発火条件未成立とcapture/emit failureを区別しない |
| standard ordinal 5（`eMINIPORT_GET_PRESENTATION_POSITION`） | `MiniportStreamAudioEngineNode.cpp` `CMiniportWaveRTStream::GetPresentationPosition` | OS/audio engineがpresentation positionをqueryしたとき | data1=current linear buffer position、data2=previous write position、data3=presentation position blocks、data4=0 | 無条件ではない。successful stream activityそのものはposition queryを保証しない。query頻度・有無に依存するconditional marker | 0 / 0。`position-event-absent`の一部だが、position発火条件未成立とは断定しない |
| standard ordinal 0（`eMINIPORT_IHV_DEFINED`） | rf diagnosticイベント各callsiteのcontainer。xrun `0x1001`-`0x1005`とposition `0x2001`を含む | ordinal 0自体は独立したlifecycle eventではなく、個別schemaのtriggerに従う | data1下位16 bit=schema id。schemaごとにdata2-4の意味が異なる | ordinal 0のcountだけでは期待可否を判断できない。schema別に見る必要がある | 0 / 0。`0x1001`/`0x1002`/`0x2001`いずれもdecodeされなかった |

訂正の要点は、旧版の「stream成功時にsource line上発火するはずの`0x1001`/`0x1002`とstandard ordinal 1/2/4」という broad な表現を、そのまま維持しないことである。`0x1001`はcapture stream初期化、`0x1002`はbuffer allocation、ordinal 2はpin state changeという条件付きだが、このrender→capture round-tripでは発火候補として意味を持つ。一方、ordinal 1/4/5はclient notification、write position update、position queryに依存し、successful stream activity一般からは発火を導けない。`miniport-lifecycle-absent-after-stream-success`はharness側のaggregate labelであり、全markerが無条件lifecycleであるという意味ではない。

## positive/negativeとlayer判定

positive controlはrun `46010050377`とrun `46013640129`で成立した。self-test emit APIはstatus 0で1件emitされ、XML identityとpayload byte patternの両方が1件decodeされた。したがってuser-mode classic ETWについてsession/ETL/tracerpt/decoderはpayloadを保持できる（layer 3 generic positive）。一方、input/output双方のcallbackが観測されたstream activityにもかかわらずminiport markerは0件だった。`miniport-lifecycle-absent-after-stream-success`はdriver/PortCls emit drop、control delivery failure、kernel WMI capture incompatibility、発火条件未成立を単独では分けない。

Kernel-EventTracing witnessでは対象session名とPortCls control GUIDのenable eventがdecodeできた。したがって`control-request-witnessed`だが、layer 1のWMI control IRP到達をdirectに証明するものではない。PortCls event GUIDのenable witnessは0であり、self-test GUID witnessは1だった。

| layer | 判定 | 根拠 |
| --- | --- | --- |
| layer 1 control IRP到達 | unresolved | controller enable witnessはpositiveだがIRP到達surfaceがない |
| layer 2 driver emit/発火条件 | unresolved/negative候補 | stream activity後も`0x1001`/`0x1002`/ordinal 2を含むmarker 0。helper null、PortCls drop/context buffering、control未到達、kernel WMI capture非互換、conditional markerの発火条件未成立を未分離 |
| layer 3 capture/decode | positive（user-mode classic generic） / kernel-wmi-capture-inconclusive | self-test identity+payload positive。kernel-mode WMIと完全同一ではない |

総合verdictはpositive runで`driver-upstream-or-portcls-emission-unresolved`である。`position-event-absent`だが、ordinal 5はposition query依存でありlifecycleにもconditional markerを含むため、position発火条件未成立とは断定しない。cpal conversion、adapter passthrough、test oracle/consumer harnessはdirect evidenceがないためpossible/unresolvedのまま残す。driver innocenceとは書かない。

## ETW session cleanupの解釈

run `46010050377` / `46013640129`では、precheck queryが`-2144337918`（Data Collector Set was not found）だった。これはstart前に同名sessionが存在しなかったことを示す。startはexit 0、session queryはexit 0、stopはexit 0だった。stop後の`logman delete`は`-2144337918`（Data Collector Set was not found）で、delete実行時点ではETW sessionが既にabsent/deletedだったことを示す。

したがって、これをbroadに`cleanup=ok`とは記録しない。正確には`stop=ok`、`post-stop delete=session-not-found`であり、deleteそのものが成功した証拠ではないが、delete時点で残留sessionが見つからなかったこととは区別する。stage-cleanupの`ok`はdriver unprovision/residue checkの結果であり、ETW session cleanupと同一ではない。run `46011572850`はbackend trace artifactが残っていないため、ETW session cleanupの状態はunknownである。`cleanup=ok`や`session-not-found`とみなさない。

## ETW clock calibration

run `46010050377` / `46013640129`のsummaryは、ETW header `perf_freq=10000000`、ETW header start time、host `Instant`基準のstart/stop_before/stop_after、Unix epoch nanosecondsを記録している。しかしsummary自身のconversion noteどおり、ETW perf clockとhost `Instant`の関係は独立に確立されていない。本correctionはこのcalibrationを確立済みへ昇格しない。ETW event timestamp、driver raw QPC、host monotonic timestampの間の変換・比較は引き続きunresolvedとして扱う。

## Path B/direct surface設計評価

本sliceではcustom IOCTL/WMI data block/KS property/direct driver counter/self-test driver counterを実装していない。layer 2を強く分離する最小surfaceは、`CAdapterCommon::WriteEtwEvent` call count、helper null count、last NTSTATUS、event ordinal/schema別counterをread-onlyに返すdebug/control surfaceである。現状の`WriteEtwEvent`はhelper nullでも`STATUS_SUCCESS`を返すため、call countなしではdriverがcallsiteへ到達したかどうかをevent absenceから区別できない。製品surface影響とleaseが必要なため実装前にCoordinator確認が必要である。direct pollingで`GetPresentationPosition`発火頻度をuser-modeから見る方法はWASAPI indirect pathとKS property direct pathを分ける設計が必要で、今回のleaseではobserverを外へ出せない。glitch-context hypothesisを検証するsafeなfault/xrun conditioned observationも別leaseである。

## verification record

canonical=mise。original local OSはOrca workerのWindows 11 Pro build 26300、provisioned OS laneはGitHub Actions `windows-2025`である。cargo直接実行は補助証拠でありcanonicalではない。

### original PR #1052 verification

| verification | OS | exit/result | 解釈 |
| --- | --- | --- | --- |
| `mise run fmt-check` | local Windows | 0 | Pass |
| `mise run check ruviform-audio-test` | local Windows | 0 | Pass |
| `mise run lint ruviform-audio-test` | local Windows | 0 | Pass |
| `mise run test ruviform-audio-test` | local Windows | 0 | Pass。deterministic unit/pure helper含む |
| `mise run build-vaudio-driver` | local Windows | 1 | NotRun/Unsupported相当。WindowsKernelModeDriver10.0 toolset不在。driver build Passではない |
| `mise run test-os-endpoint-wiring` | local Windows | process exit 0 | runtime Passとは読まない。local hostのprocess終了のみ |
| `mise exec -- prek run --all-files` | local Windows | 0 | Pass |
| provisioned run matrix | windows-2025 | 上表 | EXT-L3 evidence。workflow successとcase resultは混同しない |

pstack Verification: Runtime Verificationは上記mise laneとserialized provisioned run。Formal VerificationはNotRun（formal model/proof artifact対象の製品意味論変更なし）。NotRunをPassと読まない。

### post-merge evidence correction verification

| verification | OS | exit/result | 解釈 |
| --- | --- | --- | --- |
| `mise exec -- prek run --all-files` | local Windows | 0 | Pass。docs/evidence-only correctionのcanonical hygiene check |
| `mise exec -- python -c "import json, pathlib; ..."` | local Windows | 0 | Pass。変更した`artifact-inventory.json`をparseし、`missing_or_mismatch_count == len(missing_or_mismatch) == 25`を確認。補助証拠 |

本correctionは製品コード、diagnostic harness、workflow、build設定、raw artifactを変更しないため、Workspace build/testは不要である。ローカル検証は変更範囲の検証であり、統合検証はmerge後の対象SHAに対するCI証拠で確認する。

## raw artifact / inventory / 引き継ぎ

各`evidence/rf1034/provisioned/run*-attempt1/artifact-inventory.json`にrun id、attempt、URL、artifact id、zip SHA-256、head SHA、抽出file byte size/SHA-256を記録した。text/XML/JSONはLF/行末空白を正規化し、ETLはbyte-exactである。raw artifact zipや正規化前XMLを上書きしていない。

run `46011572850`のinventoryは、backend ETL/XML/event-split artifactがabsentなのに`missing_or_mismatch_count: 0`となっていた矛盾を訂正した。backend trace artifact 23件分の`path_pattern`と、watchdog abortでmissing copied metadataとなった2件のRunMetadata pathを`missing_or_mismatch`へ列挙し、countを25とした。backend trace filename suffixはsuccessful runではprocess idだが、このrunではtrace completion/artifact preservationへ到達しなかったため`<pid-not-recorded>` placeholderを使い、実在しないfile名を捏造していない。

次sliceへの最小引き継ぎは、(1) control IRP到達をdirectに見るread-only surfaceのlease、(2) PortClsがminiport eventを即時writeするかglitch/error contextでbufferingするかの安全な検証、(3) direct pollingによる`GetPresentationPosition`発火頻度確認、(4) driver emit surfaceを追加した場合のpost-fix provisioned validation、である。製品修正、root cause確定、regression-count-zero完了証明、docs index整備は本sliceの完了条件にしない。docs indexはCoordinatorが統合時に整備する。
