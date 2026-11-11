---
id: "IMPL-034V5Vzh8RJqYBNZRA9QZW"
type: "impl"
status: "completed"
title: "Issue #243 Windows 実 OS multiwindow smoke 証拠"
created: "2026-11-10"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_multiwindow_smoke.rs"
      - type: "file"
        path: "evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json"
      - type: "file"
        path: "evidence/issue-243-multiwindow-verify/environment-blocked-sendinput.md"
      - type: "file"
        path: "docs/specs/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "docs/designs/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "docs/plans/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "docs/tasks/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "docs/test-specs/issue-243-windows-multiwindow-smoke.md"
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
    - "docs/tasks/issue-243-windows-multiwindow-smoke.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-243-windows-multiwindow-smoke.md"
    - "docs/designs/issue-243-windows-multiwindow-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../ir/issue-243-gpui-realwin-windows.md"
    - "../ir/issue-243-multiwindow-closeout-slice.md"
    - "docs/designs/issue-243-gpui-realwin-windows.md"
    - "docs/designs/issue-243-windows-multiwindow-smoke.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/test-specs/issue-243-windows-multiwindow-smoke.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-243-windows-multiwindow-smoke.md"
  verified-by: []
  references:
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #243 Windows 実 OS multiwindow smoke 証拠

## Summary

Issue #243 の Windows 部分スコープ後続スライスとして、GPUI 標準
Driver で実 Window を 3 つ生成し、window 別 focus・OS close
（Alt+F4: Veto/Defer/Approve/無応答既定）・Dock 2-phase
move/abort/detach/reattach・Win+Arrow 実移動・minimize/restore・
再生成（window id 非再利用 + view state 初期化）・view
ownership・Window-owned Job cancel vs Application-owned Job
生存を、Win32 `SendInput`（keyboard/pointer）+ example-local
修飾キー chord で駆動する manual example
`real_os_multiwindow_smoke` を追加した。GPUI semantic oracle
と独立 OS oracle（foreground title・`WindowFromPoint`・
`GetCursorPos` read-back）を併用する。

実測では pre-refactor binary の 1 回実行で全 check green
（verdict=pass）を記録した。その後の clippy 対応 refactor
後の再実行では、host の `SendInput` がシステム全体で無効化
されている環境退行を検出し、cursor read-back oracle が
「SendInput 受理 ≠ カーソル到達」を fail として正直に記録
した。環境ブロックの切り分け結果は
`evidence/issue-243-multiwindow-verify/environment-blocked-
sendinput.md` に記録。Issue #243 全体は Open 継続
（macOS/Linux・動的 DPI/モニター・Plugin Dock・consumer
回帰は本スコープ外）。

## Context

- spec: `docs/specs/issue-243-windows-multiwindow-smoke.md`
- design: `docs/designs/issue-243-windows-multiwindow-smoke.md`（approved）
- plan: `docs/plans/issue-243-windows-multiwindow-smoke.md`
- test-spec: `docs/test-specs/issue-243-windows-multiwindow-smoke.md`
- task: `docs/tasks/issue-243-windows-multiwindow-smoke.md`
- 先行スライス: `docs/impl/ir/issue-243-gpui-realwin-windows.md`
  （driver API + `PostMessageW` 経路の lifecycle 証拠、PR #482）
- 共有 API: #313 `os_input`（SendInput）・`window_geometry`/
  `component_bounds`・capability matrix（PR #541 merged）

## Environment（全実行共通）

- OS: Windows 11 Pro x64・対話 console session（非 RDP）
- toolchain: mise 管理 rustc 1.99.0・GPUI 0.2.2
- revision: `d7b1f013`（origin/main merge 後）
- 実施日: 2026-11-10
- モニタ環境: 1920x1080 × 2 面（scale=1.0 = 100% DPI、静的）

## Implementation

### Changes

- `crates/ruviform-ui-gpui/examples/real_os_multiwindow_smoke.rs`
  （新規・manual verification 専用、`required-features = ["gpui"]`、
  CI 対象外、約 1900 行）
  - phase machine 型 fixture: `enter`/`tick` を domain helper に
    分割（clippy `too_many_lines` 適合）。wait phase は deadline
    付き bounded retry、teardown は verdict 書き込み後・失敗時も
    全 window close + cursor/topmost 復元を保証。
  - 3 window（alpha=w1 / bravo=w2 / charlie=w3）を `GpuiDriver`
    で生成。charlie は alpha 破棄後に再生成し window id 非再利用
    を検証。
  - 実入力駆動: `os_input` の `press_key`/`move_to`/`mouse_down`/
    `mouse_up`/`cursor_position`（Win32 `SendInput`/`GetCursorPos`）
    + example-local `chord()`（VK 配列 → down…up `SendInput`
    batch。`os_input` 公開 API は非修飾キー限定のため example 内
    実装）。Alt+F4 close・Win+Arrow move/minimize・Win+Up restore
    を chord で駆動。
  - 独立 OS oracle（example 内 Win32 FFI）: `GetForegroundWindow`
    +`GetWindowTextW`（foreground title）、`WindowFromPoint`+
    `GetAncestor(GA_ROOT)`（hit-test title）、`GetCursorPos`
    （cursor read-back）、`GetWindowRect`/`SetWindowPos`
    （geometry/topmost）、`FindWindowW`（title→HWND）。
  - `click_at` は `move_to` error を `st.failures` に記録し、
    click 前に cursor read-back で実位置を検証、foreground title
    を `fg_log` に記録する（「SendInput 受理 ≠ カーソル到達」
    を検出可能化）。
  - Job policy: window-owned Job の cancel と app-level Job の
    生存を区別して `job_events`/`receipts` に記録。
  - verdict JSON（`rf243-multiwindow-smoke/v1`）: 環境 metadata・
    checks・traces・focused・`os_foreground_log`・`intents`・
    `job_events`・`receipts`・geometry・monitors・capabilities・
    run 統計を出力。
- `crates/ruviform-ui-gpui/Cargo.toml`: `[[example]]` 宣言追加。
- `mise.toml`: `example-ui-gpui-real-os-multiwindow-smoke` task
  追加（canonical 実行入口。exit 0=pass/1=fail/2=unsupported）。
- `evidence/issue-243-multiwindow-verify/`:
  - `verdict-multiwindow-windows.json` — 最終コードの実機 run
    verdict（環境ブロックにより fail。click cursor read-back が
    commanded point に到達せず foreground oracle が無関係 window
    を示した記録）。
  - `environment-blocked-sendinput.md` — SendInput 無効化の
    切り分け記録（本 IR の Decisions 参照）。
- `evidence/rf313-os-capability-matrix/{report.md,results.jsonl}`:
  #243 multiwindow slice の判定行を追記（Pass（前回実測）・
  NotRun（再検証・macOS/Linux・DPI/モニター・Plugin Dock・
  consumer 非回帰）を誠実に分類）。
- docs チェーン: spec/design/plan/task/test-spec/impl-ir。

### Decisions During Implementation

- **phase machine の寿命 bug 修正**: 最後の window が予期せず
  close されると GPUI が quit する問題を、close/regenerate の
  順序を「生存 window を残す」不変条件に修正して解消。
  修正後の run で全 check green を 1 回実測。
- **click oracle 強化**: 初版は `move_to` error を握り潰し
  `WindowFromPoint` の hit-test math のみを信用していた。
  実カーソル未到達を検出できないため、`cursor_position()`
  read-back + `move_to` error 記録 + foreground title 記録に
  強化。これが環境ブロックの検出を可能にした。
- **DockOps `DuplicateView` 修正**: view は `WaitReady` で
  auto-attach 済みのため、duplicate-view 経路を実態に合わせて
  修正。
- **clippy 対応 refactor**: `enter`/`tick` を domain helper に
  分割し 27 件の lint error を解消（collapsible match・
  oversized functions・pass-by-value 等）。振る舞い保存の
  構造変更であり、実 OS での再検証は環境ブロックにより未実施
  — この gap を証拠・docs に明記した。
- **Win+Down minimize → restore**: `EvalMove` で Win+Down による
  minimize と geometry 消失を記録し、driver `focus`（activate
  経路）で restore する順序に修正。`activate_window()` が
  iconic window を restore することを利用。
- **Alt+F4 2 回目 click の flaky 対策**: activation click に
  bounded retry を追加。
- **環境ブロックの判定**: post-refactor run が
  `per_window_focus_real_click` で fail（commanded
  (1140,152)・cursor read-back (586,1079)・foreground
  `タスク マネージャー`）。host-level 切り分けで
  **SendInput が system-wide に無効**（受付を返すが注入
  イベントが一切効果を持たない）と判定。`mouse_event`/
  `keybd_event`/`SetCursorPos`/`WScript SendKeys` との対比、
  Task Scheduler context での再現、`vgk`/`vgc`/`vgtray` 停止、
  LG HUB/GameInput proxy kill、class filter・BlockInput・
  RDP・IL 差異の否定を `environment-blocked-sendinput.md`
  に記録。fixture/backend の変更は行わず、環境要因として
  記録に留めた（#313 owner の判断に委ねる — spec の
  Deferred Design Concerns どおり）。

## Related Experiments

### Adopted

<!-- 実験ログなし（既知解の機械的作業 + 診断は環境切り分けとして
     environment-blocked-sendinput.md に記録） -->

### Rejected

<!-- なし -->

## Validation

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | `cargo fmt --all -- --check` clean |
| `mise run check` | 0 | `cargo check --workspace --all-targets` clean |
| `mise run lint` | 0 | workspace clippy `-D warnings` clean |
| `mise run lint-ui-gpui` | 0 | `cargo clippy -p ruviform-ui-gpui --all-targets --features gpui -- -D warnings` clean |
| `mise run test` | 0 | workspace tests + doc-tests pass |
| `mise run test-rt` | 0 | RT lane pass |
| `mise run test-ui-gpui` | 0 | `cargo test -p ruviform-ui-gpui --features gpui-test` 104 tests pass |
| `mise run check-ui-gpui` | 0 | `cargo check -p ruviform-ui-gpui --features gpui --all-targets` clean |
| `mise run example-ui-gpui-real-os-multiwindow-smoke`（pre-refactor binary） | 0 | verdict=pass・全 check green（1 回実測） |
| 同（post-refactor binary・最終コード） | 1 | verdict=fail — `per_window_focus_real_click` 他 4 check が環境ブロックで fail。`verdict-multiwindow-windows.json` に記録 |
| host-level SendInput preflight（`SendInput`→`GetCursorPos` read-back） | — | `sent=1` だが cursor 不動・NumLock 不変。`mouse_event`/`keybd_event`/`SetCursorPos` は正常。system-wide ブロック確認 |

## Risks

- ~~**post-refactor binary の実 OS 未検証**~~ **解消（2026-11-11）**:
  Vanguard unload 後の PR #551 再実走（schema v1）と closeout slice での
  再実走（schema v2・HEAD `4eea9115`・`mise run
  example-ui-gpui-real-os-multiwindow-smoke` exit 0・verdict=pass・
  preflight effective・16 checks green）で最終コードの実 OS 直接証拠を
  取得。詳細は `docs/impl/ir/issue-243-multiwindow-closeout-slice.md`。
- ~~**SendInput 環境ブロックの原因未同定**~~ **解消（2026-11-11）**:
  Riot Vanguard `vgk.sys` armed 状態が最有力原因（driver 完全 unload 後に
  SendInput 全回復。因果は未確証 — 時系列相関ベース）。#552 の
  `os_input::preflight` fail-closed で再発時は exit 3
  `environment_blocked` で明示的に落ちる。詳細は
  `evidence/issue-243-multiwindow-verify/environment-blocked-sendinput.md`
  Resolution 節。
- **実行中のユーザー物理入力との競合**: manual lane は実カーソルを
  使用するため、実行中のマウス/キーボード操作は結果を変え得る
  （運用注意として docs に明記）。

## Follow-ups

- ~~**再検証**~~ **実施済み（2026-11-11）**: 上記 Risks 参照。canonical
  verdict JSON は v2 schema・preflight 付きで再生成済み。
- ~~**SendInput ブロック原因の特定**~~ **実施済み（2026-11-11）**:
  Vanguard armed 状態が最有力仮説として記録（因果未確証は明記）。
  恒久 filter 再発時は preflight が exit 3 で fail-closed するため、
  backend 代替判断は再発時に #313 owner へ。
- **macOS/Linux・動的 DPI/モニター・モニター切断**: 本 dispatch
  環境外。後続 #243 dispatch。
- **Native Plugin Dock**: #388/#389・Synvelis #176 の OS 別 Gate。
- **consumer 非回帰**: audio-lens#166・synvelis#113/#114 が owner。
