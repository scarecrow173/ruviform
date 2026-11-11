---
id: "TSPEC-034V4sgGc35MAPIRgG7lmm"
type: "test-spec"
status: "implemented"
title: "Issue #243 — Windows 実 OS multiwindow smoke 検証契約"
created: "2026-11-10"
updated: "2026-11-11"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
    - "docs/designs/issue-243-windows-multiwindow-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-243-gpui-realwin-windows.md"
    - "docs/test-specs/issue-313-windows-pointer-smoke.md"
    - "docs/test-specs/issue-313-windows-realos-smoke.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
    - "docs/designs/issue-243-windows-multiwindow-smoke.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
    - "docs/designs/issue-243-windows-multiwindow-smoke.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #243 — Windows 実 OS multiwindow smoke 検証契約

## Purpose

Issue #243 の Windows 部分スコープ: 実 GPUI window を 2 つ
以上生成し、window 別 focus・close・detach/reattach・移動・
再生成を **実 OS 入力経路**（Win32 `SendInput` + example-local
chord）で駆動して lifecycle 契約が成立することを、GPUI
semantic oracle と独立 OS oracle の双方で検証する契約を
定める。simulated input・`PostMessageW` のみの経路・
`#[ignore]` skip を real-os の Pass に数えない。

## Feature

実 OS multiwindow lifecycle — 実 Window の生成・focus・
close・Dock 操作・移動・再生成が、実ユーザー入力相当の
OS 注入で期待どおりの semantic/OS 状態遷移を起こすこと。

## Rules

- **R1 実 Window 要件**: `GpuiDriver` で OS handle を持つ
  実 window を 2 つ以上生成する。`gpui-test` backend の
  window は real-os 証拠に使わない。
- **R2 実入力要件**: focus 取得・close・移動の少なくとも
  一部を `SendInput`（keyboard/pointer）または example-local
  `SendInput` chord で駆動する。driver API のみの操作は
  semantic oracle 補助として許容するが、「実入力で検証した」
  項目と区別して記録する。
- **R3 独立 oracle**: 各実入力 check は GPUI semantic
  （`DriverEvent`/intent/`JobState`）と独立 OS 観測
  （`GetForegroundWindow` title・`WindowFromPoint`・
  `GetCursorPos` read-back）の双方で判定する。片方のみの
  成功を Pass にしない。
- **R4 注入不達の検出**: `SendInput` が受理を返しても実
  cursor が目標に留まらない場合（kernel driver による
  drop・物理入力との競合）、cursor read-back で検出し
  bounded retry 後に Fail とする。oracle を弱めて通さない。
- **R5 close choreography**: OS close（Alt+F4）は
  Veto→Defer→Approve→無応答既定 Approve の順で disposition
  を検証し、各段階で window/view が orphan にならないこと。
- **R6 再生成**: window 破棄後の再生成は新しい window id を
  持ち、view state は初期化される（stale handle 再利用・
  二重破棄・孤立 view がない）。
- **R7 Job policy 区別**: Window-owned Job の cancel と
  Application/Project-owned Job の生存を区別して記録する。
  consumer 実機非回帰は本契約の対象外（別 Gate）。
- **R8 honesty**: `SendInput` は OS automation injection で
  あり物理手動入力ではない。macOS/Linux・動的 DPI/モニター
  変動・Plugin Dock・consumer 非回帰は NotRun/Unsupported
  として理由つきで記録し、Pass に偽装しない。
- **R9 teardown**: verdict 書き込みの成否に関わらず全実
  window を close し、cursor 位置と topmost 状態を復元する。

## Examples

- **E1 focus**: Given bravo window が foreground でない
  When bravo client-area を実クリック
  Then OS foreground title が `rf243 bravo` になり、かつ
  GPUI `Focused(true)` が bravo に発行される。
- **E2 close**: Given alpha が `CloseDisposition::Veto` を
  返す When Alt+F4 chord を注入 Then window は残り
  `CloseRequested`+`Veto` が trace される。Approve 設定後の
  再送で `Destroyed` になる。
- **E3 regenerate**: Given alpha 破棄済み When charlie を
  `open_window` Then 新しい `WindowId`（w3 ≠ w1）が発行され、
  charlie の view は fresh state で mount される。
- **E4 job policy**: Given window-owned Job と app-level Job
  が running When 対象 window を close Then window-owned
  Job は cancel され app-level Job は生存する。

## Guarantees

- verdict JSON（`rf243-multiwindow-smoke/v1`）に環境
  metadata・各 check の bool+詳細・`fg_log`・`failures`・
  `elapsed_ms` を記録する。
- `verdict=pass` は全必須 check が true の場合のみ。
- 失敗時も JSON を書き、`failures`/`fg_log`/`last_fg` で
  原因を追跡可能にする。
- capability matrix の multiwindow 行は本 verdict と整合
  する（Pass/Fail/Unsupported/NotRun の誠実な分類）。

## Non-goals

- macOS/Linux の実機検証（環境なし、NotRun）。
- 動的 DPI 変更・モニター切断・別モニター移動の変動系
  （静的 2 モニター・100% DPI のみ実測）。
- Native Plugin Dock 受入（#388/#389・Synvelis #176）。
- consumer（AudioLens/Synvelis）実機非回帰。
- 真の OS リソース枯渇による window 生成失敗。
- `os_input`/driver 本体の変更（利用のみ）。
