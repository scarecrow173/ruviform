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
