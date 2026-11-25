# evidence/rf979 — Slice E 仮想 endpoint lane・workflow 統合（opt-in）検証記録

Issue #979（[Issue #952](https://github.com/scarecrow173/ruviform/issues/952) Slice E）の実装記録。Slice B（#977・audio driver）と Slice C（#978・MIDI miniport）の成果物を束ねる opt-in manual lane の workflow 整合・docs 整備を行い、**branch ref への `workflow_dispatch` 実走で merge 前の実走証拠を採取する**（rf977 attempt 14-21・rf978 で確立した pre-merge dispatch パターン）。driver 実装は含まない。本ファイルは設計意図・ローカル検証結果・runner 実走の状態を記録する。

## 1. workflow 設計

### 1.1 canonical lane と trigger

- canonical lane は既存 `.github/workflows/virtual-audio-driver-probe.yml` のみ — 新規 workflow・別 lane の増設はしない（POLICIES §5「独立の重複Workflowを無条件に追加しない」）。
- `on: workflow_dispatch` のみ。required gate・`ci-summary` の `needs`・ruleset とは一切接続しない独立 manual lane（`ci-yml-contract.test.mjs` 全 workflow 走査を通過済み — §3）。
- leg は `windows-2025`（x64）のみ — ARM64 は VBS CI 下で test-signing kernel load 不可（rf961 attempt-9 実測）。leg 追加なし。

### 1.2 stage ↔ AC 対応表

AC の列: install → provisioning → `endpoint_probe`/`midi_endpoint_probe` → `tests/virtual_endpoint.rs`・`tests/loopback.rs` → evidence upload。

| AC 要素 | workflow stage | 備考 |
| --- | --- | --- |
| driver install | `env-build-surface`→`driver-build`→`catalog-create`→`cert-create`→`store-register`→`sign`→`sign-verify`→`pnputil-add-driver` | Slice B/C で実装済み・無変更 |
| provisioning（devnode 確認） | `enum-verify`→`devnode-create` | 同上 |
| `endpoint_probe` | `endpoint-probe` | `tools/virtual-audio-driver/Invoke-EndpointProbe.ps1` 経由・無変更 |
| `midi_endpoint_probe` | **`midi-endpoint-probe`（本 slice で追加）** | `tools/virtual-audio-driver/Invoke-MidiEndpointProbe.ps1`（新規）経由 — §1.3 |
| `tests/virtual_endpoint.rs` | `stream-roundtrip` | `mise run test-audio-virtual-endpoint` 実走・無変更 |
| `tests/loopback.rs` | `midi-loopback` | `cargo test -p ruviform-midi-device --test loopback -- --ignored` 実走・無変更 |
| evidence upload | `Upload probe evidence` | attempt 固有 artifact 名・`midi-endpoint-probe-<os>.txt` を収集対象に追加 |

### 1.3 `midi_endpoint_probe` 整合の採用判断

AC は `endpoint_probe`/`midi_endpoint_probe` を併記するが、rf978 §1.3 では「`tests/loopback.rs` 自身の WinMM 列挙（`enumerated:`/`candidate[i]:` 行）と `midi-winmm-probe.ps1` の診断出力が probe 証拠を兼ねる」として独立 probe stage を意図的に省略していた。本 slice では **AC 語彙に忠実な明示観測として `midi-endpoint-probe` stage を追加する案 (a) を採用**した（Coordinator 確定の推奨案 — 観測が明示的で AC 監査にそのまま対応する）。設計:

- 配置は `endpoint-probe` の直後 — devnode 非依存の観測系 probe どうしを隣接させ、lane 実走 stage（`stream-roundtrip`〜`midi-loopback`）の前に両 probe が並ぶ AC の順序に一致。
- `tools/virtual-audio-driver/Invoke-MidiEndpointProbe.ps1`（新規・`Invoke-EndpointProbe.ps1` 同型）が `mise exec -- cargo run -p ruviform-midi-device --example midi_endpoint_probe` を実行し、probe 出力を `midi-endpoint-probe-<os>.txt` へ保存した上で `probe_exit`/`midi_in_ports`/`midi_out_ports`/`loopback_candidates`/`provisioning`/`candidate_ports` を `key=value` 行で返す。stage step はそれを `vaudio-probe-<os>.json` の `{stage,status,detail,exit_code}` レコードに転記する。
- absence-is-data 契約を維持 — `if: always()` + 末尾 `exit 0`、候補未検出は観測値であり失敗扱いしない（`endpoint-probe` と同契約）。`Reset probe workspace` の除去対象と `Upload probe evidence` の収集対象に `midi-endpoint-probe-<os>.txt` を追加済み。
- 代替案 (b)（rf978 §1.3 の意図的省略を runbook に明記するのみ）は不採用 — AC の列に `midi_endpoint_probe` が明示されており、example は crates に既存で追加コストが小さいため。

### 1.4 `midi_endpoint_probe` example の latent crash 修正（slice 範囲外の実在 crate bug）

`crates/ruviform-midi-device/examples/midi_endpoint_probe.rs` の `interface_id_in` は、`midiInMessage(DRV_QUERYDEVICEINTERFACE)` を **`dw1=size, dw2=buf` の引数取り違え順**で呼んでいた — rf978 §4.1 が cdb stack で確定したのと同じ bug で、adapter 側（`src/os/windows.rs` の `interface_id`）は rf978 attempt-6 で正順（`dw1=buf, dw2=size`）に修正済みだったが、example 側の独立実装には未反映だった。

- 影響: interface query に応答する midiIn port（`Ruviform MIDI Loopback` を含む real WDM device）を列挙した時点で、winmmbase が `size` を pointer 解釈したアドレスへ `StringCchCopyW` して決定的に AV crash する（rf978 attempt-4 の `0x90` write と同機構 — interface string byte size 144 = `0x90` で整合）。**provisioned 環境でこそ crash するため、`midi-endpoint-probe` stage はこの修正なしでは観測したい環境そのもので `error` を記録した**。
- 発見経緯: 本 slice の `midi-endpoint-probe` stage 設計中に example の入出力契約を確認したところ、rf978 §4.1 の修正が example に反映されていないことを静的解析で検出。Coordinator へ決裁依頼し、`crates/**` lease 拡張が承認された（条件: (a) 修正は引数順 2 行のみ・他の変更を混ぜない、(b) 本記録、(c) proportional 検証 `mise run check/lint ruviform-midi-device`）。
- 修正内容: `interface_id_in` の `midiInMessage` 呼出しで `buf.as_mut_ptr() as usize`（dw1）と `size as usize`（dw2）を正順に入れ替え — 2 行のみ。

## 2. 実走結果の記録経路

1. `gh workflow run virtual-audio-driver-probe.yml --ref feat/979-endpoint-lane` で **branch ref に pre-merge dispatch** する（checkout は `github.sha` pin — branch head が実走される）。
2. `gh run watch <run-id> --exit-status`（foreground）で完了待機し、artifact（`virtual-audio-driver-probe-windows-2025-attempt-<n>`）を `gh run download` で取得。`vaudio-probe-*.json`・`stage-*-*.txt`・`endpoint-probe-*.txt`・`midi-endpoint-probe-*.txt` を `evidence/rf979/` へ `attempt<n>-*` として記録・コミット（`vaudio-pkg/` の `.sys`/`.cat`/`.inf` バイナリは入れない — txt/json/cer のみ）。
3. 判定は stage 成立範囲の解釈として本ファイル §4 へ追記。**merge 後の main ref での再実走は統合検証として Coordinator が行う** — 本 slice の実走証拠は branch ref 実走まで。

## 3. ローカル検証記録（PR 段階・この worktree で実施）

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| フォーマット | `mise run fmt-check` | **Pass**（exit 0 — 変更後の example も整形済み） |
| 衛生チェック | `mise exec -- prek run --all-files` | **Pass**（trailing-whitespace/end-of-file/check-yaml/check-toml/mixed-line-ending/BOM/private-key/actionlint 全て Passed — exit 0） |
| workflow contract 全走査 | `mise run test-ci-daily` | **Pass**（113 pass / 0 fail — banned trigger 走査を含む） |
| tracker workflow contract | `mise run test-ci-tracker` | **Pass**（108 pass / 0 fail） |
| workflow 構文検査 | `mise exec -- actionlint .github/workflows/virtual-audio-driver-probe.yml` | **Pass**（exit 0・指摘なし） |
| cargo check（proportional — crates 変更のため） | `mise run check ruviform-midi-device` | **Pass**（exit 0） |
| cargo lint（同上） | `mise run lint ruviform-midi-device` | **Pass**（exit 0・`-D warnings`） |
| Cargo `test`/`build`・`*-full` | — | **NotApplicable**（変更は example 1 ファイルの 2 行 — test/build 対象の挙動変更なし。`lint` の `--all-targets` が example をカバー） |
| driver kernel build / 署名 / install / endpoint 列挙 / round-trip | — | **NotRun**（worker ホストに WDK/MSBuild 未搭載・かつ署名・store 書き込み・pnputil・devnode 作成は worker ホスト禁止事項 — runner lane で判定） |
| runner 実走（branch `workflow_dispatch`） | — | **Done** — §4 |

補助証拠（canonical = mise ではない・生出力は同ディレクトリの `local-syntax-check.txt`）:

- `Invoke-MidiEndpointProbe.ps1`（新規）・`Invoke-EndpointProbe.ps1`・`probe-stage.ps1`・`midi-winmm-probe.ps1`・`TestDevice.ps1` の `[Parser]::ParseFile` — 5/5 OK。
- workflow `run:` ブロックの `[Parser]::ParseInput` — 17/17 OK（rf978 時点の pre-existing な `"${{ runner.* }}"` 文字列内式の指摘は GitHub が実行前に置換するため実害なしと同様の構造だが、本走査では全 block parse OK）。
- `local-inf-check` は不実施 — INF・vcxproj は無変更（rf977/978 の検査が有効）。

## 4. runner 実走記録（branch ref dispatch — Worker 実施）

| attempt | run | ref | 結果 |
| --- | --- | --- | --- |
| 1 | [45543608858](https://github.com/scarecrow173/ruviform/actions/runs/45543608858) | `feat/979-endpoint-lane`（head SHA `e0e6aa65391b3834be0076912bbb89206da13cbd`） | **run conclusion success・全 14 stage `ok`** — 証拠 artifact `attempt1-*` 同梱 |

### attempt 1 — 全 stage 成立・AC 実走証拠

`vaudio-probe-windows-2025.json` の stage record 解釈（green job ではなく stage 語彙で判定）:

| stage | status | 解釈 |
| --- | --- | --- |
| env-build-surface | ok | msbuild/toolset 検出 |
| driver-build | ok | x64 `RvfVirtualAudio.sys` build 成立 |
| catalog-create / cert-create / store-register / sign / sign-verify | ok | test-sign チェーン成立 |
| pnputil-add-driver | ok | `oem12.inf` で store 登録 |
| enum-verify | ok | inf in store・devnode 未作成を確認（provisioning 前状態） |
| devnode-create | ok | `ROOT\MEDIA\0000`・`problem_code=0`・service=RvfVirtualAudio |
| endpoint-probe | ok | `probe_exit=0`・`pair_count=1`（Ruviform Cable Input/Output pair 検出 — AC「endpoint_probe」成立） |
| **midi-endpoint-probe** | **ok** | **`probe_exit=0` — crash なしで完走**・`midi_in_ports=1`・`midi_out_ports=2`・`loopback_candidates=2`・`provisioning=CANDIDATES_OBSERVED`・`candidate_ports=Ruviform MIDI Loopback`（AC「midi_endpoint_probe」成立 — 下記参照） |
| stream-roundtrip | ok | `test_exit=0`・`test result: ok. 1 passed`・**`finished in 3.48s`** — streaming 実走（vacuous pass の `0.00s` early return ではない — rf977 attempt-21 と同じ実走シグネチャ）。`verdict=no-verdict-line` は passing test の stderr が cargo test に capture されるためで、Fail/NotRun 時のみ verdict 行が表面化する設計（rf977 §4 の解釈と同じ） |
| midi-loopback | ok | `test_exit=0`・`test result: ok. 2 passed`・`verdict: Pass — round-trip observed through real OS ports` + `Pass — silent messages accepted by real WinMM output port`（`loopback_round_trip`・`real_device_silent_output_smoke` ともに ok） |
| cleanup | ok | devnode/store/cert 後片付け |
| artifact upload | — | `virtual-audio-driver-probe-windows-2025-attempt-1` 取得成功・`midi-endpoint-probe-windows-2025.txt` を含む（AC「evidence upload」成立） |

**`midi_endpoint_probe` example 修正の実走立証**: `midi-endpoint-probe-windows-2025.txt` で probe が最後まで完走し（`probe_exit=0`）、`[input] Ruviform MIDI Loopback index=0` の **`interface_id: \\?\root#media#0000#{6994ad04-93ef-11d0-a3cc-00a0c9223196}\midiloopback` が解決**されている。これは修正対象の `midiInMessage(DRV_QUERYDEVICEINTERFACE)` 応答そのもの — 修正前（`dw1=size, dw2=buf`）なら winmmbase が size=144(`0x90`) を pointer 解釈してこの時点で AV crash し `probe_exit!=0` となったはずの経路であり、**provisioned 環境での完走 + interface path 応答が正順修正の動作を実証**する。なお output port 側の `midiOutMessage` query は host の GS Wavetable synth driver 応答で crash する既知の host 制約で input-path query のみに退化する（example 内の既存注記どおり — 本修正とは無関係）。

### AC 対応

| AC | 判定 | 根拠 |
| --- | --- | --- |
| 1. `workflow_dispatch` 1 回で driver provisioning → probe/lane 実走 → evidence upload 完走 | **Pass** | run 45543608858 conclusion success・上表の全 stage `ok`・artifact upload 成立 |
| 2. 未 provision 環境は fail-closed `NotRun` のまま | **Pass**（設計契約の維持確認） | absence-is-data 契約は無変更（`if: always()` + 末尾 `exit 0`・候補未検出は失敗扱いしない観測記録）。`enum-verify` が `matching_devnodes=0` を devnode 作成前に記録し、tests は endpoint/port 不在時に `verdict: NotRun` を print して fail-closed（`virtual_endpoint.rs`/`loopback.rs` の契約 — 本 slice はその語彙を変更していない）。`midi-endpoint-probe` も候補未検出時は `provisioning` 値で観測を記録し stage 失败化しない |
| 3. lane 表/runbook が第三者再現可能 | **Pass** | `docs/development-environment.md` lane 表 + workflow 節・`docs/specs/issue-629-provisioning-runbook.md` 追記節（dispatch 手順〜stage/verdict 語彙〜artifact の読み方）・`docs/designs/issue-629-provisioning-runbook.md` §3.7/§3.8 を更新 |

### 残り注意事項

- `midi_in_ports=1` / `midi_out_ports=2` — output 側は `Microsoft GS Wavetable Synth`（OS 標準 synth）を含む列挙で、loopback 候補判定は双方向同名 `Ruviform MIDI Loopback` のみ。probe 設計どおりの観測値。
- merge 後の main ref での再実走は統合検証として Coordinator が実施する。docs index（`docs/**/README.md`）は Coordinator 側で同期。

## 5. 設計判断の記録

- canonical lane 増設なし・required gate 非接続・x64 leg のみ・absence-is-data 維持 — §1.1/§1.2。
- `midi_endpoint_probe` 整合は案 (a) stage 追加を採用 — §1.3。AC 文意との対応は §1.2 の対応表どおり。
- example crash 修正は slice 範囲外の実在 bug の最小修正（2 行）— §1.4。
- docs index（`docs/**/README.md`）は未編集 — 生成物で Coordinator が統合時に整備する。
