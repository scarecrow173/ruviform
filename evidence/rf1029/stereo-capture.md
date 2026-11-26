# rf#1029 stereo capture evidence

## 対象と実行条件

- 対象 Issue: ruviform `#1029`（virtual audio endpoint の stereo capture channel-width 欠落）
- branch: `fix/1029-stereo-capture`
- 実装対象 SHA: `53ebccd3419d823ecf5ef89806e6b667e3655e6e`
- rf#1009 統合済み起点 SHA: `527ddd7c1eb97377b12aee44772dfeafe6cc40c9`（PR `#1030` merge、xrun harness/workflow 統合済み）
- local OS: Microsoft Windows 11 Pro 10.0.26300（build 26300）
- local tool: rustc 1.99.1、mise 2026.11.2
- local 実行日時: 2026-11-26T12:12:21Z 前後
- 実施者: Orca Worker `orca_session_id:50868f79-8595-40d3-bbe6-508f390d54a8`

## 層別診断表

| 層 | 観測事実 | 判定 | 根拠 |
| --- | --- | --- | --- |
| driver format range | 変更前 `MICIN_DEVICE_MAX_CHANNELS=1`、supported device format はすべて mono、`MicInPinDataRangesStream` も mono range のみ。`IsFormatSupported` は pin format exact-match、`DataRangeIntersection` は channel count exact-match。 | **direct cause** | capture miniport capability 自体が stereo format を公開していなかった。 |
| topology | `MicInJackDesc.ChannelConfig` が `KSAUDIO_SPEAKER_MONO`。 | **寄与** | stream format 決定の主因ではないが、mono capability を示す metadata で stereo contract と矛盾。 |
| streaming copy | `RvfLoopbackCopyToCapture` は dst mono / src multi-channel の場合だけ arithmetic mean downmix。stereo dst には channel-mapped copy が既にある。 | **direct cause ではない** | mono negotiation の結果として downmix が使われていた。stereo capture が開ければ copy 層は左右を保持できる構造。 |
| endpoint negotiation | default/mix capture format は mono。shared-mode `IsFormatSupported` stereo success は OS conversion の受入であって content preserve の証拠ではない。既存 harness は default F32 優先で stereo 明示要求がなく、oracle も channel 0 のみ。 | **寄与** | mono capability を mono stream として表面化し、既存 oracle が silent Pass になり得た。 |

## 決定した contract

決定: **stereo preserve**。

- `Ruviform Cable Input` stereo render の独立 left/right content は、`Ruviform Cable Output` stereo capture の対応 channel で失われてはならない。
- mono format は明示的 mono consumer 向けに残すが、stereo preserve contract の成功ではない。stereo render を mono capture で開いた場合だけ arithmetic mean downmix を許す。
- stereo capture で mono copy/downmix/channel swap が観測された場合は Fail。
- endpoint 未 provisioning は NotRun。stereo/mono 以外の width や非 finite 測定など oracle が分類できない観測形状だけを Unsupported とする。

却下: mono/downmix 正式契約。Synvelis consumer は stereo input を要求し、driver/harness の bounded 変更で stereo preserve を実現できる。shared-mode stereo `IsFormatSupported` を証拠化する案も却下（conversion success と content preserve は別）。

## 実装内容

- `micinwavtable.h`: capture max channels を 2 へ、mono format 群を保持したまま stereo 16-bit PCM format 群（8 kHz–48 kHz）を追加。default/raw mode は stereo 48 kHz。speech/communications/far-field mode default も stereo。streaming pin data range を mono/stereo に分離。
- `micintoptable.h`: jack channel mask を stereo 化。
- `PROVENANCE.md`: upstream との差分記録を同期。
- `virtual_endpoint.rs`: exact Ruviform pair を選ぶ `ruviform_channel_width_contract` runtime case、channel-major bounded capture buffer、独立 left/right stimulus、tone-ratio oracle、machine-readable JSON `rf1029-channel-width-observation/v1`、endpoint 不要の deterministic `channel_width` tests を追加。
- xrun injection semantic、fault case、generic round-trip oracle、consumer oracle、workflow/tool script は変更していない。

## canonical local verification

| lane | command | exit | verdict | 要点 |
| --- | --- | ---: | --- | --- |
| fmt | `mise run fmt-check` | 0 | Pass | `cargo fmt --all -- --check` 成功。最終差分後も再実行済み。 |
| check | `mise run check ruviform-audio-device` | 0 | Pass | `cargo check --locked --all-targets -p ruviform-audio-device` 成功。 |
| lint | `mise run lint ruviform-audio-device` | 0 | Pass | clippy 成功。 |
| test | `mise run test ruviform-audio-device` | 0 | Pass | unit test 該当 0、doc-test 6 passed。 |
| real-device lint | `mise run lint-audio-real-input` | 0 | Pass | `cargo clippy -p ruviform-audio-device --all-targets --features real-device -- -D warnings` 成功。最終差分後に再実行済み。 |
| deterministic channel_width | `mise run test ruviform-audio-device --features real-device --test virtual_endpoint channel_width -- -- --test-threads=1` | 0 | Pass | 5 passed / 1 ignored / 15 filtered。ignored は runtime case で deterministic evidence ではない。 |
| local virtual endpoint runtime | `mise run test-audio-virtual-endpoint` | 0 | **NotRun**（stereo contract runtime evidence として） | 7 tests は harness 契約上 pass 終了したが、host に provisioned Ruviform pair がなく `ruviform_channel_width_contract` は `NotRun: ruviform-cable-output-capture-candidate-missing`。runtime stereo preserve evidence ではない。machine-readable copy: `local-channel-width-notrun.json`。 |
| driver build | `mise run build-vaudio-driver` | 1 | **NotRun** | MSBuild は存在するが `WindowsKernelModeDriver10.0` toolset がなく MSB8020。WDK/MSBuild driver build 環境不足であり、driver code の compile Pass/Fail evidence ではない。 |
| prek | `mise exec -- prek run --all-files` | 0 | Pass | trim whitespace/end-of-file/YAML/TOML/line-ending/BOM/private-key/actionlint が Pass または skipped。lease 外差分は発生していない。 |

補足: task spec に示された単一 `--` 区切りの `channel_width` コマンドは、この mise `raw_args` task では区切りが消えて cargo が `--test-threads` を cargo 引数と解釈し exit 1 になった。同じ cargo コマンドを成立させるため、mise 経由では `-- -- --test-threads=1` を使い、実際の cargo invocation が `... channel_width -- --test-threads=1` であることを出力で確認した。filter は `channel_width` で zero-match ではない（5 passed）。

## provisioned Windows workflow evidence

状態: **Pass**（stereo preserve runtime evidence あり）。

- workflow: `virtual-audio-driver-probe.yml`（既存 manual lane。workflow file は未変更）
- run id / attempt: `45817964602` / `1`
- URL: https://github.com/scarecrow173/ruviform/actions/runs/45817964602
- head SHA: `65572966ed0ca301ba3a76dbbe35977ea856f9c2`（実装 SHA `53ebccd3419d823ecf5ef89806e6b667e3655e6e` + evidence 記録 commit。code/driver/harness は実装 SHA と同一）
- runner OS: windows-2025 / Microsoft Windows 10.0.26100.33828
- artifact: `virtual-audio-driver-probe-windows-2025-attempt-1`、upload 成功、download/検収済み。欠損なし。
- driver build: workflow 内 `driver-build` stage `ok`、exit 0、`x64\Release\RvfVirtualAudio.sys`
- package hash: `RvfVirtualAudio.sys` sha256 `50cf5844109c16573e280374fd4ca1cdd68bf803a3208a0adf3840847d915deb`、`rvfvirtualaudio.cat` sha256 `25aa67221e880a8542eadb6ae6418f7d33a0b898a79e407ad7a01deb608b930c`、INF/cert hash は `provisioned-artifact-hashes.json`
- endpoint: render `Ruviform Cable Input (Ruviform Virtual Audio Device)` id `{0.0.0.00000000}.{d582bf43-fd3f-460e-b55f-e3cc68381c43}`、capture `Ruviform Cable Output (Ruviform Virtual Audio Device)` id `{0.0.1.00000000}.{cc912312-fed6-4655-96b2-6ed4ee361273}`
- endpoint probe mix format: render/capture とも `ch=2 rate=48000Hz bits=32`。`IsFormatSupported(shared)` は stereo f32/s16 も supported だが、Pass 判定には使用していない。
- channel-width runtime negotiation: render/capture とも実 opened stream format `F32/Stereo/48000Hz`。
- terminal verdict: `rf1029_channel_width_observation` `result=Pass`、`mode=stereo-preserve`、`detail=independent-left-right-content-preserved`。captured frames 157920。tone ratio は left 440Hz `0.9571676121756445` / left 880Hz `0.0001237105073796443` / right 440Hz `0.000000026314477805968456` / right 880Hz `0.9962726418908657` で、downmix・mono copy・channel swap ではない。machine-readable copy: `provisioned-channel-width-pass.json`。
- stream lane summary: `test-audio-virtual-endpoint` exit 0、7 passed。workflow stage detail が `verdict=no-verdict-line` なのは cargo test capture と既存 verdict 行抽出の制限であり、raw stderr に残した上記 JSON を直接証拠として使う。
- cleanup/unprovision: stage `cleanup` `ok`、`leftovers=0 errors=0`、driver store/cert/consent/service cleanup 成功。

Adjacent observations（stereo preserve の判定材料にはしない）: `os-endpoint-wiring` audio metadata は `output timestamp regressions: 2` で fail（PROVENANCE に記録済みの audiodg epoch bookkeeping 由来の既知観測であり、channel-width oracle とは別契約）。rf#1009 xrun cases は endpoint pair missing preflight で NotRun。これらは本 Issue の stereo channel-width Pass evidence へ転用しない。

## evidence の意味境界

| evidence | 意味 | stereo preserve を直接支えるか |
| --- | --- | --- |
| deterministic `channel_width` tests | oracle/schema が stereo preserve・mono capture・downmix・mono copy・channel swap・Unsupported を区別すること。endpoint runtime ではない。 | 间接（oracle correctness のみ） |
| static driver source diff | capture capability/default format/topology metadata が stereo を公開する構成であること。compile/runtime ではない。 | 间接（実装構成のみ） |
| local `test-audio-virtual-endpoint` | endpoint 不存在 host で NotRun を正直に出すこと。 | 否 |
| local `build-vaudio-driver` | この host では WDK toolset 不足で NotRun。 | 否 |
| provisioned workflow runtime | provisioned Windows endpoint で stereo format negotiation と独立 left/right content を観測する主証拠。run `45817964602` で Pass。 | **直接** |

## Synvelis#115 再評価判断

Synvelis run `45797309944` attempt `1` の `multi-track stereo split requires >=2 channels (in=1 out=2)` は、変更前 Ruviform capture endpoint の mono capability と一致する正当な consumer Fail である。本変更は capture endpoint が stereo format を公開し、stereo→stereo loopback copy で独立内容を保持する契約へ修正するため、provisioned evidence は Pass になったため、Synvelis#115 を再 run する意味がある。Synvelis 側 consumer oracle/harness/workflow は変更していない。Synvelis Pass は本 Issue の成果ではなく、default branch 対象 SHA での canonical EXT-L3 consumer run は後続 gate とする。

## 受入条件対応表

| 受入条件 | verdict | 根拠 |
| --- | --- | --- |
| mono の直接原因を層別に記録 | Pass | 上記 layer matrix と ADR。driver format range が direct cause。 |
| channel-width contract を ADR に記録 | Pass | `docs/adr/issue-1029-virtual-audio-channel-width-contract.md`。stereo preserve/mono downmix/Unsupported-NotRun を区別。 |
| stereo preserve の provisioned Windows evidence | Pass | run `45817964602` attempt 1。stereo opened format + independent left/right tone separation を観測。 |
| mono/downmix formal contract の場合の oracle | NotApplicable | stereo preserve を選択した。ただし mono capture が silent Pass にならない deterministic oracle は追加済み。 |
| Synvelis#115 再評価判断 evidence | Pass（判断材料） | consumer Fail 原因と再評価条件を記録。Synvelis 再 run は成果にしない。 |
