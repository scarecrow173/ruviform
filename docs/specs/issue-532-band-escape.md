---
id: "SPEC-034ZaaBandEscapeProbe001"
type: "spec"
status: "approved"
title: "Issue #532 band-escape probe slice（container の layered/reparent presentation 変異で child pixel 脱出可否を判別）"
created: "2026-11-20"
updated: "2026-11-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-532-band-escape.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/osmut.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/bin/plugin-dock-real-scenario.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/plugin_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-uncovered-baseline.md"
    - "docs/specs/issue-532-presentation-probe.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "evidence/rf532/band-escape-results.md"
    - "evidence/rf532/uncovered-baseline-bisect.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-uncovered-baseline.md"
  derived-by:
    - "docs/designs/issue-532-band-escape.md"
    - "docs/discovery/issue-532-band-escape.md"
    - "docs/test-specs/issue-532-band-escape.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-band-escape.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 band-escape probe slice（container の presentation 変異で child pixel 脱出可否を判別）

## 背景と目的

`issue-532-uncovered-baseline` slice により、遮蔽なし・settle
済み環境でも screen green が 0.0 のままであることが確定し、
阻害は compositor-internal/presentation 層にあると絞られた。
残る direction の選定には「container の presentation 関係を
変えれば child pixel が band を脱出するか」の実走判別が必要。

本 slice は実験 slice であり、修正でも方式採否の決裁でもない。
**Issue #532 は open のまま** — probe の green は観測であって
修正完了ではない。

## スコープ

### In Scope

- `osmut.rs` に container-targeted mutation family を追加:
  - `ContainerSnapshot`（`supported`/`container`/`alive`/`parent`/
    `exstyle`/`class_style`/`rect`/`above_count`/`above_hwnd`/
    `below_hwnd`/`truncated`）と `observe_container`。
  - `repaint_container` — `RedrawWindow(NULL,NULL,
    RDW_INVALIDATE|RDW_ERASE|RDW_UPDATENOW)`。
  - `apply_layered` — `GWL_EXSTYLE |= WS_EX_LAYERED` →
    `SWP_FRAMECHANGED` → `SetLayeredWindowAttributes(0,255,LWA_ALPHA)`
    の 3 op を順序どおり返す。
  - `reparent_to_desktop` — baseline exstyle 復帰 → frame apply →
    `SetParent(GetDesktopWindow())` → baseline screen rect を
    `HWND_TOP` で再配置の 4 op。
  - `restore_container` — baseline parent への reparent →
    baseline exstyle → frame apply → `ScreenToClient` 変換後の
    baseline rect 復帰 → best-effort sibling z-order →
    parent/exstyle/rect の verified 判定（`parent_restored`/
    `exstyle_restored`/`rect_restored`）。
  - `ContainerMutationOutcome`/`ContainerRestoreOutcome` 型。
  - `exstyle_write` の read-back 検証（last-error 成功でも
    read-back で要求値未確認なら `applied: false`）。
  - 非 Windows stub（`applied: false`/`supported: false`、
    捏造なし）。
- `real_dock.rs` に新 case `real-dcomp-bandescape` を追加
  （`real-dcomp-uncovered` の直後、`real_entries` 24 要素、
  required = `NativePluginDock`+`VisualCapture`、
  `BE_LABELS` step/artifact prefix）。
- `real_catalog.rs` に catalog row を追加（24 cases）。
- count/list 更新: `tests/console.rs`（総行数 33・selector
  一覧）、`tests/plugin_dock.rs`（blocked 行数 24）、
  `README.md`（case 行 + twenty-four 記述）、`lib.rs`（real-dock
  case 一覧）、`plugin-dock-real-scenario.rs`
  （`--pixel-artifacts` 記述）、`Cargo.toml`（windows-sys ledger
  に `GetClassLongPtrW` 等の利用 API を追記 — 依存・feature
  追加なし）。
- canonical `mise run test-ui-plugin-dock-real` の interactive
  実走 + `--pixel-artifacts` 付き補助 run の記録。
- `evidence/rf532/band-escape-results.md`（fix-decision table
  付き判別記録）。
- doc chain: `docs/discovery|designs|specs|test-specs/
  issue-532-band-escape.md`（本 repo の確立した 4 doc 型 —
  `docs/plans`/`docs/tasks` という doc category は存在しない）。

### Out of Scope

- Issue #532 の閉塞（`Refs #532` のみ、auto-close 禁止）。
- `real-dcomp-pixel`/`real-dcomp-foreground`/`real-dcomp-present`/
  `real-dcomp-uncovered` の step 名・意味論の変更。
- `osoccl`/`ospix`/`ospres` への mutation 配置。
- public/driver-neutral 境界への Win32 型の露出（HWND は `u64`）。
- production 依存・shipping code・CI 必須 lane 化。
- GPUI fork 本体への変更（layered 適用を owning thread 側で行う
  Option-B 系変更は本 slice 外）。
- green 達成の捏造、restore 成功の捏造、headless での実走偽装。

## 要件

### 機能要件

- FR1: `real-dcomp-bandescape` は editor attach lifecycle
  （`prepare_editor_host` → `attempt_real_attach` → probe →
  `conclude_editor_session`）を既存 dcomp case と共有する。
- FR2: probe は baseline 観測 → forced repaint → layered 経路
  試行 → reparent 経路試行 → verified restore の順で実行し、
  各 stage で hit-test（5 点）・screen green・capture hash を
  note 級 data として記録する。
- FR3: 全 mutation op は api 名・applied・win32_error・post-call
  snapshot を `ContainerMutationOutcome` として JSONL に残す。
- FR4: `restore_container` は parent/exstyle/rect を deterministic
  に検証し、sibling z-order は記録のみ（exact を主張しない）。
- FR5: 必須 mutation の拒否は `fail`（probe 未完走）。green/
  hit-test は verdict を決めない。

### 非機能要件

- NFR1: mutation は bounded・test-only・非リアルタイムで、常に
  restore が後続する。
- NFR2: `osmut` 以外の query-only module を変異に使わない。
- NFR3: 非 Windows target で compile 可能、stub は unsupported
  を正直に返す。
- NFR4: `windows-sys` 0.61 の既存 feature 集合内で完結
  （`Win32_Graphics_Gdi`/`Win32_UI_WindowsAndMessaging`）、
  新規依存・feature なし。

## 受入基準

- AC1: `mise run test-ui-plugin-dock-real` で 24 cases が実走し、
  `real-dcomp-bandescape` が staged mutation/observation/restore
  を JSONL に記録する。
- AC2: `be_apply_layered_*`/`be_apply_reparent_*`/`be_restore` の
  各 op が api/applied/win32_error/after を持つ。
- AC3: restore で `parent_restored`/`exstyle_restored`/
  `rect_restored` が true。
- AC4: `--blocked-probe`（headless）で `real-dcomp-bandescape`
  が `not_run`/`environment_blocked` を返し session 情報を捏造
  しない。
- AC5: `tests/console.rs` の catalog 行数 = 33、`tests/
  plugin_dock.rs` の blocked 行数 = 24。
- AC6: fmt-check・check・lint・test（default + `real-dock`）・
  `test-ui-plugin-dock*`・`test-ui-scenario-contract`・
  `check-ui-driver-boundary` が pass。
- AC7: evidence は verdict と observation を分離し、green
  達成/未達を Issue #532 の open/closed と混同しない。
