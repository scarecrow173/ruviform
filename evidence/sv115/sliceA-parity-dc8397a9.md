# sv#115 Slice A post-extraction parity / validation

- 対象実装 SHA: `dc8397a9400452f6be16b4b8b95f8c87cea0bbc7`（`scarecrow173/sv115-sliceA-provisioning`）
- post run id: `45684064214`
- post run URL: https://github.com/scarecrow173/ruviform/actions/runs/45684064214
- dispatch/完了 (UTC): 2026-11-25T19:19:35Z / 2026-11-25T19:26:20Z 前後（job 6m45s）
- branch/event: `scarecrow173/sv115-sliceA-provisioning` / `workflow_dispatch`
- workflow conclusion: `success`
- artifact: `virtual-audio-driver-probe-windows-2025-attempt-1`（retention 30 日）
- 取得コマンド: `gh workflow run virtual-audio-driver-probe.yml --ref scarecrow173/sv115-sliceA-provisioning`、`gh run watch 45684064214 --exit-status --interval 30`（foreground watch 完走）、`gh run download 45684064214 --dir <temp>\sv115-post-final`

## primitive state / cleanup

- `vaudio-primitive-state.json`: schema `rvf-virtual-audio-provisioning/v1`、terminal status `ok`、detail `OK`、exit code 0、endpoint pair_count `1`。
- canonical stages: environment/source/wdk/driver-build/catalog/cert-create/cert-install/sign/sign-verify/driver-store/devnode/endpoint-probe/mic-consent/readiness は全て `ok`。
- `vaudio-unprovision-state.json`: terminal status `ok`、detail `OK`、exit code 0、`cleanup.leftovers=[]`、`cleanup.errors=[]`。
- primitive evidence manifest/log/package/public cert は artifact に含まれる。private key export はない。

## baseline/post stage status comparison

baseline は run `45673094185`（`evidence/sv115/baseline-run-45673094185.md`）。post では `stream-roundtrip` が baseline の `error` から `ok` へ改善し、悪化した stage は zero。

| stage | baseline | post |
| --- | --- | --- |
| env-build-surface | ok | ok |
| driver-build | ok | ok |
| catalog-create | ok | ok |
| cert-create | ok | ok |
| store-register | ok | ok |
| sign | ok | ok |
| sign-verify | ok | ok |
| pnputil-add-driver | ok | ok |
| enum-verify | ok | ok |
| devnode-create | ok | ok |
| endpoint-probe | ok | ok |
| midi-endpoint-probe | ok | ok |
| stream-roundtrip | error | ok |
| midi-loopback | ok | ok |
| os-endpoint-wiring | ok | ok |
| os-endpoint-midi-e2e | ok | ok |
| fault-cases | ok | ok |
| cleanup | ok | ok |

## artifact parity

- artifact 名 pattern は baseline/post とも `virtual-audio-driver-probe-windows-2025-attempt-1`。
- post artifact に `vaudio-probe-windows-2025.json`、`vaudio-state-windows-2025.json`、18 件の legacy `stage-*-windows-2025.txt`、endpoint/MIDI probe 出力、os-endpoint RunMetadata 2 件、os-endpoint-midi-e2e RunMetadata 1 件、fault-case evidence、`vaudio-pkg/`、`vaudio-cert.cer` が存在する。
- post artifact は primitive 追加 evidence（`vaudio-primitive-state.json`、`vaudio-unprovision-state.json`、`vaudio-primitive-provision/`、`vaudio-primitive-unprovision/`、artifact manifest）を含む。既存 artifact/file を置き換えず追加している。
- `env-build-surface`〜`devnode-create` の legacy record は実測した primitive stage だけを投影した互換 record。`enum-verify` は workflow 側 read-only 観測として残っている。

## 検証コマンドと結果

- `mise exec -- prek run --all-files`: Windows 11 Pro x64 ローカル、exit 0、Pass（canonical）。
- `mise exec -- actionlint .github/workflows/virtual-audio-driver-probe.yml`: Windows 11 Pro x64 ローカル、exit 0、Pass（canonical）。
- PowerShell parser check（補助証拠、canonical = mise/workflow run）: `Provision-RvfVirtualAudioEndpoint.ps1`、`Unprovision-RvfVirtualAudioEndpoint.ps1`、`RvfVirtualAudioProvisioning.psm1` とも parser errors 0。workflow inline `run:` block は GitHub expression を仮置換した上で errors 0。
- non-ephemeral guard（補助証拠）: ローカル Windows 11 Pro では provision/unprovision とも exit 1、state JSON status `not_run`、detail `NON_EPHEMERAL_RUNNER`、OS mutation なし。
- baseline/post manual workflow run: 上記 run URL のとおり両方 `success`。post primitive terminal `ok`・endpoint pair candidate 記録・cleanup state artifact 残存を確認。

## 初回 post run で検出した実装欠陥

run `45682299459`（SHA `0cc5fae9dda6c4c7db16eebd2e72dbce528e15b6`）は workflow conclusion success だったが、primitive state が `MIC_CONSENT_FAILED` であり acceptance を満たさなかった。原因は PowerShell array return セマンティクスで空問題配列が `System.Object[]` 1 件として扱われたこと、および cleanup が既削除 INF の delete 失敗を residue としたこと。`dc8397a9` で修正し、最終 run `45684064214` で primitive/cleanup 両方 `ok` を確認した。

## 判定

- 本 slice の primitive extraction / manual probe parity: **Pass**。
- sv#115 broad acceptance、consumer workflow（Slice B）、Issue close は対象外。
