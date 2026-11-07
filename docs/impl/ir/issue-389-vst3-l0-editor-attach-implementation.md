---
id: "IMPL-qCX7OwfBENOC4OHbkUSGZ"
type: "impl"
status: "completed"
title: "Issue #389 VST3/L0 実外部 Editor attach 実装記録"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "evidence/rf389-vst3-editor/report.md"
      - type: "file"
        path: "evidence/rf389-vst3-editor/poc-vst3-vinyl-win11.txt"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/issue-389-vst3-l0-editor-attach.md"
  implemented-by: []
  depends-on:
    - "issue-389-vst3-l0-editor-attach.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../designs/issue-389-vst3-l0-editor-attach.md"
    - "../../plans/implement-issue-389-vst3-l0-editor-attach.md"
    - "../../test-specs/issue-389-vst3-l0-editor-attach.md"
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
---

# Issue #389 VST3/L0 実外部 Editor attach 実装記録

設計・計画・テスト仕様: `docs/designs|plans|test-specs/issue-389-vst3-l0-editor-attach.md`。

## 実装したもの

- `Vst3Plugin::open_gui` — `IEditController::createView("editor")` →
  `isPlatformTypeSupported(kPlatformTypeHWND)` →
  `setFrame(BridgePlugFrame)` → `attached(parent HWND)`。
  `GuiParentToken`→HWND の変換はこの adapter 境界のみ。
- `Vst3Plugin::close_gui` / `close_view` / `Drop` — `removed()` →
  view/frame release → `disconnect(IConnectionPoint)` → `terminate`。
- `BridgePlugFrame`（`IPlugFrame`）— `resizeView` は defined refusal
  `kResultFalse`（container 幾何は slot 側所有）。
- `BridgeMessage`（`IMessage`）+ `BridgeAttributeList`
  （`IAttributeList`）+ `BridgeHostApplication::createInstance` の
  CID dispatch — 実 plugin 互換に必須だった（下記「実機発見」）。
- distributed plugin 向け `IConnectionPoint` 双方向 connect /
  Drop での disconnect。
- `ruviform-poc-native-dock` `vst3` feature — `RVF_POC_VST3_PATH` で
  実外部モジュールを `Vst3Library::load` → factory scan →
  `register_factory` → `Instantiate`/`Initialize` → session open。
  `EnumChildWindows` で editor HWND を観測。probe mode は既定維持。
- `mise.toml` manual lane `poc-vst3-editor`。

## 実機発見（設計追記）

- Vinyl は distributed (`single_component=false`)。`IConnectionPoint`
  未接続では `createView` が null → `Unsupported`。接続必須。
- 接続時の `notify` が `IMessage` を `IHostApplication::createInstance`
  に要求。`kNotImplemented` だと plugin 内部で null 参照 crash。
  `IMessage`/`IAttributeList` の host class 実装が実 plugin 互換の
  前提条件だった。

## 検証結果

- headless: `cargo test -p ruviform-plugin-vst3 --features host` 51 pass。
  `open_gui` 未 init→`PluginFailed`、fixture `createView=null`→
  `Unsupported`、close 未 open→`Unsupported` を pin。
- 実機 manual lane（Vinyl 1.12.0, Windows 11, GPUI 0.2.2）:
  `open=ok`、editor HWND parenting/geometry/visibility/close/release
  全 Pass — `evidence/rf389-vst3-editor/`。
- canonical: `mise run fmt-check`/`check`/`lint`/`test` 全 green。
- 分類: keyboard/IME=NotRun、Dock PASS=NotRun（RF387 未接続）、
  L1/CLAP/AU/他 OS=Unsupported、他社 VST3 広範互換=NotRun。
