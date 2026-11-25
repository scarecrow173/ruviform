# rf#1002 Slice G — capability matrix 実測確定・fault case 実走

Issue: `scarecrow173/ruviform#1002` / rf#629 AC5 Slice G
Branch: `feat/1002-capability-fault`

## 設計判断

1. fault harness の OS 状態制御は workflow/PowerShell 側（`tools/virtual-audio-driver/Invoke-FaultCases.ps1`）へ閉じ込め、Rust test は Expected/Observed oracle・recovery reopen・機械可読 result emit だけを担当する。Rust test 本体は PnP/registry/service を直接変更しない。
2. `virtual_endpoint.rs` の fault test は `RUVIFORM_FAULT_*` で武装された場合だけ active になる。canonical lane で未武装実行された場合は `fault_case_result: ... result=inactive` を出して即時終了し、既存 round-trip の generic `verdict:` 契約に混入しない。
3. devnode loss の第一選択は `disable`/`enable`、open stream により OS が拒否した場合の fallback は remove/recreate とした。`tools/testsigned-driver/TestDevice.ps1` には既存 create/remove semantic を変えない後方互換の `disable`/`enable` を追加した。
4. permission denial は stream-roundtrip が使う microphone consent surface を Deny にし、`AudioEndpointBuilder`/`Audiosrv` restart 後に capture open を試す。oracle は緩めず exact `DeviceError::Backend("permission_denied")` とし、finally で Allow+service 状態を戻してから recovery reopen を要求する。
5. MIDI device-loss observation の正規 oracle は、devnode loss 前に存在した `Ruviform MIDI Loopback` が消失中の WinMM enumeration から消えることとした。復帰後の再出現も before/during/after の全 probe で記録するが、device recovery capability と混同しない。
6. xrun は deterministic injector ではなく、bounded user-mode starvation が現行 adapter 経路で `DeviceError::Xrun`・`ClockDiscontinuity`・`StreamInvalidated`・その他 error として観測できるかを測る probe とした。OS-L1 deterministic backend fault を EXT-L3 xrun の代替 Pass にはしない。
7. `os_endpoint.rs` には verdict 判定を変えず、input/output error kind（Xrun/ClockDiscontinuity/StreamInvalidated/Backend）の計数出力だけを追加した。
8. case-level `Pass/Fail/NotRun/Unsupported` を正規判定とし、workflow run conclusion や fault-cases stage `ok` を capability Pass と混同しない。stage `ok` は測定 harness が case-level Fail/harness problem なく完走した意味に限定する。

## ローカル検証（worker ホスト、未 provisioned）

worker ホストには `Ruviform Cable Input`/`Ruviform Cable Output`/`Ruviform MIDI Loopback` がない。したがって canonical lane の local evidence は provisioned Pass ではなく、未 provisioned fail-closed/未武装 inactive の非回帰証拠である。

| 検証 | 結果 | evidence |
| --- | --- | --- |
| `mise run fmt-check` | exit 0 | `local/fmt-check-final.txt` |
| `mise run check ruviform-audio-device` | exit 0 | `local/check-audio-device-final.txt` |
| `mise run lint ruviform-audio-device` | exit 0 | `local/lint-audio-device-final.txt` |
| `mise run test ruviform-audio-device` | exit 0 | `local/test-audio-device-final.txt` |
| `mise run lint-audio-real-input` | exit 0 | `local/lint-audio-real-input-final.txt` |
| `mise run check ruviform-audio-test` | exit 0 | `local/check-audio-test-final.txt` |
| `mise run lint ruviform-audio-test` | exit 0 | `local/lint-audio-test-final.txt` |
| `mise run test ruviform-audio-test` | exit 0 | `local/test-audio-test-final.txt` |
| `mise run test-os-endpoint-wiring` | exit 0・2 passed（canonical capture 出力） | `local/test-os-endpoint-wiring-final.txt` |
| `mise run test-audio-virtual-endpoint` | exit 0・4 passed（round-trip + 未武装 inactive fault tests） | `local/test-audio-virtual-endpoint-final.txt` |
| `mise run test-midi-virtual-endpoint` | exit 0・2 passed | `local/test-midi-virtual-endpoint-final.txt` |
| `mise run test-ci-daily` | exit 0 | `local/test-ci-daily-final.txt` |
| `mise run test-ci-tracker` | exit 0 | `local/test-ci-tracker-final.txt` |
| `mise exec -- prek run --all-files` | exit 0 | `local/prek-all-files-final.txt` |
| 補助: `mise exec -- actionlint .github/workflows/virtual-audio-driver-probe.yml` | exit 0 | `local/actionlint-virtual-audio-final.txt` |
| 補助: PowerShell parser check | `Invoke-FaultCases.ps1`/`TestDevice.ps1` とも exit 0 | `local/pwsh-parse.txt` |

補助 `--nocapture` 実行（canonical = mise lane と区別）では、audio wiring は `verdict: NotRun — no input devices on this host`、MIDI wiring は `verdict: NotRun — NOT_PROVISIONED`、audio virtual endpoint の fault tests は `result=inactive ... controller-not-armed` を確認した（`local/aux-*.txt`）。`test-midi-virtual-endpoint` の GS silent output smoke は in-box synth への Pass であり、Rvf virtual endpoint provisioning の証拠ではない。

## branch-ref provisioned 実走

run conclusion は absence-is-data 設計上の harness 完了にすぎず、判定根拠にはしていない。各 attempt の正本は artifact 内 `fault-cases-results-windows-2025.json`、`stage-fault-cases-windows-2025.txt`、`vaudio-probe-windows-2025.json`、fault probe 出力である。

### attempt 1 — run 45567369491 / SHA bc75ebc70623864a08d2907d532ddc5a4767e7b2

- artifact: `attempt1-run-45567369491-artifact/`。
- `permission_denial_audio`: exact `Backend("permission_denied")` を観測したが、PowerShell array unrolling により restore signal が出ず case `Fail`。harness bug。
- `device_loss_audio`: reopen recovery は callbacks=38 で成立したが、`StreamInvalidated`/control-path `DeviceNotFound` は未観測で case `Fail`。
- `midi_device_loss_observation`: PowerShell output pollution で probe exit が nonzero となり measurement bug。
- `xrun_probe`: active callbacks=421 に対し xrun/discontinuity/invalidated/error すべて 0 で `Unsupported`。これは有効な負の測定。

### attempt 2 — run 45569417286 / SHA 8718160524c96ac492b1e7f20cd3fb5e8d99f492

- artifact: `attempt2-run-45569417286-artifact/`。
- controller 抽出後の実走。`Disable-PnpDevice`/`pnputil /disable-device` は open stream 中に拒否され、remove/recreate fallback が stimulus として成立した。
- MIDI disappearance は有効に観測された（before=True/exit0、during=False/exit0）。ただし after=False であり、当時の oracle は再出現も要求していたため case `Fail`。
- device loss recovery が endpoint surface 復帰前に進み、後続 permission/xrun が `no-output-devices-on-this-host` で汚染された。これらは permission/xrun の capability 測定としては使わない。

### attempt 3 — run 45571459944 / SHA c3978ebeabc68558cfa79f9517528621815aeefe

- artifact: `attempt3-run-45571459944-artifact/`。
- case 実行順を permission → xrun → device loss へ変更し、device loss の surface 汚染を分離した。harness problems は 0。
- `permission_denial_audio`: `Pass`。Deny 後 capture open が exact `permission_denied`、Allow restore 後に `Ruviform Cable Output` が再列挙され clean open/start/stop（callbacks=39）まで成立。
- `xrun_probe`: `Unsupported`。callbacks=422 で xruns/discontinuities/invalidated/errors すべて 0。
- `device_loss_audio`: removal stimulus は成立したが typed invalidation/control error は 0、recreate 後 endpoint は 45 秒の bounded polling 内に再列挙されず case `Fail`。
- `midi_device_loss_observation`: before=True/exit0、during=False/exit0、after=False/exit0。消失は再現したが再出現しなかったため、当時の disappearance+reappearance oracle では `Fail`。

### attempt 4 — run 45573425912 / SHA 6265671a50541a3ace2559bfb6f1ad277fc89691

- artifact: `attempt4-run-45573425912-artifact/`。
- stage record: `fault-cases ok`、`harness_problems=0`。run conclusion ではなく case JSON を正規判定とする。
- `permission_denial_audio=Pass`: Deny 後 capture open が exact `permission_denied`、Allow restore 後に `Ruviform Cable Output` 再列挙・clean open/start/stop・callbacks=39。
- `midi_device_loss_observation=Pass`: before=True/exit0、during=False/exit0 で disappearance を実測。after=False/exit0 であり、reappearance は Pass にしていない。
- `device_loss_audio=Unsupported`: removal stimulus 成立後も `invalidated=0`・`control_error_kind=0`・`discontinuities=0`。recreate 後 reopen は `no input devices on this host`。typed invalidation と保証された recovery を組み合わせた capability は現行構成では成立しない。
- `xrun_probe=Unsupported`: callbacks=422 で xrun/discontinuity/invalidated/error すべて 0。typed xrun の決定的注入面と adapter 観測契約は `scarecrow173/ruviform#1009` へ分離済み。
- 補足: 同 run の `os-endpoint-wiring` は midi Pass だが audio が output timestamp regression 1 で Fail だった。fault-cases は wiring 後に実行されるため fault harness が原因ではなく、rf#1001 attempt1 と同じ timestamp flake として切り分ける。rf#1001 run `45555696425` と本 branch attempt3 run `45571459944` では audio/midi wiring Pass を確認済みだったが、同一 SHA の再実走を attempt5 として追加した。

### attempt 5 — run 45575005948 / SHA 6265671a50541a3ace2559bfb6f1ad277fc89691（最終確定）

- artifact: `attempt5-run-45575005948-artifact/`。
- 既存 lane 非回帰: `endpoint-probe`/`midi-endpoint-probe`/`stream-roundtrip`/`midi-loopback` は `ok`、`os-endpoint-wiring` も audio/midi 両 RunMetadata `result=pass`・`level=ext-l3`・`environment_kind=virtual-os-endpoint` で `ok`。attempt4 の audio timestamp regression は再現せず、rf#1001 で既知の flake であることを確認した。
- stage record: `fault-cases ok`、`harness_problems=0`。
- `permission_denial_audio=Pass`: Deny 後 exact `permission_denied`、restore 後 clean reopen callbacks=39。
- `midi_device_loss_observation=Pass`: before=True/exit0、during=False/exit0。after=False/exit0 は限界 evidence であり reappearance を Pass にしていない。
- `device_loss_audio=Unsupported`: `invalidated=0`・`control_error_kind=0`・`discontinuities=0`、recreate 後 reopen は `no input devices on this host`。
- `xrun_probe=Unsupported`: callbacks=423 で xrun/discontinuity/invalidated/error すべて 0。

attempt5 を capability matrix §8/§12 の最終 fault evidence とする。attempt4 は同一 SHA で fault result が一致した confirmation attempt かつ os-endpoint timestamp flake の記録として残す。

## capability matrix / OQ / xrun 結論

- `docs/designs/issue-629-provisioning-runbook.md` §8 は採用経路 `RvfVirtualAudio` + `RvfMidi` の実測 matrix へ更新した。COTS/SysVAD 表は歴史・比較資料と明記し、SysVAD の ✅ は upstream sample の理論上の可能性であって採用 driver の実測ではないことを明示した。
- OQ#1/#2/#4 は採用経路では NotApplicable、#3 は WMS absent 実測済みの NotApplicable、#5/#7 は採用 lane 限定の実測解消、#6 は permission denial/restore の Pass で実測解消とした。
- xrun は必要と判断し、driver 注入面だけでなく adapter の Xrun 観測契約も必要であることを記録した。実装は `scarecrow173/ruviform#1009` へ分離し、OS-L1 deterministic backend fault は EXT-L3 xrun Pass の代用にしない。

## 残課題

- merge 後 main ref dispatch は Coordinator の統合検証範囲。
- docs index は生成物であり本 Worker では編集しない。
