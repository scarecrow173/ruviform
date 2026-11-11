---
id: "SPEC-034V4nfv1RJmI8TqYPEWcY"
type: "spec"
status: "implemented"
title: "Issue #243 — Windows 実 OS multiwindow smoke（部分スコープ・後続スライス）"
created: "2026-11-10"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/181"
    - "https://github.com/scarecrow173/ruviform/issues/182"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "docs/test-specs/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "docs/plans/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "docs/tasks/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_multiwindow_smoke.rs"
      - type: "file"
        path: "evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json"
      - type: "file"
        path: "evidence/issue-243-multiwindow-verify/environment-blocked-sendinput.md"
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
    - "docs/specs/issue-243-gpui-realwin-windows.md"
    - "docs/specs/issue-313-windows-pointer-smoke.md"
    - "docs/specs/issue-313-windows-realos-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-181-multi-window.md"
    - "docs/specs/issue-182-dock-tree.md"
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/impl/ir/issue-243-windows-multiwindow-smoke.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #243 — Windows 実 OS multiwindow smoke（部分スコープ・後続スライス）

## What Should Be Built

先行スライス（`docs/specs/issue-243-gpui-realwin-windows.md`、
`window_lifecycle_probe`）が driver API・`PostMessageW` 経路で
記録した Window/View lifecycle を、**実 OS 入力経路**
（Issue #313 の `os_input` SendInput injection + example-local
修飾キー chord）で駆動する multiwindow smoke
（`real_os_multiwindow_smoke`）と、その証拠
（`evidence/issue-243-multiwindow-verify/` の verdict JSON・
capability matrix 追記）を追加する。

具体的には、GPUI 標準 Driver で実 Window を 3 つ
（alpha・bravo・charlie — charlie は alpha 破棄後に再生成）
作成し、window 別 focus・OS close（Alt+F4: Veto/Defer/
Approve/無応答既定）・Dock 2-phase move/abort/detach/
reattach・Win+Arrow 実移動・minimize/restore・再生成時の
window id 非再利用と view state 初期化・window 破棄後の view
ownership・Window-owned Job cancel と Application-owned Job
生存を、GPUI semantic oracle と独立 OS oracle
（foreground window title・`WindowFromPoint` hit-test・
cursor read-back）の双方で検証する manual example とする。

## Why Now

Issue #243 の Windows slice は先行 dispatch で driver API 経路の
lifecycle を記録済みだが、「実ユーザー入力で」の検証は
`SendInput` 等の OS 注入経路が #313 で整備されるまで未実施
だった。#313 pointer/keyboard smoke（PR #541）で注入 API と
計測ジオメトリが揃ったため、本スライスで multiwindow/Dock
領域の実入力検証を補完する。あわせて Window-owned Job と
Application-owned Job の close/cancel policy 区別を fixture
レベルで契約化する。

## Users and Value

- Synvelis（複数 Window/RT/Project）・AudioLens（単一 Window
  Job/CLI）の consumer が依存する Window/Dock lifecycle 契約の
  実 OS 成立性を、実入力経路の証拠つきで確認できる。
- 後続の macOS/Linux dispatch・Plugin Dock 受入
  （#388/#389・Synvelis #176）が「Windows で何が実測済みか」を
  正確に参照できる。

## Scope

### In Scope

- `real_os_multiwindow_smoke` example: 3 実 Window の生成・
  focus/close/detach/reattach/move/regenerate を SendInput
  （keyboard/pointer）+ example-local chord（Win+Arrow・
  Alt+F4）で駆動
- 独立 OS oracle: `GetForegroundWindow` title・
  `WindowFromPoint`・`GetCursorPos` read-back と GPUI
  lifecycle/focus semantic oracle の併用
- Window-owned Job cancel vs Application-owned Job 生存の
  区別（fixture/契約検証。consumer 実機は別 Gate）
- move/close/focus 失敗時の stale handle 再利用・二重破棄・
  孤立 view・RT 停止がないことの検証
- verdict JSON + capability matrix（`rf313-os-capability-
  matrix`）への multiwindow 結果追記
- `mise.toml` manual lane（`example-ui-gpui-real-os-
  multiwindow-smoke`）

### Out of Scope

- macOS / Linux の実機検証（NotRun）
- 動的 DPI 変更・モニター間移動・モニター切断（本環境は
  静的 2 モニター・100% DPI のみ実測。変動系は NotRun）
- Native Plugin Dock 受入（#388/#389・Synvelis #176。
  Floating/Generic fallback・Mock/画像を Native Dock PASS に
  しない）
- consumer（AudioLens/Synvelis）実機非回帰 — fixture/契約
  検証のみ、consumer repo は別 Gate
- 真の OS リソース枯渇による window 生成失敗（degenerate
  spec は先行スライスで記録済み）
- 物理手動入力 — `SendInput` は OS automation injection で
  あり物理入力ではない旨を証拠に明記

## Requirements

- 実測値と推測を区別して記録する。実行していない検証を
  Passed と書かない。
- `SendInput`/chord が OS automation injection であり物理
  手動入力でないこと、GPUI simulated input
  （`dispatch_input`/`simulate_*`）とは別レイヤーの証拠で
  あることを verdict/docs 双方に明記する。
- 各 check を Pass/Fail/Unsupported/NotRun に分類し、
  根拠と再現手順を残す。
- 注入が OS に受理されても実カーソルが目標へ到達しない
  環境（例: サードパーティ kernel driver による注入 drop）
  を検出できるよう、cursor read-back を独立 oracle として
  必須化する。
- 検証は `mise.toml` タスクを正本とし、cargo 直接実行は
  補助証拠として明記する。

## Acceptance Criteria

- [x] 実 GPUI Driver で 2 window 以上を生成し、window 別
  focus・close・detach/reattach・移動・再生成時の view
  ownership と状態復元を実 OS 入力で検証した
  （pre-refactor binary で 1 回実測 pass。post-refactor
  再検証は SendInput 環境ブロックで NotRun — 健全 host
  での再実行を follow-up として impl-ir に記録）
- [x] Windows 実測結果を Pass/Fail/Unsupported/NotRun・
  根拠・再現手順つきで capability matrix / verdict JSON に
  記録した（macOS/Linux は NotRun）
- [x] Window-owned Job と Application/Project-owned Job の
  close/cancel policy を区別して記録した
- [x] move/close/focus/IME/NativeSurface attach 失敗で
  stale handle 再利用・二重破棄・孤立 view・RT 停止が
  ないことを Fake semantic と実 OS 独立 oracle で検証した
  （前回実測で green。IME は capability `Unsupported`、
  NativeSurface attach 失敗注入は #388/#389 系へ defer）
- [x] `mise run fmt-check` / `mise run check` /
  `mise run lint-ui-gpui` / `mise run test-ui-gpui` /
  `mise run test-rt` が pass する

## Deferred Design Concerns

- macOS/Linux での同等実測: 本 dispatch の環境外。owner:
  後続の #243 dispatch（OS 別）。
- 動的 DPI・モニター切断・別モニター移動: 静的環境のみ
  実測。owner: #243 後続または #313 capability 拡張。
- Native Plugin Dock: #388/#389・Synvelis #176 が owner。
- consumer 非回帰: audio-lens#166・synvelis#113/#114 が
  owner。
- SendInput がサードパーティ kernel driver（例: Riot
  Vanguard `vgk`）に drop される環境での代替注入経路:
  本スライスでは検出・記録に留め、backend 変更は #313
  owner の判断に委ねる。

## Sources and Relations

- Issue #243 本文（GPUI 移行後の受入判定節が現行正本）
- 先行スライス spec `docs/specs/issue-243-gpui-realwin-
  windows.md`（driver API 経路の lifecycle 証拠）
- Issue #313 `docs/specs/issue-313-windows-realos-smoke.md`
  / `issue-313-windows-pointer-smoke.md`（`os_input`
  SendInput API・計測ジオメトリの正本）
- Issue #181/#182（multi-window・dock-tree の論理契約）
- 共有 lifecycle fixture（`crates/ruviform-ui-test/src/
  lifecycle.rs`）— headless 契約の正本
