---
id: "TESTSPEC-034Yf0GO1eY84kAmbq6BBv"
type: "test-spec"
status: "approved"
title: "Issue #532 dcomp-pixel test spec"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-pixel.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/test-specs/README.md"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-532-dcomp-pixel.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "docs/designs/issue-532-dcomp-pixel.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-532-dcomp-pixel.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-532-dcomp-pixel.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 dcomp-pixel test spec

## Purpose

`real-dcomp-pixel` case が「docked `test_editor` の画面合成画素に
`EDITOR_COLOR` 緑が出るか」を正直に判定すること、および env 記録・
catalog 整合・headless 非回帰を保証する。DComp 未解消の `Fail` を
regression として固定することが目的であり、Fail を減らすことでは
ない。

## Feature

Real-window plugin dock pixel regression（rf631.plugin-dock-real
scenario の `real-dcomp-pixel` case）。

## Rules

- Rule: open 経路は `real-editor-attach` と同一（NativeSlot →
  `lease_parent` → `ParentSlotLease` → `InProcessEditorOps` redeem
  → `session.open(Docked)`）。open 拒否は `Unsupported`、setup
  失敗は `Fail`/`NotRun`（artifact 不在は `NotRun`）。
- Rule: 画素 oracle の本命は screen DC `BitBlt` による child HWND
  画面矩形の採取（表示中の合成結果）。`PrintWindow(child)` は
  補助診断（`note`）で verdict gate にしない。
- Rule: green 判定は `g >= 200 && r <= 80 && b <= 80` の画素割合
  `>= 0.5` を可視とする。lifecycle（attach contract・close・
  detach・session 収束）全 pass かつ green なしは `Fail`、green
  ありは `Pass`、pixel 評価不能（iconic/非表示/採取失敗）は
  `NotRun`。
- Rule: `GPUI_DISABLE_DIRECT_COMPOSITION` 生値が real lane 全
  report の `environment["gpui_disable_direct_composition"]`
  に記録される（未設定は `null`）。
- Rule: `REAL_CATALOG` と `real_entries` の case/scenario/
  required が一致し、crate 内 test が drift を検出する。
- Rule: `--pixel-artifacts` 未指定では PNG を書かない。書込失敗は
  `note` で verdict に影響しない。
- Rule: headless lane の verdict 表・console exit 0/1/2・
  scenario contract・driver boundary は非回帰。

## Examples

- Example: baseline 実機（DComp 有効）で lifecycle pass・screen
  画素 green_fraction ≈ 0 → verdict `Fail`（既知 DComp 未解消の
  regression）。JSONL `environment` に
  `"gpui_disable_direct_composition": null`。
- Example: `GPUI_DISABLE_DIRECT_COMPOSITION=1` 実機で screen
  画素 green_fraction ≥ 0.5 → verdict `Pass`（A' が画素問題を
  迂回した証拠）。`environment` に `"1"` が記録される。
- Example: env-on で process が exit 127 → lane の stderr/exit
  code を捕捉し H1-H5 に分類して記録（verdict 昇格なし）。
- Example: headless host → 全 case `NotRun` +
  `observed.environment_blocked`、exit 0（fail-closed）。

## Guarantees

- Pass/Fail/NotRun/Unsupported が実測のみに基づく。headless/fake
  の結果を実 Window・実 OS の Pass に昇格しない。
- GPUI/Win32 型が公開 API・JSONL schema に出ない（HWND は u64）。
- `attempt_real_attach` refactor が `real-editor-attach` の
  verdict 経路を変えない。

## Non-goals

- 方式採否・本 fix・Focus/Resize/ReDock/入力の実走・macOS/Linux・
  `*-msrv` lane。
- `PrintWindow` 結果の verdict gate 化（補助診断に限定）。
- PNG artifact の canonical lane への常設（opt-in flag のみ）。

## Risk

- 画素 oracle が環境依存（cursor/他 window/最小化）で偽 Fail を
  出しうる → iconic/非表示は `not_run` 退避、limitation を
  evidence/文書に明記。
- env-on の exit-127 が lane を落とす → 観測結果として捕捉・記録
  し、成功条件を弱めない。
