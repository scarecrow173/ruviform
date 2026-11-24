---
id: "SPEC-rf631-ac-inventory-01"
type: "spec"
status: "approved"
title: "Issue #631 AC disposition census — GUI Test Driver AC の行別 disposition と残件分類"
created: "2026-11-24"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "docs/discovery/issue-631-ac-inventory.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-631-ac-inventory.md"
      - type: "doc"
        path: "docs/discovery/issue-631-ac-inventory.md"
      - type: "doc"
        path: "docs/test-specs/issue-631-ac-inventory.md"
      - type: "doc"
        path: "evidence/rf631/ac-inventory.md"
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
    - "docs/discovery/issue-631-ac-inventory.md"
    - "docs/specs/issue-388-virtual-first-ac.md"
    - "POLICIES.md"
  verified-by:
    - "docs/test-specs/issue-631-ac-inventory.md"
metadata:
  issue:
    number: 631
    url: "https://github.com/scarecrow173/ruviform/issues/631"
---

## 概要

Issue #631 の受入条件 7 行に対し、canonical ledger との突合結果をもとに行別の
disposition（現行 verdict の確定）を記録し、残件を 3 類に分類する。本 spec は
Issue 本文・verdict 意味・実装を変更しない close-out 判定材料であり、
checkbox の `[x]` 適用は Coordinator の判定に委ねる。証拠の生データ表は
`evidence/rf631/ac-inventory.md`、突合手順と正直性チェックは
`docs/test-specs/issue-631-ac-inventory.md`、case→AC の論理は
`docs/discovery/issue-631-ac-inventory.md` を参照。

## AC 行別 disposition

| # | AC 行（要約） | disposition | Issue checkbox 更新提案 |
| --- | --- | --- | --- |
| 1 | 実 Window 起動/入力/Receipt/画素/実 State・2Window Focus/Close/Undo・DPI/IME | **Pass** — 32/32 case の実測 pass で充足。Undo は consumer 语义として catalog 非収載の記録済み判断（NotApplicable 相当）であり、行の残り判定語彙は全て Pass | `[x]` 提案（Undo の扱いを行内で明記する判断は Coordinator） |
| 2 | Lab/Synvelis/AudioLens GUI へ driver-neutral 投影・製品 Oracle は各 repo | **部分 Pass** — `plugin-dock-lab` で Test Lab は実在・driver-neutral 投影 seam は成立。consumer 実走 legs（sv#114/#176・al#127）は external-owner の残件 | `[ ]` 維持提案（ruviform 側は done だが行が consumer 実走を含むため） |
| 3 | 実 Child HWND→NativeSurfaceSlot・DComp 既知 Fail 再現・Pass 判定・未解決を解消済みにしない | **Pass** — NativeSlot・attach・DComp family・input/focus/resize/redock 全 pass。class C 確定機序は `expected_outcome` 記録で「未解決を解消済みにしない」条項を遵守 | `[x]` 提案 |
| 4 | 証拠 field schema・verdict 異軸 | **Pass** — 全必須 field が ledger に存在し verdict 軸は分離済み | `[x]` 提案 |
| 5 | fail-closed NotRun・capability NotApplicable・macOS/Linux Deferred | **Pass** — blocked-probe ledger が fail-closed を bin 自己検査で実証 | `[x]` 提案 |
| 6 | 共通 UI/API 製・GPUI 型非漏出 | **Pass** — `check-ui-driver-boundary` が CI に収載・機械検査済み | `[x]` 提案 |
| 7 | 3OS OS-L1 seam 既存 CI・real lane を PR 必須にしない | **Pass**（CI 構造として充足）— headless seam が 3OS verify job に収載、real lane は opt-in。merge 後対象 SHA の CI 実走証拠は Coordinator gate | `[x]` 提案（CI 実走証拠の収集確認後） |

## 残件の 3 類分類

**settled-Unsupported**（構造的に Unsupported 確定・再実装対象ではない）

- headless `editor-attach` — `test_gain` は editor surface を持たない fixture
  法則により `createView` 不在を `HostError::Unsupported` として記録。dock
  経路は `test_editor` が real lane で担うため、headless 側で editor を装う
  必要はない。
- headless `dock-pixels` — editor surface 不在 + VisualCapture driver 不在の
  構造限界により `not_run` が定義済 verdict。real lane（`real-dcomp-pixel`）
  が editor 画素を担う。

**implementable**（ruviform 側で実装可能 — ただし Issue #630 AC4 側の残件であり、
Issue #631 自身の AC では catalog 提供済み）

- Issue #631 側に新規 implementable 残件はない。CLAP leg の 5 unsupported は
  fixture/ops surface の実装問題で、Issue #630 AC4 の残件として
  `docs/specs/issue-630-ac-inventory.md` に記録する。

**externally-gated**（外部条件待ち — ruviform 単独で閉じられない）

- consumer 実走 legs: Synvelis `synvelis#114`/`#176`・AudioLens
  `audio-lens#127` への実投影と製品固有 oracle の実走は consumer repo 責務
  （AC2 の残り）。ruviform 側は投影 seam と contract を提供済み。
- macOS/Linux GUI lane — POLICIES §2 の Deferred（要件発生まで実装しない）。
- provisioned runner（`rf#762`）— Issue #630 側の OS-API MIDI 経路に関係する
  外部 runner で、#631 の AC には直接対応しないが runner 全体の残件として併記。

## scope out

- 本 spec は AC disposition の確定と分類のみ。以下は行わない:
  Issue 本文の checkbox 編集（Coordinator 判定）、real-window ledger の再生成
  （read-only 正本）、新規 case の追加・verdict 意味の変更、docs index
  （`docs/**/README.md`）の更新（Coordinator が統合時に整備）。
- `plugin-dock-scenario.jsonl` 等の stale ledger 再生成は本 slice の占有範囲内
  で実施済み（`ff7013dc` → `06a5a9e0`）。
