---
id: "SPEC-034XEa3KZlRiONK0C4tHgr"
type: "spec"
status: "approved"
title: "Issue #631 Plugin-dock scenario skeleton（test-only crate ruviform-plugin-dock-test・正直な verdict・plugin artifact evidence）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "POLICIES.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/designs/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/test-specs/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/plans/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/tasks/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "docs/impl/ir/issue-631-plugin-dock-scenario.md"
      - type: "doc"
        path: "evidence/rf631/verification.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "crates/ruviform-plugin-test-vst3/README.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Plugin-dock scenario skeleton（test-only crate ruviform-plugin-dock-test・正直な verdict・plugin artifact evidence）

## What Should Be Built

Issue #631（tracker）の first slice: Ruviform GPUI 実 Window Scenario Runner 構想のうち、**Plugin Editor Native Dock を独立 Test Binary から構成する harness の骨格**を test-only crate `ruviform-plugin-dock-test` として新設する。

本 slice が提供するもの:

- `test_gain` VST3 cdylib（`ruviform-plugin-test-vst3`、Issue #630 first slice の実 ABI fixture）の解決 — env var 明示指定と `current_exe` → `deps/` 自動解決の両経路。
- 解決した artifact の **SHA-256 digest + plugin version** を case evidence に記録する経路。
- 本番 host 経路（`Vst3Library::load` → `descriptors()`/`factories()` → `InProcessHost` の `Instantiate`/`Initialize`/`Activate` → `PluginEditorSessions` + `InProcessEditorOps` → `NativeSurfaceSlot` lease）を通した plugin-dock scenario の実行と、`ScenarioReport` 語彙での verdict 記録。
- `ruviform-ui-test` の driver-neutral scenario contract 上の plugin-dock scenario spec（`NativeSlot` を含む `ViewSpec`、`required = {NativePluginDock, VisualCapture}`）と、plugin artifact descriptor を `ScenarioReport.environment`/evidence へ記録する共有経路。
- `ruviform-audio-test` の `EnvironmentKind` への real 系 variant 追加（実 ABI バイナリ case のみが名乗れる語彙）。
- manual/opt-in の `mise run test-ui-plugin-dock` lane と `evidence/rf631/` の実走証拠。

本 slice が**提供しない**もの（後続 slice / Issue 残 scope）:

- `test_editor` plugin class（`createView` が実 HWND を返す実物）— Issue #630 残存 scope。
- Native Editor の実 Dock Pass、実 child HWND の pixel/input/resize/focus/re-dock 検証、既知 DComp Fail（`scarecrow173/ruviform#532`）の再現・解消判定。
- Test Lab Console / Scenario Console GUI、Synvelis/AudioLens 製品 oracle 照合、3OS 対応、`*-msrv` lane。

## Why Now

Issue #631 は Ruviform 自身の GUI test dogfooding — Synvelis / AudioLens が使う予定の driver-neutral scenario contract を Ruviform 自身の GPUI runner/lab で先に踏む tracker である。plugin↔ui の依存禁止により composition は consumer 層にしか置けないため、「plugin artifact evidence を持つ正直な verdict の scenario skeleton」を独立 test-only crate として先に固めることが、後続の実 Window/実 editor slice の土台になる。

現行 main では `test_gain` は editor を持たない（`createView` null → `HostError::Unsupported`）。この事実を fail-closed に記録できる harness が先に必要であり、editor 実物の到着を待ってから verdict 機構を作ると「Fake/semantic 成功を Dock Pass と偽る」リスクが残る。

## Users and Value

- Ruviform 開発者 / 後続 slice の実装者: plugin artifact の digest/version を含む再現可能な scenario evidence と、Pass/Fail/NotRun/Unsupported の正直な verdict 語彙を得る。
- Synvelis / AudioLens の test 基盤設計者: driver-neutral contract が product 固有 semantics を consumer に残したまま plugin-dock scenario を表現できることの実証。

## Scope

### In Scope

- 新規 test-only crate `crates/ruviform-plugin-dock-test`（lib + scenario runner + evidence 出力 + README）。
- `crates/ruviform-audio-test/src/metadata.rs` の `EnvironmentKind` への additive variant 追加（`RealAbiTestPlugin` / `RealWindow` 系）と `lib.rs` の export。
- `crates/ruviform-ui-test/src/` への plugin-dock scenario spec（`NativeSlot` 入り `ViewSpec`、required capabilities、artifact evidence の environment/evidence 記録経路）と `ScenarioDriver` への additive variant 追加。
- ルート `Cargo.toml` `members` への 1 行追加、`Cargo.lock` 再生成。
- `mise.toml` への additive task `[tasks.test-ui-plugin-dock]`（manual/opt-in、CI 必須化しない）。
- `evidence/rf631/` への検証記録。
- 本 slice の spec/design/plan/task/test-spec/impl 文書チェーン。

### Out of Scope

- `test_editor` 実装、実 Native Editor Dock Pass、実 Window/実 OS 入力/画素 capture の実施。
- `crates/ruviform-plugin-test-vst3`・`ruviform-plugin-host`・`ruviform-plugin-core`・`ruviform-plugin-vst3`・`ruviform-ui`・`ruviform-ui-gpui`・`ruviform-poc-native-dock` の内部変更（参照・依存 edge のみ）。
- `crates/ruviform-audio-test`・`crates/ruviform-ui-test` の `Cargo.toml` 変更（新規依存が必要になった場合は別途調整）。
- `crates/README.md` 索引行（Coordinator が統合時に追加）。
- CI workflow / `docs/development-environment.md` の lane 台帳更新。
- Issue #631 の auto-close（tracker — `Refs #631` のみ）。

## Requirements

- test_gain cdylib を env var 明示指定と自動解決の両経路で解決し、**バイナリ digest（SHA-256）と plugin version を case evidence に記録**する。artifact が解決できない場合は fail-closed に `NotRun` とし、silent skip しない。
- `Vst3Library::load` → `descriptors()`/`factories()` → `InProcessHost` instantiate/initialize/activate の各 step を実走し、成功したものだけ `Pass` とする。
- `PluginEditorSessions`/`InProcessEditorOps` + `NativeSurfaceSlot` lease で editor attach を試行し、現行 main の `createView` null / `HostError::Unsupported` を `unsupported`/`not_run` verdict として detail 付きで記録する。**editor attach/dock/pixel を `Pass` にしない。**
- `ScenarioReport`/journal に RunId/CaseId/driver/capabilities/required/verdict/environment（OS・scale・SHA・plugin digest）を出力する。
- `environment_kind` の real 系 variant は実 ABI バイナリ case を実際に実行した記録だけが名乗る（POLICIES.md §2.1「実 Plugin ABI と Mock を分離」）。
- 新クレートは test-only であり、production crate は本クレートに依存しない。公開 enum は `#[non_exhaustive]`、`unsafe` は COM/FFI 境界の局所許可のみ。
- `mise run test-ui-plugin-dock` は manual/opt-in lane で、OS-L1 CI 非回帰・PR 必須条件にしない。

## Acceptance Criteria

- [ ] `mise run build ruviform-plugin-test-vst3` の cdylib を env var + 自動解決の両経路で解決し、digest + version が case evidence に記録される。
- [ ] load/discover/hash の各 step が `pass`、editor attach/dock/pixel checks が `unsupported`/`not_run`（detail に `createView` null / `HostError::Unsupported` 相当を明記）として `ScenarioReport`/journal に出力される。
- [ ] `ScenarioReport`/journal が RunId/CaseId/driver/capabilities/required/verdict/environment（OS・scale・SHA・plugin digest）を含む。
- [ ] `ruviform-ui-test` の plugin-dock scenario spec が `NativeSlot` 入り `ViewSpec` と `required={NativePluginDock, VisualCapture}` を提供し、artifact descriptor が environment/evidence に記録される。
- [ ] `EnvironmentKind` に real 系 variant が additive に追加され、公開 enum policy（`#[non_exhaustive]`）を維持する。
- [ ] `mise run check/lint/test ruviform-plugin-dock-test`・`ruviform-audio-test`・`ruviform-ui-test`・`ruviform-plugin-test-vst3`、`test-ui-scenario-contract*`・`test-ui-native-slot`・`test-ui-golden`・`test-ui-driver-contract`・`test-ui-gpui`・`check-ui-driver-boundary`・`fmt-check`・`prek` が実走して成功する。
- [ ] `mise run test-ui-plugin-dock` が実走し、結果が `evidence/rf631/` に保存される。

## Deferred Design Concerns

- `test_editor` plugin class（実 HWND を返す `createView`）| Issue #630 残存 scope であり本 slice の fixture には含めない | editor 実物の実装着手時 | Dock Pass 判定を fixture なしで主張する圧力が生じる
- 実 Window での dock/pixel 検証と `RealWindow` environment kind の実走利用 | GUI-L2 lane（`test-ui-real-scenario` 系）との統合は別 slice | real-window driver が `NativePluginDock`/`VisualCapture` を提供する実装時 | headless 結果を GUI-L2 Pass と誤読する
- `ScenarioDriver` への real-window 系 variant・IMe/2Window 等の capability 実装 | driver-neutral contract は既に語彙を持ち、adapter 側 slice の責務 | 各 driver slice の着手時 | capability vocabulary の先行拡張が実装なき約束になる

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — 本 spec はその first slice のみを対象）。
- `scarecrow173/ruviform#627` Case Schema v1 / `environment_kind` 語彙 / POLICIES.md 2026-11-15 仮想優先改訂（未実走 case を Pass にしない）。
- `scarecrow173/ruviform#630` first slice: `test_gain` cdylib fixture と `tests/host_path.rs` の env var + `current_exe`→`deps/` 解決先例。
- `docs/specs/issue-630-test-plugin-suite.md`・`docs/designs/issue-630-test-plugin-suite.md`（fixture 側の設計記録）。
- `crates/ruviform-poc-native-dock`（plugin↔ui 依存禁止下での consumer-layer composition 先例）。
