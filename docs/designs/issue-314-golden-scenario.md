---
id: "DESIGN-034UewQlz8GZ6RlexpCiXW"
type: "design"
status: "approved"
title: "Issue #314 slice 2 — Golden Utility × Scenario Contract 能力契約接続の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-314-golden-scenario.md"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/visual.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/fake.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-314-golden-scenario.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-314-golden-core.md"
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-314-golden-scenario.md"
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-314-golden-core.md"
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-314-golden-scenario.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 2 — Golden Utility × Scenario Contract 能力契約接続の設計

## Context

slice 1 の `golden` モジュール（`ruviform-ui-test`）は「得られた
`RgbaImage`」と `proto::Screenshot` の decode、baseline 比較、
verdict、evidence を提供するが、capture も scenario との接続も
持たない。#464 の `SemanticGuiTest` は capability 宣言・target pin・
report 語彙を持つが、visual/pixel の操作を表現する port がない。

本 slice は両者の間に **visual/golden 接続層**を 1 枚挿す。実 pixel
capture backend は存在しない（slice 3）ため、本 slice の全 adapter
実装は「capture できないことを正直に報告する」経路のみを実装する
— 契約の形を先に固定し、slice 3 が `capture_frame` の中身だけを
差し替えられる状態にする。

## Scope

### In Scope

- `ruviform-ui-test::scenario::visual` モジュール新設:
  `CaptureOutcome`・`VisualGuiTest` trait・`golden_check` 既定実装・
  `record_golden_step`・`From<GoldenVerdict> for ScenarioVerdict`。
- `ScenarioVerdict::lane_status`（consumer-verify lane 語彙への
  写像、`blocked`→`not_run`）の追加。
- `FakeGuiTest`・`GpuiGuiTest` への `VisualGuiTest` 実装
  （`capture_frame` のみ — `golden_check` は既定実装を共有）。
- Fake/GPUI 両側の contract test 追加、mise 説明・文書更新。

### Out of Scope

- pixel capture の実装（`capture_frame` が `Captured` を返す経路は
  slice 3 で実装。本 slice の adapter は全て `Unsupported` を返す）。
- `capabilities()` への `VisualCapture` 宣言（後述 D3）。
- `GuiAction` への golden step 追加 — golden check は action では
  なく report を返す検証ステップであり、`perform` の
  `Result<(), ScenarioError>` 形に収まらない（後述 D1）。
- JSON DSL・第二 executor・新 GUI framework（#464 の禁止事項を継承）。

## Components and Boundaries

```text
crates/ruviform-ui-test/src/scenario/visual.rs   — 接続層（新規）
crates/ruviform-ui-test/src/scenario.rs          — ScenarioVerdict::lane_status 追加
crates/ruviform-ui-test/src/scenario/fake.rs     — impl VisualGuiTest for FakeGuiTest
crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs — impl VisualGuiTest for GpuiGuiTest
```

- **`CaptureOutcome`**（`#[non_exhaustive]` enum）: capture 試行の
  正直な 3 値結果。
  - `Captured(RgbaImage)` — 実 pixel が取れた。
  - `Unsupported { reason }` — adapter/環境が pixel を生成できない
    → golden `unsupported`。
  - `NotRun { reason }` — capture 経路はあるが前提未達
    → golden `not_run`。
  `reason` は機械可読 `snake_case` コード（`String`）。
- **`VisualGuiTest`**: `SemanticGuiTest` を supertrait に持つ
  visual ポート。
  - 必須: `capture_frame(&mut self, target) ->
    Result<CaptureOutcome, ScenarioError>` — adapter 固有の唯一の
    primitive。pin 再検査（stale → `ScenarioError`）を他操作と
    同じ契約で行い、pixel を捏造しない。
  - 既定提供: `golden_check(&mut self, target, key, environment,
    store) -> Result<GoldenComparison, ScenarioError>` — capture →
    `BaselineStore::compare` → `emit_evidence` を 1 箇所に集約。
    `GoldenError` は `ScenarioError::Driver` に写像する
    （`ScenarioError` は `Clone + PartialEq` を維持するため
    `GoldenError`（`io::Error` 内包）を variant に持てない）。
- **`record_golden_step(report, comparison)`**: `ScenarioReport`
  への canonical 記録 — `evidence` に `golden:<canonical key>`、
  `observed["golden"]` に verdict/reason/detail/baseline/stats の
  JSON。`report.verdict` は触らない（consumer の総合判断）。
- **`From<GoldenVerdict> for ScenarioVerdict`**: 5 値同一写像。
- **`ScenarioVerdict::lane_status()`**: consumer-verify lane 語彙
  （`pass`/`fail`/`not_run`/`unsupported`/`skipped`）への写像。
  `Blocked` → `"not_run"`。caller は元 verdict と reason を
  detail に保持する（`blocked` = `not_run` + reason、#390 由来の
  規則をコード化）。

## Data and Control Flow

```text
scenario (consumer)
  │  gui: impl VisualGuiTest
  ├─ gui.require_capabilities({VisualCapture})   ← pre-flight（任意・既存機構）
  │     └─ Unsupported capability → ScenarioError::Unsupported
  ├─ gui.golden_check(&target, &key, env, &store)
  │     ├─ self.capture_frame(target)             ← adapter 実装
  │     │     ├─ check_target → stale なら ScenarioError
  │     │     └─ Unsupported/NotRun/Captured
  │     ├─ Captured(img) → store.compare(key, img, env)
  │     │     └─ GoldenError → ScenarioError::Driver
  │     ├─ Unsupported/NotRun → golden::unsupported/not_run report
  │     └─ emit_evidence(&report)                 ← env ゲート
  │           → Result<GoldenComparison, ScenarioError>
  ├─ record_golden_step(&mut report, &comparison) ← evidence/observed 記録
  └─ ScenarioVerdict::from(comparison.report.verdict)
        → report.verdict / lane_status() で lane 語彙へ
```

- `capabilities()`/`require_capabilities` は **pre-flight の宣言語彙**
  のまま変更しない。`golden_check` は `capture_frame` の実測
  outcome を authoritative とし、capability 未宣言でも呼べば
  正直な `unsupported` report を返す（defense in depth —
  pre-flight を忘れた consumer でも嘘の `pass` は出ない）。
- adapter の `capture_frame` は pin 再検査を最初に行う —
  「capture できない adapter でも stale target は contract error」
  を保つため、outcome 返却より先に `check_target` を通す。

## Non-Functional Constraints

- 非 RT（test/検証基盤）。`!Send` の GPUI 状態は test thread に
  留まる（既存 `GpuiGuiTest` 契約を継承）。
- MSRV 1.89、`workspace.lints` 準拠、公開 enum は
  `#[non_exhaustive]`。
- `ruviform-ui-test` → `ruviform-ui-gpui` の依存は禁止（DAG 維持）。
  GPUI 側は `gpui-test` feature で `ruviform-ui-test` に依存する
  既存方向のみ。
- 新規依存なし（既存の `golden`/`serde_json` のみ使用）。
- evidence は `RUVIFORM_GOLDEN_EVIDENCE` env ゲート — 未設定で
  副作用なし（driver-lab/golden slice 1 と同じ運用）。

## Risks and Trade-offs

| リスク | 影響 | 対策 |
| --- | --- | --- |
| `capabilities()` に `VisualCapture` を宣言しないため、preflight を通した scenario が golden_check で `unsupported` report を受け取る二段構えになる | consumer が preflight pass = golden 実行可能と誤読 | `golden_check` の doc・spec・test で「preflight は宣言語彙、golden_check は実測 outcome」を明記。scenario report の `required`/`capabilities`/`observed.golden.verdict` で全て観測可能 |
| `CaptureOutcome`/`VisualGuiTest` が slice 3 の capture API と齟齬 | slice 3 で port 形状の手戻り | `capture_frame` は「target → RGBA8」の最小 primitive で、offscreen/実窓の差は adapter 内部に閉じる。slice 3 は outcome の `Captured` を返す実装を足すだけ |
| `record_golden_step` の `observed` JSON 形が consumer 期待と衝突 | observed 上書き | `observed` が object または `None` の時のみ `golden` key を挿入/生成し、非 object の consumer 値は保持。evidence 参照は常に追加 |
| Fake/GPUI で unsupported reason が異なると parity が崩れるように見える | test の誤読 | reason は adapter 固有の機械可読コードでよい — parity は verdict 語彙・evidence schema・report 形で取り、reason 文字列の一致は要求しない（test で明示） |

## D1: golden step は `GuiAction` ではなく `VisualGuiTest` の検証メソッド

- `GuiAction`/`perform` は `Result<(), ScenarioError>` を返し、
  view への semantic 操作を表す。golden check は `GoldenComparison`
  （verdict + stats + diff image）を返す検証ステップであり、
  action の語彙に押し込めると report が失われる。
- 別 trait `VisualGuiTest: SemanticGuiTest` にすることで、
  semantic-only の consumer は今まで通り `SemanticGuiTest` のみを
  bound にでき、visual を要求する scenario だけが新しい bound を
  取る — #464 の「能力を型で表す」方針と一致。

検討して退けた案:

| 案 | 退けた理由 |
| --- | --- |
| `GuiAction::GoldenCheck` variant | `perform` が verdict/report を返せない。action と verification の責務混在 |
| `SemanticGuiTest` に `golden_check` を直接追加 | semantic-only adapter に visual 責務を強制。supertrait 分離が #464 の能力語彙と整合 |
| `golden_check` を free function 化 | capture が adapter 固有のため trait method が必要。compare/evidence は既定実装で集約済み |

## D2: capture outcome を `Result<RgbaImage, ScenarioError>` にしない

- `ScenarioError::Unsupported{capability}` は capability のみを持ち
  reason を運べず、`NotRun`（前提未達）を表現する variant がない。
  `ScenarioError` に variant を足すと `Clone + PartialEq` derive を
  維持できない（`GoldenError` の `io::Error` と同じ問題）。
- `CaptureOutcome` は「測定された事実」を運ぶ値型であり、error
  ではない — outcome を `Result` の `Ok` に入れ、contract/tool
  error のみ `Err(ScenarioError)` にするのが #464 の
  error/verdict 分離と一致する。

## D3: `capabilities()` に `VisualCapture` を宣言しない（本 slice）

- `capabilities()` の契約は「adapter が **現在** 実際に提供できる
  証拠レベル」。capture backend が存在しない以上、宣言は
  honest-capability 契約（#464）違反であり、既存 test
  `gpui_capability_preflight_matches_fake` が
  `VisualCapture` の拒否を固定している。
- 「GPUI adapter では能力を宣言し」は、scenario/report の
  `required: {VisualCapture}` 宣言と `golden_check` の verdict
  path が GPUI 側でも動くことを指すと解釈する — adapter の
  `capabilities()` 申告ではない。
- slice 3 で capture 実装を入れる際に、adapter が実際に capture
  できることを確認した上で `capabilities()` へ追加する。

## D4: verdict 写像の所有者を接続層に置く

- `From<GoldenVerdict> for ScenarioVerdict` と
  `ScenarioVerdict::lane_status` により、5 値→5 値→lane 語彙の
  写像を各 consumer が手書きしない。`blocked`→`not_run` は
  「lane 語彙に `blocked` がない」ための既存規則（#390）を
  コード化したもので、意味は変えない。
- `lane_status` を `ScenarioVerdict` の method にするのは、
  この写像が scenario verdict 全般に効く規則であり、visual 層
  固有ではないため。

## D5: adapter impl の配置

- `impl VisualGuiTest for FakeGuiTest` は `scenario/fake.rs`、
  `impl VisualGuiTest for GpuiGuiTest` は
  `gpui_driver/scenario.rs` — 各 adapter file が自分の trait impl
  を持つ既存構造（`impl SemanticGuiTest` と同じ）に倣う。
- `capture_frame` の本体は pin 再検査 + honest outcome のみ。
  Fake は `fake_viewhost_has_no_pixel_pipeline`、GPUI は
  `gpui_test_platform_has_no_pixel_readback` を reason にする
  （adapter 固有の機械可読コード — parity は verdict 語彙で取る）。

## Verification Notes

- [x] `SemanticGuiTest`/`ScenarioCapability`/`ScenarioReport` の
  既存契約を変更しない（追加のみ — `lane_status` method 追加と
  `visual` module 新設）。
- [x] `golden` モジュールの公開 API を変更しない（利用のみ）。
- [x] `check-ui-driver-boundary` の forbidden signature 検査に
  適合する型のみを公開 signature に使う。
- [x] Fake/GPUI の既存 scenario_contract test が全て通る
  （capability 集合・preflight の非回帰）。
