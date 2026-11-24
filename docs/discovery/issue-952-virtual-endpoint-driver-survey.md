---
id: "DISC-952vEPdR1v3rSurVeyX01"
type: "discovery"
status: "draft"
title: "Issue #952 virtual audio/MIDI endpoint driver method survey and GitHub-hosted runner feasibility"
created: "2026-11-25"
updated: "2026-11-25"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/952"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-952-virtual-endpoint-driver-survey.md"
      - type: "doc"
        path: "evidence/rf952/web-sources-signing-runner-wms.md"
      - type: "doc"
        path: "evidence/rf952/endpoint_probe-windows-local.txt"
      - type: "doc"
        path: "evidence/rf952/midi_endpoint_probe-windows-local.txt"
      - type: "doc"
        path: "evidence/rf952/local-queries-windows-local.txt"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "POLICIES.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-952-virtual-endpoint-driver.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-os-endpoint-lane.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-midi-loopback.md"
    - "docs/designs/issue-629-midi-output.md"
    - "docs/designs/issue-253-virtual-midi-e2e.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/952"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://learn.microsoft.com/en-us/windows-hardware/drivers/install/kernel-mode-code-signing-policy--windows-vista-and-later-"
    - "https://learn.microsoft.com/en-us/windows-hardware/drivers/install/kernel-mode-code-signing-requirements--windows-vista-and-later-"
    - "https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/driver-signing-offerings"
    - "https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-attestation"
    - "https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/dashboard-api"
    - "https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/manage-product-submissions"
    - "https://github.com/virtio-win/SDCM"
    - "https://blogs.oracle.com/linux/automatic-attestation-signing"
    - "https://docs.github.com/en/actions/reference/runners/github-hosted-runners"
    - "https://docs.github.com/en/actions/concepts/runners/github-hosted-runners"
    - "https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-Readme.md"
    - "https://github.com/actions/runner-images/blob/main/images/windows/Windows2022-Readme.md"
    - "https://github.com/actions/runner-images/blob/main/images/windows/Windows11-Arm64-Readme.md"
    - "https://github.com/actions/runner/issues/3677"
    - "https://github.com/actions/runner-images/issues/2528"
    - "https://blogs.windows.com/windowsexperience/2026/02/17/making-music-with-midi-just-got-a-real-boost-in-windows-11/"
    - "https://microsoft.github.io/MIDI/"
    - "https://microsoft.github.io/MIDI/kb/minimum-requirements/"
    - "https://microsoft.github.io/MIDI/kb/api-back-compat/"
    - "https://microsoft.github.io/MIDI/kb/config-json/"
    - "https://microsoft.github.io/MIDI/kb/virtual-loopback/"
    - "https://microsoft.github.io/MIDI/tools/midiloopbacksetup/"
    - "https://microsoft.github.io/MIDI/tools/powershell/"
    - "https://microsoft.github.io/MIDI/tools/console/"
    - "https://github.com/microsoft/MIDI/releases/"
    - "https://learn.microsoft.com/en-us/windows/win32/coreaudio/user-mode-audio-components"
    - "https://learn.microsoft.com/en-us/windows/win32/coreaudio/enumerating-audio-devices"
    - "https://learn.microsoft.com/en-us/answers/questions/1622324/how-to-use-iaudioendpoint-and-connect-it-to-audio"
    - "https://community.osr.com/t/seeking-assistance-how-to-create-virtual-input-and-output-devices-in-windows-11/59143"
    - "https://github.com/microsoft/Windows-driver-samples/blob/master/LICENSE"
    - "https://learn.microsoft.com/en-us/samples/microsoft/windows-driver-samples/sysvad-virtual-audio-device-driver-sample/"
    - "https://github.com/VirtualDrivers/Virtual-Audio-Driver"
    - "https://docs.rs/crate/tympan-apo/latest/source/docs/decisions/0001-ci-verification-strategy.md"
    - "https://github.com/actions/partner-runner-images/issues/137"
  defers: []
  deferred-by: []
---

# Issue #952 virtual audio/MIDI endpoint driver method survey and GitHub-hosted runner feasibility

## Summary

Issue #952 の調査 slice として、ruviform 独自の仮想 Audio/MIDI endpoint を Windows 上で提供する方式を洗い出し、各方式の権限・署名・install・再起動要件と GitHub-hosted Windows runner での成立可否を一次情報で判定した。結論: **user-mode での公式な仮想 audio endpoint 生成経路は存在しない**（kernel driver が唯一経路）。kernel driver は `bcdedit /set testsigning on` が再起動を必須とし、ephemeral runner の job 内 reboot は runner session を kill するため **test-signing 経路は GitHub-hosted で不成立**。残る経路は **Dev Portal 署名（attestation/WHQL）取得済みの独自 driver** で、こちらは管理者権限・image 同梱 WDK・署名 API 自動化・reboot 不要 install の全てが揃うため**条件付き成立** — 条件は EV 証明書 + Hardware Dev Program アカウントという repo 外調達。MIDI 側は加えて **`windows-11-arm`（Win11 25H2）image に in-box な Windows MIDI Services loopback** という driver 不要の経路が見つかった — こちらは runner 実測未確認項目が残るため同じく条件付きだが、成立すれば外部調達ゼロで GitHub-hosted に載る。詳細な方式選択と slice 分割は `docs/designs/issue-952-virtual-endpoint-driver.md` に記録する。

## Exploration Goal

Issue [#952](https://github.com/scarecrow173/ruviform/issues/952) のスコープ 1・2 は、rf#629（外部 driver 前提の仮想 endpoint provisioning）が rf#762（専用 self-hosted runner 調達）に blockedBy で待機していた経緯を受け、「外部ドライバを入れるのではなく ruviform 側にテスト用仮想ドライバを独自実装する」方針転換に伴う。本探索が答える問い:

- Windows で仮想 Audio/MIDI endpoint を提供する方式の選択肢と、各方式の権限・署名・install・再起動要件は何か。
- それらの方式は GitHub-hosted Windows runner で provisioning → 既存 lane 実走まで完結するか（成立 / 不成立 / 条件付き成立）。
- 独自実装が「既存 verifier が列挙・開放・streaming できる実 OS endpoint として見える」ために必要な条件は何か。

方式選択とその根拠・slice 分割案は design doc の責務であり、本書は調査結果と一次根拠の記録に留める。

## Key Issues and Assumptions

### 上流で確定済み（confirmed — 本 slice で再決定しない）

- **Issue #629 で確立した repo 資産は外部 driver 前提のまま変更しない**: capability probe 2 種（`endpoint_probe`・`midi_endpoint_probe`）、verifier 2 種（`tests/virtual_endpoint.rs`・`tests/loopback.rs`）、opt-in manual lane 4 種（`test-audio-virtual-endpoint`・`test-midi-virtual-endpoint`・`test-os-endpoint-wiring`・`test-os-endpoint-midi-e2e`）、provisioning runbook・capability matrix・`verdict: NotRun`/`PROVISIONED`/`environment_kind=virtual-os-endpoint` 語彙。本 slice は docs + evidence のみで、これらのコード・lane 定義には触れない。
- **fail-closed 規則**: `verdict: NotRun`/`NOT_PROVISIONED` を Pass と主張しない（POLICIES §3、Issue #627 contract）。本 slice の判定も同じ規則で「成立しないものを成立と書かない」。
- **「独自実装」の解釈範囲**: task spec の前提節に従い、自前 kernel driver に限定せず、OS in-box 機構（Windows MIDI Services loopback 等）を ruviform 資産（provisioning 自動化・verifier 接続）として採る選択肢も評価対象とした。
- **外部 driver の確定済み制約**（rf#762 探索より — 本調査でも一次情報の整合を確認）: loopMIDI は私的非商用限定 license、VB-CABLE は donationware（商用判定要確認）、VAC は有償、VB-CABLE install は OS 再起動必須。Issue 方針「外部ドライバを入れない」はこうした install/license/再起動コストの排除が意図であり、本調査では外部 driver そのものを方式候補から外した（自前 driver で置き換える場合の比較対象としてのみ言及）。

### コード観測事実（confirmed — 読取と probe 実走で確認）

- **Audio verifier の pair 判定は name-hint + stem match 二段構成**: `crates/ruviform-audio-device/tests/virtual_endpoint.rs` の `VIRTUAL_NAME_HINTS`（`"vb-cable"`/`"vb audio"`/`"vb-audio"`/`"cable input"`/`"cable output"`/`"hi-fi cable"`/`"hifi-cable"`/`"virtual audio cable"`/`"voicemeeter"`）に friendly name が部分一致し、方向語（`input`/`output`/`render`/`capture`/`in`/`out`）を剥がした stem が render↔capture で一致することが pair 条件。**独自 driver の endpoint 名を `"Ruviform Cable Input"`/`"Ruviform Cable Output"` 系にすれば、既存 hint に無改造で乗る**（stem = `ruviform cable` で一致）。独自名を使う場合は hint 追加の crates 変更が要る。
- **MIDI verifier の pair 判定は双方向同名 match のみ**: `crates/ruviform-midi-device/tests/loopback.rs` は `info().name` が midiIn/midiOut 両方向に出る port を candidate とし、hint list を持たない。**同名で両方向に見える port を提供する方式なら crates 変更なしで動く**（loopMIDI・WMS Basic Loopback の共通 signature）。
- **`midi_endpoint_probe` は WMS 判定を既に持つ**: `New-MidiLoopback` cmdlet の有無と `midisrv` service の状態を best-effort で報告する（`print_wms_availability`）— WMS 経路の provisioning 判定にそのまま使える。
- **本ホスト実測（Windows 11 `10.0.26300.9974` x86_64、非昇格ユーザー）**: `midisrv` Running（`midisrv.exe` 10.0.26100.9949 — WMS in-box 存在）、`New-MidiLoopback` cmdlet absent（tools/SDK 配布側の表面）、`AudioSrv`/`AudioEndpointBuilder` Running。MMDevice 65 endpoint・仮想 candidate 0 件、WinMM input 0/output 1（GS Wavetable）・loopback candidate 0 件、両 probe とも `NOT_PROVISIONED`（evidence: `evidence/rf952/`）。
- **`bcdedit`/`pnputil` は非昇格ユーザーでは query すら拒否**: task spec どおり破壊的操作は実行せず、`net session` 失敗による非昇格の確認とサービス状態・probe の query 系に限定した（evidence: `local-queries-windows-local.txt`）。

## Alternatives and Comparison

### Audio — kernel driver 系

| 方式 | 権限 | 署名要件 | install/再起動 | ephemeral での provisioning/cleanup | license/再配布 | 保守コスト | GitHub-hosted 判定 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **SysVAD 派生 driver + test-signing** | admin（runner で満たす） | self-sign + `bcdedit /set testsigning on` | devcon/pnputil install + **OS 再起動必須** | 不可能（job 内 reboot は runner session を kill → job 失敗） | MS-PL（派生・再配布可） | driver 実装・WDK build・INF 保守 | **不成立**（再起動不可制約に衝突。Secure Boot 有効環境では flag 設定自体も拒否） |
| **SysVAD 派生 driver + attestation 署名** | admin + Partner Center | Dev Portal attestation（EV 証明書 + Hardware Dev Program アカウント前提、HLK 不要・testing 限定・retail WU 配布不可） | pnputil/devcon で reboot 不要 install | 可能（install → probe → lane → uninstall を job 内で完結） | 同上 + 署名済み binary は自社配布 | driver + 署名 pipeline（Hardware API/SDCM で自動化可能） | **条件付き成立**（EV 証明書調達 + Partner Center 開設という外部 gate） |
| **SysVAD 派生 driver + WHQL/HLK** | 同上 + HLK 環境 | HLK テスト pass + Dev Portal | 同上 | 同上 | 同上 | attestation に HLK 実行・管理が乗る | **条件付き成立**（attestation より重い — 本用途には過剰） |
| **AVStream/KS・KMDF 仮想デバイス** | 同上 | 同上（kernel driver なら同じ規則） | 同上 | 同上 | 同上 | PortCls より実装自由度は高いが保守も重い | 署名経路は上記と同じ判定 |

### Audio — user-mode 系（公式経路の有無の確認）

| 経路 | 評価 | 根拠 |
| --- | --- | --- |
| MMDevice/WASAPI から仮想 endpoint を user-mode で生成 | **方式自体が不在** | endpoint device は PnP で登録された kernel adapter device から endpoint manager が生成する（Microsoft Learn Enumerating Audio Devices）。「Audio Engine only enumerates kernel devices」（OSR NTDEV・Microsoft Q&A が一致）。core audio API は kernel-mode audio driver と user-mode client を分離する層であり、endpoint 生成 API を持たない |
| Terminal Services AudioEndpoint（rdpendp・Remote Audio） | **不採用** | in-box 機構だが endpoint は RDP session 内にのみ現れ、runner の実行コンテキストから列挙できない。session 確立自体が job 外工程 |
| WASAPI render loopback capture | **不採用** | 既存 endpoint の render stream を capture する機能で、仮想 endpoint を生成しない。runner には endpoint 0 件 |
| SWD（software device）API | **不採用** | user-mode で software device node は作れるが、audio endpoint として振る舞う function driver は依然 kernel-mode 必須 |
| UMDF | **不採用** | user-mode driver framework だが audio device class（KS/PortCls/AVStream）は kernel-mode が必須 |
| APO（Audio Processing Object） | **不採用** | `audiodg.exe` に load される effect で、既存 MMDevice endpoint への attach が前提。endpoint を生成しない |

### MIDI — kernel driver 系

| 方式 | 権限 | 署名要件 | install/再起動 | ephemeral での provisioning/cleanup | license/再配布 | 保守コスト | GitHub-hosted 判定 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **独自 WinMM 仮想 port driver（loopMIDI 同型）+ test-signing** | admin | self-sign + testsigning flag | driver install + **再起動必須** | 不可能 | 自前（license 問題なし） | kernel driver 実装（PortCls/DMus 系 MIDI miniport または仮想 device + wdmaud 登録） | **不成立**（Audio と同じ再起動不可制約） |
| **同上 + attestation 署名** | admin + Partner Center | Dev Portal attestation | reboot 不要 install | 可能 | 同上 | driver + 署名 pipeline | **条件付き成立**（Audio と同一の外部 gate） |

### MIDI — user-mode 系

| 方式 | 権限 | 追加 install | OS build 依存 | WinMM 互換 | ephemeral での provisioning/cleanup | GitHub-hosted 判定 |
| --- | --- | --- | --- | --- | --- | --- |
| **WMS Basic Loopback（BLOOP）** | service への config update（admin 相当） | WMS in-box が前提。BLOOP transport の同梱可否は image 実測要（2025-11 時点では preview transports 配布側に記録） | **Win11 25H2+ desktop のみ** | `wdmaud2.drv` 互換層で WinMM に同名 in/out pair として出る（loopMIDI 同型 signature — verifier 無改造で候補になる見込み） | `midi` console・PowerShell module・`Windows.Devices.Midi2` API で transient endpoint 作成 → service restart で消える（job 内完結）。**config JSON 直書きは非サポートで採用不可** | **`windows-11-arm`（Win11 25H2 client）で条件付き成立**。x64 Server image では方式自体が不在 |
| **WMS MIDI 2.0 Loopback（LOOP）** | 同上 | in-box | 同上 | 同上だが A/B pair 構成 | 同上 | **verifier 意味論上の危険あり**: endpoint "A" は同名 in/out port を持つが送信は B 側に届く — `loopback.rs` の name-match candidate が "A" を掴むと round-trip せず `verdict: Fail` 化する危険。verifier 更新か BLOOP 限定が要る（後述 §接続点） |
| WinMM user-mode port（WMS 以外） | — | — | — | — | — | **方式自体が不在**（WinMM port は driver 由来。`midimap`/GS synth は user-mode で生成不能） |

### その他 API

| 経路 | 評価 |
| --- | --- |
| `windows-11-arm` image の活用 | 判定を変え得る唯一の image 差異。Win11 Enterprise 25H2（build 26200）で WMS in-box 条件を満たす。x64 lane 資産とは別 arch のため `aarch64-pc-windows-msvc` build 対応が別途要確認 |
| GS Wavetable Synth | in-box WinMM output port だが output-only で loopback pair を構成しない — round-trip verifier の対象外（`real_device_silent_output_smoke` の対象としては既存どおり） |
| RDP Remote Audio / Server 系 in-box 機構 | Audio §user-mode 系で棄却済み |

## GitHub-hosted Windows runner 成立性（一次根拠）

### 権限・環境

- runner は **administrators 実行 + UAC disabled**（公式 docs）— `bcdedit`・`pnputil`・`devcon`・service 操作の権限要件は満たす。
- **ephemeral VM**: job ごとに新規 VM、終了後破棄。状態の持ち越し不可。
- **job 内 reboot は成立しない**: Runner.Listener の session を kill し、job は shutdown signal/communication lost で失敗。reboot 後に job を継続する公式手段は存在しない（self-hosted + autologon でのみ可能）。
- **runner image**:
  - `windows-2022` = Windows Server 2022（10.0.20348）、`windows-2025`/`windows-latest` = Windows Server 2025（10.0.26100）。**Server SKU**。
  - **`windows-11-arm`/`windows-11-vs2026-arm` = Windows 11 Enterprise 25H2（10.0.26200, arm64）** — クライアント SKU で WMS in-box 条件を満たす。
  - 両 Windows image に **WDK component（10.0.26100.16）+ VS Enterprise** 同梱 — driver の build・`devcon`/`signtool`/`infverif` は job 内で利用可能。
- **Server SKU の audio/MIDI 状況**: AudioSrv は Server 2022 で Manual+Stopped（start は可能）、Server 2025 で Automatic+Running（別プロジェクトの CI 検証 ADR が「hosted runners expose no physical or virtual audio endpoint」を実測として記録）。audio device 不在のため endpoint 0 件。WMS は desktop SKU 限定で Server には存在しない。

### 署名要件の連鎖

1. Windows 10 1607 以降（fresh install + Secure Boot ON）では **Dev Portal 経由の Microsoft 署名なしに新規 kernel driver は load されない**。例外は Secure Boot OFF + cross-signed だが、cross-signing の新規取得は不可。
2. test-signing は `bcdedit /set testsigning on` → **OS 再起動で初めて有効**（Secure Boot 有効時は flag 設定自体も拒否）。
3. ephemeral runner は job 内 reboot 不能 → **test-signed driver は GitHub-hosted で成立しない**（連鎖が実際に成立する: flag → reboot 必須 → reboot は job kill）。
4. Dev Portal 署名は (a) attestation（HLK 不要・testing 限定）または (b) WHQL（HLK 必須）。**attestation は Hardware REST API / SDCM で CI 自動化可能**（virtio-win・Oracle の実績）。前提は EV 証明書 + Hardware Dev Program アカウント（Entra ID + Manager ロール）— repo 外の外部調達。
5. 署名済み driver の install は **reboot 不要**（runner-images maintainer が Scream 署名 driver の job 内 install を推奨回答した実績、VB-CABLE を job 内 install する公開 workflow 実績あり）。

### 判定（Audio/MIDI 別・3 値）

| 対象 | 判定 | 理由 |
| --- | --- | --- |
| **Audio — user-mode 経路** | **不成立** | 公式の user-mode 仮想 audio endpoint 生成経路が存在しない（一次情報で確定）。runner 制約以前の問題 |
| **Audio — 独自 driver + test-signing** | **不成立** | `testsigning` flag → 再起動必須 → ephemeral job 内 reboot は runner session を kill |
| **Audio — 独自 driver + Dev Portal 署名** | **条件付き成立** | 技術要件は全て満たす（admin・WDK 同梱・署名 API 自動化・reboot 不要 install）。条件: EV 証明書調達 + Hardware Dev Program アカウント + Server SKU での install 実測確認 |
| **MIDI — WMS loopback on `windows-11-arm`** | **条件付き成立** | in-box service で driver 不要。条件: image 上の midisrv 有効状態・BLOOP 同梱可否・作成経路（console/PowerShell/API）の動作・**verifier 意味論との整合（LOOP A/B の Fail 化危険）**・arm64 build 対応 — いずれも runner 実測待ち |
| **MIDI — WMS loopback on x64 Server image** | **不成立** | WMS は Win11 desktop 25H2+ 限定。Server 2025（26100・24H2 系）は SKU・世代とも対象外 |
| **MIDI — 独自 driver + test-signing** | **不成立** | Audio と同一の連鎖 |
| **MIDI — 独自 driver + Dev Portal 署名** | **条件付き成立** | Audio と同一の外部 gate |
| **MIDI — GS Wavetable 等 in-box port** | **不成立** | output-only で loopback pair を構成しない |

## 既存コードとの接続点（読取のみ — 実装 slice への入力）

- **Audio pair 判定**: endpoint 名を `* Cable Input`/`* Cable Output` 系にすれば `VIRTUAL_NAME_HINTS` 無改造で検出可能（`"cable input"`/`"cable output"` hint が部分一致）。任意名なら hint 追加（`endpoint_probe.rs`・`virtual_endpoint.rs` 両方の手動同期が必要 — コメントで同期義務が明記済み）。
- **MIDI pair 判定**: 同名双方向 port なら変更なし。WMS LOOP A/B を使う場合は `loopback.rs`/`os_endpoint.rs`/`os_endpoint_midi_e2e.rs` の candidate 意味論を見直す必要（A-out→B-in の送信相手を正しく扱うか、BLOOP のみを対象にする pair フィルタ追加）。
- **probe**: `midi_endpoint_probe` の WMS 判定（`midisrv` + `New-MidiLoopback`）は WMS 経路の判定にそのまま利用可。`endpoint_probe` の hint/署名観測も自前 driver をそのまま対象化できる。
- **lane**: provisioning さえ揃えば既存 4 manual lane が canonical 入口として不変。provisioning 手順自体（driver install・WMS endpoint 作成）は lane 前工程として runbook 側に追加する構成が自然。

## Open Questions（runner 実測が必須の項目 — 実装 slice 分割案へ引き継ぐ）

1. `windows-11-arm` image で `midisrv` は有効か（phased rollout 対象・既定有効状態）。`midi` console/PowerShell module/BLOOP transport の同梱可否。
2. BLOOP transport が GA 後の 25H2/26H2 image で in-box か、依然 preview transports 配布が要るか — install 経路（tools pkg・winget・GitHub release）の job 内自動化可否。
3. WMS loopback endpoint が WinMM `midiInGetNumDevs`/`midiOutGetNumDevs` にどう見えるか（同名双方向 port として出るか・出現遅延）— `midi_endpoint_probe` 相当の query を runner 上で実測。
4. LOOP A/B pair が verifier に `Fail` を出すか、`NotRun` 回避には BLOOP 作成または verifier 更新のどちらが要るか。
5. Dev Portal attestation-signed driver が Server 2025 SKU で pnputil/devcon install 可能か（virtio 系実績はあるが自前 driver での実測）。
6. runner VM の Secure Boot 有効状態（testsigning 経路の完全な閉塞確認 — 判定自体は変わらないが evidence として）。
7. `aarch64-pc-windows-msvc` での workspace build・対象クレート compile 可否。
8. EV 証明書・Hardware Dev Program の調達可否（組織/個人での開設条件 — repo 外工程）。

## Evidence

<!-- Append-only: dated entries, never edit/reorder/delete existing. -->

### 2026-11-25 — Web 一次情報収集 + ローカル query 系実測（この worktree の Windows ホスト）

環境: Windows 11 `10.0.26300.9974` x86_64（非昇格ユーザー `akiyoshi`、driver 未 provisioning）。

- `mise exec -- cargo run -p ruviform-audio-device --example endpoint_probe` → exit 0。MMDevice 65 endpoint、仮想 candidate 0 件、`provisioning: NOT_PROVISIONED`。`evidence/rf952/endpoint_probe-windows-local.txt` に全文保存。
- `mise exec -- cargo run -p ruviform-midi-device --example midi_endpoint_probe` → exit 0。WinMM input 0/output 1（GS Wavetable）、loopback candidate 0 件、`midisrv: Running` + `New-MidiLoopback: absent`、`NOT_PROVISIONED`。`evidence/rf952/midi_endpoint_probe-windows-local.txt` に全文保存。
- サービス・権限 query（`evidence/rf952/local-queries-windows-local.txt`）: `AudioSrv`/`AudioEndpointBuilder` Running(Automatic)、`midisrv` Running(Manual, 10.0.26100.9949)。`net session` は system error 5（非昇格）、`bcdedit /enum`・`pnputil /enum-drivers` も権限不足で拒否 — task spec の破壊的操作禁止に従い query 系のみで終了。
- Web 一次情報の URL・取得日・要点一覧は `evidence/rf952/web-sources-signing-runner-wms.md` に記録（署名要件・runner 事実・WMS・user-mode audio・SysVAD license・`windows-11-arm` image・loopback 作成経路）。
