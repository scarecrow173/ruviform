---
id: "SPEC-034UjKJM3JWo1yfqUTlRka"
type: "spec"
status: "approved"
title: "Issue #313 — Windows 実 OS SendInput キーボード smoke と OS Capability Matrix（部分スコープ）"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-313-windows-realos-smoke.md"
      - type: "file"
        path: "docs/test-specs/issue-313-windows-realos-smoke.md"
      - type: "file"
        path: "docs/plans/issue-313-windows-realos-smoke.md"
      - type: "file"
        path: "docs/tasks/issue-313-windows-realos-smoke.md"
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
    - "docs/specs/issue-479-keyboard-value-controls.md"
    - "docs/specs/issue-243-gpui-realwin-windows.md"
    - "docs/specs/issue-314-golden-capture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
    - "docs/specs/issue-312-test-gui-host-lifecycle.md"
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

# Issue #313 — Windows 実 OS SendInput キーボード smoke と OS Capability Matrix（部分スコープ）

## What Should Be Built

RF-GT-04（Issue #313）の Windows 部分スコープとして、実 GPUI window に
value control（Fader/Seek）を mount し、OS レベルのキーボード注入
（Win32 `SendInput`）で Tab フォーカス移動と矢印キーによる値調整を送り、
`WindowLifecycle::Action` intent が OS 入力経路で到達することを検証する
manual smoke example（`real_os_smoke`）と、その実測結果を記録する
OS Capability Matrix 証拠（`evidence/rf313-os-capability-matrix/`）を
整備する。併せて consumer が再現できる manual mise lane を追加する。

## Why Now

#479 で Seek/Fader/Knob/Toggle の共通キーボード規約（Tab フォーカス +
矢印キーで値調整）が実装され、#451 で Tab/Shift-Tab のフォーカス巡回が
配線された。しかし既存の検証は GPUI の `dispatch_keystroke` /
`simulate_keystrokes`（simulated input）に留まり、**実 OS の入力経路**
（物理キーボード相当の `SendInput`）で intent が届くことは未実測である。
Issue #313 は「実際のユーザー入力経路を通る Smoke と OS 別可否の証拠化」
を要求しており、#479 spec の K4（実窓でのキー操作の実機証拠、NotRun）
の受け皿が本 Issue である。

## Users and Value

- Ruviform の consumer（Synvelis / AudioLens 等）が、実 OS 上で
  キーボード操作 → intent → receipt の経路が機能することを
  再現可能な手順で確認できる。
- Issue #313 の OS 別 Capability Matrix の Windows 行について、
  推測ではなく実測値（Pass/Fail）と環境 metadata を証拠化できる。

## Scope

### In Scope

- `crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs`: Win32
  `SendInput` によるキーストローク注入の薄いモジュール
  （Windows のみ実装、他 OS は `Unsupported` を正直に返す）。
- `crates/ruviform-ui-gpui/examples/real_os_smoke.rs`: 実 GPUI window
  に value control を mount し、Ready + Focused(true) を待ってから
  `SendInput` で `Tab → Right → Tab → Left` を送信し、OS 入力経路で
  `WindowLifecycle::Action` intent（`rvf.ui.command.v1` /
  `kind: commit`、`atomic: true`）が 2 つの異なる control に届くことを
  検証する。verdict JSON に環境 metadata（OS・GPU・DPI・revision・
  実行日時）を含める。
- `evidence/rf313-os-capability-matrix/`: report.md（OS Capability
  Matrix の Windows 行の実測記録、#243/#314 証拠へのリンク、
  SendInput = OS automation injection であり物理手動入力と区別する
  旨の明記）と結果 JSONL。
- `mise.toml` への manual lane 1 件と、consumer 向け実行方法の記録。

### Out of Scope

- macOS / Linux の全項目（本 dispatch の環境は Windows のみ）。
- IME・a11y（OS 支援技術）・DPI/マルチモニター・モニター切断・
  2 window・マウス/ポインタ実入力（カーソル実占有を伴うため
  NotRun とし理由を明記）。
- Native Plugin Dock（#388/#389 が owner）。
- `ScenarioDriver::RealOs` / `RealOsPointerOrKeyboard` capability の
  adapter 本体の実装（別タスク。本 dispatch は example 経路の
  直接検証）。
- AudioLens/Synvelis での consumer 側受入。
- 既存の `dispatch_input` / `simulate_keystrokes` 経路・
  `render.rs`・既存 gpui_driver コードの挙動変更。

## Requirements

- 実 OS 入力経路の検証であること: `SendInput` は OS の入力キューへ
  イベントを注入する automation であり、「物理的手動入力」とは
  区別して証拠に明記する。GPUI `dispatch_input` / simulated
  keystroke を real-os の Pass に数えない（Issue #313 横断改訂の
  「模擬入力との厳密分離」に従う）。
- skip-as-pass 禁止: 未実装・attach 不能・環境要因の未実行は
  Pass に偽装せず `Unsupported`/`NotRun`/`Fail` を区別する。
- 再現可能性: consumer が同じコマンドで再実行でき、verdict JSON に
  OS・GPU・DPI・revision・実行日時を含める。
- 製品挙動を変えない: `os_input` は検証経路として追加し、既存経路を
  置き換えない。OS 固有の仕組みはテスト依存（example / manual lane）
  に閉じる。

## Acceptance Criteria

- [ ] `real_os_smoke` example が Windows 実機で実行でき、
  Ready → Focused(true) → `SendInput` キーストローク →
  `WindowLifecycle::Action` intent 到達を検証し、verdict を
  JSON で出力する（環境 metadata 込み）。
- [ ] `evidence/rf313-os-capability-matrix/report.md` に Windows 行の
  実測 Pass/Fail と根拠、#243・#314 の証拠へのリンク、
  SendInput の性質（OS automation injection）の明記を含む。
- [ ] 結果 JSONL にケース毎の verdict（Pass/Fail/Unsupported/NotRun
  + 理由）を記録する。
- [ ] manual mise lane が追加され、実行方法が文書化される。
- [ ] ポインタ実入力は本 dispatch では NotRun とし理由を明記する。
- [ ] `mise run test-ui-gpui` / `check` / `lint` / `fmt-check` が
  通る（既存挙動を壊さない）。

## Deferred Design Concerns

- `ScenarioDriver::RealOs` adapter 本体（`RealOsPointerOrKeyboard`
  capability を含む）| 本 dispatch は example 経路の直接検証に限定 |
  別タスクで adapter を実装する際に本 smoke の知見を接続 |
  adapter 実装時に capability 判定と証拠形式の整合が必要。
- macOS / Linux の実 OS 計測 | 本環境は Windows のみ |
  各 OS 環境での実行時に同じ example/lane を再利用して行を埋める。
- ポインタ（マウス）実入力 | カーソル実占有を伴い自動実行の副作用が
  大きい | pointer 対応タスクで座標解決・focus 契約と併せて設計する。

## Sources and Relations

- Issue #313 本文（RF-GT-04、GUI Testing v1.0、2026-11-08 横断改訂の
  Real OS Adapter / 模擬入力との厳密分離 / 実行証拠の最低要素）。
- `docs/specs/issue-479-keyboard-value-controls.md` — K4（実窓キー操作の
  実機証拠）を本 Issue が引き受ける。
- `docs/specs/issue-243-gpui-realwin-windows.md` /
  `docs/impl/ir/issue-243-gpui-realwin-windows.md` — 実窓 lifecycle の
  先行実測手法（manual example + 証拠記録）の先例。
- `docs/specs/issue-314-golden-capture.md` — `capture_window` による
  実窓 pixel capture（本 dispatch では未使用、証拠リンクのみ）。
- `docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md` —
  `ScenarioDriver::RealOs` capability enum の定義元（adapter 本体は
  対象外）。
