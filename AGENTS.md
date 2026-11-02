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
- `mise.toml` は uv / prek / git-lfs / pipx 系の共通基盤ツールに加え、このリポジトリで pin した Rust ツールチェーン（`rust = "1.99.0"`）と actionlint、UI PoC ビルド用の Node.js（`node = "26.12.1"`、Issue #83 のセキュリティ修正版）を管理する。`fmt` / `check` / `lint` / `test` / `build` 等の検証タスクも同ファイルが正本（対応表は `## Rust ワークスペースと暫定開発規約` 節を参照）。
- 初回セットアップでは、miseをインストールした後に以下を実行する（Windows 11 x64・Git Bash で検証済み。macOS / Linux は未検証）。

```sh
# ツール復元（mise.lock に記録済みの固定バージョンを復元）
# ただし pipx:serena-agent は lazy = true のため対象外。エージェント用途の
# オプションツールであり、初回の mise exec 経由利用時に自動インストールされる
# （明示的に復元する場合は mise install --include-lazy）
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

- ルート `Cargo.toml` がワークスペースの正本。`[workspace]`（`resolver = "2"`）、`[workspace.package]`（`version = "0.1.0"` は仮置き。`edition = "2024"` は Issue #34 で確定。`rust-version = "1.89"` は Issue #34 で 1.85 確定後、Issue #52 / docs/adr/workspace-msrv-1-85-1-89-fearless-simd.md で 1.89 に引上げ（fearless_simd 本番昇格前提）。`license` の現行値は仮置き。`publish = false` は未決定クレートを公開しない licensing ADR の原則であり、メンバーは `publish.workspace = true` で継承する）、`[workspace.lints]`（共通 lint の正本）、`[profile.dev]` / `[profile.ci]` を定義する。
- メンバー構成（層別の計29クレート一覧と各クレート README へのリンク）は [crates/README.md](crates/README.md) を索引とする。各クレートの責務・許可依存層・RT 制約・実装 Issue の詳細は `crates/<name>/README.md` と crate-level doc に転記しており、そちらを参照。層構造と `path` 依存方向・クレート DAG の正本は [docs/designs/ruviform-architecture-design-baseline.md](docs/designs/ruviform-architecture-design-baseline.md)（Issue #1 由来・approved）であり、`members` は実装 Phase に応じて段階追加される。残りの `path` 依存は API 確定に伴い追加する。上記以外の公開 API は引き続き未確定であり、実装 Issue で確定する。
- Adapter クレートとして `crates/ruviform-audio-device` を Issue #23（cpal 条件付き採用・[docs/adr/select-audio-device-backend.md](docs/adr/select-audio-device-backend.md)・accepted）で新設済み。adapter 層はドメイン基盤の外側（engine と同じ利用側層）で、foundations/Primitives への下向き依存のみを持つ。`ruviform-audio-device` は cpal を内部依存に閉じ込め、`ruviform_audio_core::device` のポート trait（`DeviceEnumerator`/`Device`/`OutputStream`）を `Box<dyn>` 経由で実装する。cpal の型・エラー・feature 名を公開 API に露出せず、コアクレートからの強制依存・adapter→adapter 依存は作らない。手動検証 example（`list_devices`・`play_sine`）は実機依存で CI 対象外。

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
| `test-rt` | `cargo test --workspace rt_`（`rt_` プレフィックス命名の RT テストを名前フィルタで選択。0 件一致でも成功するため RT テスト未存在時はスキップ相当） | RT 契約テスト（Issue #18・#21・#26 で導入後、media-core・dsp・audio-analysis・plugin 系・math::dsp・audio-device 等の RT-suitable API を持つ各クレートへ順次拡大済み） |
| `bench` | `cargo bench --workspace` | ベンチマーク全量計測入口（現在は `ruviform-math` の SIMD PoC ベンチと `ruviform-rt` の RT 観測 PoC ベンチが対象。計測時間が長いためローカル手動実行向け） |
| `bench-smoke` | `cargo bench -p ruviform-math -p ruviform-rt -- --test` | ベンチスモーク（各ベンチを計測なしで 1 回実行。CI `bench` ジョブが呼ぶ入口） |
| `check-ci` / `lint-ci` / `test-ci` / `build-ci` | 上記各コマンドを `--profile ci`・`CARGO_INCREMENTAL=0` で実行 | CI 用途限定 |
| `verify-ci` | `fmt-check` → `check-ci` → `lint-ci` → `test-ci` を順次実行 | CI 同等の検証列 |
| `test-l1-real-plugin` | `cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host` → `cargo test -p ... --features host --lib --bins` → `cargo test -p ... --features host --test l1_real_adapter_e2e -- --ignored` | 実規格プラグイン L1 E2E opt-in lane（Issue #114。fixture cdylib + `host` feature runner bin を要し `#[ignore]` + `#![cfg]` + `required-features` で既定経路から分離。CI `l1-real-verify` ジョブが呼ぶ入口） |
| `lint-l1-real-plugin` | `cargo clippy -p ruviform-plugin-clap -p ruviform-plugin-vst3 --all-targets --features host -- -D warnings` | `host` feature 配下の bridge/runner/E2E ターゲット lint（既定 `lint` の feature 集合外） |
| `check-ui-webview` | `cargo check -p ruviform-ui-webview --features webview --all-targets` | UI webview adapter の feature 有効型チェック（Issue #156。既定 `check` は headless のまま） |
| `lint-ui-webview` | `cargo clippy -p ruviform-ui-webview --all-targets --features webview -- -D warnings` | `webview` feature 配下の host/example ターゲット lint（既定 `lint` の feature 集合外） |
| `test-ui-webview` | `cargo test -p ruviform-ui-webview --features webview` | `webview` feature 配下テスト（`rvf://` スキーム/data 面は headless 検証可能、窓付き実動作は手動検証。CI `ui-verify` ジョブが呼ぶ入口） |
| `check-ui-headless` | `npm --prefix ui/core ci --no-audit --no-fund` → `npm --prefix ui/core run typecheck` | headless UI 契約パッケージ `ui/core` の lockfile 固定依存インストール + `tsc --noEmit` 型検査（Issue #258。DOM lib 非含有の vanilla TS） |
| `test-ui-headless` | `npm --prefix ui/core ci --no-audit --no-fund` → `npm --prefix ui/core test` | `ui/core` の `node --test` headless テスト（Issue #258。UI framework / renderer 非依存） |

- `*-ci` タスクと `verify-ci` は CI 用途限定。成果物が `target/ci/` に `target/debug/` と並存して容量を消費するため常用しない。
- `.github/workflows/ci.yml` は `pull_request`・`push`（`main`）・`workflow_dispatch` の 3 trigger で実行され、`main` の required checks として 12 job が稼働する。`verify`（`mise run verify-ci`）と `l1-real-verify`（`mise run test-l1-real-plugin` + `mise run lint-l1-real-plugin`・Issue #114 の実規格プラグイン lane）と `ui-verify`（`check-ui-webview`/`lint-ui-webview`/`test-ui-webview`・Issue #156 の webview feature lane、および `check-ui-headless`/`test-ui-headless`・Issue #258 の `ui/core` headless 契約 lane。Linux はジョブ内で WebKitGTK 開発パッケージを導入）は `ubuntu-latest` / `windows-latest` / `macos-latest` の 3 OS matrix（`verify` のディスク計測ステップは ubuntu のみ）、`rt-verify`（`mise run test-rt`）・`icc-verify`（`test-video-proc-icc`/`lint-video-proc-icc`）・`bench`（`mise run bench-smoke`）は `ubuntu-latest` 単独。別 workflow の `.github/workflows/consumer-verify.yml`（Issue #219 Cross-Repo 検証 lane）は `workflow_dispatch` 専用の情報 lane で required checks に含めず、片 Consumer の失敗・未検証が他 Consumer や merge を止めない。
- Consumer 影響分類の運用（Issue #219）: 公開 API / feature / behavior / dependency / CI 基盤の変更 PR は本文の Consumer 影響欄で Synvelis・AudioLens を独立して `impacted` / `no impact` / `unknown` に分類する。`unknown` 残置は互換性未確認であり、merge 前にレビュアーが分類欄の記入と `unknown` 解消先リンクを確認する（required check 化していないため human gate）。

### 暫定規約

- `unsafe` は原則禁止。必要な場合のみ SAFETY コメント付きの局所許可（`#[expect(unsafe_code)]`、未対応なら `#[allow(unsafe_code)]`）で例外的に認める。`workspace.lints.rust.unsafe_code = "warn"` は `lint` / `lint-ci` の `-D warnings` ゲートで実質 deny として機能し、局所許可のない `unsafe` は検証を通過しない。
- feature: default feature は最小限とする。新規 feature・optional 依存は追加理由を PR に記録する。
- 依存追加: メンテナンス状況・ライセンス・依存木サイズを確認し理由を記録する。`Cargo.lock` は追跡を継続する。
- lint: `workspace.lints` が正本。CI は `clippy -- -D warnings` を必須ゲートとする。rustfmt は標準設定（`rustfmt.toml` は必要時に追加）。
- MSRV clippy 検証（Issue #223）: lint gate の対象は pin toolchain のみで、MSRV 1.89 上の clippy は情報 lane として扱う。1.89 検出器のみが発火する `float_cmp` / `similar_names` は toolchain 版差の既知ノイズであり、MSRV 上で検査する場合は `-A` フラグで除外する: `cargo +1.89.0 clippy --workspace --all-targets -- -D warnings -A clippy::float_cmp -A clippy::similar_names`。これら 2 lint は pin toolchain でも pedantic に残るため `workspace.lints` で `allow` しない（pin gate を弱めない）。MSRV でこれら以外の失敗が残る場合は除外リストへ機械追加せず、個別に判定する。
- 公開 enum: クレート外から到達可能な全公開 enum に enum レベルの `#[non_exhaustive]` を付与する（クレート内部限定 enum は対象外）。`#![allow(clippy::exhaustive_enums)]` は置かず、`workspace.lints.clippy` で restriction lint `exhaustive_enums = "warn"` を有効化し `-D warnings` ゲートで強制する。正本: [docs/adr/public-enum-non-exhaustive-policy.md](docs/adr/public-enum-non-exhaustive-policy.md)。
- doc test: 公開 API の例は doctest に書き、`cargo test` で検証する。
- RT 検証レーン（Issue #22）: `rt_` プレフィックスの RT 契約テストは `assert_no_alloc` 測定窓内で処理経路を実行し、窓内の alloc/dealloc はテストバイナリを abort させて CI で検出する。fixture・warm-up の確保は窓外に置く。ruviform-rt 既存の `alloc_counter`（API セマンティクスの実装機能）は変更せず併存し、新規の検証用途には `assert_no_alloc` を使う。CLAP アダプタは CLAP `thread-check` 拡張を `plugin.init` で一度照会し、ホストが非オーディオスレッドと応答した `process` を defined rejection する（拡張未提供・応答不定は従来経路）。機械検出の対象は alloc/dealloc と当該スレッド契約のみ — ブロッキング操作・ファイル/ネットワーク I/O・無制限待機/リトライ・実行時間無界・ロック保持時間等の静的検証不能な制約はドキュメント契約 + コードレビュー観点であり、CI 成功は RT 安全性の完全証明ではない。

### アーキテクチャ確定契約（設計ベースライン）

以下は Issue [#1](https://github.com/scarecrow173/ruviform/issues/1) 由来の設計ベースライン（[docs/designs/ruviform-architecture-design-baseline.md](docs/designs/ruviform-architecture-design-baseline.md)、approved）で確定した契約の要約。詳細な定義・根拠は同書の各節が正本であり、本節と齟齬がある場合は同書を優先する。

- RT/オフライン分離: リアルタイム（RT）経路と非 RT を API・ライフサイクルで分離する。処理契約は `Processor`（`prepare`/`process` 分離）と `RtContext` capability が定義する。RT クリティカルパスでは alloc/dealloc を禁止し、ブロッキング操作・I/O・無制限の待機/リトライ・非有界処理を制限する。「RT-safe」等の保証範囲は API 単位で定義・明示し、未確定の API を RT-safe と断定しない。検証可能な違反は CI で検出するが、CI 成功は完全証明ではない。OS レベルのハードリアルタイム保証は提供しない。正本: 同書「RT／非 RT の分離方式」「RT 契約（API 単位の保証定義）」節。
- SIMD: スカラー参照実装を必須とし、カーネルごとに「ビット完全一致」または「ulp 上限内」の等価性契約を定義してスカラーを参照として検証する。stable Rust のみを使用し、`unsafe` は暫定規約どおり SAFETY コメント付きの局所許可のみで例外的に認める。SIMD 二層構造（ランタイム分岐＋フォールバック、dispatch 層の `ruviform-math` 集約を含む構造仮説）とバックエンド選定の採否は要 PoC。正本: 同書「DSP／SIMD 設計」採否。
- クレート依存方向: 依存は上位 → 下位のみとし、循環依存は禁止。基礎クレートから GUI・オーディオデバイス・GPU への強制依存を作らない。詳細な層内順序・adapter ルール・契約型の所有は同書「レイヤー構造と依存方向ルール」節が正本。

### 共通基盤・Observability・Diagnostics・Debug の設計方針（必要時参照）

- 共通基盤の責務境界、段階的なモジュール→クレート分割、Memory / Collections / Sync / Identity / Telemetry、Logging / Profiling / Metrics / Diagnostics / Debug、OS / IPC との境界は [docs/designs/shared-foundations-and-modularization-policy.md](docs/designs/shared-foundations-and-modularization-policy.md)（Issue #101）を設計方針の参照先とする。
- これらの機能や RT 計測経路、クレート・依存 DAG・公開 API の所有者・feature 境界を新設・変更・レビューする際は、必要に応じて同書の該当節を参照する。無関係な変更に同書の全読は要求しない。
- `ruviform-foundation` / `ruviform-observability` / `ruviform-debug` / `ruviform-platform` 等は現時点では構成候補であり、クレート・API が実装されたことを意味しない。具体的な実装・既存 API の移動は別途設計・検証する。既存 accepted ADR（特に RT 契約・RT プリミティブ方式）と矛盾する変更は、必要な ADR 改訂を経て行う。

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
