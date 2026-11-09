---
id: "TSPEC-suZHG6N60Cpt2NxAwyqq2G"
type: "test-spec"
status: "approved"
title: "Issue #314 slice 5 — Golden 環境正規化・registry 結合・runner 統合・証拠 lane のテスト仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-314-golden-evidence.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/normalize.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/visual.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-314-golden-evidence.md"
    - "docs/designs/issue-314-golden-evidence.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-314-golden-evidence.md"
  verifies:
    - "docs/specs/issue-314-golden-evidence.md"
    - "docs/designs/issue-314-golden-evidence.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 5 — Golden 環境正規化・registry 結合・runner 統合・証拠 lane のテスト仕様

全て合成 `EnvironmentMeta`・`StoryFixture`・実 `stories.json`
fixture・tempdir による headless 検証。実描画・実 OS・GPU・
フォント環境には依存しない。

## Feature: `golden::normalize`（normalize.rs）

### Rule: env class の正規化規則

- Example: `EnvironmentMeta{os:" windows ", gpu:{class:Some("dx11"),
  vendor:Some("nvidia")}, scale_factor:2.0, font_set:Some(" Inter-V13 ")}`
  に対し `key_env(&env, "gpui-real-window")` を呼ぶと、
  `os_class=="windows"`、`gpu_class==Some("dx11")`（class 優先）、
  `scale_factor==2.0`、`font_class==Some("Inter-V13")`（trim のみ、
  大小文字は保持）、`driver_class=="gpui-real-window"` を返す。
- Example: `gpu.class==None, gpu.vendor==Some("intel")` なら
  `gpu_class==Some("intel")`。両方 `None` なら `gpu_class==None`。
  `font_set==Some("  ")`（空白のみ）なら `font_class==None`。
- 検証: `key_env_*` unit tests。

### Rule: 不正な環境値は key に混入しない

- Example: `scale_factor` が `f64::NAN`/`0.0`/`-1.0`/`f64::INFINITY`
  の場合、`key_env` は `Err(NormalizeError::InvalidScaleFactor)`。
  `os==""` や `driver_class=="  "` は `Err(EmptyOsClass)`/
  `Err(EmptyDriverClass)`。
- Example: `EnvironmentMeta.gpu.name`（生 GPU 名）は
  `gpu_class` に採用されない — `gpu.class`/`gpu.vendor` のみ。
- 検証: unit tests。

### Rule: fixture 結合 key は registry の story_id/theme を持つ

- Example: `sv_fader_dock_fixture()`（story_id=`LAB-SV-FADER-DOCK`、
  theme=`synvelis-dark`）に対し `key_for_fixture(fixture, "idle",
  &env, "rev")` を呼ぶと、`GoldenKey.story_id=="LAB-SV-FADER-DOCK"`、
  `theme=="synvelis-dark"`、`state=="idle"`、env class は
  `GoldenKeyEnv` の値、`revision=="rev"` となる。
- Example: 2 fixture（`sv_fader_dock_fixture`/`al_waveform_seek_fixture`）
  で異なる `state` を与えると `canonical()`/`file_stem()` が
  決定的に異なる。
- 検証: unit tests + `tests/golden.rs` integration。

### Rule: manifest 整合検査

- Example: 実 `fixtures/driver-lab/stories.json` を
  `driver_lab::load_stories` で読み、fixture 由来の整合 key
  （`LAB-SV-FADER-DOCK`/`synvelis-dark`/`idle`/scale `1.0`）に
  `registry_mismatches` を適用すると空 Vec。
- Example: `story_id` を未登録値に、`theme` を `manifest.themes`
  外に、`state` を entry.states 外に、`scale_factor` を
  `scaleFactor` 外に変えた key は、それぞれ対応 field の
  `RegistryMismatch` を 1 件ずつ返す。
- 検証: unit tests（`CARGO_MANIFEST_DIR` 相対で実 fixture を読む）。

## Feature: `GoldenEvidence.lane`（report.rs）

### Rule: evidence は pixel-golden lane を名乗る

- Example: 任意の `GoldenReport`（pass/fail/unsupported/
  not_run/blocked のいずれでも）から `GoldenEvidence::from` で
  生成し `append_evidence` で JSONL に書くと、各行が
  `"lane":"pixel-golden"` と `"schema":"rvf.golden.evidence/v1"`
  を含む。
- Example: `lane` は verdict に依らず常に `"pixel-golden"` —
  `unsupported`/`not_run`/`blocked` も lane を偽装しない。
- 検証: `tests/golden.rs` の evidence test 拡張。

## Feature: `record_golden_outcome`（scenario/visual.rs）

### Rule: ScenarioOutcome への golden fold

- Example: Fake adapter に対し `run_scenario` の body 内で
  `resolve_target` → `golden_check`（→ `Unsupported`）→
  `record_golden_outcome(&mut outcome, &comparison)` を呼び、
  `outcome.verdict = ScenarioVerdict::from(comparison.report.
  verdict)` とすると、`run_scenario` の `ScenarioReport` が
  `verdict==Unsupported`、`evidence` に `golden:<canonical>`、
  `observed["golden"]["verdict"]=="unsupported"` を持つ。
- Example: `record_golden_outcome` は `outcome.verdict` を
  変更しない — body が verdict を設定しない限り report の
  verdict は body の `ScenarioOutcome::new` 既定値のまま。
- Example: 既存 `observed` が object の場合は `golden` key を
  merge し、非 object の場合は温存する（`record_golden_step`
  と同一規則）。
- 検証: `tests/scenario_contract.rs` 追加 test。

## Guarantees

- `key_env`/`key_for_fixture`/`registry_mismatches`/
  `record_golden_outcome` は全て決定的で、環境・時刻・乱数に
  依存しない（`GoldenEvidence.recorded_at_unix_ms` を除く —
  既存仕様）。
- `unsupported`/`not_run`/`blocked` が `pass` に昇格する経路は
  存在しない（既存契約の非回帰 + lane が verdict を偽装しない）。
- `run_scenario`/`ScenarioSpec`/`ScenarioOutcome`/`settle_until`
  と `record_golden_step` の既存公開挙動は不変。

## Non-goals

- 実 pixel capture・offscreen・実 OS 環境 — headless 検証のみ。
- `EnvironmentMeta`/`GoldenKey` への field 追加 — pub struct
  変更は file boundary 外。
- draw plan 比較・consumer product fixture・manifest schema
  変更・新規依存。

## Risk

- 実 `stories.json` fixture path の解決 — unit test は
  `env!("CARGO_MANIFEST_DIR")` 相対で読む（`driver_contract.rs`
  と同じ規約）。fixture 移動時は test が fail して検出される。
- `record_golden_outcome` の merge 規則が `record_golden_step`
  と乖離するリスク — 観測 JSON 組み立てを共有 helper に集約し
  両者が同一関数を呼ぶ構造で防ぐ（test で双方の規則を固定）。
