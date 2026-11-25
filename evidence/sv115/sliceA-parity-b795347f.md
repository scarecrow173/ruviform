# sv#115 Slice A post-extraction parity / validation

- 対象実装 SHA: `b795347f20906c1af9071af149fa1075d44c6b8f`（`scarecrow173/sv115-sliceA-provisioning`）
- post run id: `45692342361`
- post run URL: https://github.com/scarecrow173/ruviform/actions/runs/45692342361
- baseline run id / URL: `45673094185` / https://github.com/scarecrow173/ruviform/actions/runs/45673094185
- dispatch/完了 (UTC): 2026-11-25T20:10:15Z / 2026-11-25T20:16:10Z（job 5m50s）
- branch/event: `scarecrow173/sv115-sliceA-provisioning` / `workflow_dispatch`
- workflow conclusion: `success`
- artifact: `virtual-audio-driver-probe-windows-2025-attempt-1`（retention 30 日）
- 取得コマンド: `gh workflow run virtual-audio-driver-probe.yml --ref scarecrow173/sv115-sliceA-provisioning`、`gh run watch 45692342361 --exit-status --interval 30`（foreground watch 完走）、`gh run download 45692342361 --dir <temp>\sv115-run-45692342361`

## primitive state / cleanup

- `vaudio-primitive-state.json`: schema `rvf-virtual-audio-provisioning/v1`、terminal status `ok`、detail `OK`、exit code 0、endpoint pair_count `1`。
- canonical stages: environment/source/wdk/driver-build/catalog/cert-create/cert-install/sign/sign-verify/driver-store/devnode/endpoint-probe/mic-consent/readiness は全て `ok`。
- `os_state.pre_devnode`: `inf_in_store=True`、`published_name=oem12.inf`、`matching_devnodes=0`、`devices=[]`、recorded `2026-11-25T20:12:55Z`。driver store staging 後・devnode 作成前の read-only 観測であり、`vaudio-primitive-provision/stage-enum-verify-pre-devnode.log` にも同じ観測が残る。
- `vaudio-unprovision-state.json`: terminal status `ok`、detail `OK`、exit code 0、`cleanup.leftovers=[]`、`cleanup.errors=[]`。
- primitive evidence manifest/log/package/public cert は artifact に含まれる。private key export はない。

## legacy stage order / status / numeric parity

baseline/post とも legacy stage の順序は完全一致し、18 件とも同じ順序だった:

`env-build-surface` → `driver-build` → `catalog-create` → `cert-create` → `store-register` → `sign` → `sign-verify` → `pnputil-add-driver` → `enum-verify` → `devnode-create` → `endpoint-probe` → `midi-endpoint-probe` → `stream-roundtrip` → `midi-loopback` → `os-endpoint-wiring` → `os-endpoint-midi-e2e` → `fault-cases` → `cleanup`

| stage | baseline | post | parity note |
| --- | --- | --- | --- |
| env-build-surface | ok | ok | detail の measured values が一致: `msbuild=present; toolset=3; kmHeaders=0; kmLibs=0; signtoolKits=3; inf2catKits=0; winget=present` |
| driver-build | ok | ok | detail一致。legacy log には primitive `stage-wdk.log` の winget/WDK install 記録と driver build log の両方が残る |
| catalog-create | ok | ok | detail一致（互換投影は repo workspace にコピーした `D:\a\ruviform\ruviform\vaudio-pkg\rvfvirtualaudio.cat` を指す） |
| cert-create | ok | ok | thumbprint は run ごとの動的値のため数値は異なるが形式・状態は同じ |
| store-register | ok | ok | thumbprint は cert-create と同じ動的値。stores=`Root+TrustedPublisher` 一致 |
| sign | ok | ok | detail一致 |
| sign-verify | ok | ok | detail一致 |
| pnputil-add-driver | ok | ok | detail一致: `published=oem12.inf` |
| enum-verify | ok | ok | detail一致: `inf_in_store=True; matching_devnodes=0`。legacy record は devnode 作成後ではなく primitive `os_state.pre_devnode` の pre-devnode 実測値から投影 |
| devnode-create | ok | ok | detail一致: `instance_id=ROOT\MEDIA\0000; status=OK; problem_code=0; service=RvfVirtualAudio` |
| endpoint-probe | ok | ok | detail一致: endpoint 2、candidate 2、pair_count 1 |
| midi-endpoint-probe | ok | ok | detail一致: midi_in 1、midi_out 2、loopback candidate 2 |
| stream-roundtrip | error | ok | baseline は `test_exit=101`、post は `test_exit=0`。provisioning parity の悪化ではなく round-trip 結果の改善/変動。post の `verdict=no-verdict-line` は passing test の stdout が capture される既知の読み方どおり |
| midi-loopback | ok | ok | detail一致 |
| os-endpoint-wiring | ok | ok | detail一致 |
| os-endpoint-midi-e2e | ok | ok | detail一致 |
| fault-cases | ok | ok | detail一致（harness_problems=0） |
| cleanup | ok | ok | post detail は `primitive_status=ok; detail=OK; leftovers=0; errors=0`。baseline の空 detail より情報量が増えた意図的な互換投影であり、residue/error 数値は 0 |

## legacy state key parity

`vaudio-state-windows-2025.json` の共有 state key は baseline/post で同じ set と型を維持している。run 固有の証明書 thumbprint を除き、主要値は一致した。

- `msbuild`: 両方 `C:\Program Files\Microsoft Visual Studio\18\Enterprise\MSBuild\Current\Bin\amd64\MSBuild.exe`
- `vs_install`: 両方 `C:\Program Files\Microsoft Visual Studio\18\Enterprise`
- `km_headers`: 両方 `1`（workflow 互換 state では baseline と同じく、winget install 実行後の `wdm.h` count を記録）
- `km_libs`: 両方 `0`（baseline と同じく env-build-surface の初期観測値を保持）
- `sys_path` / `cat_path` / `pkg_dir` / `cert_cer_path`: path 形式と場所が一致
- `inf_published`: 両方 `oem12.inf`
- `inf_staged`: 両方 JSON boolean `true`。中間修正前に発生した文字列化 regressions は解消済み
- `instance_id`: 両方最終 state では `ROOT\MEDIA\0001`（fault/recovery 後の値。devnode-create record 自体は両方 `ROOT\MEDIA\0000`）

## artifact parity

- artifact 名 pattern は baseline/post とも `virtual-audio-driver-probe-windows-2025-attempt-1`。
- baseline の 59 files、post の 94 files。`vaudio-probe-windows-2025.json`、`vaudio-state-windows-2025.json`、18 件の legacy `stage-*-windows-2025.txt`、endpoint/MIDI probe 出力、os-endpoint RunMetadata 2 件、os-endpoint-midi-e2e RunMetadata 1 件、fault-case evidence、`vaudio-pkg/`、`vaudio-cert.cer` は post に存在する。
- baseline にのみあった `vaudio-pkg/stage-catalog-create-windows-2025.txt` は、package directory を driver payload（`.inf`/`.sys`/`.cat`）だけに保つ意図的な cleanup。catalog create の evidence 自体は legacy 互換の top-level `stage-catalog-create-windows-2025.txt` として post artifact に存在するため、観測喪失ではない。
- post には primitive 追加 evidence（`vaudio-primitive-state.json`、`vaudio-unprovision-state.json`、`vaudio-primitive-provision/`、`vaudio-primitive-unprovision/`、artifact manifest）が増える。既存 artifact/file を置き換えず追加している。
- `fault-audio-recovery-poll-10` / `fault-midi-recovery-poll-10` は post の recovery poll が 10 回まで採取されたための動的差分であり、provisioning parity の退行ではない。

## 検証コマンドと結果

- `mise exec -- prek run --all-files`: Windows 11 Pro x64 ローカル、exit 0、Pass（canonical）。
- `mise exec -- actionlint .github/workflows/virtual-audio-driver-probe.yml`: Windows 11 Pro x64 ローカル、exit 0、Pass（canonical）。
- PowerShell parser check（補助証拠、canonical = mise/workflow run）: `Provision-RvfVirtualAudioEndpoint.ps1`、`Unprovision-RvfVirtualAudioEndpoint.ps1`、`RvfVirtualAudioProvisioning.psm1` とも parser errors 0。workflow `Provision virtual audio endpoint primitive` inline `run:` block は GitHub expression を仮置換した上で errors 0。
- non-ephemeral guard（補助証拠）: ローカル Windows 11 Pro では provision/unprovision とも exit 1、state JSON status `not_run`、detail `NON_EPHEMERAL_RUNNER`、OS mutation なし。
- baseline/post manual workflow run: 上記 run URL のとおり両方 `success`。post primitive terminal `ok`・endpoint pair candidate 記録・cleanup state artifact 残存を確認。

## 中間 run で検出し最終 run までに解消した差分

- run `45682299459`（SHA `0cc5fae9dda6c4c7db16eebd2e72dbce528e15b6`）は workflow conclusion success だったが、primitive state が `MIC_CONSENT_FAILED` であり acceptance を満たさなかった。原因は PowerShell array return セマンティクスで空問題配列が `System.Object[]` 1 件として扱われたこと、および cleanup が既削除 INF の delete 失敗を residue としたこと。`dc8397a9` で修正した。
- run `45684064214`（SHA `dc8397a9...`）は status だけでは成功に見えたが、enum-verify が devnode 作成後の live 観測（`matching_devnodes=5`）に変わり、baseline の pre-devnode absence evidence（`0`）を失っていた。`19b17001` で pre-devnode 観測と legacy 順投影を復元した。
- run `45689860355`（SHA `19b17001...`）では enum-verify は復元された一方、env-build-surface の `toolset` が all-platform baseline `3` ではなく x64-only `1`、互換 state の `km_headers/km_libs` が baseline state semantics と異なり、`inf_staged` が文字列化していた。`b795347f` で measured values/state key projection を修正し、最終 run `45692342361` で上記の一致を確認した。

## 判定

- 本 slice の primitive extraction / manual probe parity: **Pass**。
- enum-verify の causal order と数値（pre-devnode `matching_devnodes=0`）、env-build-surface measured values、legacy state key semantics は baseline と一致した。
- sv#115 broad acceptance、consumer workflow（Slice B）、Issue close は対象外。
