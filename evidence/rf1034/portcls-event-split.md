# rf#1034 PortCls event control/emit/capture split evidence

- Issue: `scarecrow173/ruviform#1034`（closeoutではない。PR linkageは `Refs #1034`）
- branch: `feat/1034-portcls-event-split`
- base main: `5b1379f5753b7d87d8b3d74161feab472fbb9ee0`; 最終main取り込み: `9aab551cb4d258896035a7db5a44cbc7222ae3ef`
- 最終head: `26bf4fdf8334808c854839321724d24272e01c52`
- Worker/Dispatch: `orca_session_id:6042f97d-4ac3-4aa7-9718-a79e27efa013` / `task_a2fe58b942a2` / `ctx_151e50ec371b`
- 実施日: 2026-11-27 UTC

本sliceは製品修正ではなく、control IRP到達・driver emit/発火条件・ETW capture/decodeを分離するdiagnostic-only観測である。timestamp oracle、startup grace、regression row、driver behavior、workflow、cpal、adapter、RunMetadata/Case Schemaは変更していない。

## 読んだ一次資料とnegative分解

Issue #1034、PR #1050、render timestamp contract、`backend-os-observation.md`、`driver-position-observation.md`、run `45992928206`/`45994869878` artifact、現行`os_endpoint.rs`、`MiniportStreamAudioEngineNode.cpp`、`minwavertstream.cpp`、`common.cpp`、`adapter.cpp`、Microsoft Learnのclassic ETW/WMI/PortCls/Kernel-EventTracing資料を確認した。PR #1050ではsession enablementとXML conversionは成立しMicrosoft-Windows-Audio eventもdecodeできたが、rf schema `0x2001`・standard miniport event・lifecycle eventは0件であり、control到達・emit・captureのどこで失われたか未分離だった。本sliceはそのzero resultの繰り返しではなく、positive controlとlifecycle markerを追加した。

## 観測boundary設計

1. controller enablement: `Microsoft-Windows-Kernel-EventTracing` event id 14/15（Enable/Stop opcode）を同じsessionでdecodeし、対象GUIDとsession名の一致を見る。これはcontroller requestがETW subsystemへ入った証拠であり、WMI control IRPがPortCls driverへ届いた直接証拠ではない。
2. classic self-test: test-only classic provider `{6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2001}`、event class `{6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2002}`、32 byte既知payloadを`RegisterTraceGuidsW`/control callback/`TraceEvent`で1件emitする。positiveでもkernel-mode WMI captureの完全証明ではない。
3. miniport lifecycle: stream成功時にsource line上発火するはずの`0x1001`/`0x1002`とstandard ordinal 1/2/4をdecode inventoryへ追加した。
4. position comparison: lifecycleと`0x2001`/standard ordinal 5を比較する。event absenceをdriver innocenceやroot cause exclusionに使わない。

provider matrixはKernel-EventTracing `{B675EC37-BDB6-4648-BC92-F3FDC74D3CA2}`、Microsoft-Windows-Audio、KernelStreaming、AudioEngineVerbose、PortCls control `{10EB6007-818C-4DB6-A694-B518E589D07A}`、PortCls event `{97BCD203-6286-4974-801D-A293B921F84B}`、self-test controlをlevel `0xff`・keywords allでenableする。PortCls Position/Root event GUIDはidentityとして記録しenableしない。raw discovery/provider log/session query/start/stop/delete/tracerpt logは各run dirに保持した。

self-test implementationは`os_endpoint.rs`内のWindows限定small unsafe moduleに閉じ込め、RT callback外でregister/start後enable待ち/1件emit/unregisterする。registration/enable/emit errorはoracleを変えずsidecarへfail-closedに記録する。run 1の解析でKernel witness message中のGUID文字列をPortCls eventと誤認するlegacy scanner分類を発見したため、最終headではprovider/event identityだけで分類する修正を入れた。event split verdict自体はこのlegacy coverage分類に依存しない。

## provisioned serializationとrun matrix

各dispatch前にin-progress/queuedが空であること、headと`origin/feat/1034-portcls-event-split`の一致を確認した。最初のdispatch前にはCoordinatorがOrca正本で競合workerなしを確認した。同時runは1本だけで、各runをforeground `gh run watch --exit-status`でterminalまでwatchし、artifact download/抽出後に次へ進んだ。workflow fileとrunner状態は変更していない。

| run | attempt | head SHA | artifact id / zip sha256 prefix | watch/result | rf1034 | rf629 audio | split verdict | key counts |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `46010050377` | 1 | `d690cdbdb88a` | `15203288492` / `11b7ea9fc4dc922e` | watch exit 0 / success | fail | pass | `driver-upstream-or-portcls-emission-unresolved` | witness control enable/disable 1/1, selftest payload 1, miniport total 0, 0x1001/0x1002 0/0, 0x2001 0 |
| `46011572850` | 1 | `d690cdbdb88a` | `15205360513` / `df9d167a64e086b1` | watch exit 0 / success | pass | missing | `artifact-missing-watchdog-timeout` | no event-split sidecar |
| `46013640129` | 1 | `26bf4fdf8334` | `15205487173` / `838b2eceb9a0554d` | watch exit 0 / success | fail | fail | `driver-upstream-or-portcls-emission-unresolved` | witness control enable/disable 1/1, selftest payload 1, miniport total 0, 0x1001/0x1002 0/0, 0x2001 0 |

- run `46010050377`: selftest=classic-selftest-decoded; witness=control-request-witnessed; lifecycle=miniport-lifecycle-absent-after-stream-success; position=position-event-absent; driver-build=ok/exit 0; wiring=error/exit 101; cleanup=ok; ETL 5578752 bytes sha256:16df1178a47bce54; XML 57167235 bytes sha256:ecf5b92affd42093; artifact zip `11b7ea9fc4dc922e6526160ae50b507d01f876f1f4894ca4abd7d6b707aa96ca`。
- run `46011572850`: os-endpoint-wiring exited before round-trip/backend trace artifact emission; driver-build=ok/exit 0; wiring=error/exit 3; cleanup=ok; ETL absent; XML absent; artifact zip `df9d167a64e086b1f5e8c7301766e71f6b4df171ef7b7c4aae8b1205ded58f6b`。
- run `46013640129`: selftest=classic-selftest-decoded; witness=control-request-witnessed; lifecycle=miniport-lifecycle-absent-after-stream-success; position=position-event-absent; driver-build=ok/exit 0; wiring=error/exit 101; cleanup=ok; ETL 5586944 bytes sha256:f34d2edf900b5e84; XML 57330608 bytes sha256:4ab1bbd0a4126425; artifact zip `838b2eceb9a0554d389cd23d897a1b464faa02adfebb0ef65190305f1fd87cb8`。

run `46011572850`は`audio_render_timestamp_baseline`自体はPassしたが、次のround-trip legが60秒watchdog timeoutとなりbackend trace/event split artifactを出力できなかった。これはtechnical blocker/artifact欠損として扱い、driver evidenceには使わない。run `46010050377`と`46011572850`のheadはscanner誤検知修正前`d690cdbd...`、run `46013640129`は最終headである。run 1/2のself-test/lifecycle観測はscanner誤検知修正の影響を受けないが、最終headのdirect evidenceはrun 3として区別する。

## positive/negativeとlayer判定

positive controlは少なくともrun 1で成立した。self-test emit APIはstatus 0で1件emitされ、XML identityとpayload byte patternの両方が1件decodeされた。したがってuser-mode classic ETWについてsession/ETL/tracerpt/decoderはpayloadを保持できる（layer 3 generic positive）。一方、rf629 stream成功にもかかわらずminiport lifecycle markerは0件だった。`miniport-lifecycle-absent-after-stream-success`はdriver/PortCls emit drop、control delivery failure、kernel WMI capture incompatibilityを単独では分けない。

Kernel-EventTracing witnessでは対象session名とPortCls control GUIDのenable eventがdecodeできた。したがって`control-request-witnessed`だが、layer 1のWMI control IRP到達をdirectに証明するものではない。PortCls event GUIDのenable witnessは0であり、self-test GUID witnessは1だった。

| layer | 判定 | 根拠 |
| --- | --- | --- |
| layer 1 control IRP到達 | unresolved | controller enable witnessはpositiveだがIRP到達surfaceがない |
| layer 2 driver emit/発火条件 | unresolved/negative候補 | stream成功後もlifecycle marker 0。helper null、PortCls drop/context buffering、control未到達、kernel WMI capture非互換を未分離 |
| layer 3 capture/decode | positive（user-mode classic generic） / kernel-wmi-capture-inconclusive | self-test identity+payload positive。kernel-mode WMIと完全同一ではない |

総合verdictはpositive runで`driver-upstream-or-portcls-emission-unresolved`である。`position-event-absent`だがlifecycleもabsentなのでposition発火条件未成立とは断定しない。cpal conversion、adapter passthrough、test oracle/consumer harnessはdirect evidenceがないためpossible/unresolvedのまま残す。driver innocenceとは書かない。

## Path B/direct surface設計評価

本sliceではcustom IOCTL/WMI data block/KS property/direct driver counter/self-test driver counterを実装していない。layer 2を強く分離する最小surfaceは、`CAdapterCommon::WriteEtwEvent` call count、helper null count、last NTSTATUS、event ordinal/schema別counterをread-onlyに返すdebug/control surfaceである。製品surface影響とleaseが必要なため実装前にCoordinator確認が必要である。direct pollingで`GetPresentationPosition`発火頻度をuser-modeから見る方法はWASAPI indirect pathとKS property direct pathを分ける設計が必要で、今回のleaseではobserverを外へ出せない。glitch-context hypothesisを検証するsafeなfault/xrun conditioned observationも別leaseである。

## verification record

canonical=mise。local OSはOrca workerのWindows 11 Pro build 26300、provisioned OS laneはGitHub Actions `windows-2025`である。cargo直接実行は補助証拠でありcanonicalではない。

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

## raw artifact / inventory / 引き継ぎ

各`evidence/rf1034/provisioned/run*-attempt1/artifact-inventory.json`にrun id、attempt、URL、artifact id、zip SHA-256、head SHA、抽出file byte size/SHA-256を記録した。text/XML/JSONはLF/行末空白を正規化し、ETLはbyte-exactである。raw artifact zipや正規化前XMLを上書きしていない。run 2はbackend artifact欠損があり、inventoryではなく本文でartifact-missing watchdogとして記録した。

次sliceへの最小引き継ぎは、(1) control IRP到達をdirectに見るread-only surfaceのlease、(2) PortClsがminiport eventを即時writeするかglitch/error contextでbufferingするかの安全な検証、(3) direct pollingによる`GetPresentationPosition`発火頻度確認、(4) driver emit surfaceを追加した場合のpost-fix provisioned validation、である。製品修正、root cause確定、regression-count-zero完了証明、docs index整備は本sliceの完了条件にしない。docs indexはCoordinatorが統合時に整備する。
