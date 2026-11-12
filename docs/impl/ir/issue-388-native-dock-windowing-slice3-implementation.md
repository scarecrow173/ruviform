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

# 実装内容

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

# 発見（証拠は `evidence/rf388-native-dock-windowing/report.md`）

- GPUI 0.2.2 `platform/windows/directx_renderer.rs:793` が
  `CreateTargetForHwnd(hwnd, topmost=true)` を呼ぶため、DComp swapchain が
  window の全 child HWND を覆い、plugin editor の画素は画面に合成されない
  （Unsupported・実測）。slice 2 で「Vinyl 背景」と解釈した `38,45,54` は
  GPUI `control_track`（`0x262d36`）であり plugin 画素ではなかった。
- HWND 機構（parentage/geometry/visibility/clip/hit-test/focus/key/lifetime/
  re-dock）は全て実測 Pass。Native Dock PASS は画素表示未達のため不成立。

# 既知の制限（review 由来・非ブロッキング）

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
