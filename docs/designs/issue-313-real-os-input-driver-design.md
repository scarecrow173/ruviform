---
id: "DESIGN-34VOSINPDRV4REAL0SIN2"
type: "design"
status: "approved"
title: "Issue #313 — `RealOsPointerOrKeyboard` capability 設計（OS-input port on `RealOsGuiTest`）"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-313-real-os-input-driver-plan.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-313-real-os-input-driver.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-313-real-os-input-driver.md"
    - "docs/designs/issue-313-windows-pointer-smoke.md"
    - "docs/designs/issue-314-real-window-adapter.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — `RealOsPointerOrKeyboard` capability 設計

## 設計判断: OS-input op の置き場所

`SemanticGuiTest`/`GuiAction` に OS-input op がない構造的問題への
対処として、3 案を比較した。

| 案 | 内容 | 判定 |
| --- | --- | --- |
| A. contract `GuiAction` 追加 | `GuiAction::OsKey{...}`/`OsPointer{...}` + `SemanticGuiTest` 新メソッド（default `Unsupported`） | **不採用** |
| B. contract 並行 trait | `OsInputGuiTest` を `ruviform-ui-test::scenario` に新設（`VisualGuiTest` 先例） | **不採用** |
| C. adapter 固有メソッド | `RealOsGuiTest` に `os_*` 固有メソッド群（本 design） | **採用** |

### A を不採用とする理由

- `SemanticGuiTest::perform`/`GuiAction` の戻りは
  `Result<(), ScenarioError>`。`OsInputOutcome` の正直な語彙
  （`Injected` の計測証拠・`EnvironmentBlocked` 判定）を運べず、
  `run_scenario` の `verdict_for_error` が `Unsupported` 以外を
  `fail` へ潰す — **environment_blocked を fail に偽装する結果になり
  受入条件に反する**。`ScenarioError::EnvironmentBlocked` 追加は
  `scenario.rs` の許可範囲（`GuiAction`/`ScenarioCapability` 追加のみ）
  を越える。
- `OsKey` は `ruviform-ui-gpui` 側の型。contract に neutral な
  キー語彙を新設すると `os_input::OsKey` と二重定義になる。
- evidence 分離の要件に反して、同じ `perform` 語彙の中に
  semantic dispatch と OS 注入が混在する。

### B を不採用とする理由

- 実装者が `RealOsGuiTest` 1 つしかない段階で contract trait は
  偽の抽象化（YAGNI）。`VisualGuiTest` は Fake/GPUI-simulated/
  RealOs の 3 実装を持つ先例であり状況が異なる。
- `scenario/` 配下の新モジュールは `scenario.rs` の `pub mod` 宣言
  変更を要し、許可された編集範囲を超える。
- S2（windowed `TestIo`/`rvf.uitest.v1`）で contract 側の wire が
  必要になった時点で再評価すればよい。

### C の採用理由

- OS 注入は **window/screen スコープ**（component 非対象の
  keystroke は foreground window へ届く）であり、`GuiAction` の
  component-key 対象モデルと一致しない。
- `run_scenario` の `body(&mut G)` は concrete `G = RealOsGuiTest`
  を受け取るため、固有メソッドをそのまま呼べる — contract runner
  （capability pre-flight・report 組立・timeout 記録）はそのまま
  再利用でき、scenario contract 経由の証拠になる。
- 証拠分離が構造的になる: `SemanticGuiTest` op と `os_*` op は
  別の呼び出し面であり、verdict JSON の `channel` タグで
  `os-injection` vs `semantic` を明記できる。

## 設計詳細

### capability claim

`supported_capabilities()` は `os_input::supported()`
（= `cfg!(windows)`）のとき `ScenarioCapability::RealOsPointerOrKeyboard`
を加える。claim = 「OS 注入 backend が存在する」という静的宣言であり、
**preflight の成否で claim を変えない** — 注入が環境遮断された
Windows host で capability を落とすと verdict が `unsupported` に
偽装され、`environment_blocked` と区別できなくなるため。実行時
gate は op 側の preflight cache が担う。

### `OsInputOutcome`（`os_input` モジュール、public `#[non_exhaustive]`）

```text
Injected { action, commanded, cursor, detail }  // 受理 + read-back 証拠
Unsupported { reason }                          // backend 不在（非 Windows）
EnvironmentBlocked { detail }                   // preflight ineffective
NotRun { reason }                               // 前提未達（未 focus 等）
```

- `Err(ScenarioError)` = contract 違反（stale pin・未知 window/
  component・session 断・部分注入 `UiError::Host`）→ `fail`。
- `Ok(Unsupported)` → `unsupported`、`Ok(EnvironmentBlocked)` →
  `environment_blocked`、`Ok(NotRun)` → `not_run` の lane verdict。
- `os_input` に置く理由: (1) 注入層の正直な結果語彙であり層の
  責務と一致、(2) `pub use gpui_driver::os_input` 経由で外部 test
  から到達可能（`lib.rs` 再エクスポート行の追加不要）。

### `RealOsGuiTest` 追加要素

- `focused: Arc<Mutex<BTreeMap<WindowId, bool>>>` — `run` の event
  handler が `WindowLifecycle::Focused(active)` を記録する共有
  map（`intents` と同じ仕組み）。`window_focused(window)` が読む。
- `os_preflight: RefCell<BTreeMap<&'static str, PreflightCheck>>` —
  family 毎の session-cached preflight。`os_input_preflight(scope)`
  が未検証 family を probe し結果を merge して返す。各 `os_*` op
  は自 family の cache を lazy に評価する（gate 自動組み込み）。
- `request_focus(window, focused)` — `UiJob` 経由で
  `WindowDriver::focus` を呼ぶ（`Op::Focus` → `activate_window`）。
  `focused=false` は driver 側 `Unsupported` をそのまま返す。
- `os_press_key(target, OsKey)` — gate("keyboard") →
  `window_focused` 必須（`NotRun`）→ `UiJob` で pin-check →
  `press_key` → `Injected`。
- `os_pointer_move_to(target, component, frac)` — gate("pointer") →
  `UiJob` で pin-check + `component_bounds` + `window_geometry` →
  `origin + (rect.origin + frac*size) * scale` の screen point →
  `move_to` → `cursor_position` read-back → `Injected{commanded,
  cursor}`。focus 不問（hover aim）。
- `os_mouse_down`/`os_mouse_up(target)` — gate("pointer") → focus
  必須 + pin-check → 注入 → `Injected`。クリックの照準は直前の
  `os_pointer_move_to` の read-back が保証する。

### E2E 証拠（`tests/real_window.rs` 新規テスト）

Windows: `RealOsGuiTest::run` → `require_capabilities` →
`os_input_preflight(PointerAndKeyboard)`（ineffective →
`environment_blocked` 記録 + `panic!("environment_blocked:")`、
`session_failure` 先例）→ `request_focus` → `window_focused` 待ち →
`run_scenario` 内で `os_press_key(Tab/Right/Tab/Left)` と
pointer drag/click → `drain_intents` で atomic pair・非 atomic
gesture を検証 → `post_event` で receipt 返送 → `observe` で
`CommandPhase::Applied` → verdict JSON（env metadata 実測）を
`RUVIFORM_REAL_OS_INPUT_VERDICT` env 指定時に
`evidence/rf313-os-capability-matrix/` へ書き、driver_lab
evidence を emit。

非 Windows（Linux xvfb lane）: capability 未 claim →
`require_capabilities` → `Unsupported` → `unsupported` 証拠を
記録して終了（pass 偽装なし）。

### mise lane

`test-ui-real-os-scenario`（opt-in manual）:
`RUVIFORM_REAL_OS_INPUT_VERDICT=evidence/rf313-os-capability-matrix/
verdict-real-os-input-windows.json` + `RUVIFORM_DRIVER_LAB_EVIDENCE` を
設定し `cargo test -p ruviform-ui-gpui --features real-scenario
--test real_window real_os_input` を実行（既存
`test-ui-real-scenario` と同型・name filter 付き）。
