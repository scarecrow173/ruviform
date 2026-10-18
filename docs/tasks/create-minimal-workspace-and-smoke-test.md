---
id: "TASK-034IdQmBa6UdauVn9IG59X"
type: "task"
status: "todo"
title: "最小ワークスペースと smoke test の構築"
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
  blocks:
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
    - "../test-specs/mise-verification-tasks.md"
    - "../test-specs/ci-profile-build-behavior.md"
    - "../test-specs/lockfile-reproducible-setup.md"
  references: []
  defers: []
  deferred-by: []
---

# 最小ワークスペースと smoke test の構築

## Work

plan `PLAN-034IdrispOUms1c32192B6` のスライス B（Phase B-1〜B-7）。workspace ルート `Cargo.toml` と最小メンバークレート `crates/ruviform-core` を新規作成し、smoke test が通る状態にする。`cargo new` / `cargo init` 等の雛形コマンドは使わず、生成物を完全制御するため全ファイル手書きで作成する。

- [ ] workspace ルート `Cargo.toml` を新規作成する。design の定義ブロック全体を verbatim で転記する: `[workspace]`（`resolver = "2"`、`members = ["crates/ruviform-core"]`）、`[workspace.package]`（`edition = "2021"`、`rust-version = "1.85"`、`license = "MIT"`。いずれも仮置きとコメントで明記）、`[workspace.lints.rust]`（`unsafe_code`・`unused_qualifications`）、`[workspace.lints.clippy]`（`pedantic`・`missing_errors_doc`）、`[profile.dev]`（`debug = "full"`）、`[profile.ci]`（`inherits = "dev"`・`debug = "line-tables-only"`・`incremental = false`）、`[profile.ci.package."*"]`（`debug = "none"`）
- [ ] `crates/ruviform-core/Cargo.toml` を新規作成する。`package.name = "ruviform-core"`、`edition.workspace = true`、`rust-version.workspace = true`、`license.workspace = true`、`[lints] workspace = true` を設定し、profile・独自 lint の重複は書かない（Cargo がメンバー側 profile を警告対象とするため）
- [ ] `crates/ruviform-core/src/lib.rs` を新規作成する。`pub fn version() -> &'static str { env!("CARGO_PKG_VERSION") }` 程度の最小公開 API と、`#[cfg(test)]` 内に値が空でないことを確認する smoke test を書く（デバイス非依存の純粋関数のみ）
- [ ] `mise exec -- cargo test --workspace` で smoke test が成功することを確認する
- [ ] `Cargo.lock` が生成され、ignore されていないことを確認して追跡対象にする（ADR の決定どおり Git 追跡）
- [ ] `.gitignore` に `target/` を追記する（既存エントリは変更しない）
- [ ] `git status` で `target/`・`Cargo.lock` 以外に意図しない生成物が出ていないことを確認する

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dev-foundation-bootstrap.md](../plans/dev-foundation-bootstrap.md) — `PLAN-034IdrispOUms1c32192B6`（approved）。Phase B / V-3・V-4・V-10 の正本
- [../designs/dev-foundation.md](../designs/dev-foundation.md) — `DESIGN-034IddXFCRKDIW01WXkSla`。workspace ルート `Cargo.toml` の verbatim ブロック定義・ディレクトリ構成の直接入力
- [../specs/establish-rust-dev-foundation.md](../specs/establish-rust-dev-foundation.md) — `SPEC-034IcwhwIFSNUNKtX7KK0Q`。Requirements §2（最小ワークスペース）・§3（ci プロファイル）
- [../test-specs/ci-profile-build-behavior.md](../test-specs/ci-profile-build-behavior.md) — `TSPEC-034IdHsItEuo0TZWTPOJUu`。プロファイル定義の置き場と成果物分離の契約
- [../test-specs/mise-verification-tasks.md](../test-specs/mise-verification-tasks.md) — `TSPEC-034IdHshzpNiPTqyyvwL6J`。「最小 workspace と smoke test で全タスクが成功する」保証の成立条件
- [../test-specs/lockfile-reproducible-setup.md](../test-specs/lockfile-reproducible-setup.md) — `TSPEC-034IdHmKPCUPXNnJh1Y7f7`。`Cargo.lock` 追跡・`target/` 非追跡の分離契約

## Dependencies

- 前提タスク: `pin-rust-toolchain-and-lockfile.md`（`TASK-034IdQiK2RW8ol4gpyUYLb`）— cargo toolchain が導入済みであること
- 後続をブロック: `define-mise-dev-and-ci-tasks.md`（タスクが操作する workspace が前提）、`verify-reproducible-setup-and-work-memo.md`
- 仮置き値（クレート名 `ruviform-core`・edition 2021・MSRV 1.85）は #1 で再評価される前提であり、仮置きであることをコメントで明記する

## Affected Files

- `Cargo.toml` — 新規。workspace ルート。プロファイル・`workspace.package`・`workspace.lints` の唯一の置き場
- `Cargo.lock` — cargo が生成。Git 追跡対象
- `crates/ruviform-core/Cargo.toml` — 新規。workspace ポリシー継承のみ
- `crates/ruviform-core/src/lib.rs` — 新規。最小 API + smoke test
- `.gitignore` — `target/` を追記（既存ルール保持）

## Done When

- [ ] workspace ルート `Cargo.toml` が design 定義と verbatim で一致し、メンバークレート側に profile・lint 重複がない
- [ ] `cargo test --workspace` で smoke test が成功する
- [ ] `Cargo.lock` が追跡対象で、`target/` が `.gitignore` により除外される
- [ ] `dev` と `ci` プロファイルの成果物が `target/debug/` と `target/ci/` に分離生成される

## Verification

- [ ] `mise exec -- cargo test --workspace` exits 0（smoke test 含む）
- [ ] `mise exec -- cargo build --workspace --profile ci` exits 0 かつ `test -d target/ci` exits 0
- [ ] `mise exec -- cargo build --workspace` exits 0 かつ `test -d target/debug` exits 0
- [ ] `git check-ignore target/` が `target/` を出力する
- [ ] `git check-ignore Cargo.lock` exits 1（ignore されていないこと）
- [ ] `grep -c '^\[profile\.' Cargo.toml` が `3` を返し、`grep -c '^\[profile' crates/ruviform-core/Cargo.toml` が `0` を返す
- [ ] `grep -n 'workspace = true' crates/ruviform-core/Cargo.toml` が `edition` / `rust-version` / `license` / `lints` の継承設定を返す
- [ ] Manual: `git status` で `target/`・`Cargo.lock`・新規ソース以外の意図しない生成物がないこと（補助）
