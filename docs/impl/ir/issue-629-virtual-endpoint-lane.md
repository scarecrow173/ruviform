---
id: "IMPL-034YnB8vZodK9rvl3LOMoQ"
type: "impl"
status: "completed"
title: "Issue #629 virtual endpoint lane registration"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/tasks/issue-629-virtual-endpoint-lane.md"
    - "docs/plans/issue-629-virtual-endpoint-lane.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-629-virtual-endpoint-lane.md"
      - type: "doc"
        path: "docs/designs/issue-629-virtual-endpoint-lane.md"
      - type: "doc"
        path: "docs/plans/issue-629-virtual-endpoint-lane.md"
      - type: "doc"
        path: "docs/tasks/issue-629-virtual-endpoint-lane.md"
      - type: "doc"
        path: "docs/test-specs/issue-629-virtual-endpoint-lane.md"
      - type: "doc"
        path: "docs/impl/ir/issue-629-virtual-endpoint-lane.md"
      - type: "doc"
        path: "docs/impl/exp/issue-629-virtual-endpoint-lane.md"
      - type: "doc"
        path: "docs/impl/exp/issue-629-virtual-endpoint-lane-exp.jsonl"
      - type: "evidence"
        path: "evidence/rf629/virtual_endpoint_slice5_lane_selfrun-windows-local.txt"
    modified:
      - type: "code"
        path: "mise.toml"
      - type: "doc"
        path: "docs/development-environment.md"
      - type: "code"
        path: "crates/ruviform-audio-device/tests/virtual_endpoint.rs"
      - type: "code"
        path: "crates/ruviform-audio-device/examples/endpoint_probe.rs"
      - type: "code"
        path: "crates/ruviform-audio-test/src/metadata.rs"
      - type: "code"
        path: "crates/ruviform-audio-test/tests/metadata.rs"
      - type: "doc"
        path: "docs/designs/issue-629-wasapi-endpoint.md"
      - type: "doc"
        path: "docs/discovery/issue-629-virtual-endpoint-survey.md"
      - type: "evidence"
        path: "evidence/rf629/endpoint_probe-windows-local.txt"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-629-virtual-endpoint-lane.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-629-virtual-endpoint-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-virtual-endpoint-lane.md"
    - "docs/impl/exp/issue-629-virtual-endpoint-lane.md"
    - "docs/impl/ir/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-629-virtual-endpoint-lane.md"
    - "docs/plans/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-629-virtual-endpoint-lane.md"
  verified-by: []
  references:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-629-virtual-endpoint-lane.md"
    rejected: []
---

# Issue #629 virtual endpoint lane registration

## Summary

Issue #629 5th slice。4th slice で実装済みの WASAPI streaming
verifier（`tests/virtual_endpoint.rs`）の canonical opt-in lane
`test-audio-virtual-endpoint` を `mise.toml` に登録し、
`docs/development-environment.md`・verifier module doc・
`endpoint_probe` verdict・wasapi-endpoint design・survey doc の
stale 参照を同期した。#628 Test Runtime 結線は headless 範囲で
照合し、`EnvironmentKind` に Case Schema v1 closed set の欠落
variant（`VirtualOsEndpoint`/`PhysicalDevice`）を additive
追加した。lane self-run は未 provisioning のため
`verdict: NotRun`（fail-closed）を記録し、Pass は主張しない。

## Context

`docs/tasks/issue-629-virtual-endpoint-lane.md`（plan
`docs/plans/issue-629-virtual-endpoint-lane.md`、spec
`docs/specs/issue-629-virtual-endpoint-lane.md`、design
`docs/designs/issue-629-virtual-endpoint-lane.md`）に従う。
本 slice は Issue AC6 の headless 部分（opt-in lane 登録と
lane 説明の文書化）と AC3 の headless 部分（#628 Test Runtime
結線 IF の照合・`EnvironmentKind` 語彙欠落の補完）を担当
する。Coordinator 回答 (A) により `EnvironmentKind` は
additive variant 追加のみとし、`real-abi-test-plugin`↔
`test-plugin` の serde 綴り差は既知 gap として記録した。
`ruviform-audio-device` → `ruviform-audio-test` dev-dep は
不承認のため typed 実結線は行わない。production 契約・
verifier/probe の挙動は無変更（文言・doc comment のみ）。

## Implementation

### Changes

- `mise.toml`: `[tasks.test-audio-virtual-endpoint]` を
  `lint-audio-real-input` 直後・`test-midi-real-device` 直前に
  追加。`run` は 4th slice design の AC6 lane 設計記録どおり
  `cargo test -p ruviform-audio-device --features real-device
  --test virtual_endpoint -- --ignored --test-threads=1`。
  description に provisioning 依存・manual・never CI・
  未 provisioning 時の `verdict: NotRun` を明記。専用 lint
  lane は追加せず `lint-audio-real-input` が coverage する旨を
  コメントに記録。
- `docs/development-environment.md`: lane 一覧表に
  `test-audio-virtual-endpoint` 行を追加（`lint-audio-real-
  input` の次）、`lint-audio-real-input` 行の説明に
  `virtual_endpoint` test target の coverage を追記、opt-in
  lane 注記節に本 lane の bullet を追加。
- `crates/ruviform-audio-device/tests/virtual_endpoint.rs`:
  module doc の「no dedicated mise lane is registered in this
  slice」を canonical lane `mise run
  test-audio-virtual-endpoint` への参照に更新。
- `crates/ruviform-audio-device/examples/endpoint_probe.rs`:
  verdict の streaming proof 文言を canonical lane 名
  `mise run test-audio-virtual-endpoint`（provisioning-
  dependent manual lane, never CI）に更新。
- `docs/designs/issue-629-wasapi-endpoint.md`: AC6 lane 関連
  記述を同期 — Non-Functional Constraints の lane 項目・
  依存項目（`mise.toml` 変更不要→5th slice で additive 追加）、
  Deferred Design Concerns の AC6 項目（mise task 登録は
  解消・`.github/**` は残存）、Verification Notes の AC6 lane
  設計記録（登録済み・専用 lint lane 不採用の確定を追記）。
- `docs/discovery/issue-629-virtual-endpoint-survey.md`:
  Evidence 節に 2026-11-16 5th slice dated entry を追記
  （append-only 規約に従い既存 entry は不変）— lane 登録・
  4th slice「canonical=mise lane 未登録」記述の解消・self-run
  結果を記録。
- `crates/ruviform-audio-test/src/metadata.rs`:
  `EnvironmentKind` に `VirtualOsEndpoint`（→
  `virtual-os-endpoint`）・`PhysicalDevice`（→
  `physical-device`）を additive 追加。`as_str`・doc
  comment を schema 語彙に整合させ、enum doc に
  `real-abi-test-plugin`↔`test-plugin` の既知語彙 gap を
  明記。既存 variant・serde 語彙・API は不変。
- `crates/ruviform-audio-test/tests/metadata.rs`:
  `environment_kind_serde_matches_as_str` の対象配列に 2
  variant を追加し、Case Schema v1 語彙の literal pin テスト
  `environment_kind_wire_spellings_match_case_schema_v1` を
  新設（`RealAbiTestPlugin` は既知 gap のため対象外と明記）。
- `evidence/rf629/endpoint_probe-windows-local.txt`: probe
  再実行（exit 0）で verdict 文言更新を反映。
- `evidence/rf629/virtual_endpoint_slice5_lane_selfrun-
  windows-local.txt`: lane self-run の canonical 実行ログ +
  `--nocapture` 補助証拠（verdict 行採取用）を新設。
- docs chain: spec/design/plan/task/test-spec/impl ir・exp
  （+ `*-exp.jsonl`）を `issue-629-virtual-endpoint-lane`
  slug で新設。

### Decisions During Implementation

- **専用 lint lane は追加しない（Coordinator 確定）**: 新 test
  target は `#![cfg(feature = "real-device")]` 配下のため
  既存 `lint-audio-real-input`（`--all-targets --features
  real-device`）が coverage 済み。4th slice design の
  「専用 lane を増やすかは Coordinator 判断」を「増やさない」
  で確定し、design doc・`mise.toml` コメント・
  `docs/development-environment.md` に記録。
- **survey doc Evidence 節は append-only を維持**: task spec
  は「canonical=mise lane 未登録」注記の更新を指示したが、
  同節の「never edit/reorder/delete existing」規約を優先し、
  既存 entry は編集せず新規 dated entry で lane 登録と
  self-run を記録し旧記述の解消を明記した。
- **`EnvironmentKind` は additive のみ（Coordinator 回答
  (A)）**: `VirtualOsEndpoint`・`PhysicalDevice` を追加。
  `RealAbiTestPlugin` の `real-abi-test-plugin` ↔ schema
  `test-plugin` の綴り差は serde 語彙の破壊的変更
  （`ruviform-plugin-dock-test` が evidence JSONL に出力中・
  AL/SV が dev-dep 消費）であるため本 slice では解消せず、
  enum doc・design doc・本記録に既知 gap として記録し後続
  slice の serde 移行判断へ hand-off。
- **#628 結線 IF の照合結果**: merge 済み API
  （`PcmFixture`・`oracle::pcm_reference_block`・RunMetadata
  JSON save/load）は前回 docs の結線 IF と齟齬なし。
  verifier verdict→RunMetadata 対応付けは dev-dep 不承認を
  前提に docs 記録のみ — typed 結線は後続 slice。
- **self-run evidence は 2 部構成**: canonical lane
  `mise run test-audio-virtual-endpoint` の実行ログ（exit 0・
  `1 passed`）に加え、verdict 行採取のため `--nocapture` 付き
  直接 cargo 実行を補助証拠として併記（4th slice と同じ
  libtest capture の注意）。canonical `run` に `--nocapture`
  は含めない。
- **docs index は未再生成**: `docs/**/README.md` は生成物で
  本 slice の編集範囲外 — Coordinator 統合時に再生成される
  前提で PR 本文の「未対応の作業」に明記。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-629-virtual-endpoint-lane.md`

### Rejected

- なし

## Validation

- `mise run fmt-check` — Pass。
- `mise run check|lint|test ruviform-audio-device` — Pass。
- `mise run check|lint|test ruviform-audio-test` — Pass
  （`environment_kind_wire_spellings_match_case_schema_v1`
  含む metadata tests 11 件全 pass）。
- `mise run lint-audio-real-input`（`--all-targets --features
  real-device` の `-D warnings`）— Pass。
- `mise run test-audio-virtual-endpoint` — exit 0、
  `test render_capture_round_trip ... ok`（1 passed）。
  補助証拠（`--nocapture`）で `verdict: NotRun — no input
  devices on this host`（fail-closed）を確認。
- `mise exec -- cargo run -p ruviform-audio-device --example
  endpoint_probe`（Windows 実機）— exit 0、
  `evidence/rf629/endpoint_probe-windows-local.txt` 更新
  （`provisioning: NOT_PROVISIONED`・verdict 文言は canonical
  lane 参照）。
- `mise exec -- prek run --all-files` — Pass（`check-toml` が
  `mise.toml` を検査）。
- MSRV lane — NotRun（daily CI 管轄、task spec 指定どおり未
  実行）。

## Risks

- lane 登録は additive で既存 task・3OS 経路を変更しない。
  `prek` の `check-toml` で TOML 妥当性を確認済み。
- `EnvironmentKind` 追加は additive — 既存 variant・serde
  語彙・API・`Cargo.toml`/`Cargo.lock` は無変更。消費側
  （plugin-dock-test 等）の compile 影響は `check/lint/test
  ruviform-audio-test` で確認（enum は `#[non_exhaustive]`
  で外部 match は wildcard 必須）。
- self-run は未 provisioning の NotRun — Pass・
  `virtual-os-endpoint`・実 OS L3 実走 Pass を主張しない。

## Follow-ups

- provisioned runner での verifier 実走 Pass（AC1/AC2/AC5）・
  実 provisioning — 後続 slice / repo 外工程。
- `.github/**` CI/runner 統合（定期 opt-in 実行）— 外部 gate。
- Test Runtime 完全結線（dev-dep 承認待ち）・verdict→
  RunMetadata 実結線 — 後続 slice。
- `real-abi-test-plugin`↔`test-plugin` の serde 移行 —
  consumer 影響評価 + 承認のうえ後続 slice で決定。
- consumer 照合（AC7 — `scarecrow173/synvelis#115`/`#117`/
  `#126`・`scarecrow173/audio-lens#127`）。
- docs index 再生成 — Coordinator 統合時。
