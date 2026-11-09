---
id: "DESIGN-L3YSlsSU8u5EdRMDfpHhy6"
type: "design"
status: "approved"
title: "Issue #314 slice 5 — Golden 環境正規化・registry 結合・runner 統合・証拠 lane の設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/ruviform/issues/464"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-314-golden-evidence.md"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/normalize.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/report.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/visual.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-314-golden-evidence.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-314-golden-scenario.md"
    - "docs/designs/issue-314-golden-core.md"
    - "docs/designs/issue-464-scenario-runner.md"
    - "docs/designs/issue-390-driver-contract-lab.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-314-golden-evidence.md"
    - "docs/designs/issue-314-golden-scenario.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-314-golden-scenario.md"
    - "docs/designs/issue-314-golden-core.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-314-golden-evidence.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 5 — Golden 環境正規化・registry 結合・runner 統合・証拠 lane の設計

## Context

slice 1〜3 で `GoldenKey`/`EnvironmentMeta`/`RgbaImage`/`DiffPolicy`/
`BaselineStore`/`GoldenReport`/`GoldenEvidence` と
`VisualGuiTest::golden_check`/`record_golden_step`、GPUI 実窓
`capture_window` が揃った。しかし:

- `GoldenKey` の env 側 class（`driver_class`/`os_class`/
  `gpu_class`/`scale_factor`/`font_class`）を `EnvironmentMeta`
  から導く規則が存在せず、consumer が手組みしている
  （`ruviform-ui-gpui/tests/golden_capture.rs` の
  `story_golden_key`/`environment`、`lab_story.rs` の
  `story_golden_key`）。`key.rs` は「生値の丸め規則は capture
  層の責務」としたが、capture 層（slice 3）は GPUI backend の
  みで、key 構築規則は提供していない。
- `GoldenKey.story_id`/`theme` が #390 manifest（stories.json）
  ・#426 fixture（`lab_story::StoryFixture`）に実在するかの
  検査経路がない — `key.rs` は「registry への結合・実在検査を
  行わない」と明記して先送りした。
- `record_golden_step` は `ScenarioReport`（run_scenario の
  戻り値）向けだが、`run_scenario`/`ScenarioOutcome`（PR #511）
  の body 内で golden step を畳み込む helper がない。
- `GoldenEvidence`（`rvf.golden.evidence/v1`）は pixel golden の
  結果のみを記述するが、どの証拠 lane に属するかを機械可読に
  持たない。

## Scope

### In Scope

- `golden/normalize.rs` 新設（`GoldenKeyEnv`、`key_env`、
  `NormalizeError`、`key_for_fixture`、`registry_mismatches`、
  `RegistryMismatch`）。
- `golden.rs` の mod 宣言 + re-export。
- `golden/report.rs` の `GoldenEvidence.lane` 追加
  （additive、schema `rvf.golden.evidence/v1` のまま）。
- `scenario/visual.rs` の `record_golden_outcome` 追加と
  観測 JSON 組み立ての共有化、module doc 更新。
- `tests/golden.rs`/`tests/scenario_contract.rs` への test 追加。
- 本 slice の文書チェーン。

### Out of Scope

- spec の Out of Scope と同一（実 capture、consumer fixture
  実接続、pub struct への field 追加、manifest schema 変更、
  draw plan 型、他 crate、新規依存）。

## Components and Boundaries

```text
crates/ruviform-ui-test/src/golden/normalize.rs   — 正規化 + registry 結合（新規）
crates/ruviform-ui-test/src/golden.rs             — mod 宣言 + re-export
crates/ruviform-ui-test/src/golden/report.rs      — GoldenEvidence.lane 追加
crates/ruviform-ui-test/src/scenario/visual.rs    — record_golden_outcome + 共有 helper
crates/ruviform-ui-test/tests/{golden,scenario_contract}.rs — test 追加
```

依存方向: `golden::normalize` → `crate::lab_story`（`StoryFixture`）
と `crate::driver_lab`（`StoryManifest`/`StoryEntry`）。`golden` は
既に `driver_lab` の `pub(crate)` evidence helper を使っており
（`report.rs`）、同一 crate 内の下向き参照として許容する。
`lab_story`/`driver_lab` は `golden` を参照しないため循環しない。
`scenario::visual` → `scenario::runner::ScenarioOutcome` は同一
module tree 内の参照。

### `GoldenKeyEnv` / `key_env`

```rust
pub struct GoldenKeyEnv {
    pub driver_class: String,    // caller 宣言の capture lane class
    pub os_class: String,        // env.os trim 済み
    pub gpu_class: Option<String>, // env.gpu.class → env.gpu.vendor → None
    pub scale_factor: f64,       // env.scale_factor（有限かつ正）
    pub font_class: Option<String>, // env.font_set trim 済み、空は None
}

pub fn key_env(env: &EnvironmentMeta, driver_class: &str)
    -> Result<GoldenKeyEnv, NormalizeError>
```

正規化規則（全て決定的・推測しない）:

| GoldenKey field | 導出元 | 規則 |
| --- | --- | --- |
| `driver_class` | 引数 | trim、空は `EmptyDriverClass`。backend 名からの推測はしない — lane class は caller の宣言。 |
| `os_class` | `env.os` | trim、空は `EmptyOsClass`。`EnvironmentMeta.os` は既に class 文字列を想定。 |
| `gpu_class` | `env.gpu` | `gpu.class`（既に正規化 class）優先、無ければ `gpu.vendor`、両方無ければ `None`。生 GPU 名（`gpu.name`）は key に入れない — key は class のみ。 |
| `scale_factor` | `env.scale_factor` | 有限かつ `> 0` のみ受理。NaN/±inf/0 以下は `InvalidScaleFactor` — 不正値を key に混入させない。 |
| `font_class` | `env.font_set` | trim、空は `None`。`font_set` は field doc 上「正規化表現」を期待するため追加の丸めはしない（大小文字・空白の解釈は consumer の正規化規約に委ね、ここでは空値の排除のみ）。 |

`NormalizeError` は `#[non_exhaustive]` enum
（`EmptyDriverClass`/`EmptyOsClass`/`InvalidScaleFactor{value}`）で
`Display`+`Error` を実装。tool error であり verdict ではない。

### `key_for_fixture` / `registry_mismatches`

```rust
pub fn key_for_fixture(
    fixture: &StoryFixture,
    state: &str,
    env: &GoldenKeyEnv,
    revision: &str,
) -> GoldenKey
```

`story_id`/`theme` を `fixture` から写すため、code registry
（#426 `lab_story::all_fixtures()`）に存在しない id/theme を
key に入れる経路が型で排除される。`state` は consumer の state
軸（semantic walk の state 名を想定するが強制しない — golden
state は必ずしも walk state と一致しない）。`revision` は
caller が `env!("CARGO_PKG_VERSION")` 等を渡す。

```rust
pub struct RegistryMismatch {
    pub field: &'static str,  // "story_id" | "theme" | "state" | "scale_factor"
    pub expected: String,     // registry 側の期待（宣言値または許容一覧）
    pub actual: String,
}

pub fn registry_mismatches(key: &GoldenKey, manifest: &StoryManifest)
    -> Vec<RegistryMismatch>
```

manifest（#390 `stories.json`）に対する整合検査:

- `story_id`: `manifest.stories` に `id` 一致の entry があるか。
- `theme`: `manifest.themes` に含まれるか（story の宣言 theme は
  1 つだが #426 matrix は両 theme を走るため、theme 軸は
  manifest 全体の宣言集合で検査する）。
- `state`: 該当 story entry の `states` に含まれるか
  （entry-level — story 未登録の場合は skip）。
- `scale_factor`: 該当 story entry の `scaleFactor` に f64 完全
  一致で含まれるか（NaN は一致しない — 正直に mismatch。
  entry-level — story 未登録の場合は skip）。
- `theme` は manifest-level の宣言集合に対する検査のため、
  story_id の成否に依らず常に行う（未登録 story + 未宣言 theme
  の key は両方の mismatch を報告する）。

空 Vec = 整合。manifest に無い story（probe 用 key 等）への適用は
caller の判断 — 強制ではなく検査手段。`driver_class`/`os_class`/
`gpu_class`/`font_class`/`revision` は registry の管轄外
（driver lane・環境 class・revision は manifest に宣言がない）。

### `GoldenEvidence.lane`

`rvf.golden.evidence/v1` JSONL 行に `"lane":"pixel-golden"` を
追加する。`GoldenEvidence` は `Serialize` のみで `From<&GoldenReport>`
経由でのみ構築されるため、field 追加は additive で既存の読み手
（JSONL を parse する外部 tool は存在しない — evidence は人間/
CI の grep と将来の集約が消費）を壊さない。

lane 語彙の対応:

| lane | 証拠の形 | owner |
| --- | --- | --- |
| semantic | `StoryStep`/`ComponentSnapshot` trace、`ScenarioReport.observed` | #464/#390/#426 |
| draw plan | `DrawnPlan` 比較（`ruviform-ui-gpui`、geometry/値のみ） | GPUI driver |
| pixel golden | `GoldenEvidence`（`lane="pixel-golden"`） | 本 Issue #314 |
| OS input | #313 real-OS driver の実行記録 | #313 |

`GoldenReport`/`GoldenVerdict` は型として pixel golden にのみ
到達可能（`DrawnPlan` から `GoldenReport` を作る経路は存在し
ない）— plan pass が golden pass に化けるのを型で防ぐ。evidence
の `lane` field は artifact 側でも同じ分離を固定する。

### `record_golden_outcome`

```rust
pub fn record_golden_outcome(
    outcome: &mut ScenarioOutcome,
    comparison: &GoldenComparison,
)
```

`run_scenario` の body 内で `golden_check` を呼んだ直後に使う
`ScenarioOutcome` 版の fold。`record_golden_step`（`ScenarioReport`
版）と同一規則:

- `outcome.evidence` に `golden:<canonical key>` を push。
- `outcome.observed["golden"]` に `{verdict, reason, detail,
  baseline, stats, artifacts}` を merge（既存 object への追加・
  `None` からの生成・非 object 温存の規則は `record_golden_step`
  と同一）。
- `outcome.verdict` は触れない — golden `pass` は semantic
  check の pass を意味しない。body が golden step のみなら
  consumer は `ScenarioVerdict::from(comparison.report.verdict)`
  を `ScenarioOutcome::new` に渡してよい（name-for-name 写像は
  `From<GoldenVerdict>` が保証）。

`record_golden_step`/`record_golden_outcome` で共有する観測 JSON
組み立てと observed-merge を private helper に抽出する
（`golden_observed_entry`/`merge_golden_observed`）— 同一規則の
重複実装を避けるための局所的集約であり、公開挙動は不変。

### #262/#313 への証拠リンク構造

`GoldenEvidence` の各 field が下流 gate の要件にどう対応するか:

| #262 category / #313 要件 | 対応する evidence field |
| --- | --- |
| #262 C（実 pixel/数値/性能の実測） | `lane="pixel-golden"` + `verdict`/`reason`/`stats`（pixel 差分の実測値） |
| #262 A/B/D/E（contract・OS input・plugin dock・製品 oracle） | 対象外 — 別 lane/別 Issue の証拠。golden evidence をそれらの pass に使わない |
| #313 OS/version・GPU/driver | `environment.os`/`os_version`/`arch`/`backend`/`backend_detail`/`gpu` |
| #313 dpi/font | `environment.scale_factor`/`font_set`（`key.scale_factor`/`font_class` は正規化 class） |
| #313 fixture ID / expected-actual | `subject`（canonical key = story+theme+state+env+revision）と `baseline`/`stats` |
| #313 locale・executable command・logs/video・CPU/RAM | 本 slice 対象外 — `EnvironmentMeta` に field がなく、pub struct 変更は file boundary 外（Deferred 参照） |

## Data and Control Flow

```text
capture 層（slice 3 / consumer）
  │  OsCapturedFrame / Screenshot → RgbaImage（decode 済み）
  ├─ EnvironmentMeta（観測した環境証拠）
  │     └─ golden::key_env(&env, "<driver-class>")
  │           → GoldenKeyEnv（os/gpu/scale/font 正規化）
  ├─ lab_story::StoryFixture（#426 code registry）
  │     └─ golden::key_for_fixture(fixture, state, &env_classes, rev)
  │           → GoldenKey（story_id/theme は fixture 由来）
  ├─ driver_lab::load_stories(stories.json)（#390 JSON registry）
  │     └─ golden::registry_mismatches(&key, &manifest)
  │           → Vec<RegistryMismatch>（空 = 整合、audit 用）
  ├─ BaselineStore::compare → GoldenComparison
  │     ├─ emit_evidence → rvf.golden.evidence/v1 JSONL
  │     │     （lane="pixel-golden"、subject=canonical key、
  │     │      environment=EnvironmentMeta）
  │     └─ record_golden_step（ScenarioReport 向け・既存）
  │     └─ record_golden_outcome（ScenarioOutcome 向け・新規）
  │           ↑ run_scenario body 内で consumer が呼ぶ
  └─ verdict: pass/fail/not_run/unsupported/blocked
        — pixel 比較の実測のみ pass/fail、未取得は昇格しない
```

## Non-Functional Constraints

- 非 RT only（既存 golden module と同じ）。
- 新規依存なし — `serde`/`serde_json`/`std` のみ。
- `unsafe` なし。
- 全 pub enum に `#[non_exhaustive]`（`NormalizeError` が対象。
  `RegistryMismatch`/`GoldenKeyEnv` は struct であり、既存の
  pub struct（`GoldenKey`/`EnvironmentMeta` 等）と同じく
  `#[non_exhaustive]` は付けない）。
- 公開 signature に GPUI/wry/OS handle 型を出さない
  （driver-neutral）。

## Risks and Trade-offs

| リスク | 影響 | 対策 |
| --- | --- | --- |
| `key_env` の正規化規則が consumer の期待とずれる | key の env class が不安定 | 規則を design に明文化し unit test で固定。`font_set` の追加丸めをしない判断は「consumer が正規化済み表現を渡す」field doc の契約に従う |
| `registry_mismatches` が manifest の scaleFactor に無い scale を fail にする | 正当な scale 追加時に mismatch | 検査は opt-in の audit 手段であり強制ではない。manifest 更新とセットで運用 |
| `record_golden_outcome` と `record_golden_step` の規則が将来乖離 | 同一 scenario の report/outcome で evidence が違う | 観測 JSON 組み立てを共有 helper に集約し、両者が同一関数を呼ぶ構造にする |
| `lane` field 追加で古い evidence consumer が未知 field で失敗 | evidence parse error | `GoldenEvidence` は Serialize-only で repo 内に Deserialize consumer は存在しない。外部 consumer ができた場合は schema versioning で対応 |
| `golden` → `lab_story`/`driver_lab` の依存追加 | module 間結合 | `golden` は既に `driver_lab` helper を使用。`lab_story` は golden を参照しないため循環なし。fixture registry への結合は本 slice の目的そのもの |

## D1: 正規化は `EnvironmentMeta` → `GoldenKeyEnv` の一方向投影

`GoldenKey`/`EnvironmentMeta` は既に pub で consumer が literal
構築している（`ruviform-ui-gpui` test 含む）。field 追加は file
boundary 外の修正を要するため行わず、**新しい投影型
`GoldenKeyEnv` + 関数 `key_env`** で正規化規則を 1 箇所に集約
する。`key_for_fixture` が `GoldenKey` を組み立てる最終形。

## D2: registry 結合は 2 段（code fixture + JSON manifest）

- `key_for_fixture` は `StoryFixture`（code、compile 時に実在
  保証）から story_id/theme を取る — **構築時に drift しない**。
- `registry_mismatches` は `StoryManifest`（JSON、runtime 検査）
  に対する **監査** — manifest と code fixture の drift
  （`driver_contract.rs` が既に story id 集合の一致を検査）を
  前提に、key の state/scale/theme が manifest 宣言内かを
  報告する。強制ではなく `Vec<RegistryMismatch>` を返す。

## D3: `record_golden_outcome` は helper であり runner ではない

「別の汎用 Runner は作らない」契約に従い、`run_scenario` の
body 内で使う fold helper のみ追加する。`run_scenario` 自体・
`ScenarioSpec`/`ScenarioOutcome` の構造は変更しない。
`record_golden_step`（report 版）と観測 JSON 組み立てを共有
helper に抽出して規則の重複を防ぐ。

## D4: `lane` は evidence artifact のみに付与

`GoldenReport`（in-memory）は型として pixel golden にのみ
到達可能なため lane 不要。`GoldenEvidence`（永続 artifact、
複数 lane が混在し得る JSONL）のみ `"pixel-golden"` を記録し、
gate 側が lane で選別できる構造にする。semantic/plan/OS-input
側の lane 語彙は各 owner の schema が持つ（本 slice では
golden 側の宣言のみ）。

## Verification Notes

- [ ] `mise run fmt-check` / `check` / `lint` / `test`
- [ ] `mise run test-ui-golden`（既存 + 追加 test）
- [ ] `mise run test-ui-scenario-contract` / `-fake` / `-gpui`
      （非回帰 + 追加 test）
- [ ] `mise run check-ui-driver-boundary`（公開 signature の
      driver-neutral 性）
- [ ] `mise run test-ui-test-msrv`（MSRV 1.89）
- [ ] `mise run test-ui-driver-contract`（manifest 非回帰）
