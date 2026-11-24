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
- **署名規則**: Windows 10 1607 以降・Secure Boot ON の fresh install では Dev Portal（Hardware Dev Center）署名なしの新規 kernel driver は load されない。`bcdedit /set testsigning on` は再起動で初めて有効（Secure Boot 有効時は flag 設定自体も拒否）→ **test-signing → 再起動必須 → ephemeral runner で再起動不可** の連鎖が実際に成立する。
- **image**: `windows-2022` = Server 2022（10.0.20348）、`windows-2025`/`windows-latest` = Server 2025（10.0.26100）。**`windows-11-arm`/`windows-11-vs2026-arm` = Windows 11 Enterprise 25H2（10.0.26200, arm64）** — WMS の in-box 条件（Win11 desktop 25H2+）を満たす唯一の hosted image。Windows 両 image に WDK component（10.0.26100.16）同梱 — driver build・`devcon`/`signtool`/`infverif` は job 内で利用可能。
- **署名済み driver の install は reboot 不要**: runner-images maintainer が署名済み仮想 audio driver（Scream）の job 内 install を推奨した実績、VB-CABLE を job 内 install する公開 workflow 実績あり。
- **Server SKU**: AudioSrv は存在するが（Server 2025 では既定 Running）audio device 0 件で endpoint は 0 件 — 別プロジェクトの CI 検証 ADR が「hosted runners expose no physical or virtual audio endpoint」を実測として記録。WMS は Server には存在しない。

### Audio — **条件付き成立**（Dev Portal 署名経路のみ）

| 経路 | 判定 | 理由 |
| --- | --- | --- |
| user-mode 仮想 endpoint | **不成立** | 公式経路が存在しない（runner 制約以前に方式自体が不在） |
| 独自 driver + test-signing | **不成立** | testsigning → 再起動必須 → ephemeral runner で再起動不可 |
| **独自 driver + Dev Portal 署名（attestation/WHQL）** | **条件付き成立** | admin・WDK 同梱・署名 API 自動化（Hardware REST API/SDCM）・署名済み driver の reboot 不要 install が全て揃う。条件は後述の外部 gate |

条件（全て満たす必要あり）:

1. **EV code signing 証明書の調達**（有償・実在審査・ハードウェアトークン — repo 外の外部調達）。
2. **Hardware Dev Program アカウント**（Partner Center + Microsoft Entra ID アプリケーション + Manager ロール）。
3. **attestation submission の CI 自動化**（Hardware REST API または SDCM — 自動化実績あり）。
4. **Server SKU での install 実測確認**（attestation 署名は「Windows 10 Desktop and later」対象 — Server での virtio 系実績はあるが自前 driver での実測が要る → 実測 slice）。
5. ruviform 独自 driver の実装・保守（SysVAD 派生 PortCls miniport + INF + WDK build — 実装 slice）。

### MIDI — **条件付き成立**（2 経路）

| 経路 | 判定 | 理由 |
| --- | --- | --- |
| WMS loopback on `windows-2022`/`windows-2025`（x64 Server） | **不成立** | WMS は Win11 desktop 25H2+ 限定。Server SKU は対象外で、`midisrv` 自体が存在しない |
| **WMS loopback on `windows-11-arm`（arm64 Win11 25H2）** | **条件付き成立** | in-box service で **driver 不要** — 外部調達ゼロで GitHub-hosted に乗る可能性が最も高い経路。条件は後述 |
| 独自 driver + test-signing | **不成立** | Audio と同一の連鎖 |
| 独自 driver + Dev Portal 署名 | **条件付き成立** | Audio と同一の外部 gate（x64 lane を維持したい場合の経路） |

WMS 経路の条件（いずれも runner 実測待ち — 実測 slice で確定）:

1. `windows-11-arm` image で `midisrv` が有効か（phased rollout・既定状態）。
2. **BLOOP transport が in-box か**（2025-11 時点の releases 記録では preview transports 配布側に記載 — GA 後の同梱状況は image 実測要。preview transport なら tools package の job 内 install 経路を確立）。
3. **作成経路が headless で動くか**: `midi` console utility・PowerShell module（`New-MidiLoopback`/`New-MidiBasicLoopback`）・`Windows.Devices.Midi2` API のいずれか。config JSON 直書きは Microsoft が明示的に非サポートとしており採用不可。transient endpoint は service restart で消えるため ephemeral job 向き。
4. **verifier 意味論との整合**: LOOP A/B pair は各 endpoint が同名 in/out port を持つが送信は相手側に届くため、`loopback.rs` の name-match candidate が "A" を掴むと round-trip せず `verdict: Fail` 化する危険がある。**BLOOP（loopMIDI 同型の単一 endpoint）を provisioning 対象とするか、verifier の pair 意味論を A/B 対応に更新する**かの選択が要る（crates 変更 — 実装 slice 見積もり）。
5. **`aarch64-pc-windows-msvc` での workspace build** — 対象クレート・dev-dep 含む compile が arm64 で通るか。

## 方式選択とその根拠

### 採用案

**Audio — SysVAD 派生 PortCls/waveRT 独自 kernel driver + Dev Portal attestation 署名**を第一候補とする。

根拠: user-mode 経路の不存在を一次情報で確定した以上、仮想 audio endpoint は kernel driver が唯一経路であり、署名済み driver のみが reboot なしで ephemeral runner に install できる。attestation は HLK 不要で submission が REST API/SDCM で自動化可能 — 「build → 署名 submission → download → install → lane 実走」を同一 job 内で完結させうる設計が成立する。SysVAD（MS-PL）は派生・再配布が合法で、仮想 speaker/mic pair の実績ある骨格をそのまま使える。独自 driver は rf#629 capability matrix で COTS が ❌/⚠️ だった device-loss・xrun・permission denial の故障注入を driver 層で実現できる — 外部 driver では再現不能だった Case を回収する副次的利点がある。

**MIDI — Windows MIDI Services loopback on `windows-11-arm` を第一候補とする**。

根拠: `windows-11-arm` は Win11 25H2 client で WMS in-box — **driver 実装すら不要**で、「ruviform 資産」として必要なのは provisioning 自動化（endpoint 作成手順）と verifier 接続点だけになる。transient loopback は job 内で作成・消去が完結し cleanup も軽い。x64 lane を維持したい場合の代替として独自 MIDI kernel driver（attestation 署名）を併記する — Audio driver を実装するなら同 package に MIDI miniport を同梱する統合案も取れる。

**外部調達の構図**: Audio の GitHub-hosted 成立には EV 証明書 + Partner Center という外部調達が不可避（rf#762 の runner 調達を証明書調達に置き換える形）。**外部調達を一切避ける路線では**: Audio は self-hosted runner（rf#762 継続）+ test-signing（永続化可能 — 専用機なら reboot 後の状態が残る）が現実解、MIDI は `windows-11-arm` + WMS で外部調達ゼロの可能性が高い（実測待ち）。この2つの分岐を Deferred Design Concerns のユーザー判断論点として記録する。

### 不採用案

- **test-signing + GitHub-hosted**: 再起動不可制約に確定的に衝突 — 実現不能として棄却。
- **user-mode audio（全経路）**: 方式自体が存在しない。
- **WMS on x64 Server image**: SKU 制約で service 自体が不在。
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

### Slice B — 独自 audio driver 実装（attestation 経路前提）

- **scope**: SysVAD 派生 PortCls/waveRT miniport driver のソースツリー新設（仮想 speaker/mic pair、INF、WDK build 手順）、test-signing での専用機/ローカル動作確認、`Ruviform Cable *` 命名の endpoint 構成。
- **範囲見積もり**: 新規 driver ソースツリー（`crates/**` 外 — `drivers/` 等の配置は実装 slice で判断）+ WDK build 手順 + `mise.toml`（build/provision lane 追加見込み）+ `docs/designs/`。
- **確定条件**: test-signing 環境で endpoint 列挙・`endpoint_probe` が pair candidate を検出・`tests/virtual_endpoint.rs` が round-trip する。

### Slice C — MIDI provisioning（Slice A の結果で分岐）

- **WMS 経路採用時**: provisioning script（`midi` console 経由または `Windows.Devices.Midi2` API 薄 wrapper）+ BLOOP 前提の pair 意味論確認（必要なら verifier の candidate フィルタ調整 — `crates/ruviform-midi-device`/`ruviform-audio-test`/`ruviform-plugin-test-vst3`）+ `windows-11-arm` での lane 実走。
- **独自 driver 経路時**: MIDI miniport 実装（Slice B の driver package への同梱も選択肢）+ 同じく verifier 接続点確認。
- **範囲見積もり**: provisioning 資産（scripts/examples 等）+ `crates/**`（verifier 意味論調整が要る場合）+ `mise.toml` lane 追加。

### Slice D — 署名 pipeline（attestation 経路採用時）

- **scope**: EV 証明書 + Partner Center + Hardware API/SDCM による attestation submission の CI 自動化、署名済み driver package の取得・install 手順・revocation/rotation 方針。
- **範囲見積もり**: `.github/**`（署名 job・secrets 管理）+ scripts + `docs/`。**外部 gate: EV 証明書調達と Partner Center 開設は本 slice 着手の前提**（ユーザー判断要）。
- **確定条件**: Server SKU で attestation-signed driver が pnputil/devcon で install でき、`test-audio-virtual-endpoint` が `verdict: Pass` を返す実走証拠。

### Slice E — lane/workflow 統合

- **scope**: provisioning → probe → lane 実走 → evidence 採取を束ねる opt-in manual workflow または self-hosted runner 向け runbook 更新、`docs/development-environment.md` lane 表更新。
- **範囲見積もり**: `.github/**`（opt-in workflow — POLICIES §5 の opt-in lane 規範に従う）+ `docs/` 更新。

## Risks and Trade-offs

| Risk | 影響 | 対応 |
| --- | --- | --- |
| `windows-11-arm` で midisrv/BLOOP が実測で不在 | MIDI の driver 不要経路が消え、独自 driver 経路に一本化 | Slice A の実測を方式確定の前置条件とし、潰れたら独自 driver + attestation（Audio と同一 pipeline を再利用）へ切替 |
| EV 証明書・Partner Center の調達が通らない | Audio の GitHub-hosted 経路が閉じる | 外部調達ゼロ路線へ — self-hosted runner（rf#762 継続）+ test-signing。MIDI は arm/WMS が残り得る |
| attestation 署名が Server SKU で install できない | Audio GitHub-hosted 不成立に確定 | virtio 系実績はあるが未確認項目として Slice A/D で実測。潰れたら self-hosted へ |
| LOOP A/B pair が verifier を Fail 化 | 誤判定で provisioning を疑い無駄な切り分け | BLOOP を provisioning 対象に固定するか verifier の pair 意味論を A/B 対応に更新（Slice C の範囲） |
| WDK/PortCls driver の保守コスト | WDK toolchain・INF・署名更新の継続保守 | MS-PL の SysVAD をベースに最小構成から始め、driver 層の制御（hotplug/loss/xrun）を capability matrix の ❌/⚠️ 項目回収に限定して段階追加 |
| `aarch64-pc-windows-msvc` で対象クレートが compile しない | arm/WMS 経路が使えない | Slice A で compile 確認。潰れたら x64 独自 driver 経路へ |

## Deferred Design Concerns

- **runner 実測の未実施**: 本 slice は `.github/**` 変更を含まないため runner 上の実測は行っていない。MIDI の WMS 経路・`windows-11-arm` の arm build・midisrv/BLOOP 同梱・Secure Boot 状態は全て Slice A（probe workflow）の実測待ちであり、判定が「条件付き成立」から動く可能性がある。
- **外部 gate: EV 証明書 + Hardware Dev Program**: Audio の GitHub-hosted 成立はこの調達が前提。調達可否・費用・主体（個人/組織での開設条件）は repo 外判断 — **`ask`/ユーザー判断要**。
- **「self-hosted 不要ならそちら」への回答の分岐**: MIDI は arm/WMS で self-hosted 不要の可能性が高いが、Audio は GitHub-hosted でも証明書調達が要る。「runner 調達（rf#762）を証明書調達に置き換える」か「外部調達ゼロで self-hosted + test-signing」かはユーザー判断 — 本 doc では両経路を併記し確定を保留する。
- **rf#762 の扱い**: 独自+attestation で GitHub-hosted が成立しても、test-signing が使える self-hosted は debug 速度・証明書不要・GUI-L2 併用の点で残余価値がある。不要化は #762 owner 判断。
- **独自 driver の置き場所**: `crates/**` は Rust workspace の領域。driver ソース（C/C++/INF/WDK project）の配置は実装 slice で repo 構成判断 — `drivers/` 新設・別 workspace・別 repo の選択肢。
- **verifier の LOOP A/B 意味論**: `loopback.rs`/`os_endpoint.rs`/`os_endpoint_midi_e2e.rs` の name-match candidate が WMS A/B pair で Fail 化する危険。BLOOP 限定 vs A/B 対応更新の設計は実装 slice で決定。
- **BLOOP transport の in-box 状態**: 2025-11 時点では preview transports 配布側に記録 — GA 後（2026-11 現在）の in-box 同梱状況は image 実測が要る。
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
