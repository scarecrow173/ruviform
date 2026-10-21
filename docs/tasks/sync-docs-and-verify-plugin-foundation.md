---
id: "TASK-034JgsIynX51YyqUrzgppO"
type: "task"
status: "done"
title: "プラグイン基盤のドキュメント同期と総合検証"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/sync-docs-and-verify-plugin-foundation.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/plans/implement-plugin-foundation.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/specs/implement-plugin-foundation.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/discovery/evaluate-plugin-binding-options.md"
      - type: "file"
        path: "docs/discovery/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-plugin-foundation.md"
  implemented-by:
    - "../impl/ir/sync-docs-and-verify-plugin-foundation.md"
  depends-on:
    - "../plans/implement-plugin-foundation.md"
    - "create-plugin-core-foundation.md"
    - "implement-borrowed-planar-view.md"
    - "poc-clap-binding-adapter.md"
    - "poc-vst3-binding-adapter.md"
    - "decide-plugin-binding-method-adr.md"
    - "implement-plugin-adapters.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../README.md"
    - "../../Cargo.toml"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-plugin-foundation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/plugin-foundation.md"
  references: []
  defers: []
  deferred-by: []
---

# プラグイン基盤のドキュメント同期と総合検証

## Work

plan `PLAN-034JcvpHIlt5JmqvgOquvI` の T7（Task Breakdown の `sync-docs-and-verify-plugin-foundation` 仮称行に対応）。T1〜T6 の完了を受け、ドキュメント記述の同期・plan 相互リンクのバックフィル・status/`changes` 同期・ワークスペース総合検証を実施する。

- [x] ルート `Cargo.toml` の層内順序コメント、`README.md`（`crates/` 節・実装状態）、`AGENTS.md`（ワークスペース構成節）を実装状態に同期する
- [x] plan `docs/plans/implement-plugin-foundation.md` の `relations.implemented-by` に 7 タスク全件のパスをバックフィルする（Task Breakdown 分割ルールの相互リンク義務）
- [x] spec / plan / test-spec / task の status・`changes` 記録を実施結果に同期する
- [x] 総合検証スイートを実行し全合格を確認（Verification 節のコマンド群 — plan V-7）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-plugin-foundation.md](../plans/implement-plugin-foundation.md) — `PLAN-034JcvpHIlt5JmqvgOquvI`（approved）。T7 チェックリスト・V-7 の正本
- [../specs/implement-plugin-foundation.md](../specs/implement-plugin-foundation.md) — `SPEC-034JcgdAKPNeh1YCFyprzh`。受け入れ基準の突合対象
- [../test-specs/plugin-foundation.md](../test-specs/plugin-foundation.md) — `TSPEC-034Jg6YSP6CLjJHnFGG5oL`。振る舞い保証の意図（`relations.verified-by`）

## Dependencies

- 前提タスク（全て完了していること — plan 依存グラフ T7←T1〜T6）:
  - [create-plugin-core-foundation.md](create-plugin-core-foundation.md)（`TASK-034JgsFV6CWAjSnPuekBgs`）
  - [implement-borrowed-planar-view.md](implement-borrowed-planar-view.md)（`TASK-034JgsGmcPdPRoeRTvicjW`）
  - [poc-clap-binding-adapter.md](poc-clap-binding-adapter.md)（`TASK-034JgsGOtyuy2MkLRLZQI5`）
  - [poc-vst3-binding-adapter.md](poc-vst3-binding-adapter.md)（`TASK-034JgsHeZt7Bm40uBMo9wg`）
  - [decide-plugin-binding-method-adr.md](decide-plugin-binding-method-adr.md)（`TASK-034JgsHEkIUa3P4jzOizoM`）
  - [implement-plugin-adapters.md](implement-plugin-adapters.md)（`TASK-034JgsI5J4HU4qcoEBQfDx`）

## Affected Files

- `Cargo.toml` — 層内順序コメントの実装状態への同期
- `README.md` — `crates/` 節のクレート一覧と実装状態記述の同期
- `AGENTS.md` — ワークスペース構成節の同期
- `docs/plans/implement-plugin-foundation.md` — `relations.implemented-by` の 7 タスクバックフィル・status/`changes` 同期
- `docs/specs/implement-plugin-foundation.md` / `docs/test-specs/plugin-foundation.md` / `docs/tasks/` 各ファイル — status・`changes` 記録の同期

## Done When

- [x] `Cargo.toml` / `README.md` / `AGENTS.md` の記述が実装状態と一致する
- [x] plan `relations.implemented-by` が 7 タスク全件を指し、spec/plan/test-spec/task の status・`changes` が実施結果と一致する
- [x] 検証スイート全項目が PASS し、`cargo metadata` で全クレート `publish` が `[]`・依存辺が plugin 系→Domain foundations+Primitives のみであることを確認した

## Verification

- [x] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` / `mise run test-rt` / `mise run bench` が全て PASS
- [x] `mise exec -- cargo metadata --format-version 1 --no-deps` で全クレート `publish` が `[]`・依存辺が plugin 系→Domain foundations+Primitives のみであることを確認
- [x] `mise exec -- prek run --all-files` で全フック Passed
