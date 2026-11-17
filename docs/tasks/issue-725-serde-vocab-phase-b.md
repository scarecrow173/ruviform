---
id: "TASK-034Z7dTQ1avXmDsygN9lTA"
type: "task"
status: "in-progress"
title: "Issue #725 serde 語彙 Phase B — task"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-725-serde-vocab-phase-b.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-725-serde-vocab-phase-b.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-725-serde-vocab-phase-b.md"
  implemented-by:
    - "docs/impl/ir/issue-725-serde-vocab-phase-b.md"
  depends-on:
    - "docs/plans/issue-725-serde-vocab-phase-b.md"
    - "docs/discovery/issue-725-serde-vocab-phase-b.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-725-serde-vocab-phase-b.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #725 serde 語彙 Phase B — task

## Work

`EnvironmentKind::RealAbiTestPlugin` の serde emit 側を schema
語彙 `test-plugin` に統一する Phase B slice（Phase A design の
第2 slice — atomic flip）: serde `rename` 切替 + `as_str()` flip
+ plugin-dock-test の literal assert・コメント更新 + pin テスト
更新 + issue-629 docs の「解消済み」status 追記（append-only）+
evidence 記録。

- [x] T1: discovery → spec → design（Phase A design の第2 slice
  を実施形態として確定）
- [x] T2: `metadata.rs` — serde attr を
  `#[serde(rename = "test-plugin", alias = "real-abi-test-plugin")]`
  へ切替 + `as_str()` を `"test-plugin"` に flip + enum doc:90-98・
  variant doc:111-115 を Phase B 完了状態に更新
- [x] T3: `tests/metadata.rs` — emit pin 3 テスト更新
  （wire spellings pin 追加・deserialize 両綴り受容テストの emit
  期待値 flip・`RunMetadata` re-emit 期待値 flip）
- [x] T4: `plugin_dock.rs` :168,:211 + `Cargo.toml` :77-79 を
  `test-plugin` に更新
- [x] T5: plan → task → test-spec → ADR（最小限）
- [x] T6: 正規検証 — fmt-check / check / lint / test
  （ruviform-audio-test・ruviform-plugin-dock-test）/
  test-ci-consumer-verify / prek run --all-files。cdylib 解決
  可否による degrade 経路の実効範囲を記録
- [x] T7: issue-629 docs 5 ファイルへ「解消済み」status 追記
  （append-only — flip 検証後）
- [x] T8: `evidence/rf725/` に Phase B 検証記録・impl/ir・
  impl/exp 記録
- [ ] T9: review-protocol レビュー・blocker 解消 →
  Conventional Commits でコミット・push・PR 作成

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-725-serde-vocab-phase-b.md`
- `docs/specs/issue-725-serde-vocab-phase-b.md`
- `docs/designs/issue-725-serde-vocab-phase-b.md`
- `docs/test-specs/issue-725-serde-vocab-phase-b.md`
- `docs/discovery/issue-725-serde-vocab-phase-b.md`
- `docs/designs/issue-725-serde-vocab-phase-a.md`（移行方針の正本）

## Dependencies

- なし（正式 blockedBy なし）。Phase A merge（PR#736）・rf#631
  lease 解除（PR#741）済みで全前提が揃っている。

## Affected Files

- `crates/ruviform-audio-test/src/metadata.rs`
- `crates/ruviform-audio-test/tests/metadata.rs`
- `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs`
- `crates/ruviform-plugin-dock-test/Cargo.toml`（コメントのみ）
- `docs/designs|specs|test-specs/issue-629-*.md` 5 ファイル
  （追記のみ）
- `evidence/rf725/`（Phase B 検証記録）
- `docs/{discovery,specs,designs,plans,tasks,test-specs,adr,
  impl/ir,impl/exp}/issue-725-serde-vocab-phase-b.md` +
  `impl/exp/*-exp.jsonl`（新規）

## Done When

- [x] emit flip が atomic に実装され、serde emit・`as_str()`・
  plugin-dock-test assert が全て `test-plugin` に一致
- [x] deserialize が新旧両綴りを受容し続け、未知語彙拒否・
  round-trip pin が維持される
- [x] issue-629 docs の gap status が「解消済み」に追記済み
  （flip 検証後・append-only）
- [x] 正規検証（check/lint/test 両 crate・test-ci-consumer-verify・
  fmt-check・prek）が全て pass
- [x] degrade 経路のみの場合は実効確認の可否を NotRun として
  記録（「解消済み」の主張範囲を正直に限定）

## Verification

- [x] `mise run fmt-check` — pass
- [x] `mise run check ruviform-audio-test` — pass
- [x] `mise run lint ruviform-audio-test` — pass
- [x] `mise run test ruviform-audio-test` — pass
- [x] `mise run check ruviform-plugin-dock-test` — pass
- [x] `mise run lint ruviform-plugin-dock-test` — pass
- [x] `mise run test ruviform-plugin-dock-test` — pass
- [x] `mise run test-ci-consumer-verify` — pass（schema 非回帰）
- [x] `mise exec -- prek run --all-files` — pass
- [ ] `*-full`/`*-msrv`/3OS 横断・opt-in lane 実走 — NotRun
  （daily CI 委譲・運用判断）
