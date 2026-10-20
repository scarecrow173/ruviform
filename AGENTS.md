# AGENTS.md

このリポジトリは Ruviform — Rust を中心とした再利用可能なマルチメディア基盤（設計・初期セットアップ段階）です。
以下はこのリポジトリで作業するエージェント共通の作業契約であり、開発・設計原則、リポジトリ固有の作業契約、Rust ワークスペースの暫定開発規約、mise / APM の運用、Git と安全性の方針を定めます。

## 開発・設計原則

### 変更前に診断する

- 変更する前に、対象となる挙動を理解する。
- 観測された症状と、その背後にある原因を区別する。
- 期待する挙動が最初に崩れる地点を特定できるまで、必要な制御フロー、データフロー、状態遷移を追う。
- 修正方法を選ぶ前に、破られた不変条件、契約、または責務を特定する。
- 下流で補正するより、不正な状態を生み出している源を修正することを優先する。

### 正しい境界で修正する

- 安全に変更できるなら、破られた不変条件や責務を所有するレイヤーで問題を修正する。
- 複数の呼び出し元で同じ補正を重複させるより、ルールやポリシーの authoritative な実装を一箇所に置く。
- 信頼できない入力や外部データの検証は、適切なシステム境界で行う。
- 必要性を示す根拠がないまま、信頼された内部コード全体へ防御的チェックを拡散させない。
- 呼び出し側が実装詳細を知らなければ正しく振る舞えないなら、抽象化境界を見直す。

### 最小diffより整合した変更を優先する

- 最小のテキスト差分ではなく、根本の問題を正しく解決する最小の整合的変更を選ぶ。
- 「最小」とは、意図された挙動、不変条件、責務、契約を回復するために必要な範囲を超えないことを意味する。
- 既存構造自体が問題の原因である、または正しい修正を不自然に難しくしている場合は、局所的なリファクタリングを行ってよい。
- diffを小さく保つためだけに、欠陥のある設計を温存しない。
- 周辺コードを綺麗にできるという理由だけで、無関係なリファクタリングを行わない。

### 構造的な問題の兆候を認識する

- 複数の呼び出し元が同じ不正状態を補正しているなら、設計を見直す。
- 同じドメインルールやポリシーが複数箇所に存在するなら、設計を見直す。
- 一つの挙動変更のために、多数の無関係な場所を協調して変更する必要があるなら、設計を見直す。
- 既存のworkaroundの上にさらにworkaroundを追加しようとしているなら、設計を見直す。
- 責務の分散によって正しい実装が不自然に難しくなっているなら、設計を見直す。
- これらの構造的問題が現在のタスクの原因なら、局所的な例外を増やすより、抽象化または責務境界の修復を優先する。

### 実装原則は意図的に適用する

- KISS: 必要な挙動と不変条件を正しく保てる中で、最も単純な設計を選ぶ。
- YAGNI: 仮想的な将来要件のためだけに、柔軟性、抽象化、基盤を追加しない。
- DRY: 同じ責務に属する知識やポリシーの重複は集約するが、表面的に似ているだけのコードを誤った抽象化で統合しない。
- SOLID: 責務境界、依存方向、代替可能性、保守性を改善する場合に用い、機械的には適用しない。
- 巧妙なコードより、明示的で理解しやすいコードを優先する。

### 実際の解決を検証する

- テストは正しさを検証するものであり、実装方法を定義するものではない。
- 観測されたテストを通すためだけのハードコードや特殊ケースを追加しない。
- 不具合修正では、表面的な症状だけでなく、破られた契約や不変条件を検証するテストを優先する。
- 適切な場合は、意味のある最下位レイヤーと外部から観測できる挙動の両方で検証する。
- 検証の範囲は、変更の規模とリスクに比例させる。
- 実際に実行して成功していない検証を、成功したと報告しない。

### スコープと比例性を守る

- 問題を理解するのに十分な関連コードを調べるが、理由のない網羅的探索は行わない。
- 関連する契約、不変条件、責務の所在を特定するのに必要な範囲まで依存先や呼び出し元を追う。
- 原因となる箇所が管理下にあり安全に変更できるなら、根本原因の修正を優先する。
- 根本原因が変更境界の外にある、または安全に修正できない場合は、封じ込め、適応、フォールバック、workaroundを用いる。
- 意図的なworkaroundは、適切な境界へ隔離し、その存在を明確にする。
- より深い構造問題を発見しても、現在の変更の原因でなければ変更せず、別途報告する。

## リポジトリの作業契約

- 調査・レビューのみの依頼ではファイルを編集しない。
- 変更依頼では、依頼範囲に必要な実装・設定・テストの整合性を保ち、無関係な変更や既存の作業を保持する。
- 調査の深さはタスクに応じて調整する。機械的な変更に、不具合調査と同じ深さの原因追跡を強制しない。
- 公開APIや外部との契約、永続データ形式などの互換性を変える場合は、影響を確認して変更方針を明示する。
- 完了時には変更内容、実行した検証と結果、未実行の検証、残る問題を区別して報告する。
- 実装されていない API・クレート・検証を、実在するものまたは実行済みと報告しない。

## 開発環境（mise）

- 開発ツールとタスクの正本は `mise.toml` とする。
- `mise.toml` は uv / prek / git-lfs / pipx 系の共通基盤ツールに加え、このリポジトリで pin した Rust ツールチェーン（`rust = "1.99.0"`）と actionlint を管理する。`fmt` / `check` / `lint` / `test` / `build` 等の検証タスクも同ファイルが正本（対応表は `## Rust ワークスペースと暫定開発規約` 節を参照）。
- 初回セットアップでは、miseをインストールした後に以下を実行する（Windows 11 x64・Git Bash で検証済み。macOS / Linux は未検証）。

```sh
# ツール復元（mise.lock に記録済みの固定バージョンを復元）
mise install
# global mise config でリポジトリ管理外のツールを設定している環境では
# それらも併せて解決・インストールされます

# APM 依存・prek フック・リポジトリローカル Git LFS のセットアップ
mise run setup

# lockfile の厳格検証が必要な場合（apm.yml と apm.lock.yaml が非同期なら失敗）
mise exec -- apm install --frozen
```

- 管理対象ツールを直接起動する必要がある場合は、原則 `mise exec -- <command>` を使用する。
- `mise run setup` は `apm install` で依存関係を復元し、`prek install` でコミットフックを、`git lfs install --local` でリポジトリローカルの Git LFS フィルターとフックを有効化する。prekのフック定義は `.pre-commit-config.yaml` を正本とする。
- Gitフックでは言語非依存の軽量チェックのみ実行し、Lint・テスト・ビルド等の言語別検証は `mise.toml` のタスクで管理する。
- 設定済みフックを全ファイルに実行する場合は `mise exec -- prek run --all-files` を使用する。
- Git LFS は `mise.toml` でバージョン管理し、`.gitattributes` で対象を定義する。LFSを必要とするGitコマンドはmise管理のツールがPATH上にある環境で実行する（例: `mise exec -- git push`）。初回cloneでLFSオブジェクトが未取得なら、セットアップ後に `mise exec -- git lfs pull` を実行する。
- lockfile の再生成条件は以下のとおり。

| lockfile | 再生成トリガ | 手順 |
| --- | --- | --- |
| `mise.lock` | `mise.toml` の `[tools]` 変更・バージョン更新時 | `mise install` → `mise lock`（現行ツールは github / aqua 系バックエンドのため `.mise/locks/` sidecar は生成されない。PyPI・npm 解決の pipx / npm 系ツールを追加して sidecar が生成された場合は `.gitignore` の `!.mise/locks/` 例外によりそのままコミット対象になる） |
| `apm.lock.yaml` | `apm.yml` の dependencies / targets / MCP 変更時 | `apm install`（lock 自動更新）→ `apm lock` で整合確認 |
| `Cargo.lock` | ワークスペースの `[dependencies]` 変更時 | `cargo build` / `cargo generate-lockfile` で自動更新 |

- いずれかの lockfile が不整合となった場合は `git checkout -- <lockfile>` で戻し、上記の手順で再生成する。

## Rust ワークスペースと暫定開発規約

### ワークスペース構成

- ルート `Cargo.toml` がワークスペースの正本。`[workspace]`（`resolver = "2"`）、`[workspace.package]`（`version = "0.1.0"` は仮置き。`edition = "2024"`・`rust-version = "1.85"` は Issue #34 で確定。`license` の現行値は仮置き。`publish = false` は未決定クレートを公開しない licensing ADR の原則であり、メンバーは `publish.workspace = true` で継承する）、`[workspace.lints]`（共通 lint の正本）、`[profile.dev]` / `[profile.ci]` を定義する。
- メンバーは Phase 1 基盤6クレート（Issue #17 で仮置き `crates/ruviform-core` を分割置換）: Primitives `crates/ruviform-math`・`crates/ruviform-rt`（最下位）→ `crates/ruviform-media-core`・`crates/ruviform-exec`、Domain foundations `crates/ruviform-audio-core`・`crates/ruviform-video-core`。各クレートの crate-level doc には責務・許可依存層・RT 制約を転記する。media-core・audio-core・video-core は Issue #18 でデータモデル API が確定・実装され、audio-core・video-core → media-core の `path` 依存が設定済み。math・rt・exec は引き続きスケルトン（`version()` + doctest + smoke test。rt は `RtContext` のみ実装済み）であり、残りの `path` 依存は API 確定に伴い追加する。クレート DAG・分割案は [docs/designs/ruviform-architecture-design-baseline.md](docs/designs/ruviform-architecture-design-baseline.md)（Issue #1 由来・approved）に記録され、`members` は実装 Phase に応じて段階追加される。上記3クレート以外の公開 API は引き続き未確定であり、実装 Issue で確定する。

### 検証入口

- cargo コマンドは直接実行せず、`mise.toml` のタスクを `mise run <task>` 経由で呼ぶ。ローカル開発と CI で同じ入口を使う。

| mise タスク | 実行内容 | 用途 |
| --- | --- | --- |
| `setup` | `apm install` + `prek install` + `git lfs install --local` | 初回セットアップ |
| `fmt` | `cargo fmt --all` | 整形（ソースを書き換える） |
| `fmt-check` | `cargo fmt --all -- --check` | 整形チェック（非破壊） |
| `check` | `cargo check --workspace --all-targets` | 型チェック（dev プロファイル） |
| `lint` | `cargo clippy --workspace --all-targets -- -D warnings` | lint（dev プロファイル） |
| `test` | `cargo test --workspace` | テスト（dev プロファイル） |
| `build` | `cargo build --workspace` | ビルド（dev プロファイル） |
| `test-rt` | `cargo test --workspace rt_`（`rt_` プレフィックス命名の RT テストを名前フィルタで選択。0 件一致でも成功するため RT テスト未存在時はスキップ相当） | RT 契約テスト（現時点では RT テスト未実装） |
| `bench` | `cargo bench --workspace` | ベンチマーク入口（ベンチマーク未実装のため現状は実質 no-op） |
| `check-ci` / `lint-ci` / `test-ci` / `build-ci` | 上記各コマンドを `--profile ci`・`CARGO_INCREMENTAL=0` で実行 | CI 用途限定 |
| `verify-ci` | `fmt-check` → `check-ci` → `lint-ci` → `test-ci` を順次実行 | CI 同等の検証列 |

- `*-ci` タスクと `verify-ci` は CI 用途限定。成果物が `target/ci/` に `target/debug/` と並存して容量を消費するため常用しない。
- `.github/workflows/ci.yml` は `workflow_dispatch` 専用の手動検証ワークフロー（push / pull_request / schedule 等の自動トリガーは設けない）。`verify` ジョブは `ubuntu-latest` / `windows-latest` / `macos-latest` の 3 OS matrix（ディスク計測ステップは ubuntu のみ）、`rt-verify`（`mise run test-rt`）と `bench`（`mise run bench`）ジョブは `ubuntu-latest` 単独。

### 暫定規約

- `unsafe` は原則禁止。必要な場合のみ SAFETY コメント付きの局所許可（`#[expect(unsafe_code)]`、未対応なら `#[allow(unsafe_code)]`）で例外的に認める。`workspace.lints.rust.unsafe_code = "warn"` は `lint` / `lint-ci` の `-D warnings` ゲートで実質 deny として機能し、局所許可のない `unsafe` は検証を通過しない。
- feature: default feature は最小限とする。新規 feature・optional 依存は追加理由を PR に記録する。
- 依存追加: メンテナンス状況・ライセンス・依存木サイズを確認し理由を記録する。`Cargo.lock` は追跡を継続する。
- lint: `workspace.lints` が正本。CI は `clippy -- -D warnings` を必須ゲートとする。rustfmt は標準設定（`rustfmt.toml` は必要時に追加）。
- doc test: 公開 API の例は doctest に書き、`cargo test` で検証する。

### アーキテクチャ確定契約（設計ベースライン）

以下は Issue [#1](https://github.com/scarecrow173/ruviform/issues/1) 由来の設計ベースライン（[docs/designs/ruviform-architecture-design-baseline.md](docs/designs/ruviform-architecture-design-baseline.md)、approved）で確定した契約の要約。詳細な定義・根拠は同書の各節が正本であり、本節と齟齬がある場合は同書を優先する。

- RT/オフライン分離: リアルタイム（RT）経路と非 RT を API・ライフサイクルで分離する。処理契約は `Processor`（`prepare`/`process` 分離）と `RtContext` capability が定義する。RT クリティカルパスでは alloc/dealloc を禁止し、ブロッキング操作・I/O・無制限の待機/リトライ・非有界処理を制限する。「RT-safe」等の保証範囲は API 単位で定義・明示し、未確定の API を RT-safe と断定しない。検証可能な違反は CI で検出するが、CI 成功は完全証明ではない。OS レベルのハードリアルタイム保証は提供しない。正本: 同書「RT／非 RT の分離方式」「RT 契約（API 単位の保証定義）」節。
- SIMD: スカラー参照実装を必須とし、カーネルごとに「ビット完全一致」または「ulp 上限内」の等価性契約を定義してスカラーを参照として検証する。stable Rust のみを使用し、`unsafe` は暫定規約どおり SAFETY コメント付きの局所許可のみで例外的に認める。SIMD 二層構造（ランタイム分岐＋フォールバック、dispatch 層の `ruviform-math` 集約を含む構造仮説）とバックエンド選定の採否は要 PoC。正本: 同書「DSP／SIMD 設計」採否。
- クレート依存方向: 依存は上位 → 下位のみとし、循環依存は禁止。基礎クレートから GUI・オーディオデバイス・GPU への強制依存を作らない。詳細な層内順序・adapter ルール・契約型の所有は同書「レイヤー構造と依存方向ルール」節が正本。

### ライセンス注記

- ライセンスは未決定であり、`LICENSE` ファイルは存在しない。`Cargo.toml` の `license = "MIT"` は配布方針確定までの仮置きであり、ライセンスの決定を意味しない。

## AIエージェント環境（APM）

- APM依存パッケージ、対象エージェント、MCPの定義は `apm.yml`（パッケージ名 `ruviform`）を正本とする。
- 共通の開発支援パッケージをAPM経由で導入し、各パッケージの詳細な指示をこのファイルに複製しない。
- `apm install` で依存関係を復元し、生成された `apm.lock.yaml` を確認してコミットする。
- `apm.yml` を変更したら依存関係を再解決し、lockfileと生成物の整合性を確認する。
- `apm_modules/`、各エージェント向けの配布生成物、およびSerena等のローカル状態はコミットしない。追跡する設定と生成物の区別は `.gitignore` を確認する。
- APMで管理されるMCPは `apm.yml` に従って復元する。Serenaの起動にはmise管理下の実行環境を使用する。

## Gitと安全性

- コミットは論理的な変更単位に分け、Conventional Commitsを基本とする。
- 明示的な依頼なく、履歴書き換え、強制push、ブランチ削除などの破壊的なGit操作を行わない。
- 秘密情報、環境固有の認証情報、キャッシュやビルド生成物をコミットしない。
- `.gitattributes` と `.gitignore` は共通ルールを保持し、言語別・製品固有の例外は必要になったときに追加する。
- テストフィクスチャは非メディア形式（JSON / YAML 等）で Git に置き、CI はフィクスチャが必要になるまで LFS オブジェクトを取得しない（`.gitattributes` の音声・動画・大型編集ソース拡張子の LFS ルールは維持。現行 `.github/workflows/ci.yml` も LFS 取得なし）。
- `mise.lock`・`apm.lock.yaml`・`Cargo.lock` は生成物であっても再現性のため追跡する。これらを他の一時生成物と混同しない。
