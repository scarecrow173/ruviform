---
id: "SPEC-f5oEE57iTJs4gZ26iTRyvD"
type: "spec"
status: "approved"
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
  derives-from:
    - "docs/discovery/issue-631-real-window-catalog-flip.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-window-close/dpi catalog flip

## What Should Be Built

`rf631.plugin-dock-real` catalog の `real-window-close` /
`real-window-dpi` 2 row を、`RealOsGuiTest` の観測 API
（`window_events`/`request_close`/`arm_close_response`/
`respond_close`/`window_geometry` — window-surface slice で実装済み）
を使った実判定へ flip する。`crates/ruviform-plugin-dock-test` 内に
閉じ、session は window A（sentinel）+ window B/C（close target）の
3 mount 構成へ拡張する。あわせて AC4 stretch として JSONL evidence
field（`receipts`/`case_seed`/`environment.scale_factor` 実測値）を
lockfile 変更なしの範囲で拡充し、実走 evidence を再生成する。

## Why Now

Issue #631 AC1 の残行は観測面の欠如が理由で `Unsupported` だったが、
window-surface slice（PR #709）で観測 API が実装・実走済みとなり、
catalog flip の前提が揃った。本 slice はその接続作業 — 観測面は
既にあるため、判定ロジック・catalog 宣言・evidence の整合だけを
行う。

## Users and Value

- GUI Test Driver / Scenario Runner の consumer — close lifecycle
  （veto/defer/approve 全経路）と DPI/geometry 観測を case 単位で
  Pass/Fail/NotRun/Unsupported 判定できる。
- Issue #631 の受入判定者 — AC1 の window close/DPI 採用能力が
  実観測 evidence 付きで判定される。
- 後続 slice（IME・resize/redock verb・pixel Pass・consumer 投影）
  の実装者 — 本 slice の verdict contract と evidence 形式を土台に
  できる。

## Scope

### In Scope

- `real_dock.rs`: `real_window_close_case`/`real_window_dpi_case` の
  実判定化、window C の第 3 mount 追加、`RealDockContext` への
  `window_c`/`session_scale_factor` 追加、モジュール doc の
  honest-outcome 表更新。
- `real_catalog.rs` + `real_dock::real_entries`: `real-window-close`
  の required を `MultiWindow` へ（`real-window-dpi` は
  `SemanticAction` 維持）、doc コメント整合。
- `jsonl.rs`（AC4 stretch）: `case_line` に `receipts` と
  deterministic `case_seed` を追加。
- `real_dock.rs`（AC4 stretch）: session 実測 `scale_factor` を
  `environment` へ配線、DPI case の observed に window geometry
  digest を記録。
- `evidence/rf631/plugin-dock-real-scenario.jsonl` /
  `plugin-dock-real-headless.jsonl` の再生成と
  `evidence/rf631/verification.md` への slice 記録追記。
- doc chain（discovery/spec/design/plan/task/test-spec/impl）。

### Out of Scope

- `real-window-ime` の判定化（`Ime` capability 未 claim・#389 領域 —
  `Unsupported` のまま）。
- DPI **変更**の駆動（mixed-monitor move 等 — `RealOsGuiTest` に
  変更 API がなく #389 領域）。本 slice は観測のみを判定する。
- pixel Pass（#532 gate — `real-dcomp-pixel` verdict=fail を正直に
  維持）。
- `real-editor-resize`/`real-editor-redock` の実 op 化、
  `real-session-crash`/`real-session-hang` の out-of-process 化
  （#389 領域・boundary row）。
- OS 起点 close（user が window の X を押す経路）の scenario 駆動。
- consumer 投影（Synvelis/AudioLens 実 GUI — synvelis#114/#176・
  audio-lens#127 gate）。
- `crates/ruviform-ui`/`ruviform-ui-gpui` の変更（観測面は実装済み）。
- 新規依存追加・`Cargo.lock`/manifest 変更を要する AC4 field
  （GPU 情報・完全な semantic tree・window HWND）。

## Requirements

- close case は window A を破壊しない — A は sentinel として全 21
  case の session を生かし続ける（GPUI loop は最終 window close で
  終了する）。
- close case は armed `Veto`・armed `Defer`→`respond_close
  (Approve)`・未回答既定 `Approve` の 3 経路全てを駆動し、
  `CloseRequested`/`Destroyed` を `window_events` で観測する。
  対象 window の lifecycle を sibling/sentinel と分離して assert
  し、terminal id 退役（`Destroyed` 以降 `CloseRequested` なし・
  `windows()` から退場）を確認する。
- DPI case は live `window_geometry`（原点・物理 px・scale
  factor）を観測し、`window_events` の最新 `Resized`（物理 px）・
  `ScaleFactorChanged` と整合するかを判定する。DPI 変更経路が
  未駆動である旨を `detail`/`evidence` に正直に記録する。
- `Pass`/`Fail`/`NotRun`/`Unsupported` は実 session の実挙動のみで
  決まる — headless/blocked では `environment_blocked`/`NotRun`
  の fail-closed 形状を維持し、Required case を偽 Pass にしない。
  `EnvironmentKind::RealWindow` は実走 session 内の report のみが
  名乗る。
- `REAL_CATALOG` と `real_entries` の `case`/`scenario`/`required`
  は協調して更新する（feature-gated 整合 test が検査）。
- evidence 拡充は `Cargo.lock` 変更を伴わない — `CommandReceipt`
  の `Serialize`・既存 `sha2` 依存・`extra` 経由の environment
  merge のみを使う。GPUI 型を evidence contract へ漏らさない。
- 観測値の合成・補完をしない — `window_events`/`window_geometry`
  の失敗は honest error → `Fail`、未観測は `null`/未記録のまま。

## Acceptance Criteria

- [ ] `real-window-close` が armed `Veto`/`Defer`→`Approve`/既定
  `Approve` の全経路で `CloseRequested`→`Destroyed` を実観測し、
  sentinel window A の非干渉と terminal 退役を assert した
  `Pass`/`Fail` verdict を返す（実 Windows desktop で実走）。
- [ ] `real-window-dpi` が live `OsWindowGeometry` と lifecycle
  `Resized`/`ScaleFactorChanged` の整合を判定し、origin・物理
  px・scale factor を `observed` に記録する `Pass`/`Fail` verdict
  を返す（実 Windows desktop で実走）。
- [ ] `REAL_CATALOG`/`real_entries` の required capabilities が
  flip 後の実態と一致し、`real_catalog_matches_entries`・
  `blocked_reports_fail_closed` が green。
- [ ] `test-ui-plugin-dock-real-headless`（`--blocked-probe`）が
  全行 `NotRun`+`environment_blocked` を exit 0 で維持し、
  `plugin-dock-real-headless.jsonl` が再生成される。
- [ ] AC4 stretch: `case_line` が `receipts`/`case_seed` を出力し、
  real lane の `environment.scale_factor` が session 実測値を持つ
  （headless lane は `null` のまま）。
- [ ] 非回帰 lane（`fmt-check`/`check`/`lint`/`test`
  （default+`real-dock`+`lab-gui` features）/
  `test-ui-plugin-dock-console`/`test-ui-scenario-contract`/
  `check-ui-driver-boundary`）が green。
- [ ] `evidence/rf631/verification.md` に本 slice の検証記録が
  追記され、flip 前の `unsupported` 記載との関係が明記される。

## Deferred Design Concerns

- DPI 変更（mixed-monitor move）・window move/resize verb — verb
  不在（#389 領域）| #389 verb 拡張後 | DPI 変更経路が未検証のまま。
- `real-window-ime`・resize/redock/input/focus verb — #389 領域 |
  #389 session lifecycle 着地時 | 該当 row が `Unsupported` のまま。
- pixel Pass — #532 gate | DComp occlusion 解消時 | `real-dcomp-pixel`
  verdict=fail を正直に保持。
- consumer 投影 — synvelis#114/#176・audio-lens#127 gate | 各 repo
  の受入時 | driver-neutral 投影が未検証のまま。
- AC4 の残 field（GPU 情報・完全な semantic tree・window HWND
  digest）— API/依存の制約 | `RealOsGuiTest` 公開面拡張 or 依存
  追加の承認時 | evidence の網羅性が部分的なまま。

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — auto-close しない）。
- `docs/discovery/issue-631-real-window-catalog-flip.md` — 探索記録。
- `docs/specs/issue-631-window-surface.md` — 観測面の spec（本 slice
  の前提）。
- `docs/impl/ir/issue-631-window-surface.md` — 観測面の実装記録・
  sentinel window 要件。
- `POLICIES.md`（検証レベル・Pass/Fail/NotRun/Unsupported 規範・
  fail-closed）。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`・
  `real_catalog.rs`・`jsonl.rs` — 変更対象。
- `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs`・
  `lib.rs` — 利用する観測 API（変更なし）。
- `evidence/rf631/real-window-surface.jsonl` — close/DPI 観測の
  実走済み先例 evidence。
