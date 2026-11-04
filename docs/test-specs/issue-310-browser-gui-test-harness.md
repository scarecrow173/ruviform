---
id: "TSPEC-034Rp62M5XwHUCClhVcEPk"
type: "test-spec"
status: "approved"
title: "Issue #310 Browser GUI Test Harness 検証仕様"
created: "2026-11-03"
updated: "2026-11-03"
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
  derives-from: []
  derived-by: []
  verifies:
    - "docs/designs/issue-310-browser-gui-test-harness.md"
    - "docs/specs/issue-310-browser-gui-test-harness.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #310 Browser GUI Test Harness 検証仕様

## Purpose

Issue #310 の受入（Issue 本文正本）を実ブラウザ spec に落とした検証仕様。Harness API が廃止・仕様変更されたとき、ここが「何を守るべきか」の正本になる。RF-GT-04/05 で上位 lane が確立されても browser lane の意味的検証は残る。

## Feature

実ブラウザ DOM 上の framework-neutral UI 検証 — Showcase 行列・DAW-shaped・analysis-shaped fixture を Playwright（chromium 既定）で操作し、Gesture 終端・Receipt・Theme・Viz・a11y・lifecycle を意味的に検査する。

## Rules

- Rule: Gesture は `begin`→`preview{0..N}`→終端（commit/cancel）で、終端は同一 gestureId に exactly one。sequence は単調。
- Rule: Command は user intent 1 回につき重複なく gateway へ届き、Receipt は `pending`→`accepted|applied|rejected|unsupported` を一度だけ通る。Receipt 未到達で truth を誤確定しない。
- Rule: typed DOM property・CustomEvent（bubbles+composed）は element 公開契約どおり機能し、controlled `value` write は intent を再発火しない。
- Rule: Theme 変更・remount/reconnect（StrictMode 相当含む）は Source/Project 真値を改変せず、subscription/query の重複・資源漏れを生じない。
- Rule: Canvas 描画は空白でなく、NaN/欠測は 0 補間せず明示 gap。stale は status+overlay で示され、resize/DPR/viewport は bounded re-query を発行する。
- Rule: `rvf-backend` capability と draw stats は記録され、WebGL2 不可環境は「不可」を証拠化する（失敗としない）。
- Rule: Ruviform package に React/Solid/Vue/Svelte・Playwright 等の必須依存がない（browser-test の dev+peer を除く）。

## Examples

- fader 上で pointer down→drag→up: `rvf-gesture` が begin, preview…, commit の順で 1 組だけ記録され、`__rvf.drain("apply")` 後に element `pending` が解消・`value` が commit 値・revision +1。
- knob 上で wheel →`clock.runFor(200)` → adjust+commit が 1 組。drag 中 Escape → cancel が exactly one。
- commit を drain せず置く → `pending` 属性のままで `applied` 扱いにならない。drain("reject") → `invalid` + 前値 revert。
- Showcase 全列で `rvf-theme.theme` を別 ThemeInput に差替え → computed token が変化し gateway 真値は不変。
- `unmount()`→再 mount → `subscribeCalls` は target ごと 1 回で増えず、再 drag でも commands が 2 重発行されない。
- waveform に revision push → `status` part が "stale" → `queries()` に新 query。`setSize(幅変更)` → 新 buckets の query。
- デモ波形 ch0 gap（0.45–0.55）で lane 中央 fill ratio が近傍より有意に低い。
- `mountDawFixture` で fader drag + transport click + remount、`mountAnalysisFixture` で `QueryBinding` resolve/stale/cancel — どちらも同じ `openHarness`/`control`/`receipts`/`events` API。

## Guarantees

- 全待機は状態到達 poll（`expect.poll`/Node evaluate loop）で固定 sleep を使わない。
- Screenshot は失敗時の補助証拠のみ。意味的 oracle は DOM 属性・event log・gateway 記録・pixel probe。
- browser lane の PASS/FAIL/NOT-RUN は `evidence.json` に環境・revision・seed とともに記録され、OS/GPU/製品 E2E と別集計。
- `ui/browser-test` 以外の Ruviform 配布面に `@playwright/test` が混入しない。

## Non-goals

- OS WebView 実操作（#313）・Native GPU（#314）・製品真正シナリオ/Golden（Consumer Owner）・pixel 厳密比較。

## Risk

- <!-- What is lost or breaks if these guarantees are dropped -->
