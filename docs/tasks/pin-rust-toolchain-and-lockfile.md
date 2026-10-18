---
id: "TASK-034IdQiK2RW8ol4gpyUYLb"
type: "task"
status: "todo"
title: "Rust toolchain pin と lockfile 確定"
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
  blocks:
    - "create-minimal-workspace-and-smoke-test.md"
    - "define-mise-dev-and-ci-tasks.md"
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
    - "../test-specs/lockfile-reproducible-setup.md"
  references: []
  defers: []
  deferred-by: []
---

# Rust toolchain pin と lockfile 確定

## Work

plan `PLAN-034IdrispOUms1c32192B6` のスライス A（Phase A-1〜A-8）。`mise.toml` の `[tools]` に具体的 stable バージョンで Rust toolchain を pin し、`mise install` → `mise lock` で `mise.lock` に確定値を記録する。既存のツール・`[tasks.setup]`・APM/prek/Git LFS 運用を壊さないことが前提条件。

- [ ] `mise ls-remote rust` で利用可能な stable を確認し、`minimum_release_age = "7d"` を考慮した最新の具体バージョンを選定する（`stable` / `latest` の浮動指定は禁止。design 仮置き MSRV `1.85` 以上であること）
- [ ] `mise.toml` の `[tools]` に `rust = "<選定バージョン>"` を追記する（既存エントリ・`[settings]`・`[env]`・`[tasks.setup]` は変更しない）
- [ ] `mise install` で toolchain を導入する
- [ ] `cargo` / `rustfmt` / `clippy` の疎通を確認する。component が欠ける場合は rustup component 追加手順を `setup` タスクに明示する
- [ ] `mise lock` で `mise.lock` を更新し、rust エントリが記録されたことを確認する
- [ ] `mise run setup` を再実行し、`apm install` / `prek install` / `git lfs install --local` が成功すること（既存運用の無破壊）を確認する
- [ ] `mise exec -- apm lock` で `apm.lock.yaml` の整合を確認し、`apm.yml` との差分がないことを確認する
- [ ] 選定した Rust バージョンと根拠（stable 最新・仮置き MSRV 以上）を `docs/impl/ir/` の Implementation Record（未作成なら impl-doc 規約で作成）と実施 PR 本文に記録する

破壊的再初期化（`apm init --yes` 等）は行わない。`.github/`・`Cargo.toml`・クレート作成は後続タスクの範囲。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dev-foundation-bootstrap.md](../plans/dev-foundation-bootstrap.md) — `PLAN-034IdrispOUms1c32192B6`（approved）。Phase A / V-1・V-2 の正本
- [../designs/dev-foundation.md](../designs/dev-foundation.md) — `DESIGN-034IddXFCRKDIW01WXkSla`。toolchain pin 方針（`rust-toolchain.toml` 併用禁止・lockfile 運用手順）
- [../adr/rust-dev-foundation-toolchain-and-ci-profile.md](../adr/rust-dev-foundation-toolchain-and-ci-profile.md) — `ADR-034IcJdtqm6TP6MylyP5J7`（accepted）。toolchain pin 方法の決定
- [../specs/establish-rust-dev-foundation.md](../specs/establish-rust-dev-foundation.md) — `SPEC-034IcwhwIFSNUNKtX7KK0Q`。Requirements §1（lockfile による再現セットアップ）
- [../test-specs/lockfile-reproducible-setup.md](../test-specs/lockfile-reproducible-setup.md) — `TSPEC-034IdHmKPCUPXNnJh1Y7f7`。本タスクが満たす検証契約（最終的な復元検証は `verify-reproducible-setup-and-work-memo.md` で実施）

## Dependencies

- 前提: なし（クリティカルパス A → B → C → D → E の先頭）
- 先行ドキュメント: `../plans/dev-foundation-bootstrap.md`（approved）
- 後続をブロック: `create-minimal-workspace-and-smoke-test.md`（cargo が必要）、`define-mise-dev-and-ci-tasks.md`（toolchain と workspace が前提）、`verify-reproducible-setup-and-work-memo.md`（`mise.lock` 復元対象を確定する）

## Affected Files

- `mise.toml` — `[tools]` に `rust = "<pin バージョン>"` を追記（それ以外は不変）
- `mise.lock` — `mise lock` で再生成し rust エントリを確定（追跡継続）
- `docs/impl/ir/` — 選定バージョン・根拠を記録する IR（未作成ならこのタスクで作成し随時更新）

## Done When

- [ ] `mise.toml` に `rust` が具体バージョンで pin され、`mise.lock` に rust エントリが記録されている
- [ ] `cargo` / `rustfmt` / `clippy` が疎通し、欠落時の component 追加手順が必要なら `setup` タスクに明示されている
- [ ] `mise run setup` が成功し、`apm.yml` / `apm.lock.yaml` に意図しない差分がない
- [ ] 選定根拠が IR / 実施 PR に記録されている

## Verification

- [ ] `mise exec -- cargo --version` exits 0
- [ ] `mise exec -- cargo fmt --version` exits 0
- [ ] `mise exec -- cargo clippy --version` exits 0
- [ ] `git ls-files mise.lock apm.lock.yaml` が両ファイルを出力する
- [ ] `git diff --exit-code -- apm.yml` exits 0
- [ ] `mise run setup` exits 0
- [ ] `grep -n 'rust = "' mise.toml` が `stable` / `latest` 以外の具体バージョン指定を返す
- [ ] Manual: `mise.lock` に rust エントリが存在し、選定根拠が IR / PR に記録されている（補助）
