# rf631 plugin-dock scenario — verification record

Issue `scarecrow173/ruviform#631` first slice（plugin-dock scenario
skeleton）のローカル検証記録。canonical = `mise` lane。

- Date: 2026-11-15
- OS: Windows (x86_64)
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1
- Repo SHA at run: `d512334ad533c924e8e35c0c02a7f5b2ce13a676`
  （`origin/main` 取り込み前の branch HEAD — merge 後に再採取）

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
