---
id: "SPEC-034Ivc5nj1AE6P2PcZU6Ki"
type: "spec"
status: "approved"
title: "テンプレート由来のドキュメント・識別子をRuviform向けに置き換える"
created: "2026-10-19"
updated: "2026-10-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  changes:
    added: []
    modified:
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "apm.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".pre-commit-config.yaml"
      - type: "file"
        path: ".gitignore"
      - type: "file"
        path: ".gitattributes"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "establish-rust-dev-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../README.md"
    - "../../apm.yml"
    - "../../mise.toml"
    - "../../.pre-commit-config.yaml"
    - "../../.gitignore"
    - "../../.gitattributes"
    - "../../Cargo.toml"
    - "../designs/dev-foundation.md"
    - "../designs/overview.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  derived-by:
    - "../adr/replace-template-derived-docs-metadata-with-ruviform.md"
    - "../designs/replace-template-derived-docs-with-ruviform.md"
    - "../plans/replace-template-derived-docs-with-ruviform.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/2"
    - "https://github.com/scarecrow173/ruviform/issues/1"
    - "../impl/ir/verify-reproducible-setup-and-work-memo.md"
  defers: []
  deferred-by: []
---

# テンプレート由来のドキュメント・識別子をRuviform向けに置き換える

## What Should Be Built

GitHub Issue #3 の作業成果物として、リポジトリ初期化時に引き継いだ**汎用プロジェクトテンプレート由来の説明・識別子・作業規約**を、Ruviform の現状と開発方針に合わせて更新する。単なる文言置換ではなく、利用者・コントリビューター・AI エージェントが Ruviform の実態を誤解しない内容にする。テンプレート由来の有用な仕組み（mise / APM / prek / Git LFS の運用、設計原則）は維持する。

- `README.md` の Ruviform 固有化（タイトル・概要・対象読者・実測済み Quickstart・リポジトリ構成・設計文書/Issue への導線）
- `AGENTS.md` の作業契約の Ruviform 固有化（有用な設計原則は保持）
- `apm.yml` の `name` / `description` 更新と APM 命名制約の確認
- `mise.toml` のテンプレート前提コメント・task description の現状適合
- `.pre-commit-config.yaml` / `.gitignore` / `.gitattributes` のレビューと必要箇所のみの修正
- 全追跡ファイルのテンプレート由来プレースホルダー（`project-template`・`Project Template`・汎用説明）の洗い出しと更新

## Why Now

- 実施順序は **#2 → #3 → #1**。前提 Issue #2（Rust 開発基盤: mise/APM lockfile、Rust 1.99.0 ツールチェーン、最小クレート `crates/ruviform-core`、CI プロファイル、mise 検証タスク、手動 CI ワークフロー）は完了・マージ済み（PR #4）であり、README/AGENTS に転記可能な実測済み手順・前提条件・未検証項目が IR `verify-reproducible-setup-and-work-memo.md` の「作業メモ（#3 転記用）」に記録されている。
- 現状の `README.md` は `# Project Template` で「Use this template」による新規プロジェクト作成手順を持ち、`AGENTS.md` は「言語・フレームワーク非依存テンプレート」「派生プロジェクト」前提の記述と「テンプレートに言語固有の構成・CI・docs を追加しない」等の禁止事項を含み、`apm.yml` は `name: project-template` のままである。これらは Ruviform の実態と矛盾し、訪問者・コントリビューター・AI エージェントを誤導する。
- **#1（詳細設計）の完了は本 spec の前提ではない。** README は #1 の詳細設計に先行する構想の説明文書として、現時点の意図と未決定事項（最終クレート分割、対応プラットフォーム、公開 API、GPU/UI/プラグイン採否、ライセンス等）を正確に記載する。#1 完了後に必要なら README/AGENTS のアーキテクチャ記述を追従更新する。

## Users and Value

- **リポジトリ訪問者・将来の利用者**: Ruviform の目的・方向性（Rust を中心とした再利用可能なマルチメディア基盤）と開発ステータス（設計・初期セットアップ段階）を正確に把握でき、テンプレート用の手順に迷わされない。
- **コントリビューター**: #2 で実機検証済みのセットアップ・検証コマンドに従い、新規 clone から環境を再現できる。
- **AI エージェント**: Ruviform 固有の作業契約と検証入口を得て、未実装の API・クレート・未実行の検証を実在/実行済みと報告する誤りを防ぐ。
- **#1 の設計者**: 決定済み・暫定・未決定の境界が明示された README/AGENTS を前提に詳細設計へ進める。

## Scope

### In Scope

- `README.md` の全面改稿（Requirements 1）
- `AGENTS.md` の作業契約改稿（Requirements 2）
- `apm.yml` の `name`/`description` 更新と命名制約確認、`name`・依存・`targets` を変更する場合の `apm.lock.yaml` 再生成・整合確認（Requirements 3）
- `mise.toml` のコメント・task description の現状適合（Requirements 4）
- `.pre-commit-config.yaml` / `.gitignore` / `.gitattributes` のレビューと必要箇所のみの修正（Requirements 5）
- 追跡ファイル全体のテンプレート由来識別子・説明のスイープ（Requirements 6）
- 変更後の構文検証（APM YAML / mise TOML / prek 設定）と prek フック実行（Requirements 7）

### Out of Scope

- メディア機能・クレートの実装（`ruviform-audio` 等のコード自体は作らない）
- ライセンスの決定（`LICENSE` ファイルの作成・`Cargo.toml` の `license` 確定を含む）、フォーマット対応、OS 対応、GPU/UI/プラグイン採用の既成事実化
- #1 の詳細設計に属する決定（最終クレート分割・公開 API 契約・RT/オフライン API 保証・対象プラットフォーム戦略）を本 spec で確定させること
- テンプレート由来箇所の修正と無関係なインフラ基盤の再構成（mise/APM/prek/LFS の仕組み自体の作り直し）
- `docs/` 配下の既存設計文書の改稿（#1 以降の成果物として別途扱う）

## Requirements

### 1. `README.md` の Ruviform 固有化

- タイトル・概要・目的・対象読者を Ruviform 固有のものにする
- Rust を中心とした再利用可能なマルチメディア基盤という**現時点の方向性**と、音声・動画・下位計算基盤の関係を簡潔に説明し、詳細設計は #1 の後続タスクとして案内する
- `ruviform-audio` / `ruviform-video` と、その下位に置く DSP・解析・codec・container・SIMD 等の構想を、**計画・アーキテクチャ方向として**記載し、実装済み機能と混同させない
- `wgpu`、WebView UI、ネイティブ UI、プラグイン連携は**設計検討中の項目**として整理する
- 現在の開発ステータス（設計・初期セットアップ段階）を明示し、対応プラットフォーム・公開 API・サポート形式が未確定である旨を記載する
- 「Use this template」手順を削除し、このリポジトリ用の Quickstart として #2 で実機検証済みのコマンドを記載する: `mise install --locked`、`mise run setup`、`mise run fmt-check`、`mise run check`、`mise run lint`、`mise run test`、`mise run build`、`mise run verify-ci`（必要に応じて `mise exec -- apm install --frozen` 等の厳格検証も案内する）
- repository layout、参加方法（contribution guidance）、設計文書・Issue への導線を設ける。存在しない文書・テストを実在するものとして案内しない
- ライセンス表記は実際の `LICENSE` ファイルの存在・内容を確認してから記載する。現時点で `LICENSE` はリポジトリに存在しないため、ライセンス未決定であることを明示する（`Cargo.toml` の `license = "MIT"` は仮置きであり、配布方針の確定ではない）

### 2. `AGENTS.md` の作業契約の Ruviform 固有化

- 冒頭の「汎用テンプレート」「派生プロジェクト」説明を、Ruviform 固有の作業契約に書き換える
- 現行の有用な設計原則を保持する: 変更前の診断（症状と原因の区別）、責務境界での修正、スコープと比例性の保持、実行した検証と未実行の検証を区別した報告
- 実際の Rust ワークスペース（ルート `Cargo.toml` + `crates/ruviform-core`）と #2 で定義した検証規約を反映する。`cargo fmt` / `cargo clippy` / `cargo test` 等の実行方法は `mise run` タスク経由の記述に同期させる
- クレート依存方向・公開 API・`unsafe`・SIMD の前提・不変条件は**暫定的な設計原則**として示し、詳細な契約は #1 で確定する旨を明記する
- リアルタイム音声経路でブロッキング操作や意図しない allocation を回避する等、RT/オフラインを区別する**設計上の意図**を明記する。API ごとの具体的保証は #1 で決め、未確定の API を RT-safe と断定しない
- 「テンプレートに言語固有の構成・CI・docs を追加しない」等の**派生前提の禁止事項**を削除する
- APM/mise/prek/LFS の既存運用と、秘密情報・生成物（`apm_modules/`・`.agents/`・lockfile 追跡方針等）の取り扱い規約は維持する
- 実装されていない API・クレート・検証を実在/実行済みと報告しないことを明文化する

### 3. `apm.yml` の識別子更新

- `name: project-template` を `ruviform` に変更し、`description` を Ruviform に沿う内容にする。変更前に APM の命名制約・互換性を確認する
- `targets`（copilot/codex/claude/cursor/windsurf）、依存パッケージ、Serena/mise MCP 設定は、正当な理由がない限り維持する
- `name`・依存・`targets` のいずれかを変更する場合は `apm install` / `apm lock` を実行して `apm.lock.yaml` を再生成し、lockfile・APM 生成物との整合を確認する

### 4. `mise.toml` のコメント適合

- 「Infrastructure tools only; add a project language toolchain as needed.」等のテンプレート前提コメントを、Rust ツールチェーン（`rust = "1.99.0"`）追加済みの現状に合わせて修正する
- task description も実態に合わせて見直す。タスク定義の本体（#2 の成果）は変更しない

### 5. `.pre-commit-config.yaml` / `.gitignore` / `.gitattributes` のレビュー

- 各ファイルのコメント・フック・設定が Ruviform（Rust + 音声/動画アセット方針）に適するか確認し、**必要な箇所のみ**修正する。不要な全置換はしない
- `.gitattributes` の音声・動画ファイルを拡張子で一律 LFS 対象とするポリシーと小さなテストフィクスチャの取り扱いについて、#2 の結論（小さいフィクスチャは非メディア形式で Git に置く、CI はフィクスチャが必要になるまで LFS オブジェクトを取得しない）と整合することを確認し、方針を文書化する

### 6. 全追跡ファイルのプレースホルダースイープ

- `project-template`、`Project Template`、汎用テンプレート説明などのプレースホルダーを追跡ファイル全体から洗い出して更新する。現時点で確認済みの対象（`README.md`・`AGENTS.md`・`apm.yml`・`mise.toml` 等）に加え、着手時に `git ls-files` 相当で再確認する
- APM が生成する無視対象ファイル（`apm_modules/` 等）はコミットしない

### 7. 検証

- `apm.yml` / `mise.toml` / `.pre-commit-config.yaml` の構文を検証し、prek フック（`mise exec -- prek run --all-files` 等）を実行する
- README/AGENTS に記載するコマンドが #2 で実機検証済みの手順と一致することを確認する
- 未実行・未検証の条件（未検証 OS、未実施の GitHub Actions 実ディスパッチ等）は明示する

## Acceptance Criteria

Issue #3 の受け入れ条件に基づく。

- [ ] README/AGENTS/APM の名称・説明・手順が Ruviform の実態に合っている
- [ ] テンプレートを再利用するための手順や、Ruviform では不適切な派生プロジェクト向け制約が残っていない（追跡ファイルに `project-template` / `Project Template` / テンプレート前提の記述が残存しない）
- [ ] 設計中・未実装・実装済みの境界が明示され、README が #1 の詳細設計に先行する構想の説明であることが分かる。#1 に未決定事項を委譲し、詳細決定の完了を要求しない
- [ ] #2 の完了時点で確認済みの開発手順・検証タスク（`mise install --locked`、`mise run setup`、`mise run fmt-check` / `check` / `lint` / `test` / `build` / `verify-ci`）を README/AGENTS に反映する。未実行・未検証の条件は明示する
- [ ] `apm.yml` / `mise.toml` / `.pre-commit-config.yaml` の構文が有効で、prek フックが実行できる
- [ ] インフラ基盤を壊す無関係な再構成をせず、必要な差分で更新する
- [ ] 存在しない文書・実装・検証を実在/実行済みとして記載していない（`LICENSE`、公開 API、`ruviform-audio`/`ruviform-video`、RT-safe 保証、未実施の CI ディスパッチ等）

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 判断・移管先 | 未対応のままのリスク |
| --- | --- | --- | --- |
| ライセンスの決定（`LICENSE` 作成・`Cargo.toml` の `license = "MIT"` 仮置きの確定） | `LICENSE` 不在の確認と未決定の明示まで。`Cargo.toml` は変更しない | 配布方針の決定（#1 以降） | マニフェストの仮置き値と実ライセンスが不整合のまま公開が進む |
| 最終的なクレート分割・公開 API（`ruviform-audio` / `ruviform-video` 等） | README には構想としてのみ記載 | #1 の詳細設計 | 構想記述が実装と乖離する |
| `wgpu`・WebView・ネイティブ UI・プラグイン連携の採否 | 「検討中」として記載 | #1 の詳細設計 | 同上 |
| 対応プラットフォームの確定・macOS/Linux での手順検証 | 未確定・未検証を明示（#2 の実機検証は Windows 11 x64 のみ） | #1 の対象プラットフォーム設計 | 未検証 OS の差異がドキュメント上見えなくなる |
| RT/オフライン API 契約・`unsafe`/SIMD 規約の確定 | AGENTS には設計意図・暫定原則のみ記載 | #1 の詳細設計 | 暫定原則が未検証の保証として固定化する |
| GitHub Actions の実ディスパッチ検証 | #2 時点では未実施（静的検証のみ pass）。README/AGENTS には検証済み範囲のみ記載 | default branch マージ後の初回ディスパッチ | CI 手順の記述と実挙動が乖離する |
| README/AGENTS のアーキテクチャ記述の追従 | 本 spec では現時点の意図と未決定事項を記載 | #1 完了後の必要時更新 | #1 決定後も古い構想記述が残る |

## Sources and Relations

- 一次ソース: GitHub Issue [#3 [docs] README・AGENTS・APM等のテンプレート由来部分をRuviform向けに置き換える](https://github.com/scarecrow173/ruviform/issues/3)（`relations.source` / `relations.derives-from`）。目的・着手順序・変更内容・受け入れ条件・スコープ外はすべてこの Issue を正本とする。
- 前提（`relations.depends-on`）: spec `SPEC-034IcwhwIFSNUNKtX7KK0Q`（[establish-rust-dev-foundation.md](./establish-rust-dev-foundation.md)、Issue #2、approved・実装済み）。その成果が本 spec で記述する実測済み手順・検証タスクの基盤となる。検証済みコマンド・前提条件・未検証 OS の一次記録は IR [`verify-reproducible-setup-and-work-memo.md`](../impl/ir/verify-reproducible-setup-and-work-memo.md) の「作業メモ（#3 転記用）」節を正本とする（`relations.references`）。
- 関連 Issue（外部、`relations.references`）: [#2](https://github.com/scarecrow173/ruviform/issues/2)（開発環境、前提）、[#1](https://github.com/scarecrow173/ruviform/issues/1)（詳細設計、後続。#1 の完了は本 spec の前提ではない）。
- 関連ドキュメント・ファイル（`relations.related`）: 変更対象の `README.md`・`AGENTS.md`・`apm.yml`・`mise.toml`、レビュー対象の `.pre-commit-config.yaml`・`.gitignore`・`.gitattributes`、ライセンス表記の整合確認対象 `Cargo.toml`、AGENTS へ反映する暫定開発規約の出典 [`dev-foundation.md`](../designs/dev-foundation.md)、#3 の責務境界を記録した [`overview.md`](../designs/overview.md)。
- 変更対象（`relations.changes.modified`）: `README.md`・`AGENTS.md`・`apm.yml`・`mise.toml` は改稿・更新を確定。`.pre-commit-config.yaml`・`.gitignore`・`.gitattributes` はレビュー対象として列挙し、レビューで Ruviform に不適切と判明した箇所のみ修正する（変更なしも許容）。
