---
id: "SPEC-kIDA2z7xvjXFGeni2wVkwU"
type: "spec"
status: "approved"
title: "Issue #532 presentation-mechanism 判別 probe slice（cover inventory・capture path variant・presentation attributes）"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-532-presentation-probe.md"
      - type: "file"
        path: "docs/designs/issue-532-presentation-probe.md"
      - type: "file"
        path: "docs/plans/issue-532-presentation-probe.md"
      - type: "file"
        path: "docs/tasks/issue-532-presentation-probe.md"
      - type: "file"
        path: "docs/test-specs/issue-532-presentation-probe.md"
      - type: "file"
        path: "docs/discovery/issue-532-presentation-probe.md"
      - type: "file"
        path: "docs/impl/ir/issue-532-presentation-probe.md"
      - type: "file"
        path: "docs/impl/exp/issue-532-presentation-probe.md"
      - type: "file"
        path: "docs/impl/exp/issue-532-presentation-probe-exp.jsonl"
      - type: "file"
        path: "evidence/rf532/presentation-probe.md"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/ospres.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/ospix.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "file"
        path: "evidence/rf631/verification.md"
      - type: "file"
        path: "evidence/rf631/plugin-dock-real-scenario.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-real-scenario-dcomp-disabled.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-real-headless.jsonl"
    deleted: []
    renamed: []
    moved: []
    generated:
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
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-foreground-probe.md"
    - "docs/specs/issue-532-dcomp-occlusion.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-foreground-probe.md"
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "docs/specs/issue-532-dcomp-native-dock.md"
    - "docs/specs/issue-631-real-window-dock.md"
    - "evidence/rf631/verification.md"
    - "evidence/rf532/foreground-probe-bisect.md"
    - "evidence/rf532/gpui-public-surface-audit.md"
  refines:
    - "docs/specs/issue-532-foreground-probe.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-foreground-probe.md"
    - "docs/specs/issue-532-dcomp-occlusion.md"
  derived-by:
    - "docs/designs/issue-532-presentation-probe.md"
    - "docs/plans/issue-532-presentation-probe.md"
    - "docs/test-specs/issue-532-presentation-probe.md"
    - "docs/discovery/issue-532-presentation-probe.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-presentation-probe.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 presentation-mechanism 判別 probe slice（cover inventory・capture path variant・presentation attributes）

## 背景と目的

foreground-probe slice（`issue-532-foreground-probe`）の実走で、
外部 HWND stacking は第一の境界として確定したが、**HWND z-order
では説明できない compositor/presentation 内部の遮蔽**が残った:

- `WindowFromPoint` が自 child HWND（`RuviformTestEditor`）を返す
  stage 以降も screen green は 0 のまま。
- DComp off run では child の window DC が green_fraction 1.0 を
  持つのに screen / `PrintWindow` は 0 — 「surface にはあるが
  合成結果に無い」方向の証拠。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` は残余を解消しないため、
  GPUI root の DComp swapchain 経路だけには帰属しない。

`foreground-probe-bisect.md` が残した未解決候補は 3 系統:

- **(a) hit-test-transparent surface** — `WS_EX_TRANSPARENT`/
  layered/DComp visual が `WindowFromPoint` を透過して視覚的に
  覆ったまま残る可能性（HWND 木に現れない or hit-test が skip
  する window）。
- **(b) child presentation 経路** — child HWND 自身の surface →
  DWM 合成の presentation 経路（plugin 描画・redirection・
  flip-model）の問題。green が child surface にあるのに
  container/root の raster にも screen にも出ない。
- **(c) capture 盲点** — screen capture（`BitBlt` screen DC）
  自体が対象 content を捉えられない可能性。

本 slice はこの 3 候補を判別する**観測専用**の probe slice で
あり、修正でも方式選定でもない。変異は採用しない（design で
評価・不採用を確定する）。

## スコープ

### In Scope

- `ruviform-plugin-dock-test` に新規 real-lane catalog case
  `real-dcomp-present` を追加（`REAL_CATALOG` 15 行・
  `real_entries` 15 要素・`real-dcomp-foreground` 直後の位置）。
- 新規 `src/ospres.rs` module — query-only の presentation
  observation: top-level window inventory（`EnumWindows`、
  z-order・class・rect・visible/alive/disabled・exstyle・owner・
  pid）と presentation attributes（`DWMWA_CLOAKED`・
  `GetWindowRgn`/`GetRgnBox`・`GetUpdateRect`・
  `GetLayeredWindowAttributes`）。Windows `windows-sys 0.61`
  既存 feature + `dwmapi` local extern のみ、非 Windows は
  `supported:false` stub。
- `ospix` に `capture_print_window_flags(hwnd, flags)` を追加 —
  既存 `capture_print_window`（`PW_CLIENTONLY|PW_RENDERFULLCONTENT`）
  の flag 引数化。`PrintWindow` flag 変種
  （0 / `PW_CLIENTONLY` / `PW_RENDERFULLCONTENT` / 両方）を
  child・container・root に適用し、child 既存 3 経路
  （screen/printwindow/windowdc）と対比する。
- cover inventory: child rect と交差する全 top-level window を
  z-order で列挙し、direct child/sibling/root 関係・visibility・
  disabled・styles・rect・pid（own-process vs foreign）を記録。
  `WindowFromPoint` が skip し得る window（transparent/layered/
  cloaked）も列挙対象に含める。
- presentation attributes を child・container・root・cover 候補に
  対して観測する。
- fingerprint: screen capture（child rect）を suspect window の
  `PrintWindow` raster・root/container raster と量子化色一致率で
  対比し、「screen に実際に何が出ているか」を HWND 候補へ帰属
  させる note 級データを記録する。screen green・hit-test・
  fingerprint のいずれも verdict 入力にしない。
- verdict は「probe 実行 + lifecycle」の honest oracle。観測 API
  の致命的失敗で probe 自体が成立しない場合のみ `Fail`、probe
  前提不成立は `NotRun`、非 Windows は `Unsupported`。
- baseline + env-on（`GPUI_DISABLE_DIRECT_COMPOSITION=1`）の
  interactive session 実走記録、3 候補の判別結果を
  `evidence/rf631/verification.md` と
  `evidence/rf532/presentation-probe.md` に記録。

### Out of Scope

- Issue `#532` の解決・方式採否（A patch/B-region/B-dcomp2/C
  viability の最終判断は本 slice の判別結果を入力とする後続
  slice）。
- window 状態の変異（`osmut` の再利用・新規 mutation）— 本
  slice は観測専用。`RedrawWindow`/`UpdateWindow`/
  `DwmRegisterThumbnail`/Magnification/WGC 等の変異・別 capture
  系は design で評価・不採用とし理由を記録する。
- DComp visual tree の直接走査（`DComposition`/`IDComposition*`
  の visual 列挙）— 公開面に無く、後続 slice の候補として記録。
- consumer crate・adapter・GPUI vendor・production compositor
  code・`mise.toml`・docs index の変更。
- Issue `#532` の auto-close、PR merge、`main` への push。

## 要件

### 機能要件

1. `REAL_CATALOG` に `real-dcomp-present` 行を追加し、
   `real_entries`/`blocked_reports`/`blocked_probe_violations`/
   console 期待値の件数を 14→15 に整合させる。
2. 新 case は `real-dcomp-foreground` と同じ attach lifecycle
   （`prepare_editor_host` → `attempt_real_attach` →
   `conclude_editor_session`）を通り、attach 中・lease 生存中に
   presentation probe を実行する。
3. `ospres`/`ospix` 拡張は query-only を維持し、Win32 型を公開
   境界に出さない（HWND は `u64`、plain 値 record のみ）。
   `osmut` には触れない。
4. cover inventory は `EnumWindows` の top-level 列挙に限定し、
   child rect との交差判定・own-process/foreign pid・z-order・
   visibility・disabled・exstyle を記録する。走査は bounded
   （cap + `truncated` 記録）。
5. presentation attributes は `DWMWA_CLOAKED`・window region・
   update region・layered attributes を対象 HWND 毎に記録し、
   query 失敗は `None`+error として縮退させる（捏造しない）。
6. `capture_print_window_flags` は flag をそのまま `PrintWindow`
   の `nflags` に渡し、DIB サイズを flag と rect 種別
   （`PW_CLIENTONLY` 有無で client/window rect）に整合させる。
   flag・backend・dimensions・green stats を step row に記録する。
7. fingerprint は screen capture と suspect raster の overlap 領域
   で量子化 RGB bin の一致率を記録する。比較不能（rect/raster
   寸法不一致・capture 失敗）は `error`/`skipped` として記録し、
   一致率を捏造しない。
8. `real-dcomp-pixel`/`real-dcomp-foreground` の authoritative
   挙動を変えない — 新 case 内に閉じ、既存 case の観測順序・
   verdict に影響しない。

### 非機能要件

- `unsafe` は FFI 境界の局所許可のみ（`#[expect(unsafe_code)]` +
  SAFETY）。
- 新規 dependency・feature 追加なし（`dwmapi` は local extern、
  windows-sys 既存 feature の範囲内）。
- 全 probe は read-only で、変異・状態復元の契約を持たない
  （変異を採用しない design 判断）。
- RT contract: non-RT only。

## 受入基準

- [ ] `real-dcomp-present` が catalog/entry/console で整合し、
  件数 assertion が 15 に更新されている。
- [ ] `ospres` が Windows stub/実装を持ち、plain 値のみを返す。
- [ ] `capture_print_window_flags` が flag 変種を記録し、
  既存 `capture_print_window` の挙動を変えない。
- [ ] baseline + env-on の interactive lane で case が実走し、
  cover inventory・presentation attributes・capture 変種・
  fingerprint が JSONL に記録される。
- [ ] `real-dcomp-pixel`/`real-dcomp-foreground` の verdict が
  従来どおり維持される（`real-dcomp-pixel` は `Fail` のまま）。
- [ ] `mise run fmt-check`、crate check/lint/test
  （default + `real-dock`）、console/contract/boundary lane、
  `prek run --all-files` が green。
- [ ] 3 候補（hit-test-transparent surface / child presentation /
  capture 盲点）の判別結果が `evidence/rf532/` に記録され、
  未解決部分は正直に `unresolved` と明記される。
- [ ] Issue `#532` は open のまま、PR は merge しない。
