---
id: "DESIGN-034Y11NzSzQKwkY0SHh8oe"
type: "design"
status: "approved"
title: "Issue #630 test_editor slice — 設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-test-editor.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-fault.md"
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-test-editor.md"
  implements:
    - "docs/specs/issue-630-test-editor.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-test-editor.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-fault.md"
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
  blocks:
    - "docs/plans/issue-630-test-editor.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-editor.md"
    - "docs/plans/issue-630-test-editor.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-editor.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_editor slice — 設計

## Context

Issue #630 AC-4 は「Native Editor が実 child HWND を生成できる」
ことを要求する。本 slice はその plugin-fixture 前提 —
VST3 plugin 側の `IPlugView` 実装 — のみを担う。host 側の
`open_gui` 呼出列・`BridgePlugFrame`・neutral session/lease seam
は Issue #389 (`docs/designs/issue-389-vst3-l0-editor-attach.md`
D1/D2/D4/D6) で実装済みであり、変更しない。

制約の要点:

- **test-only・独立実装**: fixture は `vst3` binding + neutral
  契約の上に直接実装し、`ruviform-plugin-vst3` 内部を再利用しない
  (suite 規約)。`windows-sys` は `ruviform-poc-native-dock` の
  PoC probe 先例と同一 edge のみを承認済みで追加する。
- **multi-class factory**: test_fault slice で `countClasses` 4・
  generic `TestComponent<P>`/`ParamController` shell が導入済み。
  本 slice は index 4/5 に `test_editor` component/controller を
  追加し 6 class とする。
- **host 契約**: `open_gui` は `createView("editor")` の owned
  pointer を `ComPtr::from_raw` で受け取り、
  `isPlatformTypeSupported` に `kResultTrue` を要求し、
  `setFrame(BridgePlugFrame)` → `attached(parent, "HWND")` を
  この順で呼ぶ。`attached` 失敗時は `setFrame(null)` rollback の
  上 `Err(PluginFailed)`。`close_view` は `removed()` を
  debug-assert `kResultOk` した後 `setFrame(null)` で frame を
  解放する。
- **正直な拒否**: null/非 HWND parent、非 Windows、window 生成
  失敗は `kResultFalse` の defined rejection — 成功を偽装しない。
  fake lane の lease payload は real HWND ではないため
  `attached` → `kResultFalse` が正しい応答であり、fake-lane の
  editor-attach verdict は本 slice では変えない。

## Scope

### In Scope

- `src/editor.rs`: `TestEditorProcessor`（param なし pass-through）
  + `TestEditorComponent`（`TestComponent<P>` alias、独自 CID）。
- `src/editor_view.rs`: `TestEditorView`（`IPlugView` impl）+
  Windows child-HWND block。
- `src/controller.rs`: `TestEditorController`（`IEditController`、
  `createView("editor")` のみ非 null）。
- `src/factory.rs`: index 4/5 の class-info/createInstance arms、
  `countClasses` 6。
- `src/strings.rs`: bounded `cstring_eq` helper。
- `src/lib.rs`: module 追加と doc 更新。
- `Cargo.toml`: 承認済み `windows-sys` edge。
- `tests/host_path.rs`: descriptor pin 更新 + COM-level/headless
  editor case。
- `README.md`: class 一覧更新。

### Out of Scope

spec の Out of Scope と同一（CLAP editor、real-HWND attach
成功証拠、fake-lane 変更、L1 attach、production crate）。

## Components and Boundaries

```
crates/ruviform-plugin-test-vst3/
├── Cargo.toml            # +[target.'cfg(windows)'.dependencies] windows-sys
├── README.md             # 3 class 構成へ更新
└── src/
    ├── lib.rs            # mod editor; mod editor_view; suite doc 更新
    ├── strings.rs        # +cstring_eq (bounded, NUL 必須)
    ├── editor.rs         # TestEditorProcessor + TestEditorComponent (新規)
    ├── editor_view.rs    # TestEditorView: IPlugView + win32 child HWND (新規)
    ├── controller.rs     # +TestEditorController + EDITOR_CONTROLLER_CID
    ├── factory.rs        # index 4/5 arms、countClasses 6
    ├── component.rs      # 変更なし (generic shell 流用)
    ├── fault.rs          # pass_through を pub(crate) 化のみ
    └── block.rs/gain.rs  # 変更なし
```

境界:

- **ABI 境界** (`editor_view.rs`): `IPlugViewTrait` の全 method。
  内部状態は `RefCell<ViewState>`（`hwnd: Option<usize>`、
  `frame: Option<ComPtr<IPlugFrame>>`、`size: ViewRect`）。
  `try_borrow_mut` で re-entrancy を `kResultFalse` に写像
  （component.rs と同一方針）。state を触らない verb
  （platform support 判定・ack のみの通知系）は borrow せず
  純粋応答する。
- **Win32 境界** (`editor_view.rs` 内 `#[cfg(target_os =
  "windows")] mod win32`): `windows-sys` 呼出を局所化。
  `RegisterClassW` は `std::sync::Once` で once-per-module、
  solid brush は `CreateSolidBrush` を `Once` で 1 回確保して
  `WNDCLASSW.hbrBackground` に渡す。`lpfnWndProc` は
  **`DefWindowProcW`**（実装時修正: DLL 登録の window class は
  module unload 後も process に残存するため、本 binary 内の
  `wnd_proc` を登録すると unload/reload 後に code pointer が
  dangling して host が crash する。`DefWindowProcW` は
  `user32` 内にあり unload されず、class `hbrBackground` 経由で
  `WM_ERASEBKGND` の solid fill が得られる — module 所有 code
  を class に残さない設計）。`hInstance` は
  `GetModuleHandleExW(FROM_ADDRESS)` で自 module のものを使う
  （`GetModuleHandleW(NULL)` は host EXE を返すため不可）。
  自前の message pump・thread・timer は持たない
  — host が parent HWND の thread 上で呼ぶ契約（#389 D1 の
  thread-affinity 契約と同一）。`IsWindow` pre-check →
  `CreateWindowExW(WS_CHILD|WS_VISIBLE|WS_CLIPSIBLINGS)` →
  失敗時 null。非 Windows build ではこの module 自体が存在せず、
  `attached` は `kResultFalse` の defined rejection に収束する。
- **Controller 境界** (`controller.rs`): `ParamController` は
  param 駆動で editor の要件（param 0 個・view 生成）に合わない
  ため別 struct。`getParameterCount` → 0、`createView` は
  `name == "editor"` のときのみ `ComWrapper::new(TestEditorView)`
  → `to_com_ptr::<IPlugView>()` → `into_raw()` で +1 owned
  pointer を返す（`GetPluginFactory` の所有権移転と同型）。
  それ以外の name（null 含む）は null。
- **Component 境界** (`editor.rs`): `TestComponent<P>` shell を
  `TestEditorProcessor` で再利用。`param_id` は suite に存在しない
  id を sentinel (`ParamId(u32::MAX)`) とし、`collect_param_events`
  が全 queue を reject する — param を持たない component が
  param event を誤って採用しない構造的保証。DSP は `fault.rs` の
  `pass_through`（`pub(crate)` 化）を共有 — 同一責務
  （channel-for-channel bounded copy）の重複実装を避ける。

## Data and Control Flow

### `open_gui` 到達経路（host 側・変更なし、本 fixture が応える契約）

```text
host: controller.createView("editor") → *mut IPlugView (owned, +1)
      null                              → Err(Unsupported)
view.isPlatformTypeSupported("HWND")
      != kResultTrue                    → Err(Unsupported)
view.setFrame(BridgePlugFrame) != ok   → Err(PluginFailed)
view.attached(parent, "HWND") != ok    → setFrame(null) rollback → Err(PluginFailed)
close: view.removed() (kResultOk 必須) → setFrame(null) → release
```

### plugin 側 `IPlugView` 契約（本 slice の実装）

| method | 応答 |
| ------ | ---- |
| `isPlatformTypeSupported(t)` | `t == "HWND"` → `kResultTrue`、他 → `kResultFalse` |
| `attached(parent, t)` | `t != "HWND"` → `kResultFalse`。Windows: `parent` null/`!IsWindow` → `kResultFalse`、`CreateWindowExW` 失敗 → `kResultFalse`、成功 → child HWND を state に保持し `kResultOk`。非 Windows: 常に `kResultFalse`。attach 済みの再 `attached` → `kResultFalse`（defined rejection） |
| `removed()` | child HWND を `DestroyWindow`、state を初期化、`kResultOk`（常に成功 — host が debug-assert） |
| `getSize(*mut ViewRect)` | null → `kInvalidArgument`。state の size（初期 640x480、`onSize` で更新）を書き `kResultOk` |
| `onSize(*mut ViewRect)` | null → `kInvalidArgument`。size を更新し `kResultOk` |
| `setFrame(*mut IPlugFrame)` | null → 旧 frame release のみ `kResultOk`。非 null → `ComRef::from_raw` で +1 retain、旧 frame release、`kResultOk` |
| `canResize()` | `kResultFalse`（`BridgePlugFrame::resizeView` の honest refusal と対称） |
| `checkSizeConstraint(*mut ViewRect)` | null → `kInvalidArgument`、非 null → `kResultOk`（size をそのまま受理） |
| `onFocus`/`onWheel`/`onKeyDown`/`onKeyUp` | `kResultOk`（ack のみ — 入力転送の実装はしない） |

### 文字列比較

`FIDString`（`*const c_char`）の内容比較は `strings.rs` の
`cstring_eq(ptr, expected)` に集約: null → false、bounded scan
（上限 1024 byte — 非終端 hostile 入力での runaway を防ぐ）で
`expected` の byte 列 + NUL と一致したときのみ true。
`"HWND"`・`"editor"` の両方がこの helper を使う。

## Non-Functional Constraints

- **RT**: editor view は `process` 経路に接続しない
  （component は既存 generic shell のまま — editor は
  `process`/`setupProcessing` の RT path を変更しない）。
  window-management は UI thread 契約で RT thread からは呼ばれない。
- **unsafe**: COM 境界（`#[expect(unsafe_code)]` on trait impls、
  既存様式）+ Win32 block（`#[cfg(target_os="windows")] mod
  win32` に `#[expect(unsafe_code)]`、SAFETY コメント付き）。
  `unsafe` を window-management 以外へ拡散しない。
- **依存**: `windows-sys = "0.61"` の承認済み feature のみ
  （`Win32_Foundation`/`Win32_Graphics_Gdi`/
  `Win32_System_LibraryLoader`/`Win32_UI_WindowsAndMessaging`）。
  `windows-sys 0.61.2` は workspace lock に既存 — 新規 package
  entry は出ない想定。
- **thread-affinity**: view は message pump・thread を持たず、
  parent HWND を所有する thread 上でのみ Win32 呼出を行う
  （host の `open_gui` 呼出契約と同一 — #389 D1）。
- **lint**: workspace lints `-D warnings` 通過、`#[allow]` 禁止、
  `#[expect]` のみ。

## Risks and Trade-offs

- **`IsWindow` pre-check の意味**: `ParentLease::new(0,0)` の
  payload 0 → `attached(null, "HWND")` → `IsWindow(0)` は false
  → `kResultFalse` → host `Err(PluginFailed)`。fake lane の
  opaque `u64` payload も同じく `IsWindow` で落ちる — fake
  attach が Pass を偽装しない構造的保証になる。
- **class 登録の一意性**: window class 名は
  `"RuviformTestEditor"` で PoC の `"RvfProbeEditor"` と別。
  `Once` で once-per-module — 複数 view instance が同 class を
  共有する（hwnd は instance ごと）。
- **`removed` の冪等性**: host は `removed` を 1 回呼ぶが、
  plugin 側でも hwnd state を初期化するため二重 `removed` /
  `removed` 後の `attached`（再 attach）は安全に扱える。
- **非 Windows build**: `editor_view.rs` の Win32 block は cfg で
  消えるが `IPlugView` impl 自体は全 OS で compile される —
  `attached` は非 Windows で常に `kResultFalse`（host は
  `EDITOR_PLATFORM_TYPE=None` で先に `Unsupported` を返すため
  到達しないが、COM-level 直接呼出でも defined rejection）。
- **`test_gain`/`test_fault` への影響**: factory index 0-3 の arm・
  CID・descriptor は不変。`"Audio Module Class"` filter により
  descriptor 数は 2→3（component のみ数える）。

## Deferred Design Concerns

- real-HWND への successful `attached` + pixel/compositing 判定 —
  `scarecrow173/ruviform#631` real-dock lane slice が本 fixture を
  消費する（real parent HWND + message pump が必要）。
- `onSize` の container→plugin 通知経路、`checkSizeConstraint`
  の実 resize、focus/key/wheel 転送 — #389 D7 で deferred 済み、
  real attach が存在してから扱う。
- CLAP `test_editor` — clack gui extension、別 dep 承認が必要。
- L1 editor attach — Issue #115 スコープ、Unsupported by design。

## Verification Notes

- [ ] `mise run fmt-check` / `check` / `lint` / `test`
  （`ruviform-plugin-test-vst3`）
- [ ] `mise run check-l1-real-plugin` / `lint-l1-real-plugin` /
  `test-l1-real-plugin`（既存 gain/fault e2e が緑のまま）
- [ ] `mise run test-rt`（inert 経路のみ — editor は process 非接続）
- [ ] `mise exec -- prek run --all-files`
- [ ] `git diff` で forbidden path（production crates・
  `ruviform-plugin-test-clap`・`ruviform-plugin-dock-test`・
  `mise.toml`・root `Cargo.toml`）が空
- [ ] COM-level `IPlugView` case が unit test として収集・実行
  されること（`cargo test -p ruviform-plugin-test-vst3`）
