---
id: "TSPEC-034YnCis5QOFWOXGztvMIg"
type: "test-spec"
status: "approved"
title: "Issue #532 dcomp-occlusion test spec"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-occlusion.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-532-dcomp-occlusion.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/designs/issue-532-dcomp-occlusion.md"
    - "docs/test-specs/issue-532-dcomp-pixel.md"
    - "docs/plans/issue-532-dcomp-occlusion.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-532-dcomp-occlusion.md"
    - "docs/specs/issue-532-dcomp-occlusion.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-532-dcomp-occlusion.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 dcomp-occlusion test spec

## Purpose

`real-dcomp-pixel` に追加する query-only 遮蔽層診断の
「観測は正直・verdict は不変・query-only 境界を破らない」ことを
保証する。Issue #532 の遮蔽層二分（HWND 層 vs 合成器層）の根拠
となる証拠が改変・取り繕いなく記録されることを pin する。

## Feature

```gherkin
Feature: real-dcomp-pixel の遮蔽層診断
  docked test_editor child HWND について、GPUI root HWND の
  extended style、root/parent の sibling z-order、
  WindowFromPoint の代表点 probe を観測し、note 級の証拠として
  JSONL に記録する。これらの観測は screen-pixel oracle の
  verdict に一切影響しない。
```

## Rules

- Rule: 診断は query-only — window の作成・破棄・移動・属性
  変更・z-order 変更・region 設定・メッセージ送信を行わない
  （`GetAncestor`/`GetWindowLongPtrW`/`GetWindow`/`GetParent`/
  `GetClassNameW`/`GetWindowRect`/`IsWindow`/`IsWindowVisible`/
  `WindowFromPoint` のみ）。
- Rule: 診断 step row は `verdict: "note"` — `PixelScan`、
  `PixelObservation`、case verdict、exit code に影響しない。
- Rule: HWND は `u64` の plain 値 — Win32 型・GPUI 型を
  evidence schema に出さない（driver-neutral 境界）。
- Rule: root exstyle の `noredirectionbitmap` 値は baseline と
  env-on（`GPUI_DISABLE_DIRECT_COMPOSITION=1`）の両 run で
  記録される。
- Rule: z-order 走査と `WindowFromPoint` 結果は case/run ごとに
  記録され、role 分類（`editor_child`/`container`/`root`/
  `other`/`none`）と `above_*` 判定が付く。
- Rule: 診断失敗は note または正直な unevaluable 観測として
  記録され、Pass/Fail を捏造しない。
- Rule: 非 Windows build は stub で compile し、観測は
  Unsupported/NotRun — 捏造した観測値を出さない。
- Rule: `real-editor-attach` の既存挙動・verdict は不変。

## Examples

- Example: baseline run で `noredirectionbitmap=true` が観測される
  （GPUI DComp 経路が有効 — audit 予測と一致）。
- Example: env-on run で `noredirectionbitmap=false` が観測される
  （env var が GPUI 初期化に実効 — `WS_EX_NOREDIRECTIONBITMAP` は
  `!disable_direct_composition` のときのみ付与）。
- Example: `WindowFromPoint` が child rect 内の全代表点で
  `editor_child` を返し screen green = 0.0 の場合、遮蔽は
  合成器レベル（DComp visual / flip-model plane）の傍証 —
  ただし `WindowFromPoint` の GPU 合成下での決定性は limitation
  として記録する。
- Example: `WindowFromPoint` が `root`/`container`/`other` を返す
  場合、WM 級遮蔽 — `occlusion_zorder` 行で犯人 sibling を特定
  できる。

## Guarantees

- 診断は既存 `real-dcomp-pixel` の screen-pixel oracle と
  lifecycle verdict を authoritative に保つ — 診断の成否・内容が
  verdict を変えない。
- baseline/env-on の両 run で同じ診断 step row が出力され、
  exstyle 差分が env var 実効の証拠になる。
- 全ての観測値は plain JSON 値（整数・文字列・真偽・null）で
  evidence に記録される。

## Non-goals

- 遮蔽の根本原因の確定・修正（方式 A/B/C の採否は後続 slice）。
- `WindowFromPoint` 以外の hit-test 系・変異系 probe・
  `SetWindowRgn`/`SetWindowPos` 実験。
- macOS/Linux での同等診断（`NativePluginDock` は Windows-only）。
- docs index の手編集（`doc-maintenance` 管理スクリプトで再生成）。

## Risk

診断が verdict に干渉したり query-only 境界を破ったりすると、
Issue #532 の根因二分の証拠が信頼できなくなる。観測の取り繕いや
「child が返る = 合成器レベル確定」のような過剰解釈は、後続の
方式採否を誤導する。
