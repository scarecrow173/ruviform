# rf389 editor-verbs — verification record

Issue `scarecrow173/ruviform#389` の adapter editor verbs slice
（`InProcessEditorOps` の `set_size`/`set_visibility`/`reattach`
defined `Unsupported` refusal を除去し、plugin-core trait →
`HostCommand` → host dispatch → composite → VST3/CLAP adapter の
production chain へ実装。`rf631.plugin-dock-real` lane の
`real-editor-resize`/`real-editor-redock`/`real-editor-visibility`
を refusal/deferred から実実行 + effect oracle 検証へ flip）の
ローカル検証記録。canonical = `mise` lane。

- Date: 2026-11-18
- OS: Windows (x86_64) — interactive desktop session あり（real lane 実走）
- Toolchain: rustc 1.99.1 / cargo 1.99.1（mise 管理）
- Repo SHA: lane JSONL の `environment.repo_sha` は
  `b6d0cc6a2e14f47e5324061d8dcac3d905e4abd5`（branch HEAD、
  本 slice の変更は working tree 未コミット状態で実走）。

## Lane results

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass (exit 0) |
| `cargo test -p ruviform-plugin-host --test editor_session` | pass — 54/54（`l0_set_size_reaches_plugin_through_setguisize`・`l0_set_visibility_cycles_hidden_shown_through_host`・`l0_reattach_redeems_lease_and_reaches_set_gui_parent`・`l0_session_open_close_leaves_dsp_processing_untouched` 等を含む） |
| `cargo test -p ruviform-plugin-test-vst3` | pass — unit 91 + integration 16、fixture の `canResize == kResultTrue`・`checkSizeConstraint`/`onSize` 契約 test を含む |
| `cargo clippy -p ruviform-plugin-{core,host,vst3,clap,test-vst3} --all-targets` | pass — warning 0 |
| `cargo clippy -p ruviform-plugin-dock-test --all-targets --features real-dock` | pass — warning 0 |
| `cargo test -p ruviform-plugin-dock-test` | pass — 4/4 integration + unit 全 green |
| `cargo test -p ruviform-plugin-dock-test --features real-dock` | pass — `real_catalog_matches_entries`・`blocked_reports_fail_closed`・console 14/14 を含む全 green |
| `mise run test-ui-plugin-dock-real-headless` | pass — 22 case row 全て `not_run` + `observed.environment_blocked`、violation 0、exit 0 |
| `mise run test-ui-plugin-dock-real` | exit 1 — `real-dcomp-pixel` の既知 `fail`（GPUI DComp topmost occlusion・Issue #532 決裁待ち・本 slice 対象外）による設計通りの exit code |

## Real lane verdicts（`evidence/rf631/plugin-dock-real-scenario.jsonl`）

| case | verdict | 内容 |
| --- | --- | --- |
| `real-editor-resize` | **pass** | `session.set_size(800×600)` が REAL container lease 経由で accepted。child HWND の `GetWindowRect` が要求サイズと一致（fixture `onSize` が自 window を resize）、session `Attached` 維持、events `[Attached{Docked}, Closed]`、close 収束を実測 |
| `real-editor-redock` | **pass** | `session.reattach` が同一 live lease で accepted。pre-reattach child HWND 全て破棄（`removed()` → `DestroyWindow`）、新 attach generation（`rvf-test-editor-attach-{n}` title 前進）が container 配下に出現、events `[Attached{Docked}, Detached, Reattached{Docked}, Closed]` を実測 |
| `real-editor-visibility` | **pass** | `set_visibility(false)` → `Hidden{Docked}` で pre-hide child HWND 全破棄、`set_visibility(true)` → `Attached{Docked}` で child contract が新 generation で再成立、events `[Attached{Docked}, Hidden, Shown{Docked}, Closed]` を実測 |
| `real-window-ime` / `real-session-crash` / `real-session-hang` | unsupported | 既存の honest boundary row（本 slice 無変更） |
| `real-dcomp-pixel` | fail | 既知・対象外（#532） |

その他の real row は全て `pass`。`real-window-dpi`/`real-window-close`
も `pass`（前 slice の `unsupported` からの改善は本 slice 範囲外の
既存 lane 動作）。

## Evidence

- `evidence/rf631/plugin-dock-real-scenario.jsonl`（canonical、
  本 slice 実走版）
- `evidence/rf631/plugin-dock-real-headless.jsonl`（blocked-probe
  fail-closed proof、本 slice で再実走）
- `evidence/rf389/plugin-dock-real-scenario.jsonl` /
  `plugin-dock-real-headless.jsonl`（本 slice snapshot コピー）
