# RF389 — 実外部 VST3 native Editor (Windows/L0) 証拠レポート

Run: `run_e9e66b833704` / Task `task_55dd89a91078` / branch `scarecrow173/rf389-vst3-editor`。
前段 PR #399 (main `e8adcc09aaa6444b00b9ba62c24fd2d217fb39f0`) の Slot/lease/EditorSession/GPUI container/probe 実装の上に、
本 increment は `ruviform-plugin-vst3` host bridge の `Vst3Plugin::open_gui`/`close_gui` を実装し、
実外部 VST3 モジュールの `IPlugView` を `PluginEditorSession`/`NativeSurfaceSlot` lease 経路で実 HWND に attach/detach する。

## 対象 plugin binary

| key | value |
| --- | --- |
| plugin path | `C:/Program Files/Common Files/VST3/iZotope/Vinyl.vst3` |
| size | 2,045,952 bytes |
| sha256 | `4bfe8ba1453fe6875c518327436c67e4d629a27f1f7384c01714664f8a5b1e92` |
| scanned descriptor | name=`Vinyl` version=`1.12.0` id=`69545356565a696e5a6f746f70652056` (1 class, Audio Module Class) |
| component/controller | distributed (`single_component=false` → `IConnectionPoint` 接続が必要だった) |

## 実施コマンド / 環境

- `RVF_POC_VST3_PATH='C:/Program Files/Common Files/VST3/iZotope/Vinyl.vst3' ./target/debug/ruviform-poc-native-dock.exe`
  （正規入口: `mise run poc-vst3-editor` — `--features gpui,vst3`）
- host process: `ruviform-poc-native-dock.exe` x86-64 / Windows 11 / GPUI 0.2.2 / RTX 4070 SUPER
- repo SHA: worktree base `e8adcc09aaa6444b00b9ba62c24fd2d217fb39f0` + 本 branch の変更（コミット SHA は PR 参照）
- exit code: 0 / TestID: `poc-vst3-vinyl-win11` / log: `evidence/rf389-vst3-editor/poc-vst3-vinyl-win11.txt`

## 結果（実測値）

| gate | result | evidence |
| --- | --- | --- |
| 実外部モジュール `LoadLibrary` + factory scan | **Pass** | `vst3_descriptors=1`, `vst3_plugin id=... name=Vinyl version=1.12.0` |
| `Instantiate` + `Initialize` | **Pass** | `instance=PluginHandle{slot:0,generation:0}`, `initialize=ok` |
| `createView("editor")` → `isPlatformTypeSupported(HWND)` → `setFrame` → `attached(container HWND)` | **Pass** | `open=ok caps_dock=Supported caps_close=Supported` |
| 実 editor HWND parenting | **Pass** | `editor_hwnd=0x20f1bd6 parent_token=0x1151820 parenting_ok=true editor_visible=true`（`EnumChildWindows` で観測） |
| `set_geometry` → editor rect 追従 | **Pass** | `(360,192,553,400) → (400,242,553,400)`（container 移動に実追従） |
| hide/show → editor visibility | **Pass** | `child_visible=false/true` |
| `request_focus` | **Pass (container のみ)** | `focus_hwnd=0x1151820` = container HWND。Editor 内部への key/IME ルーティングは未検証 |
| `close` → `removed()` → editor 破棄 + session `Closed` | **Pass** | `close=true state=Closed child_alive=false` |
| `release_lease`/`release_slot` → container 破棄 + revoke イベント | **Pass** | `container_alive=false`, `LeaseRevoked`/`Destroyed` イベント順序正常 |
| プロセス終了時 Drop（view→disconnect→terminate） | **Pass** | ログ完走 `exited`、クラッシュなし（debug 版の `debug_assert` 経路も通過） |

## 発見・対応した実 plugin 互換要件

1. **`IConnectionPoint` 接続は distributed plugin では必須** — 未接続では Vinyl の `createView` が null を返した。
   component↔controller を双方向 `connect` し、Drop で `disconnect` → `terminate` の順を遵守するよう実装。
2. **`IHostApplication::createInstance` は `IMessage`/`IAttributeList` を提供する必要がある** —
   接続時の `notify` が `IMessage` を要求し、未実装 (`kNotImplemented`) では plugin 側が null 参照して crash (segfault) した。
   `BridgeMessage`/`BridgeAttributeList` を実装し `createInstance` で CID dispatch。
3. `IPlugFrame::resizeView` は現状 `kResultFalse`（正直な拒否）— container 幾何は slot 側所有のため。
   plugin 主導 resize の上流経路は未実装（将来の slot/session resize 要求チャネルで対応）。

## 未達 / 正直な分類

| gate | status | 理由 |
| --- | --- | --- |
| GPUI main dock 内 layout/clip/tab 管理下での Native Dock PASS | **NotRun** | RF387 の Dock widget は別 owner・未接続。本 increment は slot container への実 attach まで |
| keyboard/IME 入力 routing | **NotRun** | container focus のみ確認。Editor への key/IME 配送は未実装・未検証 |
| L1 (cross-process) editor | **Unsupported** | 現行 L1 経路は GUI を Unsupported で拒否 — 黙って L0 に退化させていない |
| CLAP/AU editor | **Unsupported** | スコープ外のまま維持 |
| macOS/Linux editor attach | **Unsupported** | `kPlatformTypeHWND` のみ実装 |
| plugin 主導 resize (`resizeView`) | **Unsupported (defined refusal)** | `kResultFalse` で応答 — 実装済みだが受理しない |
| 他社 VST3 の広範互換 | **NotRun** | Vinyl 1 バイナリのみ検証 |

## fixture 側回帰（CI-safe 経路）

- `cargo test -p ruviform-plugin-vst3 --features host`: 51 pass — `open_gui` 未 init→`PluginFailed`、
  bundled fixture `createView=null`→`Unsupported`、close 未 open→`Unsupported` を pin。
- `mise run fmt-check` / `check` / `lint` (`clippy -D warnings`) / `test` (workspace 全 suite 0 fail): 全 Pass。
  （`ruviform-plugin-host` の L1 video test が 1 度だけ並列 flake を起こしたが、単独・全体再実行とも Pass。
  本 increment 非対象の既存 flake。）
