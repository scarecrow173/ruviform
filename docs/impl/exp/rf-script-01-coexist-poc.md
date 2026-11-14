# RF-SCRIPT-01 Coexist PoC — Experiment Log

headless 単一バイナリ Luau+Rhai 共存 PoC の実験記録。
対象: `poc/script-coexist/`（独立 workspace crate `script-coexist-poc`）。
判定台帳: [`evidence/rf559/gap-report.md`](../../../evidence/rf559/gap-report.md)、
検証記録: [`evidence/rf559/verification.md`](../../../evidence/rf559/verification.md)。

## 実験環境

- OS: Windows 11 x86_64
- Toolchain: mise 管理 Rust（`mise.toml` 正本）、canonical 検証は
  `mise run poc-script-coexist-*` 経由
- 検証日時: 2026-11-14
- 依存: `mlua 0.12.2`（features `luau`,`send` — vendored `luau0-src`
  C++ build）、`rhai 1.27.0`（feature `sync`）、`serde_json`、`sha2`
- Fixture: `tests/fixtures.rs` 36 test（FX-P-01〜08 / FX-N-01〜28）+
  `src/main.rs` デモ binary

## 結果サマリ

| Case | 能力 | 結果 | 備考 |
| --- | --- | --- | --- |
| Coexist-01 | 同一バイナリ同時リンク・初期化 | Pass | mlua+luau0-src と rhai を 1 crate に link。registry に 2 backend 同時登録 |
| Coexist-02 | 実行時明示選択・交互/並行評価 | Pass | language/backend requirement resolve、交互 4 回・2 thread 並行で各 backend が正しく生成 |
| Coexist-03 | 同一意味 artifact digest 一致 | Pass | action_recipe・workflow_definition とも両言語で `sha256:v1:254a…` 一致 |
| Coexist-04 | 混種 Workflow 参照（Host Typed Port） | Pass | Luau 産 recipe を Rhai 産 workflow が digest 参照、`build_workflow_plan` が 2 node plan を構築 |
| Coexist-05 | VM ポインタ直渡し・循環拒否 | Pass | `vm_reference`/`handle`/`pointer`/`direct` → `DirectReferenceRejected`。真の循環は digest 上構築不能（防御的 `CycleDetected` 実装済み） |
| Value-01 | i64 厳密・Null/Absent・ID・Time | Pass | `i64::MAX` が `$rv` tag 経路で厳密往復、Absent≠Null、Time/ResourceId/Bytes 保持、cross-backend digest 一致 |
| Value-02 | Unsafe Host API 拒否 | Pass | `os.system` 等の binding 名・宣言/handler 不整合を `InvalidHostSurface` で拒否 |
| Budget-01 | Quota 強制 | Pass | instructions/wall-time/output/host-calls は両 backend で `BudgetExceeded`。memory は Luau のみ強制、Rhai は `UnsupportedBudget`（広告どおり fail-closed） |
| Budget-02 | 協調 Cancel・Session Isolation | Pass | `CancelToken` + safepoint で `Cancelled`。session 間 global 漏洩なし |
| Registry-01 | Fail-Closed 解決 | Pass | UnknownLanguage/UnknownBackend/Duplicate/Ambiguous/UnsupportedContractVersion/MissingCapability |
| Session-01 | Fail-Closed session 検証 | Pass | digest mismatch/language mismatch/host surface/source too large/RT context → typed error |
| Plan-01 | Plan Fail-Closed | Pass | unknown/stale/kind/digest/schema/not-a-workflow → `PlanError` |

## Gap 判定

PoC 範囲内の機能 Gap なし。共存は **成立**。
詳細・OS matrix・#558/#553 への引き継ぎは gap report を参照。

## 実行コマンドと結果

- `mise run fmt-check` → Pass
- `mise run poc-script-coexist-check` → Pass
- `mise run poc-script-coexist-lint` → Pass（`-D warnings`）
- `mise run poc-script-coexist-test` → Pass（36 test）
- `mise run poc-script-coexist-run` → Pass（demo 出力は verification.md）

## スコープ外（NotRun/Deferred）

- macOS/Linux の Build/Link/Run — 本環境（Windows）では実行不能のため
  **NotRun**（技術非対応は未確定、Unsupported にしない）。
- 実 Runtime 製品移管・3OS 配布・Consumer 適用受入 — #558/#553/#294/
  synvelis#264 の統合 Gate。
- `*-msrv`、`*-full`、Feature Matrix — タスク指定により実行しない。

## 採用した設計判断（実装中）

- **`mlua` `send` feature**: `ScriptSession: Send`（契約）を満たすため
  `Lua` を Send 可能にした。session は依然 per-eval 独立インスタンス。
- **`$rv` reserved tag record**: Luau の f64 のみの数値表現と Rhai の
  INT/FLOAT 区別を吸収し、i64 厳密・Time・ResourceId・Absent/Null を
  vendor 非依存に往復させる codec を両 backend で共通利用。
- **digest = canonical payload の SHA-256**: `version`/`revision` は
  artifact の `source_revision` と payload 内 `version` で表現し、
  semantic equality を digest 一致で検証。
- **memory quota は capability 広告で差分を正直に表現**: Rhai は
  `MaxMemoryBytes` 非対応を広告し、要求時 `UnsupportedBudget` で
  fail-closed（silent ignore しない — 契約の能力広告規則どおり）。
- **`ScriptError.message: Box<str>`**: `clippy::result_large_err`
  （Err ≥128B）対応。エラーは頻繁に生成されないため box 化のコストは
  無視できる。
