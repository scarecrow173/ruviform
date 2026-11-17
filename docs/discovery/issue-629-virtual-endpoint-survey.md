---
id: "DISC-034XC9utDSmNZnzNq2DOds"
type: "discovery"
status: "draft"
title: "Issue #629 Windows virtual audio/MIDI endpoint survey and capability probe"
created: "2026-11-14"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/251"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/synvelis/issues/115"
    - "https://github.com/scarecrow173/synvelis/issues/117"
    - "https://github.com/scarecrow173/synvelis/issues/126"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-629-virtual-endpoint-survey.md"
      - type: "doc"
        path: "evidence/rf629/endpoint_probe-windows-local.txt"
      - type: "doc"
        path: "evidence/rf629/midi_endpoint_probe-windows-local.txt"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "POLICIES.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
    - "docs/test-specs/issue-627-virtual-first-verification-contract.md"
    - "docs/specs/issue-629-midi-loopback.md"
    - "docs/designs/issue-629-midi-loopback.md"
    - "docs/test-specs/issue-629-midi-loopback.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://shop.vb-audio.com/en/win-apps/11-vb-cable.html"
    - "https://vb-audio.com/Cable/VBCABLE_ReferenceManual.pdf"
    - "https://vac.muzychenko.net/en/purchase.htm"
    - "https://vac.muzychenko.net/en/manual/install.htm"
    - "https://www.tobias-erichsen.de/software/loopmidi.html"
    - "https://www.tobias-erichsen.de/software/virtualmidi.html"
    - "https://microsoft.github.io/MIDI/"
    - "https://microsoft.github.io/MIDI/tools/midiloopbacksetup/"
    - "https://microsoft.github.io/MIDI/tools/powershell/"
    - "https://learn.microsoft.com/en-us/samples/microsoft/windows-driver-samples/sysvad-virtual-audio-device-driver-sample/"
  defers: []
  deferred-by: []
---

# Issue #629 Windows virtual audio/MIDI endpoint survey and capability probe

## Summary

Issue #629 の first slice として、Windows 仮想 Audio/MIDI Endpoint の候補調査と capability probe PoC を行った。結論（暫定）: Audio 側は署名済み COTS の VB-CABLE 系が第一候補、MIDI 側は loopMIDI（teVirtualMIDI）と Windows MIDI Services の in-box loopback が候補であり、いずれも「名前・インストール方法を決め打ちしない」Issue 方針に従い Capability Probe が provisioning 判定を担う。runtime-driven controller・本番 Backend 配線・CI lane は後続 slice であり、本 slice は列挙・format 問い合わせ・fail-closed 判定までを範囲とする。

## Exploration Goal

Issue [#629](https://github.com/scarecrow173/ruviform/issues/629) は「Windows 実 OS 仮想 Audio/MIDI Endpoint・Provisioning・本番 API 通過 Gate」を目的とし、Signal/MIDI Generator → 仮想 Endpoint → 本番 WASAPI/WinMM Backend → Ruviform Device/Clock Contract → Synvelis Consumer の経路を、実 OS API を通る形で検証可能にすることを求める。

本探索が答える問い:

- 署名済み・合法に配布可能な仮想 Audio Endpoint 候補（VB-CABLE / VB-CABLE Hi-Fi / VAC / その他）は何か。署名状態・ライセンス・インストール方法・管理者権限・制御可能性（hotplug / format switch / device-loss）はどうか。
- WinMM から列挙可能な仮想 MIDI loopback 候補（loopMIDI / loopBE / Windows MIDI Services in-box loopback）は何か。OS 版・導入状態への依存はどうか。
- それらで不足する場合の SysVAD 系 fallback のスコーピングと採用判定基準は何か。
- Provisioning 判定 + runbook、Capability matrix（再現可能 / NotRun の境界）はどう定義するか。
- 既存コードのギャップ（MIDI input-only、`MidiPortOrigin::Unknown`、windows feature 不足）は何か。

本書は調査・候補比較・probe PoC の記録であり、endpoint の採用決定・driver 実装・runtime controller の設計確定は行わない（後続 spec/design/ADR の責務）。

## Key Issues and Assumptions

### 上流で確定済み（confirmed — 本 slice で再決定しない）

- **`environment_kind` 語彙と fail-closed 規則**: `scarecrow173/ruviform#627` contract（`docs/specs/issue-627-virtual-first-verification-contract.md` / `docs/designs/issue-627-virtual-first-verification-contract-design.md`）で確定。仮想環境の `NotRun`/`Unsupported` を Pass にしない。実 OS API を通っていない Mock を `os_virtual_endpoint`（`environment_kind=virtual-os-endpoint`）と称さない。本 slice の probe 出力は `virtual-os-endpoint` の evidence を**主張しない**（列挙・format 問い合わせは streaming 証明ではない）。
- **本番 API 契約の不変性**: `ruviform_audio_device`（cpal WASAPI backend + `wasapi_input.rs` ネイティブ入力経路）と `ruviform_midi_device`（WinMM `midiIn*`）の公開境界 trait を変更せず、仮想 endpoint を既存入口から駆動する。OS 固有型は adapter crate 内に閉じる。
- **RT 契約**: callback は allocation-free・bounded。probe は control-path の手動 example であり callback を持たない（後続 slice の runtime controller が担う）。
- **POLICIES.md（2026-11-15 仮想優先）**: Windows OS-L1 必須、GUI-L2/EXT-L3 は利用機能のみ。仮想 OS endpoint は標準環境。物理機材・市販 Plugin は初回必須でない。日次 CI に kernel driver 導入を強制しない — opt-in Windows lane は許容。
- **本 slice の編集範囲**: 新規 example 2 本 + docs + `evidence/rf629/` のみ。`Cargo.toml`/`Cargo.lock`/`mise.toml`/workflow/`src/` 変更なし。driver の無断導入禁止。

### コード観測事実（confirmed — probe 実装と実行で検証済み）

- **Audio 列挙経路**: `ruviform_audio_device` は cpal `Host`（`available_host_ids`/`host`/`default_host`）を公開し、Windows では `wasapi_input.rs` が `IMMDeviceEnumerator`/`IMMDevice`/`IAudioClient`/`IAudioCaptureClient` を直接使用（`windows = "0.62"`、`Win32_System_Com`・`Win32_Media_Audio` 等 enabled）。probe はこの enabled feature のみで `EnumAudioEndpoints`（`DEVICE_STATEMASK_ALL`）・`GetId`・`GetState`・`Activate<IAudioClient>`・`GetMixFormat`・`IsFormatSupported`・`GetDevicePeriod`・`GetStreamLatency` を実行できた — **新規 feature・新規依存は不要**。
- **`IsFormatSupported` の実測 quirk**: `ppclosestmatch=NULL` で呼ぶと本ホスト（Windows 11 build 26300.9974）では全 format が `0x80004003`（E_POINTER）を返した。`ppclosestmatch` に有効な out-param を渡すと正常に `S_OK`/`AUDCLNT_E_UNSUPPORTED_FORMAT` + closest format を返す。probe は closest-match を表示する形で対処済み — **後続 slice の format 交渉実装でも同じ引数規約が必要**。
- **`GetStreamLatency` は `Initialize` 前に `AUDCLNT_E_NOT_INITIALIZED`（0x88890001）を返す** — probe は Initialize を呼ばない（query-only、audio session を作らない）ため latency は常に NotRun 相当。open/start/stream は次 slice。
- **not-present endpoint の Activate は `0x88890004`（`E_NOTFOUND` 系）で拒否** — state=not-present の stale endpoint（本ホストで 61 endpoint 中大半）は列挙できても activate できないことを実測。hotplug 観測では `GetState` の遷移を見る必要がある。
- **MIDI 列挙経路**: `ruviform_midi_device` は WinMM `midiInGetNumDevs`/`midiInGetDevCapsW`/`midiInMessage`（`DRV_QUERYDEVICEINTERFACE*`）を使用（`windows-sys = "0.61"`）。~~**input-only で `midiOut*` 送信経路は存在しない**~~ → **2nd slice `scarecrow173/ruviform#658`（merge `7e7270f`）で解消**: `midi_output` port + WinMM `midiOut*` backend（open/send/send_sysex/close）が main に存在する。残る gap は provisioned loopback pair での round-trip 実証のみ（3rd slice `tests/loopback.rs` verifier、provisioning 確立までは `NotRun`）。
- **`midiOutMessage` の `DRV_QUERYDEVICEINTERFACE*` は信用できない**: 本ホストの `Microsoft GS Wavetable Synth`（唯一の midiOut port）で size query が `MMSYSERR_NOERROR`+size=2 を返し、follow-up fetch が winmm 内で **segfault** した。probe は input-path のみ query し、output 側は `NotRun` として記録する形に修正済み — **interface-id ベースの仮想 port 識別は input 方向に限定される**。
- **`MidiPortOrigin::Unknown`**: WinMM は仮想/物理の区別を公開しないため adapter は `Unknown` を報告（`midi_input.rs` の契約どおり正直）。仮想 port の識別には name/interface-id heuristic か MIDI Services metadata が必要。
- **`MIDIINCAPSW` の windows-sys 定義は `szPname` で終端**（`wTechnology`/`wVoices`/`wNotes`/`wChannelMask`/`dwSupport` を含まない）— input caps の `dwSupport` 相当は取得不可。`MIDIOUTCAPSW` は full layout。
- **本ホスト実測（Windows 11 10.0.26300.9974, x86_64）**: MMDevice 61 endpoint（active render 7 / active capture 2 / 残り not-present）、仮想候補 name heuristic 0 件、`provisioning: NOT_PROVISIONED`。WinMM input 0 port / output 1 port（GS Wavetable Synth）、loopback 候補 0 件、`NOT_PROVISIONED`。**未 provisioning 環境が fail-closed に `NotRun` を報告することを実機で確認**。

### プローブ実装の事実

- `crates/ruviform-audio-device/examples/endpoint_probe.rs`: `#[cfg(windows)]` 本体 + 非 Windows stub（`NotRun` 出力）。手動実行・CI 非実行。`unsafe` は module 内 `#![expect(unsafe_code)]` + SAFETY コメント（`wasapi_input.rs` 先例）。
- `crates/ruviform-midi-device/examples/midi_endpoint_probe.rs`: 同構成。loopback 判定は「同名 port が midiIn/midiOut 双方に見える」bidirectional name match（loopMIDI/loopBE/WMS loopback の共通 signature）+ name heuristic ではなく caps/ interface-id の実データを報告。
- 両 probe とも `verdict` 節で provisioning 状態と各 capability の `observed`/`NotRun` を明示し、`virtual-os-endpoint` evidence を主張しない。

## Alternatives and Comparison

### Audio 仮想 Endpoint 候補

| 候補 | 署名 | ライセンス/配布 | Install | WASAPI 可視性 | 制御可能性 | 評価 |
| --- | --- | --- | --- | --- | --- | --- |
| **VB-CABLE**（標準） | 署名済み kernel driver（VB-Audio） | Donationware（個人無償・商用は $5/ユーザ） | zip 展開 → setup を admin 実行 → **要再起動** | "CABLE Input"（render）/"CABLE Output"（capture）の endpoint pair として WASAPI/MME/KS/DirectX に出る | 内部 SR 48kHz 固定・SRC あり。format は endpoint 側で変更可（44.1k–192k/16–24bit）。endpoint 自体は静的（hotplug なし） | 第一候補。実績・配布物が安定。再起動が provisioning runbook に必須 |
| **VB-CABLE Hi-Fi** | 同上（同 driver 系） | 同上 | 同上（**再起動要**） | 同上 pair | SRC なし bit-perfect、最大 24bit/384kHz、in/out 同一 sample rate 必須 | format 厳密性が要る Case（f64/bit-exact 近傍）の補助候補。標準 CABLE と排他 install ではないが併用時は endpoint 名で区別 |
| **VAC（Virtual Audio Cable）** | 署名済み（reseller cert: NTONYX 名義） | 商用 $30 home / $50 business（perpetual） | installer（admin）・**通常再起動不要** | "Line N (Virtual Audio Cable)" endpoint として WASAPI に出る。最大 256 cable | **最も制御面が広い**: cable 数・format 上限（384kHz/32bit/32ch）・multi-client・stream format limiting・channel remap・**position/clock register support**。control panel API 的操作 | clock-domain/format-switch 系 Case の第一候補になり得るが商用 license が要る。trial は 30 分音声 reminder で CI 不適、Lite は 1 cable/48k/16bit/1 client で不足 |
| **VoiceMeeter 系** | 署名済み | Donationware | admin install | 複数 virtual input/output | mixer 的で endpoint 意味論が複雑 | 対象外（単純 loopback 目的に過剰） |
| **SysVAD 系 fallback** | test-signing（自己署名）or attestation | Microsoft sample（WDK） | WDK build → test-signing ON → devcon install → **専用機前提** | 自前 INF で任意 endpoint 構成可 | driver 改変で hotplug/device-loss/format を自在に制御可能 — **唯一「故障注入を実 OS driver 層で再現」できる選択肢** | COTS で不足する capability（device-loss・xrun 強制・permission denial）が出た場合の最終手段。WDK/署名/専用 runner の維持コスト大 |

### MIDI 仮想 Endpoint 候補

| 候補 | 署名 | ライセンス/配布 | Install | WinMM 可視性 | 動的性 | 評価 |
| --- | --- | --- | --- | --- | --- | --- |
| **loopMIDI**（teVirtualMIDI） | 署名済み kernel driver（Tobias Erichsen） | 無償・**私的非商用利用のみ**・再配布は書面同意要 | installer（admin）・app 起動中のみ port 存在 | port は midiIn/midiOut **双方**に同名で出る（bidirectional pair） | **port は app 実行中のみ存在・ユーザ単位・動的作成/破棄** — hotplug Case に最適 | 第一候補。非商用限定ライセンスと「app 常駐が前提」の運用を runbook に明記。Windows 11 での動作は capability probe で要確認 |
| **loopBE Audio** | 署名済み | freemium | installer | 同上（bidirectional pair） | app 依存 | loopMIDI と同等。代替候補 |
| **Windows MIDI Services loopback** | in-box（OS component） | OS 付属・追加 install なし | Win11 25H2+ で提供。`New-MidiLoopback`/`New-MidiBasicLoopback`（PowerShell）または SDK。transient or `-SaveToConfiguration` | **WinMM/WinRT API は新 service へ repoint**（`wdmaud2.drv` 互換層経由で legacy app に見える）— MIDI 1.0 Basic Loopback は単一 endpoint、MIDI 2.0 Loopback は A/B pair | service 管理下で動的作成可能。transient は service restart で消える | OS-native で provisioning が軽いが、**利用可否が OS build（25H2+）と feature rollout に依存** — 採用は capability probe 判定。本ホスト build 26300 では loopback 未作成（probe 実測 in=0）|
| **rtpMIDI / 仮想 MIDI SDK** | 署名済み | SDK は商用 | driver + SDK | WinMM pair | SDK で完全制御 | loopMIDI で不足する制御（独自 port 名・metadata）が要る場合の候補。現時点では過剰 |

### SysVAD 系 fallback の採用判定基準（記録のみ — 本 slice では実装しない）

- WDK build 環境・test-signing（`bcdedit /set TESTSIGNING ON` + cert install + reboot）・Secure Boot/BitLocker との整合・専用 Windows runner が前提。
- 採用トリガー: COTS 候補で「再現可能」と判定すべき capability（例: device-loss の強制、xrun の決定的注入、permission denial の再現、format switch の境界）が `NotRun`/`Unsupported` に留まる場合。
- SysVAD は「音声 render/capture endpoint を driver として自作」するため hotplug・format・loss を**実 driver 層で**制御できる唯一の選択肢だが、署名・配布・保守のコストが桁違い。Issue 方針「非互換なら SysVAD 系 PoC/専用機に限定」に従い、COTS 採用を既定・SysVAD を例外経路とする。

## Capability Matrix（暫定 — probe 実測 + 公開情報ベース。runner provisioning 後に確定）

凡例: ✅=再現可能見込み / ⚠️=条件付き / ❓=未確認（probe 要）/ ❌=不可または非現実 / `NotRun`=本 slice 範囲外

| Capability | VB-CABLE/Hi-Fi | VAC | loopMIDI/WMS loopback | SysVAD |
| --- | --- | --- | --- | --- |
| endpoint 列挙（全 state） | ✅ | ✅ | ✅（WinMM caps） | ✅ |
| endpoint identity/state 取得 | ✅（`GetId`/`GetState`） | ✅ | ✅（caps/interface-id） | ✅ |
| format 交渉（shared mode） | ✅（endpoint format 変更 + IsFormatSupported） | ✅（format 上限広い） | n/a（MIDI に format 概念なし — UMP/byte stream） | ✅（INF/driver で制御可） |
| capture/render open・stream | ✅（要 provisioning） | ✅ | n/a | ✅ |
| callback/stream viability | ✅ 見込み | ✅ 見込み | ✅（`midiInStart` callback） | ✅ |
| **virtual hotplug**（endpoint 出現/消失） | ❌（静的 driver endpoint — install/uninstall のみ） | ❌（同上） | ✅ **loopMIDI は app 起動/終了で port 出現/消失** — 最良の hotplug 再現源。WMS transient loopback も同等 | ✅（devcon/PnP で再現可） |
| **format switch**（streaming 中の format 変更） | ⚠️（endpoint format は OS 側設定変更で可・streaming 中の動的切替は driver 依存）❓ | ✅（format limiting/remap 制御あり） | n/a | ✅ |
| **device-loss / invalidation** | ⚠️（無効化・取り外しは PnP 経由 — 再現は限定的）❓ | ⚠️ 同左 | ⚠️（loopMIDI app kill で port 消失 → `MM_MIM_CLOSE`/device vanished を再現可能か要 probe） | ✅（driver が失敗を注入可能） |
| **permission denial** | ⚠️（Windows の microphone privacy 設定で capture を拒否可能 — OS 標準機能で再現可） | 同左 | n/a | ✅ |
| **xrun / latency** | ⚠️（VB-CABLE は内部バッファ 7168 frame・latency 調整可。決定的 xrun 注入は困難） | ⚠️（clock register で位置制御は可・xrun 強制は ❓） | n/a | ✅（driver で drop 注入可） |
| **loopback latency**（signal→capture の往復遅延） | ✅（internal SR 48kHz・latency パラメータあり・実測可能） | ✅ | ✅（timestamp は WinMM ms domain — adapter が `MidiPortCapabilities.clock_domain` で報告） | ✅ |
| **MIDI Note/CC/PitchBend/Timestamp/Order** | n/a | n/a | ✅（loopback pair 経由 — send path は 2nd slice `scarecrow173/ruviform#658` で実装済み。round-trip verifier は 3rd slice `tests/loopback.rs`、provisioned pair 前提で未 provision 環境は `NotRun`） | n/a |

## Known Code Gaps（後続 slice への引き継ぎ事項）

1. ~~**`ruviform-midi-device` は INPUT-only**~~ → **resolved（2nd slice `scarecrow173/ruviform#658`、merge `7e7270f`）**: `midi_output` port trait + WinMM `midiOut*` backend（`midiOutGetDevCapsW`/`midiOutOpen`/`midiOutShortMsg`/`midiOutLongMsg`+`MM_MOM_DONE` 待機/close）が main に存在し、`send`/`send_sysex`/`close` と `StreamInvalidated` 契約を `tests/output_ports.rs` が固定済み。残る gap は provisioned loopback pair での send→receive 実証で、3rd slice の `tests/loopback.rs` round-trip verifier が引き継ぐ（pair 未 provision 環境では fail-closed `NotRun`）。
2. **`MidiPortOrigin::Unknown`** — WinMM は仮想/物理を区別しない。仮想 port の識別は (a) 同名 bidirectional pair heuristic（loopMIDI/WMS loopback）、(b) `DRV_QUERYDEVICEINTERFACE` の interface-id 接頭辞、(c) Windows MIDI Services metadata（`MidiEndpointDeviceInformation` — 別 API 面・windows feature 追加が要る）のいずれか。識別結果を `Virtual` に格上げするかは adapter 契約の変更を伴うため後続で判断。
3. **`midiOutMessage` `DRV_QUERYDEVICEINTERFACE*` の不安定性** — input 側は実績あるが output 側は本ホストで crash を観測。interface-id ベース識別は input 方向のみ安全。
4. **windows feature 不足（probe gap として記録）**: SetupAPI/driver-store 系（driver 署名・package 検証）と `Win32_System_SystemInformation`（`GetVersionEx`/`RtlGetVersion` による正確な OS build 取得）は enabled feature にない。probe は `cmd /c ver` で build 文字列を取得する workaround を取った。driver signature の機械的検証は provisioning runner 側の別手段（例: PowerShell `Get-AuthenticodeSignature`）が要検討。→ **4th slice で部分解消**: `endpoint_probe` が PowerShell 経由で `Win32_PnPSignedDriver`（provider/version/date/inf）+ staged catalog への `Get-AuthenticodeSignature` を best-effort 観測するようになった（失敗は `NotRun` note、exit 0 維持）。SetupAPI 面の機械的検証と microphone privacy キーの体系的収集は provisioning runner 側の残件。
5. **runtime-driven controller は本 slice 対象外** — 仮想 endpoint の provisioning 判定・streaming 駆動・Expected/Observed 照合・証拠記録は #628 Test Runtime + 後続 slice の責務。

## Tentative Conclusions（仮説 — spec/design/ADR で確定）

- **Audio 第一候補は VB-CABLE 系**: 署名済み・実績・WASAPI 標準 endpoint として安定。再起動を伴う install は専用 runner の provisioning runbook で吸収する。Hi-Fi は format 厳密性 Case の補助。
- **VAC は「制御面が要る Case」の候補**: clock register・format limiting・multi-client は他にない制御面だが商用 license が前提。VB-CABLE で不足が判明した場合に検討。
- **MIDI 第一候補は loopMIDI（teVirtualMIDI）**: bidirectional pair で hotplug（app 起動/終了による port 出現/消失）まで再現でき、WinMM からそのまま見える。**非商用ライセンス制約**と「app 常駐」運用を runbook に明記する必要あり。
- **Windows MIDI Services loopback は OS-native 代替**: provisioning が軽い（PowerShell cmdlet）が、OS build・feature rollout 依存のため capability probe で採用判定。loopMIDI と排他ではなく併用可。
- **SysVAD は例外経路**: COTS で再現不能な capability（device-loss 強制・xrun 注入・permission denial の完全制御）が必要になった場合のみ、専用機 + test-signing で scoped PoC。
- **本 slice の probe は「provisioning 判定器」として十分**: 未 provisioning → `NotRun` fail-closed、provisioning 済み → endpoint 列挙・state・format 交渉・period/latency query の実測値を evidence として出力する設計が実機で確認できた。

## Open Questions

1. VB-CABLE endpoint の hotplug 相当（disable/enable・既定 device 切替）で `IMMNotificationClient` が発火するか — `wasapi_input.rs` の notification 経路は既に実装済みだが、仮想 endpoint での発火は provisioning 後に要実測。
2. loopMIDI port の出現/消失が WinMM `midiInGetNumDevs` に動的反映されるか（app 起動/終了で index が変わるか）— hotplug Case の成立条件。
3. Windows MIDI Services が対象 runner の OS build で利用可能か（25H2+ かつ feature rollout 状態）。
4. VB-CABLE/loopMIDI の license が「専用 CI/test runner へのインストールと自動 provisioning」を許容するか — donationware/非商用条項の解釈を runner 運用前に確認。
5. `IsFormatSupported` の `ppclosestmatch` 必須挙動は Windows build 依存か恒常か — 他 build での再現性。
6. capture 側の permission denial（Windows 設定の microphone privacy）を仮想 endpoint で再現できるか — 実 OS 標準機能のため恐らく可だが未実測。
7. `midiOutMessage` crash が GS Wavetable Synth 固有か winmm 全般か — 他 driver での再現性。

## Promotion Candidates

- `docs/specs/issue-629-virtual-endpoint-survey.md` — 本 slice の要件（probe 範囲・fail-closed 規則・provisioning 境界・後続 hand-off）。
- `docs/designs/issue-629-virtual-endpoint-survey.md` — probe アーキテクチャ（control-path example・OS API 面・private adapter 境界の記録方針）。
- `docs/plans/` + `docs/tasks/` — slice の作業分解。
- `docs/test-specs/issue-629-virtual-endpoint-survey.md` — probe 出力の Case 化（`NotRun`/`observed` の判定規則）。
- 将来 ADR 候補: 採用 endpoint の確定（VB-CABLE vs VAC vs SysVAD）、MIDI loopback の採用（loopMIDI vs WMS）、`midiOut*` 送信経路の adapter 追加方針、`MidiPortOrigin` 識別方式 — いずれも provisioning 実測後。

## Sources and Relations

- **Issue #629**: first slice = investigation + capability-probe。runtime-driven controller・本番 Backend 配線・CI lane・driver 実装は後続。
- **Issue #627 contract**: `environment_kind`・Case schema・fail-closed 規則の正本（`docs/specs/`・`docs/designs/` issue-627）。
- **POLICIES.md**: 2026-11-15 仮想優先改訂 — 仮想 OS endpoint は標準環境、driver 導入は opt-in runner、物理/市販 Plugin は初回必須でない。
- **関連 Issue**: #628（Test Runtime — 後続で probe を駆動）、#251（実 WASAPI backend — probe が検証対象）、#253（実 WinMM backend）、#294（consumer 証拠集約）、Synvelis #115/#117/#126・AudioLens #127（consumer 影響照合 — 本 slice では範囲記録のみ）。
- **外部一次情報**: VB-Audio 公式（shop/download/reference manual）、Muzychenko VAC 公式（purchase/manual）、Tobias Erichsen 公式（loopMIDI/virtualMIDI）、Microsoft Learn（SysVAD sample）、Microsoft MIDI（Windows MIDI Services docs）。Evidence の `## Evidence` 節に URL と実測出力を記録。

## Evidence

<!-- Append-only: dated entries, never edit/reorder/delete existing.
     Raw research dumps, command output, captured tables go here. -->

### 2026-11-14 — Capability probe 実機実行（この worktree の Windows ホスト）

環境: Windows 11 `10.0.26300.9974` x86_64（`DESKTOP-NH00KUI`）。driver 未 provisioning の状態で実行。

- `mise exec -- cargo run -p ruviform-audio-device --example endpoint_probe` → exit 0。`evidence/rf629/endpoint_probe-windows-local.txt` に全文保存。要点: MMDevice 61 endpoint 列挙（active render 7 / capture 2、残り not-present）、name heuristic による virtual candidate 0 件、`provisioning: NOT_PROVISIONED`、全 virtual-endpoint case `NotRun`。`IsFormatSupported` は `ppclosestmatch` 有効時に全 active endpoint で `supported`（f32/2ch/48k・44.1k・s16/2ch/48k）。`GetDevicePeriod` default=100000/minimum=30000 (100ns)、`GetStreamLatency` は `AUDCLNT_E_NOT_INITIALIZED`（Initialize 未呼出のため想定どおり）。
- `mise exec -- cargo run -p ruviform-midi-device --example midi_endpoint_probe` → exit 0。`evidence/rf629/midi_endpoint_probe-windows-local.txt` に全文保存。要点: WinMM input 0 port / output 1 port（Microsoft GS Wavetable Synth, mid=0x0001 pid=0x001b）、loopback candidate 0 件、`NOT_PROVISIONED`、全 virtual case `NotRun`。
- 途中観測: `midiOutMessage` + `DRV_QUERYDEVICEINTERFACE` が GS Wavetable Synth で size=2 を返し follow-up で segfault → probe を input-path query のみに修正して再実行（上記が修正後の結果）。

### 2026-11-14 — 候補調査メモ（Web 一次情報）

- **VB-CABLE**: donationware（個人無償、商用 $5）。zip → admin setup → **再起動要**。"CABLE Input/Output" が WASAPI endpoint pair として出る。内部 SR 48kHz・SRC 内蔵（Hi-Fi は SRC なし bit-perfect・最大 24bit/384kHz）。driver は署名済み。https://shop.vb-audio.com/en/win-apps/11-vb-cable.html / https://vb-audio.com/Cable/VBCABLE_ReferenceManual.pdf
- **VAC**: 商用 perpetual $30 home/$50 business。署名済み（reseller cert）。最大 256 cable・multi-client・format 上限 384kHz/32bit/32ch・channel remap・**position/clock register support**。trial は 30 分 reminder・Lite は 1 cable/48k/16bit/1 client で CI 不適。https://vac.muzychenko.net/en/purchase.htm / https://vac.muzychenko.net/en/manual/install.htm
- **loopMIDI**: 無償・私的非商用のみ・再配布は書面同意要。teVirtualMIDI 署名 driver 上に構築。port は app 実行中のみ存在・ユーザ単位・動的作成/破棄。midiIn/midiOut 双方に同名で出る。https://www.tobias-erichsen.de/software/loopmidi.html / https://www.tobias-erichsen.de/software/virtualmidi.html
- **Windows MIDI Services**: Win11 25H2+ in-box。WinMM/WinRT は新 service へ repoint（`wdmaud2.drv` 互換層）。loopback は `New-MidiLoopback`（MIDI 2.0 A/B pair）/`New-MidiBasicLoopback`（MIDI 1.0 単一 endpoint）で作成、transient or `-SaveToConfiguration`。https://microsoft.github.io/MIDI/ / https://microsoft.github.io/MIDI/tools/midiloopbacksetup/ / https://microsoft.github.io/MIDI/tools/powershell/
- **SysVAD**: Microsoft sample driver。WDK build + `bcdedit /set TESTSIGNING ON` + cert install + reboot + devcon install。test-signing 前提のため専用機限定。https://learn.microsoft.com/en-us/samples/microsoft/windows-driver-samples/sysvad-virtual-audio-device-driver-sample/

### 2026-11-16 — 4th slice: endpoint_probe 拡張 + streaming verifier fail-closed 確認（この worktree の Windows ホスト）

環境: Windows 11 `10.0.26300.9974` x86_64（`DESKTOP-NH00KUI`）。仮想 endpoint 未 provisioning のまま実行。

- `mise exec -- cargo run -p ruviform-audio-device --example endpoint_probe` → exit 0。`evidence/rf629/endpoint_probe-windows-local.txt` を更新。要点: MMDevice 65 endpoint 列挙、production port view は output 5 / input 2（Audient iD4 analogue+loopback、NVIDIA HDA、HD Audio Device）。新設の `== pair candidates (virtual name-stem match) ==` は `none — NOT_PROVISIONED`。driver signature 観測は `Win32_PnPSignedDriver` で 1 件ヒット（"NVIDIA Virtual Audio Device (Wave Extensible) (WDM)"、provider=NVIDIA、inf=oem77.inf、catalog:absent — 情報記録であり仮想 endpoint の証明ではない）。permission 観測は `ConsentStore\microphone => Allow`、`…\NonPackaged => Allow`、`DeviceAccess\Global\{2EEF81BE-…} => absent`。cleanup 行で persistent state なしを明記。
- `cargo test -p ruviform-audio-device --features real-device --test virtual_endpoint -- --ignored --test-threads=1 --nocapture`（補助証拠・canonical=mise lane 未登録）→ exit 0、`verdict: NotRun — NOT_PROVISIONED`。verifier は `real-device` feature + `#[ignore]` で、pair candidate 不在時に assert/panic せず正直に NotRun を返すことを実機で確認。evidence は `evidence/rf629/virtual_endpoint_slice4_verification-windows-local.txt`。
- 補足: libtest は passing test の stdout/stderr を capture するため、verdict 行を evidence として採取する際は `--nocapture` が必要（canonical 起動式自体は task spec どおり `--ignored --test-threads=1`）。

### 2026-11-16 — 5th slice: `test-audio-virtual-endpoint` opt-in lane 登録 + self-run（この worktree の Windows ホスト）

環境: Windows 11 `10.0.26300.9974` x86_64（`DESKTOP-NH00KUI`）。仮想 endpoint 未 provisioning のまま実行。

- `mise.toml` に `[tasks.test-audio-virtual-endpoint]` を登録 — `cargo test -p ruviform-audio-device --features real-device --test virtual_endpoint -- --ignored --test-threads=1`（4th slice design の AC6 lane 設計記録どおり）。provisioning 依存の manual opt-in lane で CI 対象外、未 provisioning では `verdict: NotRun`（fail-closed）。専用 lint lane は追加せず既存 `lint-audio-real-input`（`--all-targets --features real-device`）が本 test target をカバー（Coordinator 確定）。
- 上記 2026-11-16 4th slice 記録の「canonical=mise lane 未登録」は本 slice で解消 — canonical 起動式は `mise run test-audio-virtual-endpoint`。証拠採取時の `--nocapture` 注意は変わらず有効（mise lane 経由でも libtest の capture は同じ — verdict 行を evidence に採る場合は lane の `run` に `--nocapture` を足すか、補助証拠として直接 cargo 実行する）。
- `mise run test-audio-virtual-endpoint` self-run → exit 0、`test render_capture_round_trip ... ok`（1 passed）。canonical lane の `run` には `--nocapture` を含めないため verdict 行は libtest capture 内 — 補助証拠として `--nocapture` 付き直接実行を併記し `verdict: NotRun — no input devices on this host`（fail-closed・pair 未 provision と同じ NotRun 系。4th slice 時点の `NOT_PROVISIONED` とは理由文字列が異なるが判定は同じ）を確認。evidence は `evidence/rf629/virtual_endpoint_slice5_lane_selfrun-windows-local.txt`。Pass は主張しない（provisioned runner での実走は外部 gate として残る）。

### 2026-11-17 — closeout-prep slice: MIDI 仮想 lane 分離 + provisioning runbook + capability matrix 確定 + AC7 照合（headless）

環境: Windows 11 `10.0.26300.9974` x86_64（`DESKTOP-NH00KUI`）。仮想 endpoint 未 provisioning のまま実行。

- `mise.toml` に `[tasks.test-midi-virtual-endpoint]` を登録 — `cargo test -p ruviform-midi-device --test loopback -- --ignored --test-threads=1`（`loopback_round_trip` 仮想 pair verifier + `real_device_silent_output_smoke` GS Wavetable smoke の 2 テストを target 全体で収集。feature gate なしのため `--features` 不要 — Coordinator 確定）。provisioning 依存の manual opt-in lane で CI 対象外、未 provisioning では `verdict: NotRun`（fail-closed）。専用 lint lane は追加せず既定 `lint ruviform-midi-device`（`--all-targets`）が `loopback` target をカバー。`test-midi-real-device` は run 不変の包括 lane とし description/注記で仮想 pair 専用 lane との関係を明記。`docs/development-environment.md` の lane 表に `test-midi-real-device`（従来未収録の doc gap）と本 lane を併記。
- `crates/ruviform-midi-device/tests/loopback.rs` の module doc（「`test-midi-real-device` が `#[ignore]` を収集するので lane 登録不要」— stale）と `loopback_round_trip` doc comment の canonical lane 参照を `mise run test-midi-virtual-endpoint` に更新（挙動不変 — doc のみ）。
- provisioning runbook を `docs/designs/issue-629-provisioning-runbook.md` に文書化 — 専用 Windows runner 要件（repo 外工程・開発 PC 無断 driver 導入禁止）・license 事前確認を必須ゲート化（VB-CABLE donationware/loopMIDI 私的非商用限定/VAC 商用 perpetual/WMS loopback OS 付属 — survey Open Question #4）・候補別 install/cleanup/probe 手順（Audio=VB-CABLE/Hi-Fi/VAC、MIDI=loopMIDI/loopBE/WMS loopback、例外=SysVAD）・capability probe 判定主導フロー・両 lane 検証手順・NotRun 解釈・driver 署名/権限/cleanup の記録責任分担（probe best-effort vs runner 残件: SetupAPI 面・privacy キー体系的収集）。
- Capability Matrix 確定版を同 design に収録 — survey 暫定 matrix を引き継ぎ、❓/⚠️ 項目と Open Questions #1-#7 を「provisioning 実測待ち」として明記（確定済みとは主張しない）。
- AC7 consumer 照合を同 design に記録 — `scarecrow173/synvelis#115`（仮想 WASAPI Endpoint→本番 Callback + 独立 Oracle、物理 iD4/USB 抜去/ADC-DAC は未保証明記済み）・`scarecrow173/synvelis#117`（物理 Direct Hardware Monitoring は仮想で未保証・通常必須にしない明記済み）・`scarecrow173/synvelis#126`（仮想 WinMM Port→timestamp→Host の L3、物理 MIDI 機材は通常必須でなく MIDI2 は `scarecrow173/ruviform#317` future）・`scarecrow173/audio-lens#127`（device boundary 非消費で #629/#630 は NotApplicable 方針）。機材固有未保証範囲（物理 USB 抜去・ADC/DAC bit-exact・機種固有 driver・物理 Direct Monitoring・UMP/MPE）を明記。
- 本 slice で解消: AC6 MIDI lane 分離・AC4 runbook 文書化・AC5 headless matrix 確定・AC7 headless 照合。未解消（後続/外部 gate）: 実 provisioning・driver install・license 判断・実走 Pass（AC1/AC2/AC5 実測）・`.github/**` CI/runner 統合・AC3 typed 結線・consumer 側独立 Oracle での実受入判定・`real-abi-test-plugin`↔`test-plugin` 語彙 gap（#389 占有中）。
- lane self-run（`test-midi-virtual-endpoint`/`test-midi-real-device`/`test-audio-virtual-endpoint`）の結果は `evidence/rf629/` に記録 — 未 provisioning のため `verdict: NotRun`（fail-closed）であり Pass を主張しない。

### 2026-11-17 — AC3 Test Runtime wiring slice: 逆方向 dev-dep 裁定 + verdict→RunMetadata 結線 + deterministic signal/Event 供給（headless）

環境: Windows 11 `10.0.26300.9974` x86_64（`DESKTOP-NH00KUI`）。仮想 endpoint 未 provisioning のまま実行。

- 依存方向裁定（design §1・ADR 相当）: `ruviform-audio-device`→`ruviform-audio-test` dev-dep は不承認のまま。逆方向 `ruviform-audio-test` dev-deps → `ruviform-audio-device`/`ruviform-midi-device`（test target のみ）を採用 — crate cycle なし、`audio-plugin-bridge`→`midi-device`・`plugin-test-vst3`→両 device crate の先例と整合。`Cargo.lock` は dev-dep 辺のみ再生成。
- `RunMetadata::for_environment`（`crates/ruviform-audio-test/src/metadata.rs`）を additive 追加 — `result` は `NotRun` 初期化で構築のみでは Pass にならない fail-closed。既存 `deterministic` shorthand・serde 語彙は不変。
- `crates/ruviform-audio-test/tests/os_endpoint.rs`（新設）— deterministic legs 3 件（全ホスト実行）+ `#[ignore]` leg 2 件。MIDI leg は send list を `MidiScenario`→`run_midi_trace` 由来とし expected は独立 literal 4-event oracle の ordered subsequence。Audio leg は `virtual_endpoint.rs` の name-stem pairing + `PcmFixture` deterministic PCM + 独立 marker/RMS oracle。両 leg は実際に endpoint を stream した candidate run のみ `level=ext-l3`/`environment_kind=virtual-os-endpoint` の `RunMetadata` を emit — name match のみの Pass は OS-L1 証拠に留める。
- wiring leg self-run（補助証拠・canonical=mise lane 未登録）: `cargo test -p ruviform-audio-test --test os_endpoint -- --ignored --nocapture` → exit 0、audio leg が `observed:"no input devices on this host"`、MIDI leg が `observed:"NOT_PROVISIONED — no bidirectional name match"` の `run_metadata` JSON（いずれも `result:"not-run"`, `level:"ext-l3"`, `environment_kind:"virtual-os-endpoint"`）+ `verdict: NotRun` を emit。audio leg の理由は host の input device 列挙状態依存で、device verifier lane でも `NOT_PROVISIONED`/`no input devices` の両方が観測済み（同じ fail-closed NotRun 系）。evidence: `evidence/rf629/os_endpoint_wiring_selfrun-windows-local.txt`。
- lane self-run/regression: `test-audio-virtual-endpoint`・`test-midi-virtual-endpoint`・`test-midi-real-device` 全て exit 0・既存 verifier 挙動不変（`loopback_round_trip` は NotRun、`real_device_silent_output_smoke` は GS Wavetable への Pass — 仮想証明ではない）。evidence: `evidence/rf629/test_runtime_wiring_slice_selfrun-windows-local.txt`。
- stale 参照同期: 3 design（wasapi-endpoint/virtual-endpoint-lane/provisioning-runbook）の AC3 typed 結線 Deferred を「解消済み + provisioned 実走は外部 gate に残る」へ更新。`virtual_endpoint.rs`/`loopback.rs` の module doc は結線状況を直接記述していないため変更不要。
- 本 slice で解消: AC3 headless typed 結線。未解消（後続/外部 gate）: provisioned runner 実走 Pass（AC1/AC2/AC5 実測）・`.github/**` CI/runner 統合・AC4 実値記録・AC5 実測項目・consumer 実受入（AC7）・wiring test の canonical lane 登録（`mise.toml` は本 slice 範囲外）。lint/compile の Pass を `virtual-os-endpoint` や実 OS L3 実走 Pass と主張しない。
