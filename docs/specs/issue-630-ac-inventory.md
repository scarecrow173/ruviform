---
id: "SPEC-rf630-ac-inventory-01"
type: "spec"
status: "approved"
title: "Issue #630 AC disposition census — VST3/CLAP Test Plugin Suite AC の行別 disposition と残件分類"
created: "2026-11-24"
updated: "2026-11-26"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-ac-inventory.md"
  changes:
    added:
      - type: "doc"
        path: "evidence/rf630/closeout-verification.md"
    modified:
      - type: "doc"
        path: "docs/specs/issue-630-ac-inventory.md"
      - type: "doc"
        path: "docs/discovery/issue-630-ac-inventory.md"
      - type: "doc"
        path: "docs/test-specs/issue-630-ac-inventory.md"
      - type: "doc"
        path: "evidence/rf630/ac-inventory.md"
      - type: "doc"
        path: "docs/evidence/plugin-format-evidence.md"
      - type: "doc"
        path: "docs/knowledge/plugin-format-adapters.md"
  depends-on:
    - "docs/discovery/issue-630-ac-inventory.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-388-virtual-first-ac.md"
    - "POLICIES.md"
  verified-by:
    - "docs/test-specs/issue-630-ac-inventory.md"
metadata:
  issue:
    number: 630
    url: "https://github.com/scarecrow173/ruviform/issues/630"
---

## 概要

Issue #630 の受入条件 7 行に対し、canonical ledger との突合結果をもとに行別
disposition（現行 verdict の確定）を記録し、残件を 3 類に分類する。本 spec は
Issue 本文・verdict 意味・実装を変更しない close-out 判定材料であり、
checkbox の `[x]` 適用と Issue close は Coordinator の判定に委ねる。証拠の
生データ表は `evidence/rf630/ac-inventory.md`、突合手順と正直性チェックは
`docs/test-specs/issue-630-ac-inventory.md`、証拠対応の論理は
`docs/discovery/issue-630-ac-inventory.md`、closeout の census 記録は
`evidence/rf630/closeout-verification.md` を参照。

本版は closeout docs-only slice（2026-11-26・base SHA `ab6dc99f`）での再
census に基づく改訂である。2026-11-24 版からの主要な変化: CLAP real lane は
editor verbs（`ruviform.editor-ops` seam）と fault-carrier の実装により
14 case 全 pass となり、implementable 残件は解消した。ABI validator は
pin-47e2424 へ更新され preset-discovery の宣言的除外は解消、VST3 は
282 passed/0 failed。Issue 本文の checkbox は AC1〜AC6 が `[x]`、AC7 が
`[ ]`（2026-11-26 確認）であり、本 spec の disposition と矛盾しない。

## AC 行別 disposition

| # | AC 行（要約） | disposition | Issue checkbox との整合 |
| --- | --- | --- | --- |
| 1 | VST3/CLAP 実ファイルを本番 Host Path で lifecycle・Fake 非経由 | **Pass** — 両 format で production bridge 経由の host_path tests・headless `host-lifecycle`・L1 e2e・real attach が実測 pass | `[x]`（適用済み）と整合 |
| 2 | 既知 PCM/…/Crash/Hang/Timeout/Isolation を独立 Oracle で検証 | **Pass** — 4 fixture + spawn/pre-Hello fault + L1 fan-out/isolation + real-session legs で対象語彙を両 format で実走。CLAP leg の crash/hang は fault-carrier（`test_editor` の共有 `Fault` param）で pass 化済み。残件は OS-API MIDI 経路（rf#762 gate）のみで、行の判定語彙自体は Pass | `[x]`（適用済み）と整合（残件は下表で分離記録） |
| 3 | Host 独立 Validator/Reference Host・バイアス排除・SDK/License 記録 | **Pass** — clap-validator pin-47e2424（`0.4.1-5-g47e2424`・MIT）+ VST3 SDK validator 3.8.1（`v3.8.1_build_84`・MIT License per `vst3sdk/LICENSE.txt`）の canonical 7/7 leg Passed・VST3 282 passed/0 failed。`test_fault` の `param-(set\|fuzz)` 宣言的除外と informational unfiltered leg の Unsupported は non-pass 記録として残り、upstream 再評価は #867 | `[x]`（適用済み）と整合 |
| 4 | Native Editor 実 Window/child HWND/画素/Input/…・DComp Fail 再現・Fake/Floating 非 Pass | **Pass** — VST3 leg 32 case・CLAP leg 14 case ともに全 pass（`2dfd2e0f`）。CLAP の input/focus/popup は `ruviform.editor-ops` seam 経由の effect-verified、crash/hang は fault-carrier で実走。**qualified pass 注記**: `set_focus_target(HostWindow)` leg は両 ABI で `accepted-declared`・`effect_verified:false` — container への OS focus move は lane-topology 依存で未検証と正直に記録し、同-queue `host_path` lane が真の oracle。この caveat は ledger verdict を fail に倒すものではないが、effect-verified Pass とも区別する | `[x]`（適用済み）と整合（caveat は closeout 証拠に明記） |
| 5 | version/digest/RT/OS/SHA 再現・安全な Timeout/kill | **Pass** — ledger field + watchdog kill + deadline policy + exactly-once が実走済み | `[x]`（適用済み）と整合 |
| 6 | 3OS OS-L1 既存 CI 統合・Windows GUI-L2/EXT-L3 opt-in 分離 | **Pass**（CI 構造として充足）— `test-full`/`test-l1-real-plugin` が 3OS で fixture を実走、GUI-L2/EXT-L3 は manual/opt-in。`ci.yml` は workflow_dispatch のみで `daily-ci.yml` が main 検証を集約。merge 後対象 SHA の CI 実走証拠は Coordinator gate | `[x]`（適用済み）と整合（CI 実走証拠の収集確認は Coordinator） |
| 7 | sv#101/#127/#128/#176・al#127 の独立 Consumer Oracle 照合・AudioLens 新依存非強制 | **部分 Pass** — manifest が contract match・al#127 NotApplicable・dep 非汚染を提供済み（ruviform 側 done）。consumer 実走 legs は external-owner のまま — sv#176 S1 の headless real-ABI legs は synvelis 側実走済みだが Native Dock/PD 受入は未実走。false Pass にしない | `[ ]` 維持が整合（行が consumer 側照合を含むため — ruviform 側 done の記録で部分適用・close するかの判断は Coordinator） |

## 残件の 3 類分類

**settled-Unsupported**（構造的に Unsupported 確定・再実装対象ではない）

- headless `editor-attach` — `test_gain` に editor surface がない fixture 法則。
  `createView` 不在の `HostError::Unsupported` は定義済 refusal で、dock 経路は
  `test_editor` が real lane で担う。
- headless `dock-pixels` — editor surface 不在 + VisualCapture driver 不在の
  構造限界による `not_run`。real lane（`real-dcomp-pixel`）が editor 画素を担う。

**implementable**（ruviform 側で実装可能な残件）

- closeout 時点では **解消済み（残件なし）**。2026-11-24 版で implementable と
  分類していた 2 件はいずれも実装・実走 pass 済みである:
  - CLAP `SendGuiKey`/`SetGuiFocus`/popup verb — `ruviform.editor-ops` private
    extension seam（wire v8・fixture-private vtable）として実装され、
    `real-clap-editor-{input,focus,popup}` は effect-verified pass。
  - CLAP fault arm — `test_editor` が共有 `Fault` param を担う fault-carrier
    （VST3 #885 の CLAP mirror）として実装され、`real-clap-session-{crash,hang}`
    は in-band arm で pass。
- `real-session-rt-boundary` — 現 ledger では pass（rt-boundary の判定語彙が
  解消済み）。かつての `!Send` fixture 境界による Unsupported は Issue #920
  （CLOSED）で re-scope 済みであり、残件ではない。

**externally-gated**（外部条件待ち — ruviform 単独で閉じられない）

- `test-os-endpoint-midi-e2e`（rf#629 scope）— OS-API 仮想 MIDI endpoint 経由の
  `test_instrument` e2e は provisioned runner 不在で `NotRun`。runner は
  `rf#762` 待ち（Issue #630 直接対象外だが AC1/AC2 の完全経路に関わる）。
- consumer 実走 legs — sv#101/#127/#128/#176・al#127 の consumer repo 側
  実走は external-owner（AC7 の残り）。ruviform 側は contract match・
  dep 非汚染を提供済み。
- clap-validator upstream release — `param-fuzz` panic（informational
  unfiltered leg・exit 101）の修正は upstream fix 含む release 待ち
  （再評価 Issue: `scarecrow173/ruviform#867` OPEN）。preset-discovery の
  除外は pin-47e2424 で解消済みだが、pin は release binary ではなく upstream
  commit build のため、release 版への切替えも #867 で再評価する。
- macOS/Linux GUI lane — POLICIES §2 の Deferred。

## scope out

- 本 spec は AC disposition の確定と分類のみ。以下は行わない: Issue 本文の
  checkbox 編集・Issue close（Coordinator 判定）、real-window ledger の再生成
  （read-only 正本 — closeout では `2dfd2e0f` の ledger をそのまま証拠採用）、
  fixture/route/catalog の意味変更、新規 slice、docs index
  （`docs/**/README.md`）の更新（Coordinator が統合時に整備）。
- computer-use による実 GUI 受入と real lane の再実行は行わない
  （2026-11-26 ユーザー決定 — 既存 real lane ledger の pass 記録を closeout
  証拠として採用）。
