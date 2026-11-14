---
id: "DESIGN-034XEb9x9Tty07692Bu8cc"
type: "design"
status: "approved"
title: "Issue #631 Plugin-dock scenario skeleton 設計"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "docs/specs/issue-631-plugin-dock-scenario.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/plans/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/tasks/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/test-specs/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/impl/ir/issue-631-plugin-dock-scenario.md"
      - type: "dir"
        path: "crates/ruviform-plugin-dock-test"
      - type: "file"
        path: "evidence/rf631/verification.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "crates/ruviform-audio-test/src/metadata.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/plugin_dock.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/specs/issue-631-plugin-dock-scenario.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
  blocks:
    - "docs/plans/issue-631-plugin-dock-scenario.md"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-631-plugin-dock-scenario.md"
    - "crates/ruviform-plugin-test-vst3/README.md"
    - "crates/ruviform-poc-native-dock/Cargo.toml"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-plugin-dock-scenario.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Plugin-dock scenario skeleton 設計

## Context

Issue #631 は Ruviform 自身の GPUI test runner/lab を driver-neutral scenario contract の上に構築する tracker であり、本 slice はその **Plugin-dock scenario skeleton** — test-only crate `ruviform-plugin-dock-test`・正直な verdict・plugin artifact evidence — を納品する。

境界条件:

- plugin↔ui の相互依存は禁止（architecture baseline）。両側を見られるのは consumer 層のみ — `ruviform-poc-native-dock` が先例。
- `ruviform-ui-test` は driver-neutral scenario contract（`SemanticGuiTest`/`ScenarioSpec`/`run_scenario`/`ScenarioReport`）を所有し、GPUI 型を漏らさない。
- `ruviform-plugin-host` は `InProcessHost` + `PluginEditorSessions` + `InProcessEditorOps`（`LeaseRedeemer` seam 経由で `ParentSlotLease` → `ParentLease`）を提供する。
- `ruviform-plugin-vst3` の `host` feature は `Vst3Library::load`/`descriptors()`/`factories()` の本番 in-process bridge。
- `test_gain`（`ruviform-plugin-test-vst3`）は editor を持たない — `createView` null → `HostError::Unsupported`。現行 main で Dock Pass は**存在しない**。

## Scope

### In Scope

- `crates/ruviform-plugin-dock-test` 新設（test-only composition root）。
- `ruviform-audio-test`: `EnvironmentKind` に `RealAbiTestPlugin`/`RealWindow` variant と `as_str()` を追加。
- `ruviform-ui-test`: `ScenarioDriver::PluginHostProbe` variant、`scenario/plugin_dock.rs`（`plugin_dock_view_spec`・`plugin_dock_spec`・`PluginArtifactEvidence`）。
- `mise.toml` に `[tasks.test-ui-plugin-dock]` 追加（manual/opt-in）。
- `evidence/rf631/` 証拠。

### Out of Scope

- 実 Window/実 editor/pixel capture の実施、`test_editor` fixture、DComp Fail 再現。
- 上流 crate（plugin-test-vst3/plugin-host/plugin-core/plugin-vst3/ui/ui-gpui/poc-native-dock）の内部変更。
- audio-test/ui-test の `Cargo.toml` 変更。

## Components and Boundaries

- **`ruviform-plugin-dock-test`（新規・consumer 層）**: artifact 解決（env var `RVF_PLUGIN_DOCK_VST3_PATH` → `current_exe`/`deps/` 自動解決）→ SHA-256 digest → `Vst3Library::load` → `InProcessHost` 登録・instantiate/initialize/activate → `PluginEditorSessions` + `FakeNativeSlot` lease + `InProcessEditorOps` で `session.open(Docked)` を試行 → 各 step を `ScenarioReport` に記録。唯一 plugin 側と ui 側の両契約を消費できる層。
- **`ruviform-ui-test::scenario::plugin_dock`（追加）**: driver-neutral な scenario 宣言 — `NativeSlot` 入り `ViewSpec`、`required={NativePluginDock, VisualCapture}` の `ScenarioSpec` 組立、`PluginArtifactEvidence`（path/sha256/plugin descriptor）の environment/evidence JSON 化。GPUI 型・plugin 型を含まない純データのみ。
- **`ruviform-ui-test::scenario::ScenarioDriver`（追加 variant）**: `PluginHostProbe` — in-process plugin host 経路 + fake slot の証拠級を名指す。Fake ではなく（実 ABI binary を load する）、RealOs でもない（OS window/input/pixel なし）。
- **`ruviform-audio-test::metadata::EnvironmentKind`（追加 variant）**: `RealAbiTestPlugin`（実 ABI バイナリを実走した記録のみ）/ `RealWindow`（GUI-L2 real window lane 用に予約）。`as_str()` で Case Schema v1 の kebab-case 語彙を提供（serde 非依存 — audio-test の Cargo.toml はリース外）。

## Data and Control Flow

1. `resolve_artifact()` — env var → `current_exe().parent()` (deps/) の `{DLL_PREFIX}ruviform_plugin_test_vst3{DLL_SUFFIX}`。不在は `NotRun`（fail-closed、silent skip なし）。
2. `artifact` case — ファイル読み → SHA-256 → `Vst3Library::load` → `descriptors()` で `test_gain` 同定（id/name/vendor/version）→ `PluginArtifactEvidence` 確定。成功 step のみ `Pass`。
3. `view-mount` case — `FakeGuiTest` + `run_scenario` で `plugin_dock_view_spec()` を mount し `native_slot` component を observe。semantic evidence のみ（driver=Fake）。
4. `host-lifecycle` case — `InProcessHost` に `factories()` を登録 → `Scan` → `Instantiate` → `Initialize` → `Activate(ProcessConfig 48k/1..512)`。
5. `editor-attach` case — `PluginEditorSessions::begin_session` → `FakeNativeSlot::declare_slot`/`lease_parent` → `ParentSlotLease` → redeemer（`validate_lease` → `ParentLease`）→ `session.open(Docked)`。現行は `createView` null → `HostError::Unsupported` → `EditorOpError::Unsupported` → session `Failed{Unsupported}`。verdict = `Unsupported`（detail に原因明記）。
6. `dock-pixels` case — attach が成立しないため pixel/visual 検証は未到達 → `NotRun`。
7. teardown — session `close`（Failed から `Closed` へ）→ `end_session` → slot `release_lease`/`release_slot` → host `Destroy`。cleanup 結果を report.cleanup に記録。
8. 出力 — 各 `ScenarioReport` を JSONL（run ヘッダ + case 行）にシリアライズし、bin が `--out` パスまたは stdout へ書く。`mise run test-ui-plugin-dock` が `evidence/rf631/plugin-dock-scenario.jsonl` へ保存。

## Non-Functional Constraints

- test-only: 本番クレートは本クレートに依存しない（DAG で強制）。`publish = false` 継承。
- verdict 正直性: `Pass` は実走して一致した step のみ。`Unsupported`/`NotRun` を Pass に昇格しない。Fake slot の成功は contract conformance であり Dock PASS ではない。
- 公開 enum は `#[non_exhaustive]`。`unsafe` は本クレートでは不要（dlopen は `Vst3Library` 内に閉じる）。
- OS-L1 CI 非回帰: 新 lane は manual/opt-in。`cargo check --workspace` 系が本クレートを headless で compile できること（GPUI 不要）。
- 依存は既存 lock tree のもののみ（`sha2 0.10` は lock 内既存 version に解決）。

## Risks and Trade-offs

- `ScenarioDriver::PluginHostProbe` 追加は ui-test の公開 enum 拡張 — `#[non_exhaustive]` で additive、serde kebab-case で後方互換。driver-boundary guard は GPUI 型漏洩のみ検査するため影響なし（要 `check-ui-driver-boundary` で実証）。
- `ScenarioReport` は `Serialize` を持たない（`CommandReceipt` 等が非 Serialize）→ JSONL は本クレート側で public field から `serde_json::Value` を組み立てる。report を authority に保ち、シリアライズは出力層の責務。
- artifact 未解決時の cascade: load 不能なら downstream case は全て `NotRun`（実行不能を偽装しない）。
- `git rev-parse HEAD` で repo SHA を取得 — test harness として許容（失敗時は `"unknown"` を記録し捏造しない）。

## Deferred Design Concerns

- 実 Window driver（`ScenarioDriver::RealOs` 系 or 新 variant）での `NativePluginDock`/`VisualCapture` 実装 — 別 slice。`RealWindow` environment kind の実走利用も同時。
- `test_editor` fixture による Dock Pass — Issue #630 残存 scope。
- ScenarioReport の `Serialize` derive — `CommandReceipt` が `ruviform-ui` 側で非 Serialize のため、共通化は上流変更が必要（本 slice 範囲外）。

## Verification Notes

- [ ] `mise run check/lint/test ruviform-plugin-dock-test` + 影響先（audio-test/ui-test/plugin-test-vst3）
- [ ] `mise run test-ui-scenario-contract{,-fake,-gpui}`・`test-ui-native-slot`・`test-ui-golden`・`test-ui-driver-contract`・`test-ui-gpui`・`check-ui-driver-boundary` 非回帰
- [ ] `mise run test-ui-plugin-dock` 実走 → `evidence/rf631/` に verdict JSONL + verification.md
- [ ] `mise run fmt-check`・`prek run --all-files`
