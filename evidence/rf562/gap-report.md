# RF-CB-01 Gap Report — Issue #562 Alternate Evaluation Context PoC

対象: Synvelis Creative Branch の「固定入力下での代替 Graph/Plugin
状態評価」を Ruviform 既存 API で実現できるかの headless 検証。
Evidence: `crates/ruviform-audio/tests/alternate_evaluation.rs`
（7 test、全 Pass、2026-11-14、Windows 11 x86_64、mise toolchain）。

## 判定サマリ

**機能 Gap: なし** — Typed Graph / Offline Render / Probe の既存
API で 2 候補 alternate evaluation の全契約を検証できた。
`crates/ruviform-audio/src/` の変更は不要。

Consumer 側に必要な bookkeeping が 2 点ある（Partial、後述）。
いずれも consumer 責務であり、Ruviform 側の新規 API 追加は不要と
判断する。後続 Issue 起票案: **なし**（Coordinator が close 判断を
行う場合の根拠として本台帳を使用）。

## 採点表

| # | 能力領域 | 判定 | 根拠（Case / API） |
| --- | --- | --- | --- |
| 1 | State Isolation（候補間 mutable state 非共有・決定的比較） | **Pass** | CB-01。独立 `GraphRuntime`+`RtRuntime` pair、interleaved vs 単独再実行 bit-identical。API: `Compiler`/`GraphRuntime`/`RtRuntime` |
| 2 | Bus/Sidechain/Event | **Pass** | CB-03。`PortRole::Sidechain`、`main·(1+side·key)` 一致、generation-scoped event は `stale_events` counter で drop。API: `PortRole`/event channel/generation |
| 3 | Tail / Latency / PDC | **Pass** | CB-04a。`PreparedRender` per-instance 独立、`total_latency` 0/32 報告、`tail_evidence=Known`、emitted に latency 分の signal ずれを観測。API: `PreparedRender`/`OfflineRenderResult` |
| 4 | Preview A/B（Main/Recording 副作用禁止） | **Pass** | CB-02。Main runtime 記録列が単独 control と bit-identical。候補評価は consumer 側の出力選択であり preview 専用 API 不要。API: 独立 runtime pair |
| 5 | RT NoAlloc / NoBlocking | **Pass** | CB-02。warm-up 後 `rt.process` を `assert_no_alloc` で囲み allocation 0。既存 RT 契約を候補 runtime にもそのまま適用可能。API: `assert_no_alloc`/RT process path |
| 6 | Seek / Loop / Failure | **Pass** | CB-04b/c。`OfflineUnsupported` preflight reject で他候補非影響。seek/loop transport は run 間 bit-identical、loop body 観測。API: `RenderTransport`/`PreflightError` |
| 7 | Probe 観測・整列（cross-candidate） | **Partial** | CB-05。独立 `probe::pair` は一意 domain を mint するため anchor なしでは `Incomparable`（fail-closed、仕様どおり）。consumer が共有 engine epoch の `ClockAnchor` を配線すれば `Aligned{EngineFrame}`・offset 0・全 overlap。候補識別は `ProbeClockMark.domain`/`runtime_instance`/`ProbeId` が担う（generation は runtime ごとの counter で両候補 `single(1)` — 識別には使えない） |
| 8 | Plugin lifecycle（mock instance/state 分離） | **Partial** | 既存 `ruviform-plugin-{core,host}` の lifecycle/state 分離は既存テストで担保。本スライスの編集範囲（`crates/ruviform-plugin-*` は read-only 参照のみ）では 2 候補専用 fixture を追加できず、headless mock 経由の plugin 2 候補直接検証は未実施。機能 Gap ではなく検証範囲の制約 |

## Partial 項目の詳細（consumer bookkeeping — Gap ではない）

1. **評価入力同一性の宣言**: Ruviform は「2 候補が同一固定入力を
   受けた」ことを推測しない。consumer が各候補の `ProbeTimeContext`
   に共有 engine epoch の `ClockAnchor` を配線して宣言する
   （CB-05 で実証）。これは Issue 本文の `EvaluationInputIdentity`
   に相当する役割を既存 `ClockAnchor`/`ProbeTimeContext` で担う。
   → consumer 側の責務。新規 generic 型は不要。
2. **候補識別の bookkeeping**: probe generation は runtime ごとの
   counter で候補間で区別できない。consumer は `ProbeClockMark.domain`
   / `ProbeTimeContext.runtime_instance` / `ProbeId`+`ProbePoint`
   の組で候補を識別する対応表を保持する。これは consumer の
   orchestration 責務であり、Ruviform 側に candidate ID 概念を
   持ち込む必要はない（Synvelis 意味論の持ち込み禁止とも整合）。

## Gap 判定の結論

上記 Partial はいずれも **consumer 側の宣言/bookkeeping 責務**で
あり、Ruviform が提供すべき generic capability の不足ではない。
実証済み Gap がないため:

- `crates/ruviform-audio/src/` の変更: **なし**（最小変更不要）。
- 新規 Generic API（`EvaluationInputIdentity`/`PreparedEvaluationContext`/
  `EvaluationReceipt`/`Probe・AudioCompareMetadata` 等）: **追加不要**。
- 後続 Issue 案: **なし**。
- Issue #562 は「no new API」で close 可能 — close 判断は
  Coordinator が行う。

## 対応表（#294 接続用・記録のみ）

| Consumer | Oracle | 本 PoC の対応 |
| --- | --- | --- |
| AudioLens | f64/長尺/CLI 独立 Oracle | 対象外（Ruviform は f32/RT 系。headless mock DSP のみ） |
| Synvelis | f32/RT/Creative Branch | CB-01〜CB-06 で State Isolation/Preview/Bus/Event/Tail/PDC/Seek/Loop/NoAlloc/Probe 整列を headless 検証 |

実走 Gate（対象 SHA での oracle 実行）は #294 側の責務。
本 headless 結果を実 Plugin/実機 Pass に読み替えない。

## スコープ外記録（NotRun/Deferred）

| 項目 | 状態 | 理由 |
| --- | --- | --- |
| 実 Plugin（VST3/CLAP）2 候補 instance/state 分離 | NotRun | Windows L2/L3 は他 Owner/後続スライス |
| 実 GUI 試聴 A/B | NotRun | 同上（Synvelis 側 Owner と検証） |
| 実 Device/モニター | NotRun | EXT-L3、別スライス |
| macOS/Linux GUI-L2/EXT-L3 | Deferred | 既存 CI レーンに委譲、本スライス対象外 |
| `*-msrv` / `*-full` / 3 OS 横断 | NotRun | Daily CI の責務、タスク指定なし |
| `crates/ruviform-plugin-*` 2 候補専用 fixture | NotRun | 編集範囲外（read-only 参照のみ） |
