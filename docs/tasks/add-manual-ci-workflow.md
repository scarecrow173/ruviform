---
id: "TASK-034IdQtDnY6ZKHFhFhXKmW"
type: "task"
status: "todo"
title: "手動 CI ワークフローの追加"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/2"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/dev-foundation-bootstrap.md"
  implemented-by: []
  depends-on:
    - "../plans/dev-foundation-bootstrap.md"
    - "define-mise-dev-and-ci-tasks.md"
  blocks:
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
    - "../test-specs/manual-ci-workflow.md"
    - "../test-specs/ci-profile-build-behavior.md"
    - "../test-specs/mise-verification-tasks.md"
  references: []
  defers: []
  deferred-by: []
---

# 手動 CI ワークフローの追加

## Work

plan `PLAN-034IdrispOUms1c32192B6` のスライス D（Phase D-1〜D-6）。`workflow_dispatch` 専用の GitHub Actions 検証ワークフロー `.github/workflows/ci.yml` を新規作成する。ブランチ上では静的検証（prek `check-yaml`・採用時 `actionlint`）に留め、実動作は default branch マージ後の手動ディスパッチで確認する運用を前提とする。

- [ ] `.github/workflows/ci.yml` を新規作成する。`on: workflow_dispatch` のみ（入力なし・`push` / `pull_request` / `schedule` なし）、`permissions: contents: read`、`ubuntu-latest` 単一 job（matrix なし）、job env に `CARGO_INCREMENTAL: "0"` を設定する
- [ ] ステップを design の構成どおり記述し、すべてに明示的な `name` を付ける: checkout → `df -h`（事前計測）→ `jdx/mise-action`（フルバージョン pin）＋ `mise install --locked` → `mise exec -- prek run --all-files` → `mise run fmt-check` → `mise run check-ci` → `mise run lint-ci` → `mise run test-ci` → 事後計測（`df -h`、`du -sh target`、`du -sh target/ci/deps target/ci/build` は存在する範囲で）。生の cargo コマンドは直接記述しない
- [ ] cargo cache ステップを追加するか判断し、追加しない場合は理由を作業メモに記録する。追加する場合は `~/.cargo` の registry/git index のみを `Cargo.lock` ハッシュでキー付け（例: `key: ${{ runner.os }}-cargo-${{ hashFiles('Cargo.lock') }}`）し、`target/` 全体・中間成果物はキャッシュしない
- [ ] `mise exec -- prek run --all-files` で `check-yaml` がワークフローを構文検証して通ることを確認する
- [ ] actionlint を採用する場合は `mise.toml` `[tools]` に `actionlint = "<pin>"` を追加して `mise install` → `mise lock` → `mise exec -- actionlint .github/workflows/ci.yml` を実行する。採用しない場合はその判断を作業メモに記録する（推奨: 採用。手動ディスパッチ前にブランチ上で検出可能な唯一の意味論検証のため）
- [ ] `workflow_dispatch` は default branch にワークフローが存在することが前提という制約を作業メモに記録し、本 PR マージ後に Actions UI から手動ディスパッチして実測ログ（`df -h` / `du -sh target`）を取得する手順を明記する

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dev-foundation-bootstrap.md](../plans/dev-foundation-bootstrap.md) — `PLAN-034IdrispOUms1c32192B6`（approved）。Phase D / V-6・V-7・V-8 の正本
- [../designs/dev-foundation.md](../designs/dev-foundation.md) — `DESIGN-034IddXFCRKDIW01WXkSla`。ワークフローのトリガー・権限・ステップ構成・キャッシュ方針の直接入力
- [../adr/rust-dev-foundation-toolchain-and-ci-profile.md](../adr/rust-dev-foundation-toolchain-and-ci-profile.md) — `ADR-034IcJdtqm6TP6MylyP5J7`（accepted）。`workflow_dispatch` 限定の CI 起動方針
- [../specs/establish-rust-dev-foundation.md](../specs/establish-rust-dev-foundation.md) — `SPEC-034IcwhwIFSNUNKtX7KK0Q`。Requirements §5（手動 CI）
- [../test-specs/manual-ci-workflow.md](../test-specs/manual-ci-workflow.md) — `TSPEC-034IdHt7P0ega4N0bbNANg`。手動ディスパッチ専用・最小権限・計測の契約
- [../test-specs/ci-profile-build-behavior.md](../test-specs/ci-profile-build-behavior.md) — `TSPEC-034IdHsItEuo0TZWTPOJUu`。CI が `--profile ci`・`CARGO_INCREMENTAL=0` を使う契約
- [../test-specs/mise-verification-tasks.md](../test-specs/mise-verification-tasks.md) — `TSPEC-034IdHshzpNiPTqyyvwL6J`。CI が mise タスク経由でローカルと同一コマンドを呼ぶ契約

## Dependencies

- 前提タスク: `define-mise-dev-and-ci-tasks.md`（`TASK-034IdQqbjQcZ0Ztsy5IPpm`）— ワークフローが呼び出す `fmt-check` / `check-ci` / `lint-ci` / `test-ci` のタスク名確定が前提
- 後続をブロック: `verify-reproducible-setup-and-work-memo.md`
- 並行可能: ワークフロー本文（D-1〜D-3）の記述はタスク名が design で確定済みのためスライス C の検証実行と並行できるが、検証完了は C の後

## Affected Files

- `.github/workflows/ci.yml` — 新規。`workflow_dispatch` 専用の検証ワークフロー（`.github/` は既存、`workflows/` は新設）
- `mise.toml` / `mise.lock` — actionlint 採用時のみ `[tools]` に pin 追記と lockfile 更新
- `docs/impl/ir/` — キャッシュ採否・actionlint 採否・default branch 制約と手動ディスパッチ手順を記録

## Done When

- [ ] `ci.yml` の `on:` が `workflow_dispatch` のみで、自動トリガー・入力項目を含まない
- [ ] `permissions` が `contents: read` のみ、単一 `ubuntu-latest` job、job env に `CARGO_INCREMENTAL: "0"` がある
- [ ] 検証ステップがすべて `mise run` タスク経由で、生の cargo コマンドを含まない
- [ ] 事前/事後のディスク計測ステップがあり、`target/` キャッシュがない（または `~/.cargo` registry のみ `Cargo.lock` ハッシュキー）
- [ ] 静的検証（`check-yaml`、採用時 `actionlint`）が通り、マージ後の手動ディスパッチ手順が記録されている

## Verification

- [ ] `mise exec -- prek run --all-files` exits 0（`check-yaml` が `ci.yml` の構文を検証）
- [ ] `mise exec -- actionlint .github/workflows/ci.yml` exits 0（actionlint 採用時。不採用なら理由が作業メモに記録済み）
- [ ] `grep -n 'workflow_dispatch' .github/workflows/ci.yml` が一致を返す
- [ ] `grep -nE '(push|pull_request|schedule):' .github/workflows/ci.yml` exits 1（自動トリガーなし）
- [ ] `grep -n 'contents: read' .github/workflows/ci.yml` と `grep -n 'CARGO_INCREMENTAL' .github/workflows/ci.yml` が一致を返す
- [ ] `grep -cE 'mise run (fmt-check|check-ci|lint-ci|test-ci)' .github/workflows/ci.yml` が `4` 以上を返し、`grep -nE 'cargo (fmt|clippy|test|check|build)' .github/workflows/ci.yml` exits 1（生 cargo なし）
- [ ] `grep -n 'df -h' .github/workflows/ci.yml` が事前・事後の計測ステップを返す
- [ ] Manual: default branch マージ後に Actions UI から手動ディスパッチし、計測値入りのログが取得できる（ブランチ上は静的確認のみで可。補助）
