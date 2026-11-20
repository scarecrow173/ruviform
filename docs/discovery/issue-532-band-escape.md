---
id: "DISC-034ZaaBandEscapeProbe01"
type: "discovery"
status: "approved"
title: "Issue #532 band-escape probe — 調査記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-532-band-escape.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-532-uncovered-baseline.md"
    - "docs/discovery/issue-532-presentation-probe.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-band-escape.md"
    - "docs/designs/issue-532-band-escape.md"
    - "docs/test-specs/issue-532-band-escape.md"
    - "evidence/rf532/band-escape-results.md"
    - "evidence/rf532/uncovered-baseline-bisect.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-band-escape.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 band-escape probe — 調査記録

## 調査目的

Issue #532（GPUI 0.2.2 DirectComposition/presentation-plane
composition 下で native child HWND plugin の pixel が最終画面に
現れない）に対し、**container の presentation 関係を変えることで
child pixel が「band」を脱出するか**を実走で確定する。前 slice
（uncovered-baseline）までで compositor-internal/presentation 層
の遮蔽が確定したため、残る実装 direction は次の 3 択:

1. minimal layered-container change — container に
   `WS_EX_LAYERED` + `LWA_ALPHA(255)` を適用し、layered surface 経由で
   child pixel を脱出させる。
2. reparent / owned-popup redesign — container を
   `GetDesktopWindow()` 配下に reparent し、GPUI の presentation
   plane の外に出す。
3. 上記双方が不可なら capture-plane / child-level 診断へ escalate。

本 slice はこの選定のための実験 slice であり、修正でも方式の
決裁でもない。

## 前提となる確定事実（前 slice の蓄積）

- Option-A fork（`CreateTargetForHwnd(hwnd, false)`）でも
  `real-dcomp-pixel` は `fail` のまま（`option-a-results.md`）。
- child editor の `PrintWindow` capture は green fraction 1.000 —
  child は rasterize できる。
- `WindowFromPoint` は topmost 変異後に editor child を返す一方、
  screen capture は green 0.000 のまま。
- uncovered-baseline は「遮蔽なし・settle 200/500/1200ms 後でも
  screen green 0.0（child/window DC raster には green があり得る）」
  を確定 — pre-paint/unsettled frame 説は棄却済み。

## 本 slice の実験設計（確定したもの）

- mutation は `osmut` の container mutation family に集約 —
  `ContainerSnapshot`（parent/exstyle/**class_style**/rect/z-order/
  liveness）→ `repaint_container` → `apply_layered` →
  `reparent_to_desktop` → `restore_container`（verified）。
- `exstyle_write` は **read-back 検証付き**: last-error 成功でも
  `GetWindowLongPtrW` で要求値が確認できなければ
  `applied: false`（silent drop を成功と誤記しない）。
- `class_style`（`GCL_STYLE`）を snapshot に追加したのは、
  `WS_EX_LAYERED` 不保持の原因判別（`CS_OWNDC`/`CS_CLASSDC`
  禁止条件 vs silent drop）のため。
- verdict は「probe 実行 + restore 検証 + lifecycle 収束」で
  判定し、green/hit-test は note 級 data（既存 semantics と同一）。

## 実走結果（2026-11-20、interactive Windows x86_64）

詳細は `evidence/rf532/band-escape-results.md`。要点:

- **layered 経路 = 評価不能（refused）**:
  `SetWindowLongPtrW(GWL_EXSTYLE,|WS_EX_LAYERED)` は last-error
  上成功だが verified read-back で bit absent（silent drop）、
  `SetLayeredWindowAttributes` は error 87 で拒否。container の
  `class_style` = `0x4088`（`CS_GLOBALCLASS|CS_PARENTDC|
  CS_DBLCLKS`）で `CS_OWNDC`/`CS_CLASSDC` は非該当。container は
  caller と異なる thread の child window — cross-thread な
  exstyle write の無効化が最有力候補（機構未確定）。
- **reparent 経路 = 実行されたが pixel 欠損は不変**:
  `SetParent(desktop)` 成功、`HWND_TOP` で baseline rect に再配置、
  hit-test 5 点全て `editor_child`。それでも screen green 0.000、
  capture SHA256 は baseline/repaint/layered/reparent/restore の
  全 stage で byte 同一（`7dd8975d…`）。
- **restore = pass（verified）**: parent/exstyle/rect 全て復帰
  確認。case verdict は `fail`（必須 mutation の拒否は strict —
  probe が設計どおり完走しなかった事実の記録）。

## 確定した判別

- 「reparent / owned-popup（既存 container の desktop reparent）」
  経路は否 — container を top-level にしても child raster は
  最終合成に現れない。阻害は container の presentation plane や
  z-order/parentage ではなく、**child raster が composition
  tree に到達しない層**にある。
- 「minimal layered-container」経路は caller thread からは
  適用不可 — 評価には owning thread 側（GPUI window 生成経路 /
  fork 内）での style 適用が必要であり、それは既に minimal で
  はない。
- direction は **capture-plane / child-level 診断への escalate**
  が妥当: child HWND 自身の style/exstyle（`WS_EX_NOREDIRECTIONBITMAP`
  等）・redirect surface の DWM 供給有無・GPUI editor-host
  class（`0x4088`、`CS_PARENTDC` 含む）が child paint を抑制するか
  の切り分け。

## 残課題（次 slice へ引き継ぐ）

- child HWND の `GWL_STYLE`/`GWL_EXSTYLE`/redirect surface 状態の
  直接観測（child-level inventory）。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` が fork で DComp を実際に
  off にするかの検証（過去 run は env 有効性未検証のまま）。
- reparent 後 top-level container で `WS_EX_LAYERED` が latch
  するかの未試行 — 本 slice の spec 順序（layered→reparent 分離）
  上は含まないが、cross-thread drop の原因切り分けに有用。
