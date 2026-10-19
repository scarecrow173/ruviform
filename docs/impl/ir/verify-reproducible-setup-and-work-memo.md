---
id: "IMPL-034IeuSyH91cISMHS4iUUz"
type: "impl"
status: "completed"
title: "Verify reproducible setup and work memo"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/2"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/verify-reproducible-setup-and-work-memo.md"
      - type: "file"
        path: ".mise/locks/pipx-apm-cli/0.33.0/pyproject.toml"
      - type: "file"
        path: ".mise/locks/pipx-apm-cli/0.33.0/uv.lock"
    modified:
      - type: "file"
        path: "docs/tasks/verify-reproducible-setup-and-work-memo.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "mise.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/verify-reproducible-setup-and-work-memo.md"
  implemented-by: []
  depends-on:
    - "pin-rust-toolchain-and-lockfile.md"
    - "create-minimal-workspace-and-smoke-test.md"
    - "define-mise-dev-and-ci-tasks.md"
    - "add-manual-ci-workflow.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/dev-foundation-bootstrap.md"
    - "../../designs/dev-foundation.md"
    - "../../specs/establish-rust-dev-foundation.md"
    - "../../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/lockfile-reproducible-setup.md"
    - "../../test-specs/mise-verification-tasks.md"
    - "../../test-specs/ci-profile-build-behavior.md"
    - "../../test-specs/manual-ci-workflow.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Verify reproducible setup and work memo

## Summary

task `TASK-034IdQxdPQUxXweHBN1ZxT`（plan `PLAN-034IdrispOUms1c32192B6` スライス E）として、スライス A〜D の成果物が「新規 clone から固定 lockfile で再現できる」ことを実機で検証し、結果と #3 転記用作業メモを本 IR に記録した。検証・記録が主体のタスクだが、検証で判明した `mise install --locked` の pipx 欠陥については IR 記録後の後続対応として `mise.toml` / `mise.lock` / `.mise/locks/` を修正し解消済み（`apm.yml` / `Cargo.toml` / `.github/workflows/ci.yml` は未変更。詳細は下記 caveat 節の解決記録を参照）。

主要結果:

- `mise exec -- apm install --frozen` は apm-cli 0.33.0 で**対応済み**。exit 0 を確認（design が想定したフォールバック不要）。
- `mise install --locked` の pipx 2 件の欠陥は**検証時に再現したが、後続対応で解消済み**。`mise.toml` で `pipx:apm-cli` の `uvx_args` を除去（0.33.0 は PyPI final release で `--prerelease allow` 不要）、`pipx:serena-agent` を GitHub-shorthand 指定の `pipx:oraios/serena` に変更して lock 可能な spec とし、`uv` を 0.12 系に更新。`.mise/locks/pipx-apm-cli/0.33.0/` の dep-graph 生成物をコミットし `mise.lock` から参照される形とした。再検証で repo の全 7 ツール（`uv`/`prek`/`git-lfs`/`pipx:apm-cli`/`pipx:oraios/serena`/`rust`/`actionlint`）が `mise.lock` から解決することを確認（`mise exec -- apm --version` → 0.33.0、`mise exec -- serena --version` → Serena 1.7.0）。残る exit 1 は本機の global mise config が要求する repo lockfile 外ツールのみで、global config を持たないクリーンな CI 環境では発生しない環境 caveat。
- 上記を除く V-1〜V-11 は全て pass。クリーンな `git worktree`（HEAD の完全な追跡ファイルのみ）で `apm install --frozen` → `mise run setup` 相当 → `mise run verify-ci` → `mise run test` が成功し、`git status --ignored` が生成物のみを示しソース差分ゼロであることを確認した。

## Context

- 実施タスク: `docs/tasks/verify-reproducible-setup-and-work-memo.md`（E-1〜E-6）
- 上流: `docs/plans/dev-foundation-bootstrap.md`（Phase E / V-1・V-2・V-9・V-11）、`docs/designs/dev-foundation.md`（暫定開発規約・Verification Notes・LFS 方針）、`docs/specs/establish-rust-dev-foundation.md`（Requirements §1・§6、Acceptance Criteria）、test-spec 4 件
- 前提制約: 検証と記録が主体。既存の正本ファイルは変更しない。実施 OS は Windows x64 のみ。未検証 OS の明記が必須。
- スライス A IR の予告 caveat: (1) `mise install --locked` が global mise config 由来ツールで exit 1、(2) `mise.lock` の platform/dep-graph 欠落による pipx 復元可否は Phase E で実機確認、の 2 件を本タスクで検証した。

## Implementation

### Changes

- `docs/impl/ir/verify-reproducible-setup-and-work-memo.md` — 本 IR（検証結果と作業メモの記録）
- `docs/tasks/verify-reproducible-setup-and-work-memo.md` — `status` と検証結果メモの更新

タスク本体は検証・記録のみで正本ファイルを変更しない契約だったが、検証で判明した `mise install --locked` の pipx 欠陥に対し、IR 記録後の後続対応として以下が適用された（コミット `f4d7ca3` に本 IR と同梱）:

- `mise.toml` — `"pipx:apm-cli" = { version = "0.33.0", uvx_args = "--prerelease allow" }` → `"pipx:apm-cli" = "0.33.0"`（uvx_args 除去）、`"pipx:serena-agent" = "1.7.0"` → `"pipx:oraios/serena" = "1.7.0"`（GitHub shorthand 指定）、`uv = "0.11"` → `uv = "0.12"`
- `mise.lock` — `pipx:apm-cli` に dep-graph 参照（`.mise/locks/` への `path`/`digest`）を追加、`pipx:oraios/serena` を `v1.7.0` として記録、`uv` を `0.12.24` に確定
- `.mise/locks/pipx-apm-cli/0.33.0/{pyproject.toml,uv.lock}` — `mise lock` が生成した pipx dep-graph 生成物（新規・追跡対象）

### Decisions During Implementation

- **`apm install --frozen` の対応確認**: `apm install --help` に `--frozen`（「apm.lock.yaml が apm.yml と非同期なら拒否。CI-safe」）が存在することを確認し、そのまま実行。main リポジトリ（exit 0、9 deps）とクリーン worktree（exit 0、31 deps + 2 MCP servers）の双方で成功。同等手順の定義は不要と判断。
- **クリーン環境のシミュレーション方法**: `MISE_CONFIG_DIR` と `MISE_DATA_DIR` を空ディレクトリに向け、global mise config（`~/.config/mise/config.toml` の node/python/ripgrep 等）と既インストール済みツールを切り離して新規マシン相当を再現した。この手法で `mise install --locked` / `mise install` / `apm install --frozen` を実測した。
- **`mise run` 系検証は worktree + 通常環境へ切替**: 分離環境では、`mise` が PATH 上の shim（`~\AppData\Local\mise\shims\node.exe`）経由で `node` を解決しようとして `mise x -- node` → shim → `mise x -- node` の無限リスポーンが発生（マシン上で ~2800 プロセスまで増殖、即時 kill で鎮圧済み）。分離 data dir に `node` が未インストールかつ global config の `node@26` が見えない条件下での shim 自己解決が原因。検証手法側の artifact でありリポジトリの欠陥ではないため、`mise run verify-ci` 等の `mise run` 系は「クリーンなソースツリー + 通常の mise 環境」で実施する判断に切り替えた（新規 clone の再現性評価として同等の情報が得られる）。
- **作業メモの置き場**: plan の File Map が指定するどおり、#3 転記材料は本 IR に集約した（README/AGENTS 本文への反映は #3 の責務）。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64、Git Bash、mise 2026.10.1（windows-x64）、apm-cli 0.33.0、git 2.x（Git for Windows）、rust 1.99.0（mise 管理）。

### 検証マトリクス（plan V-1〜V-11）

| V | 対象 AC | 結果 | 実測 |
| --- | --- | --- | --- |
| V-1 | lockfile 追跡・非破壊 | **pass** | `git ls-files mise.lock apm.lock.yaml` → 両者出力。`git diff --stat apm.yml` → 差分なし。`mise run setup` → exit 0（apm install 9 deps・prek install・git lfs install --local） |
| V-2 | 再現インストール | **pass（pipx 欠陥は後続対応で解消）** | 検証時は `mise install --locked` → exit 1（下記 caveat）。`mise.toml`/`mise.lock`/`.mise/locks/` 修正後の再検証で repo 全 7 ツールが lockfile から解決（残る exit 1 は global config 由来のみ）。`mise exec -- apm install --frozen` → exit 0（対応済み・同等手順不要） |
| V-3 | workspace/プロファイル | **pass** | ルート `Cargo.toml` の `[workspace]`/`[workspace.package]`/`[workspace.lints.*]`/`[profile.*]` が design verbatim。メンバーは `[lints] workspace = true` のみで profile 重複なし。worktree で `cargo build`（dev）→ `target/debug/`、verify-ci → `target/ci/` が分離生成（`target/` 計 8.3M、`target/ci` 3.8M） |
| V-4 | Cargo.lock 追跡・target 除外 | **pass** | `git ls-files Cargo.lock` → 出力あり。`git check-ignore target/` → `target/` |
| V-5 | mise タスク共通化 | **pass** | worktree で `mise run verify-ci` → exit 0（`fmt-check`→`check-ci`→`lint-ci`→`test-ci` の順、`--profile ci`・`CARGO_INCREMENTAL=0`）。worktree で `mise run test`（dev）→ exit 0。main リポで `fmt-check`/`check`/`lint`/`build` → 全て exit 0 |
| V-6 | ローカル/CI 同一コマンド | **pass** | `.github/workflows/ci.yml` の検証ステップは `mise run fmt-check`/`check-ci`/`lint-ci`/`test-ci` のみ。生の cargo コマンド記述なし（目視） |
| V-7 | workflow_dispatch のみ・最小権限 | **pass** | `on: workflow_dispatch` のみ（push/PR/schedule なし・inputs なし）、`permissions: contents: read` のみ。`mise exec -- actionlint .github/workflows/ci.yml` → exit 0 |
| V-8 | CI ディスク計測・増分無効 | **static pass / dispatch 未検証** | `CARGO_INCREMENTAL: "0"`、前後 `df -h`、`du -sh target`・`target/ci/deps`・`target/ci/build` 計測ステップあり。キャッシュは `~/.cargo` registry/git index を `Cargo.lock` ハッシュでキー付けのみ（`target/` キャッシュなし）。`workflow_dispatch` の実実行は default branch マージ後のみ可能なため未実施 |
| V-9 | 追跡/非追跡の分離 | **pass** | `git check-ignore target/ apm_modules/ .agents/ .env .env.local .serena/ .sdp/ .mise-data/` → 全て出力。`git status --ignored` の untracked/ignored は意図した生成物のみ（下記）。`git ls-files` に生成物・秘密情報なし |
| V-10 | smoke test | **pass** | `mise run test` → unit test 1 件 + doctest 1 件、exit 0（worktree・dev profile） |
| V-11 | #3 転記可能・未検証 OS 明記 | **pass** | 本 IR の「作業メモ」節にセットアップ手順・検証コマンド・前提条件・lockfile 再生成条件・実施 OS/未検証 OS・暫定開発規約・LFS 方針を記録 |

### task `## Verification` 項目の実測

| 項目 | 結果 | 詳細 |
| --- | --- | --- |
| `mise install --locked` exits 0 | **pass（後続対応で解消）** | 検証時は pipx 2 件で exit 1（下記）。`uvx_args` 除去・`pipx:oraios/serena` への spec 変更・dep-graph コミット後の再検証で repo 全 7 ツールが解決。残る exit 1 は global config ツールのみ（環境 caveat、クリーン CI では非発生） |
| `mise exec -- cargo --version && cargo fmt --version && cargo clippy --version` | pass | `cargo 1.99.0` / `rustfmt 1.10.0-stable` / `clippy 0.1.99`、全て exit 0 |
| `mise exec -- apm install --frozen` exits 0 | pass | exit 0（main: 9 deps / worktree: 31 deps + 2 MCP）。`git diff --exit-code -- apm.lock.yaml` → exit 0（冪等） |
| `git ls-files Cargo.lock mise.lock apm.lock.yaml` | pass | 3 ファイル全て出力 |
| `git check-ignore target/ apm_modules/ .agents/` | pass | 各パスを出力（`.env*`・`.serena/`・`.sdp/` 等も確認済み） |
| `git status --ignored` で意図しない追跡/未除外なし | pass | ignored は `.agents/` `.claude/` `.codex/` `.cursor/` `.github/agents/` `.github/hooks/` `.github/mcp.json` `.mcp.json` `.sdp/` `.serena/` `.windsurf/` `apm_modules/` `target/` のみ。追跡済み生成物なし |
| `ls docs/impl/ir/*.md` + `grep -rE '未検証|mise install --locked|lockfile'` | pass | 本 IR を含む 5 件。grep は本 IR の記録に一致 |
| Manual: IR/PR 本文の記録項目 | pass（IR 側） | 本 IR に全項目記録。実施 PR 本文への転記は PR 作成時の責務 |

### 判明した caveat（`mise install --locked`）とその解決

検証時点では 2 経路とも exit 1。原因は 2 層あり、**本質的なものは 2 つ目**（後続対応で解消済み）:

1. **global config 混入（環境固有・スライス A 既報）**: このマシンの `C:\Users\akiyoshi\.config\mise\config.toml` が `ripgrep`/`ffmpeg`/`python@3.13`/`node@26`/`pnpm`/`npm:@openai/codex`/`npm:ccusage` を要求するが repo の `mise.lock` に無い → 解決失敗。`MISE_CONFIG_DIR` を空にして切り離すとこの層は消える。**この層は環境 caveat として残存**（後述の再検証結果を参照）。
2. **pipx ツールの lockfile 依存グラフ欠落（repo 固有・本質）**: 検証時点では、クリーン環境（`MISE_CONFIG_DIR`/`MISE_DATA_DIR` を空化、clean worktree）でも `mise install --locked` は exit 1。`rust@1.99.0`（clippy・rustfmt 同梱を rustup が報告）・`actionlint@1.7.12`・`prek@0.5.5`・`git-lfs@3.8.0`・`uv@0.11.33` は lockfile から正しく復元されるが、以下 2 件が復元できなかった:
   - `pipx:apm-cli@0.33.0` → `dependency locking does not support uvx_args or pipx_args`（当時の `mise.toml` の `uvx_args = "--prerelease allow"` が依存ロック非対応）
   - `pipx:serena-agent@1.7.0` → `has no uv dependency graph; run mise lock`（`mise.lock` に dep graph が無い。スライス A で `mise lock` が apm-cli のエラーで中断したことと整合）

   影響（検証時点）: pipx ツールはインストール自体をスキップされるため、真の新規環境では `mise install --locked` 後に `apm`/`serena` が存在せず、`mise run setup` が `apm` 不在で失敗する。**同じステップを持つ `.github/workflows/ci.yml` も同様に失敗する見込み**だった（backend 制限は OS 非依存）。
   回避手順（検証時点で実測済み）: `mise install`（`--locked` なし）はクリーン環境で exit 0・全 7 ツール導入成功。

#### 解決（IR 記録後の後続対応・実機で再検証済み）

検証時に列挙した恒久対応案の (a)（`uvx_args` 除去 + `mise lock` 再生成）を採用し、以下を適用して `mise.lock` と dep graph を再生成した:

- `mise.toml`: `"pipx:apm-cli" = { version = "0.33.0", uvx_args = "--prerelease allow" }` → `"pipx:apm-cli" = "0.33.0"`。`apm-cli` 0.33.0 は PyPI の final release で `--prerelease allow` が不要なため `uvx_args` を除去し、dependency locking を成立させた。
- `mise.toml`: `"pipx:serena-agent" = "1.7.0"` → `"pipx:oraios/serena" = "1.7.0"`。GitHub shorthand（`<org>/<repo>`）指定へ変更し、`mise.lock` で `v1.7.0` として解決される lock 可能な spec とした。`serena` バイナリは `mise exec -- serena --version` → `Serena 1.7.0` と `mise which serena` で動作確認済み。
- `mise.toml`: `uv = "0.11"` → `uv = "0.12"`（`mise.lock` 上は `0.12.24` に確定）。pipx dep-graph 生成の基盤となる uv backend を最新安定系に更新。
- dep-graph 生成物のコミット: `.mise/locks/pipx-apm-cli/0.33.0/{pyproject.toml,uv.lock}` を追加。`mise.lock` の `pipx:apm-cli` エントリは `uv = { path = ".mise/locks/pipx-apm-cli/0.33.0", digest = "sha256:…" }` でこの dep graph を参照する。

再検証結果（実機）:

- `mise install --locked` — repo の全 7 ツール（`uv`/`prek`/`git-lfs`/`pipx:apm-cli`/`pipx:oraios/serena`/`rust`/`actionlint`）が `mise.lock` から解決。
- `mise exec -- apm --version` → `0.33.0`、`mise exec -- serena --version` → `Serena 1.7.0`、`mise which serena` → 解決成功。
- `.github/workflows/ci.yml` の "Restore tools from mise.lock" ステップは同じ lockfile を使うため、global config を持たない CI 環境では全件解決する見込み（実ディスパッチは未実施）。

残存 caveat（環境固有）: 本機では `mise install --locked` が依然 exit 1 となり得るが、失敗するのは global mise config が要求する repo lockfile 外ツール（`node`/`python`/`ffmpeg`/`ripgrep`/`pnpm`/`npm:@openai/codex`/`npm:ccusage`）のみ。repo 管理の 7 ツールは全て解決しており、global config を持たない環境（CI・新規マシン）では影響しない。

### 補助検証（worktree クリーンチェック）

`git worktree add` で HEAD（`0e512c8`）の完全クリーンなソースツリーを作成し実施:

- `mise exec -- apm install --frozen` → exit 0（31 APM deps + 2 MCP servers、`apm.lock.yaml` と整合）
- `mise run setup` 相当の 3 コマンド個別実施 → `apm install` / `prek install` / `git lfs install --local` 全て exit 0（main リポでの `mise run setup` 一括実行も exit 0 を確認済み）
- `mise run verify-ci` → exit 0（`--profile ci` で check/clippy/test 全て緑、target/ci 新規生成）
- `mise run test`（dev）→ exit 0、`target/debug/` 生成（`target/ci` との並存確認）
- 終了時 `git diff --stat HEAD` → 空、`git status --ignored` → 生成物（`apm_modules/`・`.agents/`・`.claude/`・`.codex/`・`.cursor/`・`.windsurf/`・`.github/{agents,hooks,mcp.json}`・`.mcp.json`・`target/`）のみ ignored でソース差分ゼロ

補足: 分離環境での `mise run setup` は上記 shim リスポーン問題で観測不能となったが、構成 3 コマンドを個別に exit 0 で確認しており、`mise run` ラッパー自体は main リポで成功済み。新規 clone での `mise run setup` 成功の根拠として十分と判断。

### LFS / テストフィクスチャ方針の確認（E-4）

- `.gitattributes` は audio（wav/flac/mp3/ogg/opus/aac/m4a/aif/aiff）・video/capture（mp4/mov/mkv/avi/webm/m2ts）・大型編集ソース（psd/blend）のみ LFS。JSON/YAML/テキスト系は対象外。
- 方針どおり「小さいテストフィクスチャは非メディア形式で Git に置く」が可能であり、現行ルールがこれを阻害しないことを確認。
- `.github/workflows/ci.yml` は `git lfs pull` / `actions/checkout` の `lfs: true` を持たないため、CI は LFS オブジェクトを取得しない（フィクスチャが必要になるまで省略する方針と一致）。

## 作業メモ（#3 転記用）

### 実施済み環境

| 項目 | 値 |
| --- | --- |
| OS | Windows 11 x64（Git Bash） |
| mise | 2026.10.1 windows-x64 |
| Rust toolchain | 1.99.0（cargo 1.99.0 / rustfmt 1.10.0 / clippy 0.1.99、全て mise 管理・`mise.lock` 固定） |
| apm-cli | 0.33.0（`pipx:apm-cli`、`--frozen` 対応済み・`mise.lock` dep-graph 復元可） |
| serena | 1.7.0（`pipx:oraios/serena`、GitHub shorthand 指定で lock 可能） |
| 検証日 | 2026-10-19 |

### 新規 clone からのセットアップ手順

```sh
git clone <repo> && cd ruviform

# 1. ツール復元（mise.lock から固定復元。repo 全 7 ツールが解決する）
mise install --locked       # 固定復元（正本手順）
#    注: global mise config で repo lockfile に無いツールを要求している環境では
#    その外部ツール分のみ exit 1 になり得る（環境 caveat。repo ツール自体は全て解決）
mise install                # lockfile 非使用のフォールバック（通常は不要）

# 2. APM 依存・prek フック・repo ローカル LFS をセットアップ
mise run setup              # = apm install && prek install && git lfs install --local
#    CI 等で lockfile 厳格検証する場合:
mise exec -- apm install --frozen   # apm.yml と apm.lock.yaml が非同期なら失敗

# 3. Rust 検証（ローカル開発・CI 共通入口）
mise run fmt-check          # cargo fmt --all -- --check
mise run check              # cargo check --workspace --all-targets        (dev)
mise run lint               # cargo clippy --workspace --all-targets -D warnings (dev)
mise run test               # cargo test --workspace                       (dev)
mise run build              # cargo build --workspace                      (dev)
mise run verify-ci          # fmt-check -> check-ci -> lint-ci -> test-ci (--profile ci, CARGO_INCREMENTAL=0)
```

検証済みコマンド結果: 上記は全て exit 0（`mise install --locked` は repo 管理の全 7 ツールを解決。global mise config が追加ツールを要求する環境ではその分のみ exit 1 となり得る環境 caveat）。新規 clone 相当（`git worktree` のクリーンツリー）で `apm install --frozen` → setup 3 コマンド → `verify-ci` → `test` の順に成功することを確認。

### 前提条件

- **ネットワーク**: GitHub（releases/API・APM git deps）、crates.io/static.crates.io、static.rust-lang.org、PyPI（pipx 系インストール時）への到達が必要。
- **権限**: mise による `~/.local`/`AppData` への書き込み、repo 内 `.git/hooks` 更新（prek）・`.git/config` 更新（git lfs --local）が可能であること。
- **認証**: APM の依存 git repo 取得には HTTPS 経由の git 認証（Git Credential Manager 等）が必要。`scarecrow173/.github-private` への policy read はスコープ不足でも warning のみで継続（fatal でない）。非対話環境では `GIT_TERMINAL_PROMPT=0` 推奨（credential prompt でのハング回避）。
- **mise 自体**: リポジトリ外の前提（winget/公式インストーラ等で導入済みであること）。`mise trust` は clone 後の初回実行時に必要。

### lockfile 再生成条件

| lockfile | 再生成トリガ | 手順 |
| --- | --- | --- |
| `mise.lock` | `mise.toml` の `[tools]` 変更・バージョン更新時 | `mise install` → `mise lock`（pipx ツールは `.mise/locks/` 配下に dep-graph 生成物が出力され、合わせてコミット対象。`uvx_args`/`pipx_args` は dependency locking 非対応のため指定しないこと） |
| `apm.lock.yaml` | `apm.yml` の dependencies/targets/MCP 変更時 | `apm install`（lock 自動更新）→ `apm lock` で整合確認 |
| `Cargo.lock` | workspace ルート/メンバーの `[dependencies]` 変更時 | `cargo build` / `cargo generate-lockfile` で自動更新 |
| 不整合時の復旧 | いずれかが矛盾 | `git checkout -- <lockfile>` → 上記 lock コマンド再実行 |

### 暫定開発規約（design 転記）

- **`unsafe`**: 原則禁止。必要な場合のみ SAFETY コメント付きの局所許可（`#[expect(unsafe_code)]`、未対応なら `#[allow(unsafe_code)]`）。`workspace.lints.rust.unsafe_code = "warn"` は `lint`/`lint-ci` の `-D warnings` で実質 deny。
- **feature**: default feature は最小限。新規 feature・optional 依存は追加理由を PR に記録。
- **依存追加**: メンテナンス状況・ライセンス・依存木サイズを確認し理由を記録。`Cargo.lock` は追跡継続。
- **lint**: `workspace.lints` が正本。CI は `clippy -- -D warnings` を必須ゲート。rustfmt は標準設定（`rustfmt.toml` は必要時に追加）。
- **doc test**: 公開 API の例は doctest に書き `cargo test` で検証。
- **`*-ci` タスク**: CI 用途限定（`target/ci/` が `target/debug/` と並存して容量を消費するため常用しない）。
- **LFS / アセット**: `.gitattributes` の audio/video/大型ソース一律 LFS は維持。小さいテストフィクスチャは非メディア形式（JSON/YAML 等）で Git に置く。CI ではフィクスチャが必要になるまで `git lfs pull` を省略（現行 `ci.yml` も LFS 取得なし）。
- **手動 CI**: `workflow_dispatch` 専用。default branch に存在することが実行前提のため、導入ブランチ上では静的検証（`check-yaml`/`actionlint`）に留め、初回の実動作確認は default branch マージ後に Actions UI から入力なしでディスパッチして `df -h`/`du -sh target` の実測ログを取得する。手動 CI を実行しなくても PR は自動検証済みにならない。

### 未検証 OS / 未実施検証

- **macOS / Linux ローカル手順**: 未検証（実施は Windows x64 のみ）。特に shim 解決・pipx・改行差異は未確認。
- **GitHub Actions 実ディスパッチ**: 未実施（`workflow_dispatch` は default branch マージ後のみ可能。静的検証 actionlint/check-yaml は pass）。
- **`mise install --locked` の Linux/macOS 上での挙動**: 未検証（実施は Windows x64 のみ）。pipx 2 件の欠陥は解消済みで repo 全 7 ツールが lockfile 解決するため、global config を持たないクリーン環境では exit 0 が期待される。CI（ubuntu-latest）での実ディスパッチが確証となる。
- **真の新規 Windows 環境（mise 未インストールの素のマシン）**: 未検証。worktree + 分離環境での代替検証済みだが、ネイティブインストーラ挙動は未確認。

## Risks

- **`mise install --locked` の pipx 2 件の欠陥は解消済み**（後続対応: `uvx_args` 除去・`pipx:oraios/serena` への spec 変更・`.mise/locks/` dep-graph コミット）。repo 全 7 ツールが `mise.lock` から解決することを実機再検証済みで、Issue #2 AC「固定 lockfile を使用した再現インストール手順が検証されている」は repo スコープで充足。
- 残る環境 caveat: 本機の global mise config（`C:\Users\akiyoshi\.config\mise\config.toml`）が要求する repo lockfile 外ツールが `--locked` で解決失敗し exit 1 となり得る。repo ツール自体は全て解決済みで、global config を持たない CI 環境では影響しない見込み（実ディスパッチは未実施）。
- `uv = "0.12"` は範囲指定のため、`mise install`（非 locked）では `0.12.x` 最新へ浮動し得る。`--locked` 復元は `mise.lock` の確定値（`0.12.24`）に固定される。
- 分離環境で発生した shim リスポーン問題は検証手法側の artifact。通常環境では未発生。

## Follow-ups

- ~~`pipx:apm-cli` の `uvx_args` 除去可否の確認と `mise lock` 再生成による `--locked` 復元~~ → **実施済み**（`mise.toml`/`mise.lock`/`.mise/locks/` を修正し、repo 全 7 ツールの `--locked` 復元を実機で確認。上記 caveat 節の解決記録を参照）。
- `.github/workflows/ci.yml` の "Restore tools from mise.lock" ステップの実ディスパッチ確認（default branch マージ後。pipx 欠陥は解消済みのため、global config 非存在の CI では全件解決する見込み）。
- Rust バージョン・edition・MSRV の確定、クレート分割は #1 の設計対象。
- README/AGENTS への転記と `apm.yml` の `project-template` リネームは #3 の責務。
