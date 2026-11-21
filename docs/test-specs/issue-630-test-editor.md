---
id: "TSPEC-034Y13OOyL62lZ84qd2ILq"
type: "test-spec"
status: "approved"
title: "Issue #630 test_editor slice — test specification"
created: "2026-11-15"
updated: "2026-11-22"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-test-editor.md"
    - "docs/designs/issue-630-test-editor.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-test-editor.md"
  implements:
    - "docs/specs/issue-630-test-editor.md"
    - "docs/designs/issue-630-test-editor.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "docs/test-specs/issue-630-test-fault.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-630-test-editor.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-editor.md"
    - "docs/designs/issue-630-test-editor.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-630-test-editor.md"
    - "docs/designs/issue-630-test-editor.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_editor slice — test specification

## Purpose

`test_editor` class と plugin 側 `IPlugView` が production host
`open_gui` の要求契約（view 生成・platform gate・frame・attach・
remove）を満たすことを、実 ABI（`ComWrapper`/`ComPtr` vtbl
dispatch）と production `Vst3Library` bridge の両経路で固定する。
real parent HWND への attach 成功は本 spec の対象外 — #631
real-dock slice が別途検証する。

## Feature: test_editor fixture discovery (in-process, `tests/host_path.rs`)

- **Rule**: `test_editor` は `test_gain`/`test_fault` と同一
  cdylib/factory に第 3 component として発見される。
  **Example**: `Vst3Library::load` → `descriptors().len() == 3`、
  3 件目の name が `"test_editor"`・vendor `"Ruviform Test"`・
  version `"0.0.0"`・features `"Fx"`。`countClasses == 6`。
- **Rule**: `test_editor` は shared `Fault` param を 1 件だけ持つ
  （Issue #885 で拡張 — `getParameterCount == 1` →
  `params().len() == 1`、id `0`・name `"Fault"`・plain
  `[0.0, 8.0]`・default `0.0`・`stepCount == 7`）。本 slice 当時の
  「param を持たない（`getParameterCount == 0`）」は #630 の
  歴史的正本。#885 以後は `Fault` event が component の `wanted`
  filter を通り、武装 mode が `process()` 内で発火する。
- **Rule**: component は pass-through — 非武装の `test_fault`
  と同じ bitwise copy を stereo block で返す。

## Feature: `IEditController::createView` dispatch (COM-level, in-crate unit test)

- **Rule**: `createView("editor")` は非 null の owned
  `*mut IPlugView` を返す（host が `ComPtr::from_raw` で所有権を
  取る）。`createView("other")`・null name は null。
- **Rule**: 返された view は独立した COM object — release 後に
  controller 経由で再取得できる。

## Feature: `IPlugView` contract (COM-level, in-crate unit test)

- **Rule**: `isPlatformTypeSupported("HWND")` → `kResultTrue`、
  他の type（`"NSView"`・`"X11EmbedWindowID"`・null）→
  `kResultFalse`。
- **Rule**: `getSize` は非 null out-pointer に決定的な固定 rect
  （初期値 640x480）を書き `kResultOk`。null → `kInvalidArgument`。
  `onSize` は rect を更新して `kResultOk`、null →
  `kInvalidArgument`。`canResize` → `kResultFalse`。
  `checkSizeConstraint` は非 null → `kResultOk`、null →
  `kInvalidArgument`。
- **Rule**: `setFrame(null)` → `kResultOk`、`setFrame(frame)` →
  `kResultOk`（`IPlugFrame` を retain/release）。`onFocus`/
  `onWheel`/`onKeyDown`/`onKeyUp` → `kResultOk`。
- **Rule**: `attached` は defined rejection を正直に返す:
  `type != "HWND"` → `kResultFalse`、null parent →
  `kResultFalse`、非 HWND 値 → `kResultFalse`（Windows は
  `IsWindow` gate）、非 Windows build → 常に `kResultFalse`。
  attach 成功後の `removed` → `kResultOk` で child を破棄し、
  再 `attached`/`removed` が安全。
- **Rule**: Windows では real parent HWND を持つ test が
  `attached` → `kResultOk` + 実 child HWND 生成を検証できる
  （`IsWindow`/`GetParent` で観測）。ただし default `cargo test`
  に real-HWND success を要求しない — headless 環境でも
  defined rejection 側のみを必須とする。

## Feature: headless `open_gui` through production bridge (`tests/host_path.rs`)

- **Rule**: `open_gui(ParentLease::new(0,0))` は payload 0 が real
  HWND でないため成功を偽装しない。Windows では
  `attached` → `kResultFalse` → `Err(HostError::PluginFailed)`、
  非 Windows では `EDITOR_PLATFORM_TYPE = None` →
  `Err(HostError::Unsupported)`。
- **Rule**: `open_gui` 失敗後の instance は破損しない —
  `stop_processing`/`deactivate`/`terminate` が通常どおり
  収束する。

## Guarantees

- `IPlugView` は `createView` 経由でのみ生成され、factory class
  ではない（`IPlugView` を `createInstance` で返さない）。
- view の Win32 呼出は `#[cfg(target_os = "windows")]` 内に閉じ、
  同一 cdylib が macOS/Linux でも build できる。
- fake lane（`ruviform-plugin-dock-test`）の lease payload は
  real HWND でないため `attached` → `kResultFalse` — fake
  attach が Pass を偽装しない。

## Non-goals

- real parent HWND への attach 成功 + composited pixel 判定
  （#631 real-dock slice の証拠軸）。
- `onSize` の container→plugin 通知、resize round-trip、
  focus/key/wheel/IME の実入力転送。
- CLAP editor、L1 editor attach、外部 SDK validator、非 Windows
  GUI 実走。

## Risk

- `IsWindow` pre-check を省略すると null/opaque payload が
  `CreateWindowExW` に流れ、fake attach が誤って成功し得る —
  本 spec はこの gate を必須要件として固定する。
