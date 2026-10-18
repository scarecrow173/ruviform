---
id: "ADR-034IcJdtqm6TP6MylyP5J7"
type: "adr"
status: "accepted"
title: "開発基盤の toolchain・Cargoプロファイル・CI方針"
created: "2026-10-18"
updated: "2026-10-19"
owners: ["scarecrow173"]
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
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://doc.rust-lang.org/cargo/reference/profiles.html"
    - "https://doc.rust-lang.org/cargo/guide/build-performance.html"
    - "https://github.com/scarecrow173/cheaper/pull/109"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 開発基盤の toolchain・Cargoプロファイル・CI方針

## Context and Problem Statement

GitHub Issue #2 と上流 spec `SPEC-034IcwhwIFSNUNKtX7KK0Q`（[../specs/establish-rust-dev-foundation.md](../specs/establish-rust-dev-foundation.md)）に基づき、Ruviform の Rust 開発基盤を確立する。Ruviform は DSP・SIMD・リアルタイムスレッド・GPU 処理を扱うため、ローカル開発では変数・型・スタックを追えるフルデバッグ情報が必須である。一方、cheaper PR #109 では CI が `No space left on device` で失敗した事例が記録されており、フルデバッグ情報をそのまま CI に持ち込むと容量を圧迫する。ツールチェーン管理・Cargo プロファイル・CI 起動方針・開発コマンドの入口・`Cargo.lock` 追跡という相互に関係する決定を、1 つの ADR として記録する。

## Decision Drivers

* ローカル `dev` のフルデバッグ情報を削らない（DSP・RT スレッドのデバッグ要件）
* CI のディスク使用量とビルド時間を抑えつつ、開発時に近い検証条件を維持する
* 既存の `mise.toml` / `apm.yml` 正本化と `mise.lock` / `apm.lock.yaml` による再現性方針と整合させる
* #1（詳細設計）の完了を前提とせず、最終的なクレート分割・プラットフォーム戦略を拘束しない

## Considered Options

**CI 用 Cargo プロファイル:**

* `dev` プロファイルをそのまま CI で使用する
* `dev` を継承するカスタム `[profile.ci]` を workspace ルートに定義する（`debug = "line-tables-only"`、`incremental = false`、外部依存は `debug = "none"`）
* `release` プロファイルを CI の検証に使用する

**Rust ツールチェーンの pin 方法:**

* `mise.toml` で Rust ツールチェーン（`cargo` / `rustfmt` / `clippy`）を管理し、`mise.lock` で再現する
* `rust-toolchain.toml`（rustup 標準）で pin する
* GitHub Actions のセットアップアクションでのみバージョンを固定する

**CI ワークフローの起動方式:**

* `workflow_dispatch` のみの手動実行とする
* `push` / `pull_request` の自動トリガーを設定する
* `schedule` による定期実行を併用する

**開発コマンドの入口:**

* `mise.toml` のタスク（`fmt` / `fmt-check` / `check` / `lint` / `test` / `build`・CI 向けタスク）に一本化する
* just / Makefile など別のタスクランナーを追加する
* シェルスクリプトとドキュメント記述のみで運用する

**`Cargo.lock` の追跡方針:**

* `Cargo.lock` を Git で追跡する
* ライブラリ workspace 慣習に倣い `Cargo.lock` を追跡しない

## Decision Outcome

以下の組み合わせを採用する。

* **CI プロファイル**: `dev` を継承するカスタム `[profile.ci]` を workspace ルート `Cargo.toml` に定義する。`dev` 継承により `opt-level=0`・`debug-assertions`・`overflow-checks` の開発時に近い検証条件を保ち、`debug = "line-tables-only"` でワークスペースメンバーのバックトレースを確保しつつ、`[profile.ci.package."*"]` の `debug = "none"` で外部依存のデバッグシンボルを削り、`incremental = false` と CI 側の `CARGO_INCREMENTAL=0` で中間成果物を抑える。`dev` は `debug = "full"` を維持し、CI の容量対策を `dev` や `[profile.debugging]` には波及させない。`release` 流用は `debug-assertions` 等が外れて検証条件が弱まるため不採用。
* **ツールチェーン**: `mise.toml` で Rust ツールチェーンを管理する。本リポジトリは `mise.toml` を開発ツールの正本とする契約（[AGENTS.md](../../AGENTS.md)）がすでにあり、`rust-toolchain.toml` を併用すると toolchain バージョンの正本が二重化するため採用しない。
* **CI 起動**: 原則 `workflow_dispatch` のみ。初期段階では自動トリガーによる恒常的な Pending・未検証マージのリスクを避け、`contents: read` の最小権限で Linux 必須検証から始める。自動トリガー導入は別 Issue の明示的方針変更とする。
* **コマンド入口**: `mise.toml` タスクに一本化し、ローカルと CI で同一の検証コマンドを共有する。新たなタスクランナーは追加しない。
* **`Cargo.lock`**: 追跡する。本基盤は `mise.lock` / `apm.lock.yaml` と同じく再現可能なセットアップを目的とし、CI を固定バージョンから復元する方針と一致する。将来実行バイナリを含む workspace となる見込みでも lockfile 追跡が適切。

### Consequences

* 良い点: ローカルのデバッグ体験を損なわずに CI のディスク消費を抑えられる。ツール・タスク・lockfile の正本が `mise.toml` / `apm.yml` / `Cargo.lock` に集約され、新規 clone からの再現手順が一本化される。
* 悪い点: `target/ci/` と `target/debug/` が並存し得るため、ローカルで `--profile ci` を常用すると容量と初回ビルド時間が増える（CI では同一 job で `dev` / `ci` を二重ビルドしない）。`workflow_dispatch` は default branch にワークフローが存在することが前提で、push/PR では自動検証されないことを利用者が理解する必要がある。Rust バージョン・MSRV・初期クレート構成は仮置きであり、#1 で再評価される。

### Confirmation

下記 Verification のチェックリストを Issue #2 の受け入れ条件と併せて確認し、実装 PR で `cargo` / `mise` / GitHub Actions の実行証跡をレビューする。本決定の遵守は、`Cargo.toml`・`mise.toml`・ワークフロー定義の差分レビューと、手動ディスパッチした CI の実測ログ（`df -h` / `du -sh target`）で確認する。

## Implementation Plan

* 影響するパス:
  * `mise.toml` — Rust ツールチェーン（`cargo` / `rustfmt` / `clippy`）と `fmt` / `fmt-check` / `check` / `lint` / `test` / `build`・CI 向け検証タスク（例: `check-ci`）の追加
  * `Cargo.toml`（workspace ルート）— `[profile.dev]` の `debug = "full"` 維持、`[profile.ci]`（`inherits = "dev"`、`debug = "line-tables-only"`、`incremental = false`）、`[profile.ci.package."*"]` の `debug = "none"`、workspace package/lints/dependencies の共通ポリシー
  * 最小限のコンパイル可能な workspace メンバークレートと smoke test（最終的な分割は #1 で再評価）
  * `.github/workflows/` — `workflow_dispatch` のみの検証ワークフロー（`--profile ci`、`CARGO_INCREMENTAL=0`、`df -h` / `du -sh target` 計測、`contents: read`、まず Linux）
  * `.gitignore` — `target/` 等の追加、`Cargo.lock` 追跡方針の明文化
* 制約:
  * ローカル通常タスクは `dev` / `test` のフルデバッグを維持し、`dev` のデバッグ情報を削らない。別途 `[profile.debugging]` を作らない
  * プロファイル定義は workspace ルートのみに置き、各クレートのマニフェストで重複させない
  * CI の `cargo check` / `clippy` / `test`（必要なら `build`）は `--profile ci` を明示する。`cargo fmt` はプロファイル非依存のまま実行する
  * `push` / `pull_request` / `schedule` の自動トリガーを設定しない。`target/` 全体や中間成果物の無制限な upload/キャッシュを行わない
  * `release` プロファイルは当面 Cargo 標準設定を維持する（チューニングは #1 以降）
* 依存関係: `mise.lock` / `apm.lock.yaml` は追跡対象とし、CI は `mise install --locked` 相当で復元する。既存 `mise.toml` / `apm.yml` / prek / Git LFS 運用を破壊しない

## Pros and Cons

**CI プロファイル — `dev` をそのまま使用:**

* 良い点: 追加設定なし。ローカルと完全に同一条件。
* 悪い点: フルデバッグ情報で `target/` が膨大化し、cheaper PR #109 と同様のディスク枯渇リスクがある。

**CI プロファイル — カスタム `[profile.ci]`（採用）:**

* 良い点: `dev` 継承で検証条件を保ちつつデバッグ情報・増分コンパイル由来の容量を削減できる。プロファイル切替で `dev` 側を汚染しない。
* 悪い点: `target/ci/` と `target/debug/` の二重ディレクトリが生じ得る。Cargo カスタムプロファイルの理解が前提になる。

**CI プロファイル — `release` を使用:**

* 良い点: 成果物が小さく、最適化後の検証ができる。
* 悪い点: `debug-assertions`・`overflow-checks` が外れて検証条件が開発時と乖離し、ビルド時間も増える。

**ツールチェーン — `rust-toolchain.toml`:**

* 良い点: Rust エコシステム標準で rustup が自動解決する。
* 悪い点: `mise.toml` とツールバージョンの正本が二重化し、どちらが authoritative か曖昧になる。

**CI 起動 — 自動トリガー（push / pull_request）:**

* 良い点: PR が自動検証される。
* 悪い点: 基盤未整備の段階で恒常的な失敗・Pending を生み、未検証マージの圧力になる。初期 Issue の方針外。

**`Cargo.lock` — 非追跡:**

* 良い点: 純粋なライブラリ公開のみなら差分ノイズを減らせる。
* 悪い点: CI・新規 clone で依存解決が非再現になり、`mise.lock` / `apm.lock.yaml` の再現性方針と矛盾する。

## Verification

* [ ] `mise install --locked` で Rust ツールチェーン（`cargo` / `rustfmt` / `clippy`）が復元できる
* [ ] `cargo build` / `cargo test` が `dev` のフルデバッグ情報で動作し、`cargo check --workspace --all-targets --profile ci`、`cargo clippy --workspace --all-targets --profile ci -- -D warnings`、`cargo test --workspace --profile ci` が成功する
* [ ] workspace ルート `Cargo.toml` に `[profile.dev]` / `[profile.ci]` / `[profile.ci.package."*"]` が定義され、メンバーのマニフェストに重複がない
* [ ] `mise run` の検証タスクがローカルで実行でき、CI と同一コマンドになっている
* [ ] GitHub Actions が `workflow_dispatch` のみで起動し、`push` / `pull_request` で自動実行されないことが設定上確認できる
* [ ] CI が `--profile ci` と `CARGO_INCREMENTAL=0` を明示し、実行前後の `df -h` / `du -sh target` を記録する
* [ ] `Cargo.lock` が追跡され、`.gitignore` に `target/` が含まれる
* [ ] 最小 workspace クレートの smoke test がプロジェクト実装未着手でも成功する

## More Information

* 一次ソース: GitHub Issue [#2 [chore] mise/APM初期セットアップ・lockfile生成、Rust開発基盤と手動CIを整備する](https://github.com/scarecrow173/ruviform/issues/2)
* 上流ドキュメント: [SPEC-034IcwhwIFSNUNKtX7KK0Q](../specs/establish-rust-dev-foundation.md)（`relations.derives-from`）
* 関連ドキュメント: [AGENTS.md](../../AGENTS.md)（mise / APM の運用契約）
* 参考資料: [Cargo Book - Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)、[Cargo Book - Build Performance](https://doc.rust-lang.org/cargo/guide/build-performance.html)、[cheaper PR #109](https://github.com/scarecrow173/cheaper/pull/109)（CI ディスク枯渇の参考事例。設定をそのまま移植しない）
