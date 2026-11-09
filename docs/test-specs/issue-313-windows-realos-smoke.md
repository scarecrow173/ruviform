---
id: "TSPEC-034UjMKOaa4DzlRpbwWOuL"
type: "test-spec"
status: "approved"
title: "Issue #313 — Windows 実 OS SendInput キーボード smoke 検証契約"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-313-windows-realos-smoke.md"
    - "docs/designs/issue-313-windows-realos-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-479-keyboard-value-controls.md"
    - "docs/test-specs/issue-243-gpui-realwin-windows.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-313-windows-realos-smoke.md"
    - "docs/designs/issue-313-windows-realos-smoke.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-313-windows-realos-smoke.md"
    - "docs/designs/issue-313-windows-realos-smoke.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — Windows 実 OS SendInput キーボード smoke 検証契約

## Purpose

Issue #313 RF-GT-04 の Windows 部分スコープ: 実 OS のキーボード入力
経路（Win32 `SendInput`）で GPUI window の value control が
`WindowLifecycle::Action` intent を生むことを、再現可能な manual
smoke として検証する契約を定める。simulated input
（`dispatch_keystroke`/`simulate_keystrokes`）の結果を real-os の
Pass に数えない（Issue #313「模擬入力との厳密分離」）。

## Feature: 実 OS キーボード入力による値制御 smoke

### Rule: `os_input::press_key` は Windows で `SendInput` の down/up ペアを投入し、投入数不足を失敗として返す

- Example: `OsKey::Tab`/`Right`/`Left` 等の press が `SendInput` で
  2 イベント（keydown+keyup）を投入する。戻り値が期待数未満なら
  `UiError::Host`。非 Windows では `UiError::Unsupported`。
- 検査: `real_os_smoke` 実行時の injector が全キーで `Ok` を返す
  こと（verdict JSON の `keystrokes` 記録）。Windows 実機 manual
  lane でのみ検証可能（headless CI 対象外）。

### Rule: smoke は Ready かつ OS active（`Focused(true)`）を確認してからのみキーを注入する

- Example: `WindowLifecycle::Ready` と `Focused(true)` を観測する
  まで injector は待機する。`Focused(true)` が来ない場合は注入せず
  Fail（フォーカス不明のまま他 window へキーを送らない）。
- 検査: verdict JSON の `checks` に `ready`/`focused` 項目。

### Rule: Tab フォーカス移動 + 矢印キーが 2 つの異なる control に commit intent を生む

- Example: `Tab, Right, Tab, Left` の注入後、`Action` intent のうち
  `params.intent.kind == "commit"` かつ `atomic == true` のものが
  `control_id` = `gain`/`seek` の双方を 1 件ずつ含み、Right 側の
  normalized が 0.5 より大、Left 側が小さい。
- 検査: verdict JSON の `checks` に `commits_distinct_controls` /
  `value_direction` 項目。実値は `intents` 配列に記録。

### Rule: verdict JSON は環境 metadata を含み、exit code が verdict を反映する

- Example: verdict JSON に os/os_version/arch/gpu/dpi(scale_factor,
  monitors)/revision/timestamp/exe を含む。Pass=exit 0、Fail=exit 1。
- 検査: `verdict-windows.json` の実ファイルと report.md の転記が
  一致すること。

## Guarantees

- `SendInput` で注入されたキーが OS のフォアグラウンド経路を通り、
  GPUI の key dispatch → #479 `control_key_input` → `ViewHost` →
  `IntentSink` → `WindowLifecycle::Action` の実経路で intent が届く。
- 未実行・未対応は `NotRun`/`Unsupported`/`Fail` として区別され、
  Pass に偽装されない。

## Non-goals

- ポインタ実入力、IME/a11y、DPI/マルチモニター変化、2 window、
  macOS/Linux、`ScenarioDriver::RealOs` adapter 本体、consumer 側
  受入は本契約の対象外（report.md で NotRun/Unsupported と理由・
  owner を明記する）。
