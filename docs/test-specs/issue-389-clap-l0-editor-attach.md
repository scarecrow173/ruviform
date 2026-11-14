---
id: "TSPEC-034XAY5OnZ3JrUirJ49Rqh"
type: "test-spec"
status: "approved"
title: "Issue #389 CLAP/L0 実外部 Editor attach テスト仕様（adapter headless 契約 + 実 binary manual lane）"
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
        path: "crates/ruviform-plugin-clap/src/host.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-clap-l0-editor-attach.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-389-clap-l0-editor-attach.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-389-clap-l0-editor-attach.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-389-clap-l0-editor-attach.md"
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

# Issue #389 CLAP/L0 実外部 Editor attach テスト仕様

## Purpose

`ClapPlugin::open_gui`/`close_gui`（design D1–D6）の契約を、
**決定的な bundled fixture**（`clap.ext.gui` を実装しない既存
gain fixture — headless/CI 対象）と **実 binary manual lane**
（workspace fixture `.clap` または実第三者 `.clap` — CI 対象外、
real-OS-only）で分けて検証する。fixture 契約はサードパーティ
互換の代理にしない。manual lane の結果は PASS/Fail/NotRun/
Unsupported を独立に記録する。

## Feature

- CLAP adapter `open_gui`/`close_gui` の状態機械・error 写像
  （design D1/D3/D6）。
- `clap.ext.gui` extension 不在・API 非対応の honest
  `Unsupported` 分類。
- PoC `clap` feature lane の load/instantiate/session 経路と
  editor HWND 観測（design D5）。

## Headless 契約（`cargo test -p ruviform-plugin-clap --features host`）

| ID | Rule | 手順 | 期待 |
| --- | --- | --- | --- |
| CA-01 | dormant は不正状態 | `create()` 直後（`instance=None`）で `open_gui` | `Err(HostError::PluginFailed)`（`Unsupported` ではない — hook 未到達の不正状態、VST3 VA-02 同型） |
| CA-02 | gui ext 不在は honest Unsupported | `init` 済 fixture（`clap.ext.gui` 非搭載）で `open_gui` | `Err(HostError::Unsupported)`（stub 成功を pin しない） |
| CA-03 | close 未 open は honest refusal | `open_gui` 未成功の instance（dormant/init 済とも）で `close_gui` | `Err(HostError::Unsupported)` |
| CA-04 | view なし Drop は既存 descent を壊さない | `prepared_plugin`（Processing）を `open_gui` せず drop | 既存 teardown テスト群 green 維持（GUI teardown は defined no-op） |
| CA-05 | DSP/RT 非回帰 | 既存 host.rs テスト群（gain passthrough/params/transport/drop/RT alloc-free） | 全 green（`open_gui` 追加で DSP 経路不変） |
| CA-06 | 二重 open・実 attach 成功・gui_seq | `clap.ext.gui` 実装 plugin での open→close→open | 各回 distinct `GuiHandle`（manual lane — fixture では到達不能、code review + fixture で代替） |

## Manual 実機 lane（`RVF_POC_CLAP_PATH` + `--features gpui,clap`）

対象: workspace fixture `ruviform_plugin_clap` cdylib（実
`.clap` binary・`clap.ext.gui` 非搭載）または実第三者 `.clap`。
lane 未実行・load 失敗・editor 非搭載は `NotRun`/`Fail`/
`Unsupported` を個別記録（成功偽装しない）。

| ID | Rule | 手順 | 観測 |
| --- | --- | --- | --- |
| CR-01 | module load | `ClapLibrary::load(path)` | `clap_path`/`clap_size`/`clap_descriptors`/`clap_plugin`/`clap_load=fail error=…`。fail → exit 3 |
| CR-02 | descriptor scan | `descriptors()` | plugin id/name/vendor/version/count が実値 |
| CR-03 | session open → attach 試行 | `session.open(Docked)` → `open_gui` | fixture: `open=fail error=…Unsupported`（gui ext 不在の honest 分類）。実 gui plugin: `open=ok` + `first_child_of(container)` で editor HWND 観測 |
| CR-04 | geometry/hide/show/focus/close/release | slot `set_geometry`/`set_visibility`/`request_focus`/session close | 実 gui plugin のみ観測（fixture では open 失敗で stage Done — 到達した範囲のみ記録） |
| CR-05 | 二重 open（実 plugin） | open 済 session で再度 open 相当 | `PluginFailed`（session 側 `begin_open` が先に拒否 — adapter 層は code review + fixture で代替） |

証拠 tuple（`evidence/rf389-clap-editor/report.md` へ記録）:
TestID / OS / GPU / GPUI revision / format=clap / plugin path・
sha256・name・version・vendor / process boundary=L0
in-process / parent chain（GPUI top HWND → container HWND →
editor HWND）/ command / exit code / 3 repo SHA（ruviform・
audio-lens・synvelis — AL/SV 非結合なら該当なしと記録）/
証跡 path。

## 明示的に対象外（NotRun 記録）

- 実 GUI attach の PASS — test_editor fixture（#630 charter）
  または実第三者 `.clap` binary の後続 slice
- editor への keyboard/IME/input 実到達（container focus は
  代理にしない）
- floating window・`set_transient`/`suggest_title`・size
  negotiation（`get_size`/`set_size`/`adjust_size`/
  `can_resize`/`set_scale`）
- host 側 `clap.ext.gui` callback（`HostGuiImpl` — 未宣言）
- `save_state`/`restore_state`（`state` extension 未実装）
- L1 out-of-process GUI（`L1EditorOps` = 全 op Unsupported 維持）
- macOS/Linux attach、AU adapter
- GPUI dock layout 管理下の実 Dock PASS（RF387 側の Gate）

## Review 観点

- `Unsupported`/`PluginFailed`/`Fail`/`NotRun` の分類が design
  D6 表と一致するか
- `destroy` が instance teardown（deactivate/`plugin.destroy`）
  より先（D3）であるか
- `create` 成功後の失敗経路で `destroy` が必ず呼ばれるか
- `parent.payload()` の re-widen が adapter 境界内に閉じているか
- evidence が実機観測値のみで構成され、予測値の混入がないか
