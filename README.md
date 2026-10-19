# Ruviform

Ruviform は、Rust を中心とした再利用可能なマルチメディア基盤です。現在は設計・初期セットアップ段階であり、開発環境と最小の Rust ワークスペースのみが整っています。

音声処理と動画処理、およびその下位の計算基盤（DSP・解析・codec・container・SIMD）を、同じ基盤の上で再利用可能な形で提供する方向性を構想しています。対象読者は、リポジトリの訪問者と将来の利用者、コントリビューター、AI エージェント、および詳細設計を行う Issue [#1](https://github.com/scarecrow173/ruviform/issues/1) の設計者です。クレート分割・公開 API・対応プラットフォームなどの詳細設計は #1 の後続タスクで確定します。

## Status

開発ステータス: **設計・初期セットアップ段階**

### 実在・検証済み

以下は実在し、Windows 11 x64（Git Bash）で検証済みです。

- Rust 1.99.0 ツールチェーン（`mise.toml` で pin、`mise.lock` で固定復元）
- 最小 Rust ワークスペース: ルート `Cargo.toml` と最小メンバー `crates/ruviform-core`（仮置き）の smoke test
- `mise.toml` の検証タスク群（`setup` / `fmt-check` / `check` / `lint` / `test` / `build` / `*-ci` / `verify-ci`）
- `workflow_dispatch` 専用 CI（`.github/workflows/ci.yml`）— ubuntu-latest 上で実ディスパッチ済み（fmt-check / check-ci / lint-ci / test-ci 全て pass）

### 構想（未実装）

- 音声・動画クレート `ruviform-audio` / `ruviform-video`
- 下位計算基盤: DSP・解析・codec・container・SIMD

### 検討中（採否は #1 で決定）

- `wgpu`、WebView UI、ネイティブ UI、プラグイン連携

### 未確定（#1 で決定）

- 対応プラットフォーム、公開 API、サポート形式、ライセンス、edition / MSRV、最終的なクレート分割（`Cargo.toml` の `edition` / `rust-version` / `license` の現行値は仮置き）

## Quickstart

以下の手順は Windows 11 x64（Git Bash）でのみ実機検証済みです。**macOS / Linux では未検証**です。

### 前提条件

- mise が導入済みであること（winget や公式インストーラ等、リポジトリ外の前提）。clone 後の初回実行時に `mise trust` が必要です
- ネットワーク: GitHub（releases / API と APM の git 依存）、crates.io / static.crates.io、static.rust-lang.org、PyPI への到達
- git 認証: APM 依存の git リポジトリ取得に HTTPS 経由の git 認証（Git Credential Manager 等）が必要です。非対話環境では `GIT_TERMINAL_PROMPT=0` を推奨します
- 書き込み権限: mise による `~/.local` / `AppData` への書き込み、リポジトリ内 `.git/hooks` の更新（prek）と `.git/config` の更新（git lfs --local）が可能であること

### 手順

```sh
git clone https://github.com/scarecrow173/ruviform.git && cd ruviform

# 1. ツール復元（mise.lock からの固定復元が正本手順）
mise install --locked
#    global mise config が repo lockfile に無いツールを要求する環境では、
#    その外部ツール分のみ exit 1 になり得ます（repo 管理ツールは全て解決します）。
#    その場合は lockfile 非使用のフォールバック:
mise install

# 2. APM 依存・prek フック・リポジトリローカル Git LFS のセットアップ
mise run setup                    # = apm install && prek install && git lfs install --local

# lockfile の厳格検証が必要な場合（apm.yml と apm.lock.yaml が非同期なら失敗）:
mise exec -- apm install --frozen

# 3. Rust 検証（ローカル開発・CI 共通の入口）
mise run fmt-check                # cargo fmt --all -- --check
mise run check                    # cargo check --workspace --all-targets
mise run lint                     # cargo clippy --workspace --all-targets -- -D warnings
mise run test                     # cargo test --workspace
mise run build                    # cargo build --workspace
mise run verify-ci                # CI 同等: fmt-check -> check-ci -> lint-ci -> test-ci（--profile ci）
```

`mise run verify-ci` と `*-ci` タスクは CI 用途向けで、成果物が `target/ci/` に `target/debug/` と並存して容量を消費するため、常用は想定していません。

## Repository layout

| パス | 役割 |
| --- | --- |
| `Cargo.toml` / `Cargo.lock` | Rust ワークスペース定義（共通 lint・`[profile.dev]` / `[profile.ci]`）と依存 lockfile |
| `crates/ruviform-core/` | 最小メンバークレート（仮置き・smoke test） |
| `mise.toml` / `mise.lock` | 開発ツール・検証タスクの正本と固定復元用 lockfile |
| `apm.yml` / `apm.lock.yaml` | APM 依存パッケージ・対象エージェント・MCP の定義と lockfile |
| `.pre-commit-config.yaml` | prek による言語非依存の Git フック |
| `.gitattributes` | 改行・テキスト属性と Git LFS 対象拡張子のルール |
| `.gitignore` | 秘密情報・生成物（`target/`・APM 配布物等）の除外 |
| `.github/workflows/ci.yml` | `workflow_dispatch` 専用の手動 CI |
| `AGENTS.md` | エージェント共通の作業契約・開発規約 |
| `README.md` | このファイル |
| `docs/` | 設計・仕様・ADR・計画・タスク・実装記録などの文書群 |

LFS とテストフィクスチャの方針: `.gitattributes` の音声（wav / flac / mp3 / ogg / opus / aac / m4a / aif / aiff）・動画（mp4 / mov / mkv / avi / webm / m2ts）・大型編集ソース（psd / blend）を LFS 対象とするルールは維持します。小さなテストフィクスチャは非メディア形式（JSON / YAML 等）で Git に置き、CI は必要になるまで LFS オブジェクトを取得しません。

## Documentation

- [docs/designs/overview.md](docs/designs/overview.md) — 設計成果物の全体マップ
- [docs/designs/dev-foundation.md](docs/designs/dev-foundation.md) — Rust 開発基盤の設計（Issue #2 由来）
- [docs/specs/](docs/specs/) — 仕様と Acceptance Criteria
- [docs/adr/](docs/adr/) — 設計判断の記録（ADR）
- Issue [#1](https://github.com/scarecrow173/ruviform/issues/1) — プロダクト本体の詳細設計（後続タスク）
- Issue [#2](https://github.com/scarecrow173/ruviform/issues/2) — Rust 開発基盤の構築（完了）
- Issue [#3](https://github.com/scarecrow173/ruviform/issues/3) — リポジトリ文書・識別子の Ruviform 向け整備

## Contributing

- 作業契約・開発規約の正本は [AGENTS.md](AGENTS.md) です。変更は GitHub Issue を起点に進めます
- コミットは論理的な変更単位に分け、Conventional Commits を基本とします
- prek によるコミットフックが有効です（`mise run setup` で導入）。全ファイルへの手動実行は `mise exec -- prek run --all-files` を使用してください

## License

**ライセンスは未決定です。** `LICENSE` ファイルは存在せず、配布方針は確定していません。`Cargo.toml` の `license = "MIT"` は配布方針確定までの仮置きであり、ライセンスの決定を意味しません。
