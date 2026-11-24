# evidence/rf952 — Web 一次情報収集記録（署名要件・GitHub-hosted runner・WMS・user-mode audio）

Issue #952 方式調査の一次根拠。収集日: **2026-11-25**。収集手段: Web 検索 + 各一次ソースの直接取得（ローカル query 系実測は `local-queries-windows-local.txt`・probe 実走は `endpoint_probe-windows-local.txt` / `midi_endpoint_probe-windows-local.txt` を参照）。

## 1. Kernel driver 署名要件（Microsoft Learn / 公式）

### 1.1 1607 以降の署名強制

- https://learn.microsoft.com/en-us/windows-hardware/drivers/install/kernel-mode-code-signing-policy--windows-vista-and-later- （取得 2026-11-25）
  - 「Starting with Windows 10, version 1607, Windows will not load any new kernel-mode drivers which are not signed by the Dev Portal」— 新規 kernel driver は Dev Portal（Hardware Dev Center）経由の Microsoft 署名が必須。
  - 例外（cross-signed が通るケース）: 1607 以前からの OS アップグレード / **Secure Boot が BIOS で OFF** / 2015-07-29 以前発行で対応 CA にチェーンする証明書。新規 driver は実質 Dev Portal 必須。
  - 署名の取得経路: HLK/HCK テスト + WHQL 提出（本番）、または **attestation signing**（HLK 不要・テスト用途のみ）。
  - Dev Center アカウント開設には **EV code signing 証明書が必須**。
- https://learn.microsoft.com/en-us/windows-hardware/drivers/install/kernel-mode-code-signing-requirements--windows-vista-and-later- （取得 2026-11-25）
  - 同上: 「Valid signatures can be obtained by either Hardware Certification or Attestation」。
- https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/driver-signing-offerings （取得 2026-11-25）
  - Attestation signing の制約: 「For testing purposes only」「Attestation signing only works on Windows 10 Desktop and later versions of Windows」「requires the use of an extended validation certificate to submit the driver to Partner Center」「can't be published to Windows Update for retail audiences」。
- https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-attestation （取得 2026-11-25）
  - 前提: Hardware Developer Program 登録 + EV 証明書。

### 1.2 test signing（testsigned driver）の要件

- `bcdedit /set testsigning on`（または `TESTSIGNING` BCD option）は **OS 再起動後にのみ有効** — test-signed driver の load には再起動が必須（Microsoft Learn test-signing 手順系 docs・SysVAD sample の install 手順でも boot config 変更 + reboot が必須工程として記載。`docs/designs/issue-629-provisioning-runbook.md` §3.6 も同要件を記録済み）。
- Secure Boot 有効時は `bcdedit /set testsigning on` 自体が拒否される（test-signing mode は Secure Boot と両立しない）。
- 補足: cross-signing は新規取得不可（Microsoft が cross-signed CA 証明書の新規発行を停止済み）のため、「Secure Boot OFF + cross-signed で Dev Portal 回避」は新規 driver では現実的に成立しない。

### 1.3 署名提出の自動化可否

- https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/dashboard-api （取得 2026-11-25）
  - Microsoft Hardware API: `manage.devcenter.microsoft.com/v2.0/my/hardware/products` — product 作成・submission 作成・commit を REST API で自動化可能。前提: Hardware Partner Center プログラム所属 + Microsoft Entra ID アプリケーション（Global admin で作成・Manager ロール付与）。
- https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/manage-product-submissions （取得 2026-11-25）
  - product → submission → commit の REST 手順（attestation submission も同 API で処理可能）。
- https://github.com/virtio-win/SDCM （取得 2026-11-25）
  - Surface Dev Center Manager（SDCM）— Partner Center REST API を使った署名提出自動化の OSS ツール実例（virtio-win プロジェクト）。attestation/WHQL product・submission・signed package download・shipping label を自動化。
- https://blogs.oracle.com/linux/automatic-attestation-signing （取得 2026-11-25）
  - Oracle の attestation signing 自動化実例: EV 証明書で CAB を署名 → API で submission。stage: Package Acceptance → Preparation → Scanning → Validation → Catalog Creation → Sign → Finalize。

**判定材料**: 自前 driver を GitHub-hosted runner に reboot なしで install する唯一の経路は **Dev Portal 署名（attestation または WHQL）取得済みの driver package**。署名そのものは CI 自動化可能だが、前提として (a) EV code signing 証明書の調達（有償・組織/個人実在の審査・ハードウェアトークン）、(b) Hardware Dev Program アカウント（Microsoft Entra ID + 審査）という **repo 外の外部調達ゲート** がある。

## 2. GitHub-hosted Windows runner の事実

### 2.1 権限・UAC（公式）

- https://docs.github.com/en/actions/reference/runners/github-hosted-runners （取得 2026-11-25）
  - 「Windows virtual machines are configured to run as **administrators with User Account Control (UAC) disabled**」— runner 上の手順は管理者権限で実行可能（`bcdedit`・`pnputil`・`devcon`・service 操作の権限要件は満たす）。
  - 「each GitHub-hosted runner is a new virtual machine (VM) hosted by GitHub」— job ごとに新規 VM が割り当てられ、job 終了後に破棄される ephemeral 環境。

### 2.2 runner image の OS build・同梱ソフトウェア

- https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-Readme.md （取得 2026-11-25、image version 20261116.335.1）
  - `windows-2025` = **Windows Server 2025, OS Version: 10.0.26100 Build 33828**。
  - Visual Studio Enterprise 2022 17.14.37906.3 同梱で、workload に **`Component.Microsoft.Windows.DriverKit 10.0.26100.16` + `Windows Driver Kit Visual Studio Extension 10.0.26100.16`** — WDK が image にプリインストール済み（driver のビルド・`devcon`/`signtool`/`infverif` は job 内で利用可能）。
- https://raw.githubusercontent.com/actions/runner-images/main/images/windows/Windows2022-Readme.md （取得 2026-11-25、image version 20261116.370.1）
  - `windows-2022` = **Windows Server 2022, OS Version: 10.0.20348 Build 5870**。同じく DriverKit component 同梱。
  - `windows-latest` は 2026-11 時点で windows-2025 系列を指す（runner-images の label 表）。

### 2.3 job 内 OS reboot の挙動

- job 中の `Restart-Computer`/`shutdown /r` は Runner.Listener（long-poll session）を kill する → job は `The runner has received a shutdown signal` / communication lost で失敗・キャンセルとなる。実績:
  - https://github.com/actions/runner/issues/3677 （self-hosted での reboot → session 喪失 → "session already exists" / shutdown の振る舞いを確認する報告、2025-01）
  - https://compiler.dev/errors/runner-received-a-shutdown-signal （「runner service was stopped or the machine rebooted」→ job canceled の説明）
  - GitHub-hosted では VM は ephemeral で再起動後の session 復帰・job 継続の公式手段は**存在しない**（docs.github.com の workflow continuity 記述は「job が runner に割り当てられてから完了まで」の単一セッション前提）。「reboot してから続きを実行する」構成は self-hosted runner + autologon/service 起動でのみ成立。
- 裏取り（runner-images maintainer による署名 driver インストール推奨）:
  - https://github.com/actions/runner-images/issues/2528 — 「Virtual sound device on Windows Instance」: maintainer (al-cheb) が Scream（署名済み仮想 audio driver）の job 内インストール手順を提示（`Start-Service audio*` → cert を TrustedPublisher へ追加 → install）。**production 署名済み kernel driver は job 内 install が動作する実績**（reboot 不要経路の傍証）。
  - https://github.com/writingmate/aidictation/blob/main/.github/workflows/windows-audio-e2e.yml — `windows-latest` 上で VB-CABLE を cert pre-trust + install する workflow 実例（外部 driver の CI install 実績）。

### 2.4 Server SKU の audio/MIDI サービス状態

- https://docs.vultr.com/enable-windows-audio-on-a-windows-server-instance / https://introserv.com/fr/tutorials/microphone-forwarding-to-an-rdp-server-on-windows/ （取得 2026-11-25）
  - Windows Server（Desktop Experience）には Windows Audio（AudioSrv）スタックが存在するが既定で無効/手動（Server 2022 は Manual+Stopped、Server 2025 は Automatic+Running とされる）。`sc config Audiosrv start=auto` + start は管理者権限で可能。ただし **audio device が存在しない限り endpoint は 0 件**。
- https://docs.rs/crate/tympan-apo/latest/source/docs/decisions/0001-ci-verification-strategy.md （取得 2026-11-25）
  - 別プロジェクト（Rust/APO）の同目的 CI 検証 ADR: 「Windows Server SKUs have the Windows Audio Service disabled by default, and **hosted runners expose no physical or virtual audio endpoint**」「`audiodg.exe`-level loading is not available on hosted runners」— GitHub-hosted runner に audio endpoint が存在しないことの独立した実測記述。
  - 同 ADR: 「GitHub-hosted Windows Server 2025 runners include Visual Studio Enterprise 2022 and the Windows Driver Kit extension out of the box. `dumpbin`, `infverif`, `signtool`, `regsvr32` are all in PATH」— §2.2 の WDK 同梱を corroborate。

## 3. Windows MIDI Services（WMS）

- https://blogs.windows.com/windowsexperience/2026/02/17/making-music-with-midi-just-got-a-real-boost-in-windows-11/ （2026-02-17 公開、取得 2026-11-25）
  - **WMS GA**: 「Windows 11 now supports both MIDI 1.0 and MIDI 2.0 through Windows MIDI Services」「core components are built into Windows 11, rolling out through a phased enablement process now to in-support retail releases of **Windows 11**」— 提供対象は Windows 11 クライアント SKU。
- https://microsoft.github.io/MIDI/kb/minimum-requirements/ （取得 2026-11-25）
  - 「Windows MIDI Services will run on the latest supported 64 bit **desktop versions** (Arm and Intel/AMD) of Windows 11」「The Windows MIDI Services API (`Windows.Devices.Midi2`) needs **Windows 11 25H2 or later**. Windows MIDI Services on Windows 11 24H2 is an older version that doesn't get fixes, and 24H2 never gets the API」— **Server SKU は対象外**。Server 2025（build 26100）は SKU・build 両面で対象外。
- https://github.com/microsoft/MIDI/releases/ （In-box Preview 14, 2025-11 公開、取得 2026-11-25）
  - 「The Windows MIDI Services service and transports for USB MIDI 1.0, USB MIDI 2.0, **MIDI 2.0 loopbacks** is already in Windows 11 25h2 and higher」— service + USB + LOOP は in-box。
  - preview Transports package（GitHub 配布）に **General MIDI Synth / Basic MIDI 1.0 Loopback Transport / RTP / Network / BLE** を同梱 — **MIDI 1.0 Basic Loopback（BLOOP）は 2026-11 時点で preview transport 扱い**（in-box 化未了または配布形態が tools 側）。「Windows MIDI Services works only on Windows 11 25h2 and later. Installing these packages on older versions … is not supported」。
- https://microsoft.github.io/MIDI/tools/midiloopbacksetup/ （取得 2026-11-25）
  - MIDI 1.0 Basic Loopback = **単一 endpoint（同名の in/out port 対）**で「older loopback drivers と同じ見え方」— loopMIDI 同型の bidirectional name match signature。MIDI 2.0 Loopback = A/B pair（2 つの別名 endpoint）。「Both types of loopbacks create MIDI 1.0 API ports. The Basic Loopback creates an In (Source) and Out (Destination) pair of ports」。
  - 作成経路: MIDI Settings app / `New-MidiLoopback`・`New-MidiBasicLoopback` PowerShell cmdlet（tools/SDK 配布物）/ service configuration JSON・`Windows.Devices.Midi2` API（BLOOP 作成は `Transports.BasicLoopback` namespace）。
- https://microsoft.github.io/MIDI/kb/api-back-compat/ （取得 2026-11-25）
  - WinMM 互換表: 「Send/Receive with Basic and MIDI 2.0 Loopback MIDI Devices」= WinMM **✅**（`wdmaud2.drv` 互換層経由で legacy app に見える）/ 「Create Basic and MIDI 2.0 Loopback MIDI Devices」= WinMM **❌**（作成は新 API/service 側）。**BLOOP/LOOP endpoint は WinMM 列挙に見える** — 既存 `ruviform-midi-device` の WinMM backend から無改修で見える見込み（実測未実施・runner 非搭載のため本 slice では実証不可）。
- https://microsoft.github.io/MIDI/kb/moving-from-winmm-to-wms/ （取得 2026-11-25）
  - `MidiApi.EnsureServiceAvailable()` で service 有無を判定 — 25H2 未満 / Legacy mode / service 停止時は fallback が必要。UMP endpoint → MIDI 1.0 port 化は非同期で数秒かかる場合あり。
- https://microsoft.github.io/MIDI/kb/midi-devices-in-device-manager/ （取得 2026-11-25）
  - 「Service Test Loopback A / B / Service Test Ping (Internal)」は service の self-check 用に常時存在するが「Most music apps never show them」— WinMM port として露出するかは未確認。

### 3.1 loopback endpoint の作成経路（headless provisioning 可否の判定材料）

- https://microsoft.github.io/MIDI/kb/config-json/ （取得 2026-11-25）
  - **service config JSON への直接書込は非サポート**（「Applications, scripts and tools must never read or write the configuration file directly」「No product from outside Microsoft may manipulate the configuration file directly」）。サポート経路は `Windows.Devices.Midi2.ServiceConfig` API・MIDI Settings app・`midi` console utility・PowerShell module のみ — **provisioning を config ファイル書換で済ませる経路は採用不可**。
- https://microsoft.github.io/MIDI/tools/console/ （取得 2026-11-25）
  - `midi loopback create --name-a/-a --name-b/-b [--root-name] [--save-to-config]` — console ツールで transient loopback pair を作成可能（`--save-to-config` なしなら service restart で消える = CI の ephemeral 利用に適する）。basic loopback も同様の create 経路あり。
- https://microsoft.github.io/MIDI/tools/powershell/ （取得 2026-11-25）
  - `New-MidiLoopback -BaseName "X"`（A/B pair・base name に ` (A)`/` (B)` 接尾）・`New-MidiBasicLoopback -Name "X"`（単一 endpoint）— transient が既定、`-SaveToConfiguration` で永続化。
- https://microsoft.github.io/MIDI/sdk-reference/Transports/Loopback/MidiLoopbackManager/ + https://microsoft.github.io/MIDI/sdk-reference/ServiceConfig/MidiServiceTransportPluginConfigManager/ （取得 2026-11-25）
  - `Windows.Devices.Midi2` API（`MidiLoopbackCreationConfig` + `MidiServiceTransportPluginConfigManager.SendUpdate/SaveUpdate`）からの作成経路 — API は 2026-02 GA 時点で preview（GitHub の App SDK NuGet 配布）。
- **重要な verifier 意味論差**: `midi` console doc — 「A MIDI 2.0 loopback is a pair of endpoints, A and B. What's sent to A arrives at B, and what's sent to B arrives at A. Each pair also gets MIDI 1.0 ports」+ PowerShell doc — base name に ` (A)`/` (B)` 接尾。既存 verifier（`tests/loopback.rs`）は「同名 in/out pair」を candidate とするため、A/B pair は各 endpoint が同名 in+out を持つ一方、send は**相手側**に届く — name-match した "A" pair に send→receive しても B 側に届き round-trip しない（`verdict: Fail` 化の危険）。BLOOP（単一 endpoint・送信が自端末に返る）のみが loopMIDI 同型の意味論。

## 4. `windows-11-arm` runner image（Win11 クライアント SKU）

- https://docs.github.com/en/actions/reference/runners/github-hosted-runners （取得 2026-11-25）
  - label 表: arm64 Windows runner は `windows-11-arm` / `windows-11-vs2026-arm`（public repo: 4 vCPU/16GB、Azure Cobalt 100 Armv9）。
- https://github.com/actions/runner-images/blob/main/images/windows/Windows11-Arm64-Readme.md （image 20261006.188.1、取得 2026-11-25）
  - `windows-11-arm` = **Windows 11 Enterprise, OS Version: 10.0.26200 Build 9457** — **Win11 クライアント SKU + 25H2（build 26200）で WMS in-box 条件を満たす**（§3 の minimum requirements「desktop versions of Windows 11」「25H2 or later」に合致）。
- https://github.com/actions/partner-runner-images/issues/137 （取得 2026-11-25）
  - 同 image は 2025-10 時点では 24H2（26100.4656）だったが、2026-02 頃に 25H2 へ更新済み（上記 readme の 26200 build が実測の現状）。
- 注意: midisrv の image 内有無・既定有効状態・BLOOP transport の同梱可否・`midi` console/tools の同梱可否は readme からは不明 — **runner 実測（probe workflow）対象**。
- x64 の既存 lane 資産・検証は x86_64 前提 — arm64 lane は別 image であり、workspace の `aarch64-pc-windows-msvc` build 対応も要確認（実装 slice の範囲見積もり項目）。

## 5. Audio の user-mode 仮想 endpoint 経路（不在の確認）

- https://learn.microsoft.com/en-us/windows/win32/coreaudio/user-mode-audio-components （取得 2026-11-25）
  - core audio API（MMDevice/WASAPI/DeviceTopology/EndpointVolume）は「kernel-mode audio drivers and audio hardware」と user-mode client を分離する薄い層 — endpoint device は物理 device のソフトウェア抽象であり、endpoint の生成は PnP で発見された adapter device → endpoint manager 登録の経路のみ。
- https://learn.microsoft.com/en-us/windows/win32/coreaudio/enumerating-audio-devices （取得 2026-11-25）
  - 「adapter devices are registered by the Plug and Play manager, in contrast with endpoint devices, which are registered by the endpoint manager」— MMDevice 列挙に出るには **kernel driver による adapter device 登録が必須**。
- https://learn.microsoft.com/en-us/answers/questions/1622324/how-to-use-iaudioendpoint-and-connect-it-to-audio （Microsoft Q&A、取得 2026-11-25）
  - 「in windows, you couldn't create an audio virtual endpoint by using audio service. If you want to create an audio virtual endpoint in windows, you need to use WDK to create windows device drivers」。
- https://community.osr.com/t/seeking-assistance-how-to-create-virtual-input-and-output-devices-in-windows-11/59143 （OSR NTDEV、2024-11、取得 2026-11-25）
  - 「If your plan is to have this work with unmodified user-mode applications, then it cannot be done in user-mode. End of story. **Audio Engine only enumerates kernel devices**」。
- 評価して棄却した周辺経路:
  - **Terminal Services AudioEndpoint API（`IAudioEndpoint` 系・rdpendp）**: RDP session 内にのみ Remote Audio endpoint が出る in-box 機構だが、(a) session 内限定で runner 実行コンテキストから列挙不可、(b) RDS session の確立自体が runner job 外工程、(c) capture 側の redirection は client 依存 — 不採用。
  - **WASAPI render loopback capture**: 既存 endpoint の render stream を capture する機能で、仮想 endpoint を生成しない（runner には endpoint 0 件）。
  - **UMDF**: user-mode driver framework だが audio device class（KS/PortCls/AVStream）は kernel-mode driver 必須 — 経路なし。
  - **SWD（software device）API**: user-mode で software device node を作れるが、audio endpoint として振る舞う function driver は依然 kernel-mode が必要 — 回避にならない。
  - **APO（Audio Processing Object）**: `audiodg.exe` に load される effect で、既存 MMDevice endpoint への attach が前提 — endpoint を生成しない（§2.4 tympan-apo ADR が同じ結論）。

## 6. SysVAD / driver サンプル license

- https://github.com/microsoft/Windows-driver-samples/blob/master/LICENSE （取得 2026-11-25）
  - **Microsoft Public License (MS-PL)** — 派生物の作成・配布を許容。SysVAD 派生の ruviform 独自 driver はライセンス上成立（`docs/designs/issue-629-provisioning-runbook.md` §3.6 の「例外経路」と同じ系譜だが、今回は第一候補として評価する立場）。
- https://github.com/microsoft/Windows-driver-samples/blob/main/audio/sysvad/README.md / https://learn.microsoft.com/en-us/samples/microsoft/windows-driver-samples/sysvad-virtual-audio-device-driver-sample/ （取得 2026-11-25）
  - SysVAD = WDM/PortCls waveRT miniport による仮想 audio device driver サンプル（speaker/mic 等の仮想 endpoint を OS 標準の MMDevice 列挙に出す）。
- 参考（同型 OSS 実例）: https://github.com/VirtualDrivers/Virtual-Audio-Driver — SysVAD/SimpleAudioSample 派生の MIT 仮想 audio driver。README が「**requires test signing to be enabled**」を明記 — test-signing 経路が再起動必須である実例。

## 7. MIDI kernel driver 系（loopMIDI 同型）の補足

- WinMM MIDI port は kernel driver（`wdmaud` port driver 経由で MIDI miniport/device を登録）に由来 — user-mode サービスが WinMM port を生成する公式経路は WMS（§3）以外に存在しない（`docs/discovery/issue-629-virtual-endpoint-survey.md` の候補表と整合 — loopMIDI/loopBE は全て署名済み kernel driver 上に構築）。
- 自前 WinMM 仮想 port 実装は PortCls/DMus 系 MIDI miniport または仮想 device + wdmaud 登録の kernel driver になるため、Audio 側と同一の署名要件がそのまま掛かる（§1）。

## 8. ローカル補助実測の対応

| 確認項目 | 結果 | 証拠 |
| --- | --- | --- |
| 本ホスト OS build | Windows 11 `10.0.26300.9974` x86_64 | `local-queries-windows-local.txt` |
| 実行ユーザーの昇格 | 非昇格（`net session` エラー 5、`bcdedit`/`pnputil` も権限不足で拒否） | 同上 |
| `AudioSrv` / `AudioEndpointBuilder` | Running / Automatic | 同上 |
| `midisrv` サービス | **Running**（Manual 起動、`midisrv.exe` 10.0.26100.9949）— WMS in-box 存在 | 同上 |
| `New-MidiLoopback` cmdlet | **absent**（tools/SDK 配布側の表面） | 同上・`midi_endpoint_probe` 出力 |
| WASAPI 仮想 endpoint | 0 件・`NOT_PROVISIONED` | `endpoint_probe-windows-local.txt` |
| WinMM loopback pair | 0 件・`NOT_PROVISIONED`（input 0 port / output = GS Wavetable のみ） | `midi_endpoint_probe-windows-local.txt` |
