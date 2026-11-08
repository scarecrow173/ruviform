---
id: "DESIGN-034Ug8WYFJ37a8hKlmcQaW"
type: "design"
status: "approved"
title: "Issue #314 slice 3 — GPUI/OS capture backend の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/ruviform/issues/243"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-314-golden-capture.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/capture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/golden_capture.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/visual.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-314-golden-capture.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-314-golden-scenario.md"
    - "docs/designs/issue-314-golden-core.md"
    - "docs/designs/issue-243-gpui-realwin-windows.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-314-golden-capture.md"
    - "docs/designs/issue-314-golden-scenario.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-314-golden-scenario.md"
    - "docs/designs/issue-314-golden-core.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-314-golden-capture.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 3 — GPUI/OS capture backend の設計

## Context

slice 2 は `CaptureOutcome::Captured` を返す adapter が存在しない
状態で契約を固定した。実装調査の結果:

- `gpui-test`（`GpuiObserver`/`TestAppContext`）には pixel readback
  が存在しない — test window の `draw` は no-op、`screen-capture`
  feature は OS video-capture であり window readback ではない。
  よって `GpuiGuiTest::capture_frame` は `Unsupported` が正しい。
- `GpuiDriver::run`（#243/PR #482）は実 OS window を開き、
  `Slot::raw_parent` に `surface::raw_parent_of(window)` 由来の
  native handle（Windows では HWND）を既に保持している。
- `GpuiDriver::run` は呼び出し thread を platform event loop が
  占有するため、`SemanticGuiTest` adapter（`GpuiGuiTest` 型）の
  内側から capture を呼ぶ構造にはならない。実 window の golden
  経路は **driver 層 API** として提供し、scenario adapter 層は
  変更しない。

## Scope

### In Scope

- `gpui_driver/capture.rs` 新設（`OsCapturedFrame`、
  `GpuiDriver::capture_window`、per-OS backend）。
- `lib.rs` の mod 宣言 + re-export のみの wiring。
- Windows backend: `PrintWindow` primary + screen `BitBlt` fallback。
- 非 Windows backend stub（`Unsupported`）。
- `tests/golden_capture.rs`: error path + e2e golden test。
- `scenario/visual.rs` の module doc 更新（動作変更なし）。

### Out of Scope

- `gpui-test`/Fake の `capture_frame` — `Unsupported` 維持。
- 実 window `SemanticGuiTest` adapter — #313 系（`RealOs`）。
- macOS/Linux backend、Cargo.toml/feature 変更、#479 領域、
  consumer fixture（slice 4）。

## Components and Boundaries

```text
crates/ruviform-ui-gpui/src/gpui_driver/capture.rs   — capture backend（新規）
crates/ruviform-ui-gpui/src/lib.rs                    — mod 宣言 + re-export
crates/ruviform-ui-gpui/tests/golden_capture.rs       — e2e/error tests（新規）
crates/ruviform-ui-test/src/scenario/visual.rs        — doc 更新のみ
```

- **`OsCapturedFrame`**（pub struct）: `width`/`height`/
  `scale_factor`/`pixels`（RGBA8 row-major top-down, alpha=255）/
  `backend`（`&'static str` — 使用した OS 経路の識別子）。
  OS handle 型を含まない driver-neutral DTO。
- **`GpuiDriver::capture_window(&self, window: WindowId)`**:
  slot の `handle.is_some()`（open 済み）と `raw_parent`
  （OS handle あり = 実 `gpui` backend）を確認してから backend に
  委譲。未知 id → `UiError::Host`、未 open → `UiError::Host`、
  handle なし（gpui-test）→ `UiError::Unsupported`。
- **Windows `backend` module**（`#[cfg(windows)]`、
  `#[expect(unsafe_code)]`）:
  - probe: `IsWindow`/`IsIconic`/`IsWindowVisible`。
  - `client_screen_rect`: `GetClientRect` + `ClientToScreen`。
  - `print_window_capture`: `PrintWindow(hwnd, mem_dc,
    PW_CLIENTONLY|PW_RENDERFULLCONTENT)` — window 自身の描画内容。
  - `capture_screen_rect`: `BitBlt` screen DC → client rect —
    表示中 pixel の fallback。
  - `dib_capture`: 32bpp top-down DIB 生成・選択・render・
    BGRA→RGBA 変換・破棄を共通化（`render` closure で経路差を
    吸収）。
- **非 Windows `backend` module**（`#[cfg(not(windows))]`）:
  `capture_client_area` が常に `UiError::Unsupported`。

## Data and Control Flow

```text
event loop (driver.run callback, UI thread)
  │  DriverEvent::Tick × SETTLE_TICKS（open/settle animation 完了待ち）
  ├─ driver.capture_window(window)
  │     ├─ slot lookup → unknown/unopened → UiError::Host
  │     ├─ raw_parent なし → UiError::Unsupported（gpui-test）
  │     └─ backend::capture_client_area(hwnd, scale_factor)
  │           ├─ IsWindow/IsIconic/IsWindowVisible probe → UiError::Host
  │           ├─ client_screen_rect → (x, y, w, h)
  │           ├─ print_window_capture → Ok? → OsCapturedFrame(backend="win32-printwindow")
  │           └─ Err → capture_screen_rect → OsCapturedFrame(backend="win32-bitblt-screen")
  ├─ RgbaImage::new(w, h, pixels) → BaselineStore::compare
  │     └─ blocked/baseline_missing（初回）→ write_candidate → promote
  └─ 2 回目 capture → compare → pass/match → append_evidence(JSONL)
```

- `PrintWindow` は `extern "system"` で局所宣言 — windows-sys は
  `Win32_Storage_Xps` feature でのみ公開しており、feature 追加は
  file boundary 外。user32 は既存 `Win32_UI_*` feature で link 済み。
- `PW_CLIENTONLY|PW_RENDERFULLCONTENT`（0x1|0x2）の Win32 ABI 値は
  安定定数として局所定義（`PW_RENDERFULLCONTENT` は
  WindowsAndMessaging で利用可能だが `PW_CLIENTONLY` は Xps 側）。

## Non-Functional Constraints

- 非 RT（UI thread、`Rc` driver state — `run` callback 内でのみ
  有効）。
- `unsafe` は backend module 単位の `#[expect(unsafe_code)]` +
  各 unsafe 操作の SAFETY コメント。`unsafe fn` の内部 unsafe
  block は Rust 2024 の unsafe-op-in-unsafe-fn 規約に従い明示。
- `windows-sys` 既存 feature のみ使用、`Cargo.toml` 変更なし。
- 公開 signature に HWND/HDC 等の OS handle 型を出さない
  （`raw_parent: usize` は driver 内部、`OsCapturedFrame` は
  pixel DTO のみ）。
- `OsCapturedFrame` は `#[non_exhaustive]` 不要（struct — enum
  ポリシー対象外）だが `PartialEq` derive で test 比較可能。

## Risks and Trade-offs

| リスク | 影響 | 対策 |
| --- | --- | --- |
| `PrintWindow` が GPU 合成 window で黒画面/古い内容を返す | 誤った baseline | e2e test が非一様 pixel を要求…ではなく実測で 0 diff を確認済み。`PW_RENDERFULLCONTENT` で DWM 合成面を取得。万一の失敗は `BitBlt` fallback が real pixel を返す（ambient 混入の可能性は `backend` tag で識別可能） |
| screen `BitBlt` fallback が cursor/tooltip/occlusion を拾う | golden diff の flaky | primary が `PrintWindow` のため通常経路では発生しない。fallback 使用時は `backend="win32-bitblt-screen"` が evidence に残り、再現条件を追跡可能 |
| `extern "system"` 宣言と windows-sys の ABI 不一致 | link/呼出 error | HWND/HDC は `*mut c_void` alias、BOOL は i32 で ABI 一致。`#[link(name="user32")]` で解決を保証 |
| settle 時間が環境依存で flaky | test の環境依存失敗 | `SETTLE_TICKS=30`（~0.5s）は Windows 実測値。frame 間 0 diff を確認。根本対策（描画完了 event）は GPUI 側の制約で本 slice 範囲外 |
| `PrintWindow` が minimized/hidden window でも動作し得るが probe で拒否 | capture 可能な場面を逃す | probe は「表示中 window の pixel」という golden の意味論に合わせて維持。minimized window の capture は別要件 |

## D1: capture は driver 層 API、scenario adapter は変更しない

- `GpuiDriver::run` は platform event loop が thread を占有する
  `WindowDriver` 実装であり、`SemanticGuiTest`（`GpuiGuiTest`）は
  `gpui-test` の `TestAppContext` 上の observer — 別実行モデル。
  実 window を `ScenarioTarget` pin で扱う adapter は
  `ScenarioDriver::RealOs`（#313）の設計課題。
- 本 slice は「pixel が実際に取れる」driver 層 primitive と
  golden e2e を提供し、scenario 層への `capture_frame` 配線は
  RealOs adapter 整備時に行う — 契約（slice 2）を壊さない。

## D2: `PrintWindow` primary / `BitBlt` fallback

- 当初は `BitBlt` のみを実装したが、実測で screen capture が
  mouse cursor・tooltip 等の ambient desktop 内容を拾い、frame
  間 diff が発生することを確認（小領域・不規則な出現タイミング）。
  golden baseline には window 自身の内容のみが必要なため、
  `PrintWindow(PW_CLIENTONLY|PW_RENDERFULLCONTENT)` を primary
  に変更 — cursor・occlusion・他 window の影響を排除する。
- `PrintWindow` が API 失敗（`WM_PRINT` を処理できない window
  class 等）した場合のみ `BitBlt` に fallback。fallback は
  「表示中 pixel」という別の正直な測定であり、`backend` tag で
  区別可能。

## D3: `PrintWindow` の局所 extern 宣言

- windows-sys 0.61 は `PrintWindow` を `Win32_Storage_Xps`
  feature でのみ公開。feature 追加は `Cargo.toml`（file boundary
  外）の変更を要する。
- `extern "system"` 宣言 + `#[link(name="user32")]` で回避 —
  user32 は既存 feature で link 済み、ABI は安定。宣言箇所に
  理由をコメントで明記。

## D4: `OsCapturedFrame::backend` による経路の正直な記録

- 2 経路が異なる pixel 内容を返し得る（ambient 混入の有無）
  ため、どちらで取得したかを frame に記録し、golden
  `EnvironmentMeta::backend_detail` へ転記可能にする。
  baseline の `driver_class`/`backend` 同一性は consumer が
  key/env 構築時に制御する。

## D5: settle は tick 数でなく「open animation 完了」の実測待ち

- GPUI に「描画完了」event はないため、`Ready` 後の固定 tick 数
  （30 ticks ≈ 0.5s @ 60fps tick）で settle を待つ。実測で
  frame 間 0 diff を確認済み。過剰な待機は test 時間を増やす
  だけなので最小実測値を採用。

## Verification Notes

- [x] `cargo check -p ruviform-ui-gpui --features gpui` 通過
      （unsafe warning なし）。
- [x] `cargo test -p ruviform-ui-gpui --features gpui --test
      golden_capture` — 2 tests pass（error path + e2e
      `blocked/baseline_missing` → `pass/match`、evidence 2 行）。
- [x] e2e test 内で `OsCapturedFrame::backend` が
      `win32-printwindow`/`win32-bitblt-screen` のいずれかで
      あること、両 capture が同一経路であることを assert。
- [ ] `mise run test-ui-scenario-contract*` 非回帰（Fake/GPUI
      adapter の `Unsupported` 維持）。
- [ ] `mise run check` / `lint` / `fmt-check` / `check-ui-driver-boundary`。
