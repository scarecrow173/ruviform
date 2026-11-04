# 開発環境・CI リファレンス

この文書は [AGENTS.md](../AGENTS.md) の規約に対応する運用リファレンス（規約でなく詳細情報）です。
各項目の正本は対象の基盤ファイル（`mise.toml`・`.github/workflows/*.yml`・`.pre-commit-config.yaml`・`.gitattributes`・`apm.yml`・各 lockfile）であり、本書はそれらを横断した consolidated な説明です。齟齬がある場合は正本ファイルを優先します。

## mise タスク体系

- タスク一覧・実行内容・`description` の正本は `mise.toml`。`mise tasks` で全タスクと説明を一覧できる。
- freshness 付きタスク（`install-ui-*`・`build-ui-*`）は `sources` / `outputs` 宣言が不変なら skip される。`install-ui-*` は `sources` = package-lock + package.json、`outputs` = `node_modules/.package-lock.json`（npm が node_modules に必ず書く隠し lockfile）で lockfile 単位の `npm ci` を 1 回に集約する（Issue #330）。verify 系タスクは `depends` 経由で install/build を再利用し、fresh なら実行・不変なら skip される。
- `install-ui-browser-chromium` には mise freshness を付けない。browser path は `PLAYWRIGHT_BROWSERS_PATH` 依存のため、mise の sources/outputs ではなく Playwright 自身の per-revision 存在確認を authoritative skip とする（Issue #332）。
- `build-ui-*` の `tsc` dist 出力は `rewriteRelativeImportExtensions` で `.ts` → `.js` specifier を書き換える。`build-ui-showcase` は showcase + core + components + viz + binding の src を横断する複合 dist（rootDir `..`）を出力し、`ui/browser-test` はこの dist 宣言へ typecheck する（`depends` で showcase build を pin。Issue #310/#330）。
- `test-rt` は `rt_` プレフィックス命名の RT 契約テストを名前フィルタで選択する（Issue #18・#21・#26 で導入後、media-core・dsp・audio-analysis・plugin 系・math::dsp・audio-device 等の RT-suitable API を持つ各クレートへ順次拡大済み）。0 件一致でも成功するため RT テスト未存在時はスキップ相当。
- `bench` は全量計測入口（計測時間が長いためローカル手動実行向け）、`bench-smoke` は各ベンチを計測なしで 1 回実行するスモークで CI `bench` ジョブが呼ぶ入口。
- `check-ci` は CI verify lane に含まれない単独タスク（Issue #331 で `lint-ci` に統合済み）。clippy-driver 差分の切り分け等、素の rustc にだけ通したいローカルトリアージ向け。
- `test-l1-real-plugin` / `lint-l1-real-plugin` は実規格プラグイン L1 E2E opt-in lane（Issue #114）。fixture cdylib + `host` feature runner bin を要し `#[ignore]` + `#![cfg]` + `required-features` で既定経路（`cargo test --workspace`・既定 `lint`）から分離。CI `l1-real-verify` ジョブが呼ぶ入口。
- `test-ci-impact` は `ci/impact/` の分類契約 + `ci-gate` 判定表の contract test（Issue #333。CI `impact` ジョブが decision 出力前の自己検証として呼ぶ。node toolchain 必須）。
- `*-ui-webview` タスクは Issue #156 の webview feature lane（既定の `check`/`lint`/`test` の feature 集合外）。`rvf://` スキーム/data 面は headless 検証可能、窓付き実動作は手動検証。Linux では `libwebkit2gtk-4.1-dev` + `libgtk-3-dev` が必要（CI `ui-verify` ジョブが導入）。
- `test-ofx-msrv` は MSRV 1.89 toolchain 上での plugin-core / OFX テスト（`mise exec rust@1.89.0` 経由、成果物は `target/msrv-ofx/`）。

## CI ワークフロー構成（`.github/workflows/ci.yml`）

- トリガ: `pull_request`（types: opened/synchronize/reopened/ready_for_review/edited — `edited` は base retarget の再分類に必要）・`push`（`main`）・`workflow_dispatch`。`main` の required checks として 12 job が稼働する。
- 同一 PR の run は concurrency group（`pull_request` では `${{ github.workflow }}` + `refs/pull/<N>/merge`・`cancel-in-progress: true`）で新 run 開始時に cancel される。`push` to main・`workflow_dispatch`・別 PR の run は run ごと一意の group のため cancel されない（Issue #328。`consumer-impact-check.yml` も同じ方針）。
- `verify`（`fmt-check` → `lint-ci` → `test-ci` の step 列）: Issue #331 で `check-ci` は `lint-ci` に統合済み — clippy-driver が rustc 全診断 + clippy lint を発行するため独立 `cargo check` step は同一ユニット集合の二重 frontend 評価だった。`mise run verify-ci` も同一列。
- `l1-real-verify`（`mise run test-l1-real-plugin` + `mise run lint-l1-real-plugin`）: Issue #114 の実規格プラグイン lane。
- `ui-verify`: `check-ui-webview`/`lint-ui-webview`/`test-ui-webview`（Issue #156 webview feature lane）、`check-ui-headless`/`test-ui-headless`（Issue #258 `ui/core` headless 契約 lane）、`check-ui-components`/`test-ui-components`/`build-ui-components`（Issue #259 `ui/components` Web Components lane）、`check-ui-viz`/`test-ui-viz`/`build-ui-viz`（Issue #260 `ui/viz` Reference Viz lane）、`check-ui-binding`/`test-ui-binding`/`build-ui-binding`（Issue #261 `ui/binding` Command/Query binding lane）、`check-ui-guitest`/`test-ui-guitest`（Issue #311 `ui/guitest` 決定的 GUI テスト基盤 lane）、`check-ui-showcase`/`test-ui-showcase`/`build-ui-showcase`（Issue #263 `ui/showcase` ショーケース/検証 fixture lane）、`check-ui-browser-test`/`test-ui-browser`（Issue #310 `ui/browser-test` 実ブラウザ lane）で `test-results/` を artifact upload する。
  - Linux はジョブ内で WebKitGTK 開発パッケージと Playwright browser system deps を導入（browser binary とは別レイヤーで毎 run 実行）。
  - browser binary は `PLAYWRIGHT_BROWSERS_PATH`（workspace 内 `.cache/ms-playwright` に pin）を `actions/cache` で Playwright version・Chromium revision・OS/arch 紐づけ key で復元し、準備 step（`install-ui-browser-chromium`）と launch 検証 step（破損復元時は wipe+reinstall fallback）を挟んで `test-ui-browser` を実行する（Issue #332）。
  - ジョブは検証 lane の前に `mise run setup-ui` を 1 ステップ実行し、各 `ui/*` package-lock の `npm ci` を 1 回に集約する（Issue #330。verify タスクは freshness 付き `install-ui-*` を `depends` で再利用）。
- `verify`・`l1-real-verify`・`ui-verify` は `ubuntu-latest` / `windows-latest` / `macos-latest` の 3 OS matrix（`verify` のディスク計測ステップは ubuntu のみ）。`rt-verify`（`mise run test-rt`）・`icc-verify`（`test-video-proc-icc`/`lint-video-proc-icc`）・`bench`（`mise run bench-smoke`）は `ubuntu-latest` 単独。
- sccache: `verify`・`ui-verify` では `RUSTC_WRAPPER` として GitHub Actions cache backend で先行導入（Issue #329。`rt-verify`・`icc-verify`・`l1-real-verify`・`bench` は効果確認後の段階展開のため未適用）。repo variable `CI_SCCACHE_DISABLE=1` で全 sccache step を skip して即時無効化できる。
- Issue #333（CI-OPT-06）の **Shadow Mode** 2 job:
  - `impact`: 変更ファイル → 期待 lane 集合の分類。`ci/impact/` の stdlib-only classifier が `cargo metadata` 逆依存伝播・rt_ 保持クレート scan・mise task `-p` 引数・ui/* DAG から lane 集合を決定し、decision JSON を artifact + job outputs へ出力。`push`/`workflow_dispatch` は常に Full CI。
  - `ci-gate`: 全 lane `needs` 結果 × impact 期待集合を照合する集約 fail-closed gate。期待 lane の skipped・実行 lane の failure/cancelled・分類不能は全て Fail — skipped job を成功扱いしない。現時点は非 required で、偽陰性なし確認後の ruleset 移行で単一 required context 化する設計。分類契約と移行 runbook は [docs/designs/issue-333-impact-based-ci.md](designs/issue-333-impact-based-ci.md) が正本。
  - 緊急停止は repo variable `CI_IMPACT_DISABLE=1`（expected=ALL に退化）。両 job は non-required で既存 12 contexts に影響しない。
- `.github/workflows/consumer-verify.yml`（Issue #219 Cross-Repo 検証 lane）は `workflow_dispatch` 専用の情報 lane で required checks に含めず、片 Consumer の失敗・未検証が他 Consumer や merge を止めない。両 Consumer lane は scratch checkout 内で ruviform git 依存を候補 rev に正規化（rev pin・branch 追従・rev 未指定を同一ルールで処理）し、単一 SHA lockfile guard 後に consumer 側 mise task（audio-lens: `check-ci`/`al-cli` ci build/`check-cli-headless`/`test-ci`、synvelis: `check`/`test`）を実行する。実行は access mode で分岐する: `CROSS_REPO_DEPLOY_KEY` secret 設定済みなら SSH Deploy Key 経路（private 前提の canonical 経路）、未設定でも対象 repo が public で匿名 HTTPS read 可能なら secret 不使用の anonymous 経路で lane を実行、どちらも不可なら `not run`。全 lane `not run` → workflow 失敗（証拠ゼロを緑にしない）。
- `.github/workflows/consumer-impact-check.yml` が `pull_request`（draft 除外）で PR 本文の Consumer 影響分類欄の記入を機械検査する非 required lane として存在する。

## Consumer 影響分類の運用（Issue #219）

公開 API / feature / behavior / dependency / CI 基盤の変更 PR は本文の Consumer 影響欄で Synvelis・AudioLens を独立して `impacted` / `no impact` / `unknown` に分類する。`unknown` 残置は互換性未確認であり、merge 前にレビュアーが分類欄の記入と `unknown` 解消先リンクを確認する（required check 化していないため human gate）。`.github/workflows/consumer-impact-check.yml` は記入有無と `unknown` 解消先の記述有無のみを検査する補助 lane で、分類内容の妥当性はレビュアーが確認する。PR 本文の記入形式は `.github/PULL_REQUEST_TEMPLATE.md` が正本。

## lockfile・環境内部の詳細

- `mise.lock`: `mise.toml` の `[tools]` 変更・バージョン更新時に `mise install` → `mise lock` で再生成。現行ツールは github / aqua 系バックエンドのため `.mise/locks/` sidecar は生成されない。PyPI・npm 解決の pipx / npm 系ツールを追加して sidecar が生成された場合は `.gitignore` の `!.mise/locks/` 例外によりそのままコミット対象になる。
- `apm.lock.yaml`: `apm.yml` の dependencies / targets / MCP 変更時に `apm install`（lock 自動更新）→ `apm lock` で整合確認。厳格検証は `mise exec -- apm install --frozen`（apm.yml と apm.lock.yaml が非同期なら失敗）。
- `Cargo.lock`: ワークスペースの `[dependencies]` 変更時に `cargo build` / `cargo generate-lockfile` で自動更新。
- `pipx:serena-agent` は `lazy = true` のため `mise install` の対象外。エージェント用途のオプションツールであり、初回の `mise exec` 経由利用時に自動インストールされる（明示的に復元する場合は `mise install --include-lazy`）。
- global mise config でリポジトリ管理外のツールを設定している環境では、`mise install` はそれらも併せて解決・インストールする。
- Git LFS: バージョンは `mise.toml` で管理し、対象は `.gitattributes` で定義。`mise run setup` が `git lfs install --local` でリポジトリローカルの Git LFS フィルターとフックを有効化する。初回 clone で LFS オブジェクトが未取得なら、セットアップ後に `mise exec -- git lfs pull` を実行する。

## MSRV clippy の既知ノイズ除外（Issue #223）

MSRV 1.89 上で clippy を検査する場合のコマンド:

```sh
cargo +1.89.0 clippy --workspace --all-targets -- -D warnings -A clippy::float_cmp -A clippy::similar_names
```

1.89 検出器のみが発火する `float_cmp` / `similar_names` は toolchain 版差の既知ノイズであり `-A` フラグで除外する。これら 2 lint は pin toolchain でも pedantic に残るため `workspace.lints` で `allow` しない（pin gate を弱めない）。MSRV でこれら以外の失敗が残る場合は除外リストへ機械追加せず、個別に判定する。

## 附録: 開発・設計原則（詳細版）

以下は AGENTS.md「開発・設計原則」節の詳細版（圧縮前の原文）です。AGENTS.md 側の digest は各原則の要点のみを保持するため、判断に迷う場合はこちらを参照してください。

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
