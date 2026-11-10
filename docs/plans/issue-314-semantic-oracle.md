---
id: "PLAN-034V2hp0I1d6mdR2ya94bO"
type: "plan"
status: "approved"
title: "Issue #314 slice 6 — Consumer-shaped semantic oracle fixtures 実装計画"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-314-semantic-oracle.md"
      - type: "file"
        path: "docs/tasks/issue-314-semantic-oracle.md"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-audiolens.json"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-synvelis.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-314-semantic-oracle.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-314-semantic-oracle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-314-golden-evidence.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-314-semantic-oracle.md"
    - "docs/designs/issue-314-semantic-oracle.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-314-semantic-oracle.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 6 — Consumer-shaped semantic oracle fixtures 実装計画 Implementation Plan

## Goal

driver-lab parity fixture set に AudioLens/Synvelis 形状の
semantic oracle fixture 2 件を追加し、Fake payload 保持 lane と
GPUI drawn-plan lane の両方で同一 `expect` oracle を検証する。

## Scope

単一実装ストリーム。fixture JSON 2 件・既存 test ファイルへの
additive test 追加・文書チェーンを 1 slice で完結させる。
新規検査機構・公開 API・consumer theme infra・pixel golden/OS
input lane は対象外。

## Upstream Documents

- `docs/specs/issue-314-semantic-oracle.md`（spec — 本 plan の正本）
- `docs/specs/issue-314-golden-core.md`（slice 1 — lane 語彙の基盤）
- `docs/specs/issue-314-golden-evidence.md`（slice 5 — #262/#313
  リンク構造と lane 分離）
- `docs/specs/issue-426-gpui-lab-binding.md`（story fixture registry
  — slice 4 予約 scope、参照のみ）
- `docs/specs/issue-464-scenario-runner.md`（Scenario Contract）

## Design Inputs

- `docs/designs/issue-314-semantic-oracle.md`（approved）

## File Map

- `crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-audiolens.json`
  — AudioLens 形状 oracle。Waveform（f64/長尺/partial/gap/
  viewport 外）+ TimeAxis（fractional/viewport 外 tick）+
  Seek（committed plain/normalized）。
- `crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-synvelis.json`
  — Synvelis 形状 oracle。Meter（RT-like level/partial/gap/
  over-range/clamp/cull）+ Spectrogram（gap column/range 外
  magnitude/clamp/cull）+ TimeAxis。
- `crates/ruviform-ui-test/tests/driver_parity.rs` —
  `parity_viz_semantic_{audiolens,synvelis}_payload_retained_fake`
  追加。payload verbatim 保持 + mount 受理 key + Seek committed。
- `crates/ruviform-ui-gpui/tests/consumer_observation.rs` —
  `observes_viz_semantic_{audiolens,synvelis}_oracle_through_seam`
  追加。`GpuiObserver` seam 経由の mounted projection +
  `DrawnPlan` 一致。
- `docs/{specs,designs,plans,tasks,test-specs}/issue-314-semantic-oracle.md`
  — 本 slice の文書チェーン。
- `docs/*/README.md` — index 自動生成（`doc_maintenance apply`）。

## Implementation Sequence

- [x] `viz-semantic-audiolens.json` 新設（view + expect、
      description に lane 境界/NaN 表現/serde_json 制約/非対象
      領域を記述）。
- [x] `viz-semantic-synvelis.json` 新設（同上）。
- [x] `driver_parity.rs` に Fake payload 保持 test 2 件追加。
- [x] `consumer_observation.rs` に drawn-plan 一致 test 2 件追加。
- [x] serde_json/`str::parse` の f64 bit 一致を実測し、bit-exact
      oracle の小数を両者一致の値に限定（`-0.1234567890123456`
      等）。
- [x] 文書チェーン記述 + index 再生成。
- [x] `git fetch origin main` で最新化確認（branch = origin/main）。
- [x] mise canonical 検証一式。
- [x] 実機 lane（`golden_capture --ignored`）の実測と証拠記録
      （Windows 実 desktop で 3 passed、`gpui-test` では
      `unsupported` を確認）。
- [ ] commit / push / PR / worker_done。

## Dependencies and Risks

- slice 1〜5（`2230018e`/`7ae916b3`/`dc01934f`/`b0d06328` 系）が
  main 入り済みであることが前提 — `GpuiObserver` seam・
  `DrawnPlan`・parity fixture 構造は既存。
- `consumer_observation.rs` は `required-features = ["gpui-test"]`
  の test target — `mise run lint`/`check` の workspace
  `--all-targets` では compile されないため、clippy/doc lint の
  検証は `test-ui-gpui`（feature 付き test build）で担保する。
- `golden_capture.rs` の `#[ignore]` real-window test は対話
  desktop session を要する — 本 slice の fixture 追加とは独立に
  既存 lane の実測可否を記録する。
- fixture の `expect.*.drawn*` は `DrawnPlan` contract 面のみを
  固定し、plan 生成内部には依存しない。

## Task Breakdown

- TASK-034V2hqLGYpcCEu3a3OqFm（`docs/tasks/issue-314-semantic-oracle.md`）
  — 本 slice 全体を 1 task として管理。

## Verification

- [x] `mise run fmt-check` / `check` / `lint` / `test`
- [x] `mise run test-ui-parity` — Fake 側新規 test 含む
- [x] `mise run test-ui-gpui` — `consumer_observation` 新規 test
      含む（gpui-test feature）
- [x] `mise run test-ui-scenario-contract` / `test-ui-golden` /
      `test-ui-driver-contract` / `check-ui-driver-boundary` —
      非回帰
- [x] `mise run test-rt` — 非回帰
- [x] `cargo test -p ruviform-ui-gpui --features gpui --test
      golden_capture -- --ignored --test-threads=1` — Windows 11
      x64 実 desktop で 3 passed（PrintWindow/BitBlt、blocked →
      promote → pass/match）。`--features gpui-test` では
      `unsupported`（no OS handle）を確認 — capability 分離の
      実測証拠

## Open Questions / Gaps

- consumer story fixture（#426 `lab_story`）の pixel golden 接続
  は slice 4 予約 scope であり本 slice 対象外。
- 本 fixture view 自体の実 Window pixel baseline は未整備 —
  `golden_capture.rs` は lab_story fixture を対象とする。
- NaN 個別値・stale/LOD/Revision の oracle は既存 lane の責務。

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

- `expect.*.drawn*` が `DrawnPlan` contract 面のみを固定し、
  plan 内部実装に依存しない設計の妥当性。
- Waveform の viewport 非 cull を「現行 contract の正直な記述」
  として固定する判断（fixture description に明記済み）。
- serde_json/`str::parse` の丸め差を fixture 小数選定で回避する
  方針（bit-exact oracle の前提条件として文書化済み）。
- Fake payload 検査と GPUI plan 検査の 2 段構成が「payload 保持」
  と「描画計画」を混同しないこと。
