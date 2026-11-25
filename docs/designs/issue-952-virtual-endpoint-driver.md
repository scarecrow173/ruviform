---
id: "DESIGN-952vEPdR1v3rDr1v3rSl1"
type: "design"
status: "draft"
title: "Issue #952 ruviform 独自仮想 Audio/MIDI endpoint — 方式選択と GitHub-hosted runner 成立可否"
created: "2026-11-25"
updated: "2026-11-25"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/952"
    - "docs/discovery/issue-952-virtual-endpoint-driver-survey.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-952-virtual-endpoint-driver.md"
      - type: "file"
        path: "docs/discovery/issue-952-virtual-endpoint-driver-survey.md"
      - type: "file"
        path: "evidence/rf952/web-sources-signing-runner-wms.md"
      - type: "file"
        path: "evidence/rf952/endpoint_probe-windows-local.txt"
      - type: "file"
        path: "evidence/rf952/midi_endpoint_probe-windows-local.txt"
      - type: "file"
        path: "evidence/rf952/local-queries-windows-local.txt"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-952-virtual-endpoint-driver-survey.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "POLICIES.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-os-endpoint-lane.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-midi-loopback.md"
    - "docs/designs/issue-629-midi-output.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-253-virtual-midi-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/952"
    - "docs/discovery/issue-952-virtual-endpoint-driver-survey.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
  defers: []
  deferred-by: []
---

# Issue #952 ruviform 独自仮想 Audio/MIDI endpoint — 方式選択と GitHub-hosted runner 成立可否

## Context

Issue #629（Windows 実 OS 仮想 Audio/MIDI Endpoint・Provisioning）で確立した資産 — capability probe 2 種・streaming verifier 2 種・opt-in manual lane 4 種・provisioning runbook・capability matrix — は全て**外部 driver 前提**（VB-CABLE/VAC/loopMIDI/WMS loopback）で構成された。外部 driver の install には管理者権限・OS 再起動・license 判断が要るため、#629 の実走は rf#762（専用 self-hosted runner 調達）に blockedBy で待機していた。

ユーザー判断により方針が転換された: 外部ドライバを入れるのではなく **ruviform 側にテスト用仮想ドライバを独自実装する**（ruviform 自体の資産にする）。独自実装が GitHub-hosted Windows runner で成立するなら、rf#762 の外部調達なしで前進できる — これが本 Issue の核心判定である。

調査結果の全容と一次根拠（Microsoft Learn・署名要件・runner image・実行した補助検証）は `docs/discovery/issue-952-virtual-endpoint-driver-survey.md` と `evidence/rf952/` に記録した。本書はその結果に基づく**方式選択・成立可否の結論・接続点・実装 slice 分割案**を status: draft として記録する。実装物（driver コード・INF・テストコード・workflow・mise lane）は本 slice では作成しない。

## Scope

### In Scope

- Audio/MIDI 別の方式調査結果の要約（詳細は discovery doc）。
- **方式選択とその根拠**。
- **GitHub-hosted runner 成立可否の結論**（Audio/MIDI 別の3値判定）。
- 既存 verifier/lane/probe との接続点（無改造で使える条件・要変更点）。
- 実装 slice 分割案（scope・依存順・必要な検証 lane・範囲見積もり）。
- Deferred Design Concerns（残課題・外部 gate・判断仰ぎ論点）。

### Out of Scope

- driver/実装コード・INF・テストコード・workflow・mise lane の新規作成・変更（全て実装 slice の分割案に記録）。
- 「GitHub-hosted 不成立」確定に伴う rf#762 向け runner 調達作業（結論のみ記録 — 再開判断は Coordinator/ユーザー）。
- docs index（`docs/**/README.md`）の更新 — 生成物であり Coordinator が統合時に整備する。

## 背景: 外部 driver 経路が塞がれた理由

rf#629 survey の確定済み制約は: VB-CABLE install は **OS 再起動必須**、loopMIDI は **私的非商用限定 license**、VAC は **有償商用**。いずれも「runner への driver 導入 + provisioning 自動化」を許す license/手順上の摩擦を持ち、GitHub-hosted の ephemeral 環境では再起動要件と衝突する。独自実装はこの3つの摩擦（license・再起動・再配布）を根こそぎ排除できる可能性を持つが、kernel driver には別の壁 — **署名要件** — がある。本調査はその壁が GitHub-hosted runner で越えられるかを一次情報で検証した。

## 方式調査の結果（要約）

### Audio

- **user-mode 経路は方式自体が存在しない**。MMDevice/WASAPI の endpoint device は、PnP マネージャが登録した kernel adapter device から endpoint manager が生成する。「Audio Engine only enumerates kernel devices」（OSR NTDEV・Microsoft Q&A が一致）。SWD API・UMDF・APO・WASAPI loopback capture・Terminal Services AudioEndpoint はいずれも仮想 endpoint を生成しない — いずれも棄却（discovery doc の表に根拠を記録）。
- **kernel driver 系**: SysVAD 派生（PortCls waveRT miniport）が第一候補。MS-PL license で派生・再配布は合法。仮想 speaker/mic pair を OS 標準の MMDevice 列挙に出せる。制御面（hotplug・format switch・device-loss 注入）は driver 自作ゆえに最も広い — rf#629 capability matrix で SysVAD が唯一 ✅ を取っていた device-loss/xrun/permission-denial の故障注入も自前 driver なら実現できる。

### MIDI

- **kernel driver 系**: loopMIDI 同型の WinMM 仮想 port driver（PortCls/DMus 系 MIDI miniport または仮想 device + wdmaud 登録）。Audio と同一の署名要件が掛かる。
- **user-mode 系**: **Windows MIDI Services（WMS）loopback が唯一の公式経路**。`midisrv` service が transport plugin で endpoint を生成 — kernel driver 不要・追加 install は tools/SDK のみ。ただし **Win11 25H2+ desktop SKU 限定**で、Server SKU には存在しない。BLOOP（MIDI 1.0 Basic Loopback = loopMIDI 同型の単一 endpoint）と LOOP（MIDI 2.0 A/B pair）の2種があり、verifier 意味論との整合で後述の差異が要る。

## GitHub-hosted runner 成立可否の結論

判定は Audio/MIDI 別に3値（成立 / 不成立 / 条件付き成立）で記録する。根拠の一次情報は全て `evidence/rf952/web-sources-signing-runner-wms.md` に URL・取得日・要点付きで保存した。

### 共通の runner 事実（確定済み一次根拠）

- **権限**: GitHub-hosted Windows runner は administrators 実行 + UAC disabled — driver install・`bcdedit`・`pnputil`/`devcon`・service 操作の権限要件は満たす。
- **再起動不可**: runner VM は ephemeral（job ごと新規・終了後破棄）。job 内 OS reboot は Runner.Listener の session を kill し job を失敗させる — reboot 後の job 継続手段は存在しない。
- **署名規則**: Windows 10 1607 以降・Secure Boot ON の fresh install では Dev Portal（Hardware Dev Center）署名なしの新規 kernel driver は load されない。`bcdedit /set testsigning on` は再起動で初めて有効（Secure Boot 有効時は flag 設定自体も拒否）。**【2026-11-24 実測で一部覆った — 下記】** runner image は **`testsigning=Yes`・Secure Boot OFF が出荷状態**（rf#955 probe 実測）のため、x64 では flag 設定・reboot なしに test-signed driver が load できる。ただし **`windows-11-arm`（ARM64）では testsigning=Yes でも kernel が load を拒否**（rf#961 attempt-9 で CI event 3004 + VBS policy 稼働を実測 — VBS 配下の Code Integrity は test-signing 緩和を honor しない。`hypervisorlaunchtype=Off`+reboot が必要で job 内不可）。
- **image**: `windows-2022` = Server 2022（10.0.20348）、`windows-2025`/`windows-latest` = Server 2025（10.0.26100）。**`windows-11-arm`/`windows-11-vs2026-arm` = Windows 11 Enterprise 25H2（10.0.26200, arm64）** — WMS の in-box 条件（Win11 desktop 25H2+）を満たす唯一の hosted image。**【2026-11-24 実測訂正】** WDK は `windows-11-arm` に headers/libs/Inf2Cat 同梱（DriverKit task dll は 17.0 のみ — MSBuild 18 対応に rename workaround が必要・実測で成立）だが **`windows-2025` は WDK 未導入**（km headers/libs なし — `winget install Microsoft.WindowsWDK.10.0.26100` の job 内導入経路が存在し実測で成立）。`devcon` は両 leg とも非搭載（setupapi/newdev P/Invoke で代替可・実測済み）。
- **署名済み driver の install は reboot 不要**: runner-images maintainer が署名済み仮想 audio driver（Scream）の job 内 install を推奨した実績、VB-CABLE を job 内 install する公開 workflow 実績あり。
- **Server SKU**: AudioSrv は存在するが（Server 2025 では既定 Running）audio device 0 件で endpoint は 0 件 — 別プロジェクトの CI 検証 ADR が「hosted runners expose no physical or virtual audio endpoint」を実測として記録。WMS は Server には存在しない。

### Audio — **x64 は test-signing で成立（実測済み）/ arm64 は要外部調達**

| 経路 | 判定 | 理由 |
| --- | --- | --- |
| user-mode 仮想 endpoint | **不成立** | 公式経路が存在しない（runner 制約以前に方式自体が不在） |
| 独自 driver + test-signing on `windows-2025`（x64） | **成立（2026-11-24 実測）** | image が `testsigning=Yes`+Secure Boot OFF 出荷のため reboot 不要。rf#961 attempt-8 で自己署名 cert → Root/TrustedPublisher 登録 → `.sys`+`.cat` 署名 → pnputil install → devnode → **kernel driver Running** まで全経路を実測（evidence/rf961/）。WDK は winget で job 内導入（実測 174s） |
| 独自 driver + test-signing on `windows-11-arm`（ARM64） | **不成立（2026-11-24 実測）** | install・devnode までは成立するが kernel が load を拒否（problem 52 = CM_PROB_UNSIGNED_DRIVER・CI event 3004）。VBS policy 稼働下では testsigning が効かず、解除は `hypervisorlaunchtype=Off`+reboot が要り job 内不可 |
| 独自 driver + Dev Portal 署名（attestation/WHQL） | **条件付き成立** | admin・署名 API 自動化（Hardware REST API/SDCM）・署名済み driver の reboot 不要 install が揃う。**ARM64 coverage が必要な場合の唯一経路** — 条件は後述の外部 gate |

条件（attestation 経路を採る場合に全て満たす — **x64 のみなら不要**）:

1. **EV code signing 証明書の調達**（有償・実在審査・ハードウェアトークン — repo 外の外部調達）。
2. **Hardware Dev Program アカウント**（Partner Center + Microsoft Entra ID アプリケーション + Manager ロール）。
3. **attestation submission の CI 自動化**（Hardware REST API または SDCM — 自動化実績あり）。
4. **Server SKU での install 実測確認**（attestation 署名は「Windows 10 Desktop and later」対象 — Server での virtio 系実績はあるが自前 driver での実測が要る → 実測 slice）。
5. ruviform 独自 driver の実装・保守（SysVAD 派生 PortCls miniport + INF + WDK build — 実装 slice）。test-signing 経路で先に進める場合も同一資産。

### MIDI — **独自 driver（x64 test-signing）が唯一の実測成立経路**

| 経路 | 判定 | 理由 |
| --- | --- | --- |
| WMS loopback on `windows-2022`/`windows-2025`（x64 Server） | **不成立** | WMS は Win11 desktop 25H2+ 限定。Server SKU は対象外で、`midisrv` 自体が存在しない |
| WMS loopback on `windows-11-arm`（arm64 Win11 25H2） | **不成立（2026-11-24 実測で確定）** | `midisrv`+Midi2 transport DLL は in-box で存在するが **provisioning 経路が全て到達不能**（rf#962 実測）: compiled `Windows.Devices.Midi2` API は activation factory 未登録（全 class `REGDB_E_CLASSNOTREG`）・`midi` console・PS module は image 非搭載。client component の job 内導入（Microsoft 配布物）が別途必要かは未検証の残件 |
| 独自 driver + test-signing on `windows-2025`（x64） | **成立（2026-11-24 実測）** | Audio 行と同一 — install→devnode→kernel load 完走を実測 |
| 独自 driver + Dev Portal 署名 | **条件付き成立** | Audio と同一の外部 gate（ARM64 lane が要る場合のみ） |

WMS 経路の残件（client component 導入可否 — 未実測）:

1. `Windows.Devices.Midi2` client API・`midi` console の配布形態（App SDK/tools package）が job 内 install 可能か。可能なら service-side `midisrv` は既に存在するため BLOOP provisioning が復活し得る。
2. **verifier 意味論との整合**（provisioning 経路が復活した場合の課題として継続）: LOOP A/B pair は `loopback.rs` の name-match candidate と実配線が食い違い `Fail` 化する危険 — BLOOP 限定か verifier 更新の選択。

## 方式選択とその根拠

### 採用案（2026-11-24 実測で確定 — rf#955/rf#961/rf#962 実走）

**Audio/MIDI 共通 — SysVAD 派生独自 kernel driver + 自己署名 test-signing on `windows-2025`（x64）**を第一候補とする。

根拠: rf#961 attempt-8 で test-signed kernel driver が GitHub-hosted x64 runner で install・devnode・kernel load・Running まで全経路を実測完走（evidence/rf961/）。image が `testsigning=Yes` 出荷のため EV 証明書・Partner Center は不要 — **外部調達ゼロで Audio/MIDI ともに x64 hosted runner へ乗る**。独自 driver は rf#629 capability matrix の ❌/⚠️ 故障注入を driver 層で実現できる副次的利点も不変。ARM64 lane が必要になった場合のみ attestation 署名が要る（その際は Slice D の外部 gate）。

**MIDI — WMS 経路は実測で到達不能と確定したため独自 driver に統合**。

根拠: rf#962 で `Windows.Devices.Midi2` client API が activation factory 未登録（両 leg `REGDB_E_CLASSNOTREG`）・console/PS module 非搭載を実測。service-side `midisrv` は存在するが supported な provisioning 経路が hosted runner に存在しない。client component の job 内導入可否は未検証の残件だが、独自 MIDI miniport driver（test-signing・x64）は既に全経路が実測で成立しており、そちらを主経路とする。

**外部調達の構図（更新）**: **x64 lane では外部調達ゼロ** — 自己署名証明書は job 内で `New-SelfSignedCertificate` 生成・`Root`+`TrustedPublisher` 登録で足りる（ユーザー判断済み: テスト用途のため自己署名で可）。残る外部調達候補は: (a) ARM64 lane が要る場合の EV+Partner Center attestation、(b) WMS 復活に必要な client component 配布物（未検証）、(c) self-hosted runner（rf#762 — GUI-L2 等の別目的では残余価値あり）。

### 不採用案

- **test-signing on `windows-11-arm`（ARM64）**: VBS policy 稼働下で kernel load が拒否されることを実測（rf#961 attempt-9・CI event 3004）。解除には reboot が要り job 内不可 — ARM64 lane は attestation が唯一経路。
- **user-mode audio（全経路）**: 方式自体が存在しない。
- **WMS on x64 Server image**: SKU 制約で service 自体が不在。
- **WMS provisioning via supported API on `windows-11-arm`**: compiled `Windows.Devices.Midi2` API は activation factory 未登録・console/PS module 非搭載（rf#962 実測）— client component 導入なしでは到達不能。
- **config JSON 直書き provisioning**: Microsoft が明示的に非サポート — 採用すると保守破綻の温床になる。
- **WMS LOOP A/B pair を既存 verifier にそのまま渡す**: name-match 意味論と実配線が食い違い `Fail` 化する危険 — BLOOP 選択か verifier 更新で回避。

## 既存コードとの接続点

既存 verifier/lane/probe がそのまま使える条件と要変更点（読取のみ — `crates/**` 変更是非は実装 slice の範囲見積もり）。

| 接続点 | 条件 | 要変更 |
| --- | --- | --- |
| `tests/virtual_endpoint.rs`（audio verifier） | endpoint friendly name が `VIRTUAL_NAME_HINTS` に部分一致し、方向語を剥がした stem が render↔capture で一致すること。独自 endpoint を `Ruviform Cable Input`/`Ruviform Cable Output` 系に命名すれば **hint 追加なしで検出可能**（`"cable input"`/`"cable output"` hint が既存）。任意名にする場合は hint 追加（`endpoint_probe.rs` と手動同期 — コメントで同期義務明記済み） | 命名整合なら **変更なし**。任意名なら hint 追加 |
| `tests/loopback.rs`（MIDI verifier） | midiIn/midiOut 両方向に**同名**で見える port。BLOOP・独自 driver port は該当。**LOOP A/B は送信が相手側に届くため Fail 化危険** | BLOOP/独自 driver なら **変更なし**。LOOP A/B 対応は pair 意味論更新が要る |
| `examples/endpoint_probe.rs`（audio probe） | 同上の name hint + `Win32_PnPSignedDriver`/署名観測・microphone privacy 照会は自前 driver をそのまま対象化できる | 命名整合なら **変更なし**（独自 driver 名を hint/署名正規表現に含める場合は小変更） |
| `examples/midi_endpoint_probe.rs`（MIDI probe） | `midisrv` + `New-MidiLoopback` の WMS 判定を既存実装済み — WMS 経路の provisioning 判定にそのまま使える | **変更なし**（BLOOP 判定の細分化は必要に応じて） |
| `tests/os_endpoint.rs`（wiring legs） | audio leg は `virtual_endpoint.rs` 同型の name-stem pairing、MIDI leg は `loopback.rs` 同型の bidirectional name match — 上記と同じ条件で動く | 同上 |
| `tests/os_endpoint_midi_e2e.rs`（VST3 e2e） | `loopback.rs` 同型の name match | 同上 |
| 4 manual lane（`mise.toml`） | provisioning 前工程（driver install・WMS endpoint 作成）が揃えば canonical 入口として不変 | **変更なし** — provisioning 手順自体は runbook 側に追加する構成が自然 |
| `environment_kind`/`verdict` 語彙 | 実走のみ `virtual-os-endpoint`/`ext-l3` を claim する fail-closed 契約は不変 | **変更なし** |

## 実装 slice 分割案

依存順に並べる。**Slice A（runner 実測）の結果が MIDI 採用方式の確定条件**であり、Slice B 以降は A の実測を踏まえて着手する。

### Slice A — runner 実測（probe workflow）

- **scope**: `.github/workflows/` に `workflow_dispatch` な実測 workflow を新設し、`windows-2025` + `windows-11-arm` で query 系を実走する。(a) OS build・`AudioSrv`/`midisrv`/WMS transport・`midi` console/PowerShell module の有無、(b) `midi` console 等による BLOOP/LOOP 作成試行と WinMM への反映、(c) `endpoint_probe`・`midi_endpoint_probe` の実走（aarch64 build 確認を兼ねる）、(d) Secure Boot 有効状態の確認、(e) AudioSrv 状態と endpoint 列挙実測。
- **範囲見積もり**: `.github/**` 新規 workflow + evidence 採取。`crates/**` 変更なし。probe は Windows 実機で既に動作確認済み（本 slice の補助実測どおり）。
- **確定条件**: WMS 経路が `windows-11-arm` で provisioning→WinMM 可視化まで成立するかを判定し、MIDI 採用方式（WMS vs 独自 driver）を確定する。

### Slice B — 独自 audio driver 実装（x64 test-signing 経路 — attestation 前提を解除）

- **scope**: SysVAD 派生 PortCls/waveRT miniport driver のソースツリー新設（仮想 speaker/mic pair、INF、WDK build 手順）、`Ruviform Cable *` 命名の endpoint 構成、`virtual-driver-install-probe.yml` で実測済みの自己署名→install→devnode 手順の流用。
- **範囲見積もり**: 新規 driver ソースツリー（`drivers/` に実績あり — `drivers/testsigned-null/` が雛形として使える）+ WDK build 手順 + `mise.toml`（build/provision lane 追加見込み）+ `docs/designs/`。
- **確定条件**: `windows-2025` runner で test-signed install → endpoint 列挙・`endpoint_probe` が pair candidate を検出・`tests/virtual_endpoint.rs` が round-trip する。
- **loopback 構造（2026-11-24 stage-2 実装 — rf#977 attempt-14 系の実測対象）**: upstream SysVAD には endpoint 間の PCM 経路が存在せず、capture stream は `ToneGenerator` の合成サインを DMA buffer へ書くだけだった（attempt-13 で `observed_rms=0.3535`・440Hz share 0.000 と実測 — `Ruviform Cable Input` への render データは破棄されていた）。そこで `drivers/rvf-virtual-audio/` に VB-CABLE 同型の kernel loopback を実装した:
  - `CAdapterCommon`（両 wave miniport に共有される adapter 単位オブジェクト）に `RVF_LOOPBACK_RENDER_SOURCE` registry を保持し、render stream が `AllocateAudioBuffer` で DMA buffer 参照・format を登録（`FreeAudioBuffer` 先頭で unmap 前に解除 — UAF 防止）、`SetState` で RUN 状態を、`UpdatePosition` で linear play cursor を publish する。
  - capture stream の `WriteBytes`（`UpdatePosition` 駆動・`m_PositionSpinLock` 下・≦DISPATCH_LEVEL）を `ToneGenerator.GenerateSine` から `RvfLoopbackCaptureWrite` へ置き換え、render buffer を整数演算のみで変換しながら capture DMA buffer へ書く（PCM16/24/32・f32・stereo→mono downmix・最近傍 rate map — kernel DPC では浮動小数点不可のため）。render 未登録・未 RUN・format 非互換では silence（virtual cable の自然な挙動 — `ToneGenerator.cpp/.h` 自体は vendored 資産として残置）。読み位置は **registry 内の連続読みカーソル（`ReadBytePos`/`ReadPosValid`）** — 初回は live play cursor に anchor し、以後は消費した source frame 数だけ前進する（fill 毎の re-anchor だと両 stream の独立 position クロックの jitter が fill 境界の dup/skip となり、marker 間隔 +10.4%・440Hz share 0.085 に劣化した — attempt-15 実測。drift が半 buffer 超の時のみ再 anchor）。
  - `SetState(KSSTATE_STOP)` で DMA 時刻基準と carry-forward をリセット（upstream は残渣を残す — restart で position 計算へ混入する経路を塞ぐ）し、`SetState(KSSTATE_RUN)` の時刻 stamp を位置 lock 下へ移動した。併せて attempt-13 の `output timestamp regressions: 2` 対応として render stream の全 position clock（Play/Write mod cursor・Linear・Presentation）と `m_llPacketCounter` を STOP 跨ぎ monotonic に維持したが **regressions は不変（attempt-16/17/18 でちょうど 2）** — driver が報告しうる全値を monotonic にしても動かないため、発生源は audiodg（user-mode）自身の position epoch bookkeeping（OS 供給の packet index restart — driver 不可制御）と確定した。input 側が regressions=0 なのは `GetReadPacket` の `PerformanceCounterValue`（driver-supplied timestamp）を使う経路で、output には該当 API が無いため。
  - 変更記録の正本は `drivers/rvf-virtual-audio/PROVENANCE.md`（`≠`/`new` 表記）。lock 順序は stream `m_PositionSpinLock` → adapter `m_LoopbackLock` の一方向のみで、逆方向を取る経路は無い。

### Slice C — MIDI provisioning（WMS 到達不能が実測で確定 — 独自 driver 経路に確定）

- **scope**: MIDI miniport driver 実装（Slice B の driver package への同梱が自然 — 同一署名・install 経路を再利用）+ `loopback.rs` が検出する in/out 同名 port の構成 + `windows-2025` x64 での lane 実走。
- **範囲見積もり**: driver 実装（`drivers/` 配下）+ `crates/**`（verifier 接続点の確認・必要なら調整）+ `mise.toml` lane 追加。
- **残件**: WMS client component の job 内導入可否（Microsoft 配布物）— 成立すれば arm/WMS 経路が復活し得る別途調査項目（優先度は独自 driver 経路が既に実測成立しているため低い）。

### Slice D — 署名 pipeline（ARM64 coverage が必要になった場合のみ）

- **scope**: EV 証明書 + Partner Center + Hardware API/SDCM による attestation submission の CI 自動化、署名済み driver package の取得・install 手順・revocation/rotation 方針。**x64 のみなら本 slice は不要**（自己署名で全経路が実測成立済み）。ARM64 は test-signing 不可が実測確定（VBS CI policy）のため、attestation が唯一経路。
- **範囲見積もり**: `.github/**`（署名 job・secrets 管理）+ scripts + `docs/`。**外部 gate: EV 証明書調達と Partner Center 開設は本 slice 着手の前提**（ユーザー判断要 — 現時点で着手根拠なし）。
- **確定条件**: `windows-11-arm` で attestation-signed driver が kernel load され devnode `problem_code=0` になる実走証拠。

### Slice E — lane/workflow 統合

- **scope**: provisioning → probe → lane 実走 → evidence 採取を束ねる opt-in manual workflow または self-hosted runner 向け runbook 更新、`docs/development-environment.md` lane 表更新。
- **範囲見積もり**: `.github/**`（opt-in workflow — POLICIES §5 の opt-in lane 規範に従う）+ `docs/` 更新。

## Risks and Trade-offs

| Risk | 影響 | 対応 |
| --- | --- | --- |
| `windows-11-arm` で midisrv/BLOOP が実測で不在 | MIDI の driver 不要経路が消え、独自 driver 経路に一本化 | **実測で確定** — midisrv は存在するが provisioning 経路が不在（rf#962）。独自 driver（x64 test-signing）に一本化済み |
| GitHub runner image の `testsigning=Yes` が将来の image 更新で締められる | x64 test-signing 経路が塞がる | image の現状観測値であって公約ではない — probe workflow を残して regress 検知できる状態にする。塞がれた場合は attestation（Slice D）か self-hosted へ |
| ARM64 lane が要求される | test-signing 不可が実測確定（VBS CI policy） | EV+Partner Center attestation（Slice D）が唯一経路 — 現時点では要求なし |
| attestation 署名が Server SKU で install できない | ARM64/将来の attestation 経路が閉じる | virtio 系実績はあるが未確認項目 — Slice D 着手時に実測 |
| LOOP A/B pair が verifier を Fail 化 | 誤判定で provisioning を疑い無駄な切り分け | BLOOP を provisioning 対象に固定するか verifier の pair 意味論を A/B 対応に更新（Slice C の範囲） |
| WDK/PortCls driver の保守コスト | WDK toolchain・INF・署名更新の継続保守 | MS-PL の SysVAD をベースに最小構成から始め、driver 層の制御（hotplug/loss/xrun）を capability matrix の ❌/⚠️ 項目回収に限定して段階追加 |
| `aarch64-pc-windows-msvc` で対象クレートが compile しない | arm/WMS 経路が使えない | Slice A で compile 確認。潰れたら x64 独自 driver 経路へ |

## Deferred Design Concerns

- **【解決済み 2026-11-24】runner 実測**: rf#955（環境 probe）・rf#961（install probe・attempt 1-9）・rf#962（WMS API probe）で実走完了。`testsigning=Yes`+SecureBoot OFF（両 leg）・x64 test-signed driver 全経路成立・ARM64 kernel load 不可（VBS CI）・WMS provisioning 到達不能を確定。
- **【解決済み】「self-hosted 不要ならそちら」への回答**: **x64 lane は self-hosted 不要・外部調達ゼロで成立**（ユーザー判断済み — テスト用途は自己署名で可）。
- **外部 gate: EV 証明書 + Hardware Dev Program**: ARM64 lane が要る場合のみ必要。x64 では不要（実測済み）。ARM64 lane 要求が発生した時点でユーザー判断へ。
- **rf#762 の扱い**: x64 GitHub-hosted が成立しても、self-hosted は GUI-L2 併用・debug 速度の点で残余価値がある。不要化は #762 owner 判断。
- **独自 driver の置き場所**: `drivers/` に実績あり（`drivers/testsigned-null/`）。製品 driver の配置は同じ形で十分。
- **verifier の LOOP A/B 意味論**: 独自 driver 経路では A/B pair 自体が存在しないため不発 — BLOOP/独自 port で `loopback.rs` 要件を満たす構成にすればよい。WMS 復活時のみ再燃。
- **BLOOP transport の in-box 状態**: 実測で確定 — transport DLL は in-box だが client provisioning 経路が不在（rf#962）。
- **capability matrix の ❌/⚠️ 項目**: 自前 driver で device-loss/xrun/permission-denial の故障注入が理論上回収可能 — 実装フェーズで対象 Case を選ぶ（runbook の実測待ち項目と合流）。

## Verification Notes

- [ ] `mise run fmt-check` — Pass。
- [ ] `mise exec -- prek run --all-files`（check-toml・markdownlint 等の衛生チェック）— Pass。
- [ ] `mise exec -- cargo run -p ruviform-audio-device --example endpoint_probe`（Windows 実機・query 系補助検証、canonical=mise ではない）— exit 0、`evidence/rf952/endpoint_probe-windows-local.txt` に記録。
- [ ] `mise exec -- cargo run -p ruviform-midi-device --example midi_endpoint_probe`（同上）— exit 0、`evidence/rf952/midi_endpoint_probe-windows-local.txt` に記録。
- [ ] サービス・権限 query（非昇格の確認・`AudioSrv`/`AudioEndpointBuilder`/`midisrv`/`New-MidiLoopback`）— `evidence/rf952/local-queries-windows-local.txt` に記録。
- [ ] provisioning 依存 lane（`test-audio-virtual-endpoint` 等）— **NotRun**（未 provisioning のため本 slice では実行しない — spec 指定どおり必須にしない）。
- [ ] Workspace `check`/`lint`/`test`/`build` — **NotApplicable**（文書のみの変更 — POLICIES §5）。
- [ ] runner 上の実測（midisrv/BLOOP・arm build・Server SKU install）— **NotRun**（`.github/**` 変更が要る probe workflow — Slice A に分割記録・本 slice では未実施）。
