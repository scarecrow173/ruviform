---
id: "TSPEC-wOxWNRLHg5C2rotQihPFys"
type: "test-spec"
status: "approved"
title: "Issue #464 slice — Scenario Runner・Report 組み立て契約のテスト仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/464"
  changes: {}
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-464-scenario-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-464-scenario-runner.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-464-scenario-runner.md"
    - "docs/designs/issue-464-scenario-runner.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #464 slice — Scenario Runner・Report 組み立て契約のテスト仕様

## Feature: run_scenario による scenario 実行と report 組み立て

### Rule: capability pre-flight は本体を実行しない

- Example: `spec.required` に adapter が持たない
  `ScenarioCapability`（例: `VisualCapture` on `FakeGuiTest`）を
  含めて `run_scenario` を呼ぶと、body クロージャは一度も実行
  されず、返る `ScenarioReport.verdict ==
  ScenarioVerdict::Unsupported`、`detail` が欠落 capability を
  示し、`driver`/`capabilities`/`required` が記録される。
- 検証: `runner.rs` unit test `preflight_unsupported_skips_body`。

### Rule: ScenarioError は verdict に正直に写像される

- Example: body が `Err(ScenarioError::StaleGeneration{..})` を
  返すと `verdict == Fail` で `detail` に error 表示。
  `Err(ScenarioError::Unsupported{..})` は `Unsupported` に
  写像される。
- 検証: unit test `body_error_maps_to_verdict`。

### Rule: consumer の verdict はそのまま記録される

- Example: body が `Ok(ScenarioOutcome{verdict: NotRun, ..})`
  を返すと report.verdict は `NotRun` のまま — runner は
  pass/fail を勝手に決めない。
- 検証: unit test `outcome_verdict_is_recorded`。

### Rule: timeout 予算超過の Pass は Fail に落ちる

- Example: `spec.timeout_ms = Some(0)` で body が時間を使う
  （または elapsed が budget を超える構成で）`Pass` を返すと、
  report.verdict は `Fail`、`report.timeout.timed_out == true`、
  `elapsed_ms > budget_ms`。budget 内の `Pass` は維持される。
- 検証: unit test `timeout_exceeded_downgrades_pass`。

### Rule: settle_until は bounded で read-only に待つ

- Example: receipt/projection 投入後に `observe` の phase が
  目的状態になるまで `settle_until(gui, |g| ..., max)` を呼ぶと、
  条件成立で `Ok(true)`、`max_settles` 到達で `Ok(false)`。
  `settle` が `Err` なら `Err` が伝播する。sleep は使わない。
- 検証: unit test `settle_until_*`。

### Rule: ScenarioReport は timeout/cleanup を記録できる

- Example: `run_scenario` が `timeout_ms` 宣言時に
  `report.timeout = Some(ScenarioTimeout{budget_ms, elapsed_ms,
  timed_out})` を記録し、`report.cleanup` は consumer/driver が
  run 後に `Option<serde_json::Value>` として記録できる。
- 検証: unit test `report_records_timeout` + field 存在の
  compile 確認。

## Guarantees

- pre-flight `Unsupported` では body の副作用が一切発生しない。
- verdict 語彙は `pass`/`fail`/`not_run`/`unsupported`/`blocked`
  のまま — runner は新しい verdict を導入しない。
- `settle_until` の `ready` 述語は `&G` 経由の read-only 観測
  のみで、状態を変更できない。
- 公開 API に GPUI/adapter 型が現れない
  （`check-ui-driver-boundary` が pass）。

## Non-goals

- Real OS driver の Ready/Visible/Input 完了条件・強制 timeout
  （#313 Owner）。
- consumer 側 scenario の実適用（Synvelis #114 / AudioLens #166）。
- `tests/` 配下の contract test 追加（本 slice の検証は
  `runner.rs` unit test に閉じる）。
