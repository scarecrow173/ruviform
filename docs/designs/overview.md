---
id: "DESIGN-OVERVIEW"
type: "design"
status: "draft"
title: "System Design Overview"
created: "2026-10-19"
updated: "2026-10-24"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "ruviform-architecture-design-baseline.md"
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

Ruviform の設計成果物の全体マップ。開発基盤は GitHub Issue #2 由来の Rust 開発基盤（`dev-foundation.md`、approved）で確定済み。プロダクト本体のアーキテクチャ設計ベースライン（クレート DAG・データモデル・RT/オフライン実行モデル・DSP/SIMD・プラグイン・UI・検証計画・ロードマップ）は Issue #1 由来の `ruviform-architecture-design-baseline.md`（approved）がエントリポイントである。

## System Boundaries

- **ツールチェーン層**: `mise.toml` / `apm.yml` を正本とし、`mise.lock` / `apm.lock.yaml` / `Cargo.lock` で再現性を確定する層。Rust toolchain・開発タスク・APM 依存を含む。
- **Rust ワークスペース層**: ルート `Cargo.toml`（共通ポリシーと `[profile.dev]` / `[profile.ci]`）と最小メンバークレート `crates/ruviform-core`（仮置き）。最終的なクレート分割の設計案は #1 の設計ベースライン（`ruviform-architecture-design-baseline.md`、approved）に記録され、実装 Phase に応じて `members` を段階追加する。
- **CI 層**: `.github/workflows/ci.yml`。`workflow_dispatch` のみ・`contents: read`・Linux 必須検証・`--profile ci` での検証とディスク計測を担当する層。
- **ドキュメント層**: `docs/` 配下の spec / ADR / design 群と `AGENTS.md` の運用契約。README／AGENTS 本文の改稿は #3 の境界。#1 設計ベースライン approved に伴う設計反映の差分更新は #1 A.8 タスクとして実施。

## Product Architecture（Issue #1）

プロダクト本体のレイヤー構造。詳細・クレート一覧・実行モデル・検証計画・ロードマップの正本は [ruviform-architecture-design-baseline.md](ruviform-architecture-design-baseline.md) を参照。

```text
Applications / Hosts / Integrations（利用側）
        │
Engines:  ruviform-audio / ruviform-video
        │
Domain foundations:  ruviform-dsp / ruviform-audio-analysis / ruviform-video-proc
                     ruviform-media-io（demux/mux・decode/encode の抽象境界）
                     ruviform-audio-core / ruviform-video-core
        │
Primitives:  ruviform-media-core / ruviform-rt / ruviform-exec / ruviform-math
        ▲
Optional platform adapters（opt-in・基礎層への依存は下向きのみ）:
  ruviform-audio-device / ruviform-gpu / ruviform-io-* / ruviform-ui-webview

拡張層:  ruviform-plugin-core / -host / -export / -clap / -vst3、ruviform-ui
```

- 依存は上位 → 下位のみ（循環依存禁止）。基礎クレートから GUI・オーディオデバイス・GPU への強制依存を作らない。
- メディア I/O・codec・外部ライブラリは `ruviform-media-io` の抽象境界の背後にある分離クレート（opt-in）として扱い、外部型・エラー・機能フラグを公開 API に露出しない。
- RT/非 RT は API・ライフサイクルで分離。RT パスは事前確保・有界・alloc/dealloc 禁止。検証可能な違反は CI、静的不可分は文書＋レビュー。

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
- [issue-28-ui-poc.md](issue-28-ui-poc.md) — Issue #28 UI 技術検証 PoC の設計（使い捨て検証ハーネスと計測・評価方式、`poc/ui/` 独立ワークスペース・`ui-poc` 実験ハーネス・採否 ADR へのエビデンス供給）(approved)
- [replace-template-derived-docs-with-ruviform.md](replace-template-derived-docs-with-ruviform.md) — テンプレート由来ドキュメント・識別子のRuviform固有化の設計（README/AGENTS 改稿・識別子更新の文書設計、Issue #3）(approved)
- [ruviform-architecture-design-baseline.md](ruviform-architecture-design-baseline.md) — Ruviform アーキテクチャ設計ベースライン（クレート DAG・データモデル・RT/オフライン実行モデル・DSP/SIMD・プラグイン・UI・検証計画・暫定ロードマップ、Issue #1）(approved)
- [shared-foundations-and-modularization-policy.md](shared-foundations-and-modularization-policy.md) — 汎用 Foundation・Observability（Logging/Profiling/Metrics/Diagnostics/Debug）と段階的なクレート分割・依存/RT 境界の設計方針（Issue #101。`ruviform-foundation` 等は構成候補であり未実装）(approved)
- [foundation-observability-minimal-api.md](foundation-observability-minimal-api.md) — Foundation/Observability 候補モジュールの最小 API スケッチ・新設時期トリガ・依存 DAG・Counter 選定の設計（Issue #103。#101 設計の「適用・移行順序」2・3）(draft)
- [audio-device-boundary-and-poc.md](audio-device-boundary-and-poc.md) — オーディオデバイス境界（ポート trait・cpal adapter・コールバック配管）と PoC 検証設計（Issue #23）(approved)
- [time-model-extension.md](time-model-extension.md) — 時間モデル拡張の設計（FrameRate/TimeBase 意味分離・Timecode 拡張・丸め誤差累積方針、Issue #42）(approved)
- [ofx-host-adapter.md](ofx-host-adapter.md) — OFX ホストアダプタ詳細設計（video 契約境界・C ABI バインディング・セッション/lifecycle・image/time/param 意味論、Issue #98）(approved)
- [engine-plugin-host-integration.md](engine-plugin-host-integration.md) — エンジン/利用側へのプラグイン統合設計（non-RT 制御/RT 処理所有分離・`Send` endpoint + `FnMut` ノード接着・plan 退役シーム、Issue #113）(draft)
- [issue-115-l1-gui-cross-process-parenting.md](issue-115-l1-gui-cross-process-parenting.md) — L1 GUI 採否設計（プロセス境界越し親ウィンドウ埋め込みの OS 別調査・`Unsupported` 維持判定と境界 pin・`gui_open` keep-for-future、Issue #115）(draft)
