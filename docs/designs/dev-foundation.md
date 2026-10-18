---
id: "DESIGN-034IddXFCRKDIW01WXkSla"
type: "design"
status: "approved"
title: "Rust 開発基盤の設計（最小ワークスペース・CIプロファイル・手動CI）"
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
    - "../specs/establish-rust-dev-foundation.md"
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
    - "../adr/rust-dev-foundation-toolchain-and-ci-profile.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://doc.rust-lang.org/cargo/reference/profiles.html"
    - "https://doc.rust-lang.org/cargo/guide/build-performance.html"
    - "https://doc.rust-lang.org/cargo/reference/config.html#buildincremental"
    - "https://github.com/scarecrow173/cheaper/pull/109"
  defers: []
  deferred-by: []
---

# Rust 開発基盤の設計（最小ワークスペース・CIプロファイル・手動CI）

## Context

GitHub Issue #2（`relations.source`）と承認済み spec `SPEC-034IcwhwIFSNUNKtX7KK0Q`、accepted ADR `ADR-034IcJdtqm6TP6MylyP5J7` に基づき、Ruviform の最小 Rust 開発基盤の具体的な設計を定義する。実施順序は **#2 → #3 → #1** であり、本設計は **#1（詳細設計）の完了を前提としない**。最終的なクレート分割・公開 API・対象プラットフォーム戦略は #1 に委ね、本設計では「後で変更・分割できる最小契約」のみを確定させる。

背景となる制約:

- Ruviform は DSP・SIMD・リアルタイムスレッド・GPU 処理を扱うため、ローカル `dev` ではフルデバッグ情報（変数・型・スタック）を維持する必要がある。
- cheaper PR #109 で CI の `No space left on device` 失敗が記録されているため、CI 専用のカスタムプロファイル `ci` で容量対策を `dev` と分離する。
- 既存契約（`AGENTS.md`）は `mise.toml` / `apm.yml` を正本とし、`mise.lock` / `apm.lock.yaml` を追跡する運用を定めている。ツールバージョンの正本を二重化しないことが前提となる。

## Scope

### In Scope

- ワークスペースルート `Cargo.toml` の新設（`[workspace]`、workspace package/lints 共通ポリシー、`[profile.dev]` / `[profile.ci]` / `[profile.ci.package."*"]`）
- 最小限のコンパイル可能なメンバークレート `crates/ruviform-core`（仮置き名）と smoke test
- `mise.toml` への Rust ツールチェーン（`cargo` / `rustfmt` / `clippy`）追加と、`fmt` / `fmt-check` / `check` / `lint` / `test` / `build`・CI 向け `*-ci` タスクの定義
- `mise.lock` / `apm.lock.yaml` の生成・検証・追跡（既存ファイルの破壊的再初期化を行わない手順）
- `workflow_dispatch` のみで起動する GitHub Actions 検証ワークフロー `.github/workflows/ci.yml`（最小権限 `contents: read`、Linux 必須検証、ディスク計測）
- `.gitignore` への `target/` 追加、`Cargo.lock` 追跡の明文化、暫定開発規約（`unsafe` / feature / 依存追加 / lint / ドキュメントテスト）
- LFS フィクスチャ方針・未検証 OS の明記を含むドキュメント運用メモ（#3 転記用の記録）

### Out of Scope

- DSP・video pipeline・SIMD・WebView・VST の完成実装
- `ruviform-audio` 等の最終的なクレート分割・公開 API の決定（#1 の設計対象）
- `release` プロファイルのチューニング（`lto` / `strip` / `codegen-units` / `panic`）と `profiling` プロファイル（#1 以降）
- macOS／Windows の CI matrix 拡張（#1 の対象プラットフォーム設計後に判断）
- `wgpu`・WebView・音声デバイス・codec のシステム依存導入
- README／AGENTS 本文改稿と `apm.yml` の `project-template` リネーム（#3 の責務）
- `cargo-nextest` / `cargo-llvm-cov` / ベンチマーク補助ツールの必須化
- `push` / `pull_request` / `schedule` 自動トリガー（別 Issue の明示的方針変更）

## Components and Boundaries

### ディレクトリ構成

```text
ruviform/
├── Cargo.toml                    # workspace ルート（profiles・共通ポリシーの唯一の置き場）
├── Cargo.lock                    # 追跡対象（再現性のため commit）
├── crates/
│   └── ruviform-core/            # 最小メンバークレート（仮置き・#1 で分割再評価）
│       ├── Cargo.toml            # workspace.package / workspace.lints を継承。profile は書かない
│       └── src/lib.rs            # 最小 API + #[cfg(test)] smoke test（デバイス非依存）
├── mise.toml                     # ツール・タスクの正本（Rust toolchain + 検証タスクを追加）
├── mise.lock                     # 追跡対象（mise install --locked で復元）
├── apm.yml / apm.lock.yaml       # 既存の正本（再初期化しない）
├── .github/workflows/ci.yml      # workflow_dispatch 専用の検証ワークフロー
├── .gitignore                    # target/ 等を追加
└── .gitattributes                # 既存 LFS ルールを維持（フィクスチャ方針を記録）
```

### コンポーネントと責務

- **workspace ルート `Cargo.toml`**: ビルドプロファイル・edition・resolver・共通 lint ポリシーの authoritative な置き場。`[profile.*]` はここだけに定義し、各クレートのマニフェストへ重複させない（Cargo がメンバー側 profile を警告対象とするため）。
- **`crates/ruviform-core`**: workspace が空にならないための最小契約。公開 API・分割方針を先走らず、`lib.rs` に小さな純粋関数と `cargo test` で成功する smoke test のみ置く。命名・構造は仮置きであり #1 で自由に再構成できる。
- **`mise.toml`**: Rust ツールチェーン pin と開発コマンドの唯一の入口。`rust-toolchain.toml` は併用しない（ツールバージョンの正本が二重化するため ADR で不採用）。
- **`.github/workflows/ci.yml`**: 手動ディスパッチ専用の検証器。ローカル検証との差分を生じさせないため、cargo コマンドは生で書かず mise タスク経由で呼ぶ。
- **lockfile 群（`mise.lock` / `apm.lock.yaml` / `Cargo.lock`）**: 再現可能セットアップの境界。すべて Git 追跡。

### workspace ルート `Cargo.toml` の設計

```toml
[workspace]
resolver = "2"
members = ["crates/ruviform-core"]

[workspace.package]
edition = "2021"          # 仮置き。#1 で edition / MSRV を再評価
rust-version = "1.85"     # 仮置き MSRV。mise pin バージョン以上にはしない
license = "MIT"           # 仮置き。配布方針確定前の暫定値

[workspace.lints.rust]
unsafe_code = "warn"      # lint ゲートの -D warnings で実質 deny。例外は局所許可＋SAFETY コメント（後述）
unused_qualifications = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
missing_errors_doc = "allow"  # pedantic の過剰適用を初期段階で緩和（仮置き）

# Local development: retain full debug information.
[profile.dev]
debug = "full"

# CI: inherit development checks but generate smaller build artifacts.
[profile.ci]
inherits = "dev"
debug = "line-tables-only"
incremental = false

# CI: dependencies outside the workspace do not need debug symbols.
[profile.ci.package."*"]
debug = "none"
```

プロファイルブロックは spec / ADR の定義を verbatim で採用する。`ci` は `dev` 継承により `opt-level=0`・`debug-assertions`・`overflow-checks` の開発時に近い検証条件を保ち、メンバーは `line-tables-only` でバックトレースのファイル名・行番号を確保し、ワークスペース外依存のみ `debug = "none"` で削る。`[profile.debugging]` は作らない。`release` は Cargo 標準を維持する。

各メンバークレートの `Cargo.toml` は `[lints] workspace = true`・`edition.workspace = true`・`rust-version.workspace = true` で共通ポリシーを継承し、固有値のみを書く。`edition` / `rust-version` は `[workspace.package]` に定義しただけでは各クレートへ適用されず、メンバー側の `.workspace = true` 継承が必須である点に注意する。

### Rust ツールチェーンの pin 方針

- `mise.toml` の `[tools]` に `rust = "<具体的な stable バージョン>"`（例: `rust = "1.90"`）を追加する。`stable` / `latest` のような浮動指定は避け、`mise.lock` で解決済みバージョンを確定させる。
- `cargo` / `rustfmt` / `clippy` は rustup の default プロファイルで同梱される。CI・ローカル双方で `cargo fmt --version` / `cargo clippy --version` の疎通を確認し、欠ける場合は `setup` タスクに component 追加を明示する。
- `rust-version`（MSRV 仮置き）は mise pin バージョン以下に揃える。実バージョン・MSRV ともに #2 では根拠を残した仮置きとし、#1 で再評価する（根拠は実施 PR に記録）。
- lockfile 運用: `[settings] lockfile = true` を維持し、`mise install` → `mise lock` → `mise run setup` → `apm lock` の順で明示生成。復元は `mise install --locked`、APM は `apm install --frozen`（CLI が未対応なら同等の再現性検証方法を実施 PR に記録）。

### mise タスク設計

| タスク | コマンド | 用途 |
| --- | --- | --- |
| `fmt` | `cargo fmt --all` | ローカル整形（書き込みあり） |
| `fmt-check` | `cargo fmt --all -- --check` | 非破壊チェック（CI 共通） |
| `check` | `cargo check --workspace --all-targets` | ローカル dev 検証 |
| `lint` | `cargo clippy --workspace --all-targets -- -D warnings` | ローカル dev 検証 |
| `test` | `cargo test --workspace` | ローカル dev 検証 |
| `build` | `cargo build --workspace` | ローカル dev ビルド |
| `check-ci` | `cargo check --workspace --all-targets --profile ci` | CI 検証（`--profile ci`） |
| `lint-ci` | `cargo clippy --workspace --all-targets --profile ci -- -D warnings` | CI 検証 |
| `test-ci` | `cargo test --workspace --profile ci` | CI 検証 |
| `build-ci` | `cargo build --workspace --profile ci` | CI で必要な場合のみ |
| `verify-ci` | `fmt-check` → `check-ci` → `lint-ci` → `test-ci` を順次実行する集約タスク | CI・ローカル共通の検証入口 |

設計上の規則:

- `*-ci` タスクには `env = { CARGO_INCREMENTAL = "0" }` を付与し、ローカル実行でも CI と同一条件にする。`verify-ci` にも同じ env を伝播させる。
- ローカル通常タスクは `dev` / `test` プロファイル（フルデバッグ）を維持し、CI 向けだけが `--profile ci` を使う。検証内容（対象 workspace・`--all-targets`・`-D warnings`）は `*-ci` 系で重複定義せず同一コマンドのプロファイル差分として扱う。
- `cargo fmt` はプロファイル非依存のため `*-ci` 系を設けない。
- `prek` の軽量フック（YAML/TOML/改行/秘密鍵）と重い cargo 検証は分担し、コミットフックには cargo 検証を追加しない。
- `mise run setup` は既存の `apm install` / `prek install` / `git lfs install --local` を維持し、Rust 固有の追加準備が必要になった場合のみ追記する。

### GitHub Actions ワークフロー設計（`.github/workflows/ci.yml`）

- **トリガー**: `on: workflow_dispatch` のみ。`push` / `pull_request` / `schedule` は設定しない。入力項目は設けず、入力なしの全チェック実行を基本とする（必要になれば実行対象選択の optional input を後から追加）。
- **権限**: `permissions: contents: read` のみ。
- **ランナー**: `ubuntu-latest` 単一 job。matrix は張らない（Linux 必須検証が先）。
- **ジョブ env**: `CARGO_INCREMENTAL: "0"` を job レベルで明示（`--profile ci` の `incremental = false` と二重に保証）。
- **ステップ構成**:
  1. `actions/checkout`
  2. ディスク事前計測: `df -h`
  3. mise 導入と固定復元: `jdx/mise-action`（または同等の mise セットアップ）＋ `mise install --locked` で `mise.lock` から復元
  4. 構成ファイル検証: `mise exec -- prek run --all-files`（既存 prek フックによる基本検証）
  5. `mise run fmt-check`
  6. `mise run check-ci`
  7. `mise run lint-ci`
  8. `mise run test-ci`（必要時のみ `mise run build-ci`）
  9. ディスク事後計測: `df -h`、`du -sh target`、`du -sh target/ci/deps target/ci/build`（存在する範囲で）
- **キャッシュ方針**: `target/` 全体や中間成果物を無条件に upload／キャッシュしない。許容するキャッシュは `~/.cargo` の registry/git index を `Cargo.lock` ハッシュでキー付けしたものに限定する。同一 job で `dev` と `ci` を二重ビルドしない。
- **導入・検証手順**: `workflow_dispatch` は default branch にワークフローが存在することが前提。初回はワークフローを default branch へマージした後に Actions UI から手動ディスパッチし、ブランチ上では `actionlint` 等の静的検証と yaml 構文チェック（prek）に留める、を手順として明記する。
- **ログ方針**: コード未整備の初期段階でも失敗原因が追えるよう、各ステップ名と実行コマンドを明示する。手動 CI を実行しなくても PR が自動検証済みにならない点を運用メモに残す。

### 暫定開発規約（`unsafe` / feature / 依存 / lint / doc test）

- **`unsafe`**: 原則禁止とし、必要な場合のみ SAFETY コメントを伴う明示的な局所許可（`#[expect(unsafe_code)]`、未対応なら `#[allow(unsafe_code)]`）で例外的に認める。`workspace.lints.rust.unsafe_code = "warn"` は `lint` / `lint-ci` の `-D warnings` ゲートで実質 `deny` として機能し、局所許可のない `unsafe` は検証を通過しない。`deny`/`forbid` への恒久的引き上げと RT-safe 契約は #1 で策定する。
- **feature**: default feature は最小限。新規 feature・optional 依存は追加理由を PR に記録する。
- **依存追加**: メンテナンス状況・ライセンス・依存木サイズを確認し理由を記録。`Cargo.lock` を追跡して依存解決を再現可能にする。システム依存（wgpu・codec 等）は採用決定後に段階導入。
- **lint**: `workspace.lints` を正本とし、CI は `clippy -- -D warnings` を必須ゲートとする。rustfmt は標準設定を使い、`rustfmt.toml` は必要になった時点で追加する。
- **ドキュメントテスト**: 公開 API の例は doctest として書き、`cargo test` で検証対象に含める。
- **LFS / アセット**: `.gitattributes` の audio/video 一律 LFS は現行維持。小さいテストフィクスチャは非メディア形式（JSON/YAML 等）で Git に置き、CI ではフィクスチャが必要になるまで `git lfs pull` を省略する。実施時に確認結果を記録する。

## Data and Control Flow

### 初回セットアップ（新規 clone → 検証可能状態）

1. `mise install` → `mise lock`（`mise.lock` 生成・確認）
2. `mise run setup`（`apm install` → `apm.lock.yaml` 生成、`prek install`、`git lfs install --local`）
3. `apm lock` で APM lockfile を再確認
4. `mise.lock` / `apm.lock.yaml` / `Cargo.lock` を commit 対象にする
5. 復元検証: `mise install --locked` と `apm install --frozen`（または同等手段）

### ローカル開発フロー

```text
mise run fmt        # 整形（dev: フルデバッグ情報を維持）
mise run check      # cargo check --workspace --all-targets
mise run lint       # clippy -D warnings
mise run test       # cargo test --workspace（smoke test 含む）
mise run build      # cargo build --workspace
```

CI と同一条件での事前確認が必要な場合のみ `mise run verify-ci` を使う（`target/ci/` が `target/debug/` と並存する点に注意）。

### 手動 CI フロー

```text
workflow_dispatch (default branch)
  → checkout → df -h（事前計測）
  → mise install --locked（mise.lock から toolchain 復元）
  → prek run --all-files（構成ファイル検証）
  → fmt-check → check-ci → lint-ci → test-ci（すべて --profile ci・CARGO_INCREMENTAL=0）
  → df -h / du -sh target / target/ci/deps・build（事後計測をログへ記録）
```

## Non-Functional Constraints

- **再現性**: ツール・依存の正本は `mise.toml` / `apm.yml` / `Cargo.toml`、確定値は `mise.lock` / `apm.lock.yaml` / `Cargo.lock` に集約しすべて追跡する。
- **権限最小化**: ワークフローは `contents: read` のみ。実行 ref・依存バージョン・結果がログから追跡できること。
- **ディスク使用量**: `ci` プロファイルと `CARGO_INCREMENTAL=0` で中間成果物を抑制し、計測値を毎回ログに残す。設定変更だけで枯渇が解消したと断定せず、容量不足時はキャッシュ・複数 test binary の影響も調査する。
- **デバッグ可能性**: ローカル `dev` のフルデバッグ情報を削らない。CI の容量対策を `dev` に波及させない。
- **ヘッドレス安全性**: 基礎クレートのテストはデバイス非依存とし、GPU・音声デバイス・WebView 前提のテストを CI に置かない。

## Risks and Trade-offs

- `target/ci/` と `target/debug/` の並存によりローカル容量・初回ビルド時間が増える → `*-ci` タスクは CI 用途に限定し、常用しない旨を運用メモに明記。
- `workflow_dispatch` は default branch にワークフローが必要 → 初回導入は default マージ後の手動ディスパッチで検証し、ブランチ上は静的検証に留める手順を採用。
- 仮置きクレート名・edition・MSRV が #1 設計を拘束するリスク → すべて「仮置き」と明記し、根拠を PR に残して再評価可能にする。
- 手動 CI のみでは未検証マージが可能 → branch protection の必須チェックを設定する場合は恒常 Pending にならないよう別 Issue で方針を決める。
- `apm install --frozen` 未対応の可能性 → 実機確認し、未対応なら同等の再現性検証を記録する。

## Deferred Design Concerns

| 据え置く項目 | 理由 | 再着手トリガ | 無視した場合のリスク |
| --- | --- | --- | --- |
| 最終クレート分割・公開 API（`ruviform-audio` 等） | #1 の詳細設計対象 | #1 着手 | 仮置き構造の固定化 |
| Rust バージョン・MSRV・edition の確定 | #2 は根拠付き仮置き | #1 で再評価 | 根拠喪失 |
| `release` チューニング・`profiling` プロファイル | 性能測定需要が未発生 | #1 以降 | 未検討トレードオフの固定 |
| macOS／Windows CI matrix | 対象プラットフォーム未確定 | #1 のプラットフォーム設計後 | 未検証 OS 差異の未検出 |
| システム依存（wgpu・WebView・codec・音声デバイス） | 採用未決定 | 採用決定・検証対象確定後 | デバイス依存テストで CI 不安定化 |
| `cargo-nextest` / `cargo-llvm-cov` 等の必須化 | 導入コスト・移植性を要評価 | 必要時のみ | 不要依存の保守コスト |
| 自動トリガー・必須チェック設定 | 方針上の明示的変更が必要 | 別 Issue の方針変更 | 恒常 Pending・未検証マージ |

## Verification Notes

- [ ] `mise install --locked` で `cargo` / `rustfmt` / `clippy` が復元できる
- [ ] `cargo build` / `cargo test` が `dev` のフルデバッグ情報で動作し、`check-ci` / `lint-ci` / `test-ci`（`--profile ci`）が成功する
- [ ] workspace ルート `Cargo.toml` に spec 記載の `[profile.dev]` / `[profile.ci]` / `[profile.ci.package."*"]` が verbatim で定義され、メンバー側に重複がない
- [ ] `mise run` の各タスクがローカルで実行でき、CI と同一コマンドである
- [ ] GitHub Actions が `workflow_dispatch` のみで起動し、`push` / `pull_request` で自動実行されない
- [ ] CI が `--profile ci`・`CARGO_INCREMENTAL=0` を明示し、実行前後の `df -h` / `du -sh target` を記録する
- [ ] `Cargo.lock` が追跡され `.gitignore` に `target/` が含まれ、最小クレートの smoke test が成功する
- [ ] APM／Serena 生成ファイル・秘密情報・ローカル設定が Git に追跡されないことが確認されている
- [ ] 実施した OS での手順・検証結果・前提条件が記録され、未検証 OS が明記される（#3 転記可能な状態）
