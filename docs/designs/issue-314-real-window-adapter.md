---
id: "DESIGN-314rwBn7xK2pQwE5rTyUi"
type: "design"
status: "approved"
title: "Issue #314 slice 4 residual — real-window adapter 詳細設計"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "docs/specs/issue-314-real-window-adapter.md"
    - "docs/designs/issue-314-golden-scenario.md"
    - "docs/designs/issue-314-golden-capture.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-314-real-window-adapter.md"
---

# real-window adapter 詳細設計

## 決定: cross-thread RPC adapter

`gpui::Application`/`App`/`Entity` は `Rc` ベースで `!Send` のため、
イベントループは専用スレッドに固定し、scenario 本体は caller
スレッドで実行する。

- **UI スレッド**: `GpuiDriver` + platform loop。`Inner.ui_jobs`
  （`Option<mpsc::Receiver<UiJob>>`）に `Send` クロージャを積み、
  `drain` が window ops 適用後に `App` context 内で実行する。
  job の引数 `(&Rc<RefCell<Inner>>, &GpuiDriver, &mut App)` は
  UI スレッド上で生成されるため GPUI 型は境界を越えない。
- **caller スレッド**: `RealOsGuiTest` が `Sender<UiJob>` を持ち、
  各操作を job 化して reply channel で同期受信する
  （15 s timeout → `ScenarioError::Driver`）。

却下した案: app を worker スレッドへ移す案（`Rc`/`!Send` で不可能）、
`Entity`/`Window` のスレッド間受け渡し（thread-affinity 違反）、
frame callback 経由の coroutine（GPUI に外部 await 面がない）。

## 構成要素

- `RealOsGuiTest::run(mounts, options, body)` — app スレッドを spawn、
  `create_view_window` → `ready` 通知 → startup settle → `body` →
  quit/join（panic 時も teardown 後に resume）。
- `settle` は `drain` 境界カウント（`settle_ticks`/`startup_settle_ticks`）
  — sleep ではなく実フレームペース。
- `capabilities()`: `SemanticAction` + `MultiWindow` 常時、
  `VisualCapture` は `cfg!(windows)` のみ。`RealOsPointerOrKeyboard`
  は #313 scope のため決して claim しない。
- `dispatch_input`/`select_*`/`invoke`/`observe`/`post_event` は
  `RvfView` の `try_*` seam 経由の semantic dispatch で、pin
  （`attach_epoch`/`generation`）を毎回 `App` 内で再検査する。
- `capture_frame` は pin 検査後 `GpuiDriver::capture_window` を呼び、
  `OsCapturedFrame` → `RgbaImage` へ変換。`UiError::Unsupported` →
  `CaptureOutcome::Unsupported`、`UiError::Host` → `NotRun`、
  その他 → `ScenarioError::Driver`。成功時 `CaptureMeta`
  （width/height/scale_factor/backend）を `last_capture_meta` に記録。
- `update_view` は teardown seam（generation 安定の spec 差替え）。
- intent は app スレッドの `on_event` が `Action` intent を共有
  `Arc<Mutex<Vec<Intent>>>` に蓄積し、`drain_intents` が
  `minted_from_ipc` で decode する。

## feature 構成

`real-scenario = ["gpui", "dep:ruviform-ui-test"]` — `gpui/test-support`
を含めない（test platform は `capture_raw_parent` を stub し pixel
証拠を名乗れない）。`gpui-test` との併用は想定しない。

## RT 契約

non-RT のみ。全 GPUI 型は UI スレッドに留まり、境界を越えるのは
`Send` クロージャと結果値のみ。
