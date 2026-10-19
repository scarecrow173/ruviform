---
id: "IMPL-034IedmXdoFo5aI22WR8m8"
type: "impl"
status: "completed"
title: "Define mise dev and CI tasks"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/2"
  changes:
    added: []
    modified:
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/define-mise-dev-and-ci-tasks.md"
  implemented-by: []
  depends-on:
    - "pin-rust-toolchain-and-lockfile.md"
    - "create-minimal-workspace-and-smoke-test.md"
  blocks:
    - "../../tasks/add-manual-ci-workflow.md"
    - "../../tasks/verify-reproducible-setup-and-work-memo.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/dev-foundation-bootstrap.md"
    - "../../designs/dev-foundation.md"
    - "../../adr/rust-dev-foundation-toolchain-and-ci-profile.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/mise-verification-tasks.md"
    - "../../test-specs/ci-profile-build-behavior.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Define mise dev and CI tasks

## Summary

task `TASK-034IdQqbjQcZ0Ztsy5IPpm`（plan `PLAN-034IdrispOUms1c32192B6` スライス C / Phase C-1〜C-6）として、`mise.toml` の `[tasks.*]` にローカル開発と CI が共有する検証入口 11 タスク（`fmt` / `fmt-check` / `check` / `lint` / `test` / `build` / `check-ci` / `lint-ci` / `test-ci` / `build-ci` / `verify-ci`）を `description` 付きで追記した。`*-ci` 系と `verify-ci` には `env = { CARGO_INCREMENTAL = "0" }` を付与し、`verify-ci` は `fmt-check` → `check-ci` → `lint-ci` → `test-ci` の順次実行の集約入口とした。既存の `[settings]` / `[env]` / `[tools]` / `[tasks.setup]` および `.pre-commit-config.yaml` は不変。全タスクを実機実行して exit 0 を確認し、`target/ci/` の分離生成も確認した。

## Context

- 実施タスク: `docs/tasks/define-mise-dev-and-ci-tasks.md`（C-1〜C-6）
- 上流: `docs/plans/dev-foundation-bootstrap.md`（Phase C / V-5・V-6）、`docs/designs/dev-foundation.md`（mise タスク設計表・設計上の規則）、ADR `ADR-034IcJdtqm6TP6MylyP5J7`（mise タスク一本化）、test-spec `TSPEC-034IdHshzpNiPTqyyvwL6J`（入口一本化・プロファイル分離）・`TSPEC-034IdHsItEuo0TZWTPOJUu`（`--profile ci`・`CARGO_INCREMENTAL=0` 明示義務）
- 前提: スライス A（`IMPL-034IdUNIqDaqtMaA6rRfEx`）で `rust = "1.99.0"` が mise pin・導入済み。スライス B（`IMPL-034Ie9ddczoaNNz2v7zAYp`）で最小 workspace と `[profile.ci]` 定義が存在し、生の cargo コマンドでの同等検証はすべて成功済み。

## Implementation

### Changes

- `mise.toml` — `[tasks.setup]` の後に追記のみ（62 行追加、既存行の変更なし）:
  - ローカル通常タスク: `[tasks.fmt]`（`cargo fmt --all`）、`[tasks.fmt-check]`（`cargo fmt --all -- --check`）、`[tasks.check]`（`cargo check --workspace --all-targets`）、`[tasks.lint]`（`cargo clippy --workspace --all-targets -- -D warnings`）、`[tasks.test]`（`cargo test --workspace`）、`[tasks.build]`（`cargo build --workspace`）。`dev` / `test` プロファイル（フルデバッグ情報）を維持。
  - CI 向けタスク: `[tasks.check-ci]` / `[tasks.lint-ci]` / `[tasks.test-ci]` / `[tasks.build-ci]`。同一コマンドに `--profile ci` を付けたプロファイル差分のみで、検証内容（workspace・`--all-targets`・`-D warnings`）の重複定義はなし。すべて `env = { CARGO_INCREMENTAL = "0" }` 付き。成果物は `target/ci/` に分離される。
  - 集約タスク: `[tasks.verify-ci]` は `run = ["mise run fmt-check", "mise run check-ci", "mise run lint-ci", "mise run test-ci"]`（配列 = 順次実行）と `env = { CARGO_INCREMENTAL = "0" }`。
- `description` の副作用表記: `fmt` は「rewrites source files in place」と書き換え系であることを明示し、それ以外は「non-destructive; writes only target/」系の表現で統一し、test-spec の「書き換え系と非破壊チェックの区別が読み取れる」規則を満たした。セクションコメントでも同じ区別と、`*-ci` が CI 用途限定である旨を記述した。

### Decisions During Implementation

- **`fmt-ci` を作らない**: design / test-spec どおり `cargo fmt` はプロファイル非依存のため `fmt-ci` は定義しない。`verify-ci` の先頭はプロファイル非依存の `fmt-check` とした。
- **`verify-ci` は `run` 配列で `mise run` を順次呼ぶ形**: design が定義する `run = ["mise run fmt-check", ...]` を verbatim で採用。`depends` ではなく `run` 配列とすることで順次実行を保証する。
- **description の言語**: 既存 `[tasks.setup]` と揃えて英語とした（本文書ドキュメント群は日本語だが `mise.toml` 内の既存記述に倣った）。
- **`--profile ci` の位置**: `cargo check --workspace --all-targets --profile ci` など design のタスク表どおりのコマンド列を採用（clippy は `-- -D warnings` の前に `--profile ci`）。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64、mise 2026.10.1、Rust 1.99.0。task の `## Verification` と plan V-5 を実機実行した結果:

| Check | コマンド | 結果 |
| --- | --- | --- |
| タスク一覧 | `mise tasks` | exit 0。新規 11 タスクと `description` を表示、既存 `setup` も維持 |
| 整形（書き込み） | `mise run fmt` | exit 0。実行後 `git status --porcelain -- '*.rs'` は空（ソース差分なし）。続けて `mise run fmt-check` も exit 0 |
| fmt-check | `mise run fmt-check` | exit 0 |
| check | `mise run check` | exit 0。`Finished 'dev' profile` |
| lint | `mise run lint` | exit 0（`-D warnings` ゲート通過） |
| test | `mise run test` | exit 0。unit test 1 件 + doctest 1 件成功 |
| build | `mise run build` | exit 0。`Finished 'dev' profile` |
| verify-ci | `mise run verify-ci` | exit 0。ログ上 `fmt-check` → `check-ci` → `lint-ci` → `test-ci` の順に実行され、`check-ci`/`lint-ci`/`test-ci` が `Finished 'ci' profile`・テストバイナリが `target/ci/deps/` 配下であることを確認 |
| build-ci | `mise run build-ci` | exit 0。`Finished 'ci' profile` |
| ci 成果物分離 | `test -d target/ci` | exit 0（`target/ci` 存在。`target/debug/` と分離） |
| env 明示 | `grep -n 'CARGO_INCREMENTAL' mise.toml` | 5 行（check-ci/lint-ci/test-ci/build-ci/verify-ci 各 `env`） |
| prek 未変更 | `git diff --exit-code -- .pre-commit-config.yaml` | exit 0（差分なし。コミットフックに cargo 系検証なし） |
| mise.lock 差分 | `git diff --stat -- mise.lock` | 出力なし（差分なし。既存ツール・ロック維持） |
| mise.toml 差分 | `git diff --stat -- mise.toml` | `62 insertions(+)` のみ（`[tasks.*]` 追記、既存行不変） |
| 生成物の範囲 | `git status --porcelain` | `M mise.toml`・`M` 本 task・`??` 本 IR のみ。`target/` は ignore 済み、fmt 実行後もソース差分なし |

注意: `check-ci` / `lint-ci` / `test-ci` の個別 `mise run` は `verify-ci` 内部の `mise run <task>` 呼び出しとして実行済み（同一のタスク入口・env 伝播を実機確認）。

## Risks

- ローカルで `*-ci` / `verify-ci` を常用すると `target/ci/` と `target/debug/` が並存して容量を圧迫する。CI 用途限定の旨は `mise.toml` のコメントと description に記載済みだが、作業メモ側での正式な記録は後続タスク `verify-reproducible-setup-and-work-memo.md`（Phase E）の責務。
- 実施 OS は Windows x64 のみ。macOS/Linux での同等検証は未実施（Phase E でクリーン作業ツリー検証を実施予定）。

## Follow-ups

- スライス D（`add-manual-ci-workflow.md`）はここで確定したタスク名（`fmt-check` / `check-ci` / `lint-ci` / `test-ci`、必要時 `build-ci`）をワークフローから `mise run` 経由で呼ぶ。生の cargo コマンドは書かない。
- スライス E（`verify-reproducible-setup-and-work-memo.md`）で `*-ci` ローカル常用禁止・`target/ci`/`target/debug` 並存の注意を作業メモに記録する。
- `mise.toml` 差分・本 IR・task 更新のコミットは orchestrator が実施する。
