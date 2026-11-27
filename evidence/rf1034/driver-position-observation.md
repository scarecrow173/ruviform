# rf#1034 driver position observation evidence

## 0. 識別情報

- Issue: `rf#1034`（本sliceはcloseoutではない。PR linkageは `Refs #1034`）
- branch: `feat/1034-driver-position-observation`
- base: `5d54434f45a1cc0741d241f60747705a1bdeb0f1`（PR #1047 merge `4c9a683ebe792a87c6f8f8796f16f0bfd39bc950` を含む）
- Worker: `orca_session_id:4ed8f749-2bd2-4f23-b1f0-449499d7d5db` / task `task_f21e8753382e` / dispatch `ctx_296398dbcf31`
- 実施日: 2026-11-27
- 目的: additive rf diagnostic event `0x2001` と関連PortCls/miniport eventについて、classic/WMI ETWのenable/decode経路をdiagnostic-onlyで確立する。Path B/direct pollingは実装していない。

## 1. 読んだ一次資料とwrong-guess確認

読んだ対象は Issue #1034、merged render timestamp contract、既存 `backend-os-observation.md`、4本の既存provisioned run artifact、`os_endpoint.rs`、`MiniportStreamAudioEngineNode.cpp`、`common.cpp/.h`、`minwavertstream.cpp/.h`、cpal 0.18.2 WASAPI timestamp conversion、`virtual-audio-driver-probe.yml`、Microsoft LearnのWMI event tracing / `IoWMIWriteEvent` / `IRP_MN_ENABLE_EVENTS` / `IPortClsEtwHelper::MiniportWriteEtwEvent` / `EPcMiniportEngineEvent` である。指定されたwrong guess pathは参照・作成していない。

cpalはWASAPI `qpc_position`（100 ns units）を100倍してnsへ変換する。driver eventのraw QPCは`KeQueryPerformanceCounter`系counter valueであり、本sliceのcorrelationはdecoded frequencyで`floor(raw_qpc * 1e9 / frequency)`へ変換する。ETW header perf clockとhost `Instant`の厳密な対応は未確定のまま残す。

## 2. 現行trace実装の診断

旧実装はkeyword mask `0x0`でmanifest providerとGUID candidateを同じprovider fileへ書き、`97BCD203`/`10EB6007`をrole区別せずtoken scanしていた。4 runではETL/XML conversionとMicrosoft-Windows-Audio event captureは成立したが、PortCls/miniport eventは0件で、`absence of events is not evidence of driver innocence`だった。`0x0`はmanifest eventにはkeyword 0のeventが写る一方、classic/WMI enable flagsとしてはevent typeを有効化しない可能性があるため、同じ設定の繰り返しを新証拠とみなさない。

## 3. provider/GUID matrix

| GUID/provider | role | 根拠 | 本sliceの扱い |
| --- | --- | --- | --- |
| Microsoft-Windows-Audio `{AE4BD3BE-F36F-45B6-8D21-BDD6FB832853}` | manifest control | `logman query providers`、`wevtutil gp`成功 | enable level `0xff` keywords all |
| Microsoft-Windows-KernelStreaming `{548C4417-CE45-41FF-99DD-528F01CE0FE1}` | manifest control | `logman query providers`、`wevtutil gp`成功 | enable level `0xff` keywords all |
| Microsoft-Windows-AudioEngineVerbose `{A6A00EFD-21F2-4A99-807E-9B3BF1D90285}` | classic/unmanifested control candidate | Microsoft core-audio trace guidance。`wevtutil gp`はexit 2 | enable。manifest providerとは書かない |
| `{10EB6007-818C-4DB6-A694-B518E589D07A}` | PortCls classic/WMI control candidate | portcls.sys `.rdata 0x264a8`、code xref `PAGE+0x3baa2`、同じcore-audio trace guidance | enable。WMI event-class GUIDと混同しない |
| `{97BCD203-6286-4974-801D-A293B921F84B}` | `PortCls_AudioEngine` / `Miniport_AudioEngine_Event` WMI event class。classic WMI control candidateでもある | root\wmi class qualifier、portcls.sys `.rdata 0x252d0`、miniport emit関数 `0xa020..0xa13d` がevent headerへcopy | enableし、decode時はevent identityとしても使う |
| `{93BB152D-DF9E-4282-B709-1696392F2557}` | `PortCls_Position` WMI event class | root\wmi class qualifier | sessionではenableしない。payload schemaを主張しない |
| `{9502CBC6-AA74-4EFF-BA91-D9329BCCE758}` | abstract root `PortClsEvent` WMI class | root\wmi inheritance metadata。portcls.sys binaryには存在するがdirect code xrefなし | sessionではenableしない。control GUIDとみなさない |

却下: `9502CBC6`をroot classだからenableする方式、`97BCD203`をmanifest providerとして`wevtutil gp`成功前提にする方式、raw binary scanだけをpositive proofにする方式。選択: manifest providerはmanifest controlとして、PortCls classic/WMI candidateはkeywords all・level `0xff`でenableする。

## 4. enable/decode path設計

`os_endpoint.rs`はsession名を`rvf1034_bt_<pid>_<pair>`、artifact名を`backend-os-trace-pair<pair>-<pid>`に固定し、stream open/start前にstart、stream drop後にstopする。ETL capは64 MiB、clock typeはperf、同時sessionは1つである。start後に`logman query <session> -ets`を保存し、stop/delete/tracerptのexit statusをsummaryへ残す。`tracerpt -lr`でXML化し、XMLがなければdecode-based attributionはUnsupportedである。

run `45992928206`のsession queryでは`10EB6007`が`Level=255`、`KeywordsAny=0xffffffffffffffff`でRunning sessionへ入っていた。これはprovider file syntaxとkeyword maskの成立を示すが、driver emit/captureの成立を意味しない。

decodeはtoken scanではない。PortCls miniport payloadは`EVENT_TRACE_HEADER` + `WNODE_FLAG_USE_MOF_PTR`の0x30 byte blockとして、`u64 trace_event_type, data1, data2, data3, data4, trailing_zero`を`BinaryEventData`から読む。portcls.sys `0xa020..0xa13d`ではenum ordinalに`0x60`を加えたtrace event typeを使い、tableは`0x60/5, 0x61/5, 0x62/4, 0x63/5, 0x64/5, 0x65/5, 0x66/5, 0x67/5, 0x68/4, 0x69/4, 0x6a/4`だった。`eMINIPORT_IHV_DEFINED=0`、`eMINIPORT_GET_PRESENTATION_POSITION=5`はMicrosoft `EPcMiniportEngineEvent` documentationで確認した。

additive rf schema `0x2001`は`Data1`低16 bitがschema id、bits 16..63がfrequency、`Data2` bit0 capture・bits 8..15 pin・bits 16..31 device type、`Data3`がpresentation position blocks、`Data4`がraw QPCである。decodeではfrequencyが100,000..100,000,000 Hzの場合だけrf eventとして採用する。これはrun `45966722893`の初版scanner false positive対策である。correlation sidecar `*-driver-position-events.json`と`*-driver-position-correlation.json`はdiagnostic artifactであり、RunMetadata/Case schemaではない。

driver emitは`MiniportStreamAudioEngineNode.cpp`の既存codeがschemaどおりData1/Data2/Data3/Data4をpackingし、`common.cpp`が`IPortClsEtwHelper::MiniportWriteEtwEvent`へpass-throughすることを確認した。position/QPC計算・lock・state・timer・buffer制御のdefectは証明できず、driver sourceは変更していない。

## 5. provisioned serialization

各dispatch前にOrca worker-listでrf1034/os_endpoint/drivers/tools/evidenceを触る競合workerがいないこと、同workflowのin_progress/queuedが全branchで空であること、local headと`origin/feat/1034-driver-position-observation`が一致することを確認した。同時runは1本だけで、`gh run watch <run> --exit-status`終了後にartifactをdownload・抽出してから次のdispatch判断を行う。workflow file、runner負荷、driver install手順、fault toolingは変更していない。

## 6. run matrix

| run | attempt | head SHA | artifact id / zip SHA-256 | workflow watch | rf1034 | rf629 audio | backend coverage | driver events |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `45992928206` | 1 | `3809df8122171c7266bf0363d10a0a0ecfdc4e8c` | `15195266601` / `b1e40a566624771b8dc38bf640ecbc364911559743443dab6bb9c81b4dc1a7b6` | exit 0 | pass | pass（raw 2/effective 0、idx `[3,4]`はepoch boundary側） | `incomplete-no-driver-event-decoded` | rf 0 / standard 0 / miniport 0 |
| `45994869878` | 1 | `9a9d36fb7f999e2a6ce27b75ad9d85d4ecbb931a` | `15194914790` / `d3b16703ee3d2c68c3cd775d922a428801efc92adb555da79364880b41a1a697` | exit 0 | pass | pass（raw 2/effective 0） | `incomplete-no-driver-event-decoded` | rf 0 / standard 0 / miniport 0 |

詳細なbyte sizeとSHA-256は各`artifact-inventory.json`にある。run `45992928206`ではETL 5,545,984 bytes、XML 58,272,104 bytes、session query log 3,573 bytesを保持した。raw ETLはbyte-exact、text/XML/JSONはrepository hygieneのためLF/行末空白を正規化し、inventoryは抽出ファイルhashとartifact zip hashを分けて記録する。

## 7. 現時点の判定語彙

2本のserialized runはいずれもbounded non-reproduction（effective regression 0）で、かつ`10EB6007`単独/keywordless相当からkeywords allへの修正後も、`97BCD203`をkeywords allで有効化した後もPortCls/miniport event decodeは0件だった。session queryはrun `45994869878`で`10EB6007`と`97BCD203`の両方がlevel 255・keywords allでsessionへ入っていたことを示す。したがってprovider file syntaxとsession enablementまでは成立したが、WMI control IRPがdriverへ届いたか、driverがemitしたか、ETW sessionがclassic/WMI payloadをcaptureしたかは未分離である。2 run連続で同じobservability blockerが再現したため、残りrunは消費せず`Deferred`とする。これはpositive proofではなく、correct enable/decode pathの完全な確立にも達しなかったhonest negative resultである。

run `45992928206`はbounded non-reproduction（effective regression 0）であり、過去run `45968714712`/`45970610888`/`45972492570`のstartup grace外regressionを否定しない。同runではsession enablementとXML conversionは成立したが、PortCls/miniport event decode positive proofは得られなかった。event absenceをdriver innocenceの証拠にしない。cpal conversion、adapter passthrough、test oracle/consumer harnessはdirect evidenceが排除しない限りpossible/unresolvedのまま残す。

## 8. local verification

canonical = mise。`mise run fmt-check` exit 0、`mise run check ruviform-audio-test` exit 0、`mise run lint ruviform-audio-test` exit 0、`mise run test ruviform-audio-test` exit 0、`mise exec -- prek run --all-files` exit 0だった。`mise run build-vaudio-driver`はlocal hostにWindowsKernelModeDriver10.0 toolsetがなくexit 1であり、driver build PassではなくNotRun/Unsupported相当として扱う。`mise run test-os-endpoint-wiring`はprocess exit 0だが、補助`--nocapture`実行で3 caseとも`NOT_PROVISIONED` / verdict NotRunを確認したためruntime Passではない。cargo直接実行は補助証拠でありcanonicalではない。

## 9. observer effectと残存作業

ETW emit自体はobserver effectを持ち得る。instrumented driver上のPass/non-reproductionはuninstrumented baselineの修正証拠ではない。本sliceは製品修正、fresh-QPC adapter-fix、timestamp synthesis/smoothing/clamping、driver redesign、cpal patch、adapter変更、workflow変更、oracle/classifier/startup grace/regression row変更をしていない。

Path Aが最終的にlease内で確立できない場合のresidual future workは、Path B/direct pollingまたは別driver observability surfaceの設計と、それに必要なleaseである。製品修正候補の確定、post-fix regression-count-zero validation、cpal/adapter/test-oracleをさらに分離する別lease、docs index整備は本sliceの完了条件にしない。
