---
id: "DSC-630clapEditorVerbs01"
type: "discovery"
status: "final"
title: "Issue #630 CLAP editor verb 実バイナリ coverage — slice 調査記録"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/pull/754"
    - "G:/repositry/audio-products/.devin/dispatch-specs/rf630-residual.explore.json"
    - "docs/discovery/issue-630-clap-test-editor.md"
    - "docs/discovery/issue-630-abi-validator-rerun.md"
    - "evidence/rf630/abi-validator/run.json"
  derived-by:
    - "docs/specs/issue-630-clap-editor-verbs.md"
    - "docs/adr/issue-630-clap-editor-verbs.md"
  related:
    - "docs/discovery/issue-630-preset-discovery-reeval.md"
    - "docs/specs/issue-630-preset-discovery-reeval.md"
---

# Issue #630 CLAP editor verb 実バイナリ coverage — 調査記録

`ruviform#754`（merge `0450f92f`）で plugin-core `Plugin` trait に
`set_gui_size`/`set_gui_visibility`/`set_gui_parent` が追加され、
CLAP adapter `host.rs` に verb 実装（can_resize gate → `set_size`、
hide/show、set_parent）が production chain として着地した。しかし
実 CLAP cdylib 経由の coverage はゼロで、現行の検証面は
plugin-host scripted fake（`editor_session.rs` 系）と VST3 real lane
（`rf631` real-editor-resize/redock/visibility）のみ。本 slice は
このギャップを `tests/host_path.rs` の実バイナリ試験で埋め、
併せて `ruviform#754` で stale 化した外部 ABI validator 証拠の
digest 簿記を更新する。

## 確認した事実

- production verb 実装（`crates/ruviform-plugin-clap/src/host.rs`）:
  `close_gui` は GUI 未 open で `Err(Unsupported)`、open 済みは
  best-effort `hide -> destroy`。`set_gui_size` は GUI 未 open で
  `Unsupported`、open 済みは `can_resize` gate（false なら
  `Unsupported`）を通って `set_size` を呼ぶ。`set_gui_visibility`/
  `set_gui_parent` は GUI 未 open で `Unsupported`、open 済みは
  `hide`/`show`/`set_parent` の失敗を `PluginFailed` に写す。
  全 verb は non-RT・親 HWND の message pump 所有 thread 前提と
  明記されている。
- trait default（`crates/ruviform-plugin-core/src/plugin.rs`）:
  `set_gui_size`/`set_gui_visibility`/`set_gui_parent` は default で
  `Err(HostError::Unsupported)`。`open_gui`/`close_gui` も default
  `Unsupported` で、adapter が override する。
- CLAP fixture（`crates/ruviform-plugin-test-clap/src/editor.rs`）:
  `test_editor` は `clap.ext.gui` を宣言し Win32 embedded のみ
  対応。`GuiState { created, hwnd }` で `set_parent` が実
  `WS_CHILD` HWND を生成、`show`/`hide` が可視性を切替、
  `destroy` は冪等。現状は **fixed-size**: `get_size` は固定
  640x480、`can_resize=false`、`set_size` は defined rejection、
  `set_parent` の再 embed は `gui already embedded` で拒否、
  `set_scale` は有限正数のみ受理、`set_transient` は常に拒否。
  `set_parent_creates_real_child_hwnd`（unit test）が hidden
  top-level parent + `#[ignore]` 実走の先例。
- VST3 対称参照（`crates/ruviform-plugin-test-vst3/src/editor_view.rs`、
  `ruviform#754` で変更）: `canResize=kResultTrue`、`onSize` が
  child HWND を実 resize し attach-generation を title に刻印。
  PR #754 は verb を end-to-end で動かすため fixture を
  resize-capable にした先例。
- host_path 既存構造（`tests/host_path.rs`）:
  `load_library()` が crate 自身の cdylib を `deps/` から
  `ClapLibrary::load` し、`prepared_plugin_at(index)` が
  factory scan → `create -> init -> prepare -> start_processing` で
  `Box<dyn ErasedPlugin>` を返す。`test_editor` は index 4。
  `editor_open_gui_rejects_opaque_parent_lease` が opaque lease
  （`ParentLease::new(0,0)`）で Windows `PluginFailed` /
  non-Windows `Unsupported` を確認済み。
- clap-validator は gui extension の test を持たない（run.json の
  leg 一覧に gui 系 test なし）。fixture の resize 化は validator
  verdict に影響せず、artifact SHA-256/repo.head の簿記更新のみが
  必要。
- provisioned validator は両方存在し SHA-256 が run.json 記録と一致:
  clap-validator pin `47e2424`（`pin-47e2424/clap-validator.exe`、
  SHA-256 `b4bd6...afe26`）、VST3 SDK validator
  （`vst3sdk-build/bin/Release/validator.exe`、SHA-256
  `5889a...a4fd`）。`mise run test-plugin-abi-validator` の
  `--evidence-dir` 既定は `evidence/rf630/abi-validator/`。

## 検討した代替案

1. **fixed-size decline 維持**: fixture を変更せず、ignored 実機試験で
   `set_gui_size -> Err(Unsupported)`（can_resize=false の正直な
   decline）を観測する。変更面が `tests/host_path.rs` のみに
   絞れる利点があるが、`set_size` 呼出し経路（can_resize gate 通過後の
   実 `set_size`）は coverage されず、VST3 側が resize-capable に
   なった非対称が残る。
2. **resize-capable 化（採用）**: `can_resize=true` + `set_size` が
   child HWND を実 resize するよう fixture を拡張し、ignored 実機試験で
   `GetWindowRect` round-trip の実 resize oracle を観測する。
   `ruviform#754` が VST3 fixture に対して取ったのと同じ判断で、
   verb の success 経路まで実バイナリで証明できる。変更面は
   `src/editor.rs`（lease 内・決裁済み）+ `tests/host_path.rs` +
   `README.md` に限定される。

## 結論（決裁）

**resize-capable を採用する。** `ruviform#754` が VST3 fixture を
resize-capable にしたのと同じ理由 — verb chain の success 経路を
実バイナリで検証するには fixture 側の resize 受理が必要。decline
経路は `can_resize` gate を通らない `Unsupported` として非 ignore
層でも観測可能だが、success 経路（`set_size` → 実 HWND resize）は
fixture が受理しない限り原理的に coverage できない。VST3/CLAP の
fixture 対称性も回復する。

`adjust_size`/`get_resize_hints` は制約を宣言しない trait default
（`None`）を維持し、`set_scale`/`set_transient` の既存 semantics は
変更しない。re-embed 拒否（`gui already embedded`）は fixture 仕様
どおり維持し、`set_gui_parent` の `PluginFailed` 伝播を試験する。

## 残る未解決事項

- 実 Window/画素/Input/Focus/Popup/Close/再Open・message pump・
  real-dock の e2e lane は `ruviform#631` 責務。本 slice の
  `#[ignore]` 実機試験は hidden top-level parent への attach と
  verb 観測に留め、AC4 gate は動かさない。
- DComp 既知 Fail の解消判定は `ruviform#532` 責務。
- validator が adapter/fixture 欠陥を指摘した場合は本 slice で
  修正せず証拠つきで報告（`ruviform#722`/`ruviform#732` 先例）。
