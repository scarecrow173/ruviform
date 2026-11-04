---
id: "TASK-KayYlp12EsEWZZKk2pMZgF"
type: "task"
status: "done"
title: "T3 recorder/replay/oracle + consumer mocks + seeded シナリオ（Issue #311）"
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
        path: "ui/guitest/src/replay.ts"
      - type: "file"
        path: "ui/guitest/src/oracle.ts"
      - type: "file"
        path: "ui/guitest/src/consumers.ts"
      - type: "file"
        path: "ui/guitest/src/seed.ts"
      - type: "file"
        path: "ui/guitest/test/recorder.test.ts"
      - type: "file"
        path: "ui/guitest/test/scenarios.test.ts"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-311-ipc-conformance-replay.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-311-t1-fixture-corpus.md"
    - "docs/tasks/issue-311-t2-ts-wire-validators.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-311-ipc-conformance-replay.md"
  references: []
  defers: []
  deferred-by: []
---

# T3 recorder/replay/oracle + consumer mocks + seeded シナリオ

## Work

- `src/jsonl.ts`: record line 語彙（`v`/`tick`/`dir`/`type` + allowlist field）・parse/serialize・fail-closed 検査。
- `src/recorder.ts`: `IpcRecorder` — `wrapPort`/`observe`、既定 `enabled:false` no-op、`maxEntries`/`maxBytes` bounded、`sys/dropped` マーカー、128 文字 truncate、`toJsonl`。
- `src/seed.ts`: mulberry32 PRNG + seeded fault シナリオ生成（fault 種別・順序・遅延を seed から導出）。
- `src/consumers.ts`: `ScriptedDawBackend`（dispatch/subscribe 中心、receipt 制御）・`ScriptedAnalysisBackend`（query/cancelQuery 中心、park/resolve 制御）。`ui/binding/test/mocks.ts` の契約形状を写すが製品語彙は持ち込まない。
- `src/oracle.ts`: ContractModel — 文書化契約規則のみの観測モデル（dispatch 期待列・phase 表・receipt rank 分類・projection/query 判定・link 期待）。実装 src とコード共有なし（型 import のみ）。
- `src/replay.ts`: scenario/JSONL 入力列を実 `CommandBinding`/`QueryBinding`/`ViewLink`/`InteractionControl` に注入して trace を収集、`oracle` 出力と照合する driver。2 回実行の trace 一致を検証する `assertDeterministic`。
- `test/recorder.test.ts`: allowlist・bounded・dropped・truncate・enabled:false no-op・JSONL round-trip。
- `test/scenarios.test.ts`: scenario replay・oracle 照合・同一 seed 決定性・記録往復。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/issue-311-ipc-conformance-replay.md` §3–§4
- `docs/tasks/issue-311-t1-fixture-corpus.md`
- `docs/tasks/issue-311-t2-ts-wire-validators.md`

## Verification

- `npm --prefix ui/guitest run typecheck`・`node --test`（後に mise タスク経由）。
