---
id: "TASK-034YnAFuBxhg3H6xRVBCKB"
type: "task"
status: "done"
title: "Issue #629 virtual endpoint lane registration tasks"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/plans/issue-629-virtual-endpoint-lane.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-629-virtual-endpoint-lane.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-629-virtual-endpoint-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-virtual-endpoint-lane.md"
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-629-virtual-endpoint-lane.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-virtual-endpoint-lane.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 virtual endpoint lane registration tasks

## Work

Issue #629 5th slice の実装 slice。`test-audio-virtual-endpoint`
opt-in lane の `mise.toml` 登録と lane 説明文書化（AC6
headless）、stale 参照の同期、#628 Test Runtime 結線の headless
検証（`EnvironmentKind` additive 語彙補完を含む AC3
headless）、lane self-run の fail-closed evidence 記録を行う。

- T1: `mise.toml` に `[tasks.test-audio-virtual-endpoint]` を
  追加 + `docs/development-environment.md` の lane 一覧表・
  opt-in lane 注記へ追記。
- T2: stale 参照同期 — `virtual_endpoint.rs` module doc・
  `endpoint_probe.rs` verdict 文言・wasapi-endpoint design の
  AC6 記述・survey doc の lane 未登録記述。
- T3: `EnvironmentKind` に `VirtualOsEndpoint`・
  `PhysicalDevice` を additive 追加 + serde テスト更新 +
  #628 結線 IF 照合・verdict→RunMetadata 対応付け・既知語彙
  gap の docs 記録。
- T4: 検証 — fmt/check/lint/test（両 crate）・
  `lint-audio-real-input`・lane self-run・probe 再実行・`prek`・
  `evidence/rf629/` 記録。
- T5: docs 仕上げ — impl ir/exp（+ `*-exp.jsonl`）・
  review-protocol。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-629-virtual-endpoint-lane.md`
- `docs/specs/issue-629-virtual-endpoint-lane.md`
- `docs/designs/issue-629-virtual-endpoint-lane.md`
- `docs/test-specs/issue-629-virtual-endpoint-lane.md`
- `docs/discovery/issue-629-virtual-endpoint-survey.md`

## Dependencies

- 前提 slice（merge 済み）: `tests/virtual_endpoint.rs`・
  `endpoint_probe` 拡張（4th slice `scarecrow173/ruviform#673`）、
  `ruviform-audio-test` の `PcmFixture`/`oracle`/`RunMetadata`
  JSON（`scarecrow173/ruviform#628`）。
- coordinator 判断: `ruviform-audio-device` →
  `ruviform-audio-test` dev-dep 不承認 — typed 実結線は行わ
  ず結線 IF を docs 記録。`EnvironmentKind` 語彙差
  （`real-abi-test-plugin`↔`test-plugin`）は Coordinator
  回答 (A) により additive 追加のみ・gap は docs 記録。

## Affected Files

- `mise.toml`
- `docs/development-environment.md`
- `crates/ruviform-audio-device/tests/virtual_endpoint.rs`
- `crates/ruviform-audio-device/examples/endpoint_probe.rs`
- `crates/ruviform-audio-test/src/metadata.rs`
- `crates/ruviform-audio-test/tests/metadata.rs`
- `docs/specs|designs|plans|tasks|test-specs|
  issue-629-virtual-endpoint-lane.md`（本 chain）
- `docs/designs/issue-629-wasapi-endpoint.md`
- `docs/discovery/issue-629-virtual-endpoint-survey.md`
- `docs/impl/ir|exp/issue-629-virtual-endpoint-lane.md`
  （+ `*-exp.jsonl`）
- `evidence/rf629/`

## Done When

- [x] `mise.toml` に `test-audio-virtual-endpoint` が
  `test-audio-real-input` 先例の構造で登録され、
  provisioning 必須・manual・never CI が description に明記
  される。専用 lint lane は追加されない。
- [x] `docs/development-environment.md` に lane 行・注記が
  追記される。
- [x] stale 参照（verifier module doc・probe verdict・
  wasapi-endpoint design・survey doc）が canonical lane に
  同期され、probe 再実行で evidence が更新される。
- [x] `EnvironmentKind` に 2 variant が additive 追加され、
  serde `kebab-case` が schema 語彙と一致し、テストが整合
  する。既存 variant・serde 語彙・API は不変。
- [x] #628 結線 IF 照合・verdict→RunMetadata 対応付け・
  既知語彙 gap が docs に記録される。
- [x] lane self-run が `verdict: NotRun`（NOT_PROVISIONED）
  を `evidence/rf629/` に記録し、Pass を主張しない。
- [x] `fmt-check`/`check`/`lint`/`test`（両 crate）・
  `lint-audio-real-input`・`prek` が Pass。

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check ruviform-audio-device`
- [x] `mise run lint ruviform-audio-device`
- [x] `mise run test ruviform-audio-device`
- [x] `mise run check ruviform-audio-test`
- [x] `mise run lint ruviform-audio-test`
- [x] `mise run test ruviform-audio-test`
- [x] `mise run lint-audio-real-input`
- [x] `mise run test-audio-virtual-endpoint`（self-run —
  未 provision で `verdict: NotRun`、evidence 記録）
- [x] `mise exec -- cargo run -p ruviform-audio-device
  --example endpoint_probe`（Windows 実機、exit 0）
- [x] `mise exec -- prek run --all-files`
