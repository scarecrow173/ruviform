---
id: "SPEC-034RBIrH4H2hFNw7gfmgDa"
type: "spec"
status: "approved"
title: "Issue #330 CI-OPT-03 UI npm ci 重複排除・TypeScript検証パイプライン集約"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/330"
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/ruviform/issues/310"
    - "https://github.com/scarecrow173/ruviform/issues/263"
    - "https://github.com/scarecrow173/ruviform/issues/311"
  changes:
    added: []
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-330-ui-npm-ci-dedup.md"
    - "docs/plans/issue-330-ui-npm-ci-dedup.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-219-consumer-compat-contract.md"
    - "docs/designs/issue-310-browser-gui-test-harness.md"
    - "docs/designs/issue-311-ipc-conformance-replay.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-330-ui-npm-ci-inventory.md"
  derived-by:
    - "docs/designs/issue-330-ui-npm-ci-dedup.md"
    - "docs/plans/issue-330-ui-npm-ci-dedup.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-330-ui-npm-ci-dedup.md"
  references:
    - "mise.toml"
    - ".github/workflows/ci.yml"
    - "ui/showcase/test/deps.test.ts"
  defers: []
  deferred-by: []
---

# Issue #330 CI-OPT-03 UI npm ci 重複排除・TypeScript検証パイプライン集約

## What Should Be Built

`ui-verify` CI ジョブ（3 OS matrix）内で繰り返される `npm ci` を、CI 準備段階で package-lock ごとに原則 1 回だけ実行する経路へ分離する。typecheck / test / build の各検証タスクは準備済み `node_modules` を使い、検証ステップ自体の分離・順序・Evidence は不変とする。合わせて `ui/showcase` dist の実依存 DAG（core → components → viz / binding → showcase → browser-test、guitest は core/binding へ独立）をタスク依存として明示し、Showcase build の同一ジョブ内重複も排除する。

## Why Now

現行 `mise.toml` は `check-ui-*` / `test-ui-*` / `build-ui-*` の各タスク内に `npm --prefix ui/<pkg> ci` を内包しており、`ui-verify` ジョブ 1 回あたり同一 package-lock に最大 5 回（ui/showcase、`depends = ["build-ui-showcase"]` 経由を含む）、ジョブ合計 20 回の `npm ci` が走る。CI-OPT-00（#327）で計測基盤が定義され、UI Windows ジョブは約 313 秒を観測済み。重複 install は実行効率のみの損失であり、検証能力を削らずに回収できる。

## Users and Value

- PR 作成者・レビュアー: `ui-verify` 所要時間の短縮と、失敗時の npm install ノイズ削減。
- Ruviform CI メンテナ: package-lock 変更時の install 挙動が明示的な依存関係として追跡可能になる。
- Consumer（Synvelis / AudioLens）: Ruviform 側 CI 契約（required checks・Evidence）を変えないまま基盤が効率化される。

## Scope

### In Scope

- `mise.toml` の UI npm 系タスクを「install（lockfile 駆動・freshness 付き）」と「verify（typecheck/test/build）」に分離する。
- `ui-verify` ジョブに単一の準備ステップを追加し、同一ジョブ内で同一 package-lock の `npm ci` を 1 回にする。
- `build-ui-showcase` を含む dist 生成順を `depends` で明示し、`ui/browser-test` が正しい revision の showcase dist を参照することを保証する。
- 個別 mise タスクのローカル単独実行（Fresh checkout からの `mise run test-ui-headless` 等）を維持する。
- Before/After の install 回数・時間を採取し報告する。

### Out of Scope

- npm workspaces 化・`ui/*` の lockfile 統合（Issue は必須としない）。
- Playwright ブラウザバイナリ・OS システム依存の取得経路変更（CI-OPT-05 #332 の範囲）。
- `verify` ジョブ等 Rust 側 lane の変更（CI-OPT-02/04 の範囲）。
- 検証対象・テスト内容・Evidence 生成物の変更。
- `ui/showcase/test/deps.test.ts` の依存 DAG audit 契約の緩和。

## Requirements

- 同一 `ui-verify` ジョブ内で同一 package の `npm ci` は原則 1 回のみ実行される。
- typecheck / test / build タスクは `npm ci` を内包せず、準備済み `node_modules` で動作する。
- Fresh checkout では依存解決が自動で走り、lockfile 未変更かつ `node_modules` 健全な再実行では install がスキップされる。
- Showcase dist は 1 ジョブ 1 回生成され、browser-test の typecheck/実行はその dist 宣言・成果物を参照する。
- 既存タスク名（`check-ui-*` / `test-ui-*` / `build-ui-*`）と CI ステップ分割は維持し、失敗箇所は従来どおり独立して特定できる。
- 第 2 の独自ビルドシステム・独自キャッシュ機構を導入しない（mise task の標準 freshness 機能を用いる）。

## Acceptance Criteria

- [ ] AC-1: 同一 Job 内の同一 package への不要な `npm ci` が排除される（package-lock ごとに 1 回）。
- [ ] AC-2: Typecheck / Unit Tests / Build の失敗箇所は独立して特定可能（CI ステップ分離を維持）。
- [ ] AC-3: 3 OS Fresh checkout と Warm 実行が成功する。
- [ ] AC-4: ローカルの個別 mise task が Fresh checkout から単独実行可能。
- [ ] AC-5: Showcase / browser-test 成果物が正しい revision・依存順で生成される。
- [ ] AC-6: 検証対象 / Evidence が不変で、Before/After の時間とインストール回数を提示する。

## Deferred Design Concerns

- `ui/*` の npm workspaces / 単一 lockfile 化 | 依存解決の重複自体を減らせるが package 境界・private lockfile 契約の再設計を伴う | 更なる install 時間が支配的になった場合 | 現行 7 lockfile 運用の継続コスト。
- node_modules の actions/cache 導入 | Fresh job での install 自体を削れるが lockfile 整合の invalidate 設計が別途必要 | install 時間が critical path を支配し続ける場合 | キャッシュ破損時の fail モード追加。

## Sources and Relations

- 上流: Issue #330（本 spec の要件源）、#327（Before/After 計測テンプレートの正本）。
- 依存 DAG の正本: `ui/showcase/test/deps.test.ts`（#310/#311 で確立済み、src 間 import 方向の監査）。
- dist 参照契約: `ui/browser-test/tsconfig.json` の `@rvf/showcase` → `../showcase/dist` paths、`mise.toml` の `depends = ["build-ui-showcase"]`。
