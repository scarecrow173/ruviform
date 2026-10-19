---
id: "DESIGN-OVERVIEW"
type: "design"
status: "draft"
title: "System Design Overview"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source: []
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
    - "dev-foundation.md"
    - "replace-template-derived-docs-with-ruviform.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# System Design Overview

Ruviform の設計成果物の全体マップ。現時点で確定している設計は GitHub Issue #2 由来の Rust 開発基盤（`dev-foundation.md`）のみであり、プロダクト本体（DSP・video pipeline・SIMD・WebView・VST）の設計は #1 の詳細設計で扱う。

## System Boundaries

- **ツールチェーン層**: `mise.toml` / `apm.yml` を正本とし、`mise.lock` / `apm.lock.yaml` / `Cargo.lock` で再現性を確定する層。Rust toolchain・開発タスク・APM 依存を含む。
- **Rust ワークスペース層**: ルート `Cargo.toml`（共通ポリシーと `[profile.dev]` / `[profile.ci]`）と最小メンバークレート `crates/ruviform-core`（仮置き）。最終的なクレート分割の境界は未定義（#1）。
- **CI 層**: `.github/workflows/ci.yml`。`workflow_dispatch` のみ・`contents: read`・Linux 必須検証・`--profile ci` での検証とディスク計測を担当する層。
- **ドキュメント層**: `docs/` 配下の spec / ADR / design 群と `AGENTS.md` の運用契約。README／AGENTS 本文の改稿は #3 の境界。

## Core Components

- `mise.toml` — ツールバージョンと開発タスク（`fmt` / `check` / `lint` / `test` / `build` 系）の唯一の入口
- `Cargo.toml`（workspace ルート）— Cargo プロファイル・edition・lint 共通ポリシーの置き場
- `crates/ruviform-core` — workspace が空にならない最小メンバーと smoke test（仮置き）
- `.github/workflows/ci.yml` — 手動ディスパッチの検証ワークフロー
- lockfile 群 — `mise.lock` / `apm.lock.yaml` / `Cargo.lock`（すべて Git 追跡）

## Data Flow

- セットアップ: `mise install` → `mise lock` → `mise run setup`（APM・prek・LFS）→ `apm lock` → lockfile commit → `mise install --locked` で復元検証
- ローカル検証: `mise run fmt/check/lint/test/build`（`dev` プロファイル・フルデバッグ維持）
- 手動 CI: dispatch → checkout → `df -h` → `mise install --locked` → prek 検証 → `*-ci` タスク（`--profile ci`・`CARGO_INCREMENTAL=0`）→ 事後ディスク計測

## Non-Functional Constraints

- 再現性: 固定 lockfile からの復元（`mise install --locked` / APM frozen install 相当）
- 権限最小化: CI は `contents: read` のみ、自動トリガーなし
- ディスク使用量: `ci` プロファイルでデバッグ情報・増分コンパイル由来の容量を削減し計測を記録
- デバッグ可能性: ローカル `dev` のフルデバッグ情報を維持し、CI の容量対策を波及させない

## Detailed Design Documents

- [dev-foundation.md](dev-foundation.md) — Rust 開発基盤の設計（最小ワークスペース・CIプロファイル・手動CI）(approved)
- [replace-template-derived-docs-with-ruviform.md](replace-template-derived-docs-with-ruviform.md) — テンプレート由来ドキュメント・識別子のRuviform固有化の設計（README/AGENTS 改稿・識別子更新の文書設計、Issue #3）(approved)
