---
id: "TASK-034IdQqbjQcZ0Ztsy5IPpm"
type: "task"
status: "done"
title: "mise 開発・CI 検証タスクの定義"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/2"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/define-mise-dev-and-ci-tasks.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/dev-foundation-bootstrap.md"
  implemented-by:
    - "../impl/ir/define-mise-dev-and-ci-tasks.md"
  depends-on:
    - "../plans/dev-foundation-bootstrap.md"
    - "pin-rust-toolchain-and-lockfile.md"
    - "create-minimal-workspace-and-smoke-test.md"
  blocks:
    - "add-manual-ci-workflow.md"
    - "verify-reproducible-setup-and-work-memo.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/mise-verification-tasks.md"
    - "../test-specs/ci-profile-build-behavior.md"
  references: []
  defers: []
  deferred-by: []
---

# mise 開発・CI 検証タスクの定義

## Work

plan `PLAN-034IdrispOUms1c32192B6` のスライス C（Phase C-1〜C-6）。`mise.toml` の `[tasks.*]` に、ローカル開発と CI が共有する検証入口を一本化して定義する。変更は `[tasks.*]` への追記のみとし、既存の `[settings]` / `[env]` / `[tools]` / `[tasks.setup]` と `.pre-commit-config.yaml` は変更しない。

- [x] ローカル通常タスクを定義する: `[tasks.fmt]`（`cargo fmt --all`）、`[tasks.fmt-check]`（`cargo fmt --all -- --check`）、`[tasks.check]`（`cargo check --workspace --all-targets`）、`[tasks.lint]`（`cargo clippy --workspace --all-targets -- -D warnings`）、`[tasks.test]`（`cargo test --workspace`）、`[tasks.build]`（`cargo build --workspace`）
- [x] 各タスクに `description` を付け、実行コマンドと副作用を明記する（`fmt` はソースファイルを書き換える整形タスク、`fmt-check` / `check` / `lint` / `test` / `build` は非破壊で `target/` への成果物生成のみ、という区別が読み取れること）
- [x] CI 向けタスクを定義する: `[tasks.check-ci]`（`cargo check --workspace --all-targets --profile ci`）、`[tasks.lint-ci]`（`cargo clippy --workspace --all-targets --profile ci -- -D warnings`）、`[tasks.test-ci]`（`cargo test --workspace --profile ci`）、`[tasks.build-ci]`（`cargo build --workspace --profile ci`）。すべてに `env = { CARGO_INCREMENTAL = "0" }` を付与する。`cargo fmt` はプロファイル非依存のため `fmt-ci` は作らない
- [x] `[tasks.verify-ci]` を `run = ["mise run fmt-check", "mise run check-ci", "mise run lint-ci", "mise run test-ci"]`（順次実行の集約入口）と `env = { CARGO_INCREMENTAL = "0" }` で定義する
- [x] `mise run fmt` → `fmt-check` → `check` → `lint` → `test` → `build` を順に実行し、すべて成功することを確認する → 全て exit 0。`fmt` 実行後もソース差分なし（`git status --porcelain` で `.rs` 無変更）
- [x] `mise run verify-ci` を実行し `--profile ci` 配下の一連の検証が成功することと、`target/ci/` が生成されることを確認する → exit 0、`Finished 'ci' profile`・`target/ci/deps/` 配下のテストバイナリを確認、`test -d target/ci` exit 0
- [x] `mise.toml` / `mise.lock` の差分を確認し既存ツール・タスク定義が維持されていることを確認する。`.pre-commit-config.yaml` が未変更であることを確認し、prek の軽量フックと cargo の重い検証の分担を維持する（コミットフックに cargo 系検証を追加しない） → `mise.toml` は `[tasks.*]` 追記のみ（+62 行）、`mise.lock` 差分なし、`.pre-commit-config.yaml` 差分なし

検証内容（対象 workspace・`--all-targets`・`-D warnings`）は `*-ci` 系で重複定義せず、同一コマンドのプロファイル差分として扱う。`*-ci` タスクは CI 用途に限定しローカル常用しない旨は作業メモ側で記録する（`verify-reproducible-setup-and-work-memo.md`）。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dev-foundation-bootstrap.md](../plans/dev-foundation-bootstrap.md) — `PLAN-034IdrispOUms1c32192B6`（approved）。Phase C / V-5・V-6 の正本
- [../designs/dev-foundation.md](../designs/dev-foundation.md) — `DESIGN-034IddXFCRKDIW01WXkSla`。mise タスク表（コマンド・用途）の直接入力
- [../adr/rust-dev-foundation-toolchain-and-ci-profile.md](../adr/rust-dev-foundation-toolchain-and-ci-profile.md) — `ADR-034IcJdtqm6TP6MylyP5J7`（accepted）。mise タスク一本化の決定
- [../specs/establish-rust-dev-foundation.md](../specs/establish-rust-dev-foundation.md) — `SPEC-034IcwhwIFSNUNKtX7KK0Q`。Requirements §4（ローカル／CI 共通検証）
- [../test-specs/mise-verification-tasks.md](../test-specs/mise-verification-tasks.md) — `TSPEC-034IdHshzpNiPTqyyvwL6J`。入口一本化とプロファイル分離の契約
- [../test-specs/ci-profile-build-behavior.md](../test-specs/ci-profile-build-behavior.md) — `TSPEC-034IdHsItEuo0TZWTPOJUu`。`*-ci` タスクの `--profile ci`・`CARGO_INCREMENTAL=0` 明示義務

## Dependencies

- 前提タスク: `pin-rust-toolchain-and-lockfile.md`（toolchain 導入済み）、`create-minimal-workspace-and-smoke-test.md`（タスクが操作する workspace が存在）
- 後続をブロック: `add-manual-ci-workflow.md`（ワークフローが呼び出すタスク名をここで確定する）、`verify-reproducible-setup-and-work-memo.md`
- 並行可能: `add-manual-ci-workflow.md` のワークフロー記述はタスク名が design で確定済みのため並行着手できるが、完了検証は本タスクの後

## Affected Files

- `mise.toml` — `[tasks.*]` に fmt / fmt-check / check / lint / test / build / check-ci / lint-ci / test-ci / build-ci / verify-ci を追記（既存セクション・`[tasks.setup]` は不変）

## Done When

- [x] 11 個のタスクが `description` 付きで定義され、`*-ci` 系と `verify-ci` に `env = { CARGO_INCREMENTAL = "0" }` がある
- [x] `verify-ci` が `fmt-check` → `check-ci` → `lint-ci` → `test-ci` の順次実行である
- [x] ローカル全タスクと `verify-ci` が成功し、`target/ci/` が生成される
- [x] `.pre-commit-config.yaml` が未変更で、コミットフックに cargo 系検証が追加されていない

## Verification

- [x] `mise run fmt-check` exits 0
- [x] `mise run check && mise run lint && mise run test && mise run build` exits 0
- [x] `mise run verify-ci` exits 0
- [x] `test -d target/ci` exits 0
- [x] `grep -n 'CARGO_INCREMENTAL' mise.toml` が `*-ci`・`verify-ci` タスクの env 設定を返す → 5 行（check-ci/lint-ci/test-ci/build-ci/verify-ci）
- [x] `git diff --exit-code -- .pre-commit-config.yaml` exits 0
- [x] Manual: `mise tasks` の一覧で新規タスクと `description` が確認でき、既存 `setup` が維持されている（補助） → 12 タスク一覧で確認済み
