---
id: "DESIGN-034UjLB6Yn2hveYiujXFY0"
type: "design"
status: "approved"
title: "Issue #313 — Windows 実 OS SendInput キーボード smoke 設計（部分スコープ）"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_smoke.rs"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/report.md"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/results.jsonl"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/verdict-windows.json"
      - type: "file"
        path: "docs/test-specs/issue-313-windows-realos-smoke.md"
      - type: "file"
        path: "docs/plans/issue-313-windows-realos-smoke.md"
      - type: "file"
        path: "docs/tasks/issue-313-windows-realos-smoke.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-313-windows-realos-smoke.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-313-windows-realos-smoke.md"
    - "docs/specs/issue-479-keyboard-value-controls.md"
    - "docs/specs/issue-451-keyboard-tab-nav.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-243-gpui-realwin-windows.md"
    - "docs/designs/issue-314-golden-capture.md"
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-313-windows-realos-smoke.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-313-windows-realos-smoke.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — Windows 実 OS SendInput キーボード smoke 設計（部分スコープ）

## Context

Issue #479 で実装された値制御のキーボード規約（`control_key_input`:
矢印キー ±1 ステップ、PageUp/Down、Home/End、shift=fine）と #451 の
Tab/Shift-Tab フォーカス巡回（`observe_keystrokes` →
`window.focus_next`/`focus_prev`）は、これまで GPUI の
`dispatch_keystroke` / `simulate_keystrokes`（simulated input）でのみ
検証されている。Issue #313 RF-GT-04 は実 OS の入力経路を要求し、
模擬入力を real-os の Pass に数えないことを明示している。

本設計は Windows 限定の部分スコープ: 実 GPUI window を開き、
Win32 `SendInput` でキーストロークを OS 入力キューへ注入し、
`WindowLifecycle::Action` intent が届くことを manual example で
実測する。`ScenarioDriver::RealOs` adapter 本体は対象外。

## Scope

### In Scope

- `gpui_driver/os_input.rs`: `SendInput` ベースのキーストローク注入
  API。Windows では `windows-sys`（`Win32_UI_Input_KeyboardAndMouse`
  feature、既存依存）を使用し、非 Windows では `UiError::Unsupported`
  を返す薄いモジュール。
- `examples/real_os_smoke.rs`: view mount 済み実窓の smoke。
  Ready + Focused(true) を条件に injector thread からキー列を送り、
  Action intent の到達・control 識別・値方向を検証して verdict JSON
  を出力する。
- `mise.toml` の manual lane、`evidence/rf313-os-capability-matrix/`
  の report.md + results.jsonl + verdict JSON。

### Out of Scope

- macOS/Linux、ポインタ実入力、IME/a11y/DPI 変更多数、2 window、
  Native Plugin Dock、RealOs adapter 本体、consumer 受入。
- `render.rs` / 既存 `gpui_driver` の挙動変更、`Cargo.lock`、
  docs index、`.github/`、`.devin/`。

## Components and Boundaries

- **`os_input`（新規、`gpui_driver` 内、`pub`）**: OS 注入の唯一の
  窓口。`OsKey` enum（Tab/Left/Right/Up/Down/PageUp/PageDown/Home/End）
  と `press_key(OsKey) -> Result<(), UiError>`。Windows 実装は
  `INPUT_KEYBOARD` の down/up ペアを 1 回の `SendInput(2, …)` で
  投入し、投入数が 2 未満なら `UiError::Host` で失敗を返す
  （部分注入を成功と偽らない）。`unsafe` は `#[expect(unsafe_code)]`
  + SAFETY コメントの局所許可（surface.rs/capture.rs と同じ先例）。
  非 Windows は `Unsupported` — capability honesty（IME/
  native_surface_slot と同じ「実装しないものは正直に返す」方針）。
- **`real_os_smoke` example**: driver-neutral API
  （`GpuiDriver`/`create_view_window`/`WindowDriver`/`DriverEvent`）
  のみを消費し、gpui 型を持ち込まない（component_showcase と同じ
  境界）。view spec は `Panel[Fader(gain), Seek(seek)]` — tab stop が
  2 つだけの最小構成で、Tab 巡回の行き先を一意に検証できる。
- **injector thread**: `GpuiDriver` は `Rc<RefCell>` で Send では
  ないため、注入は UI thread の外で行う。`mpsc` channel で
  「Ready かつ Focused(true) 観測済み」の合図を受けてから
  `press_key` を呼ぶ。UI thread 側はイベントを記録するだけで、
  `os_input` を直接呼ばない（境界の分離）。
- **verdict JSON**: example が `--out <path>` または
  `RF313_SMOKE_VERDICT` env で指定された path（未指定時は stdout）に
  1 オブジェクトを書き、exit code で Pass/Fail を返す。

## Data and Control Flow

1. example 起動 → 環境 metadata 収集（OS/GPU は Windows では
   PowerShell CIM を best-effort で呼び、失敗時は `"unavailable"`。
   DPI/scale は `driver.monitors()` の実値。revision は
   `--revision` > `RF313_SMOKE_REVISION` env > `git rev-parse HEAD`
   > `"unknown"`。timestamp は unix ms + UTC ISO8601）。
2. `ViewHost` に `Panel[Fader, Seek]` を mount →
   `create_view_window` → `driver.run`。
3. UI thread: `Ready` 受信 → `focus(win, true)` を発行 →
   `Focused(true)` 受信 → injector へ `Inject` を送信。
   `Focused(false)` は記録のみ（注入中のフォーカス喪失は
   intent 不到達として Fail に自然反映）。
4. injector thread: `Inject` 受信 → 400ms 安定待ち →
   `Tab, Right, Tab, Left` を各 ~250ms 間隔で `press_key`。
   （Tab=値を変えず focus 巡回、Right=+1 step、Left=−1 step）
5. UI thread: `Action(intent)` を収集。`op == "rvf.ui.command.v1"`、
   `params.intent.kind == "commit"`、`atomic == true`、
   `control_id ∈ {gain, seek}`、`target` が spec の target と一致、
   normalized が default 0.5 の増減方向と一致することを検査。
   commit が 2 件・control_id が互いに異なり両 control を網羅した
   時点で verdict 確定 → JSON 書き出し → `driver.exit()`。
6. タイムアウト（全体 60s）または注入失敗 → Fail verdict。
   injector が未起動のまま終了する経路では `Abort` を送り、
   スレッドを残さない。

## Non-Functional Constraints

- **RT contract**: non-RT only（UI thread）。`os_input` は manual
  lane からのみ呼ばれ、RT パスに載らない。
- **依存**: 新規 crate 依存なし。`windows-sys` は既存の optional
  依存（`gpui` feature・`cfg(windows)`）の feature 範囲内で済む
  （`Win32_UI_Input_KeyboardAndMouse` は既に有効）。`serde_json` は
  `gpui` feature で既に有効。
- **unsafe**: `SendInput` 呼び出しのみ。`#[expect(unsafe_code)]` +
  SAFETY コメント。他に unsafe を増やさない。
- **lint**: workspace lints（clippy pedantic）を example /
  モジュール両方で通す。`#[non_exhaustive]` は公開 enum
  `OsKey` に付与。
- **実行条件**: 対話 session の前面 window が smoke window である
  こと。実行中にユーザーが別 window へ focus を移すと intent が
  届かず Fail になり得る — manual lane の運用注意として文書化する。

## Risks and Trade-offs

- **SendInput が foreground window に届かない**: `Focused(true)`
  （`observe_window_activation` → `is_window_active`）を注入条件に
  することで、OS が当該 window を active と認識した後にのみキーを
  送る。activate 失敗時は timeout Fail で誠実に記録する。
- **キー注入の副作用**: Tab/Right/Left はフォーカス中の control に
  対する無害な値調整のみ。対象 window 以外へは `Focused(true)`
  観測後にしか送らない。
- **Tab 巡回順の不確かさ**: tab stop は spec 内の 2 control のみ。
  assert は「2 件の commit が異なる control_id を持ち {gain, seek}
  を網羅」で順序非依存にする。
- **値の厳密一致**: step 解決（0.05 plain step）の内部表現に依存
  しないよう、normalized の増減方向（>0.5 / <0.5）と control 識別を
  assert し、実値は verdict JSON に記録する。

## Deferred Design Concerns

- `ScenarioDriver::RealOs` adapter（`RealOsPointerOrKeyboard`）|
  本 dispatch 対象外 | adapter 実装タスクで `os_input` の API を
  再利用するか別途設計する。
- macOS/Linux の注入経路（CGEvent / uinput / XTest）|
  Windows のみ実装 | 各 OS タスクで capability を実測してから
  `os_input` を拡張する。
- ポインタ注入（`SendInput` mouse / `SetCursorPos`）|
  カーソル実占有の副作用 | pointer smoke タスクで座標・
  clip・復元を設計する。

## Verification Notes

- [x] `mise run check-ui-gpui` / `lint-ui-gpui` で example と
  `os_input` が `--all-targets` で型・lint を通る。
- [x] `mise run test-ui-gpui` で既存テストが回 regress しない。
- [x] `mise run example-ui-gpui-real-os-smoke`（新規 manual lane）
  を Windows 実機で実行し verdict JSON を採取。
- [x] `evidence/rf313-os-capability-matrix/` に report/results を
  記録し、#243/#314 証拠へリンク。
