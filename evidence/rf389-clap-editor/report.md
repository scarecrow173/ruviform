# RF389 — 実外部 CLAP native Editor (Windows/L0) 証拠レポート

Run: `run_e9e66b833704` / Task `task_83458f4490d4` / branch `scarecrow173/rf389-clap-editor`。
本 increment は `ruviform-plugin-clap` host bridge の `ClapPlugin::open_gui`/`close_gui` を
`clack-extensions` 0.2.0 `gui` extension 経由で実装し、
`ruviform-poc-native-dock` の `clap` feature と `mise.toml` manual lane
`poc-clap-editor` で実 `.clap` モジュールの load→init→slot→lease→`open_gui`
分類までを検証した。workspace fixture は `clap.ext.gui` を実装しないため、
`open=Unsupported` が本 slice の正直な実測分類である。

## 対象 plugin binary

| key | value |
| --- | --- |
| plugin path | `target/debug/ruviform_plugin_clap.dll`（workspace fixture `ruviform-plugin-clap` cdylib、`--features host` build） |
| size | 486,400 bytes（`clap_size=486400` 実測） |
| sha256 | 未採取 — fixture は build 成果物であり commit pin されない。descriptor は下記 |
| scanned descriptor | id=`org.ruviform.clap-gain-poc` name=`Ruviform Gain PoC` vendor=`Ruviform` version=`0.1.0`（1 descriptor） |
| gui extension | **なし** — fixture は `clap.ext.gui` を実装しない（`PluginGui` extension 不在 → `open_gui` は `Unsupported`） |

## 実施コマンド / 環境

- `RVF_POC_CLAP_PATH='G:/orca/workspaces/ruviform/rf389-clap-editor/target/debug/ruviform_plugin_clap.dll' mise run poc-clap-editor`
  （正規入口: `mise.toml` `[tasks.poc-clap-editor]` — `cargo run -p ruviform-poc-native-dock --features gpui,clap`）
- host process: `ruviform-poc-native-dock.exe` x86-64 / Windows 11 / GPUI 0.2.2
- repo SHA: worktree base `3b8b73224f0f792d70f260e764a7b5b8f4c0aa4e` + 本 branch の変更（コミット SHA は PR 参照）
- consumer coupling: audio-lens / synvelis への直接結合なし（本 slice は `ruviform-plugin-clap` + `ruviform-poc-native-dock` のみ。consumer 影響分類は PR 本文参照）
- exit code: 0 / TestID: `poc-clap-gain-fixture-win11` / log: `evidence/rf389-clap-editor/poc-clap-gain-fixture-win11.txt`

## 結果（実測値）

| gate | result | evidence |
| --- | --- | --- |
| 実 `.clap` モジュール `LoadLibrary` + descriptor scan | **Pass** | `clap_descriptors=1`, `clap_plugin id=org.ruviform.clap-gain-poc name=Ruviform Gain PoC vendor=Ruviform version=0.1.0` |
| `Instantiate` + `Initialize` | **Pass** | `instance=PluginHandle{slot:0,generation:0}`, `initialize=ok`, `ready` |
| `declare_slot` + `lease_parent`（container HWND） | **Pass** | `declare_slot=ok index=0`, `lease_parent=ok container_hwnd=0xb60faa` |
| `open_gui` — `clap.ext.gui` 非実装の正直分類 | **Pass (Unsupported)** | `open=fail error=Ops(Unsupported)`, `done state=Some(Failed { cause: Unsupported })` |
| session Failed 状態の維持・clean exit | **Pass** | `exited`、クラッシュなし |

process boundary: **L0 in-process**（`ClapLibrary::load` → `PluginEditorSession` →
`NativeSurfaceSlot` lease → adapter `open_gui`）。parent chain:
`GPUI top HWND → container HWND (0xb60faa) → editor HWND`（editor HWND は
`clap.ext.gui` 非実装のため未生成 — chain は container まで実測）。
format: `clap` / GPUI revision: 0.2.2。

## headless / canonical 検証（CI-safe 経路）

- `cargo test -p ruviform-plugin-clap --features host`: `open_gui` 未 init→
  `PluginFailed`、fixture（`clap.ext.gui` 非実装）init 後 `open_gui`→
  `Unsupported`、未 open `close_gui`→`Unsupported`、既存 DSP/RT/state/host
  regression 全 pass。
- `mise run fmt-check` / `check` / `lint` / `test`（`ruviform-plugin-clap`
  default+host）、`check-l1-real-plugin` / `lint-l1-real-plugin` /
  `test-l1-real-plugin`、`test ruviform-plugin-host --test
  editor_session --test editor_session_driver`、`check` /
  `check --features clap` / `check --features gpui,clap`
  （`ruviform-poc-native-dock`）、`check-ui-driver-boundary`、`test-rt`:
  全て Pass（exit 0）。

## 未達 / 正直な分類

| gate | status | 理由 |
| --- | --- | --- |
| `clap.ext.gui` 実装 plugin での attach 成功（`create`→`set_parent`→`show`→`GuiHandle` mint） | **NotRun** | workspace fixture に `clap.ext.gui` なし。実 plugin binary または gui fixture が必要 — manual lane 再実行で検証可能 |
| open→close→open の distinct `GuiHandle` mint | **NotRun** | 同上 — attach 成功経路が未実行 |
| 実 editor HWND parenting / geometry / visibility | **NotRun** | 同上 |
| keyboard/IME 入力 routing | **NotRun** | container focus のみ確認。Editor への key/IME 配送は未実装・未検証 |
| L1 (cross-process) editor | **Unsupported** | 現行 L1 経路は GUI を Unsupported で拒否 — 黙って L0 に退化させていない |
| macOS/Linux CLAP gui（Cocoa/X11/Wayland） | **Unsupported** | `GuiApiType::WIN32` のみ実装（`EDITOR_GUI_API` Option gate） |
| floating window / `set_transient` / size negotiation | **Unsupported (out of scope)** | `is_floating=false` 固定、`get_size`/`set_size`/`can_resize`/`adjust_size`/`set_scale` 未実装（design D2/D7） |
| CLAP state save/restore | **Unsupported (out of scope)** | 本 slice 対象外 |
| 他社 `.clap` の広範互換 | **NotRun** | workspace fixture 1 binary のみ検証 |

## fixture 側回帰（CI-safe 経路）

- `open_gui` 未 init→`PluginFailed`、init 後 fixture `open_gui`→`Unsupported`、
  未 open `close_gui`→`Unsupported` を pin test で固定。
- `Drop` は gui teardown（`hide`+`destroy`）を processor/instance teardown より
  前に実行 — gui 非所持の既存 teardown 経路は非回帰。
- L1 runner / DSP / RT / state の既存 test は全て green（`test-l1-real-plugin`、
  `test-rt` 含む）。
