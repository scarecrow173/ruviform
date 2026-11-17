---
id: "PLAN-034Z7dMg41lvFaBPkz4Um6"
type: "plan"
status: "approved"
title: "Issue #725 serde 語彙 Phase B — 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-725-serde-vocab-phase-b.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
  implemented-by:
    - "docs/tasks/issue-725-serde-vocab-phase-b.md"
  depends-on:
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
  blocks: []
  supersedes: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-725-serde-vocab-phase-b.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #725 serde 語彙 Phase B — 実装計画

## Goal

`EnvironmentKind::RealAbiTestPlugin` の serde emit 側を schema
語彙 `test-plugin` に統一する（Phase A design の第2 slice —
atomic flip + 関連 pin・assert・docs status の更新）。

## Scope

単一 slice。Phase A（PR#736）で deserialize alias と移行方針は
確定済みで、本 plan は emit flip の実施のみを対象とする。

## Upstream Documents

- `docs/specs/issue-725-serde-vocab-phase-b.md`（approved）
- `docs/designs/issue-725-serde-vocab-phase-b.md`（approved）
- `docs/discovery/issue-725-serde-vocab-phase-b.md`
- `docs/designs/issue-725-serde-vocab-phase-a.md`（approved —
  移行方針の正本・第2 slice 手順 :100-111）
- `evidence/rf725/consumer-impact-assessment.md`（AC1 記録済み）

## Design Inputs

- `docs/designs/issue-725-serde-vocab-phase-b.md` — flip 後の
  挙動表・atomic flip の根拠・却下案・verification notes。
- `docs/designs/issue-725-serde-vocab-phase-a.md` — 移行期間中の
  挙動定義（:81-87）と serde `rename`/`alias` の使い分け。

## File Map

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-audio-test/src/metadata.rs` | `RealAbiTestPlugin` の serde attr を `#[serde(rename = "test-plugin", alias = "real-abi-test-plugin")]` へ切替、`as_str()` を `"test-plugin"` に flip、enum doc:90-98・variant doc:111-115 を Phase B 完了状態に更新。型名・variant 集合は不変 |
| `crates/ruviform-audio-test/tests/metadata.rs` | `environment_kind_wire_spellings_match_case_schema_v1`:186-204 に `RealAbiTestPlugin` → `test-plugin` pin 追加＋コメント更新。`environment_kind_deserialize_accepts_both_vocab_spellings`:206-232 の emit 期待値を `"test-plugin"` に flip（両綴り受容 assert は維持）。`run_metadata_from_json_accepts_test_plugin_spelling`:234-253 の re-emit 期待値を `"test-plugin"` に flip＋コメント更新。`environment_kind_serde_matches_as_str`:169-183 は変更しない |
| `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs` | :168 `"real-abi-test-plugin"` → `"test-plugin"`、:211 `Some("real-abi-test-plugin")` → `Some("test-plugin")`（assert メッセージの語彙言及も同様に更新） |
| `crates/ruviform-plugin-dock-test/Cargo.toml` | :77-79 コメントの `real-abi-test-plugin` 言及を `test-plugin` に更新（依存エッジ・feature は不変） |
| `docs/designs/issue-629-virtual-endpoint-lane.md` | gap 記述 3 箇所へ「解消済み」status を追記（append-only — flip 実施・検証後） |
| `docs/designs/issue-629-provisioning-runbook.md` | 同上 2 箇所 |
| `docs/specs/issue-629-virtual-endpoint-lane.md` | 同上 3 箇所 |
| `docs/specs/issue-629-provisioning-runbook.md` | 同上 2 箇所 |
| `docs/test-specs/issue-629-virtual-endpoint-lane.md` | 同上 3 箇所 |
| `evidence/rf725/` | Phase B 検証記録を追記（実施コマンド・exit code・flip 前後の emit 綴り確認） |
| `docs/{discovery,specs,designs,plans,tasks,test-specs,adr,impl/ir,impl/exp}/issue-725-serde-vocab-phase-b.md` + `impl/exp/*-exp.jsonl` | docs chain（本書含む） |

変更しないもの（確認済み）: `crates/ruviform-plugin-dock-
test/src/scenario.rs`（:280 `json!(kind.as_str())` は自動追従）、
`src/real_dock.rs`（`"real-window"` 比較のみで非影響）、
`ci/consumer-verify/**`・`.github/**`（schema 正本は既に
`test-plugin`）、`crates/ruviform-audio-test/Cargo.toml`（serde
derive 既存）、consumer repo、historical evidence、docs index。

## Implementation Sequence

- [x] discovery → spec → design（Phase A design の第2 slice を
  実施形態として確定）
- [x] `metadata.rs`: serde attr 切替 + `as_str()` flip + doc 更新
- [x] `tests/metadata.rs`: emit pin 3 テスト更新
- [x] `plugin_dock.rs` :168,:211 + `Cargo.toml` :77-79 更新
- [x] plan → task → test-spec → ADR（最小限）
- [x] 正規検証（check/lint/test 両 crate・test-ci-consumer-verify・
  fmt-check・prek）— flip の実効確認（cdylib 解決可否による
  degrade 経路の有無を記録）
- [x] issue-629 docs 5 ファイルへ「解消済み」status 追記
  （append-only — 検証後）
- [x] `evidence/rf725/` に Phase B 検証記録
- [x] impl/ir・impl/exp 記録
- [ ] review-protocol レビュー → commit/push/PR

## Dependencies and Risks

- **依存**: なし（正式 blockedBy なし）。Phase A merge・rf#631
  lease 解除済みで全前提が揃っている。
- **リスク**: flip の非 atomic 実施 → `environment_kind_
  serde_matches_as_str` と plugin-dock-test assert が検出。
  同一 slice で実施する。
- **リスク**: headless 環境で plugin_dock の綴り assert が degrade
  経路のみ → 実効確認の可否を検証結果として正直に記録し、
  degrade のみなら NotRun を PR 本文に明記。
- **リスク**: issue-629 docs の「追記のみ」制約違反 → 既存記述の
  書き換え・削除を行わず status 追記のみに限定。

## Task Breakdown

`docs/tasks/issue-725-serde-vocab-phase-b.md` に展開（T1〜T9）。

## Verification

- [x] `mise run fmt-check` — pass
- [x] `mise run check ruviform-audio-test` — pass
- [x] `mise run lint ruviform-audio-test` — clippy `-D warnings` pass
- [x] `mise run test ruviform-audio-test` — flip pin 含む全 test pass
- [x] `mise run check ruviform-plugin-dock-test` — pass
- [x] `mise run lint ruviform-plugin-dock-test` — pass
- [x] `mise run test ruviform-plugin-dock-test` — pass（degrade
  経路の実効範囲を記録）
- [x] `mise run test-ci-consumer-verify` — schema 側非回帰
- [x] `mise exec -- prek run --all-files` — 軽量チェック pass
- [ ] `*-full`/`*-msrv`/3OS 横断・opt-in lane 実走は daily CI /
  運用判断に委譲（NotRun として記録）

## Open Questions / Gaps

- なし。emit flip の方式・範囲・検証は Phase A design（approved）
  と Task Spec で確定済み。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
  consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker dispatch 内で完結）
- Independent work streams: none
- User approval required before dispatch: no（Orca worker として
  Spec に従い自律実行 — review-protocol で承認代替）
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute
  task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- reviewer focus: serde `rename`+`alias` の向き（canonical =
  `test-plugin`・旧綴り alias 受容）・`as_str()` flip との atomic
  性・plugin-dock-test assert の更新漏れがないこと・issue-629
  docs が追記のみで flip 検証後の記載であること・「解消済み」
  記載のタイミングが正しいこと・degrade 経路の実効範囲の正直な
  記録。
