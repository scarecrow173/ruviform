---
id: "SPEC-034UevQgR6YDvj7j5Urcrc"
type: "spec"
status: "proposed"
title: "Issue #314 slice 2 — Golden Utility と Scenario Contract の能力契約接続"
created: "2026-11-08"
updated: "2026-11-08"
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
        path: "docs/specs/issue-314-golden-scenario.md"
      - type: "file"
        path: "docs/designs/issue-314-golden-scenario.md"
      - type: "file"
        path: "docs/plans/issue-314-golden-scenario.md"
      - type: "file"
        path: "docs/tasks/issue-314-golden-scenario.md"
      - type: "file"
        path: "docs/test-specs/issue-314-golden-scenario.md"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/visual.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/fake.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/README.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-314-golden-core.md"
    - "docs/designs/issue-314-golden-core.md"
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  derives-from:
    - "docs/specs/issue-314-golden-core.md"
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-314-golden-scenario.md"
  related:
    - "docs/designs/issue-314-golden-core.md"
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
    - "docs/plans/issue-390-driver-contract-lab.md"
  refines: []
  refined-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 2 — Golden Utility と Scenario Contract の能力契約接続

## What Should Be Built

slice 1（`docs/specs/issue-314-golden-core.md`）で実装した headless
Golden Utility（`golden` モジュール: `GoldenKey`/`EnvironmentMeta`/
`RgbaImage`/`DiffPolicy`/`BaselineStore`/`GoldenReport`/evidence）を、
#464 の consumer-neutral Scenario Contract（`SemanticGuiTest`/
`ScenarioCapability`/`ScenarioReport`）の能力語彙に接続する。

具体的には:

- `SemanticGuiTest` を継承する visual/golden ポート
  （`VisualGuiTest` 相当）を `ruviform-ui-test` に定義し、target 単位の
  pixel capture 試行と golden 比較ステップを driver-neutral な型で
  表現する。
- capture 結果を `GoldenVerdict`（`pass`/`fail`/`not_run`/
  `unsupported`/`blocked`）へ正直に写像し、`ScenarioVerdict`・
  driver-lab/consumer-verify の lane 語彙（`blocked` =
  `not_run`+reason）との対応を一箇所に固定する。
- Fake adapter と GPUI adapter（`gpui-test`）の両方に同一の接続面を
  実装し、pixel capture backend が存在しない現状では
  `unsupported`（理由付き）を返す — semantic/draw-plan の証拠を
  pixel 証拠へ昇格させない。
- golden ステップ結果を `ScenarioReport` の evidence/observed に
  記録する canonical な経路を提供する。

## Why Now

- slice 1 は比較 utility のみを提供し、capture 経路（slice 3）も
  consumer 接続（slice 4 / #426）も未接続のまま。Scenario Contract
  （#464、PR #477 で main 入り済み）は能力宣言・preflight・report の
  語彙を既に持つため、両者をつなぐ契約層がないと slice 3 の capture
  実装がどこに挿さるべきかが不定になる。
- 「pixel capture を要求する scenario が、対応しない adapter で
  正直に `unsupported`/`not_run` を返す」経路を先に固定することで、
  slice 3 が capability 宣言と capture 実装を同時に追加できる。

## Users and Value

- consumer repo（Synvelis / AudioLens）の GUI scenario 作者:
  `VisualGuiTest` を bound に取る scenario 関数を書けば、Fake・GPUI・
  （将来の）実 capture adapter で同一コードが動く。
- driver-lab / consumer-verify の証拠集計: golden verdict が
  既存の 5 値語彙と lane 語彙にそのまま載り、`blocked`→`not_run`
  写像を壊さない。
- Issue #314 全体: slice 3（capture）・slice 4（#426 fixture 接続）
  の着地点を型で規定する。

## Scope

### In Scope

- `ruviform-ui-test` に visual/golden 接続モジュールを追加:
  capture 結果の正直な 3 値 outcome、`SemanticGuiTest` 継承の visual
  ポート、capability ゲートを通した golden compare ステップ、
  verdict 写像（`GoldenVerdict`→`ScenarioVerdict`、lane 語彙）、
  `ScenarioReport` への記録 helper、evidence emit。
- `FakeGuiTest` への visual ポート実装（capture 不可を
  `Unsupported` outcome で正直に返す）。
- `GpuiGuiTest`（`ruviform-ui-gpui`、`gpui-test` feature）への同一
  実装。public signature に GPUI 型を出さない。
- Fake/GPUI 両側の contract/parity test: 能力宣言・preflight・
  verdict 語彙・evidence 記録の一致。
- driver-lab manifest/evidence との整合確認（既存の
  `blocked`=`not_run`+reason 写像を維持）。

### Out of Scope

- pixel capture backend の実装（offscreen readback・実 Window
  capture・`TestIo::screenshot` の実 backend）— slice 3 / #313 系。
  本 slice では capture が存在しない経路の verdict のみを接続する。
- `capabilities()` への `VisualCapture` 追加 — capture 実装が無い
  adapter が能力を主張することは #464 の honest-capability 契約に
  反する。宣言は slice 3 で capture 実装と同時に行う。
- `crates/ruviform-ui/`（ViewHost 契約）・`Cargo.toml` メンバー定義・
  `.github/`・`.devin/` の変更。consumer repo・`ui/**`・`poc/**`。
- semantic oracle・OS capture 経路の実装、#426 Story fixture 接続
  （slice 4）。

## Requirements

- R1: visual ポートは `SemanticGuiTest` を supertrait に持ち、
  target（`ScenarioTarget`）単位で pixel capture を試行する。
  全ての型は `ruviform-ui`/`ruviform-ui-test` の driver-neutral な
  契約型であり、GPUI/wry/OS handle 型を公開 signature に含めない。
- R2: capture 試行は `attach_epoch`/`generation` pin を他の
  `SemanticGuiTest` 操作と同じく再検査し、stale target は
  `ScenarioError` を返す。capture 不能時に pixel を捏造しない。
- R3: capture 結果は `Captured(RgbaImage)`/`Unsupported{reason}`/
  `NotRun{reason}` の正直な outcome で表現し、golden verdict
  （`unsupported`/`not_run`）へ機械的に写像する。理由は機械可読な
  `snake_case` コード。
- R4: golden ステップは capability 宣言ではなく実測 outcome を
  authoritative とし、`BaselineStore::compare` の結果
  （`pass`/`fail`/`blocked`）と capture 層 verdict を統合した
  `GoldenComparison` を返す。`rvf.golden.evidence/v1` evidence を
  env ゲート付きで emit する。
- R5: `GoldenVerdict`→`ScenarioVerdict` は 5 値語彙で同一写像。
  consumer-verify lane 語彙への写像は `blocked`=`not_run`+reason
  （元 verdict と理由を detail に保持）を唯一の規則として固定する。
- R6: golden ステップ結果を `ScenarioReport` に記録する canonical
  helper を提供し、evidence 参照（canonical key）と observed JSON
  の形を一箇所に定義する。scenario verdict の最終判断は consumer
  に残す（golden `pass` は semantic 検証の `pass` を意味しない）。
- R7: Fake/GPUI の両 adapter が同一の visual ポートを実装し、
  capability 宣言（`capabilities()`）・preflight
  （`require_capabilities`）・golden verdict・evidence shape が
  一致する。semantic/draw-plan の証拠を pixel 証拠へ昇格させない。
- R8: 文書 — 本 slice の責務境界・verdict/evidence 意味論・
  slice 3 への引き継ぎを docs と crate README に記録する。

## Acceptance Criteria

| ID | 条件 | 検証 |
| --- | --- | --- |
| AC-1 | `VisualGuiTest` ポートが `SemanticGuiTest` を継承し、driver-neutral な型のみで構成される | signature レビュー + `mise run check-ui-driver-boundary` |
| AC-2 | capture outcome→golden verdict の写像が `unsupported`/`not_run`/`pass`/`fail`/`blocked` を正しく保持し、理由コードが report/evidence に残る | `scenario::visual` unit tests + Fake/GPUI contract tests |
| AC-3 | `golden_check` が stale target で `ScenarioError` を返し、capture 不能時に `unsupported` verdict の `GoldenComparison` を返す（`pass` にしない） | Fake/GPUI contract tests |
| AC-4 | `GoldenVerdict`→`ScenarioVerdict`・lane 語彙写像が `blocked`=`not_run`+reason を維持する | unit tests + driver-lab 整合確認 |
| AC-5 | `record_golden_step` が `ScenarioReport` に canonical な evidence/observed を記録し、verdict 判断を consumer に残す | unit tests |
| AC-6 | Fake/GPUI で能力宣言・preflight・verdict 語彙・evidence 記録が一致する | `mise run test-ui-scenario-contract`（両 lane） |
| AC-7 | 公開 signature に toolkit/GPUI 型が現れず、`ruviform-ui-test`→`ruviform-ui-gpui` の依存を作らない | `mise run check-ui-driver-boundary` + Cargo DAG |

## Deferred Design Concerns

- pixel capture backend（offscreen/Viz readback・実 Window capture）
  | 本 slice は契約接続のみ | slice 3 着手時 | capture 実装なしで契約だけが残ると slice 3 の差し込み位置が曖昧になる
- `VisualCapture` capability の宣言 | honest-capability 契約 | slice 3 で capture 実装と同時 | 宣言だけ先行すると preflight が嘘を通す
- #426 Story fixture 接続・AudioLens/Synvelis pixel 適用 | slice 4 の範囲 | slice 4 | —
- `ScenarioReport` の JSON schema 固定 | consumer-defined JSON の現行方針を維持 | consumer 側 schema 要求が出た時 | —

## Sources and Relations

- Issue #314（slice 2 の対象 Issue。Owner 範囲: pixel screenshot 取得・
  環境正規化・baseline 比較・差分承認・証拠）。
- Issue #464（consumer-neutral Scenario Contract — 本 slice が接続する
  契約層。PR #477 / merge `57e4e8ce` で main 入り済み）。
- Issue #390（Driver Contract Lab — 判定語彙・evidence 形式の先行
  owner。`blocked`=`not_run`+reason 写像の由来）。
- `docs/specs/issue-314-golden-core.md`・
  `docs/designs/issue-314-golden-core.md`（slice 1 — 接続対象の
  Golden Utility。PR #481 / merge `2230018e`）。
- `docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md`・
  `docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md`
  （Scenario Contract の正本）。
