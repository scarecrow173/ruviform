---
id: "DISC-EQytwTTrhPQ8wwzjh4tGK3"
type: "discovery"
status: "draft"
title: "Issue #631 real-window-close/dpi catalog flip"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-window-close/dpi catalog flip

## Summary

Issue #631 AC1 残行 `real-window-close` / `real-window-dpi` の catalog
row を `Unsupported` stub から実判定へ flip する slice の探索記録。
前 slice（`issue-631-window-surface`、PR #709）で `RealOsGuiTest` に
`window_events`/`request_close`/`arm_close_response`/`respond_close`/
`window_geometry` が実装・実走済みのため、本 slice は
`crates/ruviform-plugin-dock-test` 内の case 実装・catalog 宣言・
evidence 再生成に閉じる。結論: close は 3 window 構成（sentinel A +
target B/C）で veto/defer→approve/default-approve の全経路を、DPI は
live `OsWindowGeometry` と lifecycle `Resized`/`ScaleFactorChanged`
記録の整合で判定可能（confirmed — 実装・実走済み）。

## Exploration Goal

`rf631.plugin-dock-real` catalog の 2 row について、観測面 API を使った
正直な verdict 判定を `real_dock.rs` に実装し、`real_catalog.rs` の
required capabilities と整合させ、実走 evidence を再生成する。

- `real-window-close`: `CloseRequested` →（armed `Veto` / armed
  `Defer`→`respond_close(Approve)` / 未回答既定 `Approve`）→
  `Destroyed` の実遷移を対象 window 単位で観測・判定する。
- `real-window-dpi`: live `window_geometry`（原点・物理 px・
  scale factor）を `window_events` の physical-pixel 記録と照合する。
  DPI **変更**の駆動は変更 API 不在のため判定不能 — 観測のみを
  Pass 判定とし、未カバー経路を `detail`/`evidence` に記録する。

## Key Issues and Assumptions

- `Veto`/`Defer` は `CloseRequested` の同期 flush 内でしか間に合わ
  ない — `arm_close_response` の per-window FIFO が唯一のタイムリー
  経路（confirmed: window-surface slice の design・実走で確定済み）。
- GPUI platform loop は最後の window が閉じると終了し `UiJob` pump が
  死ぬ — catalog lane は 21 case が 1 session で逐次走るため、
  window A（後続 editor/session case の土台）は sentinel として
  生存させる必要がある（confirmed: window-surface slice の実走で
  timeout を再現・修正済み）。
- close の 3 経路のうち `Defer`→`Approve` と既定 `Approve` はどちらも
  対象 window を破壊する — 1 window では両立しないため、破壊可能な
  window が 2 つ必要（confirmed: driver の `Op::RequestClose`/
  `respond_close` 実装を精読して確認）。
- 現行 session は window A/B の 2 mount のみ — 3 経路を全て駆動する
  には window C の mount 追加が必要（confirmed: `real_entries` の
  case 順で `real-multiwindow-focus` は close より先に走り、
  window_b は case 12 以降未使用）。
- `window_geometry`/`window_events` の失敗は `ScenarioError::Driver`
  経由で `Fail` に写像される（`UiError` → `ScenarioError::Driver`、
  `Unsupported` variant にはならない — confirmed: `scenario.rs` の
  `From<UiError>` 実装）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| window B のみで 3 経路を逐次駆動 | mount 追加なし | Defer→Approve と既定 Approve が共に破壊的で 1 window に両立しない | no |
| close case を catalog 末尾へ移動し window A を破壊 | window 追加なし | catalog 順は `REAL_CATALOG`/blocked probe/外部投影の契約 — 並べ替えは影響範囲が大きい | no |
| window C を第 3 mount として追加（A=sentinel, B/C=target） | 全経路を実観測・window-surface slice と同構成（3 window 中 2 close・1 sentinel） | session 開始時の window/view が 1 つ増える | yes |
| DPI case を `Unsupported` のまま残す | 変更最小 | `window_geometry`/`window_events` が実装済みで観測判定が可能 — flip 対象 | no |
| DPI 変更（mixed-monitor move）まで実装 | AC1 の完全充足 | 変更 API が `RealOsGuiTest` に存在しない — scope 外（#389 領域） | no |

## Tentative Conclusions and Hypotheses

- confirmed: close case は window B に armed `Veto` → armed `Defer`
  → `respond_close(Approve)`（`Destroyed` まで）、window C に未回答
  `request_close`（既定 `Approve` → `Destroyed`）を駆動し、window A
  を sentinel として全 case 後も生存させる設計で全経路を判定できる。
- confirmed: DPI case は `window_geometry(window A)` の live 値と
  `window_events` の最新 `Resized`（物理 px）・`ScaleFactorChanged`
  を照合し、整合すれば `Pass`、不整合・取得失敗は `Fail` とする
  観測判定で実装できる。
- confirmed: `required` capabilities は close が `MultiWindow`（複数
  window の同時生存が前提）、DPI が `SemanticAction`（単一 window
  の観測 session）のままが正直な宣言。
- confirmed: AC4 stretch として `jsonl.rs` の `case_line` に
  `receipts`（`CommandReceipt` は `Serialize`）と deterministic な
  `case_seed`（sha2 — 既存依存・lockfile 不変）を追加でき、
  `finalize_real` 経由で session 実測 `scale_factor` を
  `environment` へ配線できる。

## Open Questions

- なし — 全経路の駆動方法・window 構成・capability 宣言・evidence
  拡充の方向は実装確認済み。残る不確実性は実走時の OS 挙動
  （geometry 値・イベント到達タイミング）のみで、honest
  Fail/NotRun で記録する設計。

## Promotion Candidates

- [x] spec-doc needed: `docs/specs/issue-631-real-window-catalog-flip.md`
  — flip の verdict contract・window 構成・evidence 要件を確定する。
- [ ] adr-doc needed: なし — 既存 `WindowDriver`/`WindowLifecycle`/
  `RealOsGuiTest` 契約の範囲内で、新規の architecture decision は
  発生しない。

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — auto-close しない）。
- `docs/discovery/issue-631-window-surface.md` — 観測面の探索記録・
  sentinel window 要件・armed queue 契約。
- `docs/specs/issue-631-window-surface.md` 他 `issue-631-*` 系列 —
  前 slice の spec/design/plan/task/test-spec/ir/exp。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs` — 現行
  `Unsupported` stub（`real_window_dpi_case`/`real_window_close_case`）
  と session 構成（2 mount）。
- `crates/ruviform-plugin-dock-test/src/real_catalog.rs` —
  `REAL_CATALOG` 21 行の required capabilities。
- `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs` —
  観測 API 5 メソッドと `WindowLifecycleEvent`。
- `crates/ruviform-ui-gpui/src/lib.rs` — `Op::RequestClose` drain・
  `respond_close` の pending 検査・既定 Approve の同期 flush。
- `crates/ruviform-ui-gpui/tests/real_window_lifecycle.rs` —
  veto/defer/default-approve/sentinel の実走済み先例。
- `POLICIES.md` — GUI-L2・Pass/Fail/NotRun/Unsupported・fail-closed
  規範。

## Evidence

### 2026-11-17 — catalog flip の実装と実走

- `real_window_close_case`/`real_window_dpi_case` を実判定化し、
  session に window C（`ViewInstanceId` 633・title
  `ruviform plugin-dock C`）を第 3 mount として追加。
- close case: window B に armed `Veto`→`Defer`→`respond_close
  (Approve)`、window C に未回答 `request_close`（既定 Approve）を
  駆動し、window A sentinel の非干渉を assert。
- dpi case: `window_geometry` live 値と lifecycle `Resized`/
  `ScaleFactorChanged` の整合を判定（観測のみ・変更経路は未駆動と
  正直に記録）。
- `real-window-close` の required を `MultiWindow` へ、
  `real-window-dpi` は `SemanticAction` 維持（`REAL_CATALOG` と
  `real_entries` を協調更新 — feature-gated 整合 test が検査）。
- AC4 stretch: `case_line` に `receipts`/`case_seed` を追加し、
  session 実測 `scale_factor` を `environment` へ配線
  （`Cargo.lock` 不変）。
- 実走結果は `evidence/rf631/plugin-dock-real-scenario.jsonl`・
  `plugin-dock-real-headless.jsonl`・`verification.md` を参照。
