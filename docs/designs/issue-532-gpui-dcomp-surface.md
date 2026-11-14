---
id: "DESIGN-034XAYbSgNZPA0pCsrkqfy"
type: "design"
status: "approved"
title: "Issue #532 GPUI DComp 監査 — 方式A/B/C 技術入力設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-532-gpui-dcomp-surface.md"
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
  implements:
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
    - "docs/discovery/issue-532-dcomp-native-dock.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/plans/issue-532-gpui-dcomp-surface.md"
    - "docs/test-specs/issue-532-gpui-dcomp-surface.md"
    - "evidence/rf532/gpui-public-surface-audit.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
  derived-by:
    - "docs/plans/issue-532-gpui-dcomp-surface.md"
    - "docs/test-specs/issue-532-gpui-dcomp-surface.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-gpui-dcomp-surface.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 GPUI DComp 監査 — 方式A/B/C 技術入力設計

## Context

先行監査（`docs/discovery/issue-532-dcomp-native-dock.md`）が DComp
topmost 制約の実機事実と L1 証拠を固定したが、方式 A/B/C の採否に
必要な upstream public-surface の事実が未調査だった。本設計は
vendored gpui 0.2.2 の source 監査結果を、`docs/designs/
issue-532-dcomp-native-dock.md` の方式 A/B/C 比較枠組みへ写像する
構造を定義する。**本設計は技術入力の整理であり、方式の採否・
実装・実機検証を決定しない**。

## Scope

### In Scope

- 監査結果（DComp グラフ・env var 意味論・公開面・patch 面積・
  Option B 空間・version 情勢・exit-127 仮説）の方式 A/B/C への写像。
- env var 経路を「A'（zero-code 迂回）」として比較表へ追加する根拠。
- L2/L3 再開時の調査順序への影響（exit-127 切り分けを Step 1 の
  前段に置く根拠）。

### Out of Scope

- 方式の採否判定・実装・patch/vendor 適用。
- 実機 L2/L3 調査の実施、exit-127 の実 OS 再現。
- `crates/`・CI・Cargo manifest/lockfile の変更。

## Components and Boundaries

- **正本**: `docs/discovery/issue-532-gpui-dcomp-surface.md`（監査
  findings F1-F7）+ `evidence/rf532/gpui-public-surface-audit.md`
  （行番号・コードパスの証拠）。
- **本設計**: 監査事実 → 方式 A/B/C 比較軸への写像のみを担う。
  実機調査手順の正本は `docs/designs/issue-532-dcomp-native-dock.md`
  （変更なし）。
- **境界**: gpui registry source は読み取り専用。ruviform 側の
  `NativeSurfaceSlot` 契約・adapter 境界は変更しない。

## Data and Control Flow

監査事実から方式への写像:

| 監査事実 | 方式への影響 |
| --- | --- |
| `topmost` literal は `directx_renderer.rs:793` の `true` 1 語のみ | A の patch 面積は最小（literal 1 token + fork/vendor 運用）。呼び出し側 2 箇所（初回・device-lost 復旧）に自動適用 |
| gpui DComp/DXGI/renderer は全て `pub(crate)` 以下 | A は公開 API では実現不能 → `[patch]`/fork が必須。B での gpui object 再利用も不能 |
| `GPUI_DISABLE_DIRECT_COMPOSITION`（`"true"`/`"1"`、process-wide、起動時 1 回） | **A'（zero-code 迂回）** が存在: 非 DComp 経路は `CreateSwapChainForHwnd` + `WS_EX_NOREDIRECTIONBITMAP` なしの通常 DWM 合成で、child HWND が通常 z-order に復帰する可能性。patch 不要 |
| 非 DComp 経路の swapchain desc 差分（`SCALING_NONE`/`ALPHA_MODE_IGNORE` vs `SCALING_STRETCH`/`PREMULTIPLIED`） | A' 採用時は描画特性が DComp 経路と異なる（present/vsync は同一）。比較軸「CPU/GPU/frame-time」で要計測 |
| gpui 0.2.2 が最新 publishable・zed 現行版 publish 不能 | A の patch base は 0.2.2 のみ。upstream 追従は git dep か fork 継続保守が前提 |
| exit-127 はコードパスから一意に特定不能（仮説 H1-H5） | A' の実現可能性を左右する唯一の既知 blocker。L2/L3 再開時は exit-127 切り分け（stderr/exit code 捕捉の実 OS 再現）を方式比較の前段に置く |
| Option B の実質的空間は OS interposition（env var/`SetWindowRgn`/第二 DComp target） | B は「gpui 内部再利用」ではなく「OS 公開 API による child 合成の interposition」として設計する必要がある。`SetWindowRgn` punch-out と第二 `CreateTargetForHwnd(hwnd,false)` は共に未検証仮説 |

## Non-Functional Constraints

- `NativeSurfaceSlot` 契約・lease/epoch・2-phase re-dock・private
  adapter 境界は不変（`docs/designs/issue-532-dcomp-native-dock.md`
  Step 3 を継承）。
- HWND/GPUI/DComp 型は `ruviform-ui-gpui` private adapter 内に閉じ、
  RT callback・Consumer Domain に漏らさない。
- fake/headless の成功を実 plugin 画素の証拠にしない。方式選択だけ
  では画素 Full PASS としない。

## Risks and Trade-offs

- **A' の exit-127**: env var 経路が scenario 途中で落ちる既知観測が
  未解明。原因が GPUI 非 DComp 経路固有なら A' は使用不能。実 OS
  再現が全ての比較の前提。
- **A' の描画差分**: HWND swapchain は DComp 経路と present 特性が
  異なり（scaling/alpha）、視覚品質・性能の比較が必要。
- **B-region の複雑性**: `SetWindowRgn` は window 全体の region を
  差し替えるため、GPUI 描画内容も dock 領域で clip される。layout
  追従・複数 dock・DPI で region 更新コストと正確性が論点。
- **第二 DComp target の未確認 semantics**: 同一 HWND への複数
  device/target 共存は文書化が薄く、実機検証なしに採用不可。

## Deferred Design Concerns

- 方式 A/B/C（A' 含む）の採否と実装設計 | 実機 L2/L3 調査が前提 |
  Windows L2/L3 利用要件発生時 | 採否未決のまま実装に入るリスク。
- exit-127 の実 OS 再現 | interactive Windows 11 desktop session
  依存 | L2/L3 再開時 Step 1 前段 | A' 経路の可否が不明のまま。

## Verification Notes

- [x] 監査 7 項目が `docs/discovery/issue-532-gpui-dcomp-surface.md`
  と `evidence/rf532/gpui-public-surface-audit.md` に記録された。
- [x] 全ての行番号・コードパスが vendored source で再検証された。
- [x] 実機依存事項が PENDING/未検証として明記された。
- [ ] `doc-maintenance` の `index apply` + `doc-status` で relations
  解決を確認（文書チェーン完成後に実施）。
