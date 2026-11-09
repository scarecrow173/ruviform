---
id: "IMPL-034UkdjBdy4RDY0x9rZrE0"
type: "impl"
status: "completed"
title: "Issue #313 Windows 実 OS SendInput キーボード smoke と OS Capability Matrix 証拠"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "docs/specs/issue-313-windows-realos-smoke.md"
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
        path: "docs/specs/issue-313-windows-realos-smoke.md"
      - type: "file"
        path: "docs/designs/issue-313-windows-realos-smoke.md"
      - type: "file"
        path: "docs/plans/issue-313-windows-realos-smoke.md"
      - type: "file"
        path: "docs/tasks/issue-313-windows-realos-smoke.md"
      - type: "file"
        path: "docs/test-specs/issue-313-windows-realos-smoke.md"
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
    - "docs/tasks/issue-313-windows-realos-smoke.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-313-windows-realos-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-243-gpui-realwin-windows.md"
    - "docs/designs/issue-314-golden-capture.md"
    - "evidence/rf313-os-capability-matrix/report.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/test-specs/issue-313-windows-realos-smoke.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-313-windows-realos-smoke.md"
  verified-by: []
  references:
    - "docs/specs/issue-313-windows-realos-smoke.md"
    - "docs/designs/issue-313-windows-realos-smoke.md"
  defers:
    - "ScenarioDriver::RealOs adapter 本体（#464 系の別タスク）"
    - "macOS/Linux/pointer/IME/a11y の real-OS smoke（Issue #313 残スコープ）"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #313 Windows 実 OS SendInput キーボード smoke と OS Capability Matrix 証拠

## Summary

Issue #313（RF-GT-04）の Windows 部分スコープを実装・実測した。
`gpui_driver::os_input`（Win32 `SendInput` キーストローク注入、
非 Windows は `Unsupported`）と `real_os_smoke` example を追加し、
実 GPUI window 上で `Ready → Focused(true) → SendInput(Tab,Right,
Tab,Left) → WindowLifecycle::Action commit intent` を実測 Pass
した。`evidence/rf313-os-capability-matrix/` に report/results/
verdict JSON を記録した。Issue #313 本体は Open 継続（macOS/Linux/
pointer/IME/a11y/DPI 変化は NotRun/Unsupported として理由・owner
明記済み）。

## Context

`docs/specs|designs|plans|tasks|test-specs/issue-313-windows-realos-smoke.md`
の文書チェーンに従う。#479 キーボード規約と #451 Tab 巡回はこれまで
GPUI simulated input（`dispatch_keystroke`/`simulate_keystrokes`）
でのみ検証されており、Issue #313 は実 OS 入力経路の証拠を要求する。
#243 の実窓ライフサイクル証拠・#314 `capture_window` は既存で、本
スコープは「実 OS キーボード入力 → intent」のみを補完する。

## Implementation

### Changes

- `crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs`（新規）:
  `OsKey`（`#[non_exhaustive]`、Tab/矢印/Home/End/PageUp/PageDown）
  と `press_key(OsKey)`、`supported()`。Windows は `INPUT_KEYBOARD`
  の down/up ペアを 1 回の `SendInput(2, …)` で投入し、投入数不足は
  `UiError::Host`。非 Windows は `UiError::Unsupported`。
  `unsafe` は `#[expect(unsafe_code)]` + SAFETY コメントの局所許可
  （surface.rs/capture.rs と同じ先例）。
- `crates/ruviform-ui-gpui/src/lib.rs`: `pub mod os_input` と
  `pub use gpui_driver::os_input` のみ追加（既存挙動は不変）。
- `crates/ruviform-ui-gpui/examples/real_os_smoke.rs`（新規）:
  `Panel[Fader(gain), Seek(seek)]` を mount した実窓で、Ready +
  `Focused(true)` 観測後に injector thread が `SendInput` で
  `Tab,Right,Tab,Left` を投入。`WindowLifecycle::Action` の
  commit intent（atomic、両 control、値方向）を検査し、環境
  metadata（OS/GPU/DPI/revision/timestamp）付き verdict JSON を
  出力。exit code: 0=pass / 1=fail / 2=unsupported。
- `crates/ruviform-ui-gpui/Cargo.toml`: `[[example]] real_os_smoke`
  （`required-features = ["gpui"]`）。新規依存なし。
- `mise.toml`: `example-ui-gpui-real-os-smoke` manual lane
  （canonical = mise）。
- `evidence/rf313-os-capability-matrix/`: report.md（日本語、
  SendInput=OS automation injection である旨と物理入力との区別を
  明記）、results.jsonl（case 別 Pass/Fail/Unsupported/NotRun +
  理由 + owner）、verdict-windows.json（実測 verdict）。

### Decisions During Implementation

- **注入は UI thread の外**: `GpuiDriver` は `Rc<RefCell>` で Send
  ではないため、`os_input::press_key` は injector thread から呼ぶ。
  UI thread は `Ready`+`Focused(true)` を観測してから `Go` を送り、
  イベント記録と verdict 判定のみ行う（境界分離）。
- **assert は順序非依存**: Tab 巡回順は tab stop の spec 順に依存
  するため、「+step commit（normalized>0.5）と −step commit
  （<0.5）が異なる control_id で両方観測される」ことのみを合格
  条件にした。実値は verdict JSON に記録。
- **`source` は intent に載らない**: `CommandIntent` の Begin/Commit
  は `source` フィールドを持たない（`InputSource::KEYBOARD` は
  `ControlInput`/`InteractionEvent` meta まで）。keyboard 由来の
  保証は「OS キーイベントのみがこの経路に到達する」構造的事実 +
  `atomic: true` の begin/commit pair で代替し、report に明記。
- **unsupported は exit 2**: 非 Windows では `os_input::supported()`
  = false で窓を開かず verdict=`unsupported`/exit 2。fail(1) と
  区別し「未実行」を「失敗」と偽らない。
- **Abort で注入停止**: verdict 確定時に injector へ `Abort` を送り、
  以後のキーストロークが別 window に漏れないようにした。

## Related Experiments

### Adopted

なし（experiment log なしで完了）。

### Rejected

なし。

## Validation

| 検証 | 結果 |
| --- | --- |
| `mise run check` | Pass（workspace `cargo check --all-targets`、exit 0） |
| `mise run lint` | Pass（workspace clippy `-D warnings`、exit 0） |
| `mise run check-ui-gpui` | Pass（exit 0） |
| `mise run lint-ui-gpui` | Pass（`-D warnings`、exit 0） |
| `mise run test-ui-gpui` | Pass（65+20+4+7 tests、exit 0。real-window golden 3 件は従来通り ignored） |
| `mise run fmt-check` | Pass（exit 0） |
| `mise run example-ui-gpui-real-os-smoke` | **Pass（実測）** — Windows 11 Pro x64 / RTX 4070 SUPER / 2×1920×1080@1.0、verdict=pass exit 0、`verdict-windows.json` 採取（2026-11-09T01:17:27Z、revision `9d7e930`） |

## Risks

- `SendInput` は foreground window にのみ届く。実行中にユーザーが
  別 window へ focus を移すと intent が届かず Fail になり得る
  （manual lane の運用注意として report/docs に明記済み）。
- IME/secure desktop 下では `SendInput` が阻まれる可能性 —
  その場合は Fail として誠実に記録される（Pass 偽装なし）。

## Follow-ups

- `ScenarioDriver::RealOs` adapter 本体（#464 系、別タスク）。
- macOS/Linux の real-OS smoke（各 OS タスク）。
- ポインタ実入力 smoke（カーソル占有の設計が必要、未起票）。
- IME/a11y/DPI 変更多数/2 window（Issue #313 残スコープ）。
- consumer（AudioLens/Synvelis）側受入。
