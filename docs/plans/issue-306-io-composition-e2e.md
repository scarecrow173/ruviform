---
id: "PLAN-W0d5X9GIZLFMnSGqu9OlBg"
type: "plan"
status: "approved"
title: "Issue #306 汎用 I/O 合成 E2E 実装計画 (IO-IMP-12)"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-306-io-composition-e2e.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-io-composition-e2e.md"
    - "docs/designs/issue-306-io-composition-e2e.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-306-io-composition-e2e.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 汎用 I/O 合成 E2E 実装計画 (IO-IMP-12) Implementation Plan

## Goal

`crates/ruviform-io-e2e`（試験専用・公開 API なし）を新設し、
StagedFile/Structured/Compression/Archive/WAV の 5 合成境界を
実 OS 上で pin する。`mise run test-io-e2e` を Gate command として
提供し、RF #305 候補の scratch 検証証跡を残す。

## Scope

単一実装ストリーム・1 plan・6 task。変更は `crates/ruviform-io-e2e/`
新設・workspace `members`・`crates/README.md`・`Cargo.lock`・
`AGENTS.md` クレート数・`mise.toml`・`evidence/rf306/`・本 doc
チェーンに限定。既存クレートの公開 API・挙動を変えない。

## Upstream Documents

- [docs/specs/issue-306-io-composition-e2e.md](../specs/issue-306-io-composition-e2e.md)（approved）
- Issue #306 本文（受入正本）・Issue #288 IO-06 v0.1・
  Issue #282 横断設計正本
- [docs/designs/issue-306-io-composition-e2e.md](../designs/issue-306-io-composition-e2e.md)（approved）
- [docs/test-specs/issue-306-io-composition-e2e.md](../test-specs/issue-306-io-composition-e2e.md)（検証正本）

## File Map

- `crates/ruviform-io-e2e/Cargo.toml` — dev-deps（io-file /
  io-structured[json,csv,file] / io-compression[gzip,zstd] /
  io-archive / io-wav / io-symphonia / media-io / media-core /
  audio-core / serde[derive]）・workspace lints
- `crates/ruviform-io-e2e/src/lib.rs` — crate doc のみ（試験専用・
  非 RT・公開 API なしの宣言）
- `crates/ruviform-io-e2e/README.md` — 責務・依存方向・検証入口
- `crates/ruviform-io-e2e/tests/common/mod.rs` — RAII `TestDir`・
  staged 補助（archive tests/common と同型、tempfile 依存なし）
- `crates/ruviform-io-e2e/tests/staged_lifecycle.rs` — File 境界
- `crates/ruviform-io-e2e/tests/structured_staged.rs` — Structured 境界
- `crates/ruviform-io-e2e/tests/archive_compression_staged.rs` —
  TAR→compression→StagedFile 境界
- `crates/ruviform-io-e2e/tests/extraction.rs` —
  ZIP/TAR→Secure Extraction 境界
- `crates/ruviform-io-e2e/tests/wav_staged.rs` — WAV finalize→
  seal→readback→publish→reopen 境界
- `mise.toml` — `[tasks.test-io-e2e]` 追加
- `evidence/rf306/` — scratch 検証・最終 report

## Task Breakdown

| Task | 内容 | 依存 |
| --- | --- | --- |
| T1 | クレート scaffold（Cargo.toml/lib.rs/README/common helper + workspace members + ledger） | — |
| T2 | `staged_lifecycle.rs` + `structured_staged.rs`（File・Structured 境界） | T1 |
| T3 | `archive_compression_staged.rs` + `extraction.rs`（Archive/Compression 境界） | T1 |
| T4 | `wav_staged.rs`（WAV finalize→publish 境界） | T1 |
| T5 | `mise.toml` task + `mise run` 検証（fmt/lint/test） | T1-T4 |
| T6 | RF #305 候補 scratch 検証 + evidence 記録 | T4,T5 |

## Risk Register

- Windows の file handle 共有制約: staged temp を readback 中に
  外部書込する試験は handle 競合を起こしうる → 破壊系試験は
  seal 前に bytes を直接書き、readback は読み専用で扱う。
- `WavMuxer<&mut StagedFile>` の借用: muxer が `&mut staged` を
  保持する間 `seal` 不可 → `into_sink()` で借用解除してから
  `seal` する順序を試験が固定する。
- Publish 失敗分類は OS 差がありうる（CreateNew 競合の
  `DestinationExists` は OS 共通契約として io-file が保証）— OS 依存
  分岐が必要な場合は `cfg` で分け、 Unsupported は正直に記録。
- scratch checkout は他 worker と同じ `.git` を共有 — `git worktree
  add`/`remove` のみを TEMP 配下で行い、他者 branch/worktree に
  触れない。

## Verification Matrix

- `mise run test-io-e2e` — 全合成試験 pass（Windows NTFS 実測）。
- `mise run fmt-check` / `mise run lint` / `mise run test`
  （workspace 全体の非回帰）。
- scratch（RF #305 `82256cf…`）: 旧 API grep 不在証跡 +
  io-wav 試験実走 + `WavMuxer<&mut StagedFile>` 合成の再演。
- evidence: `evidence/rf306/report.md` に SHA/command/exit code/
  OS/toolchain/NotRun 項目を実値記録。

## Rollback Strategy

新規クレートの追加のみのため、`members`・ledger・`mise.toml` の
3 箇所の追加行を除去し `crates/ruviform-io-e2e/` と
`evidence/rf306/` を削除すれば完全 rollback。既存コード変更なし。
