---
id: "IMPL-ADpOyuANu5NdEX6PjNcvoR"
type: "impl"
status: "completed"
title: "T1 verify lane check-ci 統合 (Issue #331)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/data/issue-331-verify-step-runs.jsonl"
  implements:
    - "docs/tasks/issue-331-t1-check-step-dedup.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-331-t1-check-step-dedup.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-331-check-clippy-test-dedup.md"
    - "docs/designs/issue-331-check-clippy-test-dedup.md"
    - "docs/test-specs/issue-331-check-clippy-test-dedup.md"
    - "docs/impl/exp/issue-331-check-clippy-test-dedup.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-331-check-clippy-test-dedup.md"
    rejected: []
---

# T1 verify lane check-ci 統合 (Issue #331)

## Summary

CI `verify` job から独立 `Type-check workspace (mise run check-ci)` step を除去し、type-check 責務を `lint-ci` (clippy-driver) へ統合。`check-ci` は CI lane から外れた単独タスクとして `mise.toml` に残存(素の rustc でだけ見たいローカルトリアージ向け)。実コード・`Cargo.toml`・`Cargo.lock` の変更はゼロ。

## Context

- 実施タスク: docs/tasks/issue-331-t1-check-step-dedup.md
- 上流: spec(等価性要件)・design(統合アーキテクチャ)・test-spec(注入実験計画)
- 前提実測: `cargo check --workspace --all-targets --profile ci` と `cargo clippy --workspace --all-targets --profile ci` は同一の 292 check-mode ユニット(152 依存 + 140 workspace)を評価する。deps は両コマンドで共有されるが、workspace メンバーは `rustc` と `clippy-driver` の fingerprint 差で二重コンパイルされていた(140 ユニットが冗長 frontend pass)。

## Implementation

### Changes

- `.github/workflows/ci.yml` — `verify` job の `Type-check workspace (mise run check-ci)` step を削除。残存 lint step を `Type-check & lint workspace (mise run lint-ci)` に改名し、clippy-driver が rustc 全診断 + clippy lint を発行するため独立 check は同一ユニット集合の二重評価だった旨のコメントを付与。
- `mise.toml` —
  - `verify-ci`: `fmt-check` → `lint-ci` → `test-ci` に変更(`check-ci` 除去)。
  - `check-ci`: description を「CI verify lane には含まれない単独タスク・ローカルトリアージ向け」に更新。タスク自体は削除しない。
  - `lint-ci`: description を「CI profile lint + type-check。CI verify lane の型検査を所有」に更新。
- `AGENTS.md` — 検証入口表の `check-ci`/`lint-ci`/`verify-ci` 行と `.github/workflows/ci.yml` 説明 bullet を新構成に同期。
- `docs/impl/exp/data/issue-331-verify-step-runs.jsonl` — baseline run(41816075282/41816923217/41819703706/41820208123/41820870299 系)の verify step 時間 raw data。

### 変更しなかったもの(意図的)

- `rt-verify` / `icc-verify` / `ui-verify` / `l1-real-verify` / `bench` — feature・OS・host 条件が通常 workspace 検証と非等価。通常検証で代替できると仮定しない(issue 指示)。
- `cargo test` — codegen + `cfg(test)` を持ち check-mode とは別責務。`--all-targets` は付与しない(現行維持)。
- sccache 設定・cargo registry cache — #329 スコープ、本 issue では触らない。
- dev profile と ci profile の統一 — profile 統一は等価性を実証できないため非適用。

## Verification

| 検証 | 実行 | 結果 |
| --- | --- | --- |
| `mise exec -- actionlint .github/workflows/ci.yml` | 実行 | Pass |
| `mise exec -- prek run --all-files` | 実行 | Pass(8 hook) |
| `mise run verify-ci`(新列 `fmt-check`→`lint-ci`→`test-ci`、ローカル Windows) | 実行 | Pass |
| 注入実験 8 ケース(詳細は exp 記録) | 実行 | Pass — 全注入を期待 lane が検出 |
| doc index 再生成 | 実行 | Pass — 新規 blocking finding なし(#331 由来の reciprocal relation は plan 側で解消) |
| 3OS `verify` CI run(PR run) | 実行予定 | NotRun — PR 作成後に `gh pr checks --watch` で観測 |
| `collect-ci-metrics.mjs` 対象 run 追記 | 実行予定 | NotRun — PR run 完了後に jsonl へ追記 |

### 等価性証拠(注入実験サマリ)

`crates/ruviform-numeric` への一時注入を `lint-ci`/`test-ci`/`check-ci` で検証。全件 revert 済み(作業ツリー clean)。

| 注入 | 検出 lane | 結果 |
| --- | --- | --- |
| lib 型エラー(E0308) | lint-ci | Pass |
| lib 型エラー(対称確認) | check-ci | Pass( rustc 素でも検出されることを対象確認) |
| rustc 警告(unused variable、`-D warnings` 下) | lint-ci | Pass |
| clippy lint(cast_lossless) | lint-ci | Pass |
| cfg(test) 型エラー | lint-ci / test-ci | Pass |
| example 警告 | lint-ci | Pass |
| bench 警告 | lint-ci | Pass |
| 失敗テスト | test-ci | Pass(lint-ci は check-mode のためテスト実行なし=期待どおり非検出) |

## Notes / Rollback

- Rollback: `ci.yml` の verify job に `Type-check workspace (mise run check-ci)` step を `lint-ci` step の前に復元し、`mise.toml` の `verify-ci` に `mise run check-ci` を再挿入する。`check-ci` タスク自体は削除していないためタスク復元は不要。
- 改善が実測で確認できない場合は統合を戻す方針を spec/design に記録済み(測定次第で不採用も正当な成果)。
- `check-ci` 維持理由: clippy-driver と素の rustc で挙動差を疑う場合の切り分けに、check-only の rustc pass がローカルで有用。CI から外れてもタスクとして残す。
