---
id: "SPEC-01KBZWINDOWIME0SPECDOC0000A1"
type: "spec"
status: "approved"
title: "Issue #389 real-window-ime — ScenarioCapability::Ime claim + OS-level IMM32 composition drive + WM_IME_* 配達 oracle + 正直な fallback"
created: "2026-11-21"
updated: "2026-11-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-389-window-ime.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-389-ime-verbs.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-window-ime.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #389 real-window-ime — ScenarioCapability::Ime claim + OS-level IMM32 composition drive + WM_IME_* 配達 oracle + 正直な fallback

## What Should Be Built

canonical real-dock lane（`evidence/rf631/plugin-dock-real-scenario.jsonl`）
の `real-window-ime` case を `Unsupported` から実判定へ flip する:

```
adapter.supported_capabilities() claims Ime  (os_ime::supported() gate)
→ real docked attach of test_editor (existing lease chain)
→ os_ime::preflight(child_hwnd)             (IME installed/enabled)
→ os_ime::drive_composition(child_hwnd, "かな")
     SetFocus → ImmGetContext → ImmSetOpenStatus
     → ImmSetCompositionStringW(SCS_SETSTR|GCS_COMPSTR)
     → ImmNotifyIME(NI_COMPOSITIONSTR, CPS_COMPLETE) → ImmReleaseContext
→ os_ime::collect_ime_messages(child_hwnd)
     caller-side GWLP_WNDPROC tap + bounded same-thread pump
     → WM_IME_* / WM_CHAR delivered-message log
→ verdict lattice (Pass / Fail / NotRun+environment_blocked / Unsupported)
→ session stays Attached, events [Attached{Docked}, Closed],
   close convergence (unchanged lifecycle contract)
```

- **`os_ime` backend module**（`crates/ruviform-ui-gpui/src/
  gpui_driver/os_ime.rs` 新設 — `os_input.rs` 同型）:
  `supported()`（backend presence = `cfg!(windows)`）、
  `preflight(hwnd)`（`PreflightReport` — `ime_layout`/
  `ime_context`/`ime_ui` checks）、`drive_composition(hwnd, text)`
  （`OsImeOutcome::{Injected, Unsupported, EnvironmentBlocked,
  NotRun}` — `#[non_exhaustive]`）、`collect_ime_messages(hwnd,
  budget, limit)`（`ImeDelivery` — tap 観測ログ + 解析 flag +
  `commit_confirmed()`）。OS seam は `ImeProbe` trait で mockable。
  `windows-sys` に `Win32_UI_Input_Ime` feature を追加。
- **capability claim**（`real_window.rs`）:
  `supported_capabilities()` が `os_ime::supported()` のときだけ
  `ScenarioCapability::Ime` を挿す — presence-only gate、IME
  install/enabled の実効は含めない（`RealOsPointerOrKeyboard` と
  同規約 — runtime 環境不成立は case verdict 側で診断）。
- **case body**（`real_dock.rs` `real_window_ime_case`）:
  既存 `editor_op_body` 骨格を再利用し、`observe_editor_ime_
  lifecycle` が child HWND attach contract → preflight → drive →
  delivery oracle → session-state → close convergence を段階記録。
  `OpLanding::Blocked` を新設し、`os_ime` の blocked/not-run を
  `NotRun` verdict（環境起因時は extras `observed:
  "environment_blocked"` marker 付き）へ写像。
- **catalog 行**（`real_dock.rs` `real_entries` +
  `real_catalog.rs`）: `required` を `[NativePluginDock, Ime]` に
  更新（body が実 lease chain を要するため NativePluginDock も
  必須 — `Ime` 単独では capability preflight が body 未到達で
  片方だけ fail する）。

## Why Now

canonical lane の #389 残件 unsupported 3 行のうち、
`real-window-ime` だけが bounded slice で解ける（crash/hang は
in-process lane の構造限界で別 Issue 級）。本ホストで IMM32 駆動
可能な MS-IME が実測確認済みであり、floor deliverable（blocked
維持）ではなく実 drive path が取れる。

## Users and Value

- real lane の #389 残件を 1 行減らし、`Ime` capability の
  backend-presence claim に実証根拠を与える。
- 「plugin editor が IME 経路を受けられる環境で、OS-level の
  composition が実際に HWND へ配達される」ことを非画素 oracle で
  検証する基盤ができる（semantic editor IME 処理は別 capability
  として分離記録 — 今回 claim しない）。

## Scope

**含む**:

- `os_ime.rs` 新設（IMM32 drive + preflight + tap/pump oracle +
  mockable seam + unit tests）。
- `real_window.rs` の `Ime` capability claim（presence gate）+
  既存 assert の更新。
- `real_dock.rs` の `real_window_ime_case` 実装 +
  `OpLanding::Blocked` + `editor_op_body` の verdict arm。
- `real_catalog.rs` 行の `required` 更新。
- `ruviform-plugin-dock-test/README.md` の `real-window-ime` 記述。
- doc chain（discovery/spec/test-spec + design 改訂節）。

**含まない（対象外）**:

- `real-session-crash`/`real-session-hang`（in-process 限界 —
  別 Issue 級）。
- VST3/CLAP plugin GUI ABI への IME verb 追加、session verb・
  `HostCommand`・wire の変更（捏造禁止 — `L1EditorOps` refusal-
  only 不変、`WIRE_VERSION` 不変）。
- `EditorCapabilities.ime`/`SlotCapabilities.ime`/
  `WindowCapabilities.ime` の語彙 claim（OS-delivery ≠ semantic
  editor IME — `driver.rs` 非対象）。
- `AttachMode::Floating`、plugin→host GUI 要求 channel、実 Window
  RT oracle、GPUI host window 自身の IME 経路（別 capability と
  して分離）、`mise.toml` 構造変更、Issue close。
- fixture 側（`editor_view.rs`）subclass oracle — unload-safety
  制約のため不採用（条件付き lease は使用しない）。
- `real-session-move`（第 2 slot lease への reattach move —
  本 slice の AC に観測価値を増さず別 case 級、discovery で採否
  記録済み）。

## Requirements

1. `os_ime::supported()` は IMM32 backend presence のみを報告し、
   IME install/enabled の実効は `preflight`/`drive` の診断責務に
   分離する。
2. `drive_composition` は IMM32 で composition+commit を OS-level
   駆動し、accepted/refused を `OsImeOutcome` で正直に返す —
   `Injected` は「API が受理した」のみを意味し、delivery を含意
   しない。
3. `collect_ime_messages` は caller-thread 所有 HWND に限定された
   caller-side tap + bounded pump で配達ログを観測し、oracle 機構
   が動かなかった場合を `observed: false` で区別する（pump 不能と
   「何も配達されなかった」を混同しない）。tap の proc は lane
   binary 内コードで unload せず、元 WndProc は全経路で復元する。
4. verdict 写像:
   - accepted drive + `commit_confirmed()`（START → RESULTSTR-flag
     COMPOSITION → END + committed text 流通）→ `Pass`
   - preflight `effective == false` / drive `EnvironmentBlocked`
     → `NotRun` + extras `observed: "environment_blocked"`
   - drive `NotRun`（dead HWND/foreign thread/empty text）→
     `NotRun`（marker 無し）
   - accepted drive で `commit_confirmed()` false → `Fail`
     （accepted-without-effect は契約不一致）
   - delivery `observed == false` → `NotRun`（accepted-but-
     unverifiable — `verified` は true のまま）
   - `supported() == false`（非 Windows）→ capability 未 claim →
     capability preflight `Unsupported`（body 未到達）
5. `Ime` claim は `os_ime::supported()` にのみ依存し、scenario
   catalog の存在には依存しない（capability テストが
   `caps.contains(&Ime) == os_ime::supported()` を assert）。
6. honesty 境界: session verb・`HostCommand`・wire・fixture への
   IME 経路は追加しない。判定は OS-delivery 層のみ。
7. 既存 case への非回帰: 他 28 case の verdict 写像・判定点を
   変更しない（`OpLanding::Blocked` は新設で既存 arm 不変、
   `Blocked` arm の追加は `landing` が `Blocked` のときだけ発火）。
8. cleanup: 一時 subclass・focus・IME open status・context は
   drive/観測終了時に可能な限り復元し、cleanup 失敗は evidence に
   記録して pass 化に使わない。

## Acceptance Criteria

- [ ] `supported_capabilities()` が `os_ime::supported()` のとき
  だけ `Ime` を含み、capability テストが green。
- [ ] `real-window-ime` が interactive Windows lane で実走され、
  case verdict が lattice どおり判定される（Pass または正直な
  NotRun/environment_blocked/Fail）。
- [ ] IME 不在/無効/refused 環境では `NotRun` +
  `environment_blocked` marker が記録され、Pass 捏造がない。
- [ ] accepted drive で commit sequence が配達されない場合は
  `Fail`（NotRun への降格なし）。
- [ ] headless/非 Windows 環境では `Unsupported`（capability
  preflight）または正直な `NotRun`/`environment_blocked` が
  記録される。
- [ ] `mise run check/lint/test`（対象 crate + feature 面）、
  `fmt-check`、`test-ui-scenario-contract`、
  `check-ui-driver-boundary`、headless lane が green。
- [ ] `evidence/rf631/plugin-dock-real-scenario.jsonl` に新規
  `real-window-ime` 行が append され、既存行は書き換えない。
- [ ] 他 28 case の verdict が変わらない。

## Deferred Design Concerns

- `EditorCapabilities.ime`/`SlotCapabilities.ime`/
  `WindowCapabilities.ime` の semantic 語彙 claim — OS-delivery
  では editor が IME を処理したことを証明できない。fixture が
  `IPlugView` 経由で IME 認識可能なイベントを受ける経路
  （VST3/CLAP ABI 上存在しない）か、別 oracle が必要。
- GPUI host window 自身の IME 経路（host IME UI の表示等）は別
  capability として分離記録。
- `real-session-move`（第 2 slot lease reattach）— 別 case 級。
- out-of-process editor host（crash/hang 前提）— 別 Issue 級。
- 日本語以外の IME / 複数 IME 環境での composition text 選定と
  preflight 粒度の精緻化。

## Sources and Relations

- `docs/discovery/issue-389-window-ime.md`（本 spec の確定根拠 —
  ホスト実測・oracle 選定・採否記録）。
- `docs/specs/issue-389-ime-verbs.md`（editor-verb slice — 本
  slice は verb 非追加で OS-delivery 層のみ）。
- `docs/designs/issue-389-plugin-editor-session.md`（改訂 5 で
  着地記録）。
- `docs/test-specs/issue-389-window-ime.md`（`verifies` 本 spec）。
