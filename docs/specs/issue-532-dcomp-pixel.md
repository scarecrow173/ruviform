---
id: "SPEC-034YeZkAuQGMGoBAAKoHTL"
type: "spec"
status: "approved"
title: "Issue #532 DComp 画素 regression 固定 slice（test_editor docked 画素 case + GPUI_DISABLE_DIRECT_COMPOSITION 実機観測）"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-532-dcomp-pixel.md"
    modified: []
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
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/specs/issue-631-real-window-dock.md"
    - "docs/designs/issue-631-real-window-dock.md"
    - "docs/specs/issue-631-real-editor-attach.md"
    - "evidence/rf532/gpui-public-surface-audit.md"
    - "evidence/rf388-native-dock-windowing/report.md"
    - "evidence/rf631/verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
  derived-by:
    - "docs/designs/issue-532-dcomp-pixel.md"
    - "docs/plans/issue-532-dcomp-pixel.md"
    - "docs/test-specs/issue-532-dcomp-pixel.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-dcomp-pixel.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 DComp 画素 regression 固定 slice（test_editor docked 画素 case + GPUI_DISABLE_DIRECT_COMPOSITION 実機観測）

## What Should Be Built

Issue #532 の**再現固定 slice**。`rf631.plugin-dock-real` scenario に
docked 状態の `test_editor` 実 child HWND の**画面合成画素**を採取し、
`EDITOR_COLOR`（緑 `0x00_00_FF_00`）の出現を正直な
Pass/Fail/NotRun/Unsupported で判定する新 case（`real-dcomp-pixel`）を
`crates/ruviform-plugin-dock-test` に追加する。既知の GPUI 0.2.2 DComp
topmost 制約が未解消の間、この case は `Fail` を返すことが期待される
regression であり、緩和・`NotRun` 化・期待 Fail の黙殺はしない。

さらに同一 opt-in lane `test-ui-plugin-dock-real` を
`GPUI_DISABLE_DIRECT_COMPOSITION`（方式 A'、audit 確定の受理値
`"1"`/`"true"`）env var 付きで実機実走し、baseline（env なし）と
比較した観測 — (a) 画素復帰、(b) exit-127（audit PENDING の仮説
H1-H5 の実 OS 切り分け）、(c) その他の変化なし — を evidence
（`evidence/rf631/` の JSONL・`verification.md`・pixel artifact）に
正直に記録する。

## Why Now

Issue #532 の方式 A/A'/B/C 採否は、実機での再現固定と方式 A' の
観測結果を入力に後続 slice で決める。本 slice はその入力を作る:
regression case が無いままでは「修正されたか」を機械的に判定できず、
A' 経路の exit-127 はコードパス解析では一意に特定できない
（`evidence/rf532/gpui-public-surface-audit.md` 監査項目 7 —
PENDING のまま）。`test_editor` 実 `IPlugView` child HWND
（PR #668/#670 merge 済み）と real-window lane（PR #660/#670
merge 済み）が揃った今が、画素 oracle を固定する最小 slice。

## Users and Value

- Issue #532/#388/#262 の Owner: DComp 画素問題の再現が regression
  case として固定され、方式 A' の実機観測（画素復帰/exit-127/変化
  なし）が証拠として残る。後続 slice の方式採否・本 fix の合否
  判定に使える。
- Ruviform の後続実装者: `real-dcomp-pixel` case が方式採用後の
  Pass 昇格を機械的に検証する oracle になる。
- Synvelis #176 の Owner: 実 Native Dock の OS 受入に必要な
  Ruviform 側前提（DComp 画素の未解消事実）を最新 SHA で確認
  できる。

## Scope

### In Scope

- `crates/ruviform-plugin-dock-test` への `real-dcomp-pixel` case
  追加（`real_dock.rs` executable entry + `real_catalog.rs`
  `REAL_CATALOG` 行の両更新、crate 内 catalog↔entries 一致 test の
  green 維持）。
- docked 状態の `test_editor` 実 child HWND 矩形の画面合成画素
  （screen `BitBlt`）を主 oracle、`PrintWindow(child)` を補助診断
  として採取し、`EDITOR_COLOR` 緑の出現を判定する。
- `GPUI_DISABLE_DIRECT_COMPOSITION` env var の有無を run ごとの
  evidence（case `environment`）に記録する小さな code 追加。
- `windows-sys` optional edge（`real-dock` feature）への
  `Win32_Graphics_Gdi` feature 追加（既存 edge 内の最小拡張）。
- baseline（env なし）と env-on の両実走の記録:
  `evidence/rf631/` の別 JSONL・`verification.md` 節追記・得られた
  pixel artifact（PNG）の保存。
- `test-ui-plugin-dock-real` lane の exit code 1（新 case の正直な
  `Fail` 由来）を既知 DComp 未解消の期待される信号として記録し、
  緩和しない。
- 文書チェーン（spec/design/plan/task/test-spec/impl-ir/impl-exp）
  と `docs/discovery/issue-532-dcomp-pixel.md` の作成。
- 触った `docs/<type>/` dir の index 再生成は Coordinator 統合時に
  行う（本 slice では生成しない — PR 本文に明記）。

### Out of Scope

- 方式 A/A'/B/C の採否判定・本 fix（`ruviform-ui-gpui` adapter の
  合成経路変更・`[patch]`/fork・env var の製品経路化）。
- Focus/Resize/ReDock/入力の実 Windows 実走（Issue 受入の修正後
  Pass 部分 — 後続 slice）。
- `mise.toml` の変更（lane 追加・task 化・stale 化する lane
  コメントの修正を含む — lease 外。env var 実走は canonical task
  呼出時の env 前置で行い、task 直上コメントの stale 化は PR 本文
  「未対応の作業」に明記して Coordinator に渡す）。
- `docs/**/README.md` index の再生成・手編集（Coordinator が統合
  時に再生成）。
- `evidence/rf532/` の変更（参照のみ）。`Cargo.lock`・ルート
  `Cargo.toml`・`.github/**`・`POLICIES.md`・`AGENTS.md`・他
  crate（`ruviform-plugin-test-vst3`・`ruviform-ui-gpui`・
  `ruviform-ui-test` 等）の変更。
- merge-SHA CI 証拠・macOS/Linux L1 対応付け・consumer 通知・
  Issue auto-close・`*-msrv` lane・3OS 対応。

## Requirements

- R1: `real-dcomp-pixel` case は `real-editor-attach` と同じ open
  経路（NativeSlot → `lease_parent`（real container HWND）→
  `ParentSlotLease` → `InProcessEditorOps` redeem →
  `session.open(Docked)`）で `test_editor` の実 child HWND を
  docked 状態にし、OS 観測（`EnumChildWindows`/`GetWindowRect`）で
  child HWND と画面矩形を取得する。
- R2: 画素 oracle は**画面上の合成結果**（screen DC への `BitBlt`
  による child HWND 画面矩形の採取）を本命とし、`PrintWindow(child)`
  は「child 自身の raster」の補助診断として区別して記録する。
  観測手法の limitation（PrintWindow は画面合成結果ではない、
  screen BitBlt は表示中の画素であり cursor/他 window の重なりを
  含みうる、等）を evidence/文書に明記する。
- R3: `EDITOR_COLOR`（`0x00_00_FF_00` = RGB(0,255,0)）に一致する
  画素の割合を child ごとに記録し、閾値判定で
  Pass/Fail/NotRun/Unsupported を正直に返す。DComp topmost 未解消
  の下では `Fail` が期待される verdict であり、Fail を Pass に
  昇格させない・`NotRun` に偽装しない。
- R4: `GPUI_DISABLE_DIRECT_COMPOSITION` の有無（生の env 値）を
  run ごとの case `environment` に記録し、baseline/env-on の
  JSONL が env 差分を自己記述する。canonical `plugin-dock-
  real-scenario.jsonl` を env-on run で上書きしない（別ファイル
  または run 識別可能な記録）。
- R5: env-on run で exit-127 が観測された場合、stderr/exit code を
  捕捉して記録し、audit 仮説 H1-H5 のどれに整合するかを正直に
  分類する（未確定なら未確定のまま）。再試行で取り繕わない。
- R6: headless lane（`test-ui-plugin-dock`）の verdict 表・
  `test-ui-plugin-dock-console` の exit 0/1/2 意味論・
  `test-ui-scenario-contract`・`check-ui-driver-boundary` を
  非回帰に保つ。`--list --include-real` に新行が出るのは catalog
  投影の正当な差分。
- R7: GPUI/Win32 型を本 crate の公開 API・JSONL evidence schema
  に漏らさない（HWND は既存どおり u64/整数）。`unsafe` は COM/FFI
  境界の `#[expect(unsafe_code)]` + SAFETY コメントの局所許可
  のみ。公開 enum 追加時は `#[non_exhaustive]`。
- R8: `real_dock.rs` 冒頭の case 表・`real_catalog.rs`・bin doc・
  crate README の「editor pixels は対象外」系記述を本 slice の
  追加に整合させて更新する（`mise.toml` は lease 外のため触ら
  ず、stale 化を PR 本文に明記）。

## Acceptance Criteria

- [ ] `real-dcomp-pixel` case が `rf631.plugin-dock-real` に追加
  され、`real_catalog::REAL_CATALOG` と `real_dock::real_entries`
  が一致し crate 内 test が green。
- [ ] 新 case が docked `test_editor` child HWND の画面矩形画素を
  採取し、`EDITOR_COLOR` 緑の出現を Pass/Fail/NotRun/
  Unsupported で正直判定する。既知 DComp 未解消の実機では `Fail`
  が記録される。
- [ ] `GPUI_DISABLE_DIRECT_COMPOSITION` env var 有無が run ごとの
  JSONL `environment` に記録される。
- [ ] baseline（env なし）と env-on（`"1"`）の両実走が
  `test-ui-plugin-dock-real` lane で記録され、exit code・
  stderr・JSONL verdict が証拠として `evidence/rf631/` と
  `verification.md` に残る。exit-127 観測の有無と H1-H5 分類が
  記録される。
- [ ] `mise run fmt-check`・crate の check/lint/test
  （default + `real-dock` feature）・`test-ui-plugin-dock`・
  `test-ui-plugin-dock-console`・`test-ui-scenario-contract`・
  `check-ui-driver-boundary`・`prek run --all-files` が green
  （env-on run の lane exit 1 は既知 Fail の正直な信号として
  区別して記録）。
- [ ] 文書チェーンと `docs/discovery/issue-532-dcomp-pixel.md`
  が作成され、front matter relations が相互解決可能。
- [ ] 本 slice だけでは Issue #532 を close しない
  （`Refs #532` のみ）。

## Deferred Design Concerns

- 方式 A/A'/B/C の採否と本 fix | 本 slice の観測結果が前提 |
  後続 slice | 観測なしに採否すると exit-127 未解明のまま実装へ
  入るリスク。
- Focus/Resize/ReDock/入力の実 Windows 実走 | 方式採用・本 fix
  後 | Issue 受入の修正後 Pass 部分 | 未検証のまま Pass を主張
  するリスク。
- `mise.toml` L153-170 lane コメント「editor-pixel question stays
  deferred」・`real_dock.rs` 冒頭表の stale 化 | lease 外 | Coordinator
  統合時 | コメントと実装の乖離が残る（PR 本文に明記済み）。

## Sources and Relations

- Issue `scarecrow173/ruviform#532`（本 slice 対象 — OPEN 継続、
  auto-close しない）。関連: `scarecrow173/ruviform#388`・
  `scarecrow173/ruviform#630`・`scarecrow173/ruviform#631`・
  `scarecrow173/synvelis#176`（いずれも inline code 参照）。
- `docs/discovery/issue-532-gpui-dcomp-surface.md` F5（exit-127
  仮説 H1-H5）・`evidence/rf532/gpui-public-surface-audit.md`
  （env var 意味論・patch 面積・exit-127 PENDING）。
- `docs/designs/issue-532-gpui-dcomp-surface.md`（A' 比較表）・
  `docs/designs/issue-532-dcomp-native-dock.md`（方式 A/B/C
  採否条件）。
- `evidence/rf388-native-dock-windowing/report.md`（DComp topmost
  覆い隠し実測・window_pixel/printwindow_pixel/sibling probe・
  画素診断手法）。
- `docs/specs/issue-631-real-window-dock.md`・
  `docs/specs/issue-631-real-editor-attach.md`（lane・case の
  前提 slice）。
- `POLICIES.md`（検証レベル・Pass/Fail/NotRun/Unsupported
  規範・fail-closed）。
