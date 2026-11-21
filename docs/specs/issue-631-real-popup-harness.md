---
id: "SPEC-01M7Y96P5ACBJBBKRPMWVQJNQC"
type: "spec"
status: "approved"
title: "Issue #631 real-editor-popup — Issue #630 AC4 共有残課題: VK_P popup toggle を production input path で駆動し top-level WS_POPUP HWND で検証する real-dock case"
created: "2026-11-21"
updated: "2026-11-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-editor-popup — Issue #630 AC4 共有残課題: VK_P popup toggle を production input path で駆動し top-level WS_POPUP HWND で検証する real-dock case

## What Should Be Built

`rf631.plugin-dock-real` に 29 番目の case
`real-editor-popup` を追加する。`test_editor` fixture の
plugin-owned popup（Issue #855 の `VK_P` toggle 契約）を
production chain で open/close し、その **実 HWND** を
OS 観測で検証する host harness:

```
session.send_key(GuiKeyEvent{Down, 'p', 0x50})
→ InProcessEditorOps → HostCommand::SendGuiKey
→ Vst3Plugin::send_gui_key → IPlugView::onKeyDown
→ popup_toggle → win32::popup CreateWindowExW / win32::destroy
→ title event log に po:{seq} / pc:{seq}
→ lane が GetWindowTextW + EnumWindows + GWL_STYLE で検証
```

変更は `crates/ruviform-plugin-dock-test`（+その tests/
evidence/docs）に閉じる。fixture・plugin-host・production
crate は変更しない。Issue #630 本体の consumer 実走 gate
（AC2 実行面）は本 slice の範囲外であり、`Refs #631` で
Issue を閉じない。

## 判定 oracle

non-pixel editor op family（`editor_op_body` 写像）に従い、
`Pass` は以下の **すべて** が成立したときだけ:

1. **attach 契約**: 実 container lease 下で editor child HWND
   が attach 済み（`observe_attached_children` 既存 oracle —
   `RuviformTestEditor` class、`WS_CHILD`、title prefix
   `rvf-test-editor-attach-{n}|`）。
2. **open toggle 受理**: `send_key` down+up（`'p'`/`0x50`）が
   `Ok` — `Ops(Unsupported)` なら defined refusal で
   `Unsupported` 終了。
3. **open title log**: 全 child title が
   `|kd:0070:0050:0000,po:1,ku:0070:0050:0000` で終わる
   （`GetWindowTextW` 再採取 — `title_log_check`）。
4. **open HWND oracle**: `ospres::inventory().own_windows`
   から popup が **ちょうど 1 個** —
   - class `RuviformTestEditor`
   - title が `rvf-test-editor-popup-` prefix
   - `owner` == `osmut::observe(child_hwnd).root`
     （`GA_ROOT` — fixture が popup の `GW_OWNER` に設定する値）
   - `oschild::inventory(popup)`: alive、`GWL_STYLE` に
     `WS_POPUP` set かつ `WS_CHILD` clear
   - `rect` == (0,0) 200x100
   受理済み open で HWND が存在しない/複数/契約違反は `Fail`
   （`NotRun` に降格しない）。
5. **close toggle 受理**: 2 回目の down+up が `Ok` —
   open `Accepted` のときのみ実行（refused open 後の VK_P は
   逆に popup を開き証拠を汚染する）。受理後の拒否/失敗は
   contract surprise として `Fail`。
6. **close title log**: 全 child title が
   `|kd:…,po:1,ku:…,kd:…,pc:1,ku:…` の全 suffix で終わる。
7. **close HWND oracle**: 同条件の popup が top-level
   inventory に **0 個**、かつ記録した popup HWND が
   `IsWindow == 0`（`win32::destroy` の効果）。
8. **session 維持**: `send_key` 後も `Attached`（non-fatal
   contract — verb 先例と同一）。
9. **close 収束**: `session.close` → child HWND 破棄 →
   `Closed + concluded` → events ==
   `[Attached{Docked}, Closed]`（`conclude_editor_session`）。
10. **teardown**: close 後の top-level inventory に owner 非依存で
    fixture popup が **0 個**（`removed()` が popup も破棄する
    契約。残存は契約違反かつ (0,0) 200x100 が後続 case を遮蔽する
    lane 衛生上の leak）。

画素は一切見ない — oracle は title event log と HWND
inventory のみ。`popup_open` 論理フラグの単独読みで Pass を
推論しない。

## verdict 写像（`editor_op_body` 再掲）

| verification / landing | verdict |
| --- | --- |
| `!supported`（非 Windows / env 不備） | `Unsupported` |
| `verified && landing == Accepted && effect_confirmed` | `Pass` |
| `verified && landing == Refused` | `Unsupported` |
| `verified && landing == Accepted && !effect_confirmed` | `NotRun`（accepted-but-unverifiable — 本 case の全 oracle が supported 環境で実行されるため事実上到達しない fail-safe） |
| 上記以外の verification 失敗 | `Fail`（failure detail は最初の失敗 step） |

`verified` = attach_ok ∧ state_ok ∧ effect_ok ∧ close_ok ∧
detach_ok ∧ converged_ok ∧ teardown_ok。
`effect_confirmed` = `landing == Accepted` ∧ effect_ok ∧
teardown_ok。

## `ospres::TopLevelWindow` 拡張

popup identity の title 照合に必要な最小拡張として、
`TopLevelWindow` に query-only の `title: Option<String>` を
追加する（`GetWindowTextLengthW`/`GetWindowTextW` — 失敗と
無 title は `None` に集約、照合側は両者を区別しない）。
`top_level_window_json` に `"title"` を追加し、dcomp 系 case
の inventory 証拠にも同一欄が載る（additive な証拠強化）。

## step / extras

steps: `editor_popup_open` / `editor_popup_open_log` /
`editor_popup_open_hwnd`（fail 時は `…_facts` note 行に観測
値を残す）/ `editor_popup_close` / `editor_popup_close_log` /
`editor_popup_closed_hwnd` / `session_state_after_op` /
`session.close` / `child_hwnd_detached` / `session_concluded` /
`session_events` / `editor_popup_teardown`。

`observed` extras は `child_extras`（container/child_hwnds/
threads/events）に `"popup"` object を追加 — hwnd/z_index/
class/title/owner/expected_owner/owner_via_getparent/rect/
style/exstyle/visible/alive。

## catalog / count 整合

- `real_entries`・`REAL_CATALOG`・console `--list
  --include-real`・`blocked_probe` の並びは全て
  `real-editor-focus` と `real-session-crash` の間。
- 28 → 29（real）、37 → 38（console projection）、
  headless blocked-probe は 29 `NotRun` 行。
- `required: &[ScenarioCapability::NativePluginDock]`。
- `spec_extras: editor_attach_extras`（editor family 共通）。

## 証拠

`evidence/rf631/plugin-dock-real-scenario.jsonl` を interactive
Windows で再生成し、本 case の steps/verdict/extras を
JSONL に残す。`plugin-dock-real-headless.jsonl` は
blocked-probe で 29 `NotRun` 行 +
`observed.environment_blocked` を再生成する。既知の Issue
#532 系 `Fail`（`real-dcomp-bandescape`/`real-dcomp-childinv`）
は本 slice と無関係に正直に残す — lane 全体の exit code が
非 0 でも popup 結果と区別して記録する。local verification
は merge 後 CI/integration acceptance の代替ではない。

## Non-goals

- Issue #630 本体の consumer 実走 gate / AC2 実行面。
- fixture の挙動変更・plugin-host/production crate 変更。
- popup の pixel oracle・popup 内部の描画検証。
- `docs/**/README.md` 索引の同期（Coordinator が統合時に実施）。
- `AGENTS.md` / `mise.toml` 変更・新規タスク追加。
