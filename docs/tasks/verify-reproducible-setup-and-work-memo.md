---
id: "TASK-034IdQxdPQUxXweHBN1ZxT"
type: "task"
status: "todo"
title: "再現インストール検証と #3 転記用作業メモ"
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
    - "pin-rust-toolchain-and-lockfile.md"
    - "create-minimal-workspace-and-smoke-test.md"
    - "define-mise-dev-and-ci-tasks.md"
    - "add-manual-ci-workflow.md"
  blocks: []
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
    - "../test-specs/lockfile-reproducible-setup.md"
    - "../test-specs/mise-verification-tasks.md"
    - "../test-specs/ci-profile-build-behavior.md"
    - "../test-specs/manual-ci-workflow.md"
  references: []
  defers: []
  deferred-by: []
---

# 再現インストール検証と #3 転記用作業メモ

## Work

plan `PLAN-034IdrispOUms1c32192B6` のスライス E（Phase E-1〜E-6）。スライス A〜D の成果物が「新規 clone から固定 lockfile で再現できる」ことを実機で検証し、暫定開発規約・前提条件・未検証 OS 等の #3 転記材料を `docs/impl/ir/` の Implementation Record と実施 PR 本文に記録する。本タスクは検証と記録が主体で、既存の正本ファイルは変更しない。

- [ ] `mise install --locked` を実行し、lockfile から toolchain（`cargo` / `rustfmt` / `clippy` 含む）が復元できることを確認する
- [ ] `mise exec -- apm install --frozen` の成否を実機確認する。未対応なら `apm install` の再現性を確認する同等手順（`apm.lock.yaml` 差分なしの確認等）を定義して記録する
- [ ] `git status --ignored` と `git check-ignore` で、`apm_modules/`・`.agents/`・`.env*`・`target/`・Serena ローカル状態等が意図どおり非追跡であり、`Cargo.lock`・`mise.lock`・`apm.lock.yaml` が追跡対象であることを確認する
- [ ] `.gitattributes` の audio/video 一律 LFS を確認し、「小さいテストフィクスチャは非メディア形式（JSON/YAML 等）で Git に置く」「CI ではフィクスチャが必要になるまで `git lfs pull` を省略する」方針の確認結果を記録する
- [ ] 暫定開発規約（`unsafe` 原則禁止＋SAFETY コメント付き局所許可、default feature 最小、依存追加は理由記録、`workspace.lints` 正本＋`-D warnings` ゲート、公開 API の例は doctest）を design どおり作業メモへ転記する
- [ ] 実施した OS・実行したコマンド・検証結果・前提条件（ネットワーク/権限/認証）・lockfile 再生成条件・未検証 OS を `docs/impl/ir/` の IR と実施 PR 本文に記録し、#3 が転記可能な状態にする

新規 clone 再現性の追加確認が必要な場合は、`git worktree` 等のクリーンな作業ツリーで V-1 / V-2 / V-5 相当を再実行する。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dev-foundation-bootstrap.md](../plans/dev-foundation-bootstrap.md) — `PLAN-034IdrispOUms1c32192B6`（approved）。Phase E / V-1・V-2・V-9・V-11 の正本
- [../designs/dev-foundation.md](../designs/dev-foundation.md) — `DESIGN-034IddXFCRKDIW01WXkSla`。暫定開発規約・LFS フィクスチャ方針・Verification Notes の転記元
- [../specs/establish-rust-dev-foundation.md](../specs/establish-rust-dev-foundation.md) — `SPEC-034IcwhwIFSNUNKtX7KK0Q`。Requirements §1・§6（再現セットアップ・作業メモ記録）
- [../test-specs/lockfile-reproducible-setup.md](../test-specs/lockfile-reproducible-setup.md) — `TSPEC-034IdHmKPCUPXNnJh1Y7f7`。lockfile 復元・追跡/非追跡分離・記録義務の契約
- [../test-specs/mise-verification-tasks.md](../test-specs/mise-verification-tasks.md) — `TSPEC-034IdHshzpNiPTqyyvwL6J`。`*-ci` タスクの CI 用途限定運用の記録対象
- [../test-specs/ci-profile-build-behavior.md](../test-specs/ci-profile-build-behavior.md) — `TSPEC-034IdHsItEuo0TZWTPOJUu`。`target/ci/`・`target/debug/` 並存の運用注意の記録対象
- [../test-specs/manual-ci-workflow.md](../test-specs/manual-ci-workflow.md) — `TSPEC-034IdHt7P0ega4N0bbNANg`。ブランチ上は静的検証・マージ後手動ディスパッチの運用明記対象

## Dependencies

- 前提タスク: `pin-rust-toolchain-and-lockfile.md`、`create-minimal-workspace-and-smoke-test.md`、`define-mise-dev-and-ci-tasks.md`、`add-manual-ci-workflow.md`（スライス A〜D すべての成果物が検証対象）
- 後続をブロック: なし（クリティカルパスの終端）
- 各先行タスクで未作成の場合、本タスクで `docs/impl/ir/` の IR を impl-doc 規約に従い作成する

## Affected Files

- `docs/impl/ir/` — Implementation Record を作成・更新し、暫定開発規約・LFS 方針・検証済み OS・前提条件・lockfile 再生成条件・未検証 OS を記録
- 実施 PR 本文 — IR と同内容の #3 転記用サマリを記録
- その他の既存ファイル — 変更しない（検証・確認のみ）

## Done When

- [ ] `mise install --locked` と APM の再現インストール（または記録済みの同等手順）が成功する
- [ ] 追跡対象（3 lockfile）と非追跡対象（`target/`・`apm_modules/`・`.env*` 等）の分離が確認されている
- [ ] IR / PR 本文に暫定開発規約・LFS フィクスチャ方針・実施 OS・検証コマンドと結果・前提条件・lockfile 再生成条件・未検証 OS が記録され、#3 が転記可能である
- [ ] `apm install --frozen` の対応可否と、未対応時の同等手順が記録されている

## Verification

- [ ] `mise install --locked` exits 0
- [ ] `mise exec -- cargo --version && mise exec -- cargo fmt --version && mise exec -- cargo clippy --version` exits 0（復元された toolchain の疎通）
- [ ] `mise exec -- apm install --frozen` exits 0（未対応なら `mise exec -- apm install` 後に `git diff --exit-code -- apm.lock.yaml` が exits 0、かつ同等手順が IR に記録済み）
- [ ] `git ls-files Cargo.lock mise.lock apm.lock.yaml` が 3 ファイルすべてを出力する
- [ ] `git check-ignore target/ apm_modules/ .agents/` が各パスを出力する
- [ ] `git status --ignored` で意図しない追跡ファイル・未除外の生成物がないことを確認する
- [ ] `ls docs/impl/ir/*.md` が IR を返し、`grep -rE '未検証|mise install --locked|lockfile' docs/impl/ir/` が記録内容に一致を返す
- [ ] Manual: IR と実施 PR 本文の記録項目（暫定開発規約・LFS 方針・OS・前提条件・再生成条件）が揃い #3 が転記可能（補助）
