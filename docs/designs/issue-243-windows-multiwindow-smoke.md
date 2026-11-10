---
id: "DESIGN-034V4sf4ndYXHxJt8IBeXG"
type: "design"
status: "approved"
title: "Issue #243 — Windows 実 OS multiwindow smoke 設計（部分スコープ・後続スライス）"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_multiwindow_smoke.rs"
      - type: "file"
        path: "evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json"
      - type: "file"
        path: "docs/test-specs/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "docs/plans/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "docs/tasks/issue-243-windows-multiwindow-smoke.md"
    modified:
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
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
    - "docs/specs/issue-243-gpui-realwin-windows.md"
    - "docs/specs/issue-313-windows-pointer-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-243-gpui-realwin-windows.md"
    - "docs/designs/issue-313-windows-pointer-smoke.md"
    - "docs/designs/issue-181-multi-window.md"
    - "docs/designs/issue-182-dock-tree.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-243-windows-multiwindow-smoke.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #243 — Windows 実 OS multiwindow smoke 設計（部分スコープ・後続スライス）

## Context

先行スライスの `window_lifecycle_probe` は driver API と
`PostMessageW`（`WM_CLOSE`）で Window/View lifecycle を記録した。
本スライスは同じ lifecycle 面を **実 OS 入力経路**で駆動する:
`os_input` の `press_key`/`move_to`/`mouse_down`/`mouse_up`
（Win32 `SendInput`）に加え、修飾キー chord（Win+Arrow・
Alt+F4）は example-local の `SendInput` batch として実装する
（`os_input` の公開 API は非修飾キーに限定されているため）。

対象は `crates/ruviform-ui-gpui` の manual example であり、
production コード・公開 API は変更しない。証拠は verdict JSON
（`rf243-multiwindow-smoke/v1`）として `evidence/
issue-243-multiwindow-verify/` に出力し、capability matrix
（`evidence/rf313-os-capability-matrix/`）に multiwindow 行を
追記する。

## Scope

### In Scope

- `real_os_multiwindow_smoke.rs` example（phase machine 型、
  bounded retry、全 window の teardown 保証）
- `Cargo.toml` `[[example]]` 宣言、`mise.toml` manual lane
- verdict JSON schema と check 一覧（後述）
- capability matrix report/results への追記

### Out of Scope

- `os_input`/driver/capture/render の変更（#313/#386 の本体は
  利用のみ）
- macOS/Linux・動的 DPI/モニター・Plugin Dock・consumer 実機
- headless/CI lane への組み込み（manual verification のみ）

## Components and Boundaries

- **Fixture app**（example 内）: `UiRuntime` + `DataRegistry`
  + `DemoService` 上に 3 window（alpha/bravo/charlie）を
  `GpuiDriver` で生成。各 window は `ViewHost` に value
  control view（gain/seek 相当 + job 発行ボタン）を mount。
- **Input layer**: `os_input`（keyboard/pointer SendInput）+
  example-local `chord()`（VK 配列 → down…up SendInput batch）。
  いずれも OS automation injection であり物理入力ではない。
- **OS oracle layer**（example 内 Win32 FFI）: `GetForeground
  Window`+`GetWindowTextW`（foreground title）、
  `WindowFromPoint`+`GetAncestor(GA_ROOT)`（hit-test title）、
  `GetCursorPos`（cursor read-back）、`GetWindowRect`/
  `SetWindowPos`（geometry/topmost）、`FindWindowW`（title→
  HWND 解決）。GPUI semantic oracle（`DriverEvent` の
  `Focused`/`Destroyed`/`Resized`/`CloseRequested`、intent
  drain、`JobState`）と独立に記録する。
- **Phase machine**: `Phase` enum 駆動の tick loop。
  `enter`/`tick` は domain helper に分割（clippy
  `too_many_lines` 適合）。各 wait phase は deadline 付きで
  bounded retry し、最終的に `verdict=fail` + 理由を JSON に
  書く。teardown は verdict 書き込み後・失敗時も全 window を
  close し cursor/topmost を復元する。

## Data and Control Flow

1. `WindowCapabilities` probe → `monitors()` → cursor home
   保存 → topmost 化準備。
2. alpha(w1)/bravo(w2) を `open_window` → `WaitReady`
   （両 `Ready` + view attach）→ `Settle`。
3. `Measure`: 両 window の `window_geometry` + component
   bounds を採取し focus point を算出。
4. `ClickB`→`WaitFocusB`: bravo を実クリックで foreground 化
   （OS title oracle + GPUI `Focused(true)` の AND）。
   miss 時は bounded retry。
5. keyboard: bravo 上で Tab/arrow/Page/Home/End を `press_key`
   注入 → intent drain で commit 値を検証。
6. `update_view` で spec 差し替え → view state 保持を確認。
7. Dock: detach→move（2-phase）→abort→reattach を driver API
   で実行し lifecycle trace を検証。
8. `EvalMove`: Win+Arrow chord で alpha を実移動、Win+Down で
   minimize → geometry 消失を記録 → driver `focus` で restore。
9. Jobs: window-owned Job の cancel と app-level Job の生存を
   区別して記録。
10. `EvalRegen`: charlie(w3) を生成 → alpha を Alt+F4 で close
    （Veto→Defer→Approve→無応答既定の choreography）→
    detached view を reattach → charlie を最後に close。
11. 全 check を集計して verdict JSON を書き、cursor/topmost
    を復元して終了。

## Check 一覧（verdict `checks` 要素）

`geometry_measured` / `focus_api_honest`（`focus(w,false)` が
`Unsupported`、未知 window が error）/ `keyboard_commits` /
`update_view_state_kept` / `dock_detach_move_reattach` /
`os_move_minimize_restore` / `regen_fresh_window_id` /
`regen_fresh_view_state` / `view_ownership_after_destroy` /
`per_window_focus` / `close_veto_defer_approve_default` /
`destroyed_lifecycle_trace` / `window_job_cancel_vs_app_job` /
`no_orphaned_views` / `runtime_alive_after_teardown` /
`os_foreground_oracle_consistent` / `cursor_restored`。
（最終名は実装に合わせる）

## Error Handling

- 注入 API の `Err` は `st.failures` に記録し check を fail に
  する（握り潰さない）。
- wait phase は deadline 超過で `verdict=fail` +
  `last_fg`/`fg_log`/`failures` を JSON に残す。
- `SendInput` が受理されてもカーソルが目標に留まらない環境
  （kernel driver による drop、ユーザーの物理入力との競合）
  は cursor read-back で検出し、リトライ上限後に fail とする
  — oracle を弱めて通さない。

## Verification

- `mise run fmt-check` / `check` / `lint-ui-gpui` /
  `test-ui-gpui` / `test-rt`（headless lane）
- `mise run example-ui-gpui-real-os-multiwindow-smoke`
  （manual lane、exit 0=pass/1=fail/2=unsupported）
- verdict JSON の checks と `evidence/rf313-os-capability-
  matrix/` 追記の整合
