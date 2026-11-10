---
id: "DESIGN-034UkPTR1W1N5M0K3P02"
type: "design"
status: "approved"
title: "Issue #313 — Windows 実 OS SendInput ポインタ smoke 設計（部分スコープ・後続スライス）"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_pointer_smoke.rs"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/verdict-pointer-windows.json"
      - type: "file"
        path: "docs/test-specs/issue-313-windows-pointer-smoke.md"
      - type: "file"
        path: "docs/plans/issue-313-windows-pointer-smoke.md"
      - type: "file"
        path: "docs/tasks/issue-313-windows-pointer-smoke.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/capture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/report.md"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/results.jsonl"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-313-windows-pointer-smoke.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-313-windows-pointer-smoke.md"
    - "docs/specs/issue-313-windows-realos-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-313-windows-realos-smoke.md"
    - "docs/designs/issue-243-gpui-realwin-windows.md"
    - "docs/designs/issue-314-golden-capture.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-313-windows-pointer-smoke.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-313-windows-pointer-smoke.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — Windows 実 OS SendInput ポインタ smoke 設計（部分スコープ・後続スライス）

## Context

キーボードスライスと同じ honesty 契約（`SendInput` = OS automation
injection であり物理手動入力ではない、simulated input とは別
レイヤー）の上に、ポインタ経路を追加する。ポインタ注入の固有
問題は**照準**: スクリーン座標をどう決めるか。製品 Scenario DSL
への座標固定は scope 外（spec Non-Goals）なので、座標は常に
実行時計測から導出する。

## 設計

### 1. `os_input` ポインタ API（`gpui_driver/os_input.rs`）

公開関数を keyboard と同じ薄い境界で追加:

- `cursor_position() -> Result<(i32,i32), UiError>` —
  `GetCursorPos`（物理 px、virtual-desktop 座標。副モニタ左上の
  負座標を許容）。
- `move_to(x, y)` — `SendInput` 1 イベント
  `MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK`。
  絶対座標は `GetSystemMetrics(SM_X/Y/CX/CYVIRTUALSCREEN)` で
  正規化（0..=65535）。virtual screen 全体を対象にするため
  主モニタ原点 0,0 仮定を置かない。
- `mouse_down()` / `mouse_up()` — 左ボタン
  `MOUSEEVENTF_LEFTDOWN`/`LEFTUP` 各 1 イベント。

`SendInput` の戻り値が期待イベント数未満なら `UiError::Host`
（部分注入を成功にしない — keyboard の `press` と同じ規約）。
非 Windows は全関数 `UiError::Unsupported`。`unsafe` は
`#[expect(unsafe_code)]` + SAFETY コメントの局所許可
（keyboard 実装・capture.rs と同じ先例）。

### 2. 計測ジオメトリ（`render.rs` / `lib.rs` / `capture.rs`）

照準に必要な 2 つの実測値を driver capability として公開:

- **`GpuiDriver::component_bounds(window)`** — `RvfView` が paint
  時に canvas 要素ごと記録していた bounds map を、driver が
  window slot に保持する共有 store
  （`Rc<RefCell<BTreeMap<ComponentKey, Bounds<Pixels>>>>`）として
  外部化。`Bounds<Pixels>`（GPUI 座標 = 論理 px、window client
  原点相対）を `LogicalRect` に変換して返す。paint 未実施・
  view 未 mount・未知 window は `UiError::Host`。
- **`GpuiDriver::window_geometry(window)`** — `capture.rs` の
  Windows backend に `client_geometry` を追加し、既存の
  `raw_parent`（HWND）+ `client_screen_rect`
  （`GetClientRect` + `ClientToScreen`）を再利用して client-area
  の物理スクリーン rect（`OsWindowGeometry { origin_x, origin_y,
  width, height, scale_factor }`）を返す。`capture_window` と
  同じ honesty 契約: 未 open/未知 → `Host`、`gpui-test` で
  OS handle なし → `Unsupported`、非 Windows → `Unsupported`。

`RvfView::mounted` の引数過多（clippy `too_many_arguments`）と
複雑型（`type_complexity`）は `ViewChannels { inbox, slots,
bounds }` bundle + `ComponentBounds` type alias で解消
（振る舞い不変の構造整理）。

### 3. `real_os_pointer_smoke` example

キーボード example と同じ骨格（`Panel[Fader(gain),
Seek(seek)]`、Ready + `Focused(true)` ゲート、injector thread、
verdict JSON、exit 0/1/2）に、照準とポインタ検査を追加:

- **照準**: `window_geometry` + `component_bounds` が両 control
  を返すまで tick 毎に poll。`screen_point = client_origin +
  (component_origin + frac * component_size) * scale_factor`
  で物理 px に変換（logical→physical は window の scale factor）。
- **注入系列**: `move_to(gain 下端)` → `mouse_down` →
  `move_to(gain 上端)` → `mouse_up`（Fader ドラッグ:
  Begin≈0.25 → Preview/Commit≈0.75）→ `move_to(seek 80%)` →
  `mouse_down` → `mouse_up`（Seek クリック: Begin+Commit≈0.8）。
  各 `move_to` 後に `cursor_position()` で read-back を記録
  （±4px tolerance を check 化 — SendInput 受付ではなく
  カーソル実到達の証拠）。
- **assert**: 観測 intent が非 atomic（`atomic != true`）で、
  gain は同一 `gesture_id` の begin≈0.25/commit≈0.75、seek は
  同一 `gesture_id` の begin+commit≈0.8。Preview は契約上
  coalescible なので件数のみ記録し合格条件にしない。
  normalized tolerance ±0.1（物理 px 丸め・サブピクセル
  レイアウト由来、意味論ドリフトではない）。
- **カーソル復元**: 起動時に `cursor_position()` を保存し、
  verdict 確定後（window close 前）に `move_to(home)` で
  best-effort 復元。失敗は stderr 記録のみで verdict に影響
  しない。`Abort` で injector を止め、注入が別 window に
  漏れないようにする。
- **driver 参照**: `component_bounds`/`window_geometry` は
  inherent method で `&mut dyn WindowDriver` 経由では呼べない
  ため、`driver.clone()`（`Rc<RefCell>` 共有）を closure に
  capture して query に使う。handler 呼出時は driver が
  handler を取り出してから invoke するため `inner` の
  二重 borrow は起きない。

### 4. 証拠

`verdict-pointer-windows.json`（schema
`rf313-real-os-pointer-smoke/v1`）を
`evidence/rf313-os-capability-matrix/` に追加し、report.md /
results.jsonl の pointer 行を NotRun → 実測値で更新。macOS/
Linux 行は NotRun のまま。

## 検討して採用しなかった案

- **座標のハードコード/窓中央固定**: DPI・マルチモニタ・
  レイアウトで破綻し、製品座標仮定の混入にもなるため却下。
- **`WindowDriver` trait への geometry 追加**: neutral contract
  を全 driver（Fake 含む）に拡大する変更で、本スライスの
  verification surface としては過剰。inherent method に留めた。
- **capture の pixel 座標からの逆算**: #314 の pixel 証拠と
  input targeting を混ぜると責務が濁る。geometry query を
  独立させた。
