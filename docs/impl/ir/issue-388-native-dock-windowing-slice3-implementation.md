---
id: "IMPL-rf388DockWindowingSlice301"
type: "impl"
status: "completed"
title: "Issue #388 GPUI native dock slice 3 — ancestor clip / keyboard・focus routing / 2-phase re-dock 実装記録（Windows / VST3 / L0）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc/dock_windowing.rs"
      - type: "file"
        path: "docs/specs/issue-388-native-dock-windowing-slice3.md"
      - type: "file"
        path: "docs/designs/issue-388-native-dock-windowing-slice3.md"
      - type: "file"
        path: "docs/plans/implement-issue-388-native-dock-windowing-slice3.md"
      - type: "file"
        path: "docs/tasks/issue-388-native-dock-windowing-slice3.md"
      - type: "file"
        path: "docs/test-specs/issue-388-native-dock-windowing-slice3.md"
      - type: "file"
        path: "evidence/rf388-native-dock-windowing/report.md"
      - type: "file"
        path: "evidence/rf388-native-dock-windowing/checks-vinyl.jsonl"
      - type: "file"
        path: "evidence/rf388-native-dock-windowing/dockw-vinyl-win11.txt"
      - type: "file"
        path: "evidence/rf388-native-dock-windowing/dockw-probe-win11.txt"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/main.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/Cargo.toml"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #388 GPUI native dock slice 3 — ancestor clip / keyboard・focus routing / 2-phase re-dock 実装記録

## Summary

`render.rs` に `slot_clip`（ancestor overflow mask との交差）を追加して slot clip 投影を修正し、`poc.rs` に OS 観測/入力注入ヘルパと probe editor の実描画・message counter を追加、`poc/dock_windowing.rs` の `dock-windowing` scenario と mise task を新設した。HWND 機構（parentage/geometry/visibility/clip/hit-test/focus/key/lifetime/re-dock）は全て実測 Pass だが、GPUI 0.2.2 の DirectComposition（`CreateTargetForHwnd(hwnd, topmost=true)`）が window の全 child HWND を覆うため plugin editor の画素は画面に合成されない（Unsupported・実測）。Native Dock PASS は画素表示未達のため不成立。

## Context

- Issue #388 slice 3（clip/focus/redock windowing 実測）。doc chain: spec `docs/specs/issue-388-native-dock-windowing-slice3.md`、design `docs/designs/issue-388-native-dock-windowing-slice3.md`、plan `docs/plans/implement-issue-388-native-dock-windowing-slice3.md`、task `docs/tasks/issue-388-native-dock-windowing-slice3.md`（T1–T7 全て完了）、test-spec `docs/test-specs/issue-388-native-dock-windowing-slice3.md`
- 前提: slice 2（`docs/impl/ir/issue-388-native-dock-layout-poc-implementation.md`）の PoC 基盤（`poc/dock.rs`・canonical surface・`GpuiDriver::update_view`）の上に構築。証拠の正本は `evidence/rf388-native-dock-windowing/report.md`
- Dock PASS は付与しない（evidence report 明記）。Native Dock PASS は完了条件全体の判定であり、本 slice はその一部の実測を行った

## Implementation

### Changes

- `render.rs`: `slot_clip(bounds, mask)` を追加し `project_slot_frame` が
  `window.content_mask().bounds`（ancestor overflow mask の累積値）との交差を
  slot clip として投影するよう修正。mask が狭めない場合は `None`、完全に
  mask された場合は zero-size clip（→ adapter 側で空 HRGN → child 非表示・
  hit-test 不可）。負の交差寸法は 0 に clamp。4 ケースの単体テスト追加。
- `poc.rs`: OS 観測/入力注入ヘルパ（`window_rect`/`window_pixel`/`screen_pixel`/
  `printwindow_pixel`/`region_box`/`clip_box`/`window_tree`/`ancestor_chain`/
  `probe_message_counts`/`hook_messages`/`unhook_messages`/`inject_key` 等）と
  probe editor の `WM_PAINT`/`WM_ERASEBKGND` 実描画・message counter を追加。
- `poc/dock_windowing.rs`: `dock-windowing` scenario。attach/clip（layout 駆動
  overhang + `SharedSurface::set_clip` direct）/ `WindowFromPoint` hit-test /
  focus・key routing / DockWorkspace 2-phase move（busy lease 拒否→commit）/
  detach→hidden 生存→reattach 同一 HWND / close・release・event 順序。
- `mise.toml`: `poc-native-dock-windowing` task 追加。

### Decisions During Implementation

- slot clip の投影は `window.content_mask().bounds`（ancestor overflow mask の累積値）との交差で行う。mask が狭めない場合は `None`（clip 未適用）、完全に mask された場合は zero-size clip とし、adapter 側で空 HRGN → child 非表示・hit-test 不可の経路に乗せる。負の交差寸法は 0 に clamp。
- OS 観測・入力注入は `poc.rs` のヘルパ群に集約し、probe editor に `WM_PAINT`/`WM_ERASEBKGND` 実描画と message counter を持たせる — focus・key routing を plugin の描画内容に依存しない message 単位で観測するため。
- `dock-windowing` scenario は attach/clip/hit-test/focus-key/redock/close-release/event 順序を 1 本の check 列で実行し、exit code を失敗数で決める（slice 2 の dock scenario と同じ証拠形式 `check=… result=pass|fail`）。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- task `docs/tasks/issue-388-native-dock-windowing-slice3.md` の T1–T7 は全て完了（`slot_clip` 修正+単体テスト・`poc.rs` ヘルパ・scenario+dispatch+mise task・probe mode 完走・Vinyl 実機 lane+evidence・capability matrix 更新・`mise run fmt-check check lint test test-ui-gpui` + 既存 `poc-native-dock-layout` lane 非退行確認）。
- 実行コマンド・exit・結果の一覧は証拠レポート `evidence/rf388-native-dock-windowing/report.md` の「検証」節が正本: Windows 11 x64 / Git Bash / mise pin rust 1.99.0 / gpui 0.2.2 で `mise run fmt-check`・`check`・`lint`・`test`・`test-ui-gpui`・`poc-native-dock-windowing`（Vinyl・probe）・`poc-native-dock-layout`（Vinyl）が全て exit 0。`dock-windowing` は 33 check 全 pass / `failures=0`。
- `slot_clip` の 4 ケースの単体テストを `render.rs` に追加し `test-ui-gpui` で実行。

### 実機観測の発見（証拠: `evidence/rf388-native-dock-windowing/report.md`）

- GPUI 0.2.2 `platform/windows/directx_renderer.rs:793` が
  `CreateTargetForHwnd(hwnd, topmost=true)` を呼ぶため、DComp swapchain が
  window の全 child HWND を覆い、plugin editor の画素は画面に合成されない
  （Unsupported・実測）。slice 2 で「Vinyl 背景」と解釈した `38,45,54` は
  GPUI `control_track`（`0x262d36`）であり plugin 画素ではなかった。
- HWND 機構（parentage/geometry/visibility/clip/hit-test/focus/key/lifetime/
  re-dock）は全て実測 Pass。Native Dock PASS は画素表示未達のため不成立。

## Risks

以下は review 由来の既知の制限（非ブロッキング）。

- **native surface の画素表示は Unsupported（実測）**: GPUI の DComp swapchain が child HWND を覆うため、plugin editor の画素はユーザーに一切表示されない（原因の詳細は Validation「実機観測の発見」参照）。HWND 機構は全て Pass だが NativeSurfaceSlot の視覚的目的は現行 GPUI 経路では未達。対処は GPUI 側の変更か adapter 層の設計変更が必要で、本 slice の編集範囲では解決できない。
- `SharedSurface::set_clip` の外部呼出しは renderer の projection ledger を
  更新しない。外部 set_clip 後に投影 clip が `None` のままだと dedupe で
  port が再 assert されず stale region が残りうる（PoC は明示 clear で回避。
  単一 writer 前提の契約を要検討）。
- `slot_clip` の `visible != bounds` 判定は非 2 冪 f32 で 1 ulp 差を生じうる
  （clip-capable driver では無害な dedup 済み SetWindowRgn のみ）。
- clip 証拠は window viewport mask（`content_mask` fallback）であり、
  中間 ancestor の `overflow:hidden` clip は本 layout では未発生
  （spec の Out-of-scope どおり将来課題）。
- `redock_*` の move は同一 window 内の degenerate move（busy/lease handshake
  の実証。実 re-dock の証拠は detach_view/attach_view 側）。
- `click_focuses_plugin` は click 失敗時に `SetFocus` fallback で pass しうる
  （`path=` detail に経路を記録。Vinyl run は `path=click`）。

## Follow-ups

- native surface 画素表示の対処（別 Issue/dispatch）: GPUI 側の変更（`CreateTargetForHwnd` の `topmost=false`、あるいは child を被覆しない surface 構成）か adapter 層の設計変更が必要 — 本 slice の編集範囲では解決できない（evidence report の結論）。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` 経路での child 合成確認 — GPUI 側 crash（exit 127・artifact なし）で未達。別 Issue 候補。
- plugin 側 resize（`onSize`）、DPI/マルチモニタ、2 window、crash/hang 注入、swapchain z-order、IME・popup の入力 routing — NotRun 維持（key のみ実測）。
- macOS / Linux / Wayland 全行、AU・CLAP、L1 isolation（#389 joint）— NotRun/Unsupported 維持。
- 中間 ancestor の `overflow:hidden` clip — spec の Out-of-scope、将来課題。
- `SharedSurface::set_clip` 外部呼出しと projection ledger の desync — 単一 writer 前提の契約明確化（`native_slot.rs` doc 未記載）。
