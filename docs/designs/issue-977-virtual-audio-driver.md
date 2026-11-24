---
id: "DESIGN-977vAdrSv1Stg1"
type: "design"
status: "draft"
title: "Issue #977 仮想 audio endpoint driver — SysVAD 派生最小構成と手動 install probe"
created: "2026-11-30"
updated: "2026-11-30"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/977"
    - "docs/designs/issue-952-virtual-endpoint-driver.md"
    - "evidence/rf961/verification.md"
    - "tools/testsigned-driver/probe-stage.ps1"
    - "tools/testsigned-driver/TestDevice.ps1"
    - ".github/workflows/virtual-driver-install-probe.yml"
    - ".github/workflows/virtual-endpoint-probe.yml"
  changes:
    added:
      - type: "file"
        path: "drivers/rvf-virtual-audio/**"
      - type: "file"
        path: "tools/virtual-audio-driver/**"
      - type: "file"
        path: ".github/workflows/virtual-audio-driver-probe.yml"
      - type: "file"
        path: "docs/designs/issue-977-virtual-audio-driver.md"
      - type: "file"
        path: "evidence/rf977/**"
    modified:
      - type: "file"
        path: "drivers/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/specs/issue-629-provisioning-runbook.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-952-virtual-endpoint-driver.md"
    - "evidence/rf961/verification.md"
    - "POLICIES.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-629-provisioning-runbook.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/977"
    - "docs/designs/issue-952-virtual-endpoint-driver.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  defers: []
  deferred-by: []
---

# Issue #977 仮想 audio endpoint driver — SysVAD 派生最小構成と手動 install probe

## Context

Issue #952 Slice B の実装 slice（Issue #977 stage 1）。#952 で確定した方針 —
独自 driver を ruviform の資産として作り、`windows-2025`（x64・testsigning=Yes
出荷状態）で test-signing 経路を通す — に基づき、SysVAD（MS-PL）を最小化した
仮想 audio endpoint driver のソースツリーと、build→test-sign→install→devnode→
endpoint 列挙→pair 候補検出までを段階分解して実測する manual workflow を
本 slice で提供する。

本書は stage 1 の設計判断（vendoring 範囲・endpoint 名の固定方式・INF/vcxproj
構成・workflow 契約）を記録する。runner 実走は merge 後に Coordinator が
`workflow_dispatch` で起動する（dispatch は default branch 上の workflow が
必要なため PR 内では実走しない）。

## Scope

### In Scope

- `drivers/rvf-virtual-audio/` — SysVAD 派生の最小 driver ソースツリー
  （INF + vcxproj + C/C++ + MS-PL license + provenance + README）。
- `tools/virtual-audio-driver/` — endpoint_probe 実行 helper。
- `.github/workflows/virtual-audio-driver-probe.yml` — `workflow_dispatch`
  専用・windows-2025 x64 のみ・absence-is-data stage 記録。
- `mise.toml` への build lane 追加（最小差分）。
- `docs/specs/issue-629-provisioning-runbook.md` への provisioning 追記。
- `evidence/rf977/` — ローカル検証記録の frame。

### Out of Scope

- runner 実走と `evidence/rf977/` への runner 証拠投入（Coordinator が
  merge 後に実施）。
- AC3 相当の streaming round-trip 検証（`tests/virtual_endpoint.rs` の
  `#[ignore]` verifier 実走 — pair 提供は本 slice の成果物だが走行は
  後続 stage）。
- `crates/**` の変更（endpoint probe / verifier の既存契約は不変）。
- ARM64 対応（`windows-11-arm` は VBS CI で kernel load が成立しない —
  rf961 attempt-9 実測。attestation 署名は Slice D 扱い）。
- docs index（`docs/**/README.md`）の更新 — Coordinator が統合時に整備。

## 方式選択とその根拠

### SysVAD 派生の最小構成

upstream `audio/sysvad/` は adapter 共有部品 + `EndpointsCommon` ライブラリ +
endpoint 別サンプル群（Tablet/Phone/NT 等・各々 speaker/mic/hdmi/spdif/
micarray/Bluetooth/USB 等を登録）の多プロジェクト構成。本 slice は
`TabletAudioSample` 構成から **speaker(render) + mic in(capture) の 1 pair**
だけを残す:

- `minipairs.h` を書き換えて `g_RenderEndpoints={SpeakerMiniports}`・
  `g_CaptureEndpoints={MicInMiniports}` のみ登録。
- それ以外の endpoint 固有ファイル（speakerhp/hdmi/spdif/micarray/bthhfp/
  usbhs/a2dphp の `*topo`/`*toptable`/`*wavtable`/`*minipairs`）は vendor
  しない — 参照は全て `minipairs.h` 経由か ifdef（`SYSVAD_BTH_BYPASS`/
  `SYSVAD_USB_SIDEBAND`/`SYSVAD_A2DP_SIDEBAND`）下であり、compile 面での
  依存は切れる。
- 多プロジェクト構成は単一 `RvfVirtualAudio.vcxproj` へ flatten した
  （`ClCompile` で全翻訳単位を直接列挙 — `EndpointsCommon` ライブラリ
  project 参照を持たない）。
- 削減・変更の対応表は `drivers/rvf-virtual-audio/PROVENANCE.md` が正本。

upstream pin: `microsoft/Windows-driver-samples` @
`d49bb7d72f3651952b2b6e4ad28d14fe2247eb69`（MS-PL —
`drivers/rvf-virtual-audio/LICENSE-MS-PL.txt`）。派生ファイルにも
upstream copyright 表記を保持する。

### endpoint 表示名の固定方式

MMDevice の endpoint 名は topology filter の bridge pin の pin-name GUID を
`HKR\MediaCategories\{guid}\Name` で引いた文字列になる。SysVAD の mic in が
`MICIN_CUSTOM_NAME` GUID で「External Microphone Headphone」を出している
機構をそのまま使い、独自 GUID 2 個を割り当てた:

| 方向 | pin | GUID | 表示名 |
| --- | --- | --- | --- |
| render（再生先） | `KSPIN_TOPO_LINEOUT_DEST` (speakertoptable.h) | `{131373ee-…}` | `Ruviform Cable Input` |
| capture（録音元） | mic in bridge pin (micintoptable.h) | `{46eab9d5-…}` | `Ruviform Cable Output` |

「Input = アプリが書き込む側・Output = アプリが読む側」は VB-CABLE と同じ
方向規約。`endpoint_probe` の `VIRTUAL_NAME_HINTS`（"cable input"/"cable
output"）に一致し、`DIRECTION_TOKENS`（input/output 除去）後の stem は両端
とも "ruviform cable" となり pair 候補として検出される。`crates/**` への
変更は一切不要。

### INF / vcxproj 構成

- `RvfVirtualAudio.inf`（UTF-8, testsigned-null 同型）は
  `ComponentizedAudioSample.inx` を単一 INF へ簡約: Class=MEDIA・hwid
  `ROOT\RvfVirtualAudio`・NTamd64 のみ・`Include=ks.inf,wdmaudio.inf` +
  `Needs=KS.Registration,WDMAUDIO.Registration`・interface AddReg 4 pair
  （topology+wave × speaker/micin）・`KmdfService` + `KmdfLibraryVersion=1.33`
  ・MediaCategories 登録・`DriverVer=01/01/2025,1.0.0.0`（過去日付 —
  rf961 attempt-8 の教訓）。
- `RvfVirtualAudio.vcxproj` は `DriverType=KMDF`・
  `PlatformToolset=WindowsKernelModeDriver10.0`・x64 Release/Debug のみ。
  `KmdfVersionNumber=1.33` を明示し INF `KmdfLibraryVersion` と揃える
  （kit default 依存を排除 — in-box KMDF 1.35 の Server 2025 では 1.33
  binding もそのまま load される）。
- link 依存: `portcls.lib;stdunk.lib;libcntpr.lib`（upstream 踏襲）+
  toolset 自動の KMDF libs。
- devnode 作成は `TestDevice.ps1` を `-ClassName MEDIA -ClassGuid
  {4d36e96c-e325-11ce-bfc1-08002be10318}` 付きでそのまま再利用
  （変更不要 — 既存の parameter 化で済む）。

### workflow 契約（virtual-driver-install-probe.yml 踏襲）

- `workflow_dispatch` 専用・matrix `os: [windows-2025]` のみ。required
  gate・ci-summary・ruleset へは接続しない独立 manual lane。
- stage 列: env-build-surface → driver-build（WDK 不在なら winget
  `Microsoft.WindowsWDK.10.0.26100` job 内導入 + DriverKit task dll
  workaround）→ catalog-create → cert-create → store-register → sign →
  sign-verify → pnputil-add-driver → enum-verify → devnode-create
  （MEDIA class + problem_code 診断）→ **endpoint-probe**（mise Rust
  toolchain で `endpoint_probe` 実行 — 新規 stage）→ cleanup → upload。
- 各 stage は `if: always()` + 末尾 `exit 0` で完走し、観測は
  `vaudio-probe-windows-2025.json` の `{stage,status,detail,exit_code}`
  と `stage-*-windows-2025.txt` に記録。status 語彙は
  `probe-stage.ps1` の ok/absent/blocked/not_run/error。
- endpoint-probe stage は devnode 未成立でも実行する（pair 不在の記録も
  判定材料）。`Invoke-EndpointProbe.ps1` が `pair_count=` 等の
  `key=value` を出力し stage detail へ載せる。

## 検証

### 本 PR で実施した検証（ローカル）

- `mise run fmt-check` / `mise exec -- prek run --all-files` / actionlint
  / `mise run test-ci-daily` / `mise run test-ci-tracker`。
- 補助検査: PowerShell `ParseFile` による `tools/virtual-audio-driver/
  Invoke-EndpointProbe.ps1` 構文検査、INF セクション構造の text 検査、
  vcxproj の XML well-formed 検査。
- driver の kernel build・署名・install・endpoint 列挙は worker ホストの
  WDK 不在によりローカル未実施 — runner 実走で判定する（NotRun 記録）。

### merge 後（Coordinator 実施）

- `gh workflow run virtual-audio-driver-probe.yml` → artifact を
  `evidence/rf977/` に採取し、stage 成立範囲を verification.md へ追記。

## Risks and Trade-offs

- **SysVAD 全面取り込みではなく 1 pair 削減**: round-trip を自分で完結
  させる仮想 cable 配線（render→capture の音声経路）は speaker+micin の
  bridge pin 接続では**提供されない** — SysVAD 各 endpoint は独立して
  silence/tone を返す構成。stage 1 の受入（build→install→endpoint 列挙
  →pair 候補検出）には不要。実 round-trip は後続 stage で wave filter 間
  の bridge 実装（または loopback 経路）を要する — 本書の Deferred。
- **Speaker endpoint が `eSpeakerDevice`（internal）判定**: minwavert は
  device type で system/external を判定する。MMDevice 名は pin-name GUID
  由来で固定するため表示名への影響はないが、AudioEndpoint の form
  factor 分類は Speaker になる。
- **KmdfVersionNumber=1.33 固定**: kit default（26100 → 1.35）に寄せる
  選択肢もあったが、INF との一致が自明な固定値を優先した。
- **.cat 生成は Inf2Cat → makecat fallback**: rf961 attempt-6 で Inf2Cat
  が不安定だったため同じ fallback を維持。
- **`endpoint_probe` をそのまま使う**: name-heuristic 一致（"cable
  input"/"cable output"）を前提にする。仮に MMDevice 名の構成が
  `<pin name> (<device desc>)` 型でなく pin name 単体でも hint 部分文字列
  一致は成立する。

## Deferred Design Concerns

- **render→capture 音声経路（真の cable 動作）**: 現在の pair は独立
  endpoint 2 個であり、speaker 側に書かれた PCM は capture 側へ流れない。
  AC3 round-trip には wave miniport 間のデータ経路（SysVAD 非搭載 —
  自前 bridge か `#629` 想定の cable 動作）の実装が別 stage で要る。
- **ARM64 coverage**: VBS CI 下で test-signing kernel load が不可 —
  attestation 署名（Slice D）が唯一経路。#952 参照。
- **capability 注入**（device-loss/xrun/permission-denial）: 独自 driver
  の利点だが本 slice では未実装。#952 §Slice B 残項。
- **docs index 同期**: 生成物のため本 slice では触らない。
