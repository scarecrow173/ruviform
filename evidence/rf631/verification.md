# rf631 plugin-dock scenario — verification record

Issue `scarecrow173/ruviform#631` first slice（plugin-dock scenario
skeleton）のローカル検証記録。canonical = `mise` lane。

- Date: 2026-11-15
- OS: Windows (x86_64)
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1
- Repo SHA: lane は merge 前 `d512334ad533c924e8e35c0c02a7f5b2ce13a676`
  と merge 後 `810ceac5563ec505f623c9a8fd90ad453fe40d49` の両方で実走。
  `plugin-dock-scenario.jsonl` は merge 後 HEAD の記録（verdict・
  artifact digest は同一）。

## Lane results

| Command | Result |
| --- | --- |
| `mise run fmt-check` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test` | pass (exit 0, `-D warnings`) |
| `mise run check ruviform-plugin-dock-test` | pass (exit 0) |
| `mise run test ruviform-plugin-dock-test` | pass (exit 0 — integration 2 tests) |
| `mise run check ruviform-audio-test` | pass (exit 0) |
| `mise run lint ruviform-audio-test` | pass (exit 0) |
| `mise run test ruviform-audio-test` | pass (exit 0) |
| `mise run check ruviform-ui-test` | pass (exit 0) |
| `mise run lint ruviform-ui-test` | pass (exit 0) |
| `mise run test ruviform-ui-test` | pass (exit 0 — 79 unit + integration suites) |
| `mise run check ruviform-plugin-test-vst3` | pass (exit 0) |
| `mise run lint ruviform-plugin-test-vst3` | pass (exit 0) |
| `mise run test ruviform-plugin-test-vst3` | pass (exit 0 — 3 tests) |
| `mise run test-ui-scenario-contract` | pass (exit 0) |
| `mise run test-ui-scenario-contract-fake` | pass (exit 0) |
| `mise run test-ui-scenario-contract-gpui` | pass (exit 0) |
| `mise run test-ui-native-slot` | pass (exit 0 — 13 tests) |
| `mise run test-ui-golden` | pass (exit 0 — 8 tests) |
| `mise run test-ui-driver-contract` | pass (exit 0 — 7 tests) |
| `mise run test-ui-gpui` | pass (exit 0) |
| `mise run check-ui-driver-boundary` | pass (exit 0 — dep-graph/signature/allowlist) |
| `mise run test-ui-plugin-dock` | pass (exit 0) |

## `test-ui-plugin-dock` 実走 verdict（`plugin-dock-scenario.jsonl`）

| case | driver | verdict | 備考 |
| --- | --- | --- | --- |
| `view-mount` | fake | pass | semantic contract conformance のみ — dock/window/pixel 証拠ではない（detail に明記） |
| `artifact` | plugin-host-probe | pass | `test_gain` cdylib 解決 + SHA-256 digest + descriptor 記録 |
| `host-lifecycle` | plugin-host-probe | pass | instantiate→initialize→activate→process→deactivate→destroy |
| `editor-attach` | plugin-host-probe | unsupported | `createView` null → `HostError::Unsupported` → `EditorOpError::Unsupported`（test_gain は editor surface を持たない） |
| `dock-pixels` | plugin-host-probe | not_run | docked editor surface 無し + VisualCapture driver 無し — 画素未取得を正直に記録 |

## Artifact evidence（実測値）

- path: `target/debug/deps/ruviform_plugin_test_vst3.dll`（env var 未設定時の自動解決経路）
- plugin: `test_gain` / id `218a4f6d7c3b059ef1a24c68b9d37e51` / vendor `Ruviform Test` / version `0.0.0`
- SHA-256: `a7f50c64669cf0f9ef71ce20c20a10d76fae89a9dbba6b6b518d1cc5b0ff140b`
- `environment_kind`: `real-abi-test-plugin`（実 ABI バイナリを実走した case のみが名乗る）

## NotRun / Unsupported（意図的・後続検証が必要）

- 実 Native Dock・実 child HWND・画素・Input・Resize・Focus・ReDock:
  `test_editor` fixture（`scarecrow173/ruviform#630` 残存 scope）到着後に
  同じ case が `Pass`/`Fail` を返すかで判定。本 slice では
  `unsupported`/`not_run` を維持。
- `RealWindow` kind: 実 Window driver lane 未実装のため未使用。
- 3OS・`*-msrv`・`*-full` 系: 本 slice の対象外（task spec 明記）。
- `mise exec -- prek run --all-files`: 実施結果は PR 本文に記録。

## Honest-verdict confirmation

- artifact 未解決時は integration test
  `missing_artifact_fails_closed_as_not_run` が `NotRun`/`Fail` 系を
  確認（silent pass 無し）。
- `fixture_run_reports_honest_verdicts` が上表 verdict を assert。

---

# Real-window dock lane（second slice — `rf631.plugin-dock-real`）

`mise run test-ui-plugin-dock-real`（manual/opt-in — CI 必須でない）を
interactive Windows session で実走。JSONL 正本:
`evidence/rf631/plugin-dock-real-scenario.jsonl`。

## 実走 verdict（2026-11-15, Windows x86_64, repo `5c38907b`）

| case | verdict | evidence |
| --- | --- | --- |
| `real-window-mount` | pass | `plugin_dock_view_spec()` が実 OS window に mount、`plugin.editor` NativeSlot 解決・観測 |
| `real-native-slot` | pass | declare → bind → renderer projection（Visible+実 geometry）→ `lease_parent`（実 container HWND）→ `validate_lease` → `container_token`=50269688 → release → released/stale/forged lease の validate/redeem 拒否 — 全 step 実走 |
| `real-editor-attach` | unsupported | `test_gain` `createView` null → `HostError::Unsupported` → `EditorOpError::Unsupported`。slot/lease/redeem chain 自体は実 container lease 経由で実走済み（`failed_unsupported=true`） |
| `real-capture` | pass | 1280×800 `win32-printwindow` 実画素 — window/slot 領域の画素であり plugin editor 画素ではない（detail 明記） |

- chain 記録: `NativeSlot → GpuiSurfaceAdapter（実 container HWND）→
  lease_parent → ParentSlotLease → InProcessEditorOps redeem →
  session.open(Docked)` — `real-editor-attach` で `session.open` まで
  実走し plugin 側 refuse を観測。
- `environment_kind=real-window` は実走 case のみ。
- plugin artifact: `test_gain` id `218a4f6d…` SHA-256
  `9c3cfc6bb554a8439e190347b5f6144fe0e675a3f9b50fe84c59449729a530e7`。

## NotRun / Unsupported / Deferred（後続）

- dock-pixels・child HWND input/resize/focus/re-dock・DComp #532 判定:
  `test_editor` 実物（#630 残存 scope）到着まで NotRun/Deferred。
- `real-editor-attach` の Pass: 実 editor が `OpenGui` を実 container
  lease で accept する fixture が必要。
- headless/非対話 host では全 case が `NotRun` + environment_blocked
  で fail-closed（exit 0）。
- 3OS・`*-msrv`・`*-full` 系: 対象外。

## 検証サマリ（canonical = mise）

- `mise run fmt-check`: pass
- `mise run check/lint/test ruviform-ui-gpui`（+`--features real-scenario`）: pass
  （`tests/real_window.rs::real_os_input_scenario` は実 OS input の
  timing flake で 1 度 fail → 単独再実行で pass。native-slot 変更と
  無関係）
- `mise run check/lint/test ruviform-plugin-dock-test`（default +
  `--features real-dock`）: pass
- `mise run test-ui-plugin-dock`: pass（headless lane 非回帰 —
  verdict 不変）
- `mise run test-ui-scenario-contract` / `-fake` / `-gpui`: pass
- `mise run test-ui-gpui`: pass
- `mise run check-ui-driver-boundary`: pass（`ruviform-plugin-dock-test`
  を composition-root として boundary.json に登録 — Coordinator 承認済み）
- `mise run test-ui-plugin-dock-real`: pass（上表 — 実走 evidence）
- `mise exec -- prek run --all-files`: （結果は PR 本文に記録）

# Scenario catalog + driver projection lane（third slice — `rf631.plugin-dock` catalog 化）

Issue `scarecrow173/ruviform#631` 第三 slice（plugin-dock scenario
catalog + driver projection）のローカル検証記録。catalog 化は case
列挙の表への移行であり verdict 権威を持たない — 本節は catalog 化前後で
JSONL verdict が同一であることの確認記録。

- Date: 2026-11-15
- OS: Windows (x86_64)
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1
- Repo SHA: `f03390d046ebb640ca7e3366984afed174c6d9c2`（branch base、
  catalog 変更は未 commit の working tree で実走）

## Lane results（canonical = mise）

| Command | Result |
| --- | --- |
| `mise run fmt-check` | pass (exit 0) |
| `mise run check ruviform-ui-test` | pass (exit 0) |
| `mise run lint ruviform-ui-test` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-ui-test` | pass (exit 0 — catalog unit test 4 件含む） |
| `mise run check ruviform-plugin-dock-test` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-plugin-dock-test` | pass (exit 0 — integration 4 tests: catalog↔report 対応・projection 被覆・fail-closed・honest verdicts） |
| `mise run check/lint/test ruviform-plugin-dock-test --features real-dock` | pass (exit 0 — real-dock unit test `blocked_reports_fail_closed` が catalog 導出を検証） |
| `mise run test-ui-plugin-dock` | pass (exit 0) |
| `mise run test-ui-scenario-contract`（fake 17 + gpui 7） | pass (exit 0) |
| `mise run check-ui-driver-boundary` | pass (exit 0 — dep-graph/signature/allowlist。catalog は GPUI 非依存を維持） |
| `mise exec -- prek run --all-files` | pass (exit 0) |

## `test-ui-plugin-dock` verdict 非回帰確認

catalog 化後の `plugin-dock-scenario.jsonl`（本 slice で再生成）は
first slice 記録と同一 verdict:

| case | driver | verdict |
| --- | --- | --- |
| `view-mount` | fake | pass |
| `artifact` | plugin-host-probe | pass |
| `host-lifecycle` | plugin-host-probe | pass |
| `editor-attach` | plugin-host-probe | unsupported |
| `session-lifecycle` | plugin-host-probe | pass |
| `host-terminate` | plugin-host-probe | pass |
| `open-or-degrade` | plugin-host-probe | pass |
| `dock-pixels` | plugin-host-probe | not_run |

- `HEADLESS_MATRIX`（`&'static ScenarioMatrix`、Fake/PluginHostProbe
  2 binding・8 entry）が case id/scenario/driver/required/order の
  単一ソース。`run_all` = `run_matrix` 投影。
- `tests/plugin_dock.rs` の `catalog_matches_run_all_reports` が
  cells↔report の 1:1 対応を、`catalog_projection_covers_every_
  declared_requirement` が `project()` 全 runnable を検証。
- real-dock lane は `real_entries()`（session-scoped 4 entry）を
  live/blocked 両経路で共有。`blocked_reports_fail_closed` が
  entries↔reports の一致 + 全件 `NotRun`/`environment_blocked`/
  `real-window` 非主張を検証。

## NotRun / Unsupported / Deferred（後続 — 変更なし）

- Scenario Console / Test Lab GUI 本体: 後続 slice。
- `test_editor` 実物・実 Native Dock Pass・dock-pixels・child HWND
  input/resize/focus/re-dock・DComp #532 判定: #630 残存 scope まで
  NotRun/Unsupported 維持。
- 3OS・`*-msrv`・`*-full` 系: 対象外。
