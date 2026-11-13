---
id: "TSPEC-8W-EruC9xGTrXeKm8RqkJ"
type: "test-spec"
status: "approved"
title: "Issue #275 OBS-01〜11 Owner/API/Feature Matrix 対応付けの検証規則"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "docs/specs/issue-275-obs-owner-map.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/designs/issue-275-obs-owner-map-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/plans/issue-275-obs-owner-map-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-275-obs-owner-map-task.md"
      - type: "doc"
        path: "docs/impl/ir/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/impl/exp/issue-275-obs-owner-map.md"
      - type: "file"
        path: "evidence/rf275/obs-owner-map-audit.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
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
    - "docs/test-specs/issue-275-observability-integration-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-275-obs-owner-map-design.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-275-obs-owner-map.md"
    - "docs/designs/issue-275-obs-owner-map-design.md"
  verified-by: []
  references:
    - "evidence/rf275/obs-owner-map-audit.md"
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
  defers: []
  deferred-by: []
---

# Issue #275 OBS-01〜11 Owner/API/Feature Matrix 対応付けの検証規則

## Purpose

本 test-spec は `docs/designs/issue-275-obs-owner-map-design.md`
が定める「統合設計正本への Owner/API/Feature/CI Case 対応表・
AL/SV 引き渡し・Deferred Owner/Trigger」の記述が、実在する
正本（Issue・crate・workflow・mise タスク）に基づき、かつ
状態語彙の分離を保っていることを検証する規則を固定する。
文書監査タスクであり、コードの test ではない — crate の
contract test の正本は per-issue test-spec（#276〜#293）
と `docs/test-specs/issue-275-observability-integration-
design.md` であり、本書はそれらを置き換えない。

## Feature

OBS-01〜11 Owner/API/Feature/CI Case 対応表と Deferred 範囲の
Owner/Trigger が統合設計正本に正確に記述される。

## Rules

- **R1（全 OBS 行の存在と正確性）**: 対応表は OBS-01〜11 の
  全 11 行を持ち、各行の Owner Issue・module/crate・
  feature・公開 API 代表・正本 doc が実在することを
  `evidence/rf275/obs-owner-map-audit.md` の実測と突合して
  確認できる。存在しない Issue/文書/module/feature を
  根拠にした記述がない。
- **R2（feature 実態の反映）**: feature 列は
  `default=[]`・`logging`・`tracing-export`・
  `allocation-probe`・`integration`・なし の実在値のみを
  使い、`collector`/`diagnostics`/`debug-tools` 等の
  未採用 feature 名を記載しない。OBS-04 が
  `ruviform-plugin-host` 側の実装であることを区別する。
- **R3（CI Case の実在性と非昇格）**: 「既存 CI Case」列は
  `mise.toml` タスク名・`.github/workflows/` の job 名に
  一致する既存 lane のみを参照し、新規 workflow を提案しない。
  lane 定義の存在から個別 SHA の Pass を推論せず、未実行は
  `NotRun` と記録する。
- **R4（状態語彙の分離）**: 要件状態（`Required`/`Deferred`/
  `NotApplicable`）と実行結果（`Pass`/`Fail`/`NotRun`/
  `Unsupported`）を混同せず、`NotRun`/`Fail` を `Pass` に
  置き換えない。
- **R5（OS 要件レベルの現行方針）**: Windows は OS-L1 必須・
  GUI-L2/EXT-L3 は使用機能のみ Required、macOS/Linux は
  OS-L1 必須・GUI/EXT は利用要件発生まで Deferred という
  現行方針を反映し、未使用 device/plugin format/特殊
  filesystem を global blocker にしない。
- **R6（AL/SV 引き渡しと履歴 Issue の扱い）**: AL #127・
  SV #28 の Logging/Privacy/RT/精度の独立 Headless 受入が
  #281/#294 への引き渡しとして明記され、一方の Consumer の
  Pass を他方へ転用しない。AL #138・SV #81 は CLOSED の
  履歴 Issue として扱い、再 Open する記述がない。
- **R7（Deferred の Owner/Trigger 完備）**: 実外部
  Provider・実 OS GUI・実 Device・長時間 Profiler の各
  Deferred 項目に、再開 Owner と Activation Trigger
  （利用要件の具体化）が明記される。feature compile
  matrix のような継続 gate 項目は Deferred と区別される。
- **R8（保護対象と変更範囲）**: 「Workerへの共通契約」節を
  含む既存設計書の保護対象に差分がなく、変更は `docs/`・
  `evidence/rf275/` と生成 index のみである。

## Examples

- Given OBS-01 の行、When feature 列を検査する、Then
  `logging` feature と `tests/logging.rs`（
  `required-features=["logging"]`）を実行する既存 lane と
  して `msrv-verify` の `test-observability-msrv`
  （`--all-features`）が記され、Windows/macOS native での
  feature 付き build/link が `NotRun` と記録される（R1/R3）。
- Given OBS-06 行、When Owner 列を検査する、Then OPEN の
  gate Issue #281 が検証 Owner として記され、CLOSED の
  実装 Issue 群と区別される（R1）。
- Given Deferred 表の「実外部 Provider」行、When Owner/
  Trigger 列を検査する、Then 再開 Owner（Ruviform
  Observability owner・新規 Issue 起票・#281 gate）と
  Activation Trigger（Host/Consumer が実 exporter 接続を
  製品機能として採用すると決定した時点）が記される（R7）。
- Given 本 PR の diff、When `git status`/`git diff --stat`
  を確認する、Then 変更は `docs/`・`evidence/rf275/` と
  生成 index のみで `crates/`・manifest・workflow に差分が
  なく、「Workerへの共通契約」節に差分がない（R8）。

## Guarantees

- 対応表の各行は実在する正本への参照のみを持ち、存在しない
  Issue/文書/module/feature/command を根拠にしない。
- 「設計承認・実装完了・OS-L1 証拠・Windows 使用機能
  L2/L3 証拠・Consumer 受入・candidate-SHA gate 完了」の
  各段階が分離して記述される。
- Consumer の独立 Oracle（AL #127・SV #28）が #281/#294
  経由で要求されることが明記され、履歴 Issue の再 Open を
  示唆しない。
- Issue 状態 snapshot は `gh issue view` 実測日付付きで
  記録され、将来の状態変化に対する再照合の必要性が示される。

## Non-goals

- per-module の実走テスト合否（per-issue test-spec と crate
  `tests/` が正本）。
- Consumer repo での parity/RT latency の実測（#219/#294
  consumer-verify・AL #127/SV #28 が Owner）。
- 実機 RT 計測・長時間 profiling・実 OS GUI・実 Device・
  実外部 Provider の実施（Deferred — Owner/Trigger のみ）。
- `crates/`・`Cargo.toml`・`mise.toml`・`.github/` の変更、
  GitHub blockedBy の登録・削除。

## Risk

- 本 test-spec が検証するのは文書の正しさであり、Issue #275
  の受入 Gate 実行（#281 G0〜G4・#219/#294・Consumer Oracle）
  は別段階で検証される。対応表が完璧でも gate 未実施なら
  Issue #275 の受入は未完である — この分離を崩すと「対応表
  完成＝受入完了」の誤認が起きる。
- Issue 状態・CI lane 構成は変動する。snapshot 日付の明記で
  緩和するが、将来の main 変更で lane 名や feature 構成が
  変わった場合、本表は再照合が必要になる。
