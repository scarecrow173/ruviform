---
id: "TSPEC-034UnAjuxNxMXzDMasVYeO"
type: "test-spec"
status: "approved"
title: "Issue #389 Plugin Editor Session hardening テスト仕様（fake-clock timeout・degrade・GraphicsLost contract）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-session-hardening.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-389-session-hardening.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-389-session-hardening.md"
    - "docs/specs/issue-389-session-hardening.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 Plugin Editor Session hardening テスト仕様

## Purpose

`editor_session` hardening slice（design D1–D4）の契約を、fake clock・
fake ops による**決定的な contract test** で固定する。実機・実
plugin・実 GPU を使わない — ここで保証するのは状態機械と語彙の
契約であり、実 Native Dock / pixel compositing の PASS ではない。
実装が設計から乖離した場合（degrade が `Attached` に化ける、
timeout が terminal event を重複させる、GPU cause が `Internal`
に潰れる）に退行を検出するために存在し、契約語彙が安定した後も
残す。

## Feature

Plugin Editor Session の hardening — bounded request timeout・
明示 degrade・GPU failure cause。

## Rules

- **SH-R1 (deadline arm)**: in-flight request（`Creating`/
  `Detaching`/`Closing`/`Recovering`）が存在する間、session は
  policy clock domain の絶対 deadline を保持する。`next_request`
  を通る全 request（open/detach/close/recover・detach 後の
  re-attach request）に適用される。
- **SH-R2 (timeout terminal)**: `tick` 時点で `clock.now() >=
  deadline` の in-flight request は `EditorOpError::Timeout` と
  同じ終端 — `Failed{Timeout}` state + `Failed{Timeout}` event —
  に収束する。`Creating`/`Recovering`/`Detaching` は
  `complete_attach`/`complete_detach` の Err 経路、`Closing` は
  `mark_failed` 経路（`complete_close` は error を運ばない）。
  いずれも後続 `complete_*` は `StaleRequest` で拒否され、
  terminal event は重複しない。
- **SH-R3 (no premature fire)**: deadline 未到達の `tick` は
  state・event を一切変更しない。policy 未 install
  （`set_deadline_policy(None)`）では `tick` は常に no-op。
- **SH-R4 (degrade exclusivity)**: `open_or_degrade` が fallback
  へ degrade して成功した場合、終端 event は `Degraded{mode}`
  のみ — `Attached`/`Recovered`/`Reattached` を発火しない。
  state は `Attached{fallback}`。
- **SH-R5 (degrade precondition)**: degrade retry は
  `ops.open(preferred)` が `EditorOpError::Unsupported` を返し、
  かつ `fallback != preferred` の場合に限り 1 回だけ行われる。
  他の error（`Plugin`/`HostLost`/`LeaseRejected`/`Timeout` 等）
  では retry しない。fallback の失敗は `Failed{cause}` で終端し
  `Err(Ops(e))` を返す。
- **SH-R6 (async degrade)**: `begin_degrade` は `Failed` から
  `Recovering` に入り degrade を mark し、`complete_attach(Ok)` で
  `Degraded{mode}` を発火する（`Recovered` ではない）。失敗時は
  flag がクリアされ、後続の通常 `recover` は `Recovered` を発火
  する。
- **SH-R7 (GraphicsLost mapping)**: `EditorOpError::GraphicsLost`
  は `SessionFailCause::GraphicsLost` に写像され、
  `notify_terminated(GraphicsLost)` も `Terminated{GraphicsLost}`
  を発火できる。`HostLost`/`Internal` と区別される。

## Examples

SH-xx case は `crates/ruviform-plugin-host/tests/editor_session.rs`
の test 名に対応する。

| ID | Rule | 手順 | 期待 |
| --- | --- | --- | --- |
| SH-01 | SH-R2 | fake clock で `begin_open` → 時刻を timeout 超過へ進め `tick` | state `Failed{Timeout}`、events `[Failed{Timeout}]`、後続 `complete_attach` は `StaleRequest` |
| SH-02 | SH-R3 | `begin_open` → deadline 未満で `tick` → その後 `complete_attach(Ok)` | `tick` は state/event 不変、attach は通常 `Attached` で着地 |
| SH-03 | SH-R2/R3 | `set_deadline_policy(None)` → `begin_open` → 時刻を大きく進め `tick` | state は `Creating` のまま（policy なしは unbounded） |
| SH-04 | SH-R2 | open 済 session で `begin_close` → timeout 超過 → `tick` | `Failed{Timeout}`（`editor_live` 維持）→ 再 `close` で adapter 経由 `Closed` に収束可 |
| SH-05 | SH-R2 | `Attached` で `begin_detach` → timeout 超過 → `tick` | `Failed{Timeout}`、events は `Detached` を含まず `Failed` のみ（detach 未完了） |
| SH-06 | SH-R2 | `Failed` session で `begin_recover` → timeout 超過 → `tick` | 再び `Failed{Timeout}`（cause が Timeout に更新） |
| SH-07 | SH-R2 | detach 完了後の re-attach `Creating` request が timeout | `Failed{Timeout}`（`reattach_pending` もクリア） |
| SH-08 | SH-R1 | `PluginEditorSessions::tick_all` で複数 session を一括 tick | 期限切れ session のみ `Failed{Timeout}`、戻り値は timeout 数 |
| SH-09 | SH-R4/R5 | FakeOps: Docked→`Unsupported`、Floating→Ok で `open_or_degrade(Docked, Floating)` | `Ok(OpenOutcome::Degraded{Floating,..})`、state `Attached{Floating}`、events `[Degraded{Floating}]`（`Attached` なし）、ops.open は 2 回（mode 記録） |
| SH-10 | SH-R4/R5 | FakeOps: Docked→Ok で `open_or_degrade(Docked, Floating)` | `Ok(OpenOutcome::Attached(..))`、events `[Attached{Docked}]`、ops.open は 1 回 |
| SH-11 | SH-R5 | FakeOps: Docked→`Plugin` で `open_or_degrade(Docked, Floating)` | `Err(Ops(Plugin))`、`Failed{PluginFailed}`、ops.open は 1 回（retry なし） |
| SH-12 | SH-R5 | FakeOps: 両 mode→`Unsupported` で `open_or_degrade` | `Err(Ops(Unsupported))`、`Failed{Unsupported}`、ops.open は 2 回 |
| SH-13 | SH-R5 | `open_or_degrade(Docked, Docked)`（fallback==preferred） | retry なし・`OpenOutcome::Attached` または通常失敗、ops.open は 1 回 |
| SH-14 | SH-R6 | `Failed` session → `begin_degrade(Floating)` → `complete_attach(Ok)` | state `Attached{Floating}`、events `[Degraded{Floating}]`（`Recovered` なし） |
| SH-15 | SH-R6 | `begin_degrade` → `complete_attach(Err(Timeout))` → `recover` 成功 | degrade 失敗は `Failed{Timeout}`、後続 recover は `Recovered` を発火（flag 残留なし） |
| SH-16 | SH-R7 | FakeOps open→`Err(GraphicsLost)` で `open` | `Failed{GraphicsLost}`（`HostLost`/`Internal` でない） |
| SH-17 | SH-R7 | `notify_terminated(GraphicsLost)` on live session | `Terminated{GraphicsLost}` が 1 回だけ発火 |
| SH-18 | — | `session_surface_names_no_gui_toolkit_or_os_window_handle`（既存） | 新規語彙追加後も banned 語彙なし（`SessionClock`/`OpenOutcome`/`Degraded`/`GraphicsLost`/`tick` はいずれも非該当） |

## Guarantees

- deadline 超過の request は必ず `Failed{Timeout}` + `Failed` event
  の単一終端に収束し、後続 completion は `StaleRequest` で拒否される。
- degrade 成功は `Degraded{mode}` event と `OpenOutcome::Degraded`
  の両方で通常 attach と区別可能であり、`Attached`/`Recovered` を
  発火しない。
- degrade retry は `Unsupported` + fallback 相異に限定され、他の
  error では ops を再呼出しない。
- `GraphicsLost` は ops error 経路と terminate 経路の両方で
  `HostLost`/`Internal` と区別して観測できる。
- 既存 contract test（state machine・terminal-once・L0/L1）は
  挙動不変で green を維持する（API additive のみ）。

## Non-goals

- 実 wall-clock の経過（fake clock のみ — 実時間依存 test は flaky
  になるため書かない）。
- sync convenience path の preempt（inline ops 呼出は中断不能 —
  doc で明記済み）。
- 実 plugin/GPU/OS での timeout・degrade・graphics-lost 発生
  （`NotRun` — 実機受入は別 slice）。
- pixel compositing・floating window の実 surface（DComp 制約・
  別 Issue）。
- L1 wire での timeout/degrade 通知。

## Risk

- degrade が `Attached` に化ける退行 → Native Dock PASS の水増し。
  SH-09/SH-14 が event 排他を pin する。
- timeout が terminal event を重複/欠落させる退行 → driver の
  復旧判断を誤らせる。SH-01 の `StaleRequest` 確認が pin する。
- `Closing` timeout で `editor_live` を落とし adapter `close` を
  skip する退行 → plugin editor が孤児化。SH-04 が再 close 経路を
  pin する。
- GPU cause が `Internal` に潰れる退行 → failure 分類の喪失。
  SH-16/SH-17 が pin する。
