---
id: "PLAN-034NhK1lX72CUL3VIU7gOG"
type: "plan"
status: "completed"
title: "Issue #119 実施計画 — L1 wire バージョン方針の記録と契約検証"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/119"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_runner_e2e.rs"
      - type: "file"
        path: "docs/designs/plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/specs/plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-wire-codec-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-proxy-contract-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-runner-e2e-tests.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-119-l1-wire-version-policy.md"
  implemented-by:
    - "docs/tasks/l1-wire-version-policy.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-119-l1-wire-version-policy.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/specs/plugin-l1-out-of-process.md"
  derived-by:
    - "docs/tasks/l1-wire-version-policy.md"
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/115"
  defers: []
  deferred-by: []
---

# Issue #119 実施計画 — L1 wire バージョン方針の記録と契約検証 Implementation Plan

## Goal

Issue #76 で導入済みの wire バージョン厳密一致・reserved 拒否・`WIRE_MAX_*` 上限の挙動を束ねる**単一最新バージョン方針**を L1 design/spec/`wire.rs` module doc に記録し、記録した契約のカバレッジ欠落（runner 側 shm header `wire_version` 不一致拒否）のみテストで補強する — 新規プロトコル機構を実装しない単一タスク。

## Scope

単一実装ストリーム — ドキュメント記録（design 決定ブロック + spec Requirements + `wire.rs` `//!` Versioning）・deferred concern reconcile（親文書 2 件 + issue-115 design + test-spec 3 件）・テスト 1 本追加（`l1_runner_e2e.rs`）。proxy handshake・runner・codec の製品コード挙動は既に方針どおりのため変更しない。

## Upstream Documents

- [../specs/issue-119-l1-wire-version-policy.md](../specs/issue-119-l1-wire-version-policy.md)（Issue #119 spec — 方針本文・In/Out Scope・AC の正本）
- [../specs/plugin-l1-out-of-process.md](../specs/plugin-l1-out-of-process.md)（Issue #76 spec — Requirements 条項の追加先・deferred 行の reconcile 先）
- [../designs/plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)（Issue #76 design — 決定ブロック「ワイヤバージョン方針」の記録先）

## Design Inputs

- [../designs/plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md) — wire/codec 設計節（`WIRE_VERSION` 照合・`#[non_exhaustive]`・shm header レイアウト）と Deferred Design Concerns 表
- [../impl/ir/issue-76-l1-core-wire-task-1-implementation.md](../impl/ir/issue-76-l1-core-wire-task-1-implementation.md) — wire プロトコル実装の記録（strict-equality・`Malformed`/`LimitExceeded` 拒否の実装根拠）
- [../designs/issue-115-l1-gui-cross-process-parenting.md](../designs/issue-115-l1-gui-cross-process-parenting.md) — re-engagement 前提 (a)「wire versioning 枠組みの確定」の消費者
- `crates/ruviform-plugin-core/src/wire.rs` `//!` — codec 側の正本ドキュメント（Versioning 節の拡張先）

## File Map

- `docs/designs/plugin-l1-out-of-process.md` — wire/codec 設計節へ「ワイヤバージョン方針」決定ブロック追加・deferred 行 reconcile・`derived-by`/`updated` 更新
- `docs/specs/plugin-l1-out-of-process.md` — Requirements へ方針条項追加・deferred 行 reconcile・`derived-by`/`updated` 更新
- `crates/ruviform-plugin-core/src/wire.rs` — `//!` Versioning 節へ方針要約（SHM 厳密一致・単一バージョン・交渉なし・bump 基準・capabilities 予約・上限非交渉）を追記し正本 design を参照
- `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` — `create_segment` → `create_segment_with(key, magic, wire_version)` へ fixture 一般化 + `e2e_shm_header_wire_version_mismatch` 追加
- `docs/designs/issue-115-l1-gui-cross-process-parenting.md` — 前提 (a) を確定済み注記へ更新
- `docs/test-specs/plugin-l1-{wire-codec,proxy-contract,runner-e2e}-tests.md` — `verifies` へ本 spec 追加・deferred/non-goal 記述の整合・runner shm header example へ `wire_version` 不一致を明記

## Implementation Sequence

- [x] 既存挙動の診断 — proxy `L1Instance::handshake` の厳密一致 + kill/reap、runner Hello emit（version 非判断）・runner shm attach の `wire_version` 厳密一致、wire codec の reserved/`WIRE_MAX_*` 拒否、既存テストの棚卸し
- [x] カバレッジ欠落の確定 — runner 側 shm header `wire_version` 不一致拒否のみ専用テストなし（magic/形状は `e2e_shm_header_mismatch`・Hello version は `e2e_version_mismatch_is_a_parent_side_handshake_failure` で済）
- [x] `l1_runner_e2e.rs` の fixture 一般化と `e2e_shm_header_wire_version_mismatch` 追加 → 対象テスト実行で pass 確認
- [x] `wire.rs` `//!` Versioning 節拡張 → design/spec の方針記録・deferred reconcile → issue-115 design + test-spec 3 件の整合
- [x] spec/plan/task/IR 作成 → README index 更新 → 全 gate + doc/impl audit

## Dependencies and Risks

- 新規依存・feature・unsafe なし — ドキュメントと既存 test fixture の一般化 + 1 テストのみ
- 方針記述が「新機構の要求」と誤読されないよう、各記録箇所で「実装しないことの確定」であることを明示する（Out of Scope と対にする）
- runner shm テストは実 runner バイナリを spawn する E2E — 既存 `e2e_shm_header_mismatch` と同一 fixture 経路で、プラットフォーム差異は既存スイートと同等
- 方針の記録先が 3 箇所（design/spec/module doc）あるため文言の drift リスク — design を正本・spec は要求条項・module doc は要約+design 参照と役割分担して重複を最小化

## Task Breakdown

- [../tasks/l1-wire-version-policy.md](../tasks/l1-wire-version-policy.md) — 単一タスク（方針記録 + reconcile + テスト 1 本が 1 変更単位のため分割しない）

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check`（workspace --all-targets）
- [x] `mise run lint`（workspace --all-targets・`-D warnings`）
- [x] `mise run test`（全 test binary pass — 新規 `e2e_shm_header_wire_version_mismatch` を含む）
- [x] `mise run test-rt`（全 `rt_` テスト pass — 本変更は RT 経路に非接触・件数変化なし）
- [x] doc audit: `node .agents/skills/doc-status/scripts/doc_status.js audit --json` — `blocking: 0`・`findings: []`
- [x] impl-record audit: `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` — 本 IR は findings ゼロ（legacy 4 文書の既知 findings は pre-existing）

## Open Questions / Gaps

- なし — spec で全要件確定済み。カバレッジ欠落の判定のみ実測依存（結果: runner shm `wire_version` 不一致の 1 箇所のみ）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一タスク・ドキュメント + テスト 1 本）
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 焦点: 方針本文が design/spec/`wire.rs` の 3 箇所で矛盾なく記録されているか・deferred 行の reconcile が reopen 条件（独立配布の互換要求）を残しているか・新規テストが製品コードへ検証用機構を追加せず fixture 破損のみで検証しているか・「実装しない」旨の記述が機構要求と誤読されないか
