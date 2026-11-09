---
id: "PLAN-Sv5LAPR5jiKCGifpkWxo3e"
type: "plan"
status: "approved"
title: "Issue #314 slice 5 — Golden 環境正規化・registry 結合・runner 統合・証拠 lane 実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-314-golden-evidence.md"
      - type: "file"
        path: "docs/tasks/issue-314-golden-evidence.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-314-golden-evidence.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-314-golden-evidence.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-314-golden-scenario.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-314-golden-evidence.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 5 — Golden 環境正規化・registry 結合・runner 統合・証拠 lane 実装計画 Implementation Plan

## Goal

`EnvironmentMeta` → `GoldenKey` env class の正規化規則を
`golden::normalize` に集約し、`StoryFixture`/`stories.json`
registry への結合・整合検査、`run_scenario`/`ScenarioOutcome`
への golden fold helper、evidence の `lane="pixel-golden"`
記録を additive に提供する。

## Scope

単一実装ストリーム。`ruviform-ui-test` 内の新規 module・
既存 module への additive 変更・test・文書チェーンを 1 slice
で完結させる。実 capture・consumer fixture・他 crate は
別 slice/別 issue。

## Upstream Documents

- `docs/specs/issue-314-golden-evidence.md`（spec — 本 plan の正本）
- `docs/specs/issue-314-golden-core.md`（slice 1 — Golden Utility）
- `docs/specs/issue-314-golden-scenario.md`（slice 2 — Scenario 接続）
- `docs/specs/issue-314-golden-capture.md`（slice 3 — capture backend）
- `docs/specs/issue-464-scenario-runner.md`（`run_scenario` 契約）
- `docs/specs/issue-390-driver-contract-lab.md`（manifest registry）
- `docs/specs/issue-426-gpui-lab-binding.md`（story fixture）

## Design Inputs

- `docs/designs/issue-314-golden-evidence.md`（approved）

## File Map

- `crates/ruviform-ui-test/src/golden/normalize.rs` — 新規。
  `GoldenKeyEnv`/`key_env`/`NormalizeError`/`key_for_fixture`/
  `registry_mismatches`/`RegistryMismatch` + unit tests。
- `crates/ruviform-ui-test/src/golden.rs` — `pub mod normalize;`
  + re-export のみ。
- `crates/ruviform-ui-test/src/golden/report.rs` —
  `GoldenEvidence.lane` field + `EVIDENCE_LANE` 定数 +
  `From<&GoldenReport>` での設定。
- `crates/ruviform-ui-test/src/scenario/visual.rs` —
  `record_golden_outcome` 追加、観測 JSON/merge の共有 helper
  抽出、module doc 更新（`run_scenario` 組み立て例）。
- `crates/ruviform-ui-test/tests/golden.rs` — normalize/
  registry/lane の integration test 追加。
- `crates/ruviform-ui-test/tests/scenario_contract.rs` —
  `run_scenario` + `golden_check` + `record_golden_outcome`
  の Fake adapter test 追加。
- `docs/{specs,designs,plans,tasks,test-specs}/issue-314-golden-evidence.md`
  — 本 slice の文書チェーン。
- `docs/*/README.md` — index 自動生成（`doc_maintenance apply`）。

## Implementation Sequence

- [ ] `normalize.rs` 新設: `GoldenKeyEnv`/`key_env`/
      `NormalizeError`/`key_for_fixture`/`registry_mismatches`/
      `RegistryMismatch` + unit tests（実 `stories.json` を
      `CARGO_MANIFEST_DIR` 相対で読む）。
- [ ] `golden.rs` wiring: mod 宣言 + re-export。
- [ ] `report.rs`: `GoldenEvidence.lane` + `EVIDENCE_LANE`。
- [ ] `visual.rs`: `record_golden_outcome` + 共有 helper 抽出 +
      doc 更新。
- [ ] `tests/golden.rs`/`tests/scenario_contract.rs` へ test 追加。
- [ ] 文書チェーン記述 + index 再生成。
- [ ] `git fetch origin main && git merge origin/main` で最新化。
- [ ] mise canonical 検証一式。
- [ ] review-protocol レビュー。
- [ ] commit / push / PR / worker_done。

## Dependencies and Risks

- slice 1〜3（`2230018e`/`7ae916b3`/`dc01934f`）と #464 runner
  slice（`967b6fd8`、PR #511）が main 入り済みであることが前提。
- `golden` → `lab_story`/`driver_lab` の crate 内依存追加 —
  `driver_lab` は既に利用済み、`lab_story` は新規だが循環なし。
- `record_golden_outcome` は `ScenarioOutcome`（runner slice）
  に依存 — PR #511 merge 済みであることを前提にする。
- `stories.json` の `scaleFactor`/`states`/`themes` 宣言と
  `lab_story` fixture の drift は `driver_contract.rs` が既に
  検査 — 本 slice の `registry_mismatches` は key 単位の監査で
  あり manifest 自体の整合は既存検査に委ねる。

## Task Breakdown

- TASK-aX702E4kUPjRWUHnHKK82B（`docs/tasks/issue-314-golden-evidence.md`）
  — 本 slice 全体を 1 task として管理。

## Verification

- [ ] `mise run fmt-check` / `check` / `lint` / `test`
- [ ] `mise run test-ui-golden` — Golden Utility 非回帰 + 追加分
- [ ] `mise run test-ui-scenario-contract` /
      `test-ui-scenario-contract-fake` /
      `test-ui-scenario-contract-gpui` — adapter 非回帰
- [ ] `mise run test-ui-driver-contract` — manifest 非回帰
- [ ] `mise run check-ui-driver-boundary` — 公開 signature の
      driver-neutral 性
- [ ] `mise run test-ui-test-msrv` — MSRV 1.89

## Open Questions / Gaps

- `EnvironmentMeta` への `locale` 追加は pub struct 変更を伴い
  `ruviform-ui-gpui` の既存 literal を壊すため本 slice 対象外
  （Deferred Design Concerns 参照）。
- draw plan 比較の verdict 型は GPUI/consumer 側 — 本 slice は
  lane 分離の固定のみ。
- consumer product fixture への実接続は slice 4（#426 系）/
  consumer 側後続。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（Orca dispatched worker）。
- Independent work streams: none。
- User approval required before dispatch: yes（coordinator dispatch
  済み）。
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- `key_env` の gpu_class fallback（class → vendor → None）が
  「class のみ key に入れる」原則と整合するか。
- `registry_mismatches` の scale_factor 完全一致（f64 `==`）が
  適切か（epsilon を入れない判断）。
- `record_golden_outcome` が `record_golden_step` と共有 helper
  を持つ設計の妥当性。
- `lane="pixel-golden"` の語彙が #262/#313 の gate 分類と
  矛盾しないか。
