---
id: "TSPEC-UR22WOFZb80aKDFszwBay2"
type: "test-spec"
status: "approved"
title: "Issue #631 real-window-close/dpi catalog flip のテスト仕様"
created: "2026-11-17"
updated: "2026-11-17"
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
    - "docs/designs/issue-631-real-window-catalog-flip.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-631-real-window-catalog-flip.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-window-close/dpi catalog flip のテスト仕様

## Purpose

`real-window-close`/`real-window-dpi` の flip が「実 OS window の
実挙動」を正直に判定することを保証する test spec。close lifecycle
を合成しない・sentinel window を誤って破壊しない・DPI 観測を
変更経路の証拠と偽らない、という honesty 契約が将来の変更で
崩れないようにする。IME/pixel/verb/consumer 投影の判定には
踏み込まない — 本 spec が守るのは flip した 2 row の判定正しさ。

## Feature

`rf631.plugin-dock-real` catalog の `real-window-close`/
`real-window-dpi` case（`crates/ruviform-plugin-dock-test`、
`real-dock` feature、interactive Windows desktop 前提）+ AC4
evidence field（`jsonl.rs` の `receipts`/`case_seed`・
`environment.scale_factor` 実測値）。

## Rules

- Rule: close 判定は `CloseRequested` の同期 flush 内にのみ
  `Veto`/`Defer` を適用できる — `arm_close_response` の armed
  queue が唯一のタイムリー経路であり、caller 側の後追い
  `respond_close` は既定 `Approve` に負ける。
- Rule: window A は sentinel — close case は A を破壊せず、
  `windows()==[A]` が case 終了時の証明。GPUI loop は最終 window
  close で終了するため、A の生存が後続 9 case の前提。
- Rule: `Defer` の close-pending は外部から直接読めない —
  「`Destroyed` が出ない」+「後続 `respond_close(Approve)` が
  `Ok`（pending 検査を通過）」の 2 点で実証する。pending が無い
  window への `respond_close` は honest error になる契約を
  probe step として記録する。
- Rule: `window_events` は app thread handler が記録した
  `WindowLifecycle` 遷移のみを返す — `windows()` 等の派生状態
  から推測した値を含まない。window 破壊後も append-only で
  terminal `Destroyed` を保持する。
- Rule: `window_geometry` は live OS handle + scale factor の
  実測値のみを返し、unknown/dead/handle 無しは honest error。
  値の補完・既定値での誤魔化しをしない。
- Rule: DPI case の Pass は「live geometry が lifecycle
  `Resized`/`ScaleFactorChanged` の physical-pixel 記録と整合」
  のみを意味する — DPI 変更経路（mixed-monitor move 等）は
  未駆動であり `detail`/`evidence` に正直に記録する。
- Rule: `REAL_CATALOG` と `real_entries` の `case`/`scenario`/
  `required` は協調して更新する — 片方だけの変更は
  `real_catalog_matches_entries` で検出される。
- Rule: headless/blocked では全行 `NotRun`+
  `observed.environment_blocked`・`environment_kind` が
  `real-window` を名乗らない fail-closed 形状を維持する。
- Rule: evidence 拡充は additive — `receipts`/`case_seed`/
  `scale_factor` の追加が既存 field の意味を変えない。
  `case_seed` は deterministic な case identity であり乱数
  seed を装わない。

## Examples

- Example: veto close — Given armed `Veto` + `request_close(B)`、
  When `CloseRequested` が flush される、Then window B は開いた
  まま `Destroyed` を emit せず、A/C の event log に close
  lifecycle が現れない。
- Example: defer → approve — Given armed `Defer` +
  `request_close(B)`、When close が pending のまま残る、Then
  `respond_close(B, Approve)` が `Ok` を返し `Destroyed` が
  観測され B の id が `windows()` から退場する。
- Example: default approve — Given armed decision なしの
  `request_close(C)`、When `CloseRequested` が flush される、
  Then driver 既定で `Approve` → `Destroyed`、最終的に
  `windows()==[A]`。
- Example: DPI geometry — Given 実 window A が live、When
  `window_geometry(A)` を読み `window_events(A)` の最新
  `Resized`/`ScaleFactorChanged` と照合する、Then 物理 px・
  scale factor が一致すれば `Pass`、不整合・欠落は `Fail`。
- Example: evidence 拡充 — Given 任意の case report、When
  `render_jsonl` が case line を出力する、Then `receipts`（配列）
  と `case_seed`（deterministic hex）が含まれ、real lane の
  `environment.scale_factor` が session 実測値を持つ
  （headless は `null`）。

## Guarantees

- close lifecycle の全判定経路（armed Veto・armed Defer→
  respond Approve・未回答 default Approve）が実 window の
  観測値で判定され、sibling/sentinel 分離と terminal 退役が
  assert される。
- `real-window-dpi` が live `OsWindowGeometry`（origin・物理
  px・scale factor）と lifecycle 記録の整合を判定し、
  `window_geometry_sha256` digest を observed に記録する。
- `REAL_CATALOG`/`real_entries` の required capabilities が
  flip 後の実態（close=`MultiWindow`、dpi=`SemanticAction`）と
  一致する。
- `plugin-dock-real-headless.jsonl` が全行 `NotRun`+
  `environment_blocked` を維持し、flip が headless 形状を
  変えない。
- 実 desktop 不在時は `environment_blocked`/`NotRun` が記録
  され、偽 Pass にならない。

## Non-goals

- `real-window-ime` の判定化（`Ime` capability 未 claim・#389）。
- DPI 変更（mixed-monitor move）・window move/resize verb・
  OS 起点 close の scenario 駆動。
- pixel Pass（#532 gate）・resize/redock/input/focus/IME verb
  （#389 領域）・consumer 投影（synvelis#114/#176・
  audio-lens#127）。
- GPU 情報・完全な semantic tree・HWND を含む AC4 残 field
  （API/依存制約 — PR 本文に残件として明記）。

## Risk

- sentinel window A の誤破壊は後続 9 case を全て道連れにする —
  close sequence が A に一切触れないこと・終了時
  `windows()==[A]` assert・A の event log に close lifecycle
  が無いことの 3 点で検出する。
- `Defer` の pending を直接観測できない — 「生存 + 後続
  `respond_close(Approve)` が `Ok`」の組合せで誤判定を防ぐ。
  pending 無しの `respond_close` が `Err` になる契約を probe
  step で固定する。
- geometry/lifecycle の timing ずれで偽の不整合 Fail — case
  冒頭の `gui.settle()` で pending drain を flush してから
  観測する。
- evidence field の additive 変更が既存 consumer を壊す —
  `receipts`/`case_seed` は新規 key の追加のみで既存 key の
  型・意味を変えない。
- `case_seed` を乱数 seed と誤読される — comment/doc で
  deterministic identity であることを明記する。
