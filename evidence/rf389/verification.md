# rf389 real-session-lifecycle — verification record

Issue `scarecrow173/ruviform#389` の real-window session lifecycle
slice（`rf631.plugin-dock-real` lane 15 → 21 row — degraded /
recovery / timeout pass-oracle、visibility refusal observation、
crash/hang honest `Unsupported` boundary）のローカル検証記録。
canonical = `mise` lane。

- Date: 2026-11-17
- OS: Windows (x86_64) — interactive desktop session あり（real lane 実走）
- Toolchain: rustc 1.99.1 / cargo 1.99.1（mise 管理）
- Repo SHA: lane JSONL 記録の `repo_sha` =
  `303db66b3ae0e66d1b0d0c2f51867b5c7724be6e`（作業開始時 HEAD）。
  commit 前の working tree で実走 — コード差分は本 slice の
  `real_dock.rs` / `real_catalog.rs` / `tests/console.rs` /
  crate docs / docs chain のみ。

## Lane results

| Command | Result |
| --- | --- |
| `mise run fmt-check` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test` | pass (exit 0, `-D warnings`) |
| `mise run check ruviform-plugin-dock-test` | pass (exit 0) |
| `mise run test ruviform-plugin-dock-test` | pass (exit 0) |
| `mise run check ruviform-plugin-dock-test --features real-dock` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test --features real-dock` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-plugin-dock-test --features real-dock` | pass (exit 0 — `real_catalog_matches_entries`・`blocked_reports_fail_closed`・console `list_include_real_appends_real_lane_rows` 29 行を含む全 test green) |
| `mise run check ruviform-plugin-dock-test --features lab-gui` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test --features lab-gui` | pass (exit 0) |
| `mise run build ruviform-plugin-test-vst3`（`test-ui-plugin-dock-real` 内） | pass (exit 0 — `test_editor`/`test_gain` cdylib 生成） |
| `mise run test ruviform-plugin-host --test editor_session --test editor_session_driver` | pass (exit 0) |
| `mise run test-ui-plugin-dock` | pass (exit 0) |
| `mise run test-ui-plugin-dock-real-headless` | pass (exit 0 — 21 行全て `not_run`+`observed.environment_blocked`、violation 0) |
| `mise run test-ui-plugin-dock-real` | exit 1 — **設計どおりの正直な regression 信号**: 既知 `real-dcomp-pixel` が `fail`（GPUI DComp topmost occlusion、`scarecrow173/ruviform#532` 決裁待ち・本 slice で変更なし・baseline 実走でも同一 fail を確認済み）。新規 6 row は設計どおりの verdict |
| `mise run test-ui-plugin-dock-console` | pass (exit 0) |
| `mise run check-ui-driver-boundary` | pass (exit 0 — dep-graph/signature/allowlist) |
| `mise exec -- prek run --all-files` | pass (exit 0) |

補助証拠（canonical = mise）: `plugin-dock-console --list
--include-real`（`--features real-dock`）を
`plugin-dock-console-list-include-real.jsonl` に保存 — 29 行
（headless 8 + real 21）projection、6 新規 row 含む全件一致。

## `test-ui-plugin-dock-real` 実走 verdict（`plugin-dock-real-scenario.jsonl`）

interactive Windows session で全 21 case を実走。新規 6 row の
観測 step 詳細:

| case | verdict | 観測 |
| --- | --- | --- |
| `real-session-degraded` | **pass** | `open_or_degrade(Floating→Docked)` → `Degraded{Docked}` fallback attach。events `[Degraded { mode: Docked }, Closed]`（`Attached` 非発火）+ child HWND contract/enumeration/teardown 全 pass |
| `real-session-recovery` | **pass** | `open(Floating)` → `Failed{Unsupported}`（`failed_unsupported=true`・`editor_live=false`）→ `recover(Docked)` attach。events `[Failed { cause: Unsupported }, Recovered { mode: Docked }, Closed]` + child HWND contract 全 pass |
| `real-session-timeout` | **pass** | `SessionDeadlinePolicy{timeout: 0s}` → `begin_detach`（request=2・deadline armed `3.6µs`）→ `tick_all()==1` → `Failed{Timeout}`。events `[Attached { mode: Docked }, Failed { cause: Timeout }, Closed]` + child HWND teardown 全 pass |
| `real-editor-visibility` | **unsupported** | `set_visibility(false)` → `EditorOpError::Unsupported`（no hide/show verb on L0 surface）。session は `Attached{Docked}` 維持・events `[Attached { mode: Docked }, Closed]`（`Hidden`/`Shown` 0 件）を確認した上で refused boundary として記録 |
| `real-session-crash` | **unsupported** | in-process lane は fixture の process-killing crash fault を生存できず out-of-process plugin host は存在しない — fault を実行せず boundary row として記録 |
| `real-session-hang` | **unsupported** | 同上 — wedging/hang fault は lane を永久に塞ぐため実行しない。architectural boundary を明示 |

既存 15 row の verdict は前 slice から不変（`real-dcomp-pixel`
のみ既知 `fail` — #532 決裁待ちの GPUI DComp topmost occlusion、
baseline/ baseline2 / dcomp-disabled の 3 実走でも同一
`green_pixels=0` fail を確認済みで本 slice 起因ではない）。

## 設計との対応

- degrade/recovery/timeout は `attempt_real_attach` の `drive_open`
  seam 経由で case 固有の session op を実行 — open 成功時のみ
  observe が lease 生存中に走り、close → child teardown →
  event 収束の共有 tail を通る。
- `real_editor_ops` は `RealOsGuiTest::redeem_native_lease` 経由の
  実 ledger redeem のみ — fake/direct HWND path なし。
- `real_catalog.rs`（driver-neutral）と `real_dock.rs`
  `real_entries`（executable）は case id / scenario id /
  required capabilities で 1:1 同期 — `real_catalog_matches_entries`
  test が gate。
- crash/hang は `required: &[]` で preflight を通り body が
  `Unsupported` を返す — `NotRun` 偽装・catalog 欠落なし。
