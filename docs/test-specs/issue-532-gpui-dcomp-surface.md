---
id: "TSPEC-034XB25A57vEBAhqcyKdIM"
type: "test-spec"
status: "approved"
title: "Issue #532 GPUI DComp public-surface 監査 検証仕様"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-532-gpui-dcomp-surface.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/issue-532-gpui-dcomp-surface.md"
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
    - "docs/plans/issue-532-gpui-dcomp-surface.md"
    - "docs/tasks/issue-532-gpui-dcomp-surface.md"
    - "docs/impl/ir/issue-532-gpui-dcomp-surface.md"
    - "docs/impl/exp/issue-532-gpui-dcomp-surface.md"
    - "evidence/rf532/gpui-public-surface-audit.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 GPUI DComp public-surface 監査 検証仕様

## Purpose

本 spec は docs-only slice の監査成果物が満たすべき保証を固定する。
コード変更・実機検証を伴わないため、検証は「文書・証拠ノートが
要求項目を正確に含むか」「未検証事項が Pass へ偽装されていないか」
に限定される。実機 L2/L3 検証は対象外であり、将来の実機調査時に
本 spec を継承する。

## Feature

`GPUI DComp Public-Surface Audit Documentation` — vendored gpui
0.2.2 の Windows platform 層に関する監査記録が、後続の方式 A/B/C
調査と L2/L3 再開の両方に十分な技術入力を提供することを保証する。

## Rules

- R1: `docs/discovery/issue-532-gpui-dcomp-surface.md` が正本として
  存在し、`CreateTargetForHwnd` の `topmost` 引数の全出現箇所・
  env var の意味論・public/`pub(crate)` 到達可能性・patch 面積・
  Option B 要件・version 情勢・exit-127 仮説を含む。
- R2: `evidence/rf532/gpui-public-surface-audit.md` が監査対象の
  コードパス・行番号・判定根拠を記録し、再検証可能である。
- R3: 先行 `docs/discovery/issue-532-dcomp-native-dock.md` の Open
  Questions が更新され、回答可能分は解消・実 OS 依存分は open の
  まま維持される。status は `resolved` を維持する。
- R4: 文書チェーン（spec/design/plan/task/test-spec/impl-ir/impl-exp）
  の front matter relations が相互に解決可能である。
- R5: exit-127・`topmost=false` 効果・実 plugin 画素は
  「未検証/PENDING」と明記され、Pass/Unsupported への偽装がない。
- R6: 方式 A/B/C（および A' env var）の比較に必要な事実が
  `docs/designs/issue-532-gpui-dcomp-surface.md` の写像表に記録される。

## Examples

- Given vendored `gpui-0.2.2` の `src/platform/windows/` 配下に
  `CreateTargetForHwnd` が複数箇所存在する、When 監査を実施する、
  Then 全サイトと `topmost` 引数が列挙される。
- Given `GPUI_DISABLE_DIRECT_COMPOSITION` が `platform.rs` で
  process-wide に 1 回だけ読まれる、When 非 DComp 経路の動作を
  記録する、Then `CreateSwapChainForHwnd`・`WS_EX_NOREDIRECTIONBITMAP`
  なし・`DXGI_SCALING_NONE`/`ALPHA_MODE_IGNORE` が記述される。
- Given `DirectComposition` が private struct である、When downstream
  からの到達可能性を評価する、Then 「公開 API 経由では到達不能」
  が記録される。

## Guarantees

- 監査記録が実際の vendored source 行番号・コードパスと一致する
  （行番号は本 slice で独立再検証済み）。
- `topmost=false` の効果・exit-127 の原因・実 plugin 画素の可否が
  「仮説/PENDING」として明確に区別される。
- Option A の patch 面積（literal 1 語）が過大評価されず、Option B
  が gpui object 再利用不能を前提に OS interposition として設計
  される。
- `zed-industries/zed#55271` が inline code 形式で引用され、GitHub
  auto-reference を誘発しない。

## Non-goals

- 実画面・実 plugin・実 GUI の L2/L3 試験。
- `topmost=false`・env var・`SetWindowRgn` の実画素 semantics の
  検証。
- exit-127 の実 OS 再現と原因確定。
- 方式 A/B/C の採否判定。
- cargo registry source の編集・gpui patch/vendor 適用。

## Risk

- 監査記録が実際の source と乖離した場合、方式 A/B/C の技術入力が
  誤り、L2/L3 調査の手順設計を誤導する。
- 未検証事項を Pass/Unsupported へ偽装すると、後続の実機調査で
  「確認済み」と誤認されるリスク。
- exit-127 を「解決済み」と記録すると、A' 経路の可否判断を誤る。
