# RF-VTEST-03 test_editor slice — 検証記録

- Issue: `scarecrow173/ruviform#630`（AC-4 plugin 側 `IPlugView` fixture のみ）
- Slice: `test_editor` VST3 component/controller pair + 実 `IPlugView`（Windows child HWND）
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints 正本）
- 記録日: 2026-11-15
- Case Schema v1 語彙・`environment_kind=test-plugin`。deterministic evidence と real-run evidence を分離。

## 実装概要

- `crates/ruviform-plugin-test-vst3/src/editor.rs`（新規）: `TestEditorProcessor` — parameterless pass-through。`EDITOR_NO_PARAM = ParamId(u32::MAX)` sentinel で全 `IParamValueQueue` を拒否。`TestEditorComponent = TestComponent<TestEditorProcessor>`、fresh `uid(0x3F8A_4C62,0x9D1E_7B05,0xA6F3_48D9,0xC2E7_5B81)` の component CID。
- `crates/ruviform-plugin-test-vst3/src/editor_view.rs`（新規）: `TestEditorView` — 実 `IPlugView`。`isPlatformTypeSupported("HWND")` → `kResultTrue` のみ。`attached` は HWND type + `IsWindow` live gate + 未 attach の場合のみ `win32::attach`（Windows: `WS_CHILD|WS_VISIBLE|WS_CLIPSIBLINGS` を parent client area 一杯に生成）。`removed` は冪等 `kResultOk` + `DestroyWindow`。`setFrame`/`getSize`/`onSize`/`canResize`/`checkSizeConstraint`/通知 verb は VST3 契約どおりの defined 応答。状態は `RefCell`+`try_borrow_mut`。
- `editor_view.rs` `win32` module（`#[cfg(target_os="windows")]`）: once-per-module `RegisterClassW`（`"RuviformTestEditor"`、`GetClassInfoW` fallback 確認）、`CreateSolidBrush` 緑を `WNDCLASSW.hbrBackground` に設定。**review 修正（B1）**: `lpfnWndProc` は module 内関数ではなく `DefWindowProcW` — DLL 登録 class は unload 後も残存するため module 内 `wnd_proc` は dangling code pointer となり host crash の原因になり得た。`DefWindowProcW` は `user32` 内で unload されず、class brush の `WM_ERASEBKGND` fill で solid 色を維持。`hInstance` は `GetModuleHandleExW(FROM_ADDRESS|UNCHANGED_REFCOUNT)` で自 module を取得し `MODULE_H` に保持（`CreateWindowExW` と共用）。message pump・専用 thread なし（#389 D1 thread-affinity）。非 Windows は shim が `None`/no-op → `attached` は defined `kResultFalse`。
- `controller.rs`: `EDITOR_CONTROLLER_CID`（fresh `uid(0x5E7A_9C31,0x2D8F_4B67,0xE1A3_59D4,0x8C6B_2F07)`）+ `TestEditorController` — parameter surface 全て `0`/honest rejection、state stream `kNotImplemented`、`createView("editor")` のみ owned `IPlugView` を返す。
- `factory.rs`: `countClasses` 6（gain/fault/editor × comp/ctrl）、3 `getClassInfo*` arm + `createInstance` dispatch。
- `strings.rs`: `cstring_eq`（FIDString 内容比較、null→false、**1024 byte bounded scan** — review A1 で approved design の上限防御を実装）。
- `fault.rs`: `pass_through` を `pub(crate)` 化して `test_editor` と共有。
- `Cargo.toml`: Windows-only `windows-sys 0.61` edge（Foundation/Gdi/LibraryLoader/WindowsAndMessaging）。`Cargo.lock` 差分はこの edge のみ。
- `tests/host_path.rs`: descriptor/factory 3、`test_editor` descriptor/zero-param/pass-through/opaque-lease `open_gui` rejection、gain+fault editor 不在を追加。

## 実行した検証（canonical = mise、実 exit code）

| command | result | exit |
| --- | --- | --- |
| `mise run fmt-check` | pass | 0 |
| `mise run check ruviform-plugin-test-vst3` | pass（`cargo check --locked --all-targets`） | 0 |
| `mise run lint ruviform-plugin-test-vst3` | pass（`clippy --all-targets -- -D warnings` 相当） | 0 |
| `mise run test ruviform-plugin-test-vst3` | 61 unit + 1 ignored（real HWND）+ host_path 10 + l1 bin 2 pass | 0 |
| `mise run check-l1-real-plugin` | pass（`--features l1-runner` 全 target） | 0 |
| `mise run lint-l1-real-plugin` | pass | 0 |
| `mise run test-l1-real-plugin` | pass — adapter e2e 3+3、test-plugin e2e 3+3、fault e2e 8+8（6 class 化後も runner は `plugin_id` dispatch で影響なし） | 0 |
| `mise run test-rt` | pass — workspace `rt_` filter 全 pass（exit 0）。editor の pass-through は非 RT 経路のみ、Win32 は process 経路外 | 0 |
| `mise exec -- cargo test -p ruviform-plugin-test-vst3 attached_creates_real_child_hwnd -- --ignored` | **pass**（review 修正後の `DefWindowProcW` class 版でも再実走 pass）— 実 window station 上で `attached` → `GetWindow(GW_CHILD)` で子 HWND 実在・`GetParent` で parenting 確認・`IsWindow` live・二重 `attached` は `kResultFalse`・`removed` で `IsWindow==0` 破棄を実測 | 0 |
| review-protocol（Reviewer A spec 適合 + Reviewer B 欠陥探索、独立 2 subagent pass） | CONFIRMED: B1 window class unload dangling（P1）→ `DefWindowProcW` class + own-module `hInstance` に修正、A1 `cstring_eq` unbounded（P2）→ 1024 byte bound 実装、A2 test-spec version 誤記・A3 relation path（P3）→ doc 修正。non-blocking: B2 非 Windows `kResultTrue`（approved design どおり）、B3 `getSize`/child size 乖離（spec が fixed size 許容）、B4 `DestroyWindow` 戻り値無視（host 契約違反時のみ） | — |

cargo 直接実行は補助証拠（canonical = mise）。`*-msrv`/`*-full` lane は本 slice 範囲外。

## IPlugView 契約の確定事項（実機観測で確定）

1. **platform support は `kResultTrue` 必須**: host `open_gui` は `isPlatformTypeSupported("HWND")` に `kResultOk` ではなく `kResultTrue` を要求 — `kResultOk` は unsupported 誤読。
2. **fake payload は `IsWindow` gate で拒否**: `ParentLease::new(0,0)` の opaque payload `0` は HWND に見せかけても live window でないため `attached` が `kResultFalse` → host 側 `PluginFailed`（Windows）/ 非 Windows は `Unsupported`（`EDITOR_PLATFORM_TYPE=None` で `createView` 未到達）。
3. **実 child HWND は生成・破棄とも実測**: `attached_creates_real_child_hwnd` が Windows session で pass — `WS_CHILD` 生成、`GW_CHILD` 検出、`GetParent` 一致、二重 attach 拒否、`removed` で `DestroyWindow`（`IsWindow==0`）。
4. **`test_gain`/`test_fault` は editor を持たない**: `createView` が全 name で null → `open_gui` は `Unsupported` に収束。`test_editor` が suite 唯一の GUI surface。
5. **6 class 化は L1 runner に無影響**: runner は factory scan → `plugin_id` dispatch のため class 数増加を許容。descriptor assert は registered descriptors replay のみ。

## Pass/Fail/NotRun/Unsupported

- **Pass**: `test_editor` descriptor/zero-param surface、pass-through audio（foreign `ParamValue` event も queue filter で drop）、`createView("editor")` owned pointer dispatch、platform/size/frame/attach/remove の COM-level 契約、opaque-lease `open_gui` defined rejection、gain/fault editor 不在、実 child HWND 生成・破棄（手動 `--ignored` 実走）、L1 全 lane、production host path（`Vst3Library` discovery → `ErasedPlugin`）。
- **NotRun**: real parent HWND + message pump 経由の `open_gui` **success** path — #631 real-dock lane の責務（本 slice は plugin 側供給のみ検証）。外部 SDK validator、非 Windows OS 実走（Win32 shim の defined rejection は compile-time cfg で保証、実走は既存 CI lane）、`test_stateful`/`test_instrument`（後続 slice）、`*-msrv`/`*-full`。
- **Unsupported**: 非 HWND platform（NSView/X11/UIView）は fixture が `kResultFalse` で honest rejection — 本 slice 範囲では supported surface ではない。
- **Deferred**: consumer oracle 照合、CLAP 側 editor 相当 — 後続 slice/範囲外。

## 境界遵守

- production crate（`ruviform-plugin-{vst3,clap,host,export,core}`）zero-diff。
- `unsafe` は `#[expect(unsafe_code)]` + SAFETY comment の局所許可のみ（COM vtbl dispatch・Win32 FFI・テストの raw pointer）。
- `windows-sys` edge は `cfg(windows)` target dependency — 非 Windows build に Win32 依存を持ち込まない。
- fake host-side GUI 挙動なし — opaque lease は実 `IsWindow` gate で拒否されることを host_path で実測。
- `Cargo.lock` 差分は `windows-sys 0.61.2` edge のみ。
- fixture は出荷物ではない（test-only 宣言は crate doc/README に明記）。
