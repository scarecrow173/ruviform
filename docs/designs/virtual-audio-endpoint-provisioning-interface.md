---
id: "DESIGN-rvfVirtualAudioEndpointProvisioningInterface"
type: "design"
status: "proposed"
title: "Virtual Audio Endpoint Provisioning Interface — step-level provision/unprovision primitive 設計"
created: "2026-11-25"
updated: "2026-11-25"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-provisioning-runbook.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "https://github.com/scarecrow173/synvelis/issues/115"
  changes:
    added:
      - type: "file"
        path: "docs/designs/virtual-audio-endpoint-provisioning-interface.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-provisioning-runbook.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "tools/virtual-audio-driver/README.md"
    - "tools/testsigned-driver/probe-stage.ps1"
    - "tools/testsigned-driver/TestDevice.ps1"
    - ".github/workflows/virtual-audio-driver-probe.yml"
    - "https://github.com/scarecrow173/synvelis/blob/main/docs/designs/issue-115-virtual-endpoint-provisioning.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-629-provisioning-runbook.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/762"
    - "https://github.com/scarecrow173/ruviform/issues/1001"
  defers: []
  deferred-by: []
---

# Virtual Audio Endpoint Provisioning Interface 設計

> Cross-repo link note: Synvelis 側文書への `blob/main` URL は、対応 PR merge 後の
> canonical location を示す。proposal 時点の main にはまだ存在しておらず、両 PR の統合後に
> 実在させる意図のリンクである。

## 1. 目的と非目的

本設計は、Ruviform Virtual Audio Device を ephemeral Windows runner 上で
provision/unprovision する reusable step-level primitive の interface を定義する。
primitive は consumer の test 内容を知らず、Ruviform driver package の build、
test-sign、install、root-enumerated MEDIA devnode、microphone consent、probe、
cleanup に責任を限定する。

初期 consumer は Synvelis の manual virtual endpoint acceptance workflow を
想定するが、本 primitive は Synvelis 固有の checkout・test lane・acceptance
verdict・evidence 解釈を所有しない。consumer 側 orchestration は
`https://github.com/scarecrow173/synvelis/blob/main/docs/designs/issue-115-virtual-endpoint-provisioning.md`
を参照する。本 repo では reusable primitive 契約だけを正本にする。

この文書は 2026-11-25 の design/docs slice で作られた proposal であり、
script pair はまだ実装されていない。実装済み interface として扱ってはならない。

## 2. 現行実装から抽出する責務

現行 `.github/workflows/virtual-audio-driver-probe.yml` は次を同一 manual
job に inline で保持する。

- build/tool surface の観測、driver build、catalog 作成
- 2 日失効の self-signed cert 作成と Root/TrustedPublisher 登録
- catalog signing と signature verify
- `pnputil /add-driver ... /install` による driver store staging
- `tools/testsigned-driver/TestDevice.ps1` による root-enumerated MEDIA
  devnode 作成（hardware id `ROOT\RvfVirtualAudio`）
- MMDevice endpoint probe、MIDI endpoint probe、stream roundtrip、wiring/e2e/
  fault cases
- microphone consent registry 変更と `AudioEndpointBuilder`/`Audiosrv` restart
- `vaudio-state-<os>.json` に基づく devnode/driver/cert cleanup
- stage log、probe JSON、driver package の artifact upload

primitive extraction は、このうち endpoint provisioning に必要な OS state の
作成と削除を切り出す。fault cases や stream roundtrip の test controller その
ものは generic provisioning primitive にしない。consumer は provision 後の
endpoint を使って独自 test を実行する。

## 3. 採用形状: PowerShell script pair

初期 interface は次の 2 script とする。

- `tools/virtual-audio-driver/Provision-RvfVirtualAudioEndpoint.ps1`
- `tools/virtual-audio-driver/Unprovision-RvfVirtualAudioEndpoint.ps1`

共有実装が必要な場合は内部 module
`tools/virtual-audio-driver/RvfVirtualAudioProvisioning.psm1` に置いてよいが、
外部契約は script pair と state JSON に固定する。composite action は後でこれらを
呼ぶ薄い wrapper として追加してよい。初期正本にはしない。

script pair を選ぶ理由は、GitHub Actions の同一 job からも、manual parity
run や local debug からも同じ手順で実行できることである。composite action は
step 群を隠蔽できる一方、repo 外直接実行と manual probe の段階移行が難しい。
interface の単位を job ではなく step に保つため、script pairを先に安定させる。

## 4. 実行前提と必要権限

primitive は初期版で次の環境だけを対象にする。

- Windows Server 2025 x64（GitHub-hosted `windows-2025` runner）
- ephemeral runner。`GITHUB_ACTIONS=true` かつ GitHub-hosted runner であることを
  確認し、 persistent/self-hosted runner または開発者 machine では実行しない
- 管理者権限。driver store、Root/TrustedPublisher cert store、HKLM/HKCU
  consent registry、root-enumerated devnode、service control を変更できること
- network access。WDK/Build Tools install、Rust crate/toolchain 取得が必要な
  場合があること
- repo 内 source checkout。prebuilt `.sys`/`.cat`/`.cer` や外部 binary download
  を入力にしない
- `bcdedit` 変更と reboot を行わない。`testsigning=Yes`・SecureBoot OFF は
  runner 出荷状態の観測値として記録し、primitive は変更しない

non-ephemeral 環境では `not_run` で終了し、OS state を変更しない。initial
contract に escape hatch は設けない。rf#762 の self-hosted/external runner
要件が残る場合は、別途 risk assessment と restore contract を設けてから拡張する。

## 5. Provision inputs

`Provision-RvfVirtualAudioEndpoint.ps1` は少なくとも次の入力を取る。

| input | type / default | 契約 |
| --- | --- | --- |
| `-SourceRoot` | mandatory path | Ruviform checkout root。`drivers/rvf-virtual-audio`、tools、WDK project file が存在すること |
| `-ExpectedSourceRev` | optional string | 指定時は source checkout HEAD と一致しない場合に fail-closed。証拠再現性に使う |
| `-WorkspaceRoot` | mandatory path | build/package/log の writable workspace。consumer checkout を汚さない場所を caller が選ぶ |
| `-StatePath` | mandatory path | provisioning state JSON の出力 path |
| `-EvidenceDir` | mandatory path | artifact manifest、probe 出力、human-readable log の出力先 |
| `-Arch` | default `x64` | 初期は `x64` のみ。他 arch は `not_run` |
| `-Configuration` | default `Release` | driver build configuration。実装で support する値だけを許可する |
| `-AllowWdkInstall` | switch | 未指定なら WDK/Build Tools が無い場合に install せず `absent` または `blocked`。指定時のみ winget 等で install を試行する |
| `-AllowMicConsentProvisioning` | switch | 未指定なら consent registry と audio service restart を行わず、capture readiness を `blocked` または `absent` として記録する |
| `-TimeoutSeconds` | default implementation 値 | provision 全体と long-running stage の上限。caller timeout と独立に bounded にする |
| `-CleanupOnFailure` | default `Full` | `Full` は partial state を best-effort cleanup、`Preserve` は diagnostic 用に residue を残す。consumer acceptance では `Full` 必須 |

secret、credential、GitHub token、consumer repo path、consumer test 名は入力に
含めない。credential 設定と cross-repo checkout は caller workflow の責務である。

## 6. Unprovision inputs

`Unprovision-RvfVirtualAudioEndpoint.ps1` は少なくとも次の入力を取る。

| input | type / default | 契約 |
| --- | --- | --- |
| `-StatePath` | mandatory path | provision が出力した state JSON。partial state も受け取る |
| `-WorkspaceRoot` | optional path | log や補助 file の場所。state 参照の補助に使う |
| `-EvidenceDir` | mandatory path | cleanup log、updated state、artifact manifest の出力先 |
| `-CleanupMode` | default `Full` | `Full` は devnode/driver/cert/consent/service を cleanup。`ReportOnly` は residue を観測するだけで OS state を変更しない |
| `-TimeoutSeconds` | default implementation 値 | cleanup 全体と service control の上限 |

`ReportOnly` は診断用であり、consumer acceptance workflow の通常 cleanup では
使わない。evidence/log は cleanup が削除しない。`Full` でも build log、probe
JSON、artifact manifest、driver package evidence を消さない。

## 7. Outputs と state JSON schema

両 script は human-readable log に加え、machine-readable state JSON を出力する。
caller は stdout の構文に依存せず、state JSON と exit code を読む。schema 名は
`rvf-virtual-audio-provisioning/v1` とする。

必須 field は次の通り。

| field | 内容 |
| --- | --- |
| `schema` | `rvf-virtual-audio-provisioning/v1` |
| `operation` | `provision` または `unprovision` |
| `status` | terminal status。`ok`/`absent`/`blocked`/`not_run`/`error` |
| `detail` | machine-readable detail token。人間向け文章だけにしない |
| `exit_code` | script exit code |
| `recorded_utc` / `started_utc` / `finished_utc` | UTC timestamp |
| `source` | repo URL、checkout rev、expected rev、dirty flag。dirty checkout は acceptance evidence に使わない |
| `runner` | OS、arch、runner environment、testsigning、SecureBoot、admin/user、machine 固有情報は必要最小限 |
| `inputs` | secret を含まない sanitized input echo |
| `stages` | `{stage,status,detail,exit_code,recorded_utc}` の配列。語彙は既存 `probe-stage.ps1` と同じ |
| `endpoint` | endpoint candidate、pair count、render/capture device id または name、probe result |
| `package` | INF/CAT/SYS path、hash、publisher、cert thumbprint、cert notAfter、signed flag |
| `os_state` | `instance_id`、`inf_published`、cert thumbprint、consent before/after、service restart record |
| `artifacts` | evidence file path、kind、sha256 の manifest |
| `cleanup` | unprovision 時の stage/residue/leftover/error。provision 時は null または空 object |

JSON は artifact にそのまま残せる形状とし、local path を含めてよいが、tokenや
private key、cert export password を含めない。self-signed cert の private key は
ephemeral cert store に留め、artifact へ export しない。

## 8. Exit code と失敗 vocabulary

terminal status の語彙は既存 probe stage と同じ `ok`/`absent`/`blocked`/
`not_run`/`error` を使う。consumer が workflow conclusion ではなく状態を
parse できるように、exit code との対応を固定する。

| exit code | 意味 |
| --- | --- |
| 0 | provision/unprovision が contract 上の目的を完了。provision では endpoint pair readiness が `ok`、unprovision では cleanup residue なしまたは最初から対象なし |
| 1 | script は state JSON を出力したが terminal status が非 `ok`。`absent`/`blocked`/`not_run`/`error` を含む |
| 2 | interface/contract violation。引数不正、state JSON を出力できない、schema 生成前の例外、credential/環境の不安全な指定 |

detail token の初期例は次の通り。実装で増やす場合は machine-readable のまま
維持する。

- `UNSUPPORTED_ENVIRONMENT`
- `NON_EPHEMERAL_RUNNER`
- `UNSUPPORTED_ARCH`
- `SOURCE_REV_MISMATCH`
- `WDK_ABSENT`
- `WDK_INSTALL_FAILED`
- `DRIVER_BUILD_FAILED`
- `CERT_CREATE_FAILED`
- `CERT_INSTALL_FAILED`
- `SIGN_FAILED`
- `SIGN_VERIFY_FAILED`
- `PNPUTIL_ADD_FAILED`
- `DEVNODE_CREATE_FAILED`
- `ENDPOINT_PAIR_NOT_FOUND`
- `MIC_CONSENT_FAILED`
- `SERVICE_RESTART_FAILED`
- `CLEANUP_RESIDUE`

`absent` は測定対象や必要 tool が host に無い場合、`blocked` は前提 stage が
成立せず後続を実施できない場合、`not_run` は policy guard で実行しなかった
場合、`error` は実行した操作が失敗した場合に使う。endpoint pair 不在は
provisioning が devnode 作成まで成功したかで区別し、detail に
`ENDPOINT_PAIR_NOT_FOUND` を残す。

この語彙は provisioning state の語彙であり、consumer harness の Case Schema
`pass`/`fail`/`not-run` とは別物である。`not_run` を consumer の `not-run`
へ機械的に写像せず、`ok` を consumer Pass とみなさない。consumer verdict は
consumer harness が生成する terminal metadata だけが決める。

## 9. Cleanup contract

unprovision は idempotent でなければならない。state JSON に記録された partial
state だけでなく、既知の Ruviform hardware id、INF published name、cert
thumbprint を使って residue を確認する。ただし、primitive が作成したと記録
できない pre-existing driver/cert/devnode を無条件に削除してはならない。

`Full` cleanup は少なくとも次を行う。

1. state JSON の `instance_id` と `ROOT\RvfVirtualAudio` に一致する devnode を
   remove する。`TestDevice.ps1 -Action remove` を第一経路とし、失敗時は
   `pnputil /remove-device` の結果を記録する。
2. `pnputil /delete-driver <published inf> /uninstall /force` で driver store
   から削除する。`/enum-drivers` で residue を再確認する。
3. primitive が作成した self-signed cert を Root/TrustedPublisher から削除
   する。thumbprint が一致しない cert は削除しない。
4. consent registry について、provision 時に記録した before 値へ戻す。before
   が不存在だった値は、作成した値だけを削除する。before が記録できなかった
   場合は復元を行わず `CLEANUP_RESIDUE` として報告する。
5. `AudioEndpointBuilder`/`Audiosrv` を contract に従って再起動し、restart
   failure を cleanup error として記録する。
6. evidence/log/artifact manifest は削除しない。workspace の transient build
   file を消す場合も state/evidence upload 前に必要な file を残す。
7. cleanup 後に devnode/driver/cert/consent residue を再観測し、leftover を
   `cleanup.leftovers` へ列挙する。

cleanup error は consumer test verdict と独立である。caller は cleanup script
exit/status を artifact に残し、test Pass/Fail/NotRun の metadata を書き換えて
はならない。

## 10. Provisioning stage order

provision の conceptual stage order は次の通り。各 stage は state JSON に記録
し、前 stage 不成立時は `blocked` で後続を止める。

1. `environment` — OS/arch/ephemeral/admin/testsigning/SecureBoot/tool surface
2. `source` — source root/rev/dirty/必要 file の確認
3. `wdk` — tool discovery、必要なら opt-in install
4. `driver-build` — repo source から build
5. `catalog` — INF/CAT 作成と validation
6. `cert-create` — ephemeral self-signed cert 作成。notAfter を記録
7. `cert-install` — Root/TrustedPublisher 登録
8. `sign` / `sign-verify` — catalog signing と verification
9. `driver-store` — `pnputil /add-driver ... /install`、published INF 記録
10. `devnode` — MEDIA class root-enumerated devnode 作成
11. `endpoint-probe` — MMDevice render/capture endpoint pair 候補の確認
12. `mic-consent` — opt-in の場合だけ consent 変更と service restart
13. `readiness` — endpoint pair と capture readiness の terminal 判定

MIDI endpoint probe、stream roundtrip、os-endpoint wiring/e2e、fault cases は
この primitive の必須 stage に含めない。それらは provisioning 後の consumer
または Ruviform probe workflow の test stage である。manual probe workflow が
必要とする場合は、primitive 成功後に従来どおり実行できる。

## 11. 既存 manual probe との互換性と移行

既存 `virtual-audio-driver-probe.yml` は absence-is-data の測定 lane であり、
provisioning primitive への抽出で観測能力を落とさない。移行は次の順序で行う。

1. script pair を追加し、manual probe workflow は inline 実装のまま残す。
   script を dry-run/parity run で実行し、生成 state JSON と stage record が
   inline 実装と同じ vocabulary で観測できることを確認する。
2. manual probe workflow の build/sign/install/devnode/consent/cleanup stage を
   script pair 呼び出しへ置き換える。stream/MIDI/wiring/fault/artifact upload
   stage は残す。
3. artifact 名、probe JSON、stage log、fault-case evidence が従来どおり取得
   できることを 1 回以上の manual run で確認する。退行があれば workflow を
   inline 実装へ戻し、script は evidence 付きで修正する。

`tools/virtual-audio-driver/README.md` の更新は implementation slice で行う。
本 design/docs slice では README を変更しない。

## 12. Consumer との境界

consumer が受け取るのは state JSON、artifact manifest、endpoint pair 候補、
exit code だけである。consumer は次を行わない。

- driver build/sign/install 手順の複写
- cert/devnode/consent cleanup の独自実装
- Ruviform source revision の推測
- provision state `ok` 前の endpoint test 実行
- cleanup state を consumer test verdict へ混入

逆に primitive も次を行わない。

- consumer checkout、cargo test、DAW Recorder oracle の実行
- consumer RunMetadata の生成・改変・Pass/Fail/NotRun 判定
- consumer evidence への転記
- cross-repo credential の取得や保持

この境界により、Ruviform は endpoint provisioning の再現性を所有し、consumer
は製品 acceptance の判定を独立に所有できる。

## 13. Open implementation decisions

次は implementation slice で code shape を決める。いずれも interface の責務
境界を変えない。

- script pair の内部共有 module を分けるか、2 script に最小限の重複を許すか。
- WDK install を winget で行う場合の exact package/version pin。現行 manual
  workflow の実測を起点にする。
- endpoint pair readiness に MIDI readiness を含めないことの確認。MIDI は
  consumer または manual probe の別 stage とする。
- artifact hash algorithm は SHA-256 を使い、path 正規化と UTF-8 JSON encoding
  を実装で固定する。

これらが未決でも本 interface を実装開始できる。environment guard、cleanup
契約、source revision、state schema を変える提案が出た場合は、実装前にこの
文書を更新する。
