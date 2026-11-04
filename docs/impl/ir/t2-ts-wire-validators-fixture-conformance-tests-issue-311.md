---
id: "IMPL-034Rxm7tUdo6EFHLFt4DIx"
type: "impl"
status: "completed"
title: "T2 TS wire validators + fixture conformance tests (Issue #311)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "ui/guitest/src/ipcwire.ts"
      - type: "file"
        path: "ui/guitest/src/bindingwire.ts"
      - type: "file"
        path: "ui/guitest/test/wire.test.ts"
      - type: "file"
        path: "ui/guitest/test/bindingwire.test.ts"
    modified:
      - type: "file"
        path: "ui/guitest/src/index.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-311-t2-ts-wire-validators.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-311-t1-fixture-corpus.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-311-ipc-conformance-replay.md"
    - "docs/designs/issue-311-ipc-conformance-replay.md"
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

# T2 TS wire validators + fixture conformance tests (Issue #311)

## Summary

`rvf.ipc.v1`・`rvf://data`・Binding メッセージの TS 側 fail-closed validator を実装し、T1 の fixture corpus を全 case で照合する conformance テストを通した。zod 不使用・依存ゼロ・mirror 実装は実装 src とコード共有なし。

## Context

- 実施タスク: docs/tasks/issue-311-t2-ts-wire-validators.md
- 上流: design §4（validator 契約）・docs/discovery（既存 Rust `parse_intent`/`serve_data`/binding 契約の棚卸しが判定正本）
- 前提: T1 の fixture corpus + fixtureload

## Implementation

### Changes

- `src/ipcwire.ts` — `parseIntent`（Rust `parse_intent` と同一判定: サイズ上限・`v`/`kind`/`op` 形・`id` 型・未知 field 許容）と `decodeEvent`（`v`/`kind`/相関 field・optional `detail`/`details`）。`dataUrl` 構築（segment encode・offset default 0）と `serveData` 応答モデル（bodyLen ≤ min(len,total-offset,maxChunk)・`X-Rvf-Total`・offset==total → 200 empty・status 写像 200/400/404）
- `src/bindingwire.ts` — `validateCommandIntent`/`CommandReceipt`/`StateProjection`/`ViewportQuery`/`QueryReply`。必須 field・kind 語彙・数値範囲を fail-closed 検査し、安定拒否語彙 `missing-field`/`bad-kind`/`bad-type`/`out-of-range` を返す
- `test/wire.test.ts` — ipc/intents・ipc/events・ipc/data の全 fixture case を TS 側で処理し `expect` と照合（同一 version/同一意味）
- `test/bindingwire.test.ts` — binding/messages fixture の全受理/拒否 case + 拒否理由語彙照合

### Decisions During Implementation

- **zod 不使用・依存ゼロ**: Issue 本文で「zod 等は実装候補、必須採用しない」が明記されているため手書き validator。拒否語彙を Rust 側 error code と 1:1 に合わせた
- **mirror は判定のみ共有**: wire 意味（受理/拒否・status・cap 計算）だけを mirror し、実装コードからの import は型のみ。oracle の独立性原則を T2 から適用
- **`Event::Notify` は Rust 側 constructor 不在**: wire shape のみ予約済みのため、decode 側は TS が担当する旨を fixture とコメントに明記（T4 の Rust lane 分割と整合）

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64・Git Bash・Node 26.12.1。

| Check | コマンド | 結果 |
| --- | --- | --- |
| 全 fixture conformance | `npm --prefix ui/guitest test` | PASS — intents/events/data/binding-messages の全 case が `expect` と一致 |
| 型検査 | `npm --prefix ui/guitest run typecheck` | PASS・0 errors |
| mise lane | `mise run check-ui-guitest` / `test-ui-guitest` | PASS — lockfile 復元 → typecheck → node --test まで一貫 |

## Risks

- validator は「現在の契約の mirror」。契約拡張時は fixture `v` バンプ + validator 更新がセット
- `serveData` の応答はモデル（status/bodyLen/total 判定）。実 HTTP header は webview scheme lane（T4）で pin 済み

## Follow-ups

- T3 の recorder/replay が本 validator の語彙を再利用（jsonl record allowlist の `type`/`reason` 語彙と整合）
