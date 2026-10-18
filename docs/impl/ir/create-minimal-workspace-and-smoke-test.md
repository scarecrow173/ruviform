---
id: "IMPL-034Ie9ddczoaNNz2v7zAYp"
type: "impl"
status: "completed"
title: "Create minimal workspace and smoke test"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/2"
  changes:
    added:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/ruviform-core/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-core/src/lib.rs"
    modified:
      - type: "file"
        path: ".gitignore"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "../../tasks/create-minimal-workspace-and-smoke-test.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/dev-foundation-bootstrap.md"
    - "../../designs/dev-foundation.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/ci-profile-build-behavior.md"
    - "../../test-specs/mise-verification-tasks.md"
    - "../../test-specs/lockfile-reproducible-setup.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Create minimal workspace and smoke test

## Summary

task `TASK-034IdQmBa6UdauVn9IG59X`（plan `PLAN-034IdrispOUms1c32192B6` スライス B）として、workspace ルート `Cargo.toml` と最小メンバークレート `crates/ruviform-core` を全ファイル手書きで新規作成し（`cargo new`/`cargo init` は未使用）、smoke test が成功する状態にした。`.gitignore` に `target/` を追記し、cargo が生成した `Cargo.lock` は ADR の決定どおり Git 追跡対象（ignore 非適用・コミット保留）とした。

## Context

- 実施タスク: `docs/tasks/create-minimal-workspace-and-smoke-test.md`（B-1〜B-7）
- 上流: `docs/plans/dev-foundation-bootstrap.md`（Phase B）、`docs/designs/dev-foundation.md`（workspace ルート `Cargo.toml` の verbatim ブロック・ディレクトリ構成）、test-spec `TSPEC-034IdHsItEuo0TZWTPOJUu`（ci プロファイル成果物分離）・`TSPEC-034IdHshzpNiPTqyyvwL6J`・`TSPEC-034IdHmKPCUPXNnJh1Y7f7`（lockfile 追跡／`target/` 非追跡分離）
- 前提: スライス A（`IMPL-034IdUNIqDaqtMaA6rRfEx`）で `rust = "1.99.0"` が mise pin・導入済み。design の仮置き MSRV `1.85` を満たす

## Implementation

### Changes

- `Cargo.toml`（新規）— workspace ルート。design `DESIGN-034IddXFCRKDIW01WXkSla` の定義ブロック全体を verbatim で転記: `[workspace]`（`resolver = "2"`、`members = ["crates/ruviform-core"]`）、`[workspace.package]`（`edition = "2021"`・`rust-version = "1.85"`・`license = "MIT"`、いずれも仮置きコメント付き）、`[workspace.lints.rust]`（`unsafe_code`・`unused_qualifications`）、`[workspace.lints.clippy]`（`pedantic`・`missing_errors_doc`）、`[profile.dev]`（`debug = "full"`）、`[profile.ci]`（`inherits = "dev"`・`debug = "line-tables-only"`・`incremental = false`）、`[profile.ci.package."*"]`（`debug = "none"`）
- `crates/ruviform-core/Cargo.toml`（新規）— `name = "ruviform-core"`（仮置き名とコメントで明記）、`version = "0.1.0"`（仮置き）、`edition.workspace`・`rust-version.workspace`・`license.workspace`・`[lints] workspace = true` で共通ポリシー継承。profile・独自 lint の重複記述なし
- `crates/ruviform-core/src/lib.rs`（新規）— `#[must_use] pub fn version() -> &'static str { env!("CARGO_PKG_VERSION") }` の最小公開 API（デバイス非依存の純粋関数）、doc 内 doctest、`#[cfg(test)]` 内に値が空でないことを確認する smoke test
- `.gitignore` — 末尾に `# Rust build artifacts` セクションとして `target/` を追記（既存エントリは不変）
- `Cargo.lock` — `cargo` が自動生成（`version = 4`、`ruviform-core 0.1.0` のみ）。追跡対象であり ignore されていないことを確認。コミットは orchestrator のコミット単位に委ね、本タスクでは未コミットのまま保持

### Decisions During Implementation

- **手書き作成**: `cargo new`/`cargo init` 等の雛形コマンドは使用せず、生成物を完全制御するため全ファイルを手書きした（plan Risk #6・spec 契約）。
- **smoke test の assertion 形式**: 初版は `assert!(!version().is_empty())` としたが、`cargo clippy -- -D warnings` で pedantic lint `clippy::assert_is_empty` が `error` となった。意図（非空確認）を保ったまま `assert_ne!(version(), "")` に変更し、スライス C の `lint`/`lint-ci` ゲート（`-D warnings`）で失敗しない形にした。doctest 側も同一形式に揃えた。
- **`version` フィールド**: メンバークレートに `version` は必須（`[workspace.package]` に `version` を定義していないため継承元もない）のため `version = "0.1.0"` を仮置きと明記して付与した。
- **`#[must_use]` / doc コメント**: pedantic の `must_use_candidate` 対策と暫定開発規約「公開 API の例は doctest」に合わせて付与した。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64、mise 2026.10.1、Rust 1.99.0（`cargo 1.99.0`、`rustc 1.99.0`）。task の `## Verification` 全項目を実機実行した結果:

| Check | コマンド | 結果 |
| --- | --- | --- |
| smoke test | `mise exec -- cargo test --workspace` | exit 0。`tests::version_is_not_empty` ok + doctest `version (line 11)` ok |
| dev build | `mise exec -- cargo build --workspace` | exit 0。`Finished 'dev' profile` |
| dev 成果物 | `test -d target/debug` | exit 0（`target/debug` 存在） |
| ci build | `mise exec -- cargo build --workspace --profile ci` | exit 0。`Finished 'ci' profile` |
| ci 成果物 | `test -d target/ci` | exit 0（`target/ci` 存在。dev/ci 成果物が `target/debug`/`target/ci` に分離生成） |
| target 除外 | `git check-ignore target/` | exit 0、`target/` を出力 |
| Cargo.lock 非除外 | `git check-ignore Cargo.lock` | exit 1（ignore されていない） |
| profile 定義の唯一性（root） | `grep -c '^\[profile\.' Cargo.toml` | `3`（`dev`・`ci`・`ci.package."*"`） |
| profile 重複なし（member） | `grep -c '^\[profile' crates/ruviform-core/Cargo.toml` | `0`（grep exit 1） |
| workspace 継承 | `grep -n 'workspace = true' crates/ruviform-core/Cargo.toml` | 4 行: `edition.workspace`・`rust-version.workspace`・`license.workspace`・`[lints] workspace` |
| 生成物の範囲 | `git status --porcelain` / `git status --ignored` | `M .gitignore`・`M` 本 task、`?? Cargo.toml`・`?? Cargo.lock`・`?? crates/`・`??` 本 IR、`!! target/`。意図しない生成物なし |

追加で実施した確認（スライス C のゲートを先取りし、本スライスが次段を壊さないことの確認）:

- `mise exec -- cargo check --workspace --all-targets` — exit 0、警告なし（メンバー側 profile 警告もなし）。
- `mise exec -- cargo fmt --all -- --check` — exit 0（手書きファイルが rustfmt-clean）。
- `mise exec -- cargo clippy --workspace --all-targets -- -D warnings` — exit 0（`assert_ne!` 修正後）。
- `CARGO_INCREMENTAL=0 mise exec -- cargo check --workspace --all-targets --profile ci` — exit 0。
- `CARGO_INCREMENTAL=0 mise exec -- cargo test --workspace --profile ci` — exit 0（unit + doctest ともに `target/ci/deps` 配下で成功）。
- `CARGO_INCREMENTAL=0 mise exec -- cargo clippy --workspace --all-targets --profile ci -- -D warnings` — exit 0。
- workspace ルート `Cargo.toml` の全ブロックが design `DESIGN-034IddXFCRKDIW01WXkSla` の定義と verbatim で一致することを目視確認。

## Risks

- 仮置き値（クレート名 `ruviform-core`・`version = "0.1.0"`・edition 2021・MSRV 1.85・license MIT）は #1 で再評価される前提であり、コメントで仮置きと明記済み。
- 実施 OS は Windows x64 のみ。macOS/Linux での同等検証は未実施（Phase E でクリーン作業ツリー検証を実施予定）。
- `Cargo.lock` は本報告時点で未コミット（orchestrator のコミット単位に委譲）。`git ls-files Cargo.lock` が空なのはそのためで、追跡方針自体は `git check-ignore` exit 1 で確認済み。

## Follow-ups

- スライス C（`define-mise-dev-and-ci-tasks.md`）で `mise` タスク経由の `fmt`/`check`/`lint`/`test`/`build`・`*-ci`・`verify-ci` を定義する。本スライスで生の cargo コマンドによる同等検証はすべて exit 0 を確認済み。
- `Cargo.lock`・新規 `Cargo.toml`・`crates/`・`.gitignore` 差分・本 IR・task 更新のコミットは orchestrator が実施する。
