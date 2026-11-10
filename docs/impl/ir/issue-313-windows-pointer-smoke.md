---
id: "IMPL-034UkPTR1W1N5M0K3P06"
type: "impl"
status: "completed"
title: "Issue #313 Windows 実 OS SendInput ポインタ smoke（driver 計測ジオメトリ + capability matrix 更新）"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "docs/specs/issue-313-windows-pointer-smoke.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_pointer_smoke.rs"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/verdict-pointer-windows.json"
      - type: "file"
        path: "docs/specs/issue-313-windows-pointer-smoke.md"
      - type: "file"
        path: "docs/designs/issue-313-windows-pointer-smoke.md"
      - type: "file"
        path: "docs/plans/issue-313-windows-pointer-smoke.md"
      - type: "file"
        path: "docs/tasks/issue-313-windows-pointer-smoke.md"
      - type: "file"
        path: "docs/test-specs/issue-313-windows-pointer-smoke.md"
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
      - type: "file"
        path: "docs/specs/issue-313-windows-realos-smoke.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-313-windows-pointer-smoke.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-313-windows-pointer-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-313-windows-os-sendinput-smoke-os-capability-matrix.md"
    - "docs/impl/ir/issue-243-gpui-realwin-windows.md"
    - "evidence/rf313-os-capability-matrix/report.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/test-specs/issue-313-windows-pointer-smoke.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-313-windows-pointer-smoke.md"
  verified-by: []
  references:
    - "docs/specs/issue-313-windows-pointer-smoke.md"
    - "docs/designs/issue-313-windows-pointer-smoke.md"
  defers:
    - "macOS/Linux/pointer 右中ボタン・修飾・wheel・IME/a11y・DPI 動的変化・2 window（Issue #313 残スコープ）"
    - "ScenarioDriver::RealOs adapter 本体（#464 系の別タスク）"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #313 Windows 実 OS SendInput ポインタ smoke（driver 計測ジオメトリ + capability matrix 更新）

## Summary

キーボードスライスが defer したポインタ部分スコープを実装・
実測した。`os_input` に絶対座標 move + 左ボタン注入を追加し、
`GpuiDriver::component_bounds`（paint 記録 bounds）と
`GpuiDriver::window_geometry`（client-area 物理スクリーン
rect）という計測 capability を公開。`real_os_pointer_smoke`
example が計測値から照準を導出して Fader ドラッグと Seek
クリックを `SendInput` で注入し、非 atomic gesture intent
（Begin/Preview/Commit、同一 `gesture_id`、期待 normalized
値）を実測 Pass した。`evidence/rf313-os-capability-matrix/`
の pointer 行を実測で更新。Issue #313 本体は Open 継続
（macOS/Linux・右中ボタン・IME/a11y・DPI 動的変化は
NotRun/Unsupported のまま）。

## Context

`docs/specs|designs|plans|tasks|test-specs/issue-313-windows-pointer-smoke.md`
の文書チェーンに従う。ポインタ注入の固有問題は照準であり、
製品 Scenario DSL への座標固定は scope 外 — 座標は常に
`component_bounds`（論理 px、window client 原点相対）+
`window_geometry`（物理 px、screen 座標）+ window scale
factor から実行時に導出する。

## Implementation

### Changes

- `crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs`:
  `cursor_position`/`move_to`/`mouse_down`/`mouse_up` を追加。
  Windows は `SendInput` — move は
  `MOUSEEVENTF_MOVE|ABSOLUTE|VIRTUALDESK` を
  `GetSystemMetrics` virtual screen で正規化（負原点対応）、
  ボタンは `LEFTDOWN`/`LEFTUP`。`send` 共通ヘルパーで受付数
  不足を `UiError::Host`（部分注入を成功にしない）。非
  Windows は `UiError::Unsupported`。keyboard の `press` も
  同ヘルパー経由に整理（挙動不変）。
- `crates/ruviform-ui-gpui/src/gpui_driver/capture.rs`:
  `OsWindowGeometry { origin_x, origin_y, width, height,
  scale_factor }` と `GpuiDriver::window_geometry` を追加。
  Windows backend に `client_geometry`（`IsWindow` プローブ +
  既存 `client_screen_rect` 再利用）。非 Windows は
  `Unsupported` stub。
- `crates/ruviform-ui-gpui/src/render.rs`: paint 時に記録して
  いた bounds map を呼出側所有の共有 store として
  `RvfView::mounted` に渡す形に変更。`ComponentBounds` alias
  と `ViewChannels { inbox, slots, bounds }` bundle で
  clippy（`type_complexity`/`too_many_arguments`）適合。
  `logical_rect_of` を `pub(crate)` 化。
- `crates/ruviform-ui-gpui/src/lib.rs`: window slot に bounds
  store を保持し `GpuiDriver::component_bounds` で
  `LogicalRect` map を返す（未知/未 mount/未 paint は
  `Host`）。`OsWindowGeometry` を re-export。
- `crates/ruviform-ui-gpui/examples/real_os_pointer_smoke.rs`
  （新規）: Ready + `Focused(true)` + geometry/bounds 揃いを
  ゲートに、injector thread が Fader ドラッグ（下端→上端、
  Begin≈0.25/Commit≈0.75）と Seek クリック（80%、Begin+Commit
  ≈0.8）を注入。各 move 後に `cursor_position` read-back
  （±4px check）。非 atomic・同一 `gesture_id`・normalized
  ±0.1 を assert。起動時カーソル位置を保存し終了時に復元。
  verdict schema `rf313-real-os-pointer-smoke/v1`、exit
  0=pass/1=fail/2=unsupported。
- `crates/ruviform-ui-gpui/Cargo.toml`: `[[example]]
  real_os_pointer_smoke`（`required-features=["gpui"]`）。
  新規依存なし。
- `mise.toml`: `example-ui-gpui-real-os-pointer-smoke` manual
  lane。
- `evidence/rf313-os-capability-matrix/`: verdict-pointer-
  windows.json（実測 verdict）追加、report.md/results.jsonl
  の pointer 行を NotRun → Pass（実測値）に更新。
- `docs/specs/issue-313-windows-realos-smoke.md`: defer 先の
  pointer smoke タスクを本スライスの spec へリンク更新。

### Decisions During Implementation

- **照準は実測値のみ**: `component_bounds` は最後の paint で
  記録された bounds、`window_geometry` は `GetClientRect`+
  `ClientToScreen` の実測値。座標のハードコード・窓中央仮定・
  capture pixel からの逆算は全て却下（design 参照）。
- **geometry は inherent method**: `component_bounds`/
  `window_geometry` は `GpuiDriver` の inherent method で、
  run callback の `&mut dyn WindowDriver` からは届かない。
  `driver.clone()`（`Rc<RefCell>` 共有）を closure に
  capture して query に使う。handler は driver が取り出して
  から invoke するため `inner` の二重 borrow は起きない。
- **Preview は合格条件にしない**: contract 上 coalescible で
  正当に 0 件になり得るため件数のみ記録。Begin+Commit と
  `gesture_id` 一致を要求。
- **カーソル read-back を証拠化**: `SendInput` 受付ではなく
  `GetCursorPos` の実測が commanded ±4px に収まることを check
  化 — 「注入した」ではなく「カーソルが実際にそこへ行った」
  の証拠。
- **`ViewChannels` bundle**: `mounted` の引数追加で発生した
  clippy `too_many_arguments`/`type_complexity` を、inbox/
  slots/bounds の意味的まとまり（driver 所有 channel）で
  解消。振る舞い不変。

## Validation

| 検証 | 結果 |
| --- | --- |
| `cargo check -p ruviform-ui-gpui --features gpui --all-targets` | Pass（exit 0） |
| `cargo clippy -p ruviform-ui-gpui --all-targets --features gpui -- -D warnings` | Pass（exit 0） |
| `cargo test -p ruviform-ui-gpui --features gpui-test` | Pass（69+20+4+7 tests、exit 0。real-window golden 3 件は従来通り ignored） |
| `mise run example-ui-gpui-real-os-pointer-smoke` | **Pass（実測）** — Windows 11 Pro x64 / RTX 4070 SUPER / 2×1920×1080@1.0、verdict=pass exit 0、`verdict-pointer-windows.json` 採取（2026-11-10T04:46:46Z、2 回実行とも pass・intent 値 0.25/0.75/0.8 が一致） |

## Risks

- `SendInput` は foreground window にのみ届く。実行中に
  ユーザーが別 window へ focus を移す・マウスを動かすと
  Fail になり得る（manual lane の運用注意として明記）。
- カーソル復元は best-effort — 復元失敗時はユーザーの
  カーソルが注入終点に残り得る（stderr に記録、verdict
  には影響しない）。
- `component_bounds` は「最後の paint」の記録 — paint と
  注入の間に layout が変わると照準がずれ得るが、静的 fixture
  では問題にならず、ずれた場合は intent 不一致で Fail に
  なる（Pass 偽装なし）。

## Follow-ups

- macOS/Linux の pointer real-OS smoke（各 OS タスク、
  現状 `Unsupported`）。
- 右/中ボタン・修飾キー・ホイール・ドラッグ以外の gesture
  （Issue #313 残スコープ）。
- IME/a11y/DPI 動的変化/モニター切断/2 window（同）。
- `ScenarioDriver::RealOs` adapter 本体（#464 系）。
- consumer（AudioLens/Synvelis/SV-GUI-04/05・#219/#294）側の
  実行方法・制約の周知（report.md の consumer 節を参照）。
