---
id: "DISC-fZWC8q6tfUQG3VuH1L9Eoz"
type: "discovery"
status: "approved"
title: "Issue #532 presentation-probe discovery — 残余 3 候補の判別設計探索"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-532-presentation-probe.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-presentation-probe.md"
    - "docs/designs/issue-532-presentation-probe.md"
    - "docs/discovery/issue-532-foreground-probe.md"
    - "docs/discovery/issue-532-dcomp-occlusion.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-presentation-probe.md"
  derived-by:
    - "docs/specs/issue-532-presentation-probe.md"
    - "docs/designs/issue-532-presentation-probe.md"
    - "docs/plans/issue-532-presentation-probe.md"
    - "docs/test-specs/issue-532-presentation-probe.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
---

# Issue #532 presentation-probe discovery — 残余 3 候補の判別設計探索

## 問題の枠組み

foreground-probe 実走（`evidence/rf532/foreground-probe-bisect.md`）
で確定した事実:

- `WindowFromPoint` が自 child HWND を返す stage でも screen
  green は 0 — HWND z-order では説明できない第二の境界。
- DComp off で child window DC が green 1.0、screen/`PrintWindow`
  は 0 —「surface にはあるが合成結果に無い」。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` でも残余は解消しない。

残る候補は 3 系統で、それぞれ判別に必要な観測が異なる:

| 候補 | 判別に必要な観測 |
| --- | --- |
| (a) hit-test-transparent cover | child rect と交差する全 window の列挙（`WindowFromPoint` が skip する window を含む）+ screen 内容の HWND 帰属 |
| (b) child presentation 経路 | child surface → container/root raster への green 伝播の有無 |
| (c) capture 盲点 | screen `BitBlt` 以外の経路（`PrintWindow` flag 変種、特に `PW_RENDERFULLCONTENT`）が green を捉えるか |

## 探索結果（本 slice の入力）

### cover inventory の設計

- `WindowFromPoint` は hidden/disabled/transparent/layered window
  を skip するため、hit-test が child を返しても visual cover が
  残り得る。`EnumWindows` は top-level window を z-order で全列挙
  するため、hit-test が見ない window も拾える。
- foreign top-level の子孫 HWND（例 `Chrome_RenderWidgetHostHWND`）
  は `EnumWindows` に現れない — ただし `WindowFromPoint` が返す
  HWND は fingerprint suspect に直接含める設計でカバーする。
- own-process/foreign の分離は `GetWindowThreadProcessId` の pid
  と `GetCurrentProcessId` の比較で行う（lane process = GPUI
  process）。

### presentation attributes の設計

- `DWMWA_CLOAKED` — DWM が window を compose していない
  （cloaked）状態を記録。`IsWindowVisible` が true でも
  cloaked なら screen に出ない重要なシグナル。
- `GetWindowRgn`/`GetRgnBox` — window に設定された region
  （clip）の bounding box。`NULLREGION` = 全体 clip。
- `GetUpdateRect` — pending invalid region。空なら paint 要求
  無し（正常）。
- `GetLayeredWindowAttributes` — layered window の alpha/
  color-key。hit-test-transparent layered surface の特定に使う。

### capture path variant の設計

- `PrintWindow` の `nflags` 変種（0 / `PW_CLIENTONLY` /
  `PW_RENDERFULLCONTENT` / 両方）を child・container・root に
  適用。`PW_RENDERFULLCONTENT`（Win8.1+）は DComp/flip-model
  content を捕捉し得る経路 — root の DComp swapchain 内容が
  raster に出るかが (b)/(c) の分岐点。
- `capture_window_dc` を container/root にも適用 — child 以外の
  surface が green を保持するかの確認。
- 既存 `capture_print_window`（flags=0x3）は wrapper として
  維持し、`real-dcomp-pixel` の証拠形式を変えない。

### fingerprint の設計

- screen capture（child rect）と suspect raster の overlap 領域で
  量子化 RGB bin（`>>3`、32³）の一致率を記録。
- suspect = `WindowFromPoint` hit HWND（dedup）+ inventory 交差
  window 上位 8。root/container の flags=0 raster も比較対象。
- raster dims と suspect rect の不一致は `size_mismatch` で
  skip — 誤った一致率を出さない。
- screen 領域の上位 3 色 bin を記録し、人間が「screen に何が
  出ているか」を即座に読めるようにする。

### 変異の不採用評価

- `RedrawWindow`/`UpdateWindow` — plugin paint 経路が event
  依存で強制 invalidate が再 paint を保証しない。
- `DwmRegisterThumbnail` — destination HWND が必要で新規 window
  変異になる。観測 slice の範囲外。
- Magnification/WGC — session/window 作成を伴う重い基盤、
  依存追加が必要。`PW_RENDERFULLCONTENT` で代替可能。
- `osmut` 再利用 — foreground slice で z-order/foreground を
  動かしても green は戻らなかった。同じ変異の反復は新情報なし。
- DComp visual tree 走査 — COM 経路が必要で公開面に無い。
  deferred。

### verdict の設計判断

- foreground slice の先例どおり「probe 実行 + lifecycle」を
  verdict とし、観測値（green・hit-test・fingerprint）は
  verdict 入力にしない。
- 変異が無いため「復元」契約は存在しない — `completed`（全
  family が top-level call を完了）で代替する。
- family-level 早期 return（`supported:false` 等の防御経路）
  のみ `Fail`、個別 API 失敗は `error` 記録で probe 継続。

## 残存する不確実性

- `PW_RENDERFULLCONTENT` が GPUI root の DComp content を実際に
  捕捉するかは実装依存 — 実走データが答える。
- `PrintWindow` が foreign window で refuse/空 raster を返す
  可能性 — 記録のまま残し、fingerprint は skip で継続。
- `EnumWindows` は同一 desktop の top-level のみ — 別 desktop/
  secure desktop の cover は原理的に見えない（記録上の制限）。
- fingerprint の量子化一致率は「同じ色」しか見ない — 半透明/
  変化する内容では不安定になり得る（上位色 bin と併記）。
- foreign top-level の子孫 HWND の網羅的列挙は未実装 —
  hit-test HWND を suspect に直接含める設計で一次対応、
  evidence 次第で後続判断。
