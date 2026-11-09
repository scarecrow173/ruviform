---
id: "SPEC-Ghn42uyLNbGNp71h5dfWyP"
type: "spec"
status: "approved"
title: "Issue #464 slice — Scenario Runner・Report 組み立て契約（ruviform-ui-test）"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/464"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-464-scenario-runner.md"
      - type: "file"
        path: "docs/designs/issue-464-scenario-runner.md"
      - type: "file"
        path: "docs/plans/issue-464-scenario-runner.md"
      - type: "file"
        path: "docs/tasks/issue-464-scenario-runner.md"
      - type: "file"
        path: "docs/test-specs/issue-464-scenario-runner.md"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/runner.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-314-golden-scenario.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-464-scenario-runner.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #464 slice — Scenario Runner・Report 組み立て契約（ruviform-ui-test）

## What Should Be Built

PR #477 で main 入りした Scenario Contract（`SemanticGuiTest`/
`TargetQuery`/`ScenarioCapability`/`GuiAction`/`ScenarioReport`）の
上に、Issue #464 の「Runner 組み立て」「Result の最小契約」
「能力の実行前照合」「timeout/cleanup 記録」を `ruviform-ui-test`
内で完結する additive な契約として追加する。

具体的には:

- `ScenarioSpec`: 1 回の実行が宣言する scenario/case/run ID・
  必要 `ScenarioCapability` 集合・環境情報・任意の timeout 予算。
- `ScenarioOutcome`: scenario 本体（consumer の普通の Rust 関数）
  が返す verdict・expected/observed・触れた target・receipt・
  evidence・detail。
- `run_scenario`: `&mut impl SemanticGuiTest` と `ScenarioSpec` と
  本体クロージャを受け取り、capability pre-flight（不足は
  `Unsupported` verdict の report を返し本体を実行しない）→
  本体実行 → `ScenarioError` の verdict 写像 → `ScenarioReport`
  組み立てまでを行う、ただの Rust 関数。独自 executor・DSL・
  第二 Runner ではない。
- `settle_until`: bounded な settle 繰り返しで「receipt/revision
  条件が成立したか」を判定する待機 helper（Issue #464 詳細 7 —
  一律 sleep を使わない）。
- `ScenarioReport` への `timeout`/`cleanup` 記録 field 追加
  （Issue #464 詳細 6 の report 要求項目の未充足分）。

## Why Now

- Contract・adapter・verdict 語彙は揃ったが、report 組み立て・
  capability pre-flight・timeout 記録は consumer ごとの手作業に
  なっており、Issue #464 詳細 4（Runner 組み立て）・6（レポート）
  の要求を満たす共有経路がない。
- consumer（Synvelis #114 / AudioLens #166）が同じ report 形・
  同じ pre-flight 判定・同じ待機規律を使えるよう、契約側に
  canonical な 1 関数を置く。

## Users and Value

- consumer repo の GUI scenario 作者: `#[test]` の composition
  root で adapter（Fake / GPUI simulated / Real OS）を構築し、
  `run_scenario` に scenario 関数を渡すだけで verdict/report が
  得られる。adapter 固有の組み立ては各 Owner の既存実装
  （`FakeGuiTest`・`GpuiGuiTest`・`TestDriver`）に留まる。
- 証拠集計（driver-lab / consumer-verify）: report が
  timeout/cleanup を含む Issue #464 の全記録項目を持ち、
  `Unsupported`/`NotRun`/`Blocked` が pass に昇格しない構造を
  保つ。

## Scope

### In Scope

- `crates/ruviform-ui-test/src/scenario/runner.rs` 新設
  （`ScenarioSpec`/`ScenarioOutcome`/`ScenarioTimeout`/
  `run_scenario`/`settle_until`）。
- `ScenarioReport` への `timeout: Option<ScenarioTimeout>`・
  `cleanup: Option<serde_json::Value>` 追加と `new` の初期化。
- `scenario.rs` の `pub mod runner` 公開と `lib.rs` の re-export。
- runner の unit test（`runner.rs` 内 `#[cfg(test)]`）。
- docs chain（spec/design/plan/task/test-spec）。

### Out of Scope

- `crates/ruviform-ui/**`・`crates/ruviform-ui-gpui/**`・
  `crates/ruviform-observability/**`（別 dispatch の担当範囲）。
- consumer 側 scenario/oracle の採用（Synvelis #114 / AudioLens
  #166 — Step C）。
- 実 OS 入力・IME・pixel Golden・Native Plugin Dock の個別
  Case 実装（#313/#314/#388/#389 の Owner 領域）。
- `tests/` 配下の変更・`Cargo.toml` 依存追加（不要な場合）。
- 既存 `TestDriver`/`driver`/`driver_lab`/`proto` の内部改変
  （再利用のみ）。

## Requirements

- `run_scenario` は pre-flight で `spec.required` が
  `gui.capabilities()` の部分集合でなければ本体を実行せず、
  `ScenarioVerdict::Unsupported` の report を返す。
- 本体の `Ok(ScenarioOutcome)` は consumer が選んだ verdict を
  そのまま記録する（pass 昇格・降格の暗黙変換をしない）。
  ただし `timeout_ms` 予算を超過した実行が `Pass` を名乗ることは
  許さず `Fail` へ落とす。
- 本体の `Err(ScenarioError)` は `Unsupported` →
  `ScenarioVerdict::Unsupported`、それ以外 → `Fail` に写像し、
  `detail` に error 表示を残す。
- `ScenarioReport` は driver/capabilities/required/scenario/case/
  run_id/environment/targets/expected/observed/receipts/evidence/
  verdict/detail/timeout/cleanup を保持する。
- `settle_until` は `max_settles` 回を上限とする bounded な
  settle で、`ready` 述語（read-only 観測）が真になった時点で
  `true`、上限到達で `false` を返す。一律 sleep・無期限待機を
  提供しない。
- 新規公開型は既存の driver-neutral 型のみで構成し、GPUI 型・
  adapter 固有型を露出しない。公開 enum を追加する場合は
  `#[non_exhaustive]`。

## Acceptance Criteria

- [ ] `ScenarioSpec`/`ScenarioOutcome`/`ScenarioTimeout`/
  `run_scenario`/`settle_until` が `ruviform-ui-test` に存在し、
  JSON DSL・独自 executor を含まない。
- [ ] capability 不足の scenario が pre-flight で
  `Unsupported` verdict の report を返し本体を実行しない test
  がある。
- [ ] `ScenarioError` が verdict に正直に写像される
  （`Unsupported`→`unsupported`、その他→`fail`+detail）test が
  ある。
- [ ] timeout 予算超過の `Pass` が `Fail` に落ちる test がある。
- [ ] `settle_until` が bounded に動作し read-only 観測で
  receipt/revision 条件を判定できる test がある。
- [ ] `ScenarioReport` が timeout/cleanup を記録できる。
- [ ] `mise run fmt-check`/`check`/`lint`/`test` と
  `test-ui-scenario-contract` lane が pass。

## Deferred Design Concerns

- Real OS driver（#313）側の Ready/Visible/Input 完了条件と
  明示 timeout の実装 — 本 slice は report の記録 field と
  semantic 側の bounded settle のみを提供する。
- `ScenarioOutcome` への timeout/cleanup 記録の consumer 側
  記述例 — consumer 採用 Issue で確定する。

## Sources and Relations

- Issue #464 本文（詳細 1–8・受入条件）。
- `docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md`:
  Scenario Contract 全体の正本 spec。
- `docs/specs/issue-314-golden-scenario.md`: slice 分割と
  capability 契約接続の先例。
