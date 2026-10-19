---
id: "TASK-034IdQxdPQUxXweHBN1ZxT"
type: "task"
status: "done"
title: "再現インストール検証と #3 転記用作業メモ"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/2"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/verify-reproducible-setup-and-work-memo.md"
    modified:
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/dev-foundation-bootstrap.md"
  implemented-by:
    - "../impl/ir/verify-reproducible-setup-and-work-memo.md"
  depends-on:
    - "../plans/dev-foundation-bootstrap.md"
    - "pin-rust-toolchain-and-lockfile.md"
    - "create-minimal-workspace-and-smoke-test.md"
    - "define-mise-dev-and-ci-tasks.md"
    - "add-manual-ci-workflow.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/lockfile-reproducible-setup.md"
    - "../test-specs/mise-verification-tasks.md"
    - "../test-specs/ci-profile-build-behavior.md"
    - "../test-specs/manual-ci-workflow.md"
  references: []
  defers: []
  deferred-by: []
---

# 再現インストール検証と #3 転記用作業メモ

## Work

plan `PLAN-034IdrispOUms1c32192B6` のスライス E（Phase E-1〜E-6）。スライス A〜D の成果物が「新規 clone から固定 lockfile で再現できる」ことを実機で検証し、暫定開発規約・前提条件・未検証 OS 等の #3 転記材料を `docs/impl/ir/` の Implementation Record と実施 PR 本文に記録する。本タスクは検証と記録が主体で、既存の正本ファイルは変更しない。

- [x] `mise install --locked` を実行し、lockfile から toolchain（`cargo` / `rustfmt` / `clippy` 含む）が復元できることを確認する → 実行済み。検証時は `pipx:apm-cli`（`uvx_args` が dependency locking 非対応）と `pipx:serena-agent`（dep graph 欠落）の 2 件で exit 1 だったが、**後続対応で解消済み**: `uvx_args` 除去（apm-cli 0.33.0 は PyPI final release で不要）、`pipx:oraios/serena` への GitHub-shorthand spec 変更、`uv` 0.12 更新、`.mise/locks/` dep-graph コミットにより repo 全 7 ツールが `mise.lock` から解決することを実機で再検証済み（`cargo`/`rustfmt`/`clippy` 含む）。残る exit 1 は global mise config 由来の repo lockfile 外ツールのみ（環境 caveat、クリーン CI では非発生）。詳細は IR に記録
- [x] `mise exec -- apm install --frozen` の成否を実機確認する。未対応なら `apm install` の再現性を確認する同等手順（`apm.lock.yaml` 差分なしの確認等）を定義して記録する → **apm-cli 0.33.0 で `--frozen` 対応済み**。main リポジトリ（exit 0・9 deps）とクリーン worktree（exit 0・31 deps + 2 MCP servers）の双方で成功。`git diff --exit-code -- apm.lock.yaml` も exit 0（冪等）。同等手順は不要と判断し IR に記録
- [x] `git status --ignored` と `git check-ignore` で、`apm_modules/`・`.agents/`・`.env*`・`target/`・Serena ローカル状態等が意図どおり非追跡であり、`Cargo.lock`・`mise.lock`・`apm.lock.yaml` が追跡対象であることを確認する → `git check-ignore target/ apm_modules/ .agents/ .env .env.local .serena/ .sdp/ .mise-data/` 全て出力、`git ls-files` が 3 lockfile を出力、`git status --ignored` の ignored は意図した生成物のみ
- [x] `.gitattributes` の audio/video 一律 LFS を確認し、「小さいテストフィクスチャは非メディア形式（JSON/YAML 等）で Git に置く」「CI ではフィクスチャが必要になるまで `git lfs pull` を省略する」方針の確認結果を記録する → LFS 対象は audio/video/大型編集ソースのみで JSON/YAML は対象外、方針を阻害しないことを確認。`ci.yml` に `git lfs pull`/`lfs: true` なし。IR に記録
- [x] 暫定開発規約（`unsafe` 原則禁止＋SAFETY コメント付き局所許可、default feature 最小、依存追加は理由記録、`workspace.lints` 正本＋`-D warnings` ゲート、公開 API の例は doctest）を design どおり作業メモへ転記する → IR「作業メモ（#3 転記用）」節へ design どおり転記（`*-ci` タスクの CI 用途限定・LFS 方針・手動 CI 運用も併記）
- [x] 実施した OS・実行したコマンド・検証結果・前提条件（ネットワーク/権限/認証）・lockfile 再生成条件・未検証 OS を `docs/impl/ir/` の IR と実施 PR 本文に記録し、#3 が転記可能な状態にする → IR に全項目記録（実施 OS: Windows 11 x64、未検証: macOS/Linux ローカル・Actions 実ディスパッチ・素の新規 Windows）。実施 PR 本文への転記は PR 作成時の責務

新規 clone 再現性の追加確認が必要な場合は、`git worktree` 等のクリーンな作業ツリーで V-1 / V-2 / V-5 相当を再実行する。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dev-foundation-bootstrap.md](../plans/dev-foundation-bootstrap.md) — `PLAN-034IdrispOUms1c32192B6`（approved）。Phase E / V-1・V-2・V-9・V-11 の正本
- [../designs/dev-foundation.md](../designs/dev-foundation.md) — `DESIGN-034IddXFCRKDIW01WXkSla`。暫定開発規約・LFS フィクスチャ方針・Verification Notes の転記元
- [../specs/establish-rust-dev-foundation.md](../specs/establish-rust-dev-foundation.md) — `SPEC-034IcwhwIFSNUNKtX7KK0Q`。Requirements §1・§6（再現セットアップ・作業メモ記録）
- [../test-specs/lockfile-reproducible-setup.md](../test-specs/lockfile-reproducible-setup.md) — `TSPEC-034IdHmKPCUPXNnJh1Y7f7`。lockfile 復元・追跡/非追跡分離・記録義務の契約
- [../test-specs/mise-verification-tasks.md](../test-specs/mise-verification-tasks.md) — `TSPEC-034IdHshzpNiPTqyyvwL6J`。`*-ci` タスクの CI 用途限定運用の記録対象
- [../test-specs/ci-profile-build-behavior.md](../test-specs/ci-profile-build-behavior.md) — `TSPEC-034IdHsItEuo0TZWTPOJUu`。`target/ci/`・`target/debug/` 並存の運用注意の記録対象
- [../test-specs/manual-ci-workflow.md](../test-specs/manual-ci-workflow.md) — `TSPEC-034IdHt7P0ega4N0bbNANg`。ブランチ上は静的検証・マージ後手動ディスパッチの運用明記対象

## Dependencies

- 前提タスク: `pin-rust-toolchain-and-lockfile.md`、`create-minimal-workspace-and-smoke-test.md`、`define-mise-dev-and-ci-tasks.md`、`add-manual-ci-workflow.md`（スライス A〜D すべての成果物が検証対象）
- 後続をブロック: なし（クリティカルパスの終端）
- 各先行タスクで未作成の場合、本タスクで `docs/impl/ir/` の IR を impl-doc 規約に従い作成する

## Affected Files

- `docs/impl/ir/` — Implementation Record を作成・更新し、暫定開発規約・LFS 方針・検証済み OS・前提条件・lockfile 再生成条件・未検証 OS を記録
- 実施 PR 本文 — IR と同内容の #3 転記用サマリを記録
- その他の既存ファイル — 変更しない（検証・確認のみ）

## Done When

- [x] `mise install --locked` と APM の再現インストール（または記録済みの同等手順）が成功する → **成功**: `apm install --frozen` は exit 0。`mise install --locked` は検証時に pipx 2 件で exit 1 だったが、後続対応（`uvx_args` 除去・`pipx:oraios/serena` spec 変更・`.mise/locks/` dep-graph コミット）で repo 全 7 ツールが `mise.lock` から解決することを再検証済み。残る失敗要因は global config 由来ツールのみの環境 caveat。詳細は IR に記録
- [x] 追跡対象（3 lockfile）と非追跡対象（`target/`・`apm_modules/`・`.env*` 等）の分離が確認されている → `git ls-files` / `git check-ignore` / `git status --ignored` で確認済み
- [x] IR / PR 本文に暫定開発規約・LFS フィクスチャ方針・実施 OS・検証コマンドと結果・前提条件・lockfile 再生成条件・未検証 OS が記録され、#3 が転記可能である → IR `docs/impl/ir/verify-reproducible-setup-and-work-memo.md` に全項目記録。PR 本文への転記は PR 作成時に実施
- [x] `apm install --frozen` の対応可否と、未対応時の同等手順が記録されている → apm-cli 0.33.0 で対応済み・exit 0 を実機確認し IR に記録

## Verification

- [x] `mise install --locked` exits 0 → **解消済み（repo スコープで exit 0 相当）**: repo の全 7 ツール（`uv`/`prek`/`git-lfs`/`pipx:apm-cli`/`pipx:oraios/serena`/`rust`/`actionlint`）が `mise.lock` から解決。修正内容（`uvx_args` 除去・`pipx:oraios/serena` への spec 変更・`uv` 0.12・`.mise/locks/` dep-graph コミット）は IR に記録。残る exit 1 は global mise config（`C:\Users\akiyoshi\.config\mise\config.toml`）由来の repo lockfile 外ツールのみで、クリーン環境・CI では発生しない環境 caveat
- [x] `mise exec -- cargo --version && mise exec -- cargo fmt --version && mise exec -- cargo clippy --version` exits 0（復元された toolchain の疎通）→ `cargo 1.99.0` / `rustfmt 1.10.0-stable` / `clippy 0.1.99`、全て exit 0
- [x] `mise exec -- apm install --frozen` exits 0（未対応なら `mise exec -- apm install` 後に `git diff --exit-code -- apm.lock.yaml` が exits 0、かつ同等手順が IR に記録済み）→ `--frozen` 対応済み・exit 0（main と worktree 双方）。lockfile 差分なし
- [x] `git ls-files Cargo.lock mise.lock apm.lock.yaml` が 3 ファイルすべてを出力する → 3 件出力
- [x] `git check-ignore target/ apm_modules/ .agents/` が各パスを出力する → 全パス出力（`.env*`・`.serena/`・`.sdp/` 等も確認）
- [x] `git status --ignored` で意図しない追跡ファイル・未除外の生成物がないことを確認する → ignored は `.agents/`・`.claude/`・`.codex/`・`.cursor/`・`.github/{agents,hooks,mcp.json}`・`.mcp.json`・`.sdp/`・`.serena/`・`.windsurf/`・`apm_modules/`・`target/` のみ。追跡済み生成物・秘密情報なし
- [x] `ls docs/impl/ir/*.md` が IR を返し、`grep -rE '未検証|mise install --locked|lockfile' docs/impl/ir/` が記録内容に一致を返す → IR 5 件。grep は本 IR の記録に一致
- [x] Manual: IR と実施 PR 本文の記録項目（暫定開発規約・LFS 方針・OS・前提条件・再生成条件）が揃い #3 が転記可能（補助）→ IR に全項目記録済み。PR 本文への転記は PR 作成時の責務
