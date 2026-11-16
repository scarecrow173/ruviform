---
id: "TSPEC-034YJ3x3sCrtx2chmCg7dk"
type: "test-spec"
status: "approved"
title: "Issue #631 RealOsGuiTest window-close/DPI 観測面のテスト仕様"
created: "2026-11-16"
updated: "2026-11-16"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-631-window-surface.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-631-window-surface.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 RealOsGuiTest window-close/DPI 観測面のテスト仕様

## Purpose

`RealOsGuiTest` の window-close/DPI 観測面が「実 OS window の実挙動」を
正直に記録・検証することを保証する test spec。観測値を合成しない・
未回答 close の既定 `Approve` を覆さない・destroyed window を
fabricate しない、という honesty 契約が将来の変更で崩れないように
する。catalog row flip・他 slice の判定には踏み込まない — 本 spec が
守るのは観測面そのものの正しさ。

## Feature

`RealOsGuiTest` window lifecycle / geometry observation on real OS
windows（`tests/real_window_lifecycle.rs`、`real-scenario` feature、
interactive desktop 前提）。

## Rules

- Rule: close 判定は `CloseRequested` の同期 flush 内にのみ
  `Veto`/`Defer` を適用できる — caller 側の後追い `respond_close`
  は既定 `Approve` に必ず負ける。
- Rule: `window_events` は app thread handler が記録した
  `WindowLifecycle` 遷移のみを返し、`windows()` 等の派生状態から
  推測した値を含まない。`Action` intent は含まない。
- Rule: `window_events` の log は append-only で、window 破棄後も
  terminal `Destroyed` を保持する。window id は退役後に再利用
  されない。
- Rule: `window_geometry` は live OS handle + scale factor の実測値
  のみを返し、unknown/not-open/handle 無し/backend 無しは honest
  error。値の補完・既定値での誤魔化しをしない。
- Rule: `Resized`/`Moved`/`ScaleFactorChanged` の geometry 値は
  physical pixel — logical 値や scale 未適用値を記録しない。
- Rule: 最終 window の破棄後は GPUI platform loop が終了し UiJob
  pump が死ぬ — 最終破棄を観測する session は sentinel window を
  残す。
- Rule: 実デスクトップ不在・session 開始失敗は
  `environment_blocked`/`NotRun` を記録し、fail や pass に
  見せかけない。

## Examples

- Example: veto close — Given armed `Veto` + `request_close(A)`、
  When `CloseRequested` が flush される、Then window A は開いたまま
  `Destroyed` を emit せず、sibling/sentinel は無関係のまま。
- Example: defer → approve — Given armed `Defer` + `request_close(A)`、
  When close が pending のまま残る、Then `respond_close(A, Approve)`
  で `Destroyed` が観測され id が terminal 退役する。
- Example: unanswered default — Given armed decision なしの
  `request_close(B)`、When `CloseRequested` が flush される、Then
  driver 既定で `Approve` → `Destroyed`。
- Example: DPI geometry — Given 実 window が Ready、When
  `window_geometry` を 2 回読む、Then 原点・物理サイズ・scale
  factor が安定して一致し、lifecycle `Resized` 記録と物理 pixel で
  一致する。破棄後の `window_geometry` は error。

## Guarantees

- close lifecycle の全判定経路（veto/defer/deferred-approve/
  default-approve）が実 window の観測値で検証される。
- terminal id 退役・sibling/sentinel 分離・dead-window honest
  error が assert される。
- `evidence/rf631/real-window-surface.jsonl` に verdict=pass の
  run/case 行（実測値・repo SHA・OS 入り）が記録される。
- 実 desktop 不在時は `environment_blocked` が記録され、偽 Pass
  にならない。

## Non-goals

- `real-window-close`/`real-window-dpi` の catalog row flip・
  plugin-dock evidence 再生成（後続 slice）。
- pixel capture・DPI 変更（mixed-monitor move）・window
  move/resize verb・IME・OS 起点 close（user の window X）の
  scenario 駆動。
- consumer（Synvelis/AudioLens）への投影。

## Risk

- sentinel window を外すと最終破棄後の観測が pump 死亡で timeout
  し、test が環境依存 flaky になる（実走で再現済み）。
- armed queue の消費タイミングを誤ると `Veto`/`Defer` が既定
  `Approve` に負けて window が誤って破棄される — test が全経路を
  直接 assert することで検出する。
- 観測値の合成・補完が混入すると evidence が偽の Pass を主張する
  — 観測源が app thread handler / `UiJob` に限定されていることを
  design review で確認済み。
