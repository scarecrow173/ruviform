# rf#1034 PortCls WMI control/callsite probe evidence（訂正版）

- Issue: `scarecrow173/ruviform#1034`（closeoutではない。PR linkageは `Refs #1034`）
- branch: `test/1034-portcls-wmi-control-probe`
- run対象SHA: `d24decae4db56ae9f208b55a184e986154b55137`
- 最初のevidence記録commit: `b48d8f26b7520658ae549196f6d1a2f8ec7272e1`
- この訂正前のPR head: `c231c15f4ff19ca3104d22e5cc744822d40231f8`
- 訂正Worker/Dispatch: `orca_session_id:18a44828-875d-4dfa-86dd-9b27f9263ded` / `task_99c094ea1867` / `ctx_3ae8308129e7`

本版は証跡訂正のみである。追加実走、GitHub Actions起動/再実行、workflow編集、driver/harness/raw artifact/`actions.jsonl`変更は行っていない。PRは保留、Issue #1034はopenのままである。訂正後のheadは本文書へ固定値としては記さず、完了報告に記録する。

## 訂正の要点

1. 旧版は `final head` を固定値のように記していたが、run対象SHA、evidence記録commit、訂正前headを区別する。run対象は `d24decae4db56ae9f208b55a184e986154b55137`、最初のevidence記録commitは `b48d8f26b7520658ae549196f6d1a2f8ec7272e1`、この訂正前headは `c231c15f4ff19ca3104d22e5cc744822d40231f8` である。
2. run SHAと訂正前headの実装同一性は、commitが同一であるとは書かず、変更ファイル起点で照合した。`git diff --name-only d24decae...c231c15f -- crates drivers tools .github Cargo.toml Cargo.lock mise.toml mise.ci.toml .pre-commit-config.yaml` は空であり、差分は `evidence/rf1034/` 以下の証跡ファイルだけだった。したがって実装差分がないことは確認できるが、実際に走ったSHAと訂正前headが同一commitであるという意味ではない。
3. provisioned workflow dispatchは手順上最大3回までのところ、branch上で10回確認できた。最後の2回が非重複であっても、それ以前の超過・重複という手順不適合は解消しない。
4. 既存runの実測は履歴・診断情報として記録する。serialization手順不適合があるため、正式な受入結果とはしない。

## process deviation記録

`drivers/rvf-virtual-audio/WmiControlProbe.cpp` と `WmiControlProbe.h` は事前承認なしに追加され、その後Coordinatorが遡及的に承認した。lease拡大はこの2ファイルだけであり、他のlease拡大はない。この事実は実装の正否とは別のprocess deviationとして残す。

## probe設計と観測boundary

driver-owned WMI control/callsite probeは次の診断面を分離する。製品挙動、timestamp oracle、workflow、cpal、adapter、RunMetadata/Case Schemaは変更しない。

- control GUID `{6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2101}`、event GUID `{6F1D2B3A-1034-4A7F-9E2C-5A11C0FF2102}`
- schema 1、signature `RVF1034W`、120-byte payload
- kind: control enable=1 / control disable=2 / `WriteEtwEvent` mirror=3
- mirror sampling: ordinal/schema keyごとの先頭4件、その後enabled中は32件ごと
- 専用sidecar `backend-os-trace-pair0-<pid>-wmi-control-probe.json`（schema `rf1034-portcls-wmi-control-probe/v1`）

## workflow dispatch履歴とserialization不適合

`virtual-audio-driver-probe.yml` はread-onlyのままだったが、branch `test/1034-portcls-wmi-control-probe` では次の10回のrunが確認できる。

| run | head | conclusion | createdAt UTC | 訂正上の扱い |
| --- | --- | --- | --- | --- |
| `46037662321` | `8afa85dd` | success | `2026-11-27T16:55:35Z` | implementation iteration |
| `46041953797` | `f1935e28` | success | `2026-11-27T17:27:15Z` | implementation iteration |
| `46045524841` | `2f19cc49` | cancelled | `2026-11-27T17:55:13Z` | implementation iteration |
| `46046645274` | `21b2e57f` | success | `2026-11-27T18:04:13Z` | implementation iteration / trace capture blocker |
| `46047827060` | `21b2e57f` | cancelled | `2026-11-27T18:13:29Z` | implementation iteration |
| `46048751409` | `a4a18e50` | success | `2026-11-27T18:20:40Z` | pre-final harness head |
| `46050338327` | `a4a18e50` | cancelled | `2026-11-27T18:33:18Z` | 重複seriesの先行走行 |
| `46050601140` | `a4a18e50` | cancelled | `2026-11-27T18:35:23Z` | `46050338327` と重複した誤dispatch。pending後にcancel |
| `46052251860` | `d24decae` | success | `2026-11-27T18:49:09Z` | 最終実装run 1。診断履歴として参照 |
| `46053237700` | `d24decae` | success | `2026-11-27T18:57:35Z` | 最終実装run 2。診断履歴として参照 |

`46050338327` が実行中に `46050601140` をdispatchしたことが重複である。`46050601140` はcancelledとなったが、dispatch自体は発生している。最大3回の条件に対し合計10回起動しており、最後の `46052251860` / `46053237700` が互いに重複せず逐次だったとしても、超過dispatchと重複dispatchの手順不適合は遡って解消されない。

## 最終実装runの診断履歴

以下は正式な受入結果ではなく、既存artifactに残る診断履歴である。run `46052251860` / `46053237700` は同じrun対象SHA `d24decae4db56ae9f208b55a184e986154b55137` で、workflow conclusionはsuccess、ETW trace captureとdriver-build stageは成立した。

| run | artifact id | artifact zip SHA-256 | probe verdict | write delta | mirror key/emit delta | miniport events | rf1034 / rf629 audio / rf629 midi |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `46052251860` | `15224319150` | `159ee5baf40c1249e97bec55e2c8833901e946ecd47bb79bd3a6eed920394a0e` | `driver-wmi-control-and-callsite-decoded` | 13 | 13 / 5 | 0 | pass / fail / pass |
| `46053237700` | `15225373315` | `416678b6caa4a5f8047d816cf4a8570f5d42b33bfa30aea73659c9d410ead934` | `driver-wmi-control-and-callsite-decoded` | 15 | 15 / 5 | 0 | fail / fail / pass |

両runで、driver-owned control enable/disable eventがidentity+payload一致でdecodeされ、`WriteEtwEvent` mirror 5件もdecodeされた。counter windowではhelper-null delta 0、forward-error delta 0だった。この範囲では、driver-owned WMI control callback経路、kernel WMI payload capture、sampled `WriteEtwEvent` callsite到達はpositiveである。

一方、実PortCls miniport eventは両runとも0件であり、rf position schema `0x2001`・standard presentation position eventも観測されなかった。sidecarのposition比較でもordinal4 mirror 0、ordinal5 mirror 0、schema `0x2001` mirror 0だった。したがってpin/xrun-like callsite到達とdownstream miniport absenceの分離にはなるが、position source-side callの有無・頻度はこのcontrol windowでは確定できない。

## 判定の制限

- 本evidenceはdiagnostic-onlyであり、回帰解消、root cause確定、driver無関係、driver innocence、regression-count-zeroを主張しない。
- workflow conclusion successとcase-level resultは別である。rf1034/rf629 audioのfailをpassとは読まない。
- miniport/position event未観測は、発火条件未成立、PortCls/publication/capture visibility、kernel WMI capture非互換を単独では分けない。
- cpal conversion、adapter passthrough、consumer harness、environmentはdirect evidenceで排除されていない限りunresolvedのままである。
- serialization手順不適合があるため、既存run結果は正式受入ではなく後続sliceの履歴・診断入力である。

## 検証

本訂正は `evidence/rf1034/portcls-wmi-control-probe.md` のみを変更する証跡訂正であり、driver/harness/unit testの再検証や追加実走は行わない。訂正後に実行するのは `git diff --check` と `mise exec -- prek run --files evidence/rf1034/portcls-wmi-control-probe.md` だけである。

## artifact inventory

run `46052251860` / `46053237700` の抽出artifact、byte size、SHA-256、artifact zip SHA-256は `evidence/rf1034/provisioned/run46052251860-attempt1/artifact-inventory.json` と `evidence/rf1034/provisioned/run46053237700-attempt1/artifact-inventory.json` に記録されている。raw artifactやinventoryは本訂正で変更していない。

## next minimum handoff

- 正式な受入に使う場合は、serialization条件を満たす新しいleaseとrun計画を別途確認する。
- position source-side callの発火条件未成立とcapture/emit経路を分ける最小surfaceを別sliceで設計する。
- cpal conversion、adapter passthrough、consumer harness、environmentの未排除仮説を残したまま、製品修正やIssue closeoutへ進まない。
