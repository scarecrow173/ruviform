---
id: "TESTSPEC-bnIEkLgoxp7oIJxODUlJnj"
type: "test-spec"
status: "approved"
title: "Issue #389 VST3/L0 実外部 Editor attach テスト仕様（adapter headless 契約 + 実 binary manual lane）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/host.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-389-vst3-l0-editor-attach.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
    - "docs/specs/issue-389-plugin-editor-session.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 VST3/L0 実外部 Editor attach テスト仕様

## 目的

`Vst3Plugin::open_gui`/`close_gui`（design D1–D6）の契約を、
**決定的な bundled fixture**（`createView` が null を返す既存
gain fixture — headless/CI 対象）と **実外部 module manual lane**
（installed `Vinyl.vst3` — CI 対象外）で分けて検証する。
fixture 契約はサードパーティ互換の代理にしない。manual lane の
結果は PASS/Fail/NotRun/Unsupported を独立に記録する。

## Headless 契約（`cargo test -p ruviform-plugin-vst3 --features host`）

| ID | Rule | 手順 | 期待 |
| --- | --- | --- | --- |
| VA-01 | 二重 open は拒否 | `init` 済 fixture で `open_gui(token)` → `Unsupported`（fixture は null view）— 実 attach 成功 plugin での二重 open は実機 lane で担保 | fixture: `Unsupported`。実 plugin 二重 open → `PluginFailed`（manual） |
| VA-02 | pre-init は不正状態 | `create()` 直後（`phase=Dormant`、controller 未存在）で `open_gui` | `Err(HostError::PluginFailed)`（`Unsupported` ではない — hook 未到達の不正状態） |
| VA-03 | null view は honest Unsupported | `init` 済 fixture（`createView`→null）で `open_gui` | `Err(HostError::Unsupported)` |
| VA-04 | close 未 open は honest refusal | `open_gui` 未成功の instance で `close_gui` | `Err(HostError::Unsupported)` |
| VA-05 | view なし Drop は既存 descent を壊さない | `prepared_plugin`（Processing）を `open_gui` せず drop | 既存 `drop_mid_processing_completes_teardown` green 維持 |
| VA-06 | DSP 非回帰 | 既存 host.rs テスト群（gain passthrough/params/transport/drop） | 全 green（`open_gui` 追加で DSP 経路不変） |
| VA-07 | gui_seq counter | 実 plugin で open→close→open | 各回 distinct `GuiHandle`（manual lane） |

## Manual 実機 lane（`RVF_POC_VST3_PATH` + `--features gpui,vst3`）

対象: `C:\Program Files\Common Files\VST3\iZotope\Vinyl.vst3`。
未インストール・load 失敗・editor 非搭載は `NotRun`/`Fail`/
`Unsupported` を個別記録（成功偽装しない）。

| ID | Rule | 手順 | 観測 |
| --- | --- | --- | --- |
| VR-01 | module load | `Vst3Library::load(path)` | `vst3_path`/`vst3_size`/`vst3_descriptors`/`vst3_plugin`/`vst3_load=fail error=…`。fail → exit 3 |
| VR-02 | descriptor/params scan | `descriptors()`/`params()` | plugin name/vendor/version/id/params count が実値 |
| VR-03 | session open → 実 attach | `session.open(Docked)` → `open_gui` → `EnumChildWindows(container)` | `open=ok`、editor HWND 実在、`parent_of(editor)==container` |
| VR-04 | geometry | editor rect 取得 → container を plugin preferred size へ `set_geometry` → rect 再読 | 実 rect 列を記録（追従しない場合も実値記録 — `onSize` 未通知は既知 Unsupported） |
| VR-05 | hide/show | slot `set_visibility(Hidden|Visible)` | `IsWindowVisible(editor)` の実追従 |
| VR-06 | focus | slot `request_focus` | `GetFocus()` の実値 — container 到達のみでもそのまま記録（editor 入力は別 NotRun） |
| VR-07 | close → detach | `session.close` → `close_gui` → `removed()` | editor HWND が `IsWindow` false（plugin 側が window を破棄） |
| VR-08 | release | `release_lease`/`release_slot` | container 破棄・slot event 列 |
| VR-09 | 二重 open（実 plugin） | open 済 session で再度 `open_gui` 相当（`open` は session が拒否 → 直接 `plugin.open_gui` 非経路。adapter 層の `view.is_some()` 分岐は unit 経路カバー不能のため coverage は code review + fixture で代替） | `PluginFailed`（記録のみ。session 側は `begin_open` が既に拒否する） |

証拠 tuple（`evidence/rf389-vst3-editor/report.md` へ記録）:
TestID / OS / GPU / GPUI revision / format=vst3 / plugin path・
sha256・name・version・vendor / process boundary=L0 in-process /
parent chain（GPUI top HWND → container HWND → editor HWND）/
command / exit code / 3 repo SHA（ruviform・audio-lens・synvelis
— AL/SV 非結合なら該当なしと記録）/ 証跡 path。

## 明示的に対象外（NotRun 記録）

- editor への keyboard/IME/input 実到達（D7 — container focus は
  代理にしない）
- `onSize`/`checkSizeConstraint` の host→plugin 通知、plugin-
  initiated resize roundtrip（`resizeView` は `kResultFalse` 拒否）
- L1 out-of-process GUI（`L1EditorOps` = 全 op Unsupported 維持）
- CLAP/AU adapter、`.vst3` directory bundle、macOS/Linux attach
- GPUI dock layout 管理下の実 Dock PASS（RF387 側の Gate）

## Review 観点

- `Unsupported`/`PluginFailed`/`Fail`/`NotRun` の分類が design D6
  表と一致するか
- `removed()` が release/terminate より先（D3）であるか
- COM 参照が open 失敗時にリークしないか
- evidence が実機観測値のみで構成され、予測値の混入がないか
