---
id: "IMPL-034Y1GuSJhatlqbBsFEaYI"
type: "impl"
status: "completed"
title: "Issue #630 test_editor slice — 実装記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/tasks/issue-630-test-editor.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/editor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/editor_view.rs"
      - type: "file"
        path: "docs/impl/ir/issue-630-test-editor-slice.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-test-editor-slice.jsonl"
      - type: "file"
        path: "evidence/rf630/test-editor-verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/controller.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/factory.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/fault.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/strings.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/host_path.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-630-test-editor.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-630-test-editor.md"
    - "docs/plans/issue-630-test-editor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/exp/issue-630-test-editor-slice.jsonl"
    - "docs/impl/ir/issue-630-test-fault.md"
    - "docs/test-specs/issue-630-test-editor.md"
    - "evidence/rf630/test-editor-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-630-test-editor.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-630-test-editor-slice.jsonl"
    rejected: []
---

# Issue #630 test_editor slice — 実装記録

## Summary

`ruviform-plugin-test-vst3` に `test_editor` component/controller
pair を追加し、factory を 4 class → 6 class（3 plugin）に拡張した。
`test_editor` は parameterless pass-through processor で、その
controller の `createView("editor")` が実 `IPlugView` を返す —
Windows では `attached` が host 提供 parent HWND 配下に実
`WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS` 子 HWND を生成し、
`removed` で破棄する。production host crate は zero-diff。

## Context

Follows `docs/tasks/issue-630-test-editor.md` →
`docs/plans/issue-630-test-editor.md` →
`docs/specs|designs/issue-630-test-editor.md`、verified against
`docs/test-specs/issue-630-test-editor.md`。Precedent:
`docs/impl/ir/issue-630-test-fault.md`（fixture class 追加手順）、
`crates/ruviform-poc-native-dock` の Win32 probe（class 登録・
brush・`DefWindowProcW` fallback）。

## Implementation

### Changes

- `crates/ruviform-plugin-test-vst3/src/editor.rs`（新規）—
  `TestEditorProcessor`: parameterless pass-through。
  `EDITOR_NO_PARAM = ParamId(u32::MAX)` sentinel により
  `collect_param_events` の `wanted` filter が全
  `IParamValueQueue` を拒否。`TestEditorComponent =
  TestComponent<TestEditorProcessor>`、fresh `uid` の component
  CID。
- `crates/ruviform-plugin-test-vst3/src/editor_view.rs`（新規）—
  `TestEditorView`: 実 `IPlugView` COM 実装。
  `isPlatformTypeSupported` は `"HWND"` のみ `kResultTrue`
  （`open_gui` が要求する値。`kResultOk` では unsupported と
  誤読される）。`attached` は HWND type + live window
  （`IsWindow` gate）+ 未 attach の場合のみ `win32::attach` —
  Windows では parent client area 一杯の `WS_CHILD` HWND を生成、
  非 Windows・null・非 window・二重 attach・生成失敗は全て
  `kResultFalse` の defined rejection。`removed` は冪等
  `kResultOk` で子 HWND を `DestroyWindow`。`setFrame` は
  `ComPtr<IPlugFrame>` で所有権管理（addRef/release）。
  `getSize`/`onSize` は既定 640x480 rect の往復、`canResize` は
  `kResultFalse`（host bridge の `resizeView` 拒否と対称）、
  `checkSizeConstraint` は valid rect を受理。通知 verb
  （wheel/key/focus）は `kResultOk` で受理・無視。状態は
  `RefCell` + `try_borrow_mut`（`component.rs` の
  `ComponentState` と同じ re-entrant rejection 方針）。
- `editor_view.rs` `win32` module（`#[cfg(target_os="windows")]`
  のみ）— once-per-module class 登録（`"RuviformTestEditor"`、
  `ERROR_CLASS_ALREADY_EXISTS` 時は `GetClassInfoW` で確認）、
  `CreateSolidBrush`（緑、PoC probe の赤と区別）を
  `WNDCLASSW.hbrBackground` に設定。**review 修正**: 当初は
  module 内 `wnd_proc` で `WM_ERASEBKGND`/`WM_PAINT` を fill
  する設計だったが、DLL 登録の window class は module unload
  後も process に残存するため unload/reload 後に code pointer
  が dangling して host crash する欠陥（review B1）が判明。
  `lpfnWndProc = DefWindowProcW`（`user32` 内、unload されない）
  + class brush の erase fill に変更し、module 所有 code を
  class に残さない設計へ修正。`hInstance` も
  `GetModuleHandleExW(FROM_ADDRESS|UNCHANGED_REFCOUNT)` で自
  module のものを取得（`GetModuleHandleW(NULL)` は host EXE を
  返すため不可）。message pump も専用 thread も持たない
  （#389 D1 thread-affinity — host が parent HWND の thread で
  `attached` を呼ぶ前提）。非 Windows は `win32` shim が
  `None`/no-op を返し `attached` が defined `kResultFalse` に
  収束 — cdylib は全 OS で build 可能。
- `crates/ruviform-plugin-test-vst3/src/controller.rs` —
  `EDITOR_CONTROLLER_CID`（fresh `uid`）と
  `TestEditorController`: parameter surface 全て honest
  rejection/`0`（`getParameterCount` → 0）、state stream は
  `kNotImplemented`。`createView(name)` は `"editor"` のみ
  `ComWrapper::new(TestEditorView::new()).to_com_ptr::<
  IPlugView>().into_raw()` の owned pointer を返し、それ以外
  （null 含む）は null。
- `crates/ruviform-plugin-test-vst3/src/factory.rs` —
  `countClasses` 6、index 4/5 = `test_editor` component/
  controller（`getClassInfo`/`getClassInfo2`/
  `getClassInfoUnicode` 全 arm + `createInstance` dispatch）。
- `crates/ruviform-plugin-test-vst3/src/strings.rs` —
  `cstring_eq(*const c_char, &str)`: `FIDString` の内容比較
  （null → false、prefix/大小文字を区別）。`createView`/
  `isPlatformTypeSupported`/`attached` の name dispatch に使用。
  **review 修正**: 初版は `CStr::from_ptr` の unbounded scan
  だったが、approved design の「上限 1024 byte」防御を実装 —
  byte 単位の bounded scan（1024 byte 超過・非終端は false）に
  変更し `cstring_eq_bounds_the_scan` test を追加。
- `crates/ruviform-plugin-test-vst3/src/fault.rs` —
  `pass_through` を `pub(crate)` 化し `test_editor` と共有
  （DSP law の重複を排除）。
- `crates/ruviform-plugin-test-vst3/src/lib.rs` — `editor`/
  `editor_view` module 追加、crate doc に 3 class 構成を記述。
- `crates/ruviform-plugin-test-vst3/Cargo.toml` — Windows-only
  `windows-sys 0.61` edge（`Win32_Foundation`/
  `Graphics_Gdi`/`System_LibraryLoader`/`UI_WindowsAndMessaging`、
  `ruviform-poc-native-dock` probe と同 feature 群から PoC 専用
  分を除く）。`Cargo.lock` はこの edge のみ差分。
- `crates/ruviform-plugin-test-vst3/tests/host_path.rs` —
  descriptor/factory 数を 3 に更新、`test_editor` descriptor/
  zero-param surface/pass-through/opaque-lease `open_gui`
  rejection（Windows: `PluginFailed` — `attached` の `IsWindow`
  gate が fake payload を拒否、非 Windows: `Unsupported`）、
  `test_gain`/`test_fault` の editor 不在（`Unsupported`）を追加。
- `crates/ruviform-plugin-test-vst3/README.md` — 3 plugin class
  構成、`test_editor` の HWND 契約、#631 real-dock lane への
  real-parent-HWND `open_gui` success evidence 委譲を記述。

### Decisions During Implementation

- `IPlugView` は factory class ではなく `createView` の返り値
  のみ — host が `ComPtr` で所有権を取る VST3 契約に合わせ、
  `ComWrapper::to_com_ptr().into_raw()` で owned pointer を返す。
- platform support は `kResultTrue` 固定 — `open_gui` は
  `kResultOk` を unsupported と誤読する（`host.rs` の契約）。
- `attached` の parent 検証は `IsWindow` のみ — fake-lane lease
  payload（opaque `u64`）が HWND に見せかけても live window で
  なければ拒否される。thread-affinity は host 側契約であり
  fixture は pump/thread を持たない。
- `RefCell` + `try_borrow_mut` — `component.rs` の
  `ComponentState` と同じ「VST3 thread model 前提 + re-entrant
  は defined rejection」方針を踏襲。
- 実 child HWND の e2e（`open_gui` success path）は本 slice の
  default lane に含めない — real HWND + message pump が必要で
  #631 real-dock lane の責務。代わりに `#[ignore]`d
  `attached_creates_real_child_hwnd` を Windows session で手動
  実走し plugin 側契約の実測 evidence とした。
- review-protocol（Reviewer A: spec 適合 / Reviewer B: 欠陥
  探索の独立 2 pass）実施 — CONFIRMED findings を修正:
  B1 window class unload dangling（P1 → `DefWindowProcW` class
  へ変更）、A1 `cstring_eq` unbounded scan（P2 → 1024 byte
  bound 実装）、A2 test-spec version 誤記（P3 → `"0.0.0"`）、
  A3 relation path の `-slice` suffix 不整合（P3 → 修正）。
  B2 `isPlatformTypeSupported` の非 Windows `kResultTrue`
  （approved design どおり・host は `EDITOR_PLATFORM_TYPE=None`
  で事前 gate）、B3 `getSize` と child size の乖離（spec が
  fixed size を許容）、B4 `removed` の `DestroyWindow` 失敗
  無視（host thread-affinity 契約違反時のみ）は non-blocking
  note として記録。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-630-test-editor-slice.jsonl`（adopted）—
  binding signature 確認（`TBool=u8`、`ViewRect` の
  `Steinberg` root 配置、`ComPtr`/`ComRef` 所有権 semantics）、
  Win32 probe 先例の移植、実 HWND 手動実走の記録。

### Rejected

- なし。

## Validation

`evidence/rf630/test-editor-verification.md` に全 exit code。
`mise run test-l1-real-plugin` 全 lane pass（adapter 3+3、
test-plugin 3+3、fault 8+8 — 6 class 化後も runner は
`plugin_id` dispatch で影響なし）。`fmt-check`/`check`/`lint`/
`test`/`check-l1-real-plugin`/`lint-l1-real-plugin`/`test-rt`
全 pass。`attached_creates_real_child_hwnd`（`#[ignore]`、
Windows window station 必須）は本 session で手動実走 pass —
実 `WS_CHILD` HWND 生成・parenting・`removed` 破棄を
`GetWindow(GW_CHILD)`/`GetParent`/`IsWindow` で実測確認
（`DefWindowProcW` class 版でも再実走 pass）。review-protocol
2 pass 実施・P0/P1/P2 finding は全て修正済み、P3 doc 不整合も
修正、残りは non-blocking note。

## Risks

- `attached` は呼出 thread の window station に依存 —
  非 interactive session では `CreateWindowExW` が失敗し
  `kResultFalse` に収束（defined rejection、faked success なし）。
- class 登録は process singleton — unload 後も残存するが、
  `lpfnWndProc = DefWindowProcW` のため dangling code pointer は
  発生しない（review B1 対応済み）。class ATOM と brush の
  one-time leak は許容（test fixture、process あたり 1 回）。
- `removed` は `DestroyWindow` の戻り値を見ない — host が
  thread-affinity 契約（#389 D1）を破った場合のみ子 HWND が
  残存し得る。defined rejection の経路はないため non-blocking
  note として記録（review B4）。

## Follow-ups

- #631 real-dock slice: real parent HWND + message pump 経由の
  `open_gui` success evidence（`open_gui` → `IPlugView::attached`
  → 実 dock）。本 slice の view がその plugin 側供給側。
- `test_stateful`/`test_instrument`、CLAP 側 editor 相当、
  外部 SDK validator、非 Windows OS 実走 — 後続 slice/既存
  CI lane。
