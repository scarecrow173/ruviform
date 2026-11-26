# rf1009 xrun harness/tool/workflow implementation evidence

## 対象と実施条件

- 対象 slice: harness/tool/workflow implementation
- 基底 SHA: `71f8bd4da047699eeb3ead781ff19e8a21320816`（PR #1027 までを含む main。本 evidence はこの SHA 上の本 slice working tree diff に対する検証である）
- OS: Microsoft Windows 11 Pro 10.0.26300
- 実行日時: 2026-11-26 JST
- 実施者: Orca Worker `orca_session_id:fd0cd5c5-82ef-4cec-b7f3-3a7cc6b76647`
- 対象 endpoint/pin: Ruviform Cable Input / Ruviform Cable Output pair のうち、Ruviform Cable Output を構成する capture streaming pin（`eMicInDevice` + `KSPIN_WAVEIN_HOST` + `SystemCapturePin`）。case JSON の `target_pin` は設計どおり `loopback-capture` だが、speaker filter の OS loopback pin、host capture pin 全般、MIDI path を意味しない

この文書は本 slice の implementation evidence であり、provisioned runtime evidence、capability matrix/runbook 更新、formal proof ではない。

## 実装内容

- `crates/ruviform-audio-device/tests/virtual_endpoint.rs` に `capture-overflow-drop-input` positive case と `capture-content-gap-input-negative` negative control を追加した。case は controller armed 時だけ測定し、unarmed canonical lane では endpoint を開かず `NotRun` observation を出す
- harness は typed `DeviceError::Xrun`、`ClockDiscontinuity`、stream invalidation、unexpected backend error、input/output callback 継続、forward frame position gap、backward/reset の区別、max forward gap、callback stall、channel-0 PCM の streaming interior zero-run を観測する。sample gap は error/flag/notification ではなく capture PCM の独立観測である
- harness は `rf1009_xrun_observation:` 1 行と result file に `rf1009-xrun-observation/v1` JSON を出力する。run id、artifact path、cleanup state は捏造せず、最終 `rf1009-xrun-case/v1` object への provenance 付与は controller 側に残した
- 新規 `tools/virtual-audio-driver/Invoke-XrunCases.ps1` は scenario/start/count validation、preflight、registry write 前後の記録、stream close/open 境界、harness process の bounded wait、raw stdout/stderr の分離保存、registry restore/cleanup、harness JSON parse、schema aggregate、validate-only self-test を実装する。registry path は `HKLM:\SYSTEM\CurrentControlSet\Services\RvfVirtualAudio\Parameters`、値は `RvfXrunScenario` / `RvfXrunStartPacket` / `RvfXrunPacketCount` の `REG_DWORD` で、finally 相当で削除または元の型/値へ戻す
- `.github/workflows/virtual-audio-driver-probe.yml` に additive な `xrun-cases` stage を fault-cases 後、cleanup 前に追加した。trigger、permissions、required gate 非接続、absence-is-data、既存 Provision/Unprovision/fault contract は変更していない。artifact upload path に `xrun-cases-<os>/` を追加した

## 正規ローカル検証

| コマンド | exit code | verdict | 要点 |
| --- | ---: | --- | --- |
| `mise run fmt-check` | 0 | Pass | `cargo fmt --all -- --check` 完了 |
| `mise run check ruviform-audio-device` | 0 | Pass | `cargo check --locked --all-targets -p ruviform-audio-device` 完了 |
| `mise run lint ruviform-audio-device` | 0 | Pass | clippy `-D warnings` 完了 |
| `mise run test ruviform-audio-device` | 0 | Pass | unit 84、mock backend 4、mock input backend 12、doctest 6 が pass。`virtual_endpoint` target は real-device feature なしでは 0 test |
| `mise run lint-audio-real-input` | 0 | Pass | `cargo clippy -p ruviform-audio-device --all-targets --features real-device -- -D warnings` 完了 |
| `mise run test -- ruviform-audio-device --features real-device --test virtual_endpoint xrun -- --test-threads=1` | 0 | Pass（harness-level） | xrun filter で 11 test が収集され、deterministic oracle/schema test 8件 pass、controller/provisioning が必要な 3件は ignored。`mise` の `raw_args` task では task 名直後の `--` が mise 側 separator として必要であり、表記どおり separator なしで実行した補助確認は cargo へ `--test-threads` が届かず exit 1 だった。したがって実測 Pass は cargo へ同じ引数が届く上記 invocation の結果である |
| `mise run build-vaudio-driver` | 1 | NotRun | MSBuild は解決されたが、`WindowsKernelModeDriver10.0` platform toolset 不在で MSB8020。WDK/toolset 未装備 host の環境前提未充足であり、driver 製品欠陥の Fail や build Pass ではない |
| `mise run test-ci-daily` | 0 | Pass | node contract test 113件 pass |
| `mise run test-ci-tracker` | 0 | Pass | node contract test 108件 pass |
| `mise exec -- actionlint .github/workflows/virtual-audio-driver-probe.yml` | 0 | Pass（static） | workflow 静的検査 pass。workflow は実行していない |
| `mise exec -- prek run --all-files` | 0 | Pass | trailing whitespace、EOF、YAML/TOML、mixed line ending、UTF-8 BOM、private key、actionlint hooks が pass。lease 外ファイルの変更は発生しなかった |

## harness-level deterministic oracle/schema test

実施 lane: `mise run test -- ruviform-audio-device --features real-device --test virtual_endpoint xrun -- --test-threads=1`

収集された xrun 名を含む test は 11件で、実行された deterministic test は次の 8件である。

- `xrun_positive_oracle_accepts_required_synthesis`
- `xrun_positive_oracle_rejects_missing_required_counters`
- `xrun_positive_oracle_rejects_invalidation_and_unexpected_backend_error`
- `xrun_negative_oracle_accepts_content_gap_only`
- `xrun_negative_oracle_rejects_typed_xrun_or_clock_discontinuity`
- `xrun_setup_failures_are_notrun`
- `xrun_json_writer_preserves_schema_types_and_escapes_strings`
- `xrun_gap_stall_and_sample_gap_helpers_classify_units`

これらは endpoint を使わない合成観測・分類・JSON writer/schema の検査であり、provisioned endpoint runtime evidence ではない。`virtual_audio_xrun_capture_overflow_drop_input`、`virtual_audio_xrun_capture_content_gap_input_negative`、既存 `virtual_audio_xrun_probe` の 3件は ignored であり、実 endpoint での Pass/Fail evidence を意味しない。

## PowerShell/workflow static validation と local controller

補助証拠（canonical = mise task ではない）として、PowerShell parser を `mise exec -- pwsh` 経由で実行し、`Invoke-XrunCases.ps1` の parse error は 0 だった。

`-ValidateOnly` self-test も pass した。確認した語彙・境界は次のとおりであり、生成した sample は runtime evidence ではない。

- valid scenario 1/2、unknown scenario 99
- start packet 4096 は valid、4097 は invalid
- packet count 1/16 は valid、0/17 は invalid
- final schema validation は 0 problem
- cleanup state は `cleared` / `clear_failed` / `not_applicable` の語彙に収まる

未 provisioned local host では次を実行した。

```text
mise exec -- pwsh -NoProfile -ExecutionPolicy Bypass -File tools/virtual-audio-driver/Invoke-XrunCases.ps1 -Os windows-local -OutputDir /tmp/rf1009-xrun-local-final
```

PowerShell 側で `/tmp` は `C:\Users\akiyoshi\AppData\Local\Temp\rf1009-xrun-local-final` に解決された。controller exit code は 0、endpoint probe exit code は 0 だったが、preflight は `administrator-required,driver-service-key-missing,ruviform-cable-endpoint-pair-missing` で不成立だった。registry write は行われず、positive/negative 両 case の `rf1009-xrun-case/v1` JSON は schema problem 0 の `NotRun`、stage record は `not_run` だった。生成物は repo 外の一時領域にあり、commit していない。

## 意味境界

| evidence 種別 | 本 slice の結果 | 意味 |
| --- | --- | --- |
| harness-level deterministic Pass | Pass（8件） | oracle/schema/分類の合成検査が pass。endpoint 実走や driver injection 成立を示さない |
| static validation Pass | Pass | PowerShell syntax、self-test schema/validation、actionlint、prek が pass。workflow 実行や artifact 採取を示さない |
| driver build lane verdict | NotRun | MSBuild はあるが WindowsKernelModeDriver10.0 toolset/WDK が不足。compile/link 結果ではない |
| provisioned runtime evidence | NotRun | provisioned Windows runner、workflow_dispatch 実走、run id/artifact/case result 採取は未実施。後続 gate |

## 実施しなかったもの

- provisioned Windows runner での `virtual-audio-driver-probe.yml` 実走、run id・artifact・case result 採取: NotRun。後続 gate
- capability matrix/runbook の xrun 行更新: NotRun。provisioned `rf1009-xrun-case/v1` artifact がないため実測更新しない
- WDK 装備環境での driver build/install/sign/provisioning runtime 確認: NotRun
- output render underrun investigation/implementation: NotRun。本 slice では input capture case のみ実装し、output case は追加していない
- 設計文書、capability matrix/runbook、docs index の変更: 実施しない。docs index は Coordinator が統合時に整備する
- real GUI 検証: NotApplicable。本 slice は headless harness/controller/workflow 実装であり GUI surface を変更していない
