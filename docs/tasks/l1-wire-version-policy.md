---
id: "TASK-034NhK97jcINkcEKDNbpum"
type: "task"
status: "done"
title: "L1 wire バージョン方針の記録と契約検証の補強"
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
    - "docs/plans/implement-issue-119-l1-wire-version-policy.md"
  implemented-by:
    - "docs/impl/ir/issue-119-l1-wire-version-policy-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-119-l1-wire-version-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/l1-core-contracts-and-wire-codec.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-119-l1-wire-version-policy.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
  defers: []
  deferred-by: []
---

# L1 wire バージョン方針の記録と契約検証の補強

## Work

1. `docs/designs/plugin-l1-out-of-process.md` — wire/codec 設計節へ「ワイヤバージョン方針（Issue #119 で確定）」決定ブロックを追加（単一最新バージョン・`WIRE_VERSION` は世代整数・`Hello.version`/shm `wire_version` 厳密一致・交渉機構なし・bump 判断基準・capabilities 予約維持・`WIRE_MAX_*` 固定上限・再検討条件）。Deferred Design Concerns の「wire バージョニング/進化」行を本決定を指す記述へ reconcile（reopen 条件 = 独立配布ホスト/ランナー互換要求）。`updated`・`derived-by` 更新
2. `docs/specs/plugin-l1-out-of-process.md` — Requirements へ方針条項を追加・同表の deferred 行を reconcile・`updated`・`derived-by` 更新
3. `crates/ruviform-plugin-core/src/wire.rs` — `//!` Versioning 節へ方針要約を追記（SHM `wire_version` 厳密一致・単一最新バージョン・旧版 decode/変換/共存なし・交渉機構なし・bump 判断基準・capabilities 予約）し正本 design へ参照
4. `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` — `create_segment` を `create_segment_with(key, magic, wire_version)` へ一般化し、`e2e_shm_header_wire_version_mismatch` を追加（`wire_version = WIRE_VERSION + 1` の segment → runner は `Hello` 送出後に非ゼロ終了）
5. `docs/designs/issue-115-l1-gui-cross-process-parenting.md` — re-engagement 前提 (a) を確定済み注記へ更新
6. `docs/test-specs/plugin-l1-{wire-codec,proxy-contract,runner-e2e}-tests.md` — `verifies` へ `issue-119` spec を追加・deferred/non-goal 記述を確定方針へ整合・runner shm header example へ `wire_version` 不一致を明記
7. spec/plan/IR 作成・README index 行確認・doc/impl audit clean

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-119-l1-wire-version-policy.md](../plans/implement-issue-119-l1-wire-version-policy.md)
- [../specs/issue-119-l1-wire-version-policy.md](../specs/issue-119-l1-wire-version-policy.md)
- [../designs/plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)・[../specs/plugin-l1-out-of-process.md](../specs/plugin-l1-out-of-process.md)（方針記録先・deferred reconcile 先）
- [../test-specs/plugin-l1-wire-codec-tests.md](../test-specs/plugin-l1-wire-codec-tests.md)・[../test-specs/plugin-l1-runner-e2e-tests.md](../test-specs/plugin-l1-runner-e2e-tests.md)・[../test-specs/plugin-l1-proxy-contract-tests.md](../test-specs/plugin-l1-proxy-contract-tests.md)（契約↔テスト対応の正本）

## Dependencies

- 先行タスク: なし — 対象挙動は Issue #76 系で実装済み（`l1-core-contracts-and-wire-codec` 他）。本タスクは記録 + 欠落テスト 1 本のみ

## Affected Files

- `docs/designs/plugin-l1-out-of-process.md`（決定ブロック + deferred 行 + frontmatter）
- `docs/specs/plugin-l1-out-of-process.md`（Requirements 条項 + deferred 行 + frontmatter）
- `crates/ruviform-plugin-core/src/wire.rs`（`//!` Versioning 節 — ドキュメントのみ・コード非変更）
- `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs`（fixture 一般化 + `e2e_shm_header_wire_version_mismatch`）
- `docs/designs/issue-115-l1-gui-cross-process-parenting.md`（前提 (a) 注記）
- `docs/test-specs/plugin-l1-{wire-codec,proxy-contract,runner-e2e}-tests.md`（`verifies` + 記述整合）

## Done When

- [x] 方針全条項が design 決定ブロック・spec Requirements・`wire.rs` `//!` の 3 箇所に矛盾なく記録されている
- [x] 両親文書の deferred 行が Issue #119 決定を指し reopen 条件（独立配布互換要求）を明示している
- [x] `e2e_shm_header_wire_version_mismatch` が追加され runner E2E スイートで pass（製品コードへの検証用機構追加なし）
- [x] バージョン交渉・互換レイヤー・capability 割当・上限交渉の機構が差分に存在しない

## Verification

- [x] `cargo test -p ruviform-plugin-export --test l1_runner_e2e` — 12 tests 全 pass（`e2e_shm_header_wire_version_mismatch` 含む）
- [x] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` 全パス
- [x] doc audit `blocking: 0`・`findings: []` / impl-record audit で本 IR findings ゼロ

## Completion Notes

- **欠落は runner shm `wire_version` 不一致のみ** — Hello version 厳密一致（`handshake_version_mismatch`・`e2e_version_mismatch_is_a_parent_side_handshake_failure`）・reserved/capabilities 非ゼロ（`decode_rejects_nonzero_reserved_and_bad_utf8`）・`WIRE_MAX_*`（`decode_rejects_declared_lengths_over_wire_max`・`decode_rejects_impossible_counts_without_allocating`）・shm magic/形状（`e2e_shm_header_mismatch`・`shm_header_roundtrip_matches_wire_layout`）は既存テストで固定済みだった。runner shm の `wire_version` 一致検証は実装済みだが専用テストがなく、`e2e_shm_header_wire_version_mismatch` で固定
- **製品コード変更なし** — proxy handshake・runner・codec は既に方針どおり。テストは `create_segment` を `create_segment_with` へ一般化して header の `wire_version` だけを差し替える fixture 破損方式（version override 機構は製品側に追加しない）
- IR: [issue-119-l1-wire-version-policy-implementation.md](../impl/ir/issue-119-l1-wire-version-policy-implementation.md)
