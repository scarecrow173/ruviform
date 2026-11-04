# AGENTS.md

このリポジトリは Ruviform — Rust を中心とした再利用可能なマルチメディア基盤（設計・初期セットアップ段階）です。
以下はエージェント共通の作業契約（規約）です。タスク・クレート構成・CI 等の詳細リファレンスは各節のポインタ先が正本です。

## 開発・設計原則

- 変更前に対象挙動を診断し、症状と原因・破られた不変条件・契約・責務を特定してから修正方法を選ぶ。下流の補正より、不正な状態を生む源を、その責務を所有するレイヤーで修正する（呼び出し元での補正重複より authoritative な実装一箇所を優先）。
- 最小テキスト差分ではなく根本問題を正しく解決する最小の整合的変更を選び、欠陥設計を温存しない。複数呼出元の同種補正・同一ルール分散・多箇所協調・workaround 重積・責務分散が原因なら、局所例外より抽象化・責務境界の修復を優先する。
- KISS / YAGNI / DRY / SOLID は意図的に適用し、巧妙さより明示的で理解しやすいコードを優先する。
- 破られた契約・不変条件を検証するテストを優先し、変更の規模とリスクに比例した検証を行い、未実行の検証を成功と報告しない。
- 契約・責務の所在特定に必要な範囲まで依存先・呼び出し元を追う。管理下の根本原因は修正し、境界外は封じ込め・適応・隔離明示した workaround を用いる。非因果の構造問題は変更せず別途報告する。

各原則の詳細な説明は [docs/development-environment.md](docs/development-environment.md)「附録: 開発・設計原則（詳細版）」を参照。

## リポジトリの作業契約

- 調査・レビューのみの依頼ではファイルを編集しない。
- 変更依頼では、依頼範囲に必要な実装・設定・テストの整合性を保ち、無関係な変更や既存の作業を保持する。
- 調査の深さはタスクに応じて調整する。機械的な変更に、不具合調査と同じ深さの原因追跡を強制しない。
- 公開APIや外部との契約、永続データ形式などの互換性を変える場合は、影響を確認して変更方針を明示する。
- 完了時には変更内容、実行した検証と結果、未実行の検証、残る問題を区別して報告する。
- 実装されていない API・クレート・検証を、実在するものまたは実行済みと報告しない。

## 開発環境（mise）

- 開発ツールとタスクの正本は `mise.toml`。uv / prek / git-lfs / apm / pipx 系共通基盤ツールと、pin 済み Rust（`rust = "1.99.0"`）・actionlint・`node = "26.12.1"`（Issue #83 修正版）を管理する。タスク一覧と説明は `mise tasks` で確認できる。
- 初回セットアップ（Windows 11 x64・Git Bash で検証済み）:

```sh
mise install      # mise.lock 記録の固定バージョンを復元
mise run setup    # apm install + prek install + git lfs install --local
# lockfile 厳格検証が必要な場合（apm.yml と apm.lock.yaml の非同期を検出）
mise exec -- apm install --frozen
```

- 管理対象ツールの直接起動は原則 `mise exec -- <command>`。
- Git フックは言語非依存の軽量チェックのみ（正本 `.pre-commit-config.yaml`、prek で管理）。lint・テスト・ビルド等の言語別検証は mise タスクで管理する。全ファイルへのフック適用は `mise exec -- prek run --all-files`。
- Git LFS は mise でバージョン管理し `.gitattributes` で対象を定義する。LFS を要する git コマンドは mise 管理ツールが PATH 上にある環境で実行（例: `mise exec -- git push`）。初回 clone で LFS オブジェクト未取得なら `mise exec -- git lfs pull`。
- lockfile は再現性のため追跡し、生成条件の変更時に再生成する: `mise.lock` は `mise.toml` `[tools]` 変更時に `mise install` → `mise lock`、`apm.lock.yaml` は `apm.yml` 変更時に `apm install` → `apm lock`、`Cargo.lock` は `[dependencies]` 変更時に `cargo build` / `cargo generate-lockfile` で自動更新。不整合時は `git checkout -- <lockfile>` で戻して再生成する。
- lazy ツール・freshness 機構・lockfile 内部の詳細は [docs/development-environment.md](docs/development-environment.md) を参照。

## Rust ワークスペースと暫定開発規約

### ワークスペース構成

- ルート `Cargo.toml` がワークスペースの正本（`resolver = "2"`・`edition = "2024"`・`rust-version = "1.89"`・`[workspace.lints]`・`[profile.dev]`/`[profile.ci]`。`version`・`license` は仮置き。`publish = false` はメンバーが `publish.workspace = true` で継承）。
- メンバー構成は [crates/README.md](crates/README.md) が索引。各クレートの責務・許可依存層・RT 制約は `crates/<name>/README.md` と crate-level doc が正本。層構造・`path` 依存方向・クレート DAG の正本は [docs/designs/ruviform-architecture-design-baseline.md](docs/designs/ruviform-architecture-design-baseline.md)（Issue #1・approved）。`members`・`path` 依存は実装 Issue で段階追加。上記以外の公開 API は実装 Issue で確定する。
- adapter 層（例: `crates/ruviform-audio-device`、Issue #23・[docs/adr/select-audio-device-backend.md](docs/adr/select-audio-device-backend.md)）は engine と同じ利用側層で、foundations/Primitives への下向き依存のみを持つ。バックエンド（cpal 等）を内部依存に閉じ込め、その型・エラー・feature 名を公開 API に露出せず、コアクレートからの強制依存・adapter→adapter 依存は作らない。

### 検証入口

- cargo / npm / node 等の検証コマンドは直接実行せず、`mise.toml` のタスクを `mise run <task>` 経由で呼ぶ。ローカル開発と CI で同じ入口を使う。タスク一覧・実行内容・feature 条件・依存チェーンは `mise tasks` と `mise.toml` が正本。
- `*-ci` タスクと `verify-ci` は CI 用途限定。成果物が `target/ci/` に `target/debug/` と並存して容量を消費するため常用しない。
- CI の lane 構成（required jobs・OS matrix・cache・Shadow Mode）と Consumer 影響分類の運用詳細は [docs/development-environment.md](docs/development-environment.md) を参照。

### 暫定規約

- `unsafe` は原則禁止。必要な場合のみ SAFETY コメント付きの局所許可（`#[expect(unsafe_code)]`、未対応なら `#[allow(unsafe_code)]`）で例外的に認める。`lint` / `lint-ci` の `-D warnings` ゲートで局所許可のない `unsafe` は検証を通過しない。
- feature: default feature は最小限とする。新規 feature・optional 依存は追加理由を PR に記録する。
- 依存追加: メンテナンス状況・ライセンス・依存木サイズを確認し理由を記録する。`Cargo.lock` は追跡を継続する。
- lint: `workspace.lints` が正本。CI は `clippy -- -D warnings` を必須ゲートとする。rustfmt は標準設定（`rustfmt.toml` は必要時に追加）。
- MSRV clippy 検証（Issue #223）: lint gate の対象は pin toolchain のみで、MSRV 1.89 上の clippy は情報 lane。1.89 検出器のみが発火する `float_cmp` / `similar_names` は toolchain 版差の既知ノイズで、MSRV 上で検査する場合は `-A` フラグで除外する（コマンドは [docs/development-environment.md](docs/development-environment.md) 参照）。pin gate を弱めないため `workspace.lints` で `allow` せず、これら以外の失敗は機械追加せず個別に判定する。
- 公開 enum: クレート外から到達可能な全公開 enum に enum レベルの `#[non_exhaustive]` を付与する（クレート内部限定 enum は対象外）。`#![allow(clippy::exhaustive_enums)]` は置かず、`workspace.lints.clippy` の `exhaustive_enums = "warn"` を `-D warnings` ゲートで強制する。正本: [docs/adr/public-enum-non-exhaustive-policy.md](docs/adr/public-enum-non-exhaustive-policy.md)。
- doc test: 公開 API の例は doctest に書き、`cargo test` で検証する。
- RT 検証レーン（Issue #22）: `rt_` プレフィックスの RT 契約テストは `assert_no_alloc` 測定窓内で処理経路を実行し、窓内の alloc/dealloc はテストバイナリを abort させて CI で検出する（fixture・warm-up は窓外）。ruviform-rt 既存の `alloc_counter` は変更せず併存し、新規の検証用途には `assert_no_alloc` を使う。CLAP アダプタは `thread-check` 拡張を `plugin.init` で一度照会し、非オーディオスレッド応答の `process` を defined rejection する（未提供・応答不定は従来経路）。機械検出の対象は alloc/dealloc と当該スレッド契約のみ — その他の静的検証不能な制約はドキュメント契約 + コードレビュー観点であり、CI 成功は RT 安全性の完全証明ではない。

### アーキテクチャ確定契約（設計ベースライン）

Issue #1 由来の設計ベースライン（[docs/designs/ruviform-architecture-design-baseline.md](docs/designs/ruviform-architecture-design-baseline.md)、approved）で確定した契約の要点。詳細な定義・根拠は同書が正本であり、齟齬時は同書を優先する。

- RT/オフライン分離: RT 経路と非 RT を API・ライフサイクルで分離（`Processor` の `prepare`/`process` 分離と `RtContext` capability）。RT クリティカルパスでは alloc/dealloc 禁止、ブロッキング操作・I/O・無制限の待機/リトライ・非有界処理を制限。「RT-safe」等の保証範囲は API 単位で定義・明示し、未確定 API を RT-safe と断定しない。CI 成功は完全証明ではなく、OS レベルのハードリアルタイム保証は提供しない。
- SIMD: スカラー参照実装を必須とし、カーネルごとに「ビット完全一致」または「ulp 上限内」の等価性契約でスカラー参照検証。stable Rust のみ使用し、`unsafe` は暫定規約どおり局所許可のみ。
- クレート依存方向: 依存は上位 → 下位のみ、循環依存は禁止。基礎クレートから GUI・オーディオデバイス・GPU への強制依存を作らない。

### 共通基盤・Observability・Diagnostics・Debug の設計方針（必要時参照）

- 設計方針の参照先は [docs/designs/shared-foundations-and-modularization-policy.md](docs/designs/shared-foundations-and-modularization-policy.md)（Issue #101）。該当機能や RT 計測経路・feature 境界を新設・変更・レビューする際に同書の該当節を参照する（無関係な変更に全読は要求しない）。
- `ruviform-foundation` / `ruviform-observability` / `ruviform-debug` / `ruviform-platform` 等は現時点では構成候補であり実装済みを意味しない。既存 accepted ADR と矛盾する変更は、必要な ADR 改訂を経て行う。

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
- 公開 API / feature / behavior / dependency / CI 基盤を変更する PR は、本文の Consumer 影響欄で Synvelis・AudioLens を独立して `impacted` / `no impact` / `unknown` に分類する（Issue #219・記入形式は `.github/PULL_REQUEST_TEMPLATE.md` が正本）。`unknown` 残置は互換性未確認であり、merge 前にレビュアーが分類欄の記入と `unknown` 解消先リンクを確認する。運用詳細は [docs/development-environment.md](docs/development-environment.md) を参照。
