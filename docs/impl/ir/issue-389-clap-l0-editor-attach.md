---
id: "IMPL-034XBBr5HDcEsujT6coC8S"
type: "impl"
status: "completed"
title: "Issue #389 CLAP/L0 実外部 Editor attach 実装記録"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc/dock.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc/dock_windowing.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "evidence/rf389-clap-editor/report.md"
      - type: "file"
        path: "evidence/rf389-clap-editor/poc-clap-gain-fixture-win11.txt"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "../../tasks/issue-389-clap-l0-editor-attach.md"
  implemented-by: []
  depends-on:
    - "../../tasks/issue-389-clap-l0-editor-attach.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../designs/issue-389-clap-l0-editor-attach.md"
    - "../../plans/implement-issue-389-clap-l0-editor-attach.md"
    - "../../test-specs/issue-389-clap-l0-editor-attach.md"
    - "../../designs/issue-389-vst3-l0-editor-attach.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [389]
  experiments:
    adopted: []
    rejected: []
---

# Issue #389 CLAP/L0 実外部 Editor attach 実装記録

## Summary

`ruviform-plugin-clap` に CLAP `clap.ext.gui` 経由の実外部 Editor
attach/detach 経路（`ClapPlugin::open_gui`/`close_gui`・
`PluginGui` extension・Win32 embedded HWND parenting）を実装し、
`ruviform-poc-native-dock` の `clap` feature と `mise.toml` manual
lane `poc-clap-editor` で実 `.clap` モジュールの load→init→slot→
lease→`open_gui` 分類までを検証した。workspace fixture は
`clap.ext.gui` を持たないため `open=Unsupported` が正直な実測値。
headless fixture と canonical 検証は全て green。

## Context

- 実施タスク: [../../tasks/issue-389-clap-l0-editor-attach.md](../../tasks/issue-389-clap-l0-editor-attach.md)
- 設計・計画・テスト仕様: `docs/designs|plans|test-specs/issue-389-clap-l0-editor-attach.md`
- 先行実装: `docs/designs/issue-389-vst3-l0-editor-attach.md`（D1–D7 踏襲）
- source: <https://github.com/scarecrow173/ruviform/issues/389>（Issue #389 のうち CLAP/L0 editor-attach slice）

## Implementation

### Changes

- `ClapPlugin::open_gui` — dormant（`instance.is_none()`）→
  `PluginFailed`、gui 既存の二重 open → `PluginFailed`、
  `get_extension::<PluginGui>()` が `None`（plugin が
  `clap.ext.gui` 未実装）または `is_api_supported` false →
  `Unsupported`。呼出列: `GuiConfiguration{WIN32, is_floating=false}`
  negotiation → `create` → `set_parent(Window::from_win32_hwnd(
  parent.payload()))` → `show`。`set_parent`/`from_win32_hwnd` は
  unsafe — adapter 境界内に SAFETY コメント付きで閉じ込め。
  `create` 成功後の `set_parent`/`show` 失敗は `destroy` で対称に
  巻き戻して `PluginFailed`。`GuiHandle` は attach 成功時にのみ
  mint（`gui_seq` wrapping counter）。
- `ClapPlugin::close_gui` — 未 open → `Unsupported`、open 済み →
  best-effort `hide` → `destroy` で `Ok(())`。
- `Drop` — gui teardown（`hide`+`destroy`）を processor
  `into_stopped`/deactivate より前に実行（VST3 D3 と同じ ordering）。
- platform gate — `EDITOR_GUI_API: Option<GuiApiType>` で Windows のみ
  `Some(GuiApiType::WIN32)`、非 Windows は `None` → `Unsupported`
  （VST3 `EDITOR_PLATFORM_TYPE` パターン踏襲）。
- `clack-extensions` `gui` feature を既存 `host` feature に追加
  （`clack-extensions/clack-host` と同じ dep/feat 配線。新依存なし、
  `Cargo.lock` 差分は `ruviform-poc-native-dock` →
  `ruviform-plugin-clap` の edge 追加のみ）。
- `ruviform-poc-native-dock` `clap` feature — `RVF_POC_CLAP_PATH` で
  `ClapLibrary::load` → descriptor scan → factory 登録 →
  `Instantiate`/`Initialize` → session open。`RVF_POC_CLAP_PLUGIN_ID`
  で複 plugin module の選択も mirror。probe mode は既定維持。
  `vst3_mode` を `external_mode` に rename（両 external lane 共通語彙）。
- `mise.toml` manual lane `poc-clap-editor`（`--features gpui,clap`）。
- pin test 更新 — `open_gui` 未 init→`PluginFailed`、fixture
  （`clap.ext.gui` 非実装）init 後 `open_gui`→`Unsupported`、
  未 open `close_gui`→`Unsupported` を pin。

### Decisions During Implementation

- `ParentLease::payload()` の re-widen（`*mut c_void`）は
  `open_gui` の adapter 境界のみに閉じ込め、上位層へ HWND を
  漏らさない（VST3 D1/D6 踏襲）。
- size negotiation（`get_size`/`set_size`/`can_resize`/`adjust_size`/
  `set_scale`）は `open_gui` に含めない（D2 — container 幾何は
  slot 側所有、resize callback は将来の slot/session チャネル）。
- `GuiHandle` mint は attach 成功時のみ。失敗経路では mint しない。
- `close_gui` は hide 失敗を許容して destroy を必ず呼ぶ
  best-effort 対称 teardown。
- PoC `clap` feature は opt-in（`RVF_POC_CLAP_PATH`）で probe
  mode は既定のまま維持。fixture は `clap.ext.gui` 非実装のため
  `open=Unsupported` が honest 分類 — 偽装成功しない。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- headless: `cargo test -p ruviform-plugin-clap --features host` —
  `open_gui` 未 init→`PluginFailed`、fixture `open_gui`→
  `Unsupported`、`close_gui` 未 open→`Unsupported`、既存 DSP/RT/
  state/host regression 全 pass。
- canonical 検証: `mise run fmt-check` / `check` / `lint` / `test`
  （`ruviform-plugin-clap` default+host、`ruviform-poc-native-dock`、
  `ruviform-plugin-host --test editor_session --test
  editor_session_driver`）、`check-l1-real-plugin` /
  `lint-l1-real-plugin` / `test-l1-real-plugin`、
  `check-ui-driver-boundary`、`test-rt` — 全て Pass。
- manual lane（real-OS）: `RVF_POC_CLAP_PATH=target/debug/
  ruviform_plugin_clap.dll mise run poc-clap-editor` —
  load→descriptor→init→declare_slot→lease_parent→`open=fail
  Ops(Unsupported)`→`done Failed{Unsupported}`→clean exit。
  fixture が `clap.ext.gui` 非実装のため Unsupported が正直な
  実測値。証拠: `evidence/rf389-clap-editor/`。

## Risks

- 実 `clap.ext.gui` 実装 plugin での attach 成功経路（`create`→
  `set_parent`→`show`）は workspace fixture に ext がなく headless
  では未検証 — manual lane で実 plugin binary が必要。
- `set_parent`/`from_win32_hwnd` は unsafe — adapter 境界で
  `ParentLease` の generation/lifetime 契約に依存。
- 非 Windows は `Unsupported` — macOS/Linux の CLAP gui
  （Cocoa/X11/Wayland）は本 slice 対象外。

## Follow-ups

- `clap.ext.gui` を実装する実 plugin（または test fixture）での
  open→close→open 実機検証（`GuiHandle` distinct mint 確認含む）。
- Issue #114/#115 の L1 cross-process GUI 境界は本 slice 対象外。
- #532/#388 Native Dock 画素受入は別 slice。
