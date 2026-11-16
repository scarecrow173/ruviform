# RF630 CLAP-slice 実施 report — CLAP `test_editor` test plugin (`ruviform-plugin-test-clap` 5 番目の class)

- Issue: https://github.com/scarecrow173/ruviform/issues/630 (CLAP slice: `test_editor` — `clap.ext.gui` plugin 側 fixture)
- Branch: `scarecrow173/rf630-clap-editor`(worktree `G:\orca\workspaces\ruviform\rf630-clap-editor`)
- Base: main(VST3 `test_editor` slice = merged PR#668 が直接の先例)

## 実施内容(本 slice)

| 区分 | 内容 |
| --- | --- |
| 新規 module | `src/editor.rs` — `TestEditorShared`/`TestEditorMainThread<'a>`/`TestEditorAudioProcessor<'a>`/`TestEditorClapPlugin` の 3 分割構成 + `PluginGuiImpl` + `#[cfg(windows)]` win32 module + `#[cfg(test)]` tests |
| Extension | `declare_extensions` に `PluginAudioPorts` + `PluginGui` を登録 — `clap.ext.gui` を plugin 側で宣言 |
| GUI verb | `is_api_supported`(`GuiApiType::WIN32` + `is_floating==false` + `cfg!(windows)` のみ true)、`get_preferred_api`、`create`/`destroy`(冪等)、`set_scale`、`get_size`(640×480 固定)、`can_resize==false`/`set_size` rejection、`set_parent`/`set_transient`/`show`/`hide` — 全て defined 応答、成功捏造なし |
| Win32 | once-per-module `RegisterClassW`(`lpfnWndProc=DefWindowProcW` — cdylib unload 後も class が残存しても dangling しない、VST3 `editor_view.rs` 先例)、`GetClassInfoW` 確認、`GetModuleHandleExW(FROM_ADDRESS)` own-module `HINSTANCE`、`IsWindow` pre-check → `CreateWindowExW(WS_CHILD|WS_VISIBLE|WS_CLIPSIBLINGS)` → `DestroyWindow`。非 Windows は同 module shim が defined rejection |
| DSP | paramless pass-through — `fault.rs` の `pass_through` を `pub(crate)` 化して `test_fault` と共有(DSP law の単一実装) |
| Factory | `plugin_count()` 4→5、descriptor index 4 = `test_editor`(`org.ruviform-test.clap-test-editor`、vendor `"Ruviform Test"`、version `"0.0.0"`)。既存 4 class の descriptor/id/挙動は不変 |
| Dependencies | `clack-extensions` に `gui` feature 追加(Spec 承認済み)。`[target.'cfg(windows)'.dependencies] windows-sys = "0.61"`(`Win32_Foundation`/`Graphics_Gdi`/`System_LibraryLoader`/`UI_WindowsAndMessaging` — VST3 fixture と同一 feature 集合、workspace lock 既存 `windows-sys 0.61.2`、Coordinator 承認済み) |
| Host bridge | `ruviform-plugin-clap/src/host.rs` 等 production crate は **zero-diff** — `open_gui`/`close_gui` の既存 path が fixture をそのまま観測 |

## 実機実行証拠(Windows)

実行 tuple:

| 項目 | 値 |
| --- | --- |
| TestID | rf630-clap-test-editor-win |
| OS | Windows(x64、Git Bash 経由の mise/cargo) |
| toolchain | rustc 1.99.1 / cargo 1.99.1(mise pin)、edition 2024 |
| format | CLAP cdylib `ruviform_plugin_test_clap.dll`(`target/debug/deps/` に生成、`tests/host_path.rs` が `current_exe`→`deps/` 相対で解決) |
| plugin binary/version | `test_editor` 0.0.0 / vendor `Ruviform Test` / id `org.ruviform-test.clap-test-editor` |
| process boundary | L0 in-process(`ClapLibrary::load` → dlopen → `clap_entry` → factories → `ErasedPlugin`)。L1 out-of-process は NotRun(後続 slice) |
| command | `mise run test ruviform-plugin-test-clap` + `-- --ignored`(real-HWND) |
| exit | 0 |

観測された実効果(`tests/host_path.rs` 15 test 全 pass + `editor.rs` unit tests):

- descriptor/factory pin — `descriptors().len()==5`・`factories().len()==5`、index 4 = `test_editor` の id/name/vendor/version/features
- `params()` が空(paramless)・pass-through audio が stereo block で bitwise copy・foreign `ParamValue` event は drop
- `open_gui(ParentLease::new(0,0))` — opaque payload が real HWND でないため `IsWindow` gate で defined rejection(Windows: `Err(PluginFailed)`、非 Windows: `Err(Unsupported)`)。失敗後も `stop_processing`/`deactivate`/`terminate` が通常収束
- 非 editor class(`test_gain`/`test_fault`/`test_stateful`/`test_instrument`)の `open_gui` → `Unsupported`
- `#[ignore]`d `set_parent_creates_real_child_hwnd`(Windows 実機で `--ignored` 実走、pass)— `IsWindow`/`GetParent`/`GetWindowLong(GWL_STYLE)` で `WS_CHILD` 関係・parent 関係・live window status を観測、二重 attach rejection、`destroy` 後 `IsWindow==false` に収束

## 検証(実 command/exit)

| command | 結果 |
| --- | --- |
| `cargo check --locked --all-targets -p ruviform-plugin-test-clap` | exit 0 |
| `mise run lint ruviform-plugin-test-clap` | exit 0(`clippy -- -D warnings` clean) |
| `cargo test --locked -p ruviform-plugin-test-clap` | exit 0 — 57 passed + 1 ignored + 0 failed |
| `cargo test --locked -p ruviform-plugin-test-clap -- --ignored` | exit 0 — `set_parent_creates_real_child_hwnd` pass(Windows 実機) |
| `mise run check-l1-real-plugin` | exit 0 |
| `mise run lint-l1-real-plugin` | exit 0 |
| `mise run test-l1-real-plugin` | exit 0(adapter e2e / test-plugin e2e / fault e2e 8+8 / stateful e2e 2+2 全 pass) |
| `mise run test-rt` | exit 0 — workspace `rt_` tests 全 pass |
| `mise exec -- prek run --all-files` | exit 0(全 hook pass) |
| `cargo fmt --check`(fmt lane 経由) | exit 0 |

## 未検証(NotRun / Unsupported)

| 項目 | 状態 | 理由・後続 |
| --- | --- | --- |
| real parent HWND + message pump 経由の `open_gui` **success** path | NotRun | `scarecrow173/ruviform#631` real-dock slice の責務。現行 lane は VST3 cdylib のみ、CLAP leg は lane 拡張待ち。本 slice は plugin 側 `set_parent` が実 child HWND を生成することまでを検証 |
| L1 `OpenGui`/`CloseGui` e2e | NotRun/Unsupported | `plugin-host/src/l1/mod.rs` で `HostError::Unsupported` by design の本番側 deferral |
| floating window(`is_floating=true`)・`set_transient`/`suggest_title` 実 attach | NotRun/Unsupported | #389 D7 系 deferred — fixture は defined rejection/default で正直に応答 |
| `test_stateful`/`test_instrument` editor variants | NotRun | 本 slice は `test_editor` class のみ。両 class への GUI 追加は将来要件が出た場合のみ |
| 外部 SDK validator(clap-validator 等) | NotRun/Deferred | license・環境確保後の別 slice。外部権威なしに ABI 適合を主張しない |
| macOS/Linux build・実走 | NotRun | 本 slice は Windows OS-L1 のみ。`cfg!(windows)` gate + shim で非 Windows build は compile green を維持するが runtime GUI は未検証 |
| consumer oracle(`scarecrow173/synvelis#101`/`#127`/`#128`/`#176`、`scarecrow173/audio-lens#127`)照合 | NotRun | Issue 受入の残項目、後続 slice |
| Preset 面(AC-2 残解釈)・artifact digest 記録(AC-5 残り) | NotRun | Issue 受入の残項目、後続 slice/Deferred |
| MSRV lane(`*-msrv`) | NotRun | ユーザー指定で本 slice 対象外 |
| `*-full` lane | NotRun | 日常検証は変更範囲比例、Full CI に委譲 |

## 独立実装 cross-check の記録

- plugin 側実装(descriptor/lifecycle/ports/gui/DSP)は `clack-* =0.2.0` binding + `ruviform-plugin-core`/`ruviform-audio-core` neutral contracts 上に直接実装。`ruviform-plugin-clap` の `pub(crate)` internals は不使用。
- host 側検証は `ruviform-plugin-clap` の production `ClapLibrary` bridge(dev-dep `host` feature)経由で実施 — Fake Plugin Port 非経由。adapter 本体ソースは変更なし(zero-diff)。
- `clack-host`(dev-dep)の safe fixture は plugin 側 ABI を in-process で駆動する unit/lifecycle テスト専用。本番 host 検証の主証拠は `tests/host_path.rs`。

## test-only / 配布境界

- 本 crate は test fixture であり出荷物ではない。production crate は本クレートに依存しない。
- cdylib 成果物は `target/` 配下の test fixture であり、製品配布物・AudioLens への新依存を強制しない。
- `windows-sys` は `[target.'cfg(windows)'.dependencies]` のみ — 非 Windows consumer/ビルドに Win32 依存を強制しない。
