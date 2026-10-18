---
id: "TSPEC-034IdHsItEuo0TZWTPOJUu"
type: "test-spec"
status: "proposed"
title: "ci プロファイルのビルド挙動と成果物分離"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/establish-rust-dev-foundation.md"
    - "../designs/dev-foundation.md"
    - "../adr/rust-dev-foundation-toolchain-and-ci-profile.md"
  derived-by: []
  verifies:
    - "../specs/establish-rust-dev-foundation.md"
    - "../designs/dev-foundation.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ci プロファイルのビルド挙動と成果物分離

## Purpose

「ローカル `dev` のフルデバッグ情報を削らず、CI の容量対策を専用プロファイル `ci` に分離する」という意図を保護する。[spec](../specs/establish-rust-dev-foundation.md)（Requirements §3）・[design](../designs/dev-foundation.md)（workspace ルート `Cargo.toml` の設計）・[ADR](../adr/rust-dev-foundation-toolchain-and-ci-profile.md)（`release` 流用を退けた判断）が定める不変条件を固定する。DSP・リアルタイムスレッドのデバッグ要件と cheaper PR #109 の `No space left on device` 事例の両方から生まれた契約であり、将来「`dev` を削って CI を軽くする」「`release` を CI に流用する」ような単純化を防ぐ。廃止してよいのは、プロファイル設計の再評価（#1 以降）で本構成を置き換える ADR が採用されたとき。

## Feature

ci プロファイルのビルド挙動と成果物分離: workspace ルート `Cargo.toml` のみに定義された `[profile.dev]` / `[profile.ci]` / `[profile.ci.package."*"]` により、ローカルはフルデバッグ、CI は `dev` 継承の検証条件を保った軽量成果物を得ること。

## Rules

- `[profile.dev]`・`[profile.ci]`・`[profile.ci.package."*"]` は workspace ルート `Cargo.toml` のみに定義し、メンバークレートのマニフェストで profile を重複させない（Cargo がメンバー側 profile を警告対象とするため）。
- `[profile.dev]` は `debug = "full"` を維持し、CI の容量対策を `dev` に波及させない。`[profile.debugging]` は作らない。
- `[profile.ci]` は `inherits = "dev"` により `opt-level=0`・`debug-assertions`・`overflow-checks` の開発時に近い検証条件を保ち、`debug = "line-tables-only"`・`incremental = false` とする。
- `[profile.ci.package."*"]` の `debug = "none"` はワークスペース外依存のみに適用され、workspace メンバーは `line-tables-only` でバックトレースのファイル名・行番号を確保する。
- CI 検証（`check` / `clippy` / `test`、必要時 `build`）は `--profile ci` と `CARGO_INCREMENTAL=0` を明示し、`release` やデバッグ削減済み `dev` を CI に流用しない。ローカル dev の増分コンパイルは Cargo 標準動作を維持する。
- `ci` プロファイルの成果物は `target/ci/` に、`dev` の成果物は `target/debug/` に分離生成される。

## Examples

- Given: 構築済みの workspace ルート `Cargo.toml`。When: profile ブロックを確認する。Then: `[profile.dev]` / `[profile.ci]` / `[profile.ci.package."*"]` が design の定義と verbatim で一致し、メンバー `crates/ruviform-core/Cargo.toml` に profile 記述がない。
- Given: 最小 workspace。When: `cargo build --workspace --profile ci` と `cargo build --workspace` を実行する。Then: 両方 exit 0 で、`target/ci/` と `target/debug/` が分離生成される。
- Given: `--profile ci` でのビルド。When: `CARGO_INCREMENTAL=0` が設定された環境で実行する。Then: `incremental = false` と環境変数の二重指定により増分コンパイルの中間成果物が生成されない。
- Given: CI での `release` 流用案。When: プロファイル設計を確認する。Then: `debug-assertions`・`overflow-checks` が外れて検証条件が弱まるため、`release` は CI 検証に使われず Cargo 標準設定のまま維持される。

## Guarantees

- ローカルの `cargo build` / `cargo test` は常にフルデバッグ情報（変数・型・スタック）で動作し、CI の容量対策が `dev` に波及しない。
- CI の検証条件は `dev` 継承により開発時に近い assertion 群を保ちつつ、外部依存シンボル・増分中間成果物が抑制される。
- メンバークレートのマニフェストに profile の重複がなく、プロファイル定義の authoritative な置き場が workspace ルートに一意に定まる。

## Non-goals

- `release` プロファイルのチューニング（`lto` / `strip` / `codegen-units` / `panic`）と `profiling` プロファイルは扱わない（#1 以降の範囲）。
- ディスク使用量削減の具体的な数値目標は検証しない。`df -h` / `du -sh target` の計測・記録のみを要求し、設定変更だけで枯渇が解消したとは断定しない。
- メンバークレートの最終的な分割・公開 API・edition／MSRV の確定値は扱わない（仮置きのまま #1 で再評価）。
- CI ワークフロー側のトリガー・権限・計測ステップ構成は `manual-ci-workflow` 側の関心事。

## Risk

- `dev` のデバッグ情報を削る変更は、DSP・リアルタイムスレッドを扱う開発のデバッグ体験を直接損なう。
- CI に `dev` やフルデバッグ情報を持ち込むと、cheaper PR #109 と同様のディスク枯渇を再現し得る。
- メンバー側への profile 重複は Cargo の警告対象となり、プロファイル定義の authoritative な置き場が曖昧になる。
- `[profile.ci.package."*"]` の適用範囲を誤ると、workspace メンバーのバックトレースからファイル名・行番号が失われ、CI 失敗時の原因追跡が困難になる。
