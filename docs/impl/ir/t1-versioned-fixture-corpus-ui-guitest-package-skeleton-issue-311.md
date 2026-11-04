---
id: "IMPL-034Rxm0i7UdJqQpZVGQvGB"
type: "impl"
status: "completed"
title: "T1 versioned fixture corpus + ui/guitest package skeleton (Issue #311)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "dir"
        path: "ui/guitest"
      - type: "file"
        path: "ui/guitest/package.json"
      - type: "file"
        path: "ui/guitest/package-lock.json"
      - type: "file"
        path: "ui/guitest/tsconfig.json"
      - type: "file"
        path: "ui/guitest/src/fixtureload.ts"
      - type: "file"
        path: "ui/guitest/fixtures/README.md"
      - type: "dir"
        path: "ui/guitest/fixtures/ipc"
      - type: "dir"
        path: "ui/guitest/fixtures/binding"
      - type: "dir"
        path: "ui/guitest/fixtures/scenarios"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "ui/guitest/package-lock.json"
  implements:
    - "docs/tasks/issue-311-t1-fixture-corpus.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-311-ipc-conformance-replay.md"
    - "docs/designs/issue-311-ipc-conformance-replay.md"
    - "docs/plans/issue-311-ipc-conformance-replay.md"
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

# T1 versioned fixture corpus + ui/guitest package skeleton (Issue #311)

## Summary

Issue #311 の正本である versioned fixture corpus と、それを読み込む test-only パッケージ `ui/guitest` の骨格を作成した。`fixtures/README.md` に形式 v1・バージョン規則・denylist/allowlist 語彙を正本化し、ipc・binding・scenario の 3 層 15 fixture を実装した。

## Context

- 実施タスク: docs/tasks/issue-311-t1-fixture-corpus.md
- 上流: docs/plans/issue-311-ipc-conformance-replay.md → docs/designs/（fixture 形式は design §3 が正本）
- 前提: 既存 `rvf.ipc.v1`（crates/ruviform-ui/src/ipc.rs）・`rvf://data`（ruviform-ui-webview/scheme.rs）・`ui/binding` 契約の棚卸し済み（docs/discovery/issue-311-ipc-conformance-inventory.md）

## Implementation

### Changes

- `ui/guitest/package.json` — private package `@ruviform/ui-guitest`、`typecheck`（tsc --noEmit）と `test`（node --test test/*.test.ts）scripts。依存は typescript + @types/node のみ（devDependency、テスト用途限定）
- `ui/guitest/tsconfig.json` — strict・ESNext・bundler resolution・`erasableSyntaxOnly`・`noUncheckedIndexedAccess`・`exactOptionalPropertyTypes`。DOM lib 非含有
- `ui/guitest/fixtures/README.md` — 形式 v1 の正本：suite 種別（ipc-intents/ipc-events/ipc-data/binding-messages/scenario-binding/scenario-host）、`v` フィールド規則、case/`expect` 語彙、denylist field（実 media/prompt/grant/token/binary 非含有）、`rawQuery`/`bodyGen`/`host` 特殊フィールド
- `ui/guitest/fixtures/ipc/intents.v1.json` — version/kind/op 検証・未知 field 許容・最大 body サイズ（`bodyGen: pad-intent` で byte 精度 pin）・malformed 拒否の 14 case
- `ui/guitest/fixtures/ipc/events.v1.json` — result/job/notify/lifecycle/error の 5 kind 受理 shape + malformed 拒否
- `ui/guitest/fixtures/ipc/data.v1.json` — url-build・range・maxChunk cap・offset==total empty-ok・bad_name・bad_params・unknown key 許容・Windows rewritten host の 15 case
- `ui/guitest/fixtures/binding/messages.v1.json` — CommandIntent/CommandReceipt/StateProjection/ViewportQuery/QueryReply の受理/拒否 + 安定拒否語彙
- `ui/guitest/fixtures/scenarios/binding/*.v1.json` — ack-matrix・close-view・commit-applied-observed・detach-reattach・late-applied-after-timeout・overlapping-gestures・preview-coalesce・query-matrix・unsupported-receipt の 9 scenario
- `ui/guitest/fixtures/scenarios/host/window-lifecycle.v1.json` — ready/message/tick/close/destroy/waitJobs 語彙の host lifecycle シナリオ
- `ui/guitest/src/fixtureload.ts` — `v`/`suite`/case 構造の fail-closed 検査ローダ。`loadFixtureFile`（通常 suite）と `loadScenarioFile`（setup/in/expect/seed 完全保持）の 2 経路

### Decisions During Implementation

- **`bodyGen` で byte 精度 pin**: intent 最大サイズの boundary を `padTo` で生成する方式にした。ハードコード巨大文字列を fixture に持たず、Rust/TS 両 lane が同じ生成規則で検証できる
- **`rawQuery` フィールド**: query string の parse 契約（unknown key 許容・`=` なし拒否）を JSON field 分解ではなく文字列のまま pin。scheme 層の実 parse 経路をそのまま通せる
- **scenario `ref` 語彙**: receipt 相関は `last`/`first`/`dead-epoch`/`unknown`/literal の 5 値に統一。`dead-epoch` は detach 直前 minted id を指し stale 分類を検証する
- **recordings/ fixture は作成しない方針に変更**: 録音往復は recorder test が inline JSONL で検証するため fixture 化を見送り（task doc の recordings/*.v1.jsonl 計画から乖離 — IR に記録）
- **`test/env.ts` 不作成**: Node 24+ の `node --test` が .ts を直接実行するため import hook 不要（同様に計画から乖離）

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64・Git Bash・Node 26.12.1（mise pin）。

| Check | コマンド | 結果 |
| --- | --- | --- |
| fixture 構造検査 | `npm --prefix ui/guitest test`（fixtureload 経由で全 15 fixture が parse 済み） | PASS — fail-closed ローダが全 fixture を受理・構造語彙一致 |
| 型検査 | `npm --prefix ui/guitest run typecheck` | PASS・0 errors |
| Rust 側読込 | `cargo test -p ruviform-ui --test ipc_conformance` + webview scheme fixture lane | PASS — 同 fixture 群を Rust 側でも読み込み済み（T4 で実装） |

## Risks

- fixture は手書きの正本 — 実装変更時に fixture と実コードの乖離が起き得る。両 lane のテストが乖離を検知する設計だが、契約変更時は fixture 側の `v` バンプ運用が前提
- `windows` host fixture は WebView2 rewrite 想定のみ（`rvf.` prefix 1 形態）。他の rewrite 形態が将来出た場合は case 追加が必要

## Follow-ups

- T2 で TS wire validator、T3 で recorder/replay/oracle、T4 で Rust conformance lane が同 corpus を消費する
- #294 接続時に fixture 修正は diff レビュー必須（Issue 本文の受入条件）
