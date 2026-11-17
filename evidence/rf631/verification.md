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
- Repo SHA: catalog 変更は `f03390d0` base の working tree で初回実走、
  `origin/main`（`3ae35b85`、issue-630 L1 runner slice）merge 後の
  `41857b5d` で全 canonical lane を再検証。`plugin-dock-scenario.jsonl`
  は merge 後 HEAD の記録（verdict は初回実走と同一）。

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

---

# rf631 plugin-dock console slice — verification record

Issue `scarecrow173/ruviform#631` 後続 slice（headless Scenario
Console — `plugin-dock-console` bin）のローカル検証記録。
canonical = `mise` lane。cargo 直実行は補助証拠として区別する。

- Date: 2026-11-16
- OS: Windows (x86_64)
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1
- Repo SHA: `bff3603b` base の working tree（console 変更は未 commit
  のため `repo_sha` は base HEAD を記録）

## Lane results（canonical = mise）

| Command | Result |
| --- | --- |
| `mise run fmt-check` | pass (exit 0) |
| `mise run check ruviform-plugin-dock-test` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-plugin-dock-test` | pass (exit 0 — console integration 14 tests + 既存 4 tests) |
| `mise run check/lint/test ruviform-plugin-dock-test --features real-dock` | pass (exit 0 — `real_catalog_matches_entries`・`list_include_real_appends_real_lane_rows` 含む） |
| `mise run test-ui-plugin-dock` | pass (exit 0 — verdict 非回帰、下記） |
| `mise run test-ui-scenario-contract`（fake + gpui） | pass (exit 0) |
| `mise run check-ui-driver-boundary` | pass (exit 0 — dep-graph/signature/allowlist。console 経路は GPUI 型を漏らさない） |
| `mise exec -- prek run --all-files` | pass (exit 0) |

## `test-ui-plugin-dock` verdict 非回帰確認

console 追加後に lane を再実行した `plugin-dock-scenario.jsonl` は
pre-console 記録と同一 verdict（差分は repo_sha・worktree path・
fixture DLL sha256 の provenance のみ — fixture は worktree 毎に
再 build されるため digest が変わる。verdict 列は不変）:

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

- `scenario::run_selected` が `run_all` の select-all 投影として
  `HEADLESS_MATRIX.cells()` → `run_entry` を catalog 順に実行。
  `HeadlessContext` 構築は scenario module 内に留まり、console は
  selector predicate のみを渡す。
- `real_catalog` は `real-dock` feature 非依存の metadata table
  （`RUN_ID`/`SCENARIO`/`LANE`/`DRIVER`/`REAL_CATALOG`）。
  feature 付き build では `real_catalog_matches_entries` が
  `real_entries()` との case/scenario/required 一致を検証し、
  `real_dock` の lane 文字列リテラルは `real_catalog::LANE` を
  共有する。

## 補助証拠（cargo 直実行 — canonical ではない）

| Command | Exit | Output |
| --- | --- | --- |
| `cargo run --locked -p ruviform-plugin-dock-test --bin plugin-dock-console -- --list` | 0 | `plugin-dock-console-list.jsonl`（8 行 `type:"catalog"`、verdict 語彙なし） |
| `cargo run --locked -p ruviform-plugin-dock-test --features real-dock --bin plugin-dock-console -- --list --include-real` | 0 | `plugin-dock-console-list-include-real.jsonl`（12 行 — real 4 行は `session_required:true`/`lane:"real-window-dock"`/`runnable:null`） |
| `cargo run --locked -p ruviform-plugin-dock-test --bin plugin-dock-console -- --run --case view-mount --out evidence/rf631/plugin-dock-console.jsonl` | 0 | `plugin-dock-console.jsonl`（run header `lane:"plugin-dock-console"` + `view-mount` pass） |

selector 拒否系（unknown case/scenario/driver・空 intersection・
real-lane 指定・CLI misuse・default build の `--include-real`）は
`tests/console.rs` の 14 test で exit 2 / 非実行を検証済み。

## NotRun / Unsupported / Deferred（後続 — 変更なし）

- GPUI 製 Scenario Console / Test Lab GUI 本体: 後続 slice。
- `test_editor` 実物・実 Native Dock Pass・dock-pixels・child HWND
  input/resize/focus/re-dock・DComp #532 判定: #630 残存 scope まで
  NotRun/Unsupported 維持 — console 整備で「解消済み」にしない。
- `mise.toml` への console lane task: 別 Worker 占有中のため本
  slice では追加しない（後続 slice 向け）。
- 3OS・`*-msrv`・`*-full` 系: 対象外。

---

# real-editor-attach slice — verification record（`test_editor` 実 IPlugView）

Issue `scarecrow173/ruviform#631` 後続 slice（`real-editor-attach`
を `test_editor` 実 `IPlugView` で `Unsupported` → `Pass` 化 +
実 child HWND の OS 観測）のローカル検証記録。
canonical = `mise` lane。cargo 直実行は補助証拠として区別する。

- Date: 2026-11-16
- OS: Windows (x86_64) — interactive desktop session で実走
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1
- Repo SHA: `a6d57355dcc44a732ee5352aa26aa33ef94473fd` base の
  working tree（変更は未 commit のため `repo_sha` は base HEAD を記録）

## Lane results（canonical = mise）

| Command | Result |
| --- | --- |
| `mise run fmt-check` | pass (exit 0) |
| `mise run check ruviform-plugin-dock-test` | pass (exit 0) |
| `mise run lint ruviform-plugin-dock-test` | pass (exit 0, `-D warnings`) |
| `mise run test ruviform-plugin-dock-test` | pass (exit 0 — integration 4 tests + console 14 tests 含む） |
| `mise run check/lint/test ruviform-plugin-dock-test --features real-dock` | pass (exit 0 — `--locked` 整合確認済み） |
| `mise run test-ui-plugin-dock` | pass (exit 0 — headless lane 非回帰、verdict 不変） |
| `mise run check-ui-driver-boundary` | pass (exit 0 — dep-graph/signature/allowlist。`windows-sys` 追加後も GPUI 型・driver 境界を漏らさない） |
| `mise run test-ui-plugin-dock-real` | pass (exit 0 — interactive Windows session で実走、下記 verdict) |
| `mise exec -- prek run --all-files` | （結果は PR 本文に記録） |

## `test-ui-plugin-dock-real` 実走 verdict（`plugin-dock-real-scenario.jsonl`）

| case | verdict | evidence |
| --- | --- | --- |
| `real-window-mount` | pass | `plugin_dock_view_spec()` が実 OS window に mount、`plugin.editor` NativeSlot 解決・観測 |
| `real-native-slot` | pass | declare → bind → renderer projection → `lease_parent` → validate → `container_token` → release + stale/forged 拒否 |
| `real-editor-attach` | **pass** | `test_editor` 実 `IPlugView` が実 container lease 下に実 child HWND を生成・detach — 全 OS 観測 step 実走（下記） |
| `real-capture` | pass | window/slot 領域の実画素 — plugin editor 画素ではない（detail 明記、不変） |

### `real-editor-attach` の実測観測値（observed.steps — check 行は全て pass、`thread_affinity` は観測記録の `note`）

- `container_token`: `53481548`（実 container HWND = leased container の OS parent token）
- `session.open(Docked)` → `Ok(EditorCapabilities)`（`failed_unsupported=false`）
- `child_enumerated`: `EnumChildWindows(container)` で **1 child** —
  `hwnd=27398600`, `parent_matches=true`, `visible=true`, `alive=true`,
  `thread_id=17592`
- `child_contract`: 全 child が `GetParent(child)==container`・
  `IsWindowVisible!=0`・`IsWindow!=0` — pass
- `thread_affinity`（verdict `note` — 観測記録であり check ではない）:
  `caller_thread=17592`（scenario caller）、
  `container_thread=49084`（container HWND の pump 所有 thread =
  GPUI app thread）、`child_threads=[17592]`（`attached` を実行した
  thread が child を生成）。**caller ≠ container pump thread —
  Issue #389 D1 の thread-affinity 契約からの逸脱を観測事実として
  記録**（Pass 判定の根拠には使わない。production host では UI
  thread 上での attach が前提）
- `session.close` → pass（production `close_gui` → `removed()` →
  `DestroyWindow` 経路）
- `child_hwnd_detached`: `IsWindow(27398600)==0` — **detach 後に
  child HWND が実際に破棄されたことを OS が確認**
- `session_concluded`: `Closed`+`concluded` — pass
- `session_events`: `["Attached { mode: Docked }", "Closed"]` — pass
- `cleanup`: `session-ended/lease-released/slot-released/
  instance-destroyed`

### plugin artifact evidence（実測値）

- path: `target/debug/deps/ruviform_plugin_test_vst3.dll`
- plugin: **`test_editor`** / id `624c8a3f1e9d057ba6f348d9c2e75b81`
  （`TestEditorComponent::CID` 0x3F8A4C62… の FUID 表現）/
  vendor `Ruviform Test` / version `0.0.0`
- SHA-256: `6ff44b49b97d74da55c0189a9ebf28719221463ad2ac5e743a95efa91d0ead97`
- `environment_kind`: `real-window`（実走 case のみが名乗る — 不変）

## headless lane 非回帰

`mise run test-ui-plugin-dock` 再実行 — `plugin-dock-scenario.jsonl`
の verdict は catalog slice 記録と同一（`editor-attach` は
`test_gain` のまま `unsupported` — headless lane は本 slice で
変更していない）:

| case | verdict |
| --- | --- |
| `view-mount` / `artifact` / `host-lifecycle` / `session-lifecycle` / `host-terminate` / `open-or-degrade` | pass |
| `editor-attach` | unsupported（test_gain は editor を持たない — 正しい verdict） |
| `dock-pixels` | not_run |

## NotRun / Unsupported / Deferred（後続 — 本 slice で claim しない）

- dock-pixels / editor pixel・合成判定: `scarecrow173/ruviform#532`
  DComp 未解決 — Pass を主張しない。
- child HWND input/focus・resize negotiation・re-dock: 現行
  Unsupported/NotRun 維持（本 slice の観測は存在・parent・
  visibility・detach のみ）。
- 非 Windows: capability pre-flight で `Unsupported`/`NotRun` の
  まま（`oschild` stub は観測を捏造しない）。
- headless/非対話 host: `environment_blocked` → `NotRun` 不変。
- 3OS・`*-msrv`・`*-full` 系: 対象外。
- `mise.toml` `test-ui-plugin-dock-real` description の
  `test_gain` 前提文言: 本 Worker の lease 外 — 後続 hygiene slice。
  → **#631 Test Lab slice で解消済み**（下記）。

---

# Issue #631 Test Lab slice — `plugin-dock-lab` GPUI Scenario Console 検証記録

`plugin-dock-lab`（feature `lab-gui`）実装・mise lane 追加・
`test-ui-plugin-dock-real` description 修正のローカル検証記録。
canonical = `mise` lane。GUI dogfooding は interactive Windows
session での実走（PostMessage click — 後述）。

- Date: 2026-11-16
- OS: Windows (x86_64) — interactive desktop session で実走
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1
- Repo SHA: `eafe0a08b71bd9a74566ae6678c3e0683fce1421` base の
  working tree（変更は未 commit のため `repo_sha` は base HEAD を記録）

## Lane results（canonical = mise / cargo）

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass (exit 0) |
| `cargo check --locked --all-targets -p ruviform-plugin-dock-test --features lab-gui` | pass (exit 0) |
| `cargo clippy --locked -p ruviform-plugin-dock-test --features lab-gui --all-targets --no-deps -- -D warnings` | pass (exit 0 — 本 crate の lint は clean) |
| `mise run lint ruviform-plugin-dock-test --features lab-gui`（canonical・deps 込み） | **fail (exit 101) — scope 外の既存 issue**: path dep `ruviform-audio-test` の `src/backend/stream.rs:885` `pump_input` が `too_many_lines`（105/100）。`git diff HEAD` で同 crate 無変更・`cargo clippy -p ruviform-audio-test` 単独でも同一 error を再現 — base 由来の upstream drift（#671 系）で本 slice とは無関係。対処は scope 外のため未実施・PR で報告 |
| `cargo test --locked -p ruviform-plugin-dock-test`（default） | pass (exit 0) |
| `cargo test --locked -p ruviform-plugin-dock-test --features lab-gui` | pass (exit 0 — lab unit 7 + console 14 + plugin-dock 4) |
| `cargo test --locked -p ruviform-plugin-dock-test --features real-dock` | pass (exit 0) |
| `cargo check --locked --all-targets -p ruviform-plugin-dock-test --features real-dock` | pass (exit 0) |
| `mise run test-ui-plugin-dock-console` | pass (exit 0 — `--list` projection + `--run` JSONL) |
| `mise run test-ui-plugin-dock-lab` | pass (exit 0 — window 起動 → Quit で clean exit。dogfooding は上記 GUI 節) |
| `mise run check-ui-driver-boundary` | pass (exit 0 — dep-graph/signature/allowlist。`lab-gui` feature 追加後も GPUI 型・driver 境界を漏らさない） |
| `mise run test-ui-plugin-dock`（headless lane 非回帰） | verdict 不変（`plugin-dock-scenario.jsonl` 差分は repo_sha/ artifact path/sha256 の環境 field のみ — verdict 語彙は同一） |
| workspace `cargo clippy --workspace -- -D warnings` | **fail — 同上の scope 外既存 issue**（`ruviform-audio-test` `stream.rs:885`）。scoped lint（`--no-deps`）は上記のとおり pass |
| `mise exec -- prek run --all-files` | （結果は PR 本文に記録） |

## GUI dogfooding（interactive Windows session・実 window）

`plugin-dock-lab --out evidence/rf631/plugin-dock-lab.jsonl` を
実 GPUI window（title `Ruviform Test Lab — plugin-dock scenario
console`、1116×899 outer）で起動し、Win32 `PostMessage
(WM_LBUTTONDOWN/UP)` で client 座標 click を送って操作した
（GPUI content は UIA tree に出ないため accessibility 経路は
使用不可 — 実測位置は screenshot 画素解析で同定）。

| 操作 | 観測結果 |
| --- | --- |
| window 起動・描画 | catalog 12 row（headless 8 + real 4）・status/note/results/actions が `ruviform-ui` ViewSpec 語彙で描画。palette は `ViewPalette::fallback()` dark |
| catalog row select | click → 該当 row が `surface_raised` で highlight（selection state が view に反映） |
| Run all headless | +9 JSONL 行（run header `lane=plugin-dock-lab` + 8 case）。verdict: pass×6・`editor-attach` unsupported・`dock-pixels` not_run — catalog の正直な verdict をそのまま記録 |
| Run selected（headless `artifact`） | +2 JSONL 行（run header + case `artifact` verdict pass） |
| Run selected（real row `real-capture`） | **JSONL 増加なし・非実行** — real row は `test-ui-plugin-dock-real` 委譲の fail-closed 経路（Inform/reject）で evidence を捏造しない |
| Quit | process clean exit |

証拠 file: `evidence/rf631/plugin-dock-lab.jsonl`（2 つの run
block — run-all 8 case + run-selected 1 case、いずれも実走）。

### dogfooding で発見・修正した defect

- run-all 完了後に results table が伸び actions panel が window
  下端で clip され Quit へ到達不能 → `logical_size` を
  (1100, 720) → (1100, 860) に修正（`resizable` 維持）。修正後は
  全結果表示後も 3 button が viewport 内に残ることを再確認。

## mise.toml lane 追加・修正

- `test-ui-plugin-dock-console` 新設（canonical headless console）。
- `test-ui-plugin-dock-lab` 新設（opt-in windowed — 対話 session
  前提・CI-required gate ではない）。
- `test-ui-plugin-dock-real` description を #670 後の実態へ修正
  （`test_gain` 前提・editor attach unsupported 前提の stale 文言
  → plugin-test-vst3 / `test_editor` 実 IPlugView / child HWND OS
  観測）。run block は不変 — lane 実行系は前 slice の検証済みの
  まま。

## NotRun / Unsupported / Deferred（本 slice で claim しない）

- `plugin-dock-lab` の headless/非対話 host での fail-closed
  blocked 経路（`NotRun`+`environment_blocked`・exit 0）は
  unit test で検証済みだが、実 headless host での lane 実走は
  未実施（本検証環境は interactive session）。
- lab GUI の pixel/golden 証拠・実 native dock・editor attach の
  新規 claim なし — lab が生成するのは headless scenario の
  verdict/evidence のみ。real row は委譲表示で実行しない。
- PostMessage dogfooding は Windows 固有の手動経路 — CI lane・
  他 OS での GUI 実走は対象外（opt-in manual lane の定義どおり）。
- workspace 全体 clippy の `ruviform-audio-test` 既存
  `too_many_lines` — scope 外・別 slice での対応要否を判断。

---

# Issue #532 dcomp-pixel slice — verification record（2026-11-15 追記）

`scarecrow173/ruviform#532` 再現固定 slice: `real-dcomp-pixel` case
追加 + `GPUI_DISABLE_DIRECT_COMPOSITION`（方式 A'）実機観測。

- OS: Windows 11 (10.0.26300.9974) x86_64
- GPU: NVIDIA GeForce RTX 4070 SUPER
- Toolchain: rustc/cargo 1.99.1 (58cf8f9c3 2026-10-13)
- Repo SHA: `de3ed4e81145674a635bacbe597d8d2a87a44b24`
  （branch `feat/532-dcomp-pixel`、本 slice の変更は未コミット
  diff として存在 — JSONL `environment.repo_sha` は HEAD を記録）
- Session: interactive Windows desktop session（real-window lane
  が実 window を生成・`environment_kind=real-window`）

## Lane results（canonical = mise）

| Command | Exit | Result |
| --- | --- | --- |
| `mise run fmt-check` | 0 | pass |
| `mise run check ruviform-plugin-dock-test` | 0 | pass |
| `mise run lint ruviform-plugin-dock-test` | 0 | pass（`-D warnings`） |
| `mise run test ruviform-plugin-dock-test` | 0 | pass |
| `mise run check ruviform-plugin-dock-test --features real-dock` | 0 | pass |
| `mise run lint ruviform-plugin-dock-test --features real-dock` | 0 | pass |
| `mise run test ruviform-plugin-dock-test --features real-dock` | 0 | pass（unit 3 + console 14 + plugin_dock 4） |
| `mise run test-ui-plugin-dock` | 0 | pass（headless verdict 非回帰） |
| `mise run test-ui-plugin-dock-console` | 0 | pass（`--list --include-real` が 13 行に更新） |
| `mise run test-ui-scenario-contract` | 0 | pass |
| `mise run check-ui-driver-boundary` | 0 | pass（dep-graph/signature/allowlist） |
| `mise exec -- prek run --all-files` | 0 | pass |

## `test-ui-plugin-dock-real` 実走（interactive session・実 window）

canonical lane: `mise run test-ui-plugin-dock-real`（baseline・
env 未設定）→ **exit 1**。`real-dcomp-pixel` verdict `fail` を含む
5 case 全て実走（`NotRun`/`environment_blocked` ではない）。
他 4 case（`real-window-mount`/`real-native-slot`/
`real-editor-attach`/`real-capture`）は `pass`。

| Run | env `GPUI_DISABLE_DIRECT_COMPOSITION` | JSONL | exit | `real-dcomp-pixel` verdict |
| --- | --- | --- | --- | --- |
| baseline canonical | 未設定（JSONL `null`） | `plugin-dock-real-scenario.jsonl` | 1 | fail |
| baseline 補助（`--pixel-artifacts` 付） | 未設定 | `plugin-dock-real-scenario-baseline.jsonl` | 1 | fail |
| baseline 第2サンプル | 未設定 | `plugin-dock-real-scenario-baseline2.jsonl` | 1 | fail |
| env-on 補助 | `"1"`（JSONL 生値記録） | `plugin-dock-real-scenario-dcomp-disabled.jsonl` | 1 | fail |

env-on run は canonical JSONL を上書きしないよう直接
`cargo run`（補助証拠・canonical = mise）で別 `--out` に記録。
`--pixel-artifacts` で PNG を `evidence/rf631/dcomp-pixel-*/`
に保存（baseline 2 回分・env-on 1 回分、各 3 枚:
screen/printwindow/windowdc）。

注: baseline2 の初回試行は fixture dll 未再 build で
`real-editor-attach`/`real-dcomp-pixel` が `not_run`
（`plugin artifact unavailable`・fail-closed 正常動作）となった
ため、`cargo build -p ruviform-plugin-test-vst3` 後に再実行して
上表の結果を得た。改ざん・取り繕いなし。

## `real-dcomp-pixel` 画素観測（child HWND 1278×798・1,019,844 px）

| Oracle | 役割 | baseline | env-on `"1"` |
| --- | --- | --- | --- |
| `screen_pixels`（`win32-bitblt-screen`・screen DC `BitBlt` of `GetWindowRect`） | **主 oracle** — 画面合成結果 | green 0 / frac 0.0 | green 0 / frac 0.0 |
| `printwindow_pixels`（`win32-printwindow`・`PW_CLIENTONLY|PW_RENDERFULLCONTENT`） | 診断 note | 0 / 0.0 | 0 / 0.0 |
| `windowdc_pixels`（`win32-windowdc`・`GetDC`+`BitBlt` of client） | 診断 note | 0 / 0.0 | **1,019,844 / 1.0** |

lifecycle steps（child 存在・parent 一致・visible/alive・
`session.close`・HWND 破棄・`Closed+concluded`・events
`[Attached{Docked}, Closed]`）は全 run で pass — attach 経路は
`real-editor-attach` と同一で健全。verdict `fail` の detail:
「editor lifecycle verified but 1 child screen capture(s) show no
EDITOR_COLOR green — consistent with the known GPUI `DComp`
topmost occlusion」。

## 観測の解釈（正直な記録・断定しない）

- **baseline**: 画面合成に緑が出ない = 既知 DComp topmost occlusion
  の再現を固定。`windowdc=0.0` は child の backing surface にも
  fill が観測されないことを示す（`DefWindowProcW` class brush の
  paint が window-DC surface に現れない条件は未解明 — DComp 合成下
  では child の GDI surface が提示経路に乗らない可能性）。
- **env-on（`"1"`）**: `windowdc=1.0` — child の backing surface は
  `EDITOR_COLOR` 緑で完全に満たされている（plugin は正しく描画
  している）にもかかわらず **screen 合成には依然として出ない**。
  `GPUI_DISABLE_DIRECT_COMPOSITION` は GPUI 0.2.2
  `platform.rs` で `"1"`/`"true"` を受理し
  `WindowsPlatform::new` で 1 回読むことを source 確認済みで、
  JSONL の `environment` に生値 `"1"` を記録。exit 127 は
  **未観測**（両 run とも exit 1 = `Fail` verdict による正常な
  lane 失敗。H1-H5 の exit-127 仮説は本環境では非該当）。
- 帰結（観測事実のみ）: 方式 A' 単独では本ホスト・GPUI 0.2.2・
  `test_editor` fixture の組合せで画面画素は回復しなかった。
  非 DComp 経路（`CreateSwapChainForHwnd`+`FLIP_SEQUENTIAL`）でも
  child HWND 画素が画面に合成されない可能性を示唆するが、
  env var が GPUI 初期化に実際に効いたかの内部確認手段は scenario
  内に無く、機序の断定はしない — 方式採否は後続 slice。

## Limitations

- 単一ホスト・単一 GPU（RTX 4070 SUPER）・単一 scale 環境での
  観測。`scale_factor` は JSONL で `null`。
- `PrintWindow` は `DefWindowProcW` class の child に対し green を
  返さない（実測 0.0）— child 自身の raster 証明としては機能
  せず、診断 note に留め verdict gate に不使用。
- screen `BitBlt` は観測時点のフレーム — DWM 合成タイミングの
  競合は排除できないが、env-on 2 サンプルで安定して 0.0。
- PNG artifact は opt-in（`--pixel-artifacts`）— canonical lane
  実行では未指定のため `plugin-dock-real-scenario.jsonl` の
  step row に `artifact` path は含まれない（補助 run の JSONL に
  は含まれる）。

## NotRun / Unsupported / Deferred（本 slice）

- headless host での `NotRun`+`environment_blocked` 経路は unit
  test（`blocked_reports_fail_closed`）で検証済みだが、本検証
  環境は interactive のため実 headless run は未実施。
- 方式採否・本 fix（`directx_renderer.rs` patch / adapter 変更 /
  env var 恒常化）は後続 slice — 本 slice は観測・証拠化のみ。
- `test-ui-plugin-dock-lab` / `test-ui-native-slot` / `test-ui-gpui`
  / `*-msrv` / 3OS 横断は本 slice の必須検証に含めず未実施
  （`test-ui-native-slot`・`test-ui-gpui` は ui-gpui/native-slot
  コード非接触のため任意 — 未実行）。

---

# Issue #532 dcomp-occlusion slice — verification record（2026-11-16 追記）

`scarecrow173/ruviform#532` 遮蔽層診断 slice: `real-dcomp-pixel`
case に query-only の遮蔽診断（`osoccl` module）を追加し、
baseline / `GPUI_DISABLE_DIRECT_COMPOSITION=1` env-on の双方で
実機観測を記録した。診断は read/query API のみで、既存 oracle・
verdict・exit code は不変。

- OS: Windows 11 (10.0.26300.9974) x86_64
- GPU: NVIDIA GeForce RTX 4070 SUPER
- Repo SHA: `2e856bca`（branch `feat/532-dcomp-occlusion`、
  review-protocol 指摘反映 commit。両 real run の JSONL
  `environment.repo_sha` はこの SHA を記録。初回実装 commit
  `0fe7bdec` での run も exp log に記録）
- Session: interactive Windows desktop session

## Lane results（canonical = mise）

| Command | Exit | Result |
| --- | --- | --- |
| `mise run fmt-check` | 0 | pass |
| `mise run check ruviform-plugin-dock-test` | 0 | pass |
| `mise run lint ruviform-plugin-dock-test` | 0 | pass（`-D warnings`） |
| `mise run test ruviform-plugin-dock-test` | 0 | pass |
| `mise run check ruviform-plugin-dock-test --features real-dock` | 0 | pass |
| `mise run lint ruviform-plugin-dock-test --features real-dock` | 0 | pass |
| `mise run test ruviform-plugin-dock-test --features real-dock` | 0 | pass |
| `mise run test-ui-plugin-dock` | 0 | pass（headless verdict 非回帰） |
| `mise run test-ui-plugin-dock-console` | 0 | pass |
| `mise run test-ui-scenario-contract` | 0 | pass |
| `mise run check-ui-driver-boundary` | 0 | pass（dep-graph/signature/allowlist） |
| `mise exec -- prek run --all-files` | 0 | pass |

## `test-ui-plugin-dock-real` 実走（interactive session・実 window）

| Run | env `GPUI_DISABLE_DIRECT_COMPOSITION` | JSONL | exit | `real-dcomp-pixel` verdict |
| --- | --- | --- | --- | --- |
| baseline canonical | 未設定（JSONL `null`） | `plugin-dock-real-scenario.jsonl` | 1 | fail |
| env-on 補助 | `"1"`（JSONL 生値記録） | `plugin-dock-real-scenario-dcomp-disabled.jsonl` | 1 | fail |

env-on run は canonical JSONL を上書きしないよう直接
`cargo run`（補助証拠・canonical = mise）で別 `--out` に記録し、
`--pixel-artifacts evidence/rf631/dcomp-pixel-occlusion-disabled`
で PNG を保存。`real-dcomp-pixel` 以外の 4 case
（`real-window-mount`/`real-native-slot`/`real-editor-attach`/
`real-capture`）は両 run で `pass`。exit 127 は未観測（両 run
とも exit 1 = `Fail` verdict による正常な lane 失敗）。

注: 初回 baseline run は実装 commit 前の未コミット tree
（`repo_sha=cbcc8343`）で実行したが、commit `0fe7bdec` 後に
canonical lane を再実行し JSONL を commit SHA の記録に揃えた
（exp log に初回観測も記録）。

## Occlusion 診断観測（`real-dcomp-pixel`・child HWND 1278×798）

3 つの note-level step row が両 run で記録された:

| Step | baseline | env-on `"1"` |
| --- | --- | --- |
| `occlusion_exstyle` | root `GWL_EXSTYLE`=`0x240100`、`noredirectionbitmap=true`、`appwindow=true`、他 flag false | root `GWL_EXSTYLE`=`0x40100`、`noredirectionbitmap=false`、`appwindow=true`、他 flag false |
| `occlusion_zorder` | parent children = `[editor_child z0]` のみ、root children = `[container z0]` のみ（`truncated=false`） | 同一構造（editor_child z0 / container z0 のみ） |
| `occlusion_hit_test` | child rect 内 5 点（center + 各辺 1/4・3/4 の 4 代表点・矩形内 clamp）全てで `WindowFromPoint` = `Chrome_RenderWidgetHostHWND`（hwnd 987096・rect `0,121,1920x911`・role `other`） | 同一 Chrome HWND が 5 点全てで返る |

review-protocol 指摘（`WindowFromPoint` が hidden/disabled
window とその subtree を skip するため non-child answer は
"covered OR skipped" を意味する）を受け、後続 commit
`2e856bca` で診断を強化して両 run を再実走した（本表は
`repo_sha=2e856bca` の記録）:

- `WindowRow.disabled`（`GWL_STYLE` の `WS_DISABLED`）を全 row
  に追加 — child/parent/root/Chrome 全て `disabled:false`。
- `occlusion_exstyle` は root に加え `child_exstyle`/
  `parent_exstyle` を記録 — child（`RuviformTestEditor`）と
  parent（`Static` container）は共に exstyle `0x0`（layered/
  transparent 等の hit-test 干渉 bit 無し）。
- `occlusion_hit_test` は `child_window`/`parent_window`/
  `root_window` の probe 時点 row を併記 — child/parent は
  `visible:true, alive:true, disabled:false`、root は
  class `Zed::Window`・`visible:true, alive:true,
  disabled:false`。
- hit NULL entry は uniform key（欠損 fact は `null`）。

**cover-vs-skip の判定**: probe 時点で child→container→root の
全 chain が visible/alive/enabled かつ transparent/layered
exstyle 無し — `WindowFromPoint` が skip する条件（hidden・
disabled）に該当しない。したがって 5 点全てで Chrome HWND が
返ることは **skip ではなく genuine な z-order cover** を意味する
— HWND 層の外部遮蔽が観測事実として確定した。

既存 oracle は前 slice と同じ結論: `screen_pixels`
（`win32-bitblt-screen`）green 0.0 / `printwindow_pixels` 0.0 /
`windowdc_pixels` は baseline 0.0・env-on 1.0。lifecycle steps
は全て pass、`session.close`→HWND 破棄→`Closed+concluded`→
events `[Attached{Docked}, Closed]` も pass。

## 観測の解釈（観測事実と仮説を分離・断定しない）

- **env var の実効を初めて直接確認**: baseline `0x240100`
  （`WS_EX_NOREDIRECTIONBITMAP` 立）→ env-on `0x40100`（同 flag
  消滅）。`GPUI_DISABLE_DIRECT_COMPOSITION=1` が GPUI 初期化に
  実際に効いていることを HWND extended style で裏付けた
  （前 slice の未解決問い「env var が効いたか内部確認手段が無い」
  を本 slice の診断が解消）。
- **GPUI 内部 z-order に遮蔽物無し**: root の direct children は
  container（`Static`）1 件のみ、container の children は editor
  child（`RuviformTestEditor`）1 件のみ。editor/container より
  上の GPUI 内部 sibling HWND は観測されなかった。
- **HWND 層の外部遮蔽を観測**: `WindowFromPoint` は child rect 内
  全 probe 点で `Chrome_RenderWidgetHostHWND`（role `other`）を
  返した。`WindowFromPoint` は HWND manager の z-order のみを見る
  ため、これは「観測時点で Chrome window の HWND が GPUI root
  より上にある」ことを意味する。real-window lane の mount 経路に
  `SetForegroundWindow`/activate 呼出は無く（`real_window.rs`・
  `lifecycle_fixture.rs` を source 確認）、テスト window は
  foreground 化されない — 画面に前面化していた Chrome window が
  GPUI window 全体を覆っている状態で観測が行われた。
- **帰結（観測事実のみ）**: screen `BitBlt` は覆っている topmost
  HWND（この環境では Chrome）の画素を返すため、screen green=0.0
  は「GPUI window が外部 HWND に覆われている」だけで説明可能。
  **DComp visual が child HWND の画素を隠すか（compositor 内部
  の遮蔽）は screen oracle では判別不能で `unresolved`**。
  `WindowFromPoint` は DComp visual・flip-model plane を見ない
  ため、z-order/hit-test の「遮蔽物無し」は compositor 内部の
  無遮蔽を意味しない。
- **windowdc の baseline/env-on 差**: `noredirectionbitmap` の
  有無と一致して windowdc green が 0.0→1.0 に変化。DComp 経路
  では window-DC surface が提示経路に乗らない可能性を示唆するが
  機序は断定しない（相関の観測のみ）。

## H1-H5 分類への適用

本症状は compositor 内部仮説（H 系の DComp 機序仮説）ではなく、
**HWND-layer の外部遮蔽が観測された**。二分結果は
「HWND-layer occlusion observed（外部 Chrome window・GPUI 内部
構造は無遮蔽）／ compositor-internal occlusion unresolved」。
exit-127 仮説は非該当（exit 1 = Fail verdict の正常 lane 失敗）。

## Limitations

- 単一ホスト・単一 GPU（RTX 4070 SUPER）・単一 scale 環境での
  観測。`scale_factor` は JSONL で `null`。
- child rect 内 5 点の probe（center + 各辺 1/4・3/4 の 4 代表点、
  矩形内に clamp） — 矩形端・隙間の局所的遮蔽は代表点では
  検出しない可能性がある（今回は 5 点全て同一結果で結論に
  影響なし）。
- `WindowFromPoint` は hidden/disabled window とその subtree を
  skip する — non-child answer は "covered OR skipped" を意味
  し、chain の `visible`/`disabled`/exstyle state を併記して
  判別する設計（review 反映後の row に `disabled`・
  `child_window`/`parent_window`/`root_window`・child/parent
  exstyle を含む）。
- probe 内の `GetWindowRect` は `screen_rect`/`screen_pixels`
  row とは別 query — 観測間に window が移動・resize された場合
  hit-test 点は別矩形を指し得る（実際には sub-second の race、
  今回の run では顕在化せず）。
- top-level 間 z-order は child walk では取れず `WindowFromPoint`
  の結果のみに依存。probe 時点と capture 時点の z-order 変動は
  排除できない（今回は両 run で安定）。
- PNG artifact は env-on run のみ（`--pixel-artifacts` opt-in）。
  canonical baseline run では未指定。

## NotRun / Unsupported / Deferred（本 slice）

- compositor 内部遮蔽の判別（window foreground 化・GPUI root への
  `PrintWindow` 等）は後続 slice — 本 slice は query-only 診断で
  `unresolved` を正直に記録。
- 方式採否・本 fix（GPUI patch / adapter 変更 / env var 恒常化）
  は後続 slice — Issue #532 は open のまま。
- headless host での `NotRun`+`environment_blocked` 経路は unit
  test で検証済みだが、本検証環境は interactive のため実 headless
  run は未実施。
- `mise.toml` L166-169 の stale コメント（「editor-pixel question
  stays deferred」）は scope 外・未修正 — Coordinator 統合時に対応。
- docs index（`docs/*/README.md`、`docs/impl/*/README.md`）の再生成は
  Coordinator が統合時に実施（本 slice では未実施・audit は
  `index-missing-entry` warning のみ）。

---

# Issue #631 real-row 拡充 slice — verification record

`rf631.plugin-dock-real` の real-window catalog を 5 行から 12 行へ拡充
（AC1 残件 + AC3 非画素 editor op 行）し、headless fail-closed 証明 lane
（`--blocked-probe` / `test-ui-plugin-dock-real-headless`）を追加した
slice の検証記録。canonical = `mise` lane。

- Date: 2026-11-16
- OS: Windows (x86_64)・interactive desktop あり
- Lane evidence: `plugin-dock-real-scenario.jsonl`（実走）・
  `plugin-dock-real-headless.jsonl`（blocked-probe）

## Lane results（canonical = mise / cargo）

| Command | Result |
| --- | --- |
| `cargo fmt --check` | pass (exit 0) |
| `cargo clippy -p ruviform-plugin-dock-test --features real-dock --all-targets -- -D warnings` | pass (exit 0) |
| `cargo clippy -p ruviform-plugin-dock-test --all-targets -- -D warnings`（default feature） | pass (exit 0) |
| `cargo test -p ruviform-plugin-dock-test --features real-dock` | pass（unit 3 + console 14 + plugin-dock 4 + doc） |
| `cargo test -p ruviform-plugin-dock-test`（default feature） | pass |
| `mise run test-ui-plugin-dock-real-headless` | pass (exit 0) |
| `mise run test-ui-plugin-dock-scenario` | pass（non-regression） |
| `mise run test-ui-plugin-dock-console`（`--list`/`--run`） | pass（non-regression） |
| `mise run check-ui-driver-boundary` | pass（dep-graph/signature/allowlist） |
| `mise exec -- prek run --all-files` | pass |
| `mise run test-ui-plugin-dock-real` | exit 1 — `real-dcomp-pixel` の既知 Fail のみ（下記） |

## `test-ui-plugin-dock-real` 実走 verdict（12 case）

| case | verdict | 備考 |
| --- | --- | --- |
| real-window-mount | pass | |
| real-native-slot | pass | |
| real-editor-attach | pass | |
| real-capture | pass | |
| real-dcomp-pixel | **fail** | 既知の DComp topmost occlusion（Issue #532）— screen-composited pixels に `EDITOR_COLOR` が出ない正直な regression signal。Pass に変換しない |
| real-multiwindow-focus | pass | A baseline → B → A の `window_focused` 遷移を観測。release は note のみ |
| real-os-input | pass | preflight effective → `plugin.editor` painted bounds へ pointer move（cursor read-back (959,551)）・click・`Tab` が全て `Injected`。transport-level のみ、semantic effect は claim しない |
| real-window-dpi | unsupported | 公開 API に scale-factor/DPI 観測なし |
| real-window-ime | unsupported | adapter が `Ime` を claim しない → preflight で拒否 |
| real-window-close | unsupported | close request/respond API・modifier chord 注入なし |
| real-editor-resize | unsupported | `set_size` → 定義済み `EditorOpError::Unsupported`。session `Attached` 維持・events `[Attached{Docked}, Closed]`・子 HWND 収束を検証 |
| real-editor-redock | unsupported | `reattach` → `Unsupported`。`Failed{Unsupported}` 着地・events `[Attached{Docked}, Detached, Failed{..}, Closed]` を検証 |

lane exit 1 の原因は `real-dcomp-pixel` の `Fail` 1 件のみ — 設計どおりの
正直な regression signal であり、lane defect ではない。

## `test-ui-plugin-dock-real-headless`（`--blocked-probe`）

- `RealOsGuiTest` session を開始せず window を開かない決定的経路。
- 出力 12 case 全て `not_run` + `observed.environment_blocked` +
  `environment_kind=deterministic-backend`。`real-window` claim 無し。
- 自己検査: case id ↔ catalog 行対応・driver=RealOs・environment_kind
  存在かつ非 real-window を含め、違反時 exit 1。
- detail は session start 未試行を明示 — 観測した headless run を装わない。

## Review 反映（review-protocol）

- `editor_op_body`: `OpLanding::Accepted` は `verification.verified`
  必須に修正 — 受理 + 契約検証失敗が `NotRun` に隠れず `Fail` になる。
- `real-os-input`: pointer aim が `Injected` でない場合 mouse down/up を
  発火せず `not_run` 記録（未検証 cursor 位置への stray click 防止）。
  focus settle timeout は `not_run`（未達 precondition）、driver 契約
  error は `fail` に分離。`OsInputOutcome` 未知 variant は `not_run`
  （fold が Pass に昇格させない）。
- `real-multiwindow-focus`: A baseline focus を先に観測し "A→B→A" の
  遷移を実際に証明可能にした。
- `blocked_probe_violations`: case id 順序・driver・environment_kind
  存在の検査を追加。

## NotRun / Unsupported / Deferred（本 slice）

- `real-dcomp-pixel` の `Fail` は Issue #532 の未解決 occlusion —
  本 slice 範囲外、修正しない。
- `real-window-dpi`/`ime`/`close` の `Unsupported` は adapter 公開面に
  API が無いことの正直な判定 — 実装は後続 slice。
- `real-editor-resize`/`redock` は fixture が op を `Unsupported` で
  拒否する現状に依存。受理されても非画素 effect oracle が無ければ
  `NotRun`（検証失敗時は `Fail`）。
- accepted re-dock 経路では再観測した子 HWND のみを detach 検証する
  （初回 attach の HWND は追跡対象から外れる）— accepted path は現状
  到達不能のため既知制限として記録。
- cursor read-back は `commanded` との tolerance 検査を持たず証拠記録
  のみ — driver の `Injected` 意味論に委ねる。

---

# rf532 foreground/compositor mutation probe — verification record（追記）

Issue `scarecrow173/ruviform#532` の foreground probe slice
（branch `feat/532-foreground-probe`、worktree
`rf532-foreground-probe`）のローカル検証記録。canonical = `mise` lane。
Issue #532 は open のまま — 本 slice は診断 probe であり修正ではない。

- Date: 2026-11-16
- OS: Windows (x86_64)、interactive desktop
- Lane JSONL repo_sha 記録値: `513b33450e8cef4b0e41bf1b3250b9c7418c5746`
- Bisect 結論の正本: `evidence/rf532/foreground-probe-bisect.md`
- review-protocol 反映: `IsWindow` liveness guard（foreground
  hand-back）・`topmost_restored` の fail-closed 化・`not_evaluable`
  reason の detail 伝播・doc comment 修正を適用後、fmt/lint/test
  （default + real-dock）と interactive real lane 2 本を最終
  binary で再実走 — 上記 JSONL はその記録。

## Lane results

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `mise run check ruviform-plugin-dock-test` | pass |
| `mise run lint ruviform-plugin-dock-test` | pass（`-D warnings`） |
| `mise run test ruviform-plugin-dock-test` | pass |
| `mise run check ruviform-plugin-dock-test --features real-dock` | pass |
| `mise run lint ruviform-plugin-dock-test --features real-dock` | pass（`-D warnings` — pedantic 含む全 warning 解消） |
| `mise run test ruviform-plugin-dock-test --features real-dock` | pass（unit 3 + console 14 + integration 4） |
| `mise run test-ui-plugin-dock` | pass |
| `mise run test-ui-plugin-dock-console` | pass |
| `mise run test-ui-scenario-contract` | pass（7 tests） |
| `mise run check-ui-driver-boundary` | pass |
| `mise exec -- prek run --all-files` | pass |
| `plugin-dock-real-scenario --blocked-probe` | pass — 14 case 全て `not_run` + `observed.environment_blocked` + `environment_kind=deterministic-backend`、exit 0 |
| `mise run test-ui-plugin-dock-real`（baseline） | exit 1 — `real-dcomp-pixel` の既知 Fail のみ（14 case 中 1 件）。`real-dcomp-foreground` は `pass` |
| `GPUI_DISABLE_DIRECT_COMPOSITION=1 plugin-dock-real-scenario` | exit 1 — 同上（`real-dcomp-pixel` fail、`real-dcomp-foreground` pass） |

## `test-ui-plugin-dock-real` verdict（14 case、両 run 同一構成）

| case | baseline | DComp off | 備考 |
| --- | --- | --- | --- |
| real-window-mount | pass | pass | |
| real-native-slot | pass | pass | |
| real-editor-attach | pass | pass | |
| real-capture | pass | pass | |
| real-dcomp-pixel | **fail** | **fail** | 既知 occlusion（Issue #532）— screen green 不観測を正直に維持。DComp off でも best fraction 0.0 |
| real-dcomp-foreground | **pass** | **pass** | probe 全 stage 実行・restore 検証済み。verdict は実行/復元/lifecycle のみで green 非依存 |
| real-multiwindow-focus | pass | pass | |
| real-os-input | pass | pass | |
| real-window-dpi | unsupported | unsupported | |
| real-window-ime | unsupported | unsupported | |
| real-window-close | unsupported | unsupported | |
| real-editor-resize | unsupported | unsupported | |
| real-editor-redock | unsupported | unsupported | |
| real-session-terminate | pass | pass | |

lane exit 1 の原因は `real-dcomp-pixel` の `Fail` のみ — 設計どおりの
正直な regression signal。probe case の `pass` は occlusion 解消を
意味しない。

## `real-dcomp-foreground` stage 別観測（bisect データ、最終 run）

sample 点の `WindowFromPoint` class 名（screen green は全 stage・
全 run で 0）:

| stage | baseline (DComp on) | DComp disabled |
| --- | --- | --- |
| baseline | `Chrome_RenderWidgetHostHWND` | `Zed::Window` |
| raise `HWND_TOP` | `Chrome_RenderWidgetHostHWND` | `RuviformTestEditor`（自 HWND） |
| foreground | `Chrome_RenderWidgetHostHWND` | `RuviformTestEditor` |
| topmost `HWND_TOPMOST` | `RuviformTestEditor` | `RuviformTestEditor` |
| restore | `Chrome_RenderWidgetHostHWND` | `Zed::Window` |

mutation 適用状況: `SetWindowPos` raise/topmost/restore 全て applied、
`SetForegroundWindow` は baseline run で foreground lock 拒否
（note）、DComp off run では適用（pass）— 環境依存で正直に記録。
restore は両 run で `topmost_restored:true`、DComp off run では
foreground hand-back も適用され `foreground_final` が baseline
owner に一致。z-order は `above_count` 一致・`above_hwnd` は
近似復元の別値（best-effort をそのまま記録）。

## Bisect 結論（要約 — 詳細は rf532 文書）

- 外部 HWND stacking（実行ごとに異なる foreign window — Chrome /
  Zed）は第一の境界として確定。
- hit-test が自 HWND に解決した stage（baseline run では topmost、
  DComp off run では raise 以降）でも screen green は 0 — HWND
  z-order では説明できない compositor/presentation 内部の第二の
  境界が存在。`GPUI_DISABLE_DIRECT_COMPOSITION=1` でも解消しない。
- `WindowFromPoint` は DComp visual/plane・hit-test-transparent
  surface を観測できないため、残余機構の断定はしない。

## NotRun / Unsupported / Deferred（本 slice）

- 残余 occlusion の機構特定（hit-test-transparent surface か、child
  presentation 経路か、capture 盲点か）は未解決 — 次 slice の診断
  候補は `foreground-probe-bisect.md` に記載。
- `SetForegroundWindow` の foreground-lock 拒否は OS 仕様どおりの
  note データ — foreground branch は未解決のまま。
- z-order/foreground の restore は best-effort（topmost band のみ
  検証済み保証）— Windows が保証しない完全復元は claim しない。

---

# Slice 追記: Issue #532 presentation-probe（`real-dcomp-present`）

- Date: 2026-11-16
- Lane JSONL repo_sha: `81308574`（実装 HEAD）
- 対象: `ospres` module + `ospix` PrintWindow flag variants +
  `real-dcomp-present` case（catalog 14→15 real rows、計 23 rows）

## Static lanes（全 pass）

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `mise run check/lint/test ruviform-plugin-dock-test`（default） | pass（18 tests） |
| `mise run check/lint/test ruviform-plugin-dock-test --features real-dock` | pass（real-dock 3 + console 14 + plugin-dock 4） |
| `mise run test-ui-plugin-dock` / `-console` | pass |
| `mise run test-ui-scenario-contract`（fake/gpui 両 lane） | pass |
| `mise run check-ui-driver-boundary` | pass |
| `mise exec -- prek run --all-files` | pass |
| `plugin-dock-real-scenario --blocked-probe` | pass（15 case 全て `not_run` fail-closed、`plugin-dock-real-headless.jsonl` 更新） |

## Interactive real lanes（exit 1 = `real-dcomp-pixel` Fail による設計どおり）

`mise run test-ui-plugin-dock-real`（baseline）と
`GPUI_DISABLE_DIRECT_COMPOSITION=1` env-on（`--pixel-artifacts
evidence/rf631/presentation-probe` 付き）の 2 run を実走。

| case | baseline | DComp disabled |
| --- | --- | --- |
| real-dcomp-pixel | fail | fail（verdict logic 不変、screen green=0 維持） |
| real-dcomp-foreground | pass | pass |
| **real-dcomp-present** | **pass** | **pass**（34 steps・全観測 family 完走） |
| 他 12 case | pass/unsupported（前 slice と同一分布） | 同左 |

### `real-dcomp-present` 観測ハイライト（evidence data、verdict 非入力）

- `pres_inventory`: top-level 449 windows（cap 256 truncated）、
  child rect 交差の visible foreign cover candidate 複数
  （`Chrome_WidgetWin_1` z=22 等）。own-process window は cap に
  より 0 件記録 — limitation として JSONL/本文書に明記。
- `pres_attributes`: cloaked=0・region/layered query は error/null
  を正直記録 — DWM cloak/region/layered の兆候なし。
- capture variants: child `PrintWindow` flags=0/`PW_CLIENTONLY` は
  green=1.0、`PW_RENDERFULLCONTENT` 系は 0.0（flat dark raster）。
  window-DC は DComp off で child/container/root green 優位、
  DComp on では child/root が 0.0。screen BitBlt は両 mode で 0.0。
- `pres_fingerprint`: screen raster は chain/suspect 全てと
  match_fraction=0.0（top color `(248,248,248)` light-gray 優位）。
- `real-dcomp-foreground` stage data: baseline `above_count=399`・
  hits=`Chrome_RenderWidgetHostHWND`、topmost で hits=自 child に
  切替・`above_count` 7、screen green は全 stage 0.0。
- pixel artifacts: topmost stage の screen PNG は全面 flat dark
  （新規 surface — stale frame ではない）を示し、baseline 系は
  Chrome の light-gray 静的 histogram と一致。

判別結果の正本: `evidence/rf532/presentation-probe.md`。
Issue #532 は open のまま — compositor 修正・方式選定は未実施。

## Review-round re-verification（post-review fixes、2026-11-16）

review-protocol の 2 reviewer 監査で確定した修正適用後の再検証。
対象: `ospres.rs`（pid/owner_pid の tid チェック・UpdateFacts.error・
SAFETY comment）、`ospix.rs`（SelectObject 失敗の error 化・
`window_hung` 追加・module doc 修正）、`real_dock.rs`
（`bounds_space`/`rect_space` 座標系明示・hung suspect skip・
`ChildProbeResult::NotEvaluable`・families+cover・`suspect_role`・
純粋 helper 5 unit test 追加）。

- `cargo fmt --all -- --check`: pass
- `mise run check/lint/test ruviform-plugin-dock-test`（default）: pass
- 同 lanes `--features real-dock`: pass（unit 8・console 14・
  plugin_dock 4 全 pass）
- `mise run test-ui-plugin-dock`: pass
- `mise run test-ui-plugin-dock-console`: pass
- `mise run test-ui-scenario-contract`: pass
- `mise run check-ui-driver-boundary`: pass
- `mise exec -- prek run --all-files`: pass
- `plugin-dock-real-scenario --blocked-probe`: pass（15 case
  全て `not_run` fail-closed、`plugin-dock-real-headless.jsonl`
  再生成）
- `mise run test-ui-plugin-dock-real`（baseline・最終コードで再実行）:
  exit 1（`real-dcomp-pixel` の設計どおり Fail）。`real-dcomp-present`
  = **pass**、families `["inventory","attributes","cover",
  "capture_variants","screen","fingerprint"]`、step row に
  `bounds_space`/`rect_space`/`update.error` を確認。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` run（`--pixel-artifacts`
  付き・最終コードで再実行）: exit 1 同上。`real-dcomp-present` =
  **pass**、screen green=0.0、root `noredirectionbitmap=false`
  （env 実効を HWND exstyle で再確認）、PNG artifact 22 枚再生成。
- verdict 分布は review 前 run と同一 — 修正は観測 schema の
  正直化と hang 耐性のみで、verdict 経路・既存 case の判定は不変。

---

# Slice 追記: Issue #631 real-window catalog flip（close/dpi 実判定化）

- Date: 2026-11-17
- Lane JSONL repo_sha: `f42e6c71`（実装 HEAD・未コミット差分を含む working tree で実走）
- 対象: `real-window-close`/`real-window-dpi` の Unsupported stub →
  window-surface slice の観測 API（`window_events`/`window_geometry`/
  `request_close`/`arm_close_response`/`respond_close`）による実判定化。
  catalog 21 real rows の verdict 分布を flip。jsonl.rs に AC4 stretch
  として `receipts`・決定的 `case_seed` を追加し、real lane の
  `environment.scale_factor` を実測値へ配線。

## Static lanes（全 pass）

| Command | Result |
| --- | --- |
| `mise run fmt-check` | pass |
| `mise run check ruviform-plugin-dock-test`（default） | pass |
| `mise run check ruviform-plugin-dock-test --features real-dock` | pass |
| `mise run check ruviform-plugin-dock-test --features lab-gui` | pass |
| `mise run lint ruviform-plugin-dock-test`（default） | pass（`-D warnings`） |
| `mise run lint ruviform-plugin-dock-test --features real-dock` | pass（`-D warnings`） |
| `mise run test ruviform-plugin-dock-test`（default） | pass（console 14 + plugin-dock 4 + jsonl unit 2） |
| `mise run test ruviform-plugin-dock-test --features real-dock` | pass（unit 10 + console 14 + plugin-dock 4） |
| `mise run test-ui-plugin-dock-console` | pass |
| `mise run test-ui-plugin-dock`（headless scenario） | pass |
| `mise run test-ui-plugin-dock-lab`（interactive GPUI lab） | pass（pass×7・unsupported×1・not_run×1 — 既存分布と同一） |
| `mise run test-ui-scenario-contract` | pass（gpui lane 7 tests） |
| `mise run check-ui-driver-boundary` | pass（dep-graph/signature/allowlist） |

## `test-ui-plugin-dock-real` 実走 verdict（21 case、interactive Windows desktop）

| case | verdict | 備考 |
| --- | --- | --- |
| real-window-mount | pass | |
| real-native-slot | pass | |
| real-editor-attach | pass | |
| real-capture | pass | |
| real-dcomp-pixel | **fail** | 既知の DComp topmost occlusion（Issue #532）— 前 slice と同一の正直な Fail を維持。Pass に変換しない |
| real-dcomp-foreground | pass | |
| real-dcomp-present | pass | |
| real-multiwindow-focus | pass | |
| real-os-input | pass | |
| **real-window-dpi** | **pass**（flip） | `window_geometry` live 観測（origin/物理 px/`scale_factor=1.0`）+ `window_events` の `Created`/`Ready`/`Resized` 最終遷移との一致 + 破棄後 dead-window geometry query の fail を検証。`environment.scale_factor` は実測値。DPI **変更**駆動（mixed-DPI move）は変更 API 不在のため `dpi_change_driven:false` を正直に記録 |
| real-window-ime | unsupported | `Ime` capability 未 claim（#389 領域）— 変更なし |
| **real-window-close** | **pass**（flip） | 3 window 構成: B が armed Veto → armed Defer → `respond_close(Approve)` → `Destroyed` + 退役後 probe、C が未応答 default Approve → `Destroyed`、sentinel A は close lifecycle なしで生存。20 step 全 pass |
| real-editor-resize | unsupported | 変更なし（#389 verb 領域） |
| real-editor-redock | unsupported | 変更なし |
| real-session-terminate | pass | |
| real-session-degraded | pass | |
| real-session-recovery | pass | |
| real-session-timeout | pass | |
| real-editor-visibility | unsupported | 変更なし |
| real-session-crash | unsupported | 変更なし（out-of-process 化は別 slice） |
| real-session-hang | unsupported | 変更なし |

lane exit 1 の原因は `real-dcomp-pixel` の `Fail` 1 件のみ — Issue #532 の
既知 occlusion であり lane defect ではない（baseline jsonl でも同 verdict）。

## `test-ui-plugin-dock-real-headless`（`--blocked-probe`）

- `plugin-dock-real-headless.jsonl` 再生成: 21 case 全て `not_run` +
  `environment_kind=deterministic-backend`。flip は headless 形状を
  変えない — close/dpi 行も blocked-probe detail を維持。

## JSONL evidence field 追加（AC4 stretch・lockfile 変更なし）

- 全 case 行に `case_seed`（run_id+case からの決定的 16-hex digest、
  case 間で異なることを unit test で固定）と `receipts`
  （`CommandReceipt` 直列化、現在の lane では空配列 — receipt を
  発行する case がまだ無いため正直な空）を追加。既存 field の
  rename/remove なし。`targets` は従来どおり除外。
- real lane の `environment.scale_factor` が固定 `null` から
  `finalize_real` extra 経由の実測値（本 run: `1.0`）へ配線。
- `real-window-dpi` の observed に `window_geometry_sha256`
  （HWND 相当の window id + origin + 物理 geometry + scale factor の
  canonical 文字列 digest）を追加。

## NotRun / Unsupported / Deferred（本 slice）

- `real-window-ime`・`real-editor-resize`/`redock`/`visibility`・
  `real-session-crash`/`hang` の `Unsupported` は据え置き —
  capability claim・verb・out-of-process 化は #389 / 別 slice 領域。
- `real-dcomp-pixel` の `Fail` は Issue #532 未解決のまま正直に保持。
- AC4 の未実装 field（GPU 情報・semantic tree 直列化・pixel diff
  evidence 同梱・window/plugin hash の完全 digest 化）は残件 —
  lockfile/manifest 変更を要する手段を使わない制約内で実施した分のみ。
