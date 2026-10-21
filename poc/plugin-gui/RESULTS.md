# Issue #30 プラグイン GUI PoC — 結果集約

## AC → 実験 → エビデンス

| AC | 実験 | 結果 | エビデンス |
| --- | --- | --- | --- |
| AC1 attach | E1 `build_as_child` を非 tao 生成・外部 `GetMessage` ループの素 Win32 親 HWND へ | **passed** | `e1_attach` — `docs/impl/exp/plugin-gui-attach-e1-e2.jsonl` |
| AC2 表示・resize・IPC | E1 内 `PageLoadEvent::Finished`・`set_bounds(600x400)`・`window.ipc.postMessage` roundtrip | **passed**（全実験 28ms で完了） | `e1_render`・`e1_resize`・`e1_ipc` |
| AC3 規格対応付け | CLAP gui ext / VST3 IPlugView への写像 | **doc-eval** | 下表 |
| AC4 baseview/wxp 評価 | 文書評価 | **doc-eval** | 下表 |
| AC5 リソース | E2 working set delta | **passed** — +15.7MB（WebView2 共有プロセス含むプロセス内 delta） | `e2_mem_delta_bytes` |
| AC6 隔離 | `poc/plugin-gui/` 独立 workspace | **passed** | ルート `members`/`Cargo.toml` 非接触 |

## AC3 — 規格写像（文書評価）

| 規格 attach API | PoC 対応操作 | 適合性 |
| --- | --- | --- |
| CLAP `clap_plugin_gui.set_parent(clap_window{hwnd})` | `build_as_child(&HostWindow(hwnd))` — `HasWindowHandle` adapter で HWND を包む | 適合（実測） |
| CLAP `set_size(w,h)` | `WebView::set_bounds(Rect)` | 適合（実測） |
| CLAP `create/destroy`/`show`/`hide` | `WebView` 生成/Drop・親窓 ShowWindow | 適合（wry は WebView2 子窓のみ生成・親はホスト側） |
| VST3 `IPlugView::attached(parent, "HWND")`/`onSize`/`removed` | 同一 attach/bounds/Drop 写像 | 適合（文書） |

## AC4 — baseview / wxp（文書評価）

| 経路 | 評価 |
| --- | --- |
| **baseview** | プラグインウィンドウ抽象（親 HWND への子 attach を提供）だが WebView 機能を持たない — WebView 埋め込みには wry attach が別途必要。UI 方針「内側は WebView」と直交するため採否の主経路にはならない（オプション B の補助層としては有効） |
| **wxp** | wry 上流追従 + プラグインホスト向けライフサイクルパッチ同梱（Issue #28 評価済み）。本 PoC で素 wry が外部親 attach に成功したため、wxp パッチの必須性は低下。残論点はホスト DAW 固有のライフサイクル互換（実ホスト検証は本 PoC 範囲外） |

## 再現

```sh
cd poc/plugin-gui && cargo run -- --out <jsonl path>
```
