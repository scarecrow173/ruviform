# rf389 real-session-terminate — verification record

Issue `scarecrow173/ruviform#389` の real-window session lifecycle
L2 case slice（`real-session-terminate` — test_editor attach 中の
Host Destroy → `Terminated{HostLost}` exactly-once → child HWND
teardown → host 存続）のローカル検証記録。canonical = `mise` lane。

- Date: 2026-11-16
- OS: Windows (x86_64) — interactive desktop session あり（real lane 実走）
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1
- Repo SHA: lane JSONL 記録の `repo_sha` =
  `71003d9310905437de08c4ae204f00130aa1da60`（origin/main、
  rf631 real-row merge `078571a0` 統合済み）。commit 前の working
  tree で実走 — コード差分は本 slice の `real_dock.rs` /
  `real_catalog.rs` / `tests/console.rs` / docs のみ。

## Lane results

| Command | Result |
| --- | --- |
| `mise run fmt-check` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test` | pass (exit 0, `-D warnings`) |
| `mise run check ruviform-plugin-dock-test` | pass (exit 0) |
| `mise run test ruviform-plugin-dock-test` | pass (exit 0) |
| `mise run check ruviform-plugin-dock-test --features real-dock` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test --features real-dock` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-plugin-dock-test --features real-dock` | pass (exit 0 — `real_catalog_matches_entries`・`blocked_reports_fail_closed`・console `list_include_real_appends_real_lane_rows` 21 行を含む全 test green) |
| `mise run check ruviform-plugin-dock-test --features lab-gui` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test --features lab-gui` | pass (exit 0) |
| `mise run build ruviform-plugin-test-vst3` | pass (exit 0 — `test_editor`/`test_gain` cdylib 生成) |
| `mise run test ruviform-plugin-host --test editor_session --test editor_session_driver` | pass (exit 0) |
| `mise run test-ui-plugin-dock` | pass (exit 0) |
| `mise run test-ui-plugin-dock-real-headless` | pass (exit 0 — 13 行全て `not_run`+`observed.environment_blocked`、violation 0) |
| `mise run test-ui-plugin-dock-real` | exit 1 — **設計どおりの正直な regression 信号**: 既知 `real-dcomp-pixel` が `fail`（GPUI DComp topmost occlusion、`scarecrow173/ruviform#532` 決裁待ち・本 slice で変更なし）。新規 `real-session-terminate` は `pass` |
| `mise run test-ui-plugin-dock-console` | pass (exit 0) |
| `mise run check-ui-driver-boundary` | pass (exit 0 — dep-graph/signature/allowlist) |
| `mise exec -- prek run --all-files` | pass (exit 0) |

補助証拠（canonical = mise）: `plugin-dock-console --list
--include-real`（`--features real-dock`）を
`plugin-dock-console-list-include-real.jsonl` に保存 — 21 行
（headless 8 + real 13）projection、`real-session-terminate` 含む。

## `test-ui-plugin-dock-real` 実走 verdict（`plugin-dock-real-scenario.jsonl`）

interactive Windows session で全 13 case を実走。`environment_kind`
は全行 `real-window`。

| case | verdict |
| --- | --- |
| `real-window-mount` | pass |
| `real-native-slot` | pass |
| `real-editor-attach` | pass |
| `real-capture` | pass |
| `real-dcomp-pixel` | **fail** — 既知の GPUI DComp topmost occlusion（`EDITOR_COLOR` 緑なし、`best green_fraction=0.000`）。本 slice の対象外・緩和なし |
| `real-multiwindow-focus` | pass |
| `real-os-input` | pass |
| `real-window-dpi` | unsupported |
| `real-window-ime` | unsupported |
| `real-window-close` | unsupported |
| `real-editor-resize` | unsupported（定義済み `EditorOpError::Unsupported` refusal） |
| `real-editor-redock` | unsupported（同上） |
| `real-session-terminate` | **pass** — 本 slice の新規 case |

### `real-session-terminate` の実走 step 列（全て pass、`thread_affinity` は note）

`declare_slot` → `bind_native_slot` → `project_geometry` →
`lease_parent` → `container_token` → `begin_session` →
`session.open(Docked)` → `child_enumerated`（child HWND 723124 を
実観測）→ `child_contract` → `deactivate` → `destroy` →
`notify_terminated` → `session_concluded`（`Closed`+`concluded`）→
`session_events`（drain 結果がちょうど
`[Attached{Docked}, Terminated{HostLost}]`）→
`notify_terminated_again`（binding 除去済みのため no-op・追加
event なし = exactly-once の裏付け）→ `child_hwnd_detached`
（`IsWindow(child)==0`、instance drop → `close_view` →
`removed()` → `DestroyWindow` の実効果）→ `host_survives`
（`HostCommand::Scan` → `Scanned`）→ `destroyed_handle_rejected`
（stale handle → `HostError::InvalidHandle`）。

graceful `session.close` は呼んでいない — terminal event は
`Terminated` であり `Closed` event は発行されない。teardown 側の
2 回目 `Destroy` は `InvalidHandle` で無害（`let _ =`）。

## `test-ui-plugin-dock-real-headless`（blocked-probe）

`plugin-dock-real-headless.jsonl`: header + 13 case 行。全行
`verdict:"not_run"`・`observed:"environment_blocked"`・
`environment_kind:"deterministic-backend"`（`real-window` 不請求）・
self-check violation 0・exit 0。新規行を含む fail-closed が
headless host で deterministic に証拠化された。

## `test-ui-plugin-dock`（headless lane 非回帰）

`plugin-dock-scenario.jsonl`: 8 case — `view-mount`/`artifact`/
`host-lifecycle`/`session-lifecycle`/`host-terminate`/
`open-or-degrade` = pass、`editor-attach` = unsupported、
`dock-pixels` = not_run。本 slice は headless catalog を変更して
いないため verdict 不変。

## `plugin-dock-console`（`plugin-dock-console.jsonl` + list 投影）

`--run` 全 8 cell を `lane=plugin-dock-console` で実行 — verdict は
headless lane と同一。`--list --include-real`（補助実行）は 21 行を
出力し real selector 実行拒否（exit 2）は
`tests/console.rs` の `real_lane_selectors_refused` が非回帰を
担保。

## NotRun / Unsupported / Fail（意図的・後続検証が必要）

- `real-dcomp-pixel` = fail: 既知の GPUI DComp topmost occlusion。
  `scarecrow173/ruviform#532` 決裁待ち — 本 slice で verdict を
  変えない・緩和しない。
- `real-window-dpi`/`real-window-ime`/`real-window-close` =
  unsupported: adapter 公開面に API が無い正直な判定（既存）。
- `real-editor-resize`/`real-editor-redock` = unsupported:
  `InProcessEditorOps` の定義済み `Unsupported` refusal（既存）。
- 残りの Issue `scarecrow173/ruviform#389` 受入（session
  Move/Degraded/crash/hang/recovery・実 Window RT 非干渉・adapter
  verb 実装・consumer gates）は後続 slice。
- 3OS・`*-msrv`・`*-full` 系: 本 slice の対象外（task spec 明記）。
- docs index（`docs/**/README.md`）: 生成物のため Worker は未更新
  — Coordinator が統合時に再生成。
