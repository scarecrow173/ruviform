# RF389 — Plugin Editor Session hardening slice 証拠レポート

Task: `task_34364d7fdec5` / branch `scarecrow173/rf389-session-harden`
（spec 上の意図名 `feature/389-session-hardening`）。
対象 Issue: <https://github.com/scarecrow173/ruviform/issues/389>。

本 slice は **契約層の hardening のみ** — `ruviform-plugin-host` の
`editor_session` に bounded request deadline・明示 degrade 経路・
GPU failure cause を追加し、PR #501（`evidence/rf388-native-dock-windowing/`）
の DComp 実測制約を capability matrix に正直に記録した。実 plugin・
実 GPU・実 window の受入は本 slice の対象外（後述 NotRun）。

## 変更範囲

| 領域 | 内容 |
| --- | --- |
| `crates/ruviform-plugin-host/src/editor_session.rs` | `SessionClock` port・`MonotonicSessionClock`・`SessionDeadlinePolicy`・`DEFAULT_REQUEST_TIMEOUT`・deadline arm（`next_request`）・`tick`/`request_deadline`/`set_deadline_policy`/`PluginEditorSessions::tick_all`・`OpenOutcome`・`EditorSessionEvent::Degraded`・`open_or_degrade`/`begin_degrade`・`SessionFailCause::GraphicsLost`/`EditorOpError::GraphicsLost`。pending attach marker は `reattach_pending`/`degrade_pending` 2 bool を `PendingAttach` enum に統合（clippy `struct_excessive_bools` 対応・意味論は同一） |
| `crates/ruviform-plugin-host/src/lib.rs` | 新規 public 型の re-export（additive のみ） |
| `crates/ruviform-plugin-host/tests/editor_session.rs` | `FakeClock`/`ModeOps` fixture + SH-01..17 contract test 17 件追加（既存 28 件は非回帰） |
| `crates/ruviform-plugin-host/README.md` | hardening 挙動の記述追加 |
| `docs/` | spec `issue-389-session-hardening`・design（D1–D5 + capability matrix）・plan・task・test-spec（SH-xx）チェーン |

触れていない範囲（確認済み）: `ruviform-ui-gpui`、`ruviform-plugin-core`、
`ruviform-plugin-vst3`/`clap`/`ofx`、`ruviform-poc-native-dock`（check
通過で非回帰を確認）、workspace `Cargo.toml`、`Cargo.lock`（依存追加なし
— `std::time` のみ使用）。

## 実施コマンド / 環境

- OS: Windows 11 x64 / Git Bash
- toolchain: mise 管理 Rust（`mise.toml` pin）
- repo: worktree `rf389-session-harden`、base = `origin/main`（PR #501 merge 済み `00324e10`）+ 本 branch の変更

| 検証 | コマンド | exit | 結果 |
| --- | --- | --- | --- |
| fmt | `mise run fmt-check` | 0 | Pass |
| check | `mise run check`（`cargo check --workspace --all-targets`） | 0 | Pass — `ruviform-poc-native-dock` 含む全 member compile |
| lint | `mise run lint`（clippy `-D warnings`） | 0 | Pass — warning なし |
| test | `mise run test`（`cargo test --workspace`） | 0 | Pass — workspace 全 test suite green（181 test-result セクション、failed 0） |
| 補助証拠（canonical = mise） | `cargo test -p ruviform-plugin-host --test editor_session` | 0 | 45 passed / 0 failed（既存 28 + SH-01..17） |

## contract test 対応（test-spec SH-xx）

| ID | test | 結果 |
| --- | --- | --- |
| SH-01 | `sh01_open_request_times_out_to_failed_timeout` | Pass — `Failed{Timeout}` + 単一 `Failed` event + late completion `StaleRequest` |
| SH-02 | `sh02_tick_before_deadline_is_a_noop` | Pass |
| SH-03 | `sh03_disarmed_policy_leaves_requests_unbounded` | Pass |
| SH-04 | `sh04_close_request_timeout_keeps_adapter_close_path` | Pass — `editor_live` 維持で再 `close` が adapter 経由 `Closed` に収束 |
| SH-05 | `sh05_detach_request_timeout_emits_no_detached` | Pass |
| SH-06 | `sh06_recover_request_timeout_refails_with_timeout` | Pass |
| SH-07 | `sh07_post_detach_reattach_request_times_out` | Pass |
| SH-08 | `sh08_tick_all_times_out_only_expired_sessions` | Pass |
| SH-09 | `sh09_preferred_unsupported_degrades_to_floating_explicitly` | Pass — `OpenOutcome::Degraded` + `Degraded{Floating}` event（`Attached` なし） |
| SH-10 | `sh10_preferred_supported_never_retries` | Pass |
| SH-11 | `sh11_non_unsupported_preferred_error_never_retries` | Pass |
| SH-12 | `sh12_fallback_failure_terminates_failed` | Pass |
| SH-13 | `sh13_equal_modes_degenerate_to_plain_open` | Pass |
| SH-14 | `sh14_async_degrade_lands_degraded_not_recovered` | Pass |
| SH-15 | `sh15_failed_degrade_does_not_poison_later_recover` | Pass |
| SH-16 | `sh16_graphics_lost_maps_to_distinct_fail_cause` | Pass |
| SH-17 | `sh17_notify_terminated_graphics_lost_fires_once` | Pass |
| SH-18 | `session_surface_names_no_gui_toolkit_or_os_window_handle`（既存） | Pass — 新規語彙は banned 語彙に非該当 |

## DComp / pixel compositing の正直な記録

design `docs/designs/issue-389-session-hardening.md` D5 の capability
matrix に記録済み（本 slice で再実測はしていない — PR #501 の実測を
引用）:

- **GPUI/DComp native surface pixel compositing（Windows）= Unsupported
  （実測）**: GPUI 0.2.2 `directx_renderer.rs` の
  `CreateTargetForHwnd(hwnd, topmost=true)` が swapchain を child HWND
  の上に描画し、plugin editor・対照 probe HWND 双方の画素が合成されない。
  HWND 機構（parentage/geometry/clip/hit-test/focus/lifetime/re-dock）は
  Pass だが画素は観測不能。根拠:
  `evidence/rf388-native-dock-windowing/report.md`。
- **native surface pixel compositing 実測 = NotRun**: DComp 制約により
  現行 GPUI 経路では観測不能。修復は上流 GPUI/adapter 設計の別 Issue。

## 未達 / 正直な分類（NotRun / Unsupported）

| 項目 | status | 理由 |
| --- | --- | --- |
| 実 wall-clock での timeout 発火 | NotRun | fake clock のみ — 実時間依存 test は flaky のため contract test は注入 clock で決定的に検証。実 driver の `tick` 周期接続は composition root 側の後続作業 |
| 実 plugin / 実 GPU / 実 window での timeout・degrade・graphics-lost 発生 | NotRun | 本 slice は契約層のみ。実機受入は別 slice |
| Floating window の実 surface adapter | Unsupported | floating window adapter は未実装 — degrade 経路は ops が `Unsupported` を返した場合の契約のみ。L0 `InProcessEditorOps` は依然 non-Docked を `Unsupported` で拒否し fake しない |
| GPUI/DComp pixel compositing 修復 | Unsupported（上流制約） | `ruviform-ui-gpui` 変更は本 slice の編集範囲外。修復は別 Issue |
| L1 (cross-process) editor ops | Unsupported | `L1EditorOps` は全 op defined refusal のまま — L0 退化なし |
| raw `GuiParentToken` の public API 除去 | 対象外（別 slice） | Plugin trait/コマンド面の契約変更を伴う |
| macOS/Linux/CLAP/AU・multi-plugin sweep | NotRun | Issue 残 DoD |

## doc graph 監査メモ

`doc-driven-dev-graph` router（`.agents/skills/doc-driven-dev-graph/scripts/route_graph.js`）
は本 slice 着手時点で `blocked`（`artifact-graph` + `focus-required`）を返した。
原因は RF313/RF243 系の**既存の壊れた doc relation**（本 slice 範囲外・
main 既存）であり、本 slice の新規 doc（spec/design/plan/task/test-spec）
自体の relation は生成スクリプトで正しく張られている。範囲外の修復は
行っていない — 別途報告事項。
