---
id: "IMPL-034VOSINPDRV4REAL0SIN6"
type: "impl"
status: "completed"
title: "Issue #313 `RealOsPointerOrKeyboard` capability 実装と Windows 実入力 E2E 証拠"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "docs/specs/issue-313-real-os-input-driver.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-313-real-os-input-driver.md"
      - type: "file"
        path: "docs/designs/issue-313-real-os-input-driver-design.md"
      - type: "file"
        path: "docs/plans/issue-313-real-os-input-driver-plan.md"
      - type: "file"
        path: "docs/test-specs/issue-313-real-os-input-driver.md"
      - type: "file"
        path: "docs/tasks/issue-313-task1-real-os-input-driver.md"
      - type: "file"
        path: "docs/impl/ir/issue-313-real-os-input-driver-slice.md"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/verdict-real-os-input-windows.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/real_window.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "evidence/issue-314-real-scenario/driver-lab-evidence.jsonl"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/verdict-windows.json"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/verdict-pointer-windows.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-313-task1-real-os-input-driver.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-313-real-os-input-driver-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-313-real-os-input-driver.md"
    - "docs/designs/issue-313-real-os-input-driver-design.md"
    - "docs/test-specs/issue-313-real-os-input-driver.md"
    - "docs/impl/ir/issue-552-sendinput-preflight-read-back-environment-blocked.md"
    - "docs/impl/ir/issue-314-real-scenario-lane.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-313-real-os-input-driver.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #313 `RealOsPointerOrKeyboard` capability 実装と Windows 実入力 E2E 証拠

## Summary

`RealOsGuiTest`（`ScenarioDriver::RealOs`）に OS 入力 port を追加し、
`SendInput` による実キーボード/実ポインタ注入を scenario 契約経路から
駆動できるようにした。`os_input` 層には既に注入プリミティブ
（`OsKey`/`press_key`/mouse/`preflight`）があったが、semantic な
`SemanticGuiTest`/`GuiAction` 経路には OS-input op がなく、capability
`RealOsPointerOrKeyboard` は「Issue #313 scope」注記つきで未 claim
だった構造的ギャップを解消した。

能力 claim は `os_input::supported()`（現 slice では Windows のみ）に
限定し、全 `os_*` op は Issue #552 先例の preflight read-back を
family 単位で gate する。注入の実効がない環境は GUI `fail` でなく
`EnvironmentBlocked`/`NotRun`/`Unsupported` を正直に返す。

## Context

- Issue #313 受入条件のうち本 slice の担当: 「起動 → 実入力（OS 注入
  経路）→ IPC → Receipt/表示 → 終了」の Windows 実測再現。
- semantic dispatch（`ViewHost::dispatch_input` 直叩き）と toolkit
  simulated input（GPUI `dispatch_keystroke`/`simulate_*`）と OS
  automation injection（`SendInput`）を証拠上分離するため、OS op は
  すべて新しい `os_*` メソッド群に集約し、戻り値を
  `OsInputOutcome` 語彙で返す（`Err(ScenarioError)` は contract
  失敗のみ）。
- `SendInput` は OS automation injection であり物理手動入力ではない —
  verdict JSON の `input_method`/`input_method_note` に明記。

## Implementation

### Changes

- `crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs`
  - `OsInputOutcome`（`#[non_exhaustive]`）追加:
    `Injected { action, commanded, cursor, detail }` /
    `Unsupported { reason }` / `EnvironmentBlocked { detail }` /
    `NotRun { reason }`。環境差異と契約失敗を分離する戻り値語彙。
  - `PreflightScope::families()` 追加 — scope → probe 対象 family
    （`["pointer"]` / `["keyboard"]` / 両方）の正本マッピング。
- `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs`
  - `supported_capabilities()` が `os_input::supported()` 時のみ
    `ScenarioCapability::RealOsPointerOrKeyboard` を claim（
    `SemanticAction`/`MultiWindow` 継続、`VisualCapture` は
    capture backend 条件つきのまま）。`capability_lifecycle_declares`
    ユニットテスト更新。
  - focus tracking: `WindowLifecycle::Focused(active)` を
    `focused: Arc<Mutex<BTreeMap<WindowId, bool>>>` に記録。
    `request_focus(window, true)` は `WindowDriver::focus` の queue
    経路に委譲（GPUI は unfocus を正直に出せない既存契約を維持）。
    `window_focused(window)` が最終観測を返す。
  - `os_input_preflight(scope)`: family 単位で session 内 cache
    （`os_preflight: RefCell<BTreeMap<&'static str, PreflightCheck>>`）
    — 同 family を再 probe しない。`os_gate(family)` が未 probe の
    family だけ 1 回 probe する。
  - `os_press_key` / `os_pointer_move_to` / `os_mouse_down` /
    `os_mouse_up`: 共通 prelude で target pin（epoch/generation）と
    component 実在を再検査（`exec_check_os_target`）、family
    preflight + focus 観測を gate、非対応/非実効/未 focus は
    `Unsupported`/`EnvironmentBlocked`/`NotRun` を返して注入しない。
  - `exec_resolve_os_aim`: 計測済み論理矩形（
    `GpuiDriver::component_bounds`）+ 物理 window geometry（
    `window_geometry` の origin/size/scale_factor）から
    `screen = origin + (logical + frac*size) * scale` の実スクリーン
    座標を解決。bounds 欠損/degenerate は `NotRun`。物理座標は OS
    virtual screen に収まる既知の bounded cast として局所
    `#[expect(clippy::cast_possible_truncation)]`（リポジトリ既存の
    同一パターンに準拠）。pointer move は注入後 `cursor_position`
    で read-back し commanded/measured 両点を evidence として返す。
- `crates/ruviform-ui-gpui/tests/real_window.rs`
  - `real_os_input_scenario`: 2 コントロール（`gain`/`seek`）view を
    real window に mount。unsupported OS は `unsupported` verdict を
    書いて return（capability 未 claim を検証）。focus 未確認は
    `not_run` panic、preflight 非実効は `environment_blocked` panic。
  - `os_input_scenario_body`: `Tab→Right→Tab→Left` の実キー列（
    gain 0.55 / seek 0.45、atomic Begin+Commit）→ 計測矩形上の
    pointer drag（gain ≈0.75、non-atomic、gesture_id 一致）→
    seek click（≈0.80）→ `drain_intents` で IPC intent を検証 →
    最新の receipt-expecting commit（head record）へ
    `Event::result_ok` を `post_event` して `Applied` を `observe`
    → `capture_frame` で実ピクセル取得（os-capture leg）。
  - verdict JSON `rf313-real-os-input-scenario/v1` を
    `evidence/rf313-os-capability-matrix/` へ、driver-lab evidence を
    `evidence/issue-314-real-scenario/driver-lab-evidence.jsonl` へ
    記録。run log channel は `os-injection`/`ipc`/`os-capture`。
  - ユーザーカーソルは session 前に保存し best-effort 復帰。
- `mise.toml`: `[tasks.test-ui-real-os-scenario]` — 既存
  `example-ui-gpui-real-os-*` 系と同型の opt-in manual lane。
  実カーソル/キーボードを占有し unattended 前提である旨を
  description に明記（タスク固有の例外許可 1 件を使用）。

### 設計判断

- `GuiAction`/`SemanticGuiTest` に OS op を増やさず `RealOsGuiTest`
  側の並行 port とした — semantic 契約を汚さず、`os_*` 呼出しだけが
  OS 注入を意味するという証拠上分離が保てる（design doc 記録）。
- preflight を capability claim の前提に据える Issue #552 先例を
  op 単位でも維持し、family ごとの 1 回 probe でセッション中の
  注入可効性を固定する。

## Verification

### 実測（Windows 11 Pro x64、GPUI real window、scale 1.0）

- `mise run test-ui-real-os-scenario` → `real_os_input_scenario` PASS。
  実キー列と pointer gesture が `SendInput` 経由で GPUI コントロールに
  届き、mint された intent（atomic/non-atomic・gesture_id 一致・
  expect_receipt）・receipt `Applied`・実ピクセル capture の全脚を
  検証。verdict `pass` を
  `evidence/rf313-os-capability-matrix/verdict-real-os-input-windows.json`
  に記録（preflight read-back 詳細・環境 metadata 含む）。
- 既存 real lane 回帰なし:
  `example-ui-gpui-real-os-smoke` / `example-ui-gpui-real-os-pointer-smoke`
  / `example-ui-gpui-real-os-multiwindow-smoke` /
  `test-ui-real-scenario` すべて pass。
- headless lane 回帰なし: `fmt-check` / `check` / `lint` / `test` /
  `test-rt` / `check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` /
  `test-ui-scenario-contract` / `check-ui-driver-boundary` すべて pass。
- 補助証拠（非 canonical）: `cargo clippy -p ruviform-ui-gpui
  --all-targets --features real-scenario -- -D warnings` も pass —
  canonical `lint` は `real-scenario` feature を含まないため
  feature 付き clippy を併走させた。

### 修正した実装期のバグ

- receipt leg が最初の `expect_receipt` commit（keyboard 側）へ
  receipt を投げていた。`phase()` は head record（最新の
  receipt-expecting command）を読むため head である pointer-click
  commit（`seek:4`）に送るよう `.rev().find` に修正 — 初回実走で
  `Pending` が観測され、inbox probe（1→0 drain 確認）と
  `behavior.rs` の head 契約から確定。

### 観測した既存ノイズ（本 slice 範囲外・main でも再現）

- real-window セッションの teardown で `RefCell already borrowed`
  と `HRESULT 0x80070578`（invalid window handle）が ERROR ログに
  出る — `test-ui-real-scenario`（本 slice 未変更の既存 lane）でも
  同一に出るため main 由来の既存現象。閉塞/close ハンドラの borrow
  競合が疑わしいがテスト結果には影響しない。別 Issue 候補として
  記録のみ。
- `probe` 用の一時的な診断コード（inbox_len eprintln）は debug 後に
  すべて除去済み。

## Remaining

- macOS/Linux 実機行: `os_input::supported()` が false となり
  capability 未 claim → `unsupported` verdict（本 slice の設計どおり）。
- 動的 DPI/マルチモニター（本機は静的 2×1080p@1.0 のみ）、IME
  capability probe、windowed `TestIo`/`rvf.uitest.v1` 配線（S2）、
  consumer 受入 — 後続 slice へ。
- teardown の `RefCell already borrowed` / invalid HWND ノイズの
  原因調査（本 slice 範囲外）。
