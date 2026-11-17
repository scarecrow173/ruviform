---
id: "DESIGN-C5zglV9oBGR7aXBneJDeUT"
type: "design"
status: "approved"
title: "Issue #631 real-window-close/dpi catalog flip 設計"
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
    - "docs/specs/issue-631-real-window-catalog-flip.md"
    - "docs/discovery/issue-631-real-window-catalog-flip.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-window-close/dpi catalog flip 設計

## Context

`rf631.plugin-dock-real` lane は `RealOsGuiTest::run` の 1 session 内で
21 case を逐次実行する。session は現行 2 window（A=`ctx.window`、
B=`ctx.window_b`）を mount し、window A は後続の editor/session case
（case 13-21）の土台であり sentinel として生存必須 — GPUI platform
loop は最後の window close で終了し `UiJob` pump が死ぬ
（window-surface slice で実走確定）。

close の driver 契約（`lib.rs` `Op::RequestClose` drain・
`respond_close` 実装で確認済み）:

- `request_close` → `CloseRequested` emit → 同期 flush で armed
  queue を消費 → 未回答なら既定 `Approve` → `apply_destroy`。
- armed `Veto` → `pending=false`（window 生存）。armed `Defer` →
  `pending=true`/`awaiting=false`（close-pending で生存）、後続の
  `respond_close(Approve)` が `Op::Destroy` を queue。
- `respond_close` は `close.pending` が無い window に honest error
  を返す — pending 状態の実在証明に使える。

## Scope

### In Scope

- `real_dock.rs`: 2 case の実判定化 + window C mount + context
  field + finalize の scale_factor 配線 + module doc 表更新。
- `real_catalog.rs` + `real_entries`: close の required を
  `MultiWindow` へ協調更新。
- `jsonl.rs`: `receipts`/`case_seed` field 追加（AC4 stretch）。
- evidence JSONL 2 本の再生成 + `verification.md` 追記。

### Out of Scope

- `ruviform-ui`/`ruviform-ui-gpui` の変更、IME/pixel/verb/consumer
  投影、`Cargo.lock`/manifest 変更を要する field。

## Components and Boundaries

### Session 構成（`run_all_with_pixel_artifacts`）

- window A（`WindowSpec::default()`、view 631）— sentinel。close
  case を含む全 case の土台として最後まで生存。
- window B（title `ruviform plugin-dock B`、logical (1450,80)、
  view 632）— close target 1: armed `Veto` → armed `Defer` →
  `respond_close(Approve)` で `Destroyed`。
- window C（title `ruviform plugin-dock C`、logical (2900,80)、
  view 633・新規 `ViewInstanceId`/`ViewHost`）— close target 2:
  未回答 `request_close` で既定 `Approve` → `Destroyed`。
- `RealDockContext` に `window_c: Option<WindowId>` と
  `session_scale_factor: Option<f64>` を追加。scale factor は
  session 開始時に `window_geometry(window A)` を 1 回 query した
  実測値（失敗時 `None` → environment は `null` のまま正直）。

### `real-window-close`（`close_body`）

`multiwindow_focus_body` と同じ steps+attempt 構造:

1. precondition step: `windows()` が A/B/C を含む（3 mount 契約）。
2. B honest-error probe: `respond_close(B, Approve)` が pending
   無しで `Err`（定義済み拒否の観測）。
3. B veto: `arm_close_response(B, Veto)` → `request_close(B)` →
   `settle_until(CloseRequested>=1, 40)` → `settle_until(Destroyed, 8)`
   が `Ok(false)`（生存証明）→ `windows()` に B が残る。
4. B defer: `arm_close_response(B, Defer)` → `request_close(B)` →
   `settle_until(CloseRequested>=2, 40)` → 同様の生存証明
   （close-pending — `pending` は外部から直接読めないため
   「`Destroyed` が出ない + 後続 `respond_close(Approve)` が `Ok`」
   の 2 点で実証）。
5. B approve: `respond_close(B, Approve)` → `Ok` →
   `settle_until(Destroyed, 40)` → B が `windows()` から退場、
   A/C に `CloseRequested`/`Destroyed` なし。
6. C default: arm 無し `request_close(C)` →
   `settle_until(Destroyed, 40)` → C 退場。
7. terminal: `windows() == [A]`、A の event log に
   `CloseRequested`/`Destroyed` なし、B/C の event log が
   `Destroyed` 終端で `Destroyed` 後に `CloseRequested` が無い。

観測は `window_events`（append-only・destroyed 後も読める）と
`windows()` のみ — lifecycle を派生状態から推測しない。helper は
`event_count` 相当（`window_events` を predicate で filter）を
`real_dock.rs` に追加する。

### `real-window-dpi`（`dpi_body`）

1. `gui.settle()` で pending drain を flush。
2. `step("window_geometry", gui.window_geometry(A))` — live
   `OsWindowGeometry`（origin/物理 w/h/scale factor）を記録。
3. `window_events(A)` から最新 `Resized{width,height}`・
   `ScaleFactorChanged{factor}`・イベント集計（件数・close 系の
   有無）を抽出。
4. 整合判定: `scale_factor` が有限かつ >0、最新 `Resized` が存在し
   `(width,height)` が geometry と一致、`ScaleFactorChanged` があ
   れば最新 factor が geometry と一致（epsilon 1e-6）。不整合・
   `Resized` 不在は `Fail`（観測 anomaly を正直に記録）。
5. observed extras: window id・geometry 全 field・lifecycle 集計・
   `window_geometry_sha256`（`sha2` で canonical 文字列
   `"{window}|{ox},{oy}|{w}x{h}|{scale}"` を digest — 既存依存、
   lockfile 不変）・`dpi_change_driven: false`（変更経路未駆動の
   正直な記録）。

### AC4 evidence 拡充

- `jsonl.rs` `case_line`: `"receipts": json!(report.receipts)`
  （`CommandReceipt` は `Serialize` — Command+Receipt 相関の
  evidence）と `"case_seed"`（`sha256("{run_id}|{scenario}|{case}")`
  先頭 16 hex — deterministic な case identity seed。scenario は
  deterministic であり RNG seed ではないことを comment に明記）。
- `finalize_case`（新規 wrapper）: `finalize_real` に
  `ctx.session_scale_factor` を `environment.scale_factor` として
  注入してから委譲。全 21 case の `finalize_real(report, ctx.env, …)`
  呼出を `finalize_case(report, ctx, …)` へ機械的に置換
  （`blocked_reports` は session を持たないため `finalize_real`
  のまま — `null` が正直）。
- DPI case の observed に window geometry digest（上記）。

## Data and Control Flow

close 駆動のタイミング（window-surface slice と同一契約）:

1. `arm_close_response(w, d)` — caller thread の local Mutex push、
   UiJob 不要・即時。`request_close` の op が次 drain で適用される
   まで queue に滞留するため、arm→request の順序で必ず間に合う。
2. `request_close(w)` — UiJob → `Op::RequestClose` queue → 次
   drain で `CloseRequested` emit + `flush_events` が handler を
   同期呼出 → armed decision を pop → `drv.respond_close`。
3. flush 復帰後 driver が `awaiting_response` を確認 — 応答済み
   なら既定 Approve しない。`Veto`/`Defer` はここで window を
   生存させる。
4. `respond_close(w, Approve)`（Defer 後）— UiJob → pending 検査
   → `Op::Destroy` queue → 次 drain で `Destroyed`。

## Non-Functional Constraints

- RT 境界: non-RT のみ（既存 lane と同じ）。
- honesty: 観測は `window_events`/`window_geometry`/`windows()` の
  実値のみ。`Unsupported`/`NotRun`/`Fail` を偽装しない。
- driver boundary: 追加は `real-dock` feature 内で、公開面は
  既存の driver-neutral 型のみ（`check-ui-driver-boundary` green
  維持）。
- `#[non_exhaustive]` policy: 新規公開 enum なし（既存型の利用
  のみ）。
- `workspace.lints` authoritative、`clippy -- -D warnings`。

## Risks and Trade-offs

- window C 追加による session コスト増 — window 1 つ分の mount/paint
  のみ。`real-multiwindow-focus` は `window_b`（2 番目）を使い
  `windows().len() >= 2` 判定のため 3 mount でも影響なし
  （case 順で close は focus の後）。
- close case が window B/C を破壊した後、`ctx.window_b`/`window_c`
  は stale id になる — 以降の case は `ctx.window`/`ctx.view` のみ
  使い、`window_b` を参照する case は存在しない（case 順で
  multiwindow-focus が先）。
- `environment.scale_factor` が session 代表値（window A）になる —
  per-window の authoritative 値は DPI case の `observed` が持つ。
  docs/verification で意味を明記。
- `case_seed` は evidence correlation 用の deterministic identity
  であり乱数 seed ではない — comment/doc で誤解を防ぐ。
- 実走で geometry/イベントの到達が想定と違う場合は honest
  `Fail`/`NotRun` を記録し、判定を緩めない。

## Deferred Design Concerns

- DPI 変更・window move/resize verb — `RealOsGuiTest` に変更 API
  なし（#389 領域）。観測のみの判定である旨を evidence に記録済み。
- OS 起点 close（user の window X）— `on_should_close` は同契約を
  通るが外部 WM_CLOSE 注入は対象外。
- AC4 の GPU 情報・完全な semantic tree・HWND — 公開 API/依存の
  制約で本 slice 対象外（PR 本文に残件として明記）。

## Verification Notes

- [ ] `mise run fmt-check` / `check`・`lint`・`test`
  （default/`real-dock`/`lab-gui`）green。
- [ ] `mise run test-ui-plugin-dock-real-headless` — 全行
  `NotRun`+`environment_blocked`、exit 0。
- [ ] `mise run test-ui-plugin-dock-real` — interactive Windows で
  実走、`real-window-close`/`real-window-dpi` の flip 後 verdict を
  `plugin-dock-real-scenario.jsonl` に記録（`real-dcomp-pixel` の
  既知 Fail による exit 1 は設計どおり）。
- [ ] `mise run test-ui-scenario-contract`・`check-ui-driver-boundary`
  green。
- [ ] `evidence/rf631/verification.md` へ slice 記録追記。
