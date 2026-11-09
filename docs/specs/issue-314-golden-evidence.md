---
id: "SPEC-2v1KGNSaMXtSGSr7czOhiR"
type: "spec"
status: "proposed"
title: "Issue #314 slice 5 — Golden 環境正規化・fixture registry 結合・Scenario runner 統合・証拠 lane"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/ruviform/issues/464"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/426"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-314-golden-evidence.md"
      - type: "file"
        path: "docs/designs/issue-314-golden-evidence.md"
      - type: "file"
        path: "docs/plans/issue-314-golden-evidence.md"
      - type: "file"
        path: "docs/tasks/issue-314-golden-evidence.md"
      - type: "file"
        path: "docs/test-specs/issue-314-golden-evidence.md"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/normalize.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/report.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/visual.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-314-golden-core.md"
    - "docs/designs/issue-314-golden-core.md"
    - "docs/specs/issue-314-golden-scenario.md"
    - "docs/designs/issue-314-golden-scenario.md"
    - "docs/specs/issue-314-golden-capture.md"
    - "docs/designs/issue-314-golden-capture.md"
    - "docs/specs/issue-464-scenario-runner.md"
    - "docs/specs/issue-390-driver-contract-lab.md"
    - "docs/specs/issue-426-gpui-lab-binding.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-314-golden-scenario.md"
    - "docs/designs/issue-314-golden-core.md"
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-314-golden-scenario.md"
    - "docs/specs/issue-464-scenario-runner.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-314-golden-evidence.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 5 — Golden 環境正規化・fixture registry 結合・Scenario runner 統合・証拠 lane

（slice 番号について: slice 4 は `issue-314-golden-capture` spec が
consumer story fixture 接続（#426 系・consumer 側）として予約済み。
本 slice は utility 側の残件であり、番号衝突を避けて slice 5 とする。）

## What Should Be Built

slice 1〜3（`issue-314-golden-core` / `-scenario` / `-capture`）で
main 入りした Golden Utility の残りの utility 側責務を、
`ruviform-ui-test` に additive に追加する:

- **環境正規化**: `EnvironmentMeta`（capture 時の生の環境証拠）から
  `GoldenKey` の env 側 class（`driver_class`/`os_class`/
  `gpu_class`/`scale_factor`/`font_class`）を導出する正規化規則を
  1 箇所に集約する。現状は各 consumer（`ruviform-ui-gpui` の
  `golden_capture.rs`/`lab_story.rs` 等）が `GoldenKey` を手組み
  しており、正規化規則が consumer ごとに分散・未定義である。
- **fixture registry 結合**: `GoldenKey` の `story_id`/`theme` を
  `lab_story::StoryFixture`（code registry、#426）から構築する
  経路と、`stories.json` manifest（JSON registry、#390）に対する
  `story_id`/`theme`/`state`/`scale_factor` の整合検査を提供する。
  `key.rs` が明示的に先送りした「registry への結合・実在検査」
  をここで閉じる。
- **Scenario runner 統合**: PR #511 で main 入りした
  `run_scenario`/`ScenarioSpec`/`ScenarioOutcome`/`settle_until`
  を消費し、`VisualGuiTest::golden_check` の結果を
  `ScenarioOutcome` へ畳み込む `record_golden_outcome` を追加する。
  別の汎用 Runner は作らない — `run_scenario` の body 内で使う
  step helper に留める。
- **証拠 lane 分離**: `GoldenEvidence` に `lane = "pixel-golden"`
  を記録し、semantic oracle / draw plan / 実 pixel golden /
  OS input の各証拠が同一 Scenario ID で混同されない構造を
  証拠 artifact 側に固定する。#262 採否判定（category C:
  実 pixel/数値/性能）と #313 OS Gate（screenshot 証拠の
  統合先）への証拠リンク構造を文書化する。

## Why Now

- slice 1〜3 の merge により compare/approval/evidence/capture
  の型は揃ったが、`GoldenKey`/`EnvironmentMeta` を結ぶ
  **正規化規則が存在しない** — consumer が手組みするため、
  scale factor の妥当性検査・GPU class の導出・font class の
  扱いが consumer ごとにばらつく余地がある。
- `key.rs` の doc が「registry への結合・実在検査は本 slice の
  対象外」と明記して先送りしており、その受け皿がない。
- slice 2 の `VisualGuiTest`/`golden_check`/`record_golden_step`
  は `ScenarioReport`（run 後）向けであり、PR #511 の
  `run_scenario`/`ScenarioOutcome`（run 中の body 内）への
  対応 helper がない — 新 runner を消費する正規の接続点が
  未整備。
- evidence JSONL は pixel golden の結果のみを記述するが、
  lane を機械可読に持たない — #262/#313 の gate が証拠を
  lane で選別する構造が artifact 上に固定されていない。

## Users and Value

- golden を consumer（AudioLens/Synvelis 側の後続 slice、
  `ruviform-ui-gpui` の capture test）へ展開する利用者:
  `EnvironmentMeta` → `GoldenKey` の変換を 1 つの正規化関数に
  寄せ、key の env 側が環境証拠と常に整合する。
- #390 driver-lab / #426 story fixture の保守者: golden key の
  `story_id`/`theme` が fixture/manifest に実在することを
  機械的に検査でき、rename/削除時の stale key を検出できる。
- #262 採否判定・#313 OS Gate の証拠収集者: `lane="pixel-golden"`
  と `environment`/`subject`（canonical key）で pixel lane の
  証拠だけを選別でき、semantic/plan/OS-input の pass を
  golden pass と誤読しない。

## Scope

### In Scope

- `crates/ruviform-ui-test/src/golden/normalize.rs` 新設:
  - `GoldenKeyEnv` — `GoldenKey` の env 側 class DTO。
  - `key_env(env, driver_class)` — `EnvironmentMeta` からの
    正規化（os/gpu/scale/font の導出規則と妥当性検査）。
  - `NormalizeError` — 正規化失敗の tool error（verdict では
    ない）。
  - `key_for_fixture(fixture, state, env, revision)` —
    `StoryFixture` に結合した `GoldenKey` 構築（story_id/theme
    は fixture から取り drift しない）。
  - `registry_mismatches(key, manifest)` — `StoryManifest`
    （stories.json）に対する `story_id`/`theme`/`state`/
    `scale_factor` の整合検査と `RegistryMismatch` 列挙。
- `crates/ruviform-ui-test/src/golden.rs`: `pub mod normalize;`
  宣言と re-export のみ。
- `crates/ruviform-ui-test/src/golden/report.rs`:
  `GoldenEvidence` への `lane` field 追加（`"pixel-golden"`
  固定、`rvf.golden.evidence/v1` の additive 拡張）と
  `EVIDENCE_LANE` 定数。
- `crates/ruviform-ui-test/src/scenario/visual.rs`:
  `record_golden_outcome(&mut ScenarioOutcome, &GoldenComparison)`
  追加（`record_golden_step` の `ScenarioOutcome` 版）と、
  観測 JSON 組み立ての共有 helper 化、module doc の
  `run_scenario` 組み立て例の更新。既存 API の動作変更なし。
- `crates/ruviform-ui-test/tests/golden.rs` /
  `tests/scenario_contract.rs`: 正規化・registry 検査・
  `run_scenario` 経由の golden step・evidence lane の検証を追加。
- 本 slice の文書チェーン（spec/design/plan/task/test-spec）と
  docs index の再生成。

### Out of Scope

- 実 pixel capture・offscreen backend — #313/slice 3 の範囲。
  本 slice は取得済み pixel と環境証拠の utility のみ。
- consumer story fixture への実接続（AudioLens/Synvelis の
  product fixture、baseline 画像の生成）— consumer 側の後続
  slice（#426 系）。本 slice は型と手順の提供まで。
- `EnvironmentMeta`/`GoldenKey`/`DiffPolicy` 等の既存 pub struct
  への field 追加 — `ruviform-ui-gpui` の test が struct literal
  で構築しており、本 slice の file boundary（ui-test のみ）では
  追従修正できない。`locale` 等の #313 evidence 項目の追加は
  consumer 側 field 追加が可能になった時点の別 slice。
- `stories.json`/`cases.json` manifest schema の変更、
  `lab_story` fixture の追加・変更。
- draw plan（`DrawnPlan`）比較の実装 — `ruviform-ui-gpui` 側の
  責務。本 slice は pixel golden との lane 分離を型と証拠で
  固定するのみ。
- 新規依存・`Cargo.toml`/`mise.toml`/`.github/` の変更。
- `ruviform-ui-gpui`・`ruviform-ui`・`ruviform-ui-driver` 等の
  他 crate の変更（参照のみ）。

## Requirements

- R1: `key_env(&EnvironmentMeta, driver_class)` が `GoldenKey`
  の env 側 5 class を返す。`driver_class` は caller が宣言する
  capture lane class であり、backend 名からの推測はしない。
  `os_class` は `env.os` の trim 済み非空値、`gpu_class` は
  `env.gpu.class` 優先・無ければ `env.gpu.vendor`・両方無ければ
  `None`、`font_class` は `env.font_set` の trim 済み値で空は
  `None`、`scale_factor` は有限かつ正の値のみ受理する。
- R2: `scale_factor` が非有限（NaN/±inf）または 0 以下の場合、
  正規化は `Err(NormalizeError::InvalidScaleFactor)` — 不正な
  環境値を key に混入させない。`os`/`driver_class` が空の場合も
  `Err`。取得不能な値は `None`/`unknown` のまま（捏造しない）。
- R3: `key_for_fixture` は `StoryFixture.story_id`/`theme` を
  そのまま `GoldenKey` に写す — registry に存在しない story
  id/theme を key に入れる経路を型で排除する。`state`/`revision`
  は caller 指定。
- R4: `registry_mismatches` は `stories.json` manifest に対し、
  未登録 `story_id`・未宣言 `theme`・未定義 `state`・未宣言
  `scale_factor` を `RegistryMismatch` として列挙する（空 =
  整合）。manifest に無い story（probe 用 key 等）は caller が
  検査を適用しない判断をする — 本関数は強制ではなく検査手段。
- R5: `record_golden_outcome` は `record_golden_step` と同一の
  evidence 規則（`golden:<canonical>` の evidence 参照 +
  `observed["golden"]` JSON）を `ScenarioOutcome` に適用する。
  `outcome.verdict` は consumer が決める — golden `pass` が
  scenario `pass` を意味しない点は `record_golden_step` と同じ。
- R6: `GoldenEvidence` は `lane: "pixel-golden"` を常に記録する。
  semantic oracle・draw plan・OS input の証拠は別 schema/別
  lane であり、本 evidence をそれらの pass として引用しない。
- R7: fake/headless での検証は合成 `EnvironmentMeta`/fixture
  画像/既知差分で行い、実 capture・実 OS・GPU・font 環境に
  依存しない。`unsupported`/`not_run`/`blocked` は `pass` に
  昇格しない。
- R8: 変更は additive — 既存 pub API の signature/field/
  挙動を変えない。`scenario/` は helper 追加と doc 更新のみで
  既存実装を破壊しない。

## Acceptance Criteria

| ID | 条件 | 検証 |
| --- | --- | --- |
| AC-1 | `key_env` が `EnvironmentMeta` から env class を正規化し、gpu class/vendor fallback・font trim・scale 妥当性を規則通り返す | `golden::normalize` unit tests |
| AC-2 | 不正 scale（NaN/0/負）・空 os/driver_class が `NormalizeError` になり key に混入しない | unit tests |
| AC-3 | `key_for_fixture` の `GoldenKey` が fixture の story_id/theme を持ち、canonical/file_stem が決定的 | unit tests + `tests/golden.rs` |
| AC-4 | `registry_mismatches` が実 `stories.json` に対し、整合 key で空・各 drift で対応 field の mismatch を返す | unit tests（実 fixture path を読む） |
| AC-5 | `run_scenario` body 内で `golden_check` → `record_golden_outcome` が `ScenarioOutcome`/`ScenarioReport` に `golden:` evidence と `observed.golden` を残し、Fake の `Unsupported` がそのまま verdict に記録される | `tests/scenario_contract.rs` 追加 test |
| AC-6 | `GoldenEvidence` JSONL 行が `"lane":"pixel-golden"` を含み、`pass`/`fail`/`unsupported`/`not_run`/`blocked` の verdict が lane を偽装しない | `tests/golden.rs` 追加 test |
| AC-7 | 既存 golden/scenario/store/diff の非回帰 — `mise run test-ui-golden`・`test-ui-scenario-contract*` が pass | mise tasks |
| AC-8 | #262 category C・#313 OS Gate への証拠リンク構造（どの evidence field がどの gate 要件に対応するか）が design 文書に記述される | design doc review |

## Deferred Design Concerns

- `EnvironmentMeta` への `locale` 等の field 追加 — #313 の
  evidence 最低要素に locale があるが、pub struct への field
  追加は `ruviform-ui-gpui` の既存 struct literal を壊し、本
  slice の file boundary では追従できない。consumer 側で
  field 追加可能になった時点の別 slice。
- draw plan 比較の verdict 型 — `DrawnPlan` は
  `ruviform-ui-gpui` 側。plan 比較 pass と golden pass の
  分離は本 slice では evidence lane と文書で固定し、plan 側の
  型整備は GPUI/consumer 側の後続。
- AudioLens/Synvelis product fixture への実接続 — consumer 側
  後続 slice（#426 系）。本 slice は key/evidence の型のみ。
- 実 Offscreen capture・実 OS pixel の常時 lane — #313 の
  実機環境が前提（NotRun/Unsupported を正直に記録する契約は
  slice 1〜3 で確立済み）。

## Sources and Relations

- Issue #314（本 slice の対象 Issue。Owner 範囲: pixel
  screenshot の取得・環境正規化・baseline 比較・差分承認・
  証拠）。
- Issue #464 / PR #511（`run_scenario`/`ScenarioSpec`/
  `ScenarioOutcome`/`settle_until` — 本 slice が消費する
  Scenario runner 契約）。
- Issue #390（Driver Contract Lab — `stories.json` manifest /
  `StoryManifest` registry と verdict/evidence 語彙の owner）。
- Issue #426（Story fixture — `lab_story::StoryFixture` code
  registry）。
- Issue #262（GPUI 採否判定 — category C が pixel/数値/性能の
  実測証拠を要求。本 slice の evidence lane がその入力形）。
- Issue #313（実 OS Driver — screenshot 証拠の統合先。
  `EnvironmentMeta` が環境証拠の器）。
- `docs/specs/issue-314-golden-core.md`（slice 1 — Golden
  Utility。PR #481 / merge `2230018e`）。
- `docs/specs/issue-314-golden-scenario.md`（slice 2 — Scenario
  Contract 接続。PR #483 / merge `7ae916b3`）。
- `docs/specs/issue-314-golden-capture.md`（slice 3 — GPUI/OS
  capture backend。PR #487 / merge `dc01934f`）。
