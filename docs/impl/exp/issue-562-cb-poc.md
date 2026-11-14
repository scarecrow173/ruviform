# Issue #562 Alternate Evaluation Context PoC — Experiment Log

headless 2 候補 alternate evaluation PoC の実験記録。
対象: `crates/ruviform-audio/tests/alternate_evaluation.rs`。
判定台帳: [`evidence/rf562/gap-report.md`](../../../evidence/rf562/gap-report.md)。

## 実験環境

- OS: Windows 11 x86_64
- Toolchain: mise 管理 Rust（`mise.toml` 正本）、canonical 検証は
  `mise run` 経由
- 検証日時: 2026-11-14
- Fixture: 既存 `tests/common/mod.rs`（`MockDsp`/`chain`/`feed`/
  `drain`）+ PoC 専用 fixture（`GainDsp`/`KeyDsp`/render/probe
  fixture、同 test file 内）

## 結果サマリ

| Case | 能力 | 結果 | 備考 |
| --- | --- | --- | --- |
| CB-01 | typed graph 独立 runtime・決定的比較 | Pass | interleaved vs 単独再実行が bit-identical |
| CB-02 | Preview A/B・Main/Recording 副作用なし・RT NoAlloc | Pass | Main 記録列が control と bit-identical、`assert_no_alloc` Pass |
| CB-03 | Bus/Sidechain・generation-scoped event | Pass | `main·(1+side·key)` 一致、stale event は counter drop |
| CB-04a | offline render・PDC/tail・決定性 | Pass | `total_latency` 0/32、`tail_evidence=Known`、再 render bit-identical |
| CB-04b | preflight failure 分離 | Pass | `OfflineUnsupported` reject、他候補非影響 |
| CB-04c | seek/loop 決定性 | Pass | run 間 bit-identical、loop body 観測 |
| CB-05 | probe cross-candidate 整列 | Pass（条件付き） | anchor 必須（fail-closed）。consumer が共有 epoch anchor を配線すれば `Aligned{EngineFrame}` |

## Gap 判定

機能 Gap なし。consumer 側 bookkeeping（評価入力同一性の anchor
宣言、probe domain→candidate 対応表）が必要 — 詳細は gap report
の Partial 項目を参照。`crates/ruviform-audio/src/` の変更なし。

## 実行コマンドと結果

- `mise run fmt-check` → Pass
- `mise run check ruviform-audio` → Pass
- `mise run lint ruviform-audio` → Pass（`-D warnings`）
- `mise run test ruviform-audio` → Pass（`alternate_evaluation` 7 test
  を含む全 target + doctest）

## スコープ外（NotRun/Deferred）

- 実 Plugin（VST3/CLAP）・実 GUI・実 Device、Windows L2/L3 —
  本スライス対象外。headless 結果を実機 Pass に読み替えない。
- macOS/Linux GUI-L2/EXT-L3、`*-msrv`、`*-full`、3 OS 横断 —
  既存 CI/Daily CI に委譲。
- `crates/ruviform-plugin-*` の 2 候補 fixture 専用テスト —
  編集範囲外のため Partial 記録（既存 plugin lifecycle テストで
  instance/state 分離は担保済み）。
