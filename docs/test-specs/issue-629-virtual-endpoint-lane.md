---
id: "TSPEC-034YnAGpr2IBccDJo26Mx9"
type: "test-spec"
status: "approved"
title: "Issue #629 virtual endpoint lane registration test spec"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-629-virtual-endpoint-lane.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/tasks/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 virtual endpoint lane registration test spec

## Purpose

`test-audio-virtual-endpoint` lane 登録・stale 参照同期・
`EnvironmentKind` 語彙補完が満たすべき契約を固定する。本 spec
は lane 定義と metadata enum の正しさを判定する基準であり、
provisioned runner での verifier 実走 Pass を代替しない —
未 provision 環境での `NotRun` 記録も本 spec の合格条件に含む。

## Feature

仮想 Audio Endpoint streaming verifier の opt-in manual lane
登録（AC6 headless）と、#628 Test Runtime `EnvironmentKind`
語彙の Case Schema v1 closed set への additive 補完（AC3
headless）。

## Rules

- **R1 opt-in manual lane**: `test-audio-virtual-endpoint` は
  provisioning 依存の manual/opt-in task であり、通常 Daily
  Full CI の必須前提に無条件追加しない。description に
  provisioning 必須・manual・never CI を明記する。
- **R2 canonical 起動式**: lane の `run` は 4th slice design
  の AC6 lane 設計記録どおり `cargo test -p
  ruviform-audio-device --features real-device --test
  virtual_endpoint -- --ignored --test-threads=1`。`real-device`
  feature + `#[ignore]` + `--test-threads=1` で
  `test-audio-real-input` と同型。
- **R3 fail-closed self-run**: 未 provisioning ホストでの
  self-run は exit 0 + `verdict: NotRun`（NOT_PROVISIONED）
  を返す。Pass・Skip=Pass・`virtual-os-endpoint` 主張を
  一切行わない。
- **R4 既存 lane 不変**: `mise.toml` の変更は task ブロック
  追加のみで、既存 task・3OS 経路・他 OS の検証を変えない。
- **R5 lint coverage の一元化**: 専用 lint lane を追加せず、
  既存 `lint-audio-real-input`（`--all-targets --features
  real-device`）が新 test target をカバーする — この判断が
  docs に記録される。
- **R6 additive enum**: `EnvironmentKind` への variant 追加は
  additive のみ。既存 variant の改名・serde 語彙変更・API の
  変更/削除・新規 dep は禁止。enum レベルの
  `#[non_exhaustive]` は維持。
- **R7 schema 語彙一致**: 追加 variant の serde `kebab-case`
  は Case Schema v1 closed set の綴り（`virtual-os-endpoint`・
  `physical-device`）と一致し、`as_str`・doc コメント・
  serde round-trip テストが整合する。
- **R8 既知語彙 gap の正直な記録**:
  `RealAbiTestPlugin` の `real-abi-test-plugin` ↔ schema
  `test-plugin` の差は破壊的変更のため解消せず、既知 gap と
  して enum doc・design・PR 本文に記録する。schema 語彙と
  一致しない spelling を「一致済み」と主張しない。
- **R9 stale 参照ゼロ**: 「mise lane 未登録」を示す記述は
  canonical lane `mise run test-audio-virtual-endpoint` への
  参照に更新される。survey doc Evidence 節は append-only
  規約に従い新規 dated entry で記録する。
- **R10 正直な証拠**: 実行していない検証を Pass と書かない。
  lint/compile の Pass を `virtual-os-endpoint` の Pass や
  実 OS L3 実走 Pass と主張しない。

## Examples

- **E1 lane 定義**: Given `mise.toml` When
  `[tasks.test-audio-virtual-endpoint]` を確認 Then `run` が
  記録済み設計の cargo test 起動式と一致し、description が
  provisioning 必須・manual・never CI を明記する。
- **E2 self-run 未 provision**: Given 仮想 audio pair が無い
  host When `mise run test-audio-virtual-endpoint` を実行
  Then exit 0 で `verdict: NotRun`（NOT_PROVISIONED）が
  出力され、`evidence/rf629/` に記録される。
- **E3 enum 語彙**: Given `EnvironmentKind` When serde
  serialize/deserialize と `as_str` を比較 Then
  `VirtualOsEndpoint`→`virtual-os-endpoint`・
  `PhysicalDevice`→`physical-device` が schema 語彙と一致し
  round-trip が成立する。既存 3 variant の spelling は不変。
- **E4 既知 gap**: Given `RealAbiTestPlugin` When `as_str` を
  確認 Then `real-abi-test-plugin`（schema `test-plugin` との
  既知 gap）が維持され、gap が docs に記録される。
- **E5 stale 参照**: Given verifier module doc・probe verdict・
  wasapi-endpoint design・survey doc When lane 記述を確認
  Then canonical lane `mise run test-audio-virtual-endpoint`
  への参照に更新されている。

## Guarantees

- lane 登録は additive で既存の全 lane・3OS 検証経路を壊さ
  ない。
- `EnvironmentKind` の追加は公開 enum の additive 変更であり、
  既存 consumer（AL/SV/plugin-dock-test）の compile・証拠
  vocabulary を破壊しない。
- 未 provisioning 環境での self-run は常に fail-closed の
  `NotRun` であり、Pass に昇格しない。

## Non-goals

- provisioned runner での verifier 実走 Pass・実
  provisioning・`.github/**` CI 統合・Test Runtime 実結線・
  consumer 照合 — 全て後続 slice / 外部 gate。
- `real-abi-test-plugin`↔`test-plugin` の serde 移行 — 後続
  slice の判断事項。
