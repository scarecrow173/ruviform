---
id: "SPEC-rf630-ac-inventory-01"
type: "spec"
status: "approved"
title: "Issue #630 AC disposition census — VST3/CLAP Test Plugin Suite AC の行別 disposition と残件分類"
created: "2026-11-24"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-ac-inventory.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-630-ac-inventory.md"
      - type: "doc"
        path: "docs/discovery/issue-630-ac-inventory.md"
      - type: "doc"
        path: "docs/test-specs/issue-630-ac-inventory.md"
      - type: "doc"
        path: "evidence/rf630/ac-inventory.md"
    modified:
      - type: "file"
        path: "evidence/rf631/plugin-dock-scenario.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console-list.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console-list-include-real.jsonl"
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
checkbox の `[x]` 適用は Coordinator の判定に委ねる。証拠の生データ表は
`evidence/rf630/ac-inventory.md`、突合手順と正直性チェックは
`docs/test-specs/issue-630-ac-inventory.md`、証拠対応の論理は
`docs/discovery/issue-630-ac-inventory.md` を参照。

## AC 行別 disposition

| # | AC 行（要約） | disposition | Issue checkbox 更新提案 |
| --- | --- | --- | --- |
| 1 | VST3/CLAP 実ファイルを本番 Host Path で lifecycle・Fake 非経由 | **Pass** — 両 format で production bridge 経由の host_path tests・headless `host-lifecycle`・L1 e2e・real attach が実測 pass | `[x]` 提案 |
| 2 | 既知 PCM/…/Crash/Hang/Timeout/Isolation を独立 Oracle で検証 | **Pass** — 4 fixture + spawn/pre-Hello fault + L1 fan-out/isolation + real-session legs で対象語彙を両 format で実走。残件は CLAP leg crash/hang（implementable）と OS-API MIDI 経路（rf#762 gate）であり、行の判定語彙自体は Pass | `[x]` 提案（残件は下表で分離記録） |
| 3 | Host 独立 Validator/Reference Host・バイアス排除・SDK/License 記録 | **Pass** — clap-validator 0.4.1 + VST3 SDK validator 3.8.1 の全 canonical leg Passed。宣言的除外は non-pass 記録で upstream 再評価は #867 | `[x]` 提案 |
| 4 | Native Editor 実 Window/child HWND/画素/Input/…・DComp Fail 再現・Fake/Floating 非 Pass | **部分 Pass** — VST3 leg（`test_editor`）は全判定語彙 pass。CLAP leg は 9 pass + 5 unsupported（implementable 残件） | `[ ]` 維持提案（CLAP leg 残件が行内にあるため — 分割判定する場合の選択肢は Coordinator） |
| 5 | version/digest/RT/OS/SHA 再現・安全な Timeout/kill | **Pass** — ledger field + watchdog kill + deadline policy + exactly-once が実走済み | `[x]` 提案 |
| 6 | 3OS OS-L1 既存 CI 統合・Windows GUI-L2/EXT-L3 opt-in 分離 | **Pass**（CI 構造として充足）— `test-full`/`test-l1-real-plugin` が 3OS で fixture を実走、GUI-L2/EXT-L3 は manual/opt-in。merge 後対象 SHA の CI 実走証拠は Coordinator gate | `[x]` 提案（CI 実走証拠の収集確認後） |
| 7 | sv#101/#127/#128/#176・al#127 の独立 Consumer Oracle 照合・AudioLens 新依存非強制 | **部分 Pass** — manifest が contract match・al#127 NotApplicable・dep 非汚染を提供済み。consumer 実走 legs は external-owner | `[ ]` 維持提案（行が consumer 側照合を含むため — ruviform 側 done の記録で部分適用する選択肢は Coordinator） |

## 残件の 3 類分類

**settled-Unsupported**（構造的に Unsupported 確定・再実装対象ではない）

- headless `editor-attach` — `test_gain` に editor surface がない fixture 法則。
  `createView` 不在の `HostError::Unsupported` は定義済 refusal で、dock 経路は
  `test_editor` が real lane で担う。
- headless `dock-pixels` — editor surface 不在 + VisualCapture driver 不在の
  構造限界による `not_run`。real lane（`real-dcomp-pixel`）が editor 画素を担う。

**implementable**（ruviform 側で実装可能な残件）

- CLAP `SendGuiKey`/`SetGuiFocus`/popup verb — `real-clap-editor-input`/
  `-focus`/`-popup` が CLAP L0 ops surface の trait-default `Unsupported`。
  fixture 側に event log・observable channel を実装して ops surface を拡張する
  作業（#912 の param reporting と同系の拡張点）。
- CLAP fault arm — `real-clap-session-crash`/`-hang`。`test_editor` が
  parameterless fixture で VST3 側の in-band `Fault` param carrier を持たず、
  `plugin_id` keyed route が co-resident `test_fault` arm を許さない。
  CLAP 側 param carrier の実装、または route/Bundled pool API 変更で解く
  余地がある。
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
- clap-validator upstream release — `preset-discovery-crawl`/`-load` の
  宣言的除外解除と `param-fuzz` panic 修正は upstream fix 含む release 待ち
  （再評価 Issue: `scarecrow173/ruviform#867` OPEN）。
- `mise.toml` lane comment の「editor-pixel dock acceptance stays deferred」—
  `real-dcomp-pixel` は editor の screen-composited 画素を実測しており、
  comment 由来の deferred 表現に対応する open tracker は現時点でない（#532
  CLOSED・residual disposition 確定）。深堀りの dock acceptance 拡張が必要に
  なった場合のみ再 scope する。
- macOS/Linux GUI lane — POLICIES §2 の Deferred。

## scope out

- 本 spec は AC disposition の確定と分類のみ。以下は行わない: Issue 本文の
  checkbox 編集（Coordinator 判定）、real-window ledger の再生成（read-only
  正本）、fixture/route/catalog の意味変更、新規 slice、docs index
  （`docs/**/README.md`）の更新（Coordinator が統合時に整備）。
- `evidence/rf631/plugin-dock-{scenario,console,console-list,
  console-list-include-real}.jsonl` の再生成は占有範囲内で実施済み
  （`ff7013dc` → `06a5a9e0`）。
