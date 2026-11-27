# rf#1034 PortCls WMI control/callsite probe evidence

- Issue: `scarecrow173/ruviform#1034`（closeoutではない。PR linkageは `Refs #1034`）
- branch: `test/1034-portcls-wmi-control-probe`
- PR #1057 final head: `b48d8f26b7520658ae549196f6d1a2f8ec7272e1`（evidence記録commit。provisioned run headは `d24decae4db56ae9f208b55a184e986154b55137`）
- Worker/Dispatch: `orca_session_id:18a44828-875d-4dfa-86dd-9b27f9263ded` / `task_44fc75e61f55` / `ctx_2ab735260993`

本sliceはdiagnostic-only観測であり、issue closeout、製品修正、root cause確定、driver innocence証拠、regression-count-zero完了証明ではない。timestamp oracle、startup grace、regression row、driver behavior、workflow、cpal、adapter、RunMetadata/Case Schemaは変更していない。

## process deviation記録

`drivers/rvf-virtual-audio/WmiControlProbe.cpp` と `WmiControlProbe.h` は事前承認なしに追加され、その後Coordinatorが遡及的に承認した。このprocess deviationを明示的に記録する。lease拡大はこの2ファイルのみで、他のlease拡大はない。

## probe設計

driver-owned WMI control/callsite probeを追加し、driver側のWMI制御callback経路とkernel WMI payload capture、`CAdapterCommon::WriteEtwEvent` callsite到達を分離観測する。

- control GUID: `{6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2101}`
- event GUID: `{6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2102}`
- schema 1、signature `RVF1034W`、120-byte payload
- control kind: enable=1 / disable=2 / mirror=3
- mirror sampling: keyごとに先頭4件、その後enabled中は32件ごと

## serialization制約とrun matrix

final-head runはserializedに1本ずつ実行した。各dispatch前にin-progress/queuedが空であることとhead一致を確認し、前runをforegroundでterminalまでwatchしてartifact抽出後に次へ進んだ。workflow fileとrunner状態は変更していない。

final serialized runは2本で十分と判断した。run head `d24decae...` から evidence commit `b48d8f26...` までの driver/harness差分はなく、evidence追加のみである。両runのprobe verdictが一致するため、optionalな第3runは省略した。これ以前のrunはimplementation-iteration evidenceであり、final-head proofとしては使わない。

| run | head SHA | run URL | artifact id | artifact zip SHA-256 | created / updated (UTC) | workflow conclusion |
| --- | --- | --- | --- | --- | --- | --- |
| `46052251860` | `d24decae4db56ae9f208b55a184e986154b55137` | <https://github.com/scarecrow173/ruviform/actions/runs/46052251860> | `15224319150` | `159ee5baf40c1249e97bec55e2c8833901e946ecd47bb79bd3a6eed920394a0e` | `2026-11-27T18:49:09Z` / `2026-11-27T18:57:06Z` | success |
| `46053237700` | `d24decae4db56ae9f208b55a184e986154b55137` | <https://github.com/scarecrow173/ruviform/actions/runs/46053237700> | `15225373315` | `416678b6caa4a5f8047d816cf4a8570f5d42b33bfa30aea73659c9d410ead934` | `2026-11-27T18:57:35Z` / `2026-11-27T19:06:00Z` | success |

両runともtrace captured、provider discovery / logman start / stop / tracerptはsuccess。第2runのcreated_at `18:57:35Z`は第1runのupdated_at `18:57:06Z`より後であり、serializationはtimestamp上も確認できる。

## run result matrix

両run共通のprobe結果:

- split verdict: `driver-upstream-or-portcls-emission-unresolved`
- driver_wmi_probe verdict: `driver-wmi-control-and-callsite-decoded`
- probe events 7件: control enable=1 / disable=1 / mirror=5
- helper_null delta 0、forward_error delta 0
- miniport events total 0
- classic selftest decoded
- stream success true
- sidecar `backend-os-trace-pair0-<pid>-wmi-control-probe.json`（schema `rf1034-portcls-wmi-control-probe/v1`）が存在

| run | write_etw_event_calls delta | mirror_key delta | mirror emit delta | rf1034 render timestamp baseline | rf629 audio render→capture | rf629 midi loopback | callbacks input/output |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `46052251860` | 13 | 13 | 5 | pass | fail（output timestamp regression 1件） | pass | 328 / 331 |
| `46053237700` | 15 | 15 | 5 | fail | fail | pass | 329 / 332 |

## position比較

sidecarのposition比較では、ordinal4 mirror 0、ordinal5 mirror 0、schema `0x2001` mirror 0だった。したがってpin/xrun-like `WriteEtwEvent` callsiteにはpositiveに到達したが、position source-side callはcontrol window内で観測されなかった。position absenceは帰属未確定（unattributed）のまま残す。

## positive/negative/inconclusiveの意味

positive:

- driver-owned WMI control callback経路とkernel WMI payload captureはpositive。control enable/disableとmirror eventがdecodeされ、payloadが保持された。
- sampled `WriteEtwEvent` callsiteはhelper present・forward status成功で実行された（helper_null delta 0、forward_error delta 0）。
- classic selftestはdecodeされ、user-mode classic ETWのsession/ETL/tracerpt/decoder経路はpayloadを保持できる。

negative候補（unresolved negative）:

- 実PortCls miniport eventsは両runとも0件。ただしabsenceはdriver callsite以降（PortCls/publication/capture visibility）の問題であり、driver innocenceの証明ではない。

inconclusive/unresolved:

- position source-side callがcontrol window内で観測されなかった理由は未帰属。
- cpal conversion、adapter passthrough、consumer harness、environmentはdirectに除外されていない限りunresolvedのまま。

## limitations

- 本evidenceはdiagnostic-onlyであり、製品修正・root cause確定・driver innocence・regression-count-zeroを主張しない。
- workflow conclusion successとcase resultは別物として扱う（rf1034/rf629のcase failはrun `46053237700`のrf1034 baseline、両runのrf629 audioに存在）。
- miniport event 0件は発火条件未成立・PortCls drop・kernel WMI capture非互換を単独では分けない。
- ETW clock calibrationは本sliceでも確立していない。
- local `build-vaudio-driver`はWindowsKernelModeDriver10.0 toolset不在でNotRun/Unsupported。実build evidenceはprovisioned driver-build stageである。`test-os-endpoint-wiring`はprocess exit 0だがruntime Passとは読まない。

## local verification記録

final harness変更後のlocal検証（canonical=mise、local OS）:

| verification | exit/result | 解釈 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-audio-test` | 0 | Pass |
| `mise run lint ruviform-audio-test` | 0 | Pass |
| `mise run test ruviform-audio-test` | 0 | Pass。os_endpoint 32 passed / 3 ignored |
| `mise run test-os-endpoint-wiring` | process exit 0 | runtime Passとは読まない |
| `mise run build-vaudio-driver` | NotRun/Unsupported | WindowsKernelModeDriver10.0不在。provisioned driver-build stageが実build evidence |
| `mise exec -- prek run --all-files` | 0 | Pass。evidence追加後の最終hygiene |

## artifact inventory

各runの抽出artifact一覧・byte size・SHA-256・artifact zip SHA-256は `evidence/rf1034/provisioned/run46052251860-attempt1/artifact-inventory.json` と `evidence/rf1034/provisioned/run46053237700-attempt1/artifact-inventory.json` に記録した。ETL/XMLを含む全file hashはこれらのinventoryを参照。text/XML/JSONはLF/行末空白を正規化し、ETLはbyte-exactで保持している。raw artifact zipや正規化前XMLは上書きしていない。

## next minimum handoff

- position source-side callがcontrol window内で観測されない原因の帰属（発火条件未成立 vs capture/emit経路）を分ける最小surface。
- cpal conversion、adapter passthrough、consumer harness、environmentのうちdirectに除外できていない経路の切り分け。
- 本evidenceはissue closeout・製品修正・root cause確定の完了条件にしない。
