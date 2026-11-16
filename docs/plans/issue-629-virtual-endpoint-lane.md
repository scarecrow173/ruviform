---
id: "PLAN-034YnADuTW0N5PDeaJtW36"
type: "plan"
status: "approved"
title: "Issue #629 virtual endpoint lane registration plan"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-629-virtual-endpoint-lane.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-virtual-endpoint-lane.md"
    - "docs/tasks/issue-629-virtual-endpoint-lane.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-virtual-endpoint-lane.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 virtual endpoint lane registration plan

## Goal

Issue #629 5th slice: `test-audio-virtual-endpoint` opt-in lane
の `mise.toml` 登録、stale 参照の同期、#628 Test Runtime 結線の
headless 検証（`EnvironmentKind` 語彙補完を含む）、lane
self-run の fail-closed evidence 記録を完了する。

## Scope

単一 implementation stream。lane 登録・参照同期・metadata
additive 変更・docs chain・evidence は同一 PR で論理コミットに
分けて納める。実 provisioning・lane の CI 統合・Test Runtime
実結線は後続 slice へ hand-off（spec Out of Scope と同一境界）。

## Upstream Documents

- `docs/specs/issue-629-virtual-endpoint-lane.md`
- `docs/designs/issue-629-virtual-endpoint-lane.md`（approved）
- `docs/designs/issue-629-wasapi-endpoint.md`（4th slice —
  AC6 lane 設計記録の正本）
- `docs/discovery/issue-629-virtual-endpoint-survey.md`
- `POLICIES.md` / Issue `scarecrow173/ruviform#627`（Case
  schema・`environment_kind`）/ `scarecrow173/ruviform#628`
  （Test Runtime）

## Design Inputs

- `docs/designs/issue-629-virtual-endpoint-lane.md` — lane 構成・
  lint coverage 判断・stale 参照の同期先一覧・
  `EnvironmentKind` additive 方針・verdict→RunMetadata 対応
  付け。

## File Map

- `mise.toml` - `[tasks.test-audio-virtual-endpoint]` 追加
  （`lint-audio-real-input` 直後）。
- `docs/development-environment.md` - lane 一覧表の行追加 +
  opt-in lane 注記 bullet 追加。
- `crates/ruviform-audio-device/tests/virtual_endpoint.rs` -
  module doc の canonical lane 参照更新。
- `crates/ruviform-audio-device/examples/endpoint_probe.rs` -
  verdict streaming proof 文言の canonical lane 名更新。
- `docs/designs/issue-629-wasapi-endpoint.md` - AC6 lane 関連
  記述の同期（Deferred/Verification Notes/NFC lane 項目）。
- `docs/discovery/issue-629-virtual-endpoint-survey.md` -
  Evidence 節へ 5th slice dated entry 追記。
- `crates/ruviform-audio-test/src/metadata.rs` -
  `EnvironmentKind` に `VirtualOsEndpoint`・`PhysicalDevice`
  を additive 追加（`as_str`・doc comment 含む）。
- `crates/ruviform-audio-test/tests/metadata.rs` - serde
  round-trip 配列へ 2 variant 追加 + schema 語彙 literal pin
  テスト新設。
- `docs/specs|designs|plans|tasks|test-specs/
  issue-629-virtual-endpoint-lane.md` + `docs/impl/ir|exp/
  issue-629-virtual-endpoint-lane.md`（+ `*-exp.jsonl`）- doc
  chain。
- `evidence/rf629/` - lane self-run 記録
  （`virtual_endpoint_slice5_lane_selfrun-windows-local.txt`）
  + probe 再実行による
  `endpoint_probe-windows-local.txt` 更新。

## Implementation Sequence

- [ ] T1: `mise.toml` lane 登録 + `docs/development-environment.
  md` 追記。
- [ ] T2: stale 参照同期 — `virtual_endpoint.rs`・
  `endpoint_probe.rs`・wasapi-endpoint design・survey doc。
- [ ] T3: `EnvironmentKind` additive variant + テスト更新 +
  #628 結線 IF 照合結果の docs 記録。
- [ ] T4: 検証 — fmt/check/lint/test（両 crate）・
  `lint-audio-real-input`・lane self-run・probe 再実行・`prek`・
  evidence 記録。
- [ ] T5: docs 仕上げ — impl ir/exp・exp jsonl・review-protocol。

## Dependencies and Risks

- `ruviform-audio-device` → `ruviform-audio-test` dev-dep は
  coordinator 不承認 — typed 実結線は行わず結線 IF を docs
  記録のみ。verdict→RunMetadata 対応付けは design に記録。
- `EnvironmentKind` は additive のみ — 既存 variant・serde
  語彙・API・`Cargo.toml`/`Cargo.lock` 無変更。
  `RealAbiTestPlugin`↔`test-plugin` の語彙差は既知 gap として
  記録（Coordinator 回答 (A)）。
- lane self-run は未 provisioning で `verdict: NotRun` —
  Pass を主張しない。evidence は lane 実行ログをそのまま保存
  （`--nocapture` は canonical `run` に含めない）。
- survey doc Evidence 節は append-only — 既存 entry を編集せず
  新規 dated entry で記録。
- docs index（`docs/**/README.md`）は生成物 — 本 slice では
  再生成せず Coordinator 統合時に委譲（PR 本文に明記）。

## Task Breakdown

- T1 → T2 → T3 は独立だが同一 PR。T4 は T1–T3 完了後。T5 は
  全検証後。

## Verification

- [ ] `mise run fmt-check` / `mise run check
  ruviform-audio-device` / `mise run lint
  ruviform-audio-device` / `mise run test
  ruviform-audio-device` — Pass。
- [ ] `mise run check|lint|test ruviform-audio-test` — Pass。
- [ ] `mise run lint-audio-real-input`（`--all-targets
  --features real-device` の `-D warnings`）— Pass。
- [ ] `mise run test-audio-virtual-endpoint` — exit 0 +
  `verdict: NotRun` を `evidence/rf629/` に記録。
- [ ] `mise exec -- cargo run -p ruviform-audio-device
  --example endpoint_probe`（Windows 実機）exit 0 +
  evidence 更新。
- [ ] `mise exec -- prek run --all-files` — Pass。
- [ ] MSRV lane — NotRun（daily CI 管轄）。

## Open Questions / Gaps

- `real-abi-test-plugin`↔`test-plugin` の serde 移行 —
  後続 slice で consumer 影響評価 + 承認のうえ決定。
- provisioned runner・`.github/**` 統合・Test Runtime 実結線・
  consumer 照合 — 後続 slice / 外部 gate。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
  consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatch worker が直接実装）。
- Independent work streams: none（逐次実施）。
- User approval required before dispatch: yes → review-protocol
  の blocker なしをもって承認とみなす（worker 契約）。
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- reviewer は spec AC の headless 範囲と fail-closed 境界
  （self-run NotRun を Pass にしない・additive のみ・既存
  serde 語彙不変）を重点確認。`mise.toml` の task が既存 lane
  を壊さないこと、`EnvironmentKind` 追加が schema 語彙と一致
  することも確認対象。
