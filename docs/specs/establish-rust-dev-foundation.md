---
id: "SPEC-034IcwhwIFSNUNKtX7KK0Q"
type: "spec"
status: "approved"
title: "Rust 開発基盤の確立（mise/APM lockfile・最小ワークスペース・手動CI）"
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
    - "https://github.com/scarecrow173/ruviform/issues/2"
  derived-by:
    - "../adr/rust-dev-foundation-toolchain-and-ci-profile.md"
    - "../designs/dev-foundation.md"
    - "../plans/dev-foundation-bootstrap.md"
    - "../test-specs/lockfile-reproducible-setup.md"
    - "../test-specs/mise-verification-tasks.md"
    - "../test-specs/ci-profile-build-behavior.md"
    - "../test-specs/manual-ci-workflow.md"
  verifies: []
  verified-by:
    - "../test-specs/lockfile-reproducible-setup.md"
    - "../test-specs/mise-verification-tasks.md"
    - "../test-specs/ci-profile-build-behavior.md"
    - "../test-specs/manual-ci-workflow.md"
  references:
    - "https://doc.rust-lang.org/cargo/reference/profiles.html"
    - "https://doc.rust-lang.org/cargo/guide/build-performance.html"
    - "https://doc.rust-lang.org/cargo/reference/config.html#buildincremental"
    - "https://github.com/scarecrow173/cheaper/pull/109"
  defers: []
  deferred-by: []
---

# Rust 開発基盤の確立（mise/APM lockfile・最小ワークスペース・手動CI）

## What Should Be Built

GitHub Issue #2 で定義された chore 作業の成果物として、以下を備えた再現可能な Rust 開発基盤を確立する。

- 既存の `mise.toml` / `apm.yml` を正本とする初期セットアップの完了と、`mise.lock` / `apm.lock.yaml` の生成・検証・追跡（lockfile 確定は一部実施済み）
- `mise.toml` で管理される Rust ツールチェーン（`cargo` / `rustfmt` / `clippy`）と、ワークスペースルートの `Cargo.toml`、コンパイル可能な最小クレートと smoke test
- ローカル開発のフルデバッグ情報を維持する `dev` と、成果物を小さくする CI 専用カスタムプロファイル `ci` の workspace ルート定義
- `fmt` / `fmt-check` / `check` / `lint` / `test` / `build`（および CI 向けエントリ）を一本化する mise タスク
- 原則 `workflow_dispatch` のみで起動する手動 GitHub Actions 検証ワークフロー
- ドキュメント・アセット運用方針（LFS、秘密情報・ローカル生成物の非追跡、検証済み OS の明記）の記録

## Why Now

- 実施順序は **#2 → #3 → #1**。この spec は Issue #2 を対象とし、**#1（詳細設計）の完了を前提としない**。最小限の動作する Rust 開発基盤を先に確立し、最終的なクレート分割・プラットフォーム戦略等は後続の #1 で扱う。
- 現状リポジトリには `mise.toml`（`uv` / `prek` / `git-lfs` / `apm-cli` / `serena-agent` と `mise run setup` タスク）、`apm.yml`、`.pre-commit-config.yaml`、Git LFS 用 `.gitattributes` が存在するが、Rust ツールチェーン・`Cargo.toml`・CI ワークフローは未整備。
- 参考事例として cheaper では CI の `No space left on device` 失敗が記録されている（cheaper PR #109）。Ruviform は DSP・SIMD・リアルタイムスレッド・GPU 処理を扱うため、ローカル `dev` のフルデバッグ情報を削らずに CI の容量対策を分離する設計が必要。

## Users and Value

- **Ruviform の開発者**: 新規 clone から固定 lockfile に基づく再現可能なセットアップと、ローカル／CI で同一の検証コマンドを利用できる。
- **Issue #3 の作業者**: README の Quickstart と AGENTS の検証ルールへ転記可能な、実測済みのセットアップ手順・前提条件を得られる。
- **Issue #1 の設計者**: Rust バージョン・MSRV・初期クレート等の仮置き決定とその根拠の記録を参照して再評価できる。

## Scope

### In Scope

- mise / APM の初回セットアップと `mise.lock` / `apm.lock.yaml` の生成・検証・追跡（既存 `mise.toml` / `apm.yml` の破壊的再初期化を行わない）
- `mise.toml` への Rust ツールチェーン追加と再現可能なバージョン管理
- ワークスペースルート `Cargo.toml`（edition、resolver、workspace package/lints/dependencies の共通ポリシー検討）と、後で変更・分割できる最小限のコンパイル可能なクレートと smoke test
- `[profile.dev]` のフルデバッグ維持と `[profile.ci]`（`inherits = "dev"`、`debug = "line-tables-only"`、`incremental = false`、`[profile.ci.package."*"]` の `debug = "none"`）の workspace ルート定義
- `fmt` / `fmt-check` / `check` / `lint` / `test` / `build` の mise タスク化と、CI 向け検証タスク（`--profile ci` 使用）の共通化
- 原則 `workflow_dispatch` のみで起動する GitHub Actions 検証ワークフロー（`--profile ci` と `CARGO_INCREMENTAL=0` を明示し、`df -h` / `du -sh target` の計測記録、最小権限 `contents: read`、まず Linux で必須検証）
- `.gitignore` への `target/` 等の追加、`Cargo.lock` 追跡方針の明文化、暫定的な開発規約（`unsafe`・feature・依存追加・lint・ドキュメントテスト）の策定
- `.gitattributes` の audio/video 一律 LFS 登録が小さいテストフィクスチャに適切かの確認と、LFS pull 方針の記録
- APM／Serena 生成ファイル・秘密情報・ローカル設定が Git に混入しないことの確認
- 実施済み手順・検証結果・前提条件・未検証 OS／ツールの記録（#3 で転記可能な状態にする）

### Out of Scope

- DSP、video pipeline、SIMD、WebView、VST の完成実装
- 対応 OS・外部 codec・UI 技術など、#1 で決めるべき採用事項の既成事実化
- `ruviform-audio` 等の最終的なクレート分割・公開 API の決定（#1 の設計対象）
- `release` プロファイルのチューニング（`lto` / `strip` / `codegen-units` / `panic`）と性能測定用プロファイル設計（#1 以降）
- macOS／Windows の CI 対応保証・matrix 拡張（#1 の対象プラットフォーム設計後に判断）
- `wgpu`・WebView・音声デバイス・codec のシステム依存導入（採用決定・検証対象の確定後に段階導入）
- README／AGENTS の本文改稿と `apm.yml` の `project-template` リネーム（#3 の責務）
- `cargo-nextest` / `cargo-llvm-cov` / ベンチマーク補助ツールの必須化（導入コスト・移植性を評価のうえ必要時のみ）
- GitHub Actions の `push` / `pull_request` / `schedule` 自動トリガー（例外は別 Issue の明示的方針変更として扱う）

## Requirements

### 1. mise / APM 初回セットアップと lockfile 確定（最優先）

- mise の導入はリポジトリ外部の前提とし、`mise.toml` の内容とバージョン・実行環境を確認する
- 既存 `mise.toml` / `apm.yml` を初期状態の正本として使用し、再初期化による上書き（特に `apm init --yes`）を行わない
- 既存ファイルは再初期化せず、`apm init`・`cargo init` 等の初期化・雛形生成コマンドは対象が未作成の場合に限り実行する
- Rust 等この Issue で必要なツールを `mise.toml` に定義し、`mise install` → `mise lock` → `mise run setup` → `apm lock` の順で lockfile を明示生成・確認する。`[settings] lockfile = true` を維持する
- 生成された `mise.lock` / `apm.lock.yaml` をコミット対象とし、`mise install --locked` と使用中 APM CLI が対応する lock 検証／frozen install（例: `apm install --frozen`。未対応なら同等の再現性検証方法を記録）の成否を確認する
- 初回実行で必要なネットワーク・権限・認証、追跡対象／除外対象、lockfile 再生成条件を記録する

### 2. Rust toolchain と workspace

- Rust ツールチェーン（`cargo` / `rustfmt` / `clippy` 含む）を `mise.toml` で再現可能に管理し、バージョン／stable と MSRV の関係を記録する
- ワークスペースルートに `Cargo.toml` を追加し、初期段階で workspace が空にならない最小限のコンパイル可能なクレート／smoke test を用意する
- `.gitignore` に `target/` と Rust 関連一時生成物を追加し、`Cargo.lock` の追跡可否を workspace の性格に合わせ明文化する
- `unsafe`・feature・依存追加・lint・ドキュメントテストに関する暫定的な開発規約を定める

### 3. CI 専用プロファイルとディスク使用量対策

- workspace ルート `Cargo.toml` に初期設定として以下を定義し、各クレートのマニフェストで重複させない

```toml
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

- ローカルの `cargo build` / `cargo test` は `dev` のフルデバッグ情報を維持して使用し、別途 `[profile.debugging]` は作らない
- `ci` は `dev` 継承により `opt-level=0`・`debug-assertions=true`・`overflow-checks=true` の開発時に近い検証条件を維持し、`release` や `dev` のデバッグ削減設定を CI に流用しない
- `[profile.ci.package."*"]` はワークスペース外依存のみを対象とし、メンバーは `line-tables-only` でバックトレースのファイル名・行番号を確保する
- CI の `cargo check` / `clippy` / `test`（必要なら `build`）は `--profile ci` を明示し、`cargo fmt` はプロファイル非依存で従来どおり実行する
- `incremental = false` に加えて CI では `CARGO_INCREMENTAL=0` を明示する
- ローカル dev の増分コンパイルは Cargo 標準動作を維持し、`incremental = false` と `CARGO_INCREMENTAL=0` の無効化は `--profile ci` を用いる CI 検証に限定する
- `target/ci/` と `target/debug/` が並存すると容量・初回ビルド時間が増えることを認識し、CI では同一 job で `dev` と `ci` を二重ビルドしない
- `target/` 全体や中間成果物を無条件に upload／キャッシュせず、必要なアーティファクトのみ選択保存する
- 手動 CI でビルド前後の `df -h` と `du -sh target`（対応 OS は同等コマンド）を記録し、特に `target/ci/deps`・`build` を計測する
  - 容量不足時にはキャッシュや複数 test binary の影響も調査する
  - 設定変更だけでディスク枯渇が解消したと断定しない
- `release` は当面 Cargo 標準設定を維持する

### 4. 開発コマンドの一本化

- `mise run setup` を維持し、Rust に必要な追加準備のみ明示する（既存 APM／prek／LFS を壊さない）
- mise タスクとして `fmt`・`fmt-check`・`check`・`lint`（Clippy）・`test`・`build` を定義し、実行コマンドと副作用を明確にする。ローカル通常タスクは `dev`／`test`、CI 向け検証タスク（例: `check-ci`）は `--profile ci` を使用し、共通化できる検証内容は重複管理しない
- ローカルの開発タスクでも `mise run` 経由で `mise.toml`／`mise.lock` に固定されたツールを利用し、ローカルと CI でツールバージョンの差異を生じさせない
- 以下相当の検証をローカル／CI で実行可能にする

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

- `prek` の軽量フックと時間のかかる cargo 検証の分担を決め、コミット毎の重いビルド強制を避ける

### 5. GitHub Actions：原則手動実行のみ

- 検証ワークフローは原則 `workflow_dispatch` のみとし、`push` / `pull_request` / `schedule` 等の自動トリガーを設定しない
- 手動 CI で rustfmt、`cargo check --workspace --all-targets --profile ci`、`cargo clippy --workspace --all-targets --profile ci -- -D warnings`、`cargo test --workspace --profile ci`（必要なら `cargo build --workspace --profile ci`）と、既存 `prek` による構成ファイルの基本検証を実行できるようにする。CI で使うローカル検証入口を mise タスクに設ける
- `workflow_dispatch` の入力項目は最小限とし、入力なしで全チェックを実行できる構成を優先する（必要なら実行対象・OS を選択可能にする）
- ワークフローは必要最小限の権限（原則 `contents: read`）を使い、実行対象 ref・依存バージョン・結果が追跡できるようにする
- 手動実行可能なワークフローは default branch に存在する必要があるため、最初の導入・分岐ブランチ検証手順を明示する
- CI は `mise.lock` 等の固定バージョンから復元し、依存キャッシュと lockfile 使用方針を明確化する。ローカルの mise タスクと検証コマンドを共通化する
- まず Linux で必須検証を行う
- コードを伴わない初期段階でも失敗時の原因が把握できる CI ログ・実行手順にし、手動 CI を実行しなくても PR が自動検証済みにならないことを明示する
- ヘッドレスで動く基礎クレートのテストはデバイス依存にしない

### 6. ドキュメントとアセット運用

- 実際に成功した初回セットアップ／検証コマンドと前提条件を PR または作業メモに残し、#3 で転記可能な状態にする
- `.gitattributes` の audio/video 一律 LFS 登録が小さいテストフィクスチャにも適切か確認し、サイズ・配布・CI での LFS pull 方針を記録する
- APM／Serena 生成ファイルが Git に混入せず、秘密情報とローカル設定が追跡されないことを確認する
- 実際に対象とした OS で改行・パス・ツール導入差異をチェックし、未検証の OS を明記する

## Acceptance Criteria

Issue #2 の受け入れ条件に基づく。

- [ ] 既存 `mise.toml`／`apm.yml` を破壊的に再初期化せず、mise・APM 依存のインストールと `mise.lock`／`apm.lock.yaml` の生成・追跡が完了している
- [ ] 固定 lockfile を使用した再現インストール手順（`mise install --locked` と APM の lock 検証／frozen install または同等の再現性検証）が検証されている
- [ ] workspace ルート `Cargo.toml` で `dev` のフルデバッグ情報を保持し、`ci` は `dev` を継承しつつデバッグ情報を削減（メンバーは `line-tables-only`、外部依存は `none`）・増分コンパイルを無効化する構成になっている。通常の `cargo build`／`cargo test` と `--profile ci` を使う CI 検証をそれぞれ動作確認できる
- [ ] CI は `--profile ci` と `CARGO_INCREMENTAL=0` を明示し、通常の手動 workflow で `target/ci/` のサイズと実行前後のディスク空き容量を記録する。`dev`／`ci` の不必要な重複ビルドや、全中間成果物の無制限アップロードを行わない
- [ ] 新規 clone で、記載した前提条件からセットアップと Rust の build／check／lint／test が再現できる
- [ ] `mise` タスクでローカル検証を実行でき、手動ディスパッチした CI と同じ検証ができる
- [ ] ローカル生成物・秘密情報が意図せず追跡されず、既存 APM／prek／LFS 運用が維持される
- [ ] 手順と実行済みの検証結果を PR に残す。未検証 OS／ツールは未検証と明記する
- [ ] `workflow_dispatch` だけで起動する GitHub Actions が用意され、push／PR で自動実行されないことが設定上確認できる
- [ ] プロジェクト実装が未着手でも、初期 workspace と最低限のスモークテストが成功する
- [ ] #3 が転記できるセットアップ手順・検証コマンド・前提条件を記録し、#1 の完了を待たずに本 Issue を閉じられる

## Deferred Design Concerns

| 論点 | 本基盤での扱い | 判断・移管先 | 未対応のままのリスク |
| --- | --- | --- | --- |
| 最終的なクレート分割・公開 API（`ruviform-audio` 等） | 本基盤は最小契約のみ確定 | #1 の詳細設計 | 仮置き構造を固定しすぎると後続設計を拘束する |
| Rust バージョン・MSRV の確定 | #2 では根拠を残した仮置き | #1 で再評価 | 根拠の記録がないと再評価の前提が失われる |
| `release` プロファイルの `lto`・`strip`・`codegen-units`・`panic` チューニング、シンボルを残す `profiling` プロファイル | 当面は Cargo 標準を維持 | 性能測定が必要になった時点（#1 以降） | デバッグ情報と最適化のトレードオフを未検討のまま固定するリスク |
| 対応 OS・macOS／Windows CI matrix の拡張 | まず Linux のみ | #1 の対象プラットフォーム設計後 | 未検証 OS の差異（改行・パス・導入方法）が検知されない |
| `wgpu`・WebView・音声デバイス・codec のシステム依存 | 採用未決定 | 採用決定・検証対象確定後に段階導入 | デバイス依存テストが CI を不安定にする |
| `cargo-nextest`・`cargo-llvm-cov`・ベンチマーク補助ツール | 必須化しない | 導入コスト・移植性の評価後 | 不要な依存が保守コストになる |
| 自動トリガーの導入・branch protection の必須チェック設定 | 原則手動のみ | 別 Issue の明示的方針変更 | 恒常的な Pending や未検証マージのリスク |
| `apm.yml` の `project-template` リネーム、README／AGENTS 本文改稿 | #3 の責務 | #3 実施時（APM lockfile 再確認を伴う） | テンプレート文言が残存する |

## Sources and Relations

- 一次ソース: GitHub Issue [#2 [chore] mise/APM初期セットアップ・lockfile生成、Rust開発基盤と手動CIを整備する](https://github.com/scarecrow173/ruviform/issues/2)（`relations.source` / `relations.derives-from`）。目的・着手順序・現状・作業項目・受け入れ条件・スコープ外はすべてこの Issue を正本とする。
- 関連 Issue（外部）: #3（セットアップ手順・検証コマンドの README/AGENTS 転記）、#1（詳細設計：クレート分割・対象プラットフォーム・RT-safe 契約等）。本 spec は #1 の完了を前提としない。
- 関連ドキュメント（`relations.related`）: リポジトリルートの [`AGENTS.md`](../../AGENTS.md)。mise／APM の運用契約（`mise.toml` と `apm.yml` の正本化、`mise install --locked`、lockfile の追跡方針等）を定めており、本 spec の前提となる。
- 参考資料（`relations.references`）: Cargo Book の Profiles／Build Performance／Configuration、`cheaper` PR #109（CI ディスク枯渇の参考事例。プロファイル設計はそのままコピーしない）。
