---
id: "DSGN-034ZCLAPED3dl3ceM3kQ2x"
type: "design"
status: "approved"
title: "Issue #630 CLAP test_editor slice — 設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-clap-test-editor.md"
    - "docs/discovery/issue-630-clap-test-editor.md"
    - "docs/designs/issue-630-test-editor.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-clap-test-editor.md"
  implements:
    - "docs/specs/issue-630-clap-test-editor.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-clap-test-editor.md"
    - "docs/designs/issue-630-test-editor.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-630-clap-test-editor.md"
    - "docs/tasks/issue-630-clap-test-editor.md"
    - "docs/test-specs/issue-630-clap-test-editor.md"
    - "crates/ruviform-plugin-test-vst3/src/editor_view.rs"
    - "crates/ruviform-plugin-clap/src/host.rs"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-clap-test-editor.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 CLAP test_editor slice — 設計

## Context

`docs/specs/issue-630-clap-test-editor.md` が要求する `test_editor`
plugin の構造・境界・検証軸を、VST3 `test_editor` slice（merged）と
CLAP 既存 fixture（`test_gain`/`test_fault`/`test_stateful`/
`test_instrument`）の先例に照らして確定する。host 側
（`ruviform-plugin-clap/src/host.rs`）は無変更で、本 slice は
plugin 側 fixture のみを追加する。

## Scope

### In Scope

- `src/editor.rs` 新規: 3 分割構成 + `PluginGuiImpl` + win32
  module + unit tests。
- `src/lib.rs`/`Cargo.toml`/`src/fault.rs`/`tests/host_path.rs`/
  `README.md` の最小更新。
- `Cargo.lock`（`windows-sys` target edge のみ）。
- doc chain + evidence。

### Out of Scope

- production crate への一切の変更。
- real parent HWND success path（#631）、L1 OpenGui/CloseGui
  （本番側 `Unsupported` by design）、floating window・
  `set_transient`/`suggest_title` 実 attach、外部 SDK validator、
  非 Windows 実走、consumer oracle。

## Components and Boundaries

### `TestEditorShared`（clap shared object）

- paramless・audio port pair 宣言（stereo in/out）を descriptor
  側に固定。GUI 状態を持たない — main-thread/thread-affinity
  境界を明示するため gui state は MainThread 側に閉じる。

### `TestEditorMainThread<'a>`（main-thread object）

- `PluginGuiImpl` を実装し、`RefCell<GuiState>` の interior
  mutability で GUI 状態を保持。`PluginGuiImpl` は `&self`
  verb のため borrow 規約は `try_borrow_mut`/`try_borrow` で
  reentrancy を defined rejection に収束させる（既存 fixture の
  `params`/state と同一方針）。
- `_shared: PhantomData<&'a TestEditorShared>` で lifetime を
  保持（ctor 引数として受け取るが読まない — 既存 3 class と同型）。

### `TestEditorAudioProcessor<'a>`

- `PluginProcessor` 実装。`process` は `fault.rs` の
  `pass_through`（`pub(crate)` 化済み）をそのまま呼ぶ —
  paramless pass-through law の単一実装を共有し、`rt_`
  `assert_no_alloc` 契約を維持。

### `PluginGuiImpl` verb マトリクス

| verb | 応答 |
| --- | --- |
| `is_api_supported` | `api_type==WIN32 && !is_floating && cfg!(windows)` のみ true |
| `get_preferred_api` | `GuiApiType::WIN32`（非 Windows は `None` 相当の honest 応答） |
| `create` | `GuiState` を初期化、size=640×480 固定 |
| `destroy` | child HWND を破棄して state クリア・冪等 |
| `set_scale` | 有限正値のみ受理して Ok（値は保持しない — CLAP は `set_scale` の無視を許容、child は物理 pixel）。非有限/非正は defined rejection |
| `get_size` | GuiSize{640,480}、Ok |
| `can_resize` | false |
| `get_resize_hints`/`adjust_size` | default trait 実装（resize 不可の honest 応答） |
| `set_size` | defined rejection（fixed-size editor） |
| `set_parent` | API type・create 済み・未 attach・`IsWindow` を検査し実 `WS_CHILD` HWND 生成。違反は全て defined rejection |
| `set_transient` | false/true ともに defined rejection（floating 非対応の正直な拒否） |
| `suggest_title` | `None`/default（実装しない verb は default に委譲） |
| `show`/`hide` | attach 済み child に対して `ShowWindow`/`SW_SHOW`/`SW_HIDE`。未 attach・未 create は defined rejection |

### win32 module（`#[cfg(target_os = "windows")]`）

- once-per-module class 登録: `GetModuleHandleExW
  (GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS)` で自 module の
  `HINSTANCE` を取得（host exe の instance 誤用を防止）、
  `RegisterClassW` + `GetClassInfoW` 確認、`lpfnWndProc =
  DefWindowProcW`（cdylib unload 後も登録 class が残存しても
  dangling wnd_proc を残さない — VST3 `editor_view.rs` 先例）。
  `hbrBackground` は process-lifetime の solid brush。
- `set_parent`: `Window::as_win32_hwnd()` → null 拒否 →
  `IsWindow` pre-check（opaque `ParentLease` payload を HWND と
  誤認しない境界）→ `CreateWindowExW`（`WS_CHILD|
  WS_VISIBLE|WS_CLIPSIBLINGS`、`parent` を親に、client rect
  サイズ）→ child handle を state に保持。
- `destroy`: `DestroyWindow` → state クリア。二重 destroy・
  未 create でも安全（冪等）。
- 非 Windows shim: 同名関数が defined rejection（`PluginFailed`
  相当の `Err`）に収束し、同一 cdylib が macOS/Linux でも build
  できる。

### `src/lib.rs` 変更

- `mod editor;`、descriptor index 4、`plugin_count()` 4→5、
  `create_plugin` dispatch に `TestEditorClapPlugin` を追加。
  既存 4 class の descriptor/id/挙動は不変。

### `tests/host_path.rs` 変更

- descriptor/factory pin を 4→5 に更新、editor index 4 の
  descriptor フィールド検証、params 無し・pass-through・foreign
  `ParamValue` event drop、opaque-lease `open_gui` defined
  rejection（`ParentLease::new(0,0)`）、非 editor class の
  `open_gui` → `Unsupported` を追加。

## Data and Control Flow

### `open_gui` 到達経路（host 側・変更なし、本 fixture が応える契約）

`ErasedPlugin::open_gui(ParentLease)` → `ClapPlugin::open_gui`:
`init` 前 reject・二重 GUI reject → `EDITOR_GUI_API` が `None`
（非 Windows）なら `Unsupported` →
`get_extension::<PluginGui>()` 無しなら `Unsupported` →
`is_api_supported(GuiConfiguration{api_type: WIN32, is_floating:
false})` → `create` → `Window::from_win32_hwnd(
lease.payload())` → `set_parent` → `show`。`set_parent`/`show`
失敗時 `gui.destroy` rollback → `PluginFailed`。
`close_gui`: `hide`(best-effort) + `destroy`。

### plugin 側 `PluginGuiImpl` 契約（本 slice の実装）

`set_parent` が host 提供 parent を `IsWindow` で検証してから
実 child HWND を生成する点が opaque payload 拒否の境界。
`Window` 型は handle 生存を保証しないため、`IsWindow` を省略
すると `CreateWindowExW` が opaque 値を dereference し得る。

### 文字列比較

CLAP 側は `Window::as_win32_hwnd()` の raw pointer を
`IsWindow`/`CreateWindowExW` に渡す — 文字列比較は発生しない
（VST3 `IPlugView::attached` の `"HWND"` 文字列 gate に相当する
ものは `GuiConfiguration.api_type == WIN32` enum 比較）。

## Non-Functional Constraints

- `unsafe` は `#[cfg(target_os = "windows")]` win32 module と
  `#[expect(unsafe_code)]` + SAFETY コメントに局所化。非 Windows
  build では win32 module 自体が存在しない。
- `process` は allocation-free/bounded/non-blocking —
  `pass_through` 共有で `rt_` 契約を維持。GUI verb は RT 経路に
  接続しない。
- class 登録は once-per-module、wnd_proc は `DefWindowProcW`
  （module unload 後の dangling 回避）、class/brush は
  process-lifetime fixture resource。
- fixture は test-only: production crate は本 crate に依存しない。
- `windows-sys` は `[target.'cfg(windows)'.dependencies]` のみ、
  VST3 fixture と同一 feature 集合、workspace lock 既存
  `windows-sys 0.61.2`。

## Risks and Trade-offs

- **`IsWindow` pre-check 省略リスク**: opaque payload が
  `CreateWindowExW` に流れ fake attach が誤成功し得る →
  `set_parent` の `IsWindow` gate を必須要件として固定。
- **class unload 残存**: module-local wnd_proc を登録すると
  cdylib unload 後に dangling → `DefWindowProcW` 採用で解消
  （VST3 先例と同一判断）。
- **`PluginGuiImpl` `&self` + RefCell borrow**: host が verb を
  再入した場合 `try_borrow_mut` が `Err` → defined rejection で
  収束（panic させない）。
- **非 Windows build**: `cfg!(windows)` で `is_api_supported`
  が常に false、win32 module は shim のみ — 同一 cdylib が
  全 OS で build 可能。

## Deferred Design Concerns

- real parent HWND + message pump 経由の `open_gui` success —
  #631 lane 拡張待ち（CLAP leg）。
- `set_transient`/`suggest_title` 実 attach、floating window —
  #389 D7 系 deferred。
- L1 OpenGui/CloseGui — 本番側 `Unsupported` by design。
- 外部 SDK validator — License・環境確保後の別 slice。

## Verification Notes

- in-process unit tests（`editor.rs` 内 `#[cfg(test)]`）で
  negotiation 行列・size/resize・create/destroy・set_parent
  defined rejection・show/hide・pass-through・`#[ignore]`d
  real-child-HWND を網羅。
- `tests/host_path.rs` で production `ClapLibrary` path の
  descriptor/factory pin・params 無し・pass-through・opaque-lease
  rejection・非 editor `Unsupported` を固定。
- `#[ignore]`d real-HWND test は Windows 実機で手動実走し、
  `IsWindow`/`GetParent`/`WS_CHILD`/`DestroyWindow` を観測 —
  evidence に記録。default `cargo test` では走らない
  （headless 環境でも defined rejection 側のみ必須）。
