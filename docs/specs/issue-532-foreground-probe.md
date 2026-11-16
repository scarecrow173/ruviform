---
id: "SPEC-034YxCZzURRuUuQPn2Wdgk"
type: "spec"
status: "approved"
title: "Issue #532 foreground/z-order 変異 probe slice（compositor-internal vs 外部 HWND stacking の二分）"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-532-foreground-probe.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "file"
        path: "evidence/rf631/verification.md"
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
  depends-on:
    - "docs/specs/issue-532-dcomp-occlusion.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "docs/specs/issue-532-dcomp-native-dock.md"
    - "docs/specs/issue-631-real-window-dock.md"
    - "evidence/rf631/verification.md"
    - "evidence/rf532/gpui-public-surface-audit.md"
  refines:
    - "docs/specs/issue-532-dcomp-occlusion.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-dcomp-occlusion.md"
  derived-by:
    - "docs/designs/issue-532-foreground-probe.md"
    - "docs/plans/issue-532-foreground-probe.md"
    - "docs/test-specs/issue-532-foreground-probe.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-foreground-probe.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 foreground/z-order 変異 probe slice（compositor-internal vs 外部 HWND stacking の二分）

## 背景と目的

前回 slice（`issue-532-dcomp-occlusion`）の query-only 診断で、
docked `test_editor` child rect 内の `WindowFromPoint` が外部
`Chrome_RenderWidgetHostHWND` を返すこと、GPUI 内部 z-order に
遮蔽がないことが確定したが、**遮蔽が compositor 内部か外部
HWND stacking かの二分は未解決**のまま残った。本 slice はその
二分を行う実験 slice であり、修正ではない。

GPUI root HWND の z-order/foreground を段階的に変異させ、各
段階で hit-test・screen 画素を再観測し、変異後に plugin child
の画素が screen composition に現れるかを記録する。これにより
「外部 HWND stacking が遮蔽の全原因」か「compositor 内部
（DComp/flip-model）が依然 child を覆う」かを分離する。

## スコープ

### In Scope

- `ruviform-plugin-dock-test` に新規 real-lane catalog case
  `real-dcomp-foreground` を追加（`REAL_CATALOG` 14 行・
  `real_entries` 14 要素・`real-dcomp-pixel` 直後の位置）。
- 新規 `src/osmut.rs` module — GPUI root の可変 window 状態
  （exstyle/topmost/top-level z-order/foreground）の snapshot・
  変異適用・復元。Windows `windows-sys 0.61` 既存 feature のみ
  使用、非 Windows は `supported:false` stub。
- 変異段階: baseline 観測 → (a) `SetWindowPos(HWND_TOP,
  SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)` → (b)
  `SetForegroundWindow(root)` → (c) `SetWindowPos(HWND_TOPMOST,
  ...)` → restore（NOTOPMOST/位置復元/foreground 復帰試行）→
  復元検証。各段階で `osoccl::probe` hit-test + screen green
  fraction を再観測。
- 変異前の外部遮蔽物（前回は Chrome）の識別情報を baseline
  として記録。
- 状態復元の保証: topmost flag を baseline 値に戻すことを検証
  可能な契約とし、z-order/foreground は可能な範囲で復元を試行
  して観測値を正直に記録（完全復元を装わない）。
- verdict は「probe 実行 + 状態復元 + 共有 lifecycle 契約」の
  honest oracle。screen green 出現の有無は verdict 入力にしない
  （二分の結果データ）。
- baseline + env-on（`GPUI_DISABLE_DIRECT_COMPOSITION=1`）の
  interactive session 実走記録、遮蔽層の二分結論を
  `evidence/rf631/verification.md` と `evidence/rf532/` 結論
  文書に記録。

### Out of Scope

- Issue `#532` の解決・方式採否（A patch/B-region/B-dcomp2/C
  viability の最終判断は本 slice の二分結果を入力とする後続
  slice）。
- GPUI root への `PrintWindow` 等の compositor 内部直接観測
  （後続 slice）。
- `AttachThreadInput`/`LockSetForegroundWindow`/
  `SetWindowPlacement` 等の foreground lock 回避手段
  （design で評価・不採用）。
- consumer crate・adapter・GPUI vendor・production compositor
  code・`mise.toml`・docs index の変更。
- Issue `#532` の auto-close、PR merge、`main` への push。

## 要件

### 機能要件

1. `REAL_CATALOG` に `real-dcomp-foreground` 行を追加し、
   `real_entries`/`blocked_reports`/`blocked_probe_violations`/
   console 期待値の件数を 13→14 に整合させる。
2. 新 case は `real-dcomp-pixel` と同じ attach lifecycle
   （`prepare_editor_host` → `attempt_real_attach` →
   `conclude_editor_session`）を通り、attach 中・lease 生存中に
   変異 probe を実行する。
3. `osmut` module は `osoccl`/`ospix` の query-only 契約を侵さず、
   Win32 型を公開境界に出さない（HWND は `u64`）。
4. 各変異段階で適用 API の戻り値・`GetLastError`・適用後の
   `RootSnapshot`・hit-test・screen green fraction を記録する。
5. 復元は `osmut::restore` で topmost flag を baseline 値に戻し、
   検証結果を専用 step row で記録する。復元失敗は case の
   verdict 入力とする。
6. `SetForegroundWindow` の FALSE（foreground lock 拒否）は
   正直に記録し、成功に偽装しない。
7. `real-dcomp-pixel` の authoritative screen-pixel verdict を
   変えない — 変異は新 case 内に閉じ、既存 case の観測順序・
   verdict に影響しない。

### 非機能要件

- `unsafe` は FFI 境界の局所許可のみ（`#[expect(unsafe_code)]` +
  SAFETY）。
- 新規 dependency・feature 追加なし。
- 変異は sub-second の bounded 操作で、復元を必ず試行する
  （error path でも復元を試みる）。
- RT contract: non-RT only。

## 受入基準

- [ ] `real-dcomp-foreground` が catalog/entry/console で
  整合し、件数 assertion が 14 に更新されている。
- [ ] `osmut` が Windows stub/実装を持ち、plain 値のみを返す。
- [ ] baseline + env-on の interactive lane で case が実走し、
  各段階の適用結果・hit-test・screen green・復元結果が
  JSONL に記録される。
- [ ] `real-dcomp-pixel` の verdict が従来どおり `Fail`（既知の
  occlusion）を維持する。
- [ ] `mise run fmt-check`、crate check/lint/test
  （default + `real-dock`）、console/contract/boundary lane、
  `prek run --all-files` が green。
- [ ] 二分結論（外部 stacking vs compositor 内部）が
  `evidence/rf532/` に記録され、未解決部分は正直に
  `unresolved` と明記される。
- [ ] Issue `#532` は open のまま、PR は merge しない。
