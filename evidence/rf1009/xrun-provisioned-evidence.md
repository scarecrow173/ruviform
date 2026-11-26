# rf1009 deterministic xrun provisioned evidence

## 対象と実施条件

- 対象 Issue: `scarecrow173/ruviform#1009`
- branch: `test/1009-xrun-provisioned-evidence`
- 作業開始時の main SHA: `7246595a93fc68d66383e430479a412f7642dd0f`
- provisioned workflow run: `45837299116` attempt `1`
- run URL: https://github.com/scarecrow173/ruviform/actions/runs/45837299116
- 測定対象 head SHA: `ad188873144e93a3fd384fca0a64b2fe0f49ce16`
- workflow file/ref: `.github/workflows/virtual-audio-driver-probe.yml` @ `test/1009-xrun-provisioned-evidence`
- runner OS: GitHub-hosted `windows-2025` x64（endpoint probe 観測 OS `10.0.26100.33828`）
- workflow 実施時刻: 2026-11-26T14:16:15Z 開始、2026-11-26T14:23:14Z 完了、job `vaudio-probe` は 6m51s
- artifact download 時刻: 2026-11-26T14:23:32Z
- 実施者/Worker: Orca Worker `orca_session_id:d9d17511-e570-495c-b0ca-56ae0f33d853`
- 対象 endpoint/pin: `Ruviform Cable Input (Ruviform Virtual Audio Device)` / `Ruviform Cable Output (Ruviform Virtual Audio Device)` pair のうち、Ruviform Cable Output を構成する capture streaming pin（`eMicInDevice` + `KSPIN_WAVEIN_HOST` + PinType `SystemCapturePin`）。case JSON の `target_pin=loopback-capture` は設計どおり virtual-cable capture pin を指し、speaker filter の OS loopback pin、host capture pin 全般、MIDI path を意味しない

この文書は provisioned runtime evidence と capability matrix 更新根拠である。workflow/job conclusion を case-level Pass と混同しない。本 run の workflow conclusion は `success` だったが、`xrun-cases` stage record は `error` で、positive/negative case はどちらも `Fail` だった。

## rf#1029 run の解釈と今回の workflow 修正

rf#1029 provisioned run `45817964602` attempt `1`（branch `fix/1029-stereo-capture`、head SHA `65572966ed0ca301ba3a76dbbe35977ea856f9c2`）は stereo preserve の直接 evidence ではあるが、rf#1009 positive/negative xrun を測っていない。adjacent observation では xrun cases が endpoint pair missing preflight に留まり NotRun であったため、これを xrun capability evidence には使わない。

現行 workflow は当初 `fault-cases` 後・`cleanup` 前に `xrun-cases` を置いていた。`fault-cases` は devnode remove/recreate fallback を行い得る destructive stage であり、rf#1002 run `45575005948` 系 evidence では recreate 後の audio/MIDI endpoint surface が bounded window に戻らない観測があった。rf#1029 artifact/log とこの順序を照合すると、xrun preflight missing は deterministic injection の oracle 観測ではなく、destructive fault stage 後の endpoint surface 不安定に汚染された測定配置だったと判断した。

そこで本 slice では `.github/workflows/virtual-audio-driver-probe.yml` の `xrun-cases` を `os-endpoint-wiring` / `os-endpoint-midi-e2e` 後、`fault-cases` 前へ移す最小修正を行い、design/spec runbook の stage 一覧だけを同じ順序へ同期した。trigger、permissions、manual lane、required gate 非接続、absence-is-data、`if: always()` cleanup/upload、attempt 固有 artifact 名、fault-cases contract、xrun oracle、endpoint exact match、Provision/Unprovision、cleanup は変更していない。semantics を弱める修正ではなく、destructive fault injection から測定を隔離する順序修正である。

この判断は run `45837299116` で裏付けられた。xrun-cases は destructive `fault-cases` より前に実行され、preflight endpoint probe exit=0、exact pair 行あり、`preflight ok=True problems=` で endpoint missing にはならなかった。したがって rf#1029 の NotRun を capability Unsupported や deterministic injection Fail と混同しない一方、今回の case-level Fail は preflight missing ではなく provisioned runtime での oracle 不成立である。

## canonical local verification

これらは workflow dispatch 前に branch 上で実行したローカル検証である。ローカル host は provisioned Ruviform pair を持たないため、quick exit や process exit 0 を provisioned runtime Pass とみなしていない。

| コマンド | exit code | verdict | 要点 |
| --- | ---: | --- | --- |
| `mise run fmt-check` | 0 | Pass | rustfmt check 完了 |
| `mise run check ruviform-audio-device` | 0 | Pass | `cargo check --locked --all-targets` 完了 |
| `mise run lint ruviform-audio-device` | 0 | Pass | clippy `-D warnings` 完了 |
| `mise run test ruviform-audio-device` | 0 | Pass | 対象 crate test pass |
| `mise run lint-audio-real-input` | 0 | Pass | real-device feature 配下 compile/lint。実機・provisioned endpoint 実行ではない |
| `mise run test -- ruviform-audio-device --features real-device --test virtual_endpoint xrun -- --test-threads=1` | 0 | Pass（harness-level） | xrun filter は zero-match ではなく、endpoint を使わない deterministic oracle/schema/selection test 9件 pass、endpoint-dependent 3件 ignored。cargo 側へ `--test-threads=1` が届く invocation で実施 |
| `mise run test-audio-virtual-endpoint` | 0 | NotRun（runtime evidence ではない） | local host は未 provisioned。ignored/NotRun 観測を Pass と読まない |
| `mise run build-vaudio-driver` | 1 | NotRun | MSBuild は存在するが `WindowsKernelModeDriver10.0` / WDK toolset 不在で MSB8020。環境前提未充足であり driver 製品 Pass/Fail ではない |
| `mise run test-ci-daily` | 0 | Pass | CI daily contract test pass |
| `mise run test-ci-tracker` | 0 | Pass | CI tracker contract test pass |
| `mise exec -- actionlint .github/workflows/virtual-audio-driver-probe.yml` | 0 | Pass（static） | workflow 静的検査。workflow 実行ではない |
| `mise exec -- prek run --all-files` | 0 | Pass | hooks pass。lease 外差分は commit していない |

補助証拠（canonical = mise ではない）として PowerShell parser と `Invoke-XrunCases.ps1 -ValidateOnly` を実行し、scenario validation、final schema validation、cleanup vocabulary は pass だった。生成される sample validation output は static-only であり runtime evidence ではない。

## provisioned workflow/provisioning/cleanup result

artifact `virtual-audio-driver-probe-windows-2025-attempt-1`（artifact id `15125384193`、size 154359 bytes）を download して検収した。workflow conclusion は `success`、job `vaudio-probe (windows-2025)` も success だが、これは stage/case の合否ではない。

主要 stage record:

| stage | status | 要点 |
| --- | --- | --- |
| `driver-build` | ok | exit 0、x64 Release `RvfVirtualAudio.sys` 生成 |
| `catalog-create` / `cert-create` / `store-register` | ok | test certificate thumbprint `13C88590E8D0540F6ECE92AAC9B6292693A32AF5`、Root+TrustedPublisher 登録 |
| `sign` / `sign-verify` | ok | `RvfVirtualAudio.sys`・`rvfvirtualaudio.cat` 署名/verify |
| `pnputil-add-driver` | ok | `oem12.inf` として publish |
| `devnode-create` | ok | instance `ROOT\MEDIA\0000`、status OK、problem 0、service `RvfVirtualAudio` |
| `endpoint-probe` | ok | endpoints enumerated 2、virtual candidates 2、pair_count 1 |
| `stream-roundtrip` / `midi-loopback` / `os-endpoint-wiring` / `os-endpoint-midi-e2e` | ok | 既存 endpoint/MIDI lane は完走 |
| `xrun-cases` | error | `capture-overflow-drop-input=Fail,capture-content-gap-input-negative=Fail; harness_problems=0` |
| `fault-cases` | ok | measurement harness 完走。xrun capability 判定には使わない |
| `cleanup` | ok | primitive status ok、detail OK、leftovers 0、errors 0 |

endpoint probe では render endpoint `Ruviform Cable Input (Ruviform Virtual Audio Device)` id `{0.0.0.00000000}.{89e661d0-8797-40aa-90a7-9c625fa7c022}`、capture endpoint `Ruviform Cable Output (Ruviform Virtual Audio Device)` id `{0.0.1.00000000}.{4d2bdac2-c69d-4d6e-856e-948ddc4b6d06}` が active だった。両者とも mix format は extensible 2ch/48kHz/32bit で、f32/2ch/48k、f32/2ch/44.1k、s16/2ch/48k が shared mode supported と記録された。production port view でも同じ exact pair が default output/input として観測された。

## xrun case 検収

controller log では preflight cargo/endpoint probe が成立し、positive/negative それぞれで registry write 前は 3値 absent、write 後は `RvfXrunScenario`/`RvfXrunStartPacket`/`RvfXrunPacketCount` が REG_DWORD で期待値、harness process は write 後に起動され、process 終了後に cleanup され、cleanup 後は 3値 absent だった。`applied=true` は controller が registry write/readback を確認した意味であり、driver が実際に scenario を適用したことの直接証明ではない。schema validation problem は両 case とも 0 で、aggregate harness_problems も 0 だった。

### positive: `capture-overflow-drop-input`

- schema: `rf1009-xrun-case/v1`
- result: **Fail**
- detail: `positive-oracle-not-met:typed-xrun-missing;clock-discontinuity-missing;forward-frame-position-gap-missing;max-forward-gap-zero;input-callbacks-did-not-progress-before-injection;input-callbacks-did-not-resume-after-injection`
- run provenance: id `45837299116`、attempt `1`、repo `scarecrow173/ruviform`、SHA `ad188873144e93a3fd384fca0a64b2fe0f49ce16`、OS `windows-2025`、recorded `2026-11-26T14:21:29Z`
- injection surface: `static-registry-stream-snapshot`、scenario `capture-overflow-drop`、start packet 20、packet count 3、applied true、cleanup `cleared`、reload boundary `stream-open`
- oracle: typed xruns 0、clock discontinuities 0、stream invalidated 0、backend errors 0、input callbacks 299、output callbacks 302、forward frame-position gaps 0、max forward gap 0、sample gap false、max callback stall 20ms
- raw artifact: stdout `xrun-capture-overflow-drop-input-windows-2025.stdout.txt` sha256 `81b82e9d44e3e864f1196dc1ef634ce3866b64ddcb21ebd82e065c8e979c6d6e`、stderr、controller log は artifact inventory 参照
- raw stdout には同じ oracle counters を持つ `rf1009_xrun_observation` 行があり、controller aggregate と一致した

判定理由: endpoint missing preflight や harness crash ではなく、exact pair、registry write/readback、stream open/start、callback 継続、schema/provenance/cleanup が成立した状態で必須 oracle（typed Xrun、ClockDiscontinuity、forward frame-position gap、gap 後 callback resume 条件）が成立しなかった。したがって NotRun ではなく Fail である。

### negative control: `capture-content-gap-input-negative`

- schema: `rf1009-xrun-case/v1`
- result: **Fail**
- detail: `negative-control-oracle-not-met:sample-gap-missing`
- run provenance: id `45837299116`、attempt `1`、repo `scarecrow173/ruviform`、SHA `ad188873144e93a3fd384fca0a64b2fe0f49ce16`、OS `windows-2025`、recorded `2026-11-26T14:21:33Z`
- injection surface: `static-registry-stream-snapshot`、scenario `capture-content-gap`、start packet 20、packet count 3、applied true、cleanup `cleared`、reload boundary `stream-open`
- oracle: typed xruns 0、clock discontinuities 0、stream invalidated 0、backend errors 0、input callbacks 299、output callbacks 302、forward frame-position gaps 0、max forward gap 0、sample gap false、max callback stall 20ms
- raw artifact: stdout `xrun-capture-content-gap-input-negative-windows-2025.stdout.txt` sha256 `fae1be52118a220172201883b264be04de887dede24c2e5d01d3b6d133b6f7ed`、stderr、controller log は artifact inventory 参照
- raw stdout には同じ oracle counters を持つ `rf1009_xrun_observation` 行があり、controller aggregate と一致した

判定理由: typed Xrun/discontinuity/forward gap の false positive は 0 であり、この点では negative control の false-positive 条件は発生していない。しかし scenario 2 の必須独立 oracle である zero-filled capture PCM の interior sample gap が観測されず、negative control 自体が成立しなかった。negative を省略したり、typed xrun false positive がないことだけを Pass に読み替えたりしない。

positive/negative の counters が一致する点、negative で content gap も観測されない点は、registry write/readback は成立した一方で driver 側 scenario 適用、対象 pin 判定、または adapter 観測契約のどこかで deterministic injection が観測可能な形へ繋がっていないことを示す。driver/adapter 本体は本 slice の lease 外なので本 PR では修正せず、観測された case-level Fail として記録する。sample gap、callback stall、workflow green、job success を typed Xrun の代替 evidence には使わない。

## artifact inventory / provenance

committed machine-readable copy は `evidence/rf1009/provisioned/run45837299116-attempt1/` にある。生ログ全文、driver package binary、certificate、巨大な provisioning log は commit せず、download artifact 内の全 104 file を `artifact-inventory.json` で size/sha256 付きに記録した。`copied_path` が non-null の file だけを repo へ copy した。

主要 hash:

| artifact file | sha256 |
| --- | --- |
| `vaudio-pkg/RvfVirtualAudio.sys` | `ca65edbcee581978d1b3707afcff4be7e379200589e86bb36f3cb393224b4e99` |
| `vaudio-pkg/rvfvirtualaudio.cat` | `1e931179eab432037cc545a100b07abbff0c4b65f07657c23ab22fd5a4652868` |
| `vaudio-pkg/RvfVirtualAudio.inf` | `5da93e51d9c7d772286df3287efe2f8e79b0385cbe558382597a1699f85c31b3` |
| positive final case JSON | `a89fd3c135643e2fe7704c7758f73874bbab9f61b3afbbb2306da26360093844` |
| negative final case JSON | `dc59f6b907fbe991c490120b18af5f05bf6cd27bcf9ab4f02163168e88c6e841` |
| aggregate case JSON | `a3d22435ee3e0406343d53036aa4f05ff504955ce907898b2701221986486e48` |
| xrun controller log | `b86ccb28a2d08434e058b8b2245876f3f9052a2de40e6aaee293b96e078747db` |
| xrun endpoint probe output | `34c299825e8cfe2e0d99b5c586d64966674d9615df442f7b8bf7c6d4a3fcd477` |
| workflow stage record JSON | `812a3c6434137df22e65853126d5ca817975d077715d872420935c7d9e0b8146` |
| unprovision state JSON | `a6ceabdfd4a3ff9c94093a3c16271a8cafee46bb04d4bc1ccccfb99440ae3e8e` |

case JSON の `run.id`/`run.attempt`/`run.repo`/`run.sha`/`run.os` は実 run と一致した。測定対象コードの SHA は `ad188873144e93a3fd384fca0a64b2fe0f49ce16` であり、evidence/docs/matrix を追加する後続 commit で最終 PR head SHA は変わる。後続 commit は evidence/docs のみであり、run SHA を最終 head と偽らない。

## cleanup/unprovision

xrun registry cleanup は positive/negative とも `cleared` であり、cleanup 後に `RvfXrunScenario`・`RvfXrunStartPacket`・`RvfXrunPacketCount` は absent だった。case 切替は harness process 終了後に registry cleanup し、次 case の write 後に新しい harness process を起動する stream close/open 境界だった。`clear_failed` はない。

workflow cleanup/unprovision stage は `ok`、primitive status `ok`、detail `OK`、leftovers 0、errors 0 だった。`vaudio-unprovision-state.json` では devnode、driver-store、cert、consent、service、residue-check の各 stage がすべて `ok`、residue-check は `leftovers=0`、errors は空だった。cleanup 失敗を成功として扱っていない。

## output render underrun と意味境界

output render underrun investigation/implementation は本 slice の対象外であり、現行契約では typed output xrun の観測経路がないため **Unsupported** と記録する。input capture overflow/content-gap evidence、loopback PCM、driver 内部 glitch、workflow green を output underrun evidence へ転用しない。

| evidence 種別 | 今回の結果 | 意味 |
| --- | --- | --- |
| harness-level deterministic Pass | Pass（local 9件） | oracle/schema/endpoint-selection の合成検査。provisioned runtime Pass ではない |
| static validation Pass | Pass | PowerShell/actionlint/prek/schema validation。workflow 実行や driver 観測ではない |
| local unprovisioned runtime | NotRun | local host に exact pair がなく、runtime capability evidence ではない |
| local driver build | NotRun | WDK/platform toolset 不在による環境前提不足。driver compile Pass/Fail ではない |
| provisioned runtime evidence | positive/negative とも Fail | exact pair、registry write/readback、cleanup、schema/provenance は成立したが、必須 oracle が不成立。capability Pass ではない |
| capability matrix 更新根拠 | run `45837299116` | deterministic injection の input capture positive/negative は Fail、output render underrun は Unsupported、旧 user-mode starvation Unsupported は歴史的 negative evidence として区別 |

## 受入条件対応

| 受入条件 | 判定 | 対応 evidence |
| --- | --- | --- |
| AC1 oracle 種類設計 | Pass（既存） | PR #1025 design。再オープンしていない |
| AC2 driver 注入面/adapter 観測契約の責務分担設計 | Pass（既存） | PR #1025 design。再オープンしていない |
| AC3 仮想 endpoint で決定的に注入した drop/gap/underrun が確定 oracle で観測できる provisioned 実走 evidence | **Fail/未達** | run `45837299116` artifact と case JSON は採取したが、positive drop は typed xrun/discontinuity/forward gap 不成立、negative gap は sample gap 不成立、output underrun は Unsupported。AC3 の Pass evidence にはならない |
| AC4 realtime path allocation-free/bounded/no floating point | Pass（既存 static review 引用） | driver/adapter 本体を変更していないため PR #1028 `evidence/rf1009/xrun-driver-injection.md` の static review を引用。今回の runtime/build はその証明の代替ではない |
| AC5 capability matrix/runbook xrun 行を実測値で更新し NotRun/Unsupported を区別 | Pass（実測更新） | `docs/designs/issue-629-provisioning-runbook.md` を run `45837299116` の positive/negative Fail、output underrun Unsupported、旧 starvation negative evidence に更新。NotRun を Pass にしていない |
| AC6 `mise run` 対象 lane pass | Pass（ローカル検証範囲） | 上記 canonical table。`build-vaudio-driver` は WDK不足で NotRun、未 provisioned virtual runtime lane は runtime Pass ではない |

## 残件

- deterministic input capture overflow/content-gap injection が adapter oracle で観測できない原因の切り分けと修正は、driver/adapter 本体を含む lease 外調査が必要な後続 slice。少なくとも registry write/readback と harness 完走だけでは driver 側 scenario 適用を証明できない
- output render underrun investigation/implementation は別 slice
- local `build-vaudio-driver` は WDK 装備環境でのみ再評価する。provisioned workflow の driver-build stage は ok だったが、local host の NotRun と混同しない
- docs index（`docs/**/README.md`）は本 slice で編集・再生成せず、Coordinator が統合時に整備する
