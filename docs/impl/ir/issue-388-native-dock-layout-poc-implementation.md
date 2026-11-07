---
id: "IMPL-rf388DockLayoutPoc01"
type: "impl"
status: "completed"
title: "Issue #388 GPUI Panel/Tab レイアウト内 VST3 Editor 埋め込み PoC 実装記録（Windows / L0）"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc/dock.rs"
      - type: "file"
        path: "docs/test-specs/issue-388-native-dock-layout-poc.md"
      - type: "file"
        path: "evidence/rf388-native-dock-layout/report.md"
      - type: "file"
        path: "evidence/rf388-native-dock-layout/dock-vinyl-win11.log"
      - type: "file"
        path: "evidence/rf388-native-dock-layout/pixel-sample.ps1"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/main.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/designs/issue-388-native-surface-slot.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-389-vst3-l0-editor-attach-implementation.md"
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  defers: []
  deferred-by: []
metadata:
  issues: [388]
---

# Issue #388 GPUI Panel/Tab レイアウト内 VST3 Editor 埋め込み PoC 実装記録

## 実装内容

1. **`GpuiDriver::update_view`（`ruviform-ui-gpui`）**: mounted `ViewSpec` の差し替えを window 操作と同じ
   `Op` キューで適用する。window 単位の `Slot` に mounted `RvfView` の entity を保持し、`RvfView::update_spec` で
   `ViewHost::update` を呼んで再描画する。未知 window は `UiError::Host`。
   - 拒否された spec は前の木を残す（`ViewHost::update` 契約）。拒否理由は呼び出し元へ返らない（既知の制限）。
2. **Dock layout scenario（`ruviform-poc-native-dock`）**: `RVF_POC_SCENARIO=dock` で `poc/dock.rs` を起動。
   Panel → Tabs（Editor: `NativeSlot`、Other: 空の Panel）のレイアウトを `create_view_window` で開き、
   driver の canonical surface に slot を declare → `bind_native_slot` → projection 待ち → lease → session open。
   その後 tab 切替（hide/show）、window resize（grow/shrink）、focus、close、解放を順に実行し、
   各観測を `check=… result=pass|fail` の証拠行にする。exit は失敗数で決まる。
3. **plugin 起動の共通化**: `poc.rs` の plugin 登録・instantiate・initialize を `instantiate` に集約し、
   floating probe と Dock scenario で共有する。floating probe の挙動は変えていない。
4. **mise**: `poc-native-dock-layout` タスクを追加（`RVF_POC_SCENARIO=dock`）。

## 判断

- **canonical surface を使う**: 既存の floating probe は `GpuiSurfaceAdapter::new` による独立 ledger を使うが、
  renderer の投影は canonical surface にしか届かない。Dock scenario は canonical surface を使い、
  floating probe 側は変更しない（影響範囲を最小にする）。
- **tab 切替は host 駆動**: pointer 入力の自動化は範囲外（NotRun）。`ViewHost` の既存 reconcile 経路を通すことで、
  renderer の inactive-tab 抑止（既存テストで担保）と同じ経路を実機で確認する。
- **clip は検証しない**: slot が window 端まで伸びるため、editor の overhang は画面上に出ない。
  clip の Pass 根拠がないため NotRun とした（report.md 参照）。

## 検証

- `cargo build -p ruviform-poc-native-dock --features gpui,vst3`: 成功（dev profile）
- `cargo test -p ruviform-ui-gpui --features gpui --lib update_view`: 1 pass
- 実機: `RVF_POC_SCENARIO=dock` + Vinyl 1.12.0 で exit 0、`summary failures=0`。
  証拠: `evidence/rf388-native-dock-layout/dock-vinyl-win11.log`、`report.md`、`pixel-sample.txt`。

## 残課題（本記録の範囲外）

- plugin 側 resize（host→plugin `onSize` 経路）未配線。
- keyboard / IME / popup の editor 配送、pointer 操作による tab 切替、clip（ancestor overhang）検証。
- `DockWorkspace` 2-phase move との接続（re-dock / float / multi-window）。
- `update_view` の拒否理由を呼び出し元へ返す経路（result channel）の追加。
- macOS / Linux / CLAP / AU / L1 は Unsupported / NotRun のまま。
