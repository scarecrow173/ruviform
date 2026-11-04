---
id: "IMPL-034Rxm8AqV5np7x1mgQehL"
type: "impl"
status: "completed"
title: "T3 recorder/replay/oracle + consumer mocks + fault-matrix scenarios (Issue #311)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "ui/guitest/src/jsonl.ts"
      - type: "file"
        path: "ui/guitest/src/recorder.ts"
      - type: "file"
        path: "ui/guitest/src/seed.ts"
      - type: "file"
        path: "ui/guitest/src/consumers.ts"
      - type: "file"
        path: "ui/guitest/src/oracle.ts"
      - type: "file"
        path: "ui/guitest/src/replay.ts"
      - type: "file"
        path: "ui/guitest/src/timing.ts"
      - type: "file"
        path: "ui/guitest/test/recorder.test.ts"
      - type: "file"
        path: "ui/guitest/test/scenarios.test.ts"
      - type: "file"
        path: "ui/guitest/test/faultmatrix.test.ts"
    modified:
      - type: "file"
        path: "ui/guitest/src/index.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-311-t3-recorder-replay-oracle.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-311-t1-fixture-corpus.md"
    - "docs/tasks/issue-311-t2-ts-wire-validators.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-311-ipc-conformance-replay.md"
    - "docs/designs/issue-311-ipc-conformance-replay.md"
    - "docs/test-specs/issue-311-ipc-conformance-replay.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# T3 recorder/replay/oracle + consumer mocks + fault-matrix scenarios (Issue #311)

## Summary

opt-in bounded JSONL recorder、seeded fault generator、両 Consumer 形 mock、独立 ContractModel oracle、実オブジェクト注入 replay driver を実装。scenario corpus + seeded fault matrix の全 replay が実装 trace と独立 oracle trace の deep-equality で PASS。記録済み応答の再生は PASS 条件に含まない。

## Context

- 実施タスク: docs/tasks/issue-311-t3-recorder-replay-oracle.md
- 上流: design §5–7（replay アーキテクチャ・oracle 独立性・recorder 契約）・test-spec（受入判定基準）
- 前提: T1 fixture + T2 validator 語彙

## Implementation

### Changes

- `src/timing.ts` — `ManualTimers`（注入 logical clock + deterministic scheduled callback）と `deferredFlush`（明示的 queue drain — production の event-loop deferred flush 意味を再現）
- `src/jsonl.ts` — record line 語彙（`v`/`tick`/`dir`/`type` + allowlist field）・`serializeRecord`/`verifyJsonl`/`parseRecording`・fail-closed
- `src/recorder.ts` — `IpcRecorder`: `wrapPort`/`observe`、既定 `enabled:false` no-op、`maxEntries`/`maxBytes` bounded、最初の overflow で `sys/dropped` marker 1 回 + 以後 drop 計数、文字列 128 文字 truncate、`toJsonl`。方向意味: port 境界呼出（dispatch/subscribe/query/cancelQuery）は `out`、reply は `in`
- `src/seed.ts` — mulberry32 PRNG + `genAckStorm`（reorder/duplicate/unknown-id receipt storm）+ `genAttachStorm`（detach/attach + regressed revision projection）
- `src/consumers.ts` — `ScriptedDawBackend`（dispatch/subscribe 中心・receipt 明示制御・`deliver`）と `ScriptedAnalysisBackend`（query/cancelQuery 中心・`resolveQuery`/`rejectQuery`/`currentRevision` probe）。`ui/binding/test/mocks.ts` の契約形状を写すが製品語彙は持ち込まない
- `src/oracle.ts` — `ContractModel`: 文書化契約規則のみの観測モデル（dispatch 期待列・receipt rank 分類 in-flight<accepted/queued<applied<terminal・projection revision 単調性・generation/detach 規則・query staleness/overflow・link 期待）。実装 src とコード共有なし（型 import のみ）
- `src/replay.ts` — `buildHarness`（実 `InteractionControl`/`CommandBinding`/`QueryBinding`/`LinkRegistry`/`ViewLink` + backend + recorder 組立）・`runOps`（scenario `in` 語彙を実オブジェクトへ注入）・`collectTrace`・`checkExpects`（ordered-subsequence pin 照合）
- `test/recorder.test.ts` — allowlist・bounded・dropped marker・truncate・disabled no-op・JSONL round-trip
- `test/scenarios.test.ts` — 9 scenario × （実 trace deep-equals oracle・expect pins 保持・2 回実行で同一 trace）
- `test/faultmatrix.test.ts` — seeded ack/attach storm を複数 seed で replay・oracle 照合・決定性・terminal 保存・ghost command 非発生

### Decisions During Implementation

- **方向意味の修正**: dispatch/subscribe/query/cancelQuery は全て `out`（binding→consumer）、reply のみ `in`。初期実装は意味が逆で、録画 diff レビューで発見して修正
- **`flush` op は production の deferred flush を再現**: `h.deferred.runAll()`（登録順全 callback drain）+ `model.flush()`。手動 per-binding drain ではない
- **receipt routing の修正**: minted 済み id は minting binding へ、未知 id は先頭 binding（port-edge 到着）へ。oracle の `ack()` も同規則
- **oracle の `currentRevision` 初期値 undefined**: 未設定時は source-revision probe を skip（fixture で非意図 stale 判定を防止）
- **`checkExpects` は exact equality ではなく ordered subsequence**: interleaved 中間 event（progress 等）に耐性。contract event のみ pin する設計
- **entries-bound overflow で marker 枠を確保**: recorder の entry cap 到達時にも `sys/dropped` marker が必ず出るよう 1 枠を予約（初期実装は満杯で marker 自体が drop されていた）

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64・Git Bash・Node 26.12.1。

| Check | コマンド | 結果 |
| --- | --- | --- |
| 全テスト | `npm --prefix ui/guitest test` | PASS — 135 tests・0 fail（9 scenario×3 + fault-matrix 10 + recorder + wire + binding） |
| 決定性 | scenarios.test.ts 内の 2 回実行 deep-equality + faultmatrix per-seed 再実行 | PASS — 同一 seed/clock/入力で bit 同一 trace |
| 型検査 | `npm --prefix ui/guitest run typecheck` | PASS・0 errors |
| recorder 契約 | recorder.test.ts | PASS — bounds・dropped marker・truncate・disabled 透過・round-trip |

## Risks

- oracle は「文書化契約の独立実装」だが、実装と同じ誤読を書き手が共有し得るリスクは残る → 対策として fixture `expect` pin は手書き（oracle 生成ではない）に留め、receipt rank/detach 規則は binding の doc comment から独立読解で実装した
- `genAckStorm` の `cmd-N` ref は minting 順 `cmd-(N+1)` に test 側で remap している。commandSeq の mint 規則が変わると test が追随を要する（型ではなく文字列契約）

## Follow-ups

- T4 で同一 corpus の Rust 側 lane + fault-matrix の CI 化
- #294 証拠: replay 決定性・oracle 照合・fault 保存の各 lane が証拠項目として docs に記録済み
