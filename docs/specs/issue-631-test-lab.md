---
id: "SPEC-034YaXUTzcBvabDver3uDY"
type: "spec"
status: "approved"
title: "Issue #631 GPUI Test Lab / Scenario Console GUI v1 + mise lane 簿記"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-631-scenario-catalog.md"
    - "docs/specs/issue-631-scenario-console.md"
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "docs/specs/issue-631-real-window-dock.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-464-scenario-runner.md"
    - "docs/designs/issue-631-scenario-console.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-631-test-lab.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 GPUI Test Lab / Scenario Console GUI v1 + mise lane 簿記

## What Should Be Built

Issue #631（tracker）の後続 slice。merge 済み scenario catalog
（`HEADLESS_MATRIX`・`real_catalog::REAL_CATALOG`）を driver-neutral に
投影する **GPUI 製 windowed Test Lab / Scenario Console GUI v1** と、
`mise.toml` の #631 lane 簿記 3 点を同一 slice で納品する。

- GUI: `ruviform-plugin-dock-test` に feature-gated bin
  `plugin-dock-lab`（新 feature `lab-gui`）を追加し、ruviform-ui の
  `ViewSpec`/Component vocabulary + `GpuiDriver` で構築する実 Window
  アプリ。catalog 一覧（capability projection — verdict なし）・
  selector 実行（headless cell のみ、`run_selected` 経由）・
  verdict/evidence 表示を持つ。
- mise: `test-ui-plugin-dock-console` task 新設（headless console の
  canonical 入口）、`test-ui-plugin-dock-lab` opt-in windowed lane
  新設、`test-ui-plugin-dock-real` の stale description
  （`test_gain` 前提 → `test_editor` 実態）修正。

## Why Now

headless console（`plugin-dock-console`）は catalog の headless
consumer として merge 済みだが、GUI からの dogfooding 経路がない。
Issue #631 AC2/AC6 の ruviform 側分 — 「Runner から Test Lab GUI へ
Driver-neutral に scenario を投影」「GUI/Test Lab 自身が Ruviform
共通 UI で作られ GPUI 固有型を consumer へ漏らさない」— を満たす
には、共通 UI vocabulary で構築した windowed console が必要。
あわせて canonical な mise 入口が未登録の console と、opt-in である
べき lab lane の簿記を揃える。

## Users and Value

- Ruviform 開発者・test lane 運用者: 実 Window 上で catalog
  projection・selector 実行・verdict/evidence を対話的に確認できる。
- dogfooding 証拠: ViewSpec/Table/Button/Tabs の実 window 描画・
  intent→receipt 経路が lane 自体の動作証拠になる。
- verdict の正直性: headless/fake の結果を実 Window・実 OS の Pass
  に昇格しない規範を GUI 表示・lane 記録の両面で維持する。

## Scope

### In Scope

- `crates/ruviform-plugin-dock-test`: `lab-gui` feature、
  `src/bin/plugin-dock-lab.rs`、必要な lib 公開面の最小拡張
  （blocked report 構築に必要な範囲のみ）。
- `mise.toml`: #631 lane 節への task 追加 2 件と
  `test-ui-plugin-dock-real` description 修正。
- `docs/development-environment.md` の lane 一覧更新、
  `evidence/rf631/verification.md` への本 slice 節追記、
  doc chain（本書群）と impl 記録。

### Out of Scope

- Synvelis 実 GPUI Shell・AudioLens 実 GUI への scenario 投影と製品
  oracle 照合（consumer 側 gate）。
- real catalog case の拡充（2Window/DPI/IME）と real lane の GUI
  経由実行 — real 実行は既存 `test-ui-plugin-dock-real` に委譲し、
  lab app は real case を projection/selector 表示のみ行う。
- DComp pixel 判定（`scarecrow173/ruviform#532`）— Fail/NotRun 維持。
- catalog projection API の拡張（filter/sort/group 等の richer
  projection は Deferred Design Concerns へ）。
- `docs/**/README.md` index 再生成（Coordinator が統合時に実施）。
- `*-msrv` lane・3OS 対応（macOS/Linux GUI は Deferred）・CI 組込み。

## Requirements

- GUI は `HEADLESS_MATRIX.project()`/`cells()` と
  `real_catalog::REAL_CATALOG` のみを catalog 情報源とし、case 一覧を
  複製しない。実行は `run_selected`（= `run_entry`/`run_scenario`
  経路）の consumer であり、独自 executor・2 つ目 runner を作らない。
- verdict 語彙（Pass/Fail/NotRun/Unsupported/Blocked）を変えず、
  headless/fake の成功を real-window/native-dock/editor/pixel 証拠に
  昇格しない。real lane の case は GUI 上で session-required と表示し
  実行しない。
- GPUI 固有型を公開 API・evidence JSONL に出さない
  （`check-ui-driver-boundary` green 維持）。lab app は既存登録済み
  composition root（`ruviform-plugin-dock-test`）内に置き、
  `boundary.json` 変更は行わない。
- windowed lane は interactive session 前提の manual/opt-in。
  headless/非対話 host では `environment_blocked` → 全 catalog row
  `NotRun` を JSONL に記録して exit 0（fail-closed・偽 pass なし）。
- console lane は `plugin-dock-console --list`/`--run` の canonical
  入口として exit 0/1/2 意味論を description に明記する。

## Acceptance Criteria

- [ ] `mise run test-ui-plugin-dock-lab` が feature `lab-gui` の
  `plugin-dock-lab` bin を起動し、実 Window で catalog 一覧・
  selector 実行・verdict/evidence を表示する（interactive session）。
- [ ] headless/非対話 host での lab lane 起動が全 catalog row
  `NotRun` + `observed.environment_blocked` の JSONL を
  `evidence/rf631/` に出力し exit 0 で終了する。
- [ ] `mise run test-ui-plugin-dock-console` が `--list`/`--run` を
  canonical 実行し exit 0/1/2 意味論どおりに振る舞う。
- [ ] `test-ui-plugin-dock-real` の description/コメントが
  `test_editor` 実態と一致する（`test_gain` 前提文言の除去）。
- [ ] `mise run test-ui-plugin-dock` の verdict 表が不変
  （headless lane 非回帰）。
- [ ] `check`/`lint`/`test`（default・`real-dock`・`lab-gui`）・
  `check-ui-driver-boundary`・`prek run --all-files` が green。
- [ ] GUI が ruviform-ui の ViewSpec/Component vocabulary のみで
  構築され、GPUI 型・Win32 型が consumer 公開面に現れない。

## Deferred Design Concerns

- richer catalog projection（filter/sort/group・per-driver live query）
  | v1 は project()/cells() の flat listing で要件を満たす。GUI slice
  で要件確定後に拡張 | scenario-catalog design の Deferred Design
  Concerns | listing が大きくなった時点で再検討 |
- GUI からの real lane 実行導線 | 2 つ目 real executor 禁止の制約上、
  実行は opt-in lane に委譲 | real catalog case 拡充 slice |
  launcher なしでも lane 直接実行で代替可能 |
- evidence field 拡充（screenshot/PNG・golden diff・semantic tree・
  OS/GPU/seed field） | 別 slice の担当範囲 | 後続 evidence slice |

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — 本 slice では close
  しない）。
- merge 済み前提: `HEADLESS_MATRIX`/`REAL_CATALOG`・
  `ScenarioMatrix::{cells,project}`・`run_entry`/`run_entries`
  （PR `#662` 系）、`plugin-dock-console` bin（PR `#665` 系）、
  `RealOsGuiTest` real lane（PR `#660` 系）、`test_editor` fixture
  （PR `#670`）。
- dogfooding 素材: `plugin_dock_view_spec()`・driver_lab stories・
  `windowed_test_app.rs`（uitest AUT）・`lab_binding.rs`。
- `docs/designs/issue-464-scenario-runner.md` L144-148 の制約
  （JSON DSL・独自 executor・2 つ目 runner 禁止）。
- `docs/discovery/issue-631-test-lab.md`（本 slice の調査記録）。
